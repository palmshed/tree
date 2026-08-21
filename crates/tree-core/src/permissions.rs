use crate::error::{Result, TreeError};
use crate::models::{Repository, RepositoryMember, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Read,
    Write,
    Admin,
    Delete,
}

pub struct PermissionEngine;

impl PermissionEngine {
    /// Evaluates whether an optional user can perform an action on a repository.
    pub fn check_permission(
        repo: &Repository,
        user: Option<&User>,
        member: Option<&RepositoryMember>,
        action: Action,
    ) -> Result<()> {
        // Public repositories allow read access to everyone
        if !repo.is_private && action == Action::Read {
            return Ok(());
        }

        // All other actions (or private repo read) require authentication
        let user = match user {
            Some(u) => u,
            None => {
                return Err(TreeError::Unauthorized(
                    "Authentication required to access this repository".into(),
                ));
            }
        };

        // If the user is the repository owner (for user-owned repo)
        if repo.owner_type == crate::models::OwnerType::User && repo.owner_id == user.id {
            return Ok(());
        }

        // Check member role
        if let Some(m) = member {
            if m.user_id == user.id {
                match action {
                    Action::Read => {
                        if m.role.can_read() {
                            return Ok(());
                        }
                    }
                    Action::Write => {
                        if m.role.can_write() {
                            return Ok(());
                        }
                    }
                    Action::Admin => {
                        if m.role.can_admin() {
                            return Ok(());
                        }
                    }
                    Action::Delete => {
                        if m.role.is_owner() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(TreeError::Forbidden(format!(
            "User '{}' does not have {:?} permission on repository '{}/{}'",
            user.username, action, repo.owner_name, repo.name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OwnerType, Role};
    use chrono::Utc;
    use uuid::Uuid;

    fn dummy_user(username: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: format!("{}@example.com", username),
            password_hash: "hash".to_string(),
            display_name: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn dummy_repo(owner_user: &User, is_private: bool) -> Repository {
        Repository {
            id: Uuid::new_v4(),
            owner_type: OwnerType::User,
            owner_id: owner_user.id,
            owner_name: owner_user.username.clone(),
            name: "test-repo".to_string(),
            description: None,
            is_private,
            default_branch: "main".to_string(),
            disk_path: "/tmp/test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_public_repo_anonymous_read() {
        let owner = dummy_user("alice");
        let repo = dummy_repo(&owner, false);
        assert!(PermissionEngine::check_permission(&repo, None, None, Action::Read).is_ok());
        assert!(PermissionEngine::check_permission(&repo, None, None, Action::Write).is_err());
    }

    #[test]
    fn test_private_repo_anonymous_denied() {
        let owner = dummy_user("alice");
        let repo = dummy_repo(&owner, true);
        assert!(PermissionEngine::check_permission(&repo, None, None, Action::Read).is_err());
    }

    #[test]
    fn test_owner_full_access() {
        let owner = dummy_user("alice");
        let repo = dummy_repo(&owner, true);
        assert!(PermissionEngine::check_permission(&repo, Some(&owner), None, Action::Read).is_ok());
        assert!(PermissionEngine::check_permission(&repo, Some(&owner), None, Action::Write).is_ok());
        assert!(PermissionEngine::check_permission(&repo, Some(&owner), None, Action::Admin).is_ok());
        assert!(PermissionEngine::check_permission(&repo, Some(&owner), None, Action::Delete).is_ok());
    }

    #[test]
    fn test_member_role_permissions() {
        let owner = dummy_user("alice");
        let bob = dummy_user("bob");
        let repo = dummy_repo(&owner, true);

        let read_member = RepositoryMember {
            id: Uuid::new_v4(),
            repository_id: repo.id,
            user_id: bob.id,
            username: Some(bob.username.clone()),
            role: Role::Read,
            created_at: Utc::now(),
        };

        assert!(PermissionEngine::check_permission(&repo, Some(&bob), Some(&read_member), Action::Read).is_ok());
        assert!(PermissionEngine::check_permission(&repo, Some(&bob), Some(&read_member), Action::Write).is_err());

        let write_member = RepositoryMember {
            id: Uuid::new_v4(),
            repository_id: repo.id,
            user_id: bob.id,
            username: Some(bob.username.clone()),
            role: Role::Write,
            created_at: Utc::now(),
        };

        assert!(PermissionEngine::check_permission(&repo, Some(&bob), Some(&write_member), Action::Write).is_ok());
        assert!(PermissionEngine::check_permission(&repo, Some(&bob), Some(&write_member), Action::Admin).is_err());
    }
}
