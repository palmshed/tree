use crate::refs::GitInspector;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tracing::{info, warn};
use tree_core::error::{Result, TreeError};
use tree_core::models::{Repository, RepositorySummary};

pub struct GitEngine;

impl GitEngine {
    /// Validates an owner name or repo name to ensure safety and prevent directory traversal
    pub fn sanitize_name(name: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TreeError::InvalidName("Name cannot be empty".into()));
        }

        if trimmed.starts_with('.') || trimmed.starts_with('-') {
            return Err(TreeError::InvalidName(
                "Name cannot start with '.' or '-'".into(),
            ));
        }

        // Prevent directory traversal
        if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
            return Err(TreeError::InvalidName(
                "Name cannot contain path separators or parent directory references".into(),
            ));
        }

        // Must be alphanumeric with allowed characters: '-', '_', '.'
        for ch in trimmed.chars() {
            if !ch.is_alphanumeric() && ch != '-' && ch != '_' && ch != '.' {
                return Err(TreeError::InvalidName(format!(
                    "Name contains invalid character '{}'",
                    ch
                )));
            }
        }

        Ok(trimmed.to_string())
    }

    /// Computes safe disk path for a repository
    pub fn compute_disk_path(base_dir: &Path, owner: &str, repo_name: &str) -> Result<PathBuf> {
        let clean_owner = Self::sanitize_name(owner)?;
        let clean_name = Self::sanitize_name(repo_name.trim_end_matches(".git"))?;

        let repo_folder = format!("{}.git", clean_name);
        let path = base_dir.join(clean_owner).join(repo_folder);
        Ok(path)
    }

    /// Initializes a new bare Git repository on disk
    pub async fn init_bare_repository(
        base_dir: &Path,
        owner: &str,
        repo_name: &str,
        default_branch: &str,
    ) -> Result<PathBuf> {
        let disk_path = Self::compute_disk_path(base_dir, owner, repo_name)?;

        if disk_path.exists() {
            return Err(TreeError::RepositoryAlreadyExists {
                owner: owner.to_string(),
                name: repo_name.to_string(),
            });
        }

        fs::create_dir_all(&disk_path)
            .await
            .map_err(|e| TreeError::Git(format!("Failed to create repository directory: {}", e)))?;

        info!("Initializing bare git repository at {:?}", disk_path);

        let output = Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(&disk_path)
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute git init: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = fs::remove_dir_all(&disk_path).await;
            return Err(TreeError::Git(format!("git init failed: {}", stderr)));
        }

        // Set default branch symbolic-ref HEAD
        let branch_ref = format!("refs/heads/{}", default_branch);
        let sym_output = Command::new("git")
            .arg("--git-dir")
            .arg(&disk_path)
            .args(["symbolic-ref", "HEAD", &branch_ref])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to set symbolic-ref HEAD: {}", e)))?;

        if !sym_output.status.success() {
            warn!(
                "Failed to set default branch symbolic-ref: {}",
                String::from_utf8_lossy(&sym_output.stderr)
            );
        }

        // Enable http.receivepack
        let _ = Command::new("git")
            .arg("--git-dir")
            .arg(&disk_path)
            .args(["config", "http.receivepack", "true"])
            .output()
            .await;

        Ok(disk_path)
    }

    /// Deletes bare repository directory on disk
    pub async fn delete_repository(disk_path: &Path) -> Result<()> {
        if disk_path.exists() {
            info!("Removing repository from disk: {:?}", disk_path);
            fs::remove_dir_all(disk_path).await.map_err(|e| {
                TreeError::Git(format!("Failed to remove repository directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Checks if a bare repository is valid on disk
    pub fn is_valid_bare_repo(disk_path: &Path) -> bool {
        disk_path.exists() && disk_path.join("HEAD").exists() && disk_path.join("objects").exists()
    }

    /// Builds a repository summary
    pub async fn get_repository_summary(
        disk_path: &Path,
        repo: &Repository,
        base_url: &str,
    ) -> Result<RepositorySummary> {
        let branches = GitInspector::list_branches(disk_path, &repo.default_branch)
            .await
            .unwrap_or_default();
        let tags = GitInspector::list_tags(disk_path).await.unwrap_or_default();
        let commits = GitInspector::list_commits(disk_path, Some(&repo.default_branch), 100, 0)
            .await
            .unwrap_or_default();
        let is_empty = branches.is_empty() && commits.is_empty();

        let readme_content = if !is_empty {
            GitInspector::get_readme(disk_path, Some(&repo.default_branch))
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let clone_url_http = format!(
            "{}/{}/{}.git",
            base_url.trim_end_matches('/'),
            repo.owner_name,
            repo.name
        );
        let clone_url_ssh = format!("git@tree.local:{}/{}.git", repo.owner_name, repo.name);

        Ok(RepositorySummary {
            repository: repo.clone(),
            default_branch: repo.default_branch.clone(),
            branches_count: branches.len(),
            tags_count: tags.len(),
            commits_count: commits.len(),
            is_empty,
            clone_url_http,
            clone_url_ssh,
            readme_content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(GitEngine::sanitize_name("my-repo").unwrap(), "my-repo");
        assert_eq!(
            GitEngine::sanitize_name("user_123.test").unwrap(),
            "user_123.test"
        );
        assert!(GitEngine::sanitize_name("../bad").is_err());
        assert!(GitEngine::sanitize_name("foo/bar").is_err());
        assert!(GitEngine::sanitize_name("").is_err());
    }

    #[tokio::test]
    async fn test_init_and_delete_repo() {
        let dir = tempdir().unwrap();
        let path = GitEngine::init_bare_repository(dir.path(), "alice", "project", "main")
            .await
            .unwrap();

        assert!(GitEngine::is_valid_bare_repo(&path));
        assert!(path.join("HEAD").exists());

        GitEngine::delete_repository(&path).await.unwrap();
        assert!(!path.exists());
    }
}
