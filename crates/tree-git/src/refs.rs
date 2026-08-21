use chrono::{DateTime, Utc};
use std::path::Path;
use tokio::process::Command;
use tree_core::error::{Result, TreeError};
use tree_core::models::{BranchInfo, CommitInfo, FileContent, FileEntry, TagInfo};

pub struct GitInspector;

impl GitInspector {
    /// List all branches in the bare repository
    pub async fn list_branches(repo_path: &Path, default_branch: &str) -> Result<Vec<BranchInfo>> {
        if !repo_path.exists() {
            return Err(TreeError::Git(format!(
                "Repository path does not exist: {:?}",
                repo_path
            )));
        }

        let output = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args([
                "for-each-ref",
                "--format=%(refname:short)%00%(objectname)%00%(authorname)%00%(authordate:iso-strict)%00%(subject)",
                "refs/heads/",
            ])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute git for-each-ref: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TreeError::Git(format!(
                "git for-each-ref failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut branches = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let commit_id = parts[1].to_string();
                let author = parts
                    .get(2)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let date = parts
                    .get(3)
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                let subject = parts
                    .get(4)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let is_default = name == default_branch;

                branches.push(BranchInfo {
                    name,
                    commit_id,
                    is_default,
                    commit_message: subject,
                    commit_author: author,
                    commit_date: date,
                });
            }
        }

        Ok(branches)
    }

    /// List all tags in the bare repository
    pub async fn list_tags(repo_path: &Path) -> Result<Vec<TagInfo>> {
        if !repo_path.exists() {
            return Err(TreeError::Git(format!(
                "Repository path does not exist: {:?}",
                repo_path
            )));
        }

        let output = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args([
                "for-each-ref",
                "--format=%(refname:short)%00%(objectname)%00%(taggername)%00%(taggerdate:iso-strict)%00%(subject)",
                "refs/tags/",
            ])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute git for-each-ref: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TreeError::Git(format!(
                "git for-each-ref tags failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tags = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let commit_id = parts[1].to_string();
                let tagger = parts
                    .get(2)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let date = parts
                    .get(3)
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                let message = parts
                    .get(4)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                tags.push(TagInfo {
                    name,
                    commit_id,
                    message,
                    tagger,
                    date,
                });
            }
        }

        Ok(tags)
    }

    /// List commits with pagination and optional revision/branch
    pub async fn list_commits(
        repo_path: &Path,
        revision: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CommitInfo>> {
        if !repo_path.exists() {
            return Err(TreeError::Git(format!(
                "Repository path does not exist: {:?}",
                repo_path
            )));
        }

        let rev = revision.unwrap_or("HEAD");

        // Check if revision exists first
        let check = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args(["rev-parse", "--verify", rev])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to check revision: {}", e)))?;

        if !check.status.success() {
            // Empty repository or non-existent revision
            return Ok(Vec::new());
        }

        let limit_str = limit.to_string();
        let skip_str = offset.to_string();

        let output = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args([
                "log",
                rev,
                "-n",
                &limit_str,
                "--skip",
                &skip_str,
                "--format=%H%x00%h%x00%an%x00%ae%x00%cn%x00%ce%x00%aI%x00%s%x00%b%x00%P%x1e",
            ])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute git log: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TreeError::Git(format!("git log failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for record in stdout.split('\x1e') {
            let record = record.trim();
            if record.is_empty() {
                continue;
            }
            let fields: Vec<&str> = record.split('\0').collect();
            if fields.len() >= 8 {
                let id = fields[0].to_string();
                let short_id = fields[1].to_string();
                let author_name = fields[2].to_string();
                let author_email = fields[3].to_string();
                let committer_name = fields[4].to_string();
                let committer_email = fields[5].to_string();
                let timestamp = DateTime::parse_from_rfc3339(fields[6])
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let summary = fields[7].to_string();
                let body = fields.get(8).unwrap_or(&"");
                let full_message = if body.trim().is_empty() {
                    summary.clone()
                } else {
                    format!("{}\n\n{}", summary, body.trim())
                };
                let parents = fields
                    .get(9)
                    .map(|p| p.split_whitespace().map(|s| s.to_string()).collect())
                    .unwrap_or_default();

                commits.push(CommitInfo {
                    id,
                    short_id,
                    author_name,
                    author_email,
                    committer_name,
                    committer_email,
                    message: full_message,
                    summary,
                    timestamp,
                    parents,
                });
            }
        }

        Ok(commits)
    }

    /// Get details of a single commit
    pub async fn get_commit(repo_path: &Path, commit_id: &str) -> Result<Option<CommitInfo>> {
        let commits = Self::list_commits(repo_path, Some(commit_id), 1, 0).await?;
        Ok(commits.into_iter().next())
    }

    /// List files and directories at a given path and revision
    pub async fn get_tree(
        repo_path: &Path,
        revision: Option<&str>,
        dir_path: Option<&str>,
    ) -> Result<Vec<FileEntry>> {
        let rev = revision.unwrap_or("HEAD");
        let path = dir_path.unwrap_or("");

        // Check if rev exists
        let check = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args(["rev-parse", "--verify", rev])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to verify revision: {}", e)))?;

        if !check.status.success() {
            return Ok(Vec::new());
        }

        let tree_spec = if path.is_empty() {
            format!("{}^{{tree}}", rev)
        } else {
            format!("{}:{}", rev, path.trim_matches('/'))
        };

        let output = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args(["ls-tree", "-l", &tree_spec])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute git ls-tree: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Format: <mode> <type> <object> <size>\t<file>
            // e.g.: 100644 blob 1234abcd5678    123\tREADME.md
            // or:   040000 tree 1234abcd5678      -\tsrc
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 2 {
                continue;
            }
            let filename = parts[1].to_string();
            let meta_parts: Vec<&str> = parts[0].split_whitespace().collect();
            if meta_parts.len() >= 3 {
                let mode_str = meta_parts[0];
                let obj_type = meta_parts[1];
                let size_str = meta_parts.get(3).unwrap_or(&"-");

                let mode = u32::from_str_radix(mode_str, 8).unwrap_or(0);
                let is_dir = obj_type == "tree";
                let size = size_str.parse::<u64>().unwrap_or(0);

                let entry_path = if path.is_empty() {
                    filename.clone()
                } else {
                    format!("{}/{}", path.trim_matches('/'), filename)
                };

                entries.push(FileEntry {
                    name: filename,
                    path: entry_path,
                    is_dir,
                    size,
                    mode,
                    last_commit: None,
                });
            }
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            if a.is_dir && !b.is_dir {
                std::cmp::Ordering::Less
            } else if !a.is_dir && b.is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(entries)
    }

    /// Read blob content
    pub async fn get_blob(
        repo_path: &Path,
        revision: Option<&str>,
        file_path: &str,
    ) -> Result<Option<FileContent>> {
        let rev = revision.unwrap_or("HEAD");
        let spec = format!("{}:{}", rev, file_path.trim_matches('/'));

        let output = Command::new("git")
            .arg("--git-dir")
            .arg(repo_path)
            .args(["cat-file", "-p", &spec])
            .output()
            .await
            .map_err(|e| TreeError::Git(format!("Failed to execute git cat-file: {}", e)))?;

        if !output.status.success() {
            return Ok(None);
        }

        let raw_bytes = output.stdout;
        let is_binary = raw_bytes.iter().take(8000).any(|&b| b == 0);

        let content = if is_binary {
            None
        } else {
            Some(String::from_utf8_lossy(&raw_bytes).to_string())
        };

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(file_path)
            .to_string();

        Ok(Some(FileContent {
            path: file_path.to_string(),
            name: file_name,
            size: raw_bytes.len() as u64,
            is_binary,
            content,
            commit: None,
        }))
    }

    /// Find README content if present in root
    pub async fn get_readme(repo_path: &Path, revision: Option<&str>) -> Result<Option<String>> {
        let entries = Self::get_tree(repo_path, revision, None).await?;
        for entry in entries {
            let lower = entry.name.to_lowercase();
            if lower == "readme.md" || lower == "readme" || lower == "readme.txt" {
                if let Some(blob) = Self::get_blob(repo_path, revision, &entry.name).await? {
                    return Ok(blob.content);
                }
            }
        }
        Ok(None)
    }
}
