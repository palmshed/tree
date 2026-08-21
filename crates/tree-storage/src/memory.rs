use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tree_core::error::{Result, TreeError};
use tree_core::models::{
    CreateOrgRequest, CreateRepositoryRequest, CreateUserRequest, Organization, OwnerType,
    PermissionType, Repository, RepositoryMember, RepositoryPermission, Role, User,
};
use tree_core::store::Store;
use uuid::Uuid;

#[derive(Default)]
struct MemoryState {
    users: HashMap<Uuid, User>,
    users_by_name: HashMap<String, Uuid>,
    users_by_email: HashMap<String, Uuid>,
    orgs: HashMap<Uuid, Organization>,
    orgs_by_name: HashMap<String, Uuid>,
    repositories: HashMap<Uuid, Repository>,
    repos_by_owner_and_name: HashMap<(String, String), Uuid>,
    members: HashMap<(Uuid, Uuid), RepositoryMember>, // (repo_id, user_id)
    permissions: HashMap<Uuid, RepositoryPermission>,
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    state: Arc<RwLock<MemoryState>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn create_user(&self, req: CreateUserRequest) -> Result<User> {
        let mut state = self.state.write();
        let username_lower = req.username.to_lowercase();
        let email_lower = req.email.to_lowercase();

        if state.users_by_name.contains_key(&username_lower) {
            return Err(TreeError::UserAlreadyExists {
                username: req.username,
            });
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        // The password field is a plaintext credential (same contract as PgStore);
        // it is hashed here with the canonical Argon2id implementation before
        // being stored.  When no password is provided, store an empty string:
        // this user cannot authenticate via password until a credential is set.
        let password_hash = req
            .password
            .as_deref()
            .map(tree_core::auth::hash_password)
            .unwrap_or_default();
        let user = User {
            id,
            username: req.username.clone(),
            email: req.email.clone(),
            password_hash,
            display_name: req.display_name,
            created_at: now,
            updated_at: now,
        };

        state.users.insert(id, user.clone());
        state.users_by_name.insert(username_lower, id);
        state.users_by_email.insert(email_lower, id);

        Ok(user)
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let state = self.state.read();
        Ok(state.users.get(&id).cloned())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let state = self.state.read();
        let username_lower = username.to_lowercase();
        Ok(state
            .users_by_name
            .get(&username_lower)
            .and_then(|id| state.users.get(id).cloned()))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let state = self.state.read();
        let email_lower = email.to_lowercase();
        Ok(state
            .users_by_email
            .get(&email_lower)
            .and_then(|id| state.users.get(id).cloned()))
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let state = self.state.read();
        let mut users: Vec<User> = state.users.values().cloned().collect();
        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    async fn create_org(&self, req: CreateOrgRequest) -> Result<Organization> {
        let mut state = self.state.write();
        let name_lower = req.name.to_lowercase();

        if state.orgs_by_name.contains_key(&name_lower) {
            return Err(TreeError::OrgAlreadyExists { name: req.name });
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let org = Organization {
            id,
            name: req.name.clone(),
            display_name: req.display_name,
            description: req.description,
            created_at: now,
            updated_at: now,
        };

        state.orgs.insert(id, org.clone());
        state.orgs_by_name.insert(name_lower, id);

        Ok(org)
    }

    async fn get_org_by_name(&self, name: &str) -> Result<Option<Organization>> {
        let state = self.state.read();
        let name_lower = name.to_lowercase();
        Ok(state
            .orgs_by_name
            .get(&name_lower)
            .and_then(|id| state.orgs.get(id).cloned()))
    }

    async fn list_orgs(&self) -> Result<Vec<Organization>> {
        let state = self.state.read();
        let mut orgs: Vec<Organization> = state.orgs.values().cloned().collect();
        orgs.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(orgs)
    }

    async fn create_repo(
        &self,
        owner_name: &str,
        req: CreateRepositoryRequest,
        disk_path: &str,
    ) -> Result<Repository> {
        let mut state = self.state.write();
        let owner_key = owner_name.to_lowercase();
        let name_key = req.name.to_lowercase();

        if state
            .repos_by_owner_and_name
            .contains_key(&(owner_key.clone(), name_key.clone()))
        {
            return Err(TreeError::RepositoryAlreadyExists {
                owner: owner_name.to_string(),
                name: req.name.clone(),
            });
        }

        let (owner_type, owner_id) = if let Some(uid) = state.users_by_name.get(&owner_key) {
            (OwnerType::User, *uid)
        } else if let Some(oid) = state.orgs_by_name.get(&owner_key) {
            (OwnerType::Organization, *oid)
        } else {
            let uid = Uuid::new_v4();
            let now = Utc::now();
            let user = User {
                id: uid,
                username: owner_name.to_string(),
                email: format!("{}@tree.local", owner_name),
                // Unparseable PHC placeholder: this implicitly-created owner has no
                // credential set and therefore can never authenticate until a real
                // Argon2id hash is stored.
                password_hash: String::new(),
                display_name: Some(owner_name.to_string()),
                created_at: now,
                updated_at: now,
            };
            state.users.insert(uid, user);
            state.users_by_name.insert(owner_key.clone(), uid);
            (OwnerType::User, uid)
        };

        let id = Uuid::new_v4();
        let now = Utc::now();
        let is_private = req.is_private.unwrap_or(false);
        let default_branch = req.default_branch.unwrap_or_else(|| "main".to_string());

        let repo = Repository {
            id,
            owner_type,
            owner_id,
            owner_name: owner_name.to_string(),
            name: req.name.clone(),
            description: req.description,
            is_private,
            default_branch,
            disk_path: disk_path.to_string(),
            created_at: now,
            updated_at: now,
        };

        state.repositories.insert(id, repo.clone());
        state
            .repos_by_owner_and_name
            .insert((owner_key, name_key), id);

        if owner_type == OwnerType::User {
            let member_id = Uuid::new_v4();
            let member = RepositoryMember {
                id: member_id,
                repository_id: id,
                user_id: owner_id,
                username: Some(owner_name.to_string()),
                role: Role::Owner,
                created_at: now,
            };
            state.members.insert((id, owner_id), member);
        }

        Ok(repo)
    }

    async fn get_repo(&self, owner: &str, name: &str) -> Result<Option<Repository>> {
        let state = self.state.read();
        let key = (owner.to_lowercase(), name.to_lowercase());
        Ok(state
            .repos_by_owner_and_name
            .get(&key)
            .and_then(|id| state.repositories.get(id).cloned()))
    }

    async fn get_repo_by_id(&self, id: Uuid) -> Result<Option<Repository>> {
        let state = self.state.read();
        Ok(state.repositories.get(&id).cloned())
    }

    async fn list_repos_by_owner(&self, owner: &str) -> Result<Vec<Repository>> {
        let state = self.state.read();
        let owner_lower = owner.to_lowercase();
        let mut repos: Vec<Repository> = state
            .repositories
            .values()
            .filter(|r| r.owner_name.to_lowercase() == owner_lower)
            .cloned()
            .collect();
        repos.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(repos)
    }

    async fn list_all_repos(&self) -> Result<Vec<Repository>> {
        let state = self.state.read();
        let mut repos: Vec<Repository> = state.repositories.values().cloned().collect();
        repos.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        Ok(repos)
    }

    async fn delete_repo(&self, owner: &str, name: &str) -> Result<()> {
        let mut state = self.state.write();
        let key = (owner.to_lowercase(), name.to_lowercase());
        if let Some(id) = state.repos_by_owner_and_name.remove(&key) {
            state.repositories.remove(&id);
            state.members.retain(|(repo_id, _), _| *repo_id != id);
            state.permissions.retain(|_, p| p.repository_id != id);
            Ok(())
        } else {
            Err(TreeError::RepositoryNotFound {
                owner: owner.to_string(),
                name: name.to_string(),
            })
        }
    }

    async fn add_or_update_member(
        &self,
        repo_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<RepositoryMember> {
        let mut state = self.state.write();
        let username = state.users.get(&user_id).map(|u| u.username.clone());
        let id = Uuid::new_v4();
        let now = Utc::now();

        let member = RepositoryMember {
            id,
            repository_id: repo_id,
            user_id,
            username,
            role,
            created_at: now,
        };

        state.members.insert((repo_id, user_id), member.clone());
        Ok(member)
    }

    async fn get_member(&self, repo_id: Uuid, user_id: Uuid) -> Result<Option<RepositoryMember>> {
        let state = self.state.read();
        Ok(state.members.get(&(repo_id, user_id)).cloned())
    }

    async fn list_members(&self, repo_id: Uuid) -> Result<Vec<RepositoryMember>> {
        let state = self.state.read();
        let members = state
            .members
            .iter()
            .filter(|((r_id, _), _)| *r_id == repo_id)
            .map(|(_, m)| m.clone())
            .collect();
        Ok(members)
    }

    async fn remove_member(&self, repo_id: Uuid, user_id: Uuid) -> Result<()> {
        let mut state = self.state.write();
        state.members.remove(&(repo_id, user_id));
        Ok(())
    }

    async fn add_permission(
        &self,
        repo_id: Uuid,
        user_id: Option<Uuid>,
        perm: &str,
        granted_by: Option<Uuid>,
    ) -> Result<RepositoryPermission> {
        let mut state = self.state.write();
        let id = Uuid::new_v4();
        let now = Utc::now();
        let perm_type = PermissionType::from_str(perm).map_err(TreeError::BadRequest)?;
        let username = user_id.and_then(|uid| state.users.get(&uid).map(|u| u.username.clone()));

        let permission = RepositoryPermission {
            id,
            repository_id: repo_id,
            user_id,
            username,
            permission_type: perm_type,
            granted_by,
            created_at: now,
        };

        state.permissions.insert(id, permission.clone());
        Ok(permission)
    }

    async fn list_permissions(&self, repo_id: Uuid) -> Result<Vec<RepositoryPermission>> {
        let state = self.state.read();
        let permissions = state
            .permissions
            .values()
            .filter(|p| p.repository_id == repo_id)
            .cloned()
            .collect();
        Ok(permissions)
    }
}
