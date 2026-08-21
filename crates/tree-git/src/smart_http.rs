use bytes::Bytes;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::{debug, error, info};
use tree_core::error::{Result, TreeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitService {
    UploadPack,
    ReceivePack,
}

impl GitService {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitService::UploadPack => "git-upload-pack",
            GitService::ReceivePack => "git-receive-pack",
        }
    }

    pub fn advertisement_content_type(&self) -> &'static str {
        match self {
            GitService::UploadPack => "application/x-git-upload-pack-advertisement",
            GitService::ReceivePack => "application/x-git-receive-pack-advertisement",
        }
    }

    pub fn result_content_type(&self) -> &'static str {
        match self {
            GitService::UploadPack => "application/x-git-upload-pack-result",
            GitService::ReceivePack => "application/x-git-receive-pack-result",
        }
    }

    pub fn from_service_name(name: &str) -> Option<Self> {
        match name {
            "git-upload-pack" => Some(GitService::UploadPack),
            "git-receive-pack" => Some(GitService::ReceivePack),
            _ => None,
        }
    }
}

pub struct SmartHttpHandler;

impl SmartHttpHandler {
    /// Formats a packet-line according to Git Smart HTTP protocol specification
    pub fn pkt_line(payload: &str) -> Vec<u8> {
        let len = payload.len() + 4;
        let header = format!("{:04x}", len);
        let mut res = Vec::with_capacity(len);
        res.extend_from_slice(header.as_bytes());
        res.extend_from_slice(payload.as_bytes());
        res
    }

    /// Formats a flush-pkt (0000)
    pub fn pkt_flush() -> &'static [u8] {
        b"0000"
    }

    /// Handles GET /:owner/:name.git/info/refs?service=...
    pub async fn advertise_refs(repo_path: &Path, service: GitService) -> Result<(String, Vec<u8>)> {
        if !repo_path.exists() {
            return Err(TreeError::Git(format!(
                "Repository does not exist at {:?}",
                repo_path
            )));
        }

        let service_cmd = service.as_str();
        debug!(
            "Running advertise-refs for service: {} on repo: {:?}",
            service_cmd, repo_path
        );

        let output = Command::new("git")
            .arg(service_cmd.trim_start_matches("git-"))
            .arg("--stateless-rpc")
            .arg("--advertise-refs")
            .arg(repo_path)
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute {}: {}", service_cmd, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Git advertise-refs error: {}", stderr);
            return Err(TreeError::Git(format!("Git error: {}", stderr)));
        }

        let mut body = Vec::new();
        let service_line = format!("# service={}\n", service_cmd);
        body.extend_from_slice(&Self::pkt_line(&service_line));
        body.extend_from_slice(Self::pkt_flush());
        body.extend_from_slice(&output.stdout);

        Ok((service.advertisement_content_type().to_string(), body))
    }

    /// Handles POST /:owner/:name.git/git-upload-pack or git-receive-pack
    pub async fn execute_rpc(
        repo_path: &Path,
        service: GitService,
        input_body: Bytes,
    ) -> Result<(String, Vec<u8>)> {
        if !repo_path.exists() {
            return Err(TreeError::Git(format!(
                "Repository does not exist at {:?}",
                repo_path
            )));
        }

        let service_cmd = service.as_str();
        debug!(
            "Executing RPC for service: {} on repo: {:?} (input size: {} bytes)",
            service_cmd,
            repo_path,
            input_body.len()
        );

        let mut child = Command::new("git")
            .arg(service_cmd.trim_start_matches("git-"))
            .arg("--stateless-rpc")
            .arg(repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TreeError::Git(format!("Failed to spawn {}: {}", service_cmd, e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            tokio::spawn(async move {
                let _ = stdin.write_all(&input_body).await;
                let _ = stdin.shutdown().await;
            });
        }

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_end(&mut stdout_buf).await;
        }

        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_end(&mut stderr_buf).await;
        }

        let status = child
            .wait()
            .await
            .map_err(|e| TreeError::Git(format!("Process wait error: {}", e)))?;

        if !status.success() {
            let err_msg = String::from_utf8_lossy(&stderr_buf);
            error!("Git RPC {} failed with exit code {:?}: {}", service_cmd, status.code(), err_msg);
            // Some git operations may have non-fatal warnings on stderr, but if status failed:
            if !stdout_buf.is_empty() {
                // If stdout has pack protocol response, return it
                return Ok((service.result_content_type().to_string(), stdout_buf));
            }
            return Err(TreeError::Git(format!("Git RPC failed: {}", err_msg)));
        }

        info!(
            "Git RPC {} succeeded, output size: {} bytes",
            service_cmd,
            stdout_buf.len()
        );

        Ok((service.result_content_type().to_string(), stdout_buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_pkt_line_formatting() {
        let line = SmartHttpHandler::pkt_line("# service=git-upload-pack\n");
        assert_eq!(&line[0..4], b"001e");
        assert_eq!(&line[4..], b"# service=git-upload-pack\n");
    }

    #[tokio::test]
    async fn test_advertise_refs_empty_repo() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("test.git");
        
        // Init bare repo
        let out = Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(&repo_path)
            .output()
            .await
            .unwrap();
        assert!(out.status.success());

        let (ct, body) = SmartHttpHandler::advertise_refs(&repo_path, GitService::UploadPack)
            .await
            .unwrap();

        assert_eq!(ct, "application/x-git-upload-pack-advertisement");
        assert!(body.starts_with(b"001e# service=git-upload-pack\n0000"));
    }
}
