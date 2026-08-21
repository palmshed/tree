use crate::error::Result;
use crate::models::{
    CreateOrgRequest, CreateRepositoryRequest, CreateUserRequest, Organization, Repository,
    RepositoryMember, RepositoryPermission, Role, User,
};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait Store: Send + Sync + 'static {
    // User operations
    async fn create_user(&self, req: CreateUserRequest) -> Result<User>;
    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn list_users(&self) -> Result<Vec<User>>;

    // Organization operations
    async fn create_org(&self, req: CreateOrgRequest) -> Result<Organization>;
    async fn get_org_by_name(&self, name: &str) -> Result<Option<Organization>>;
    async fn list_orgs(&self) -> Result<Vec<Organization>>;

    // Repository operations
    async fn create_repo(
        &self,
        owner_name: &str,
        req: CreateRepositoryRequest,
        disk_path: &str,
    ) -> Result<Repository>;
    async fn get_repo(&self, owner: &str, name: &str) -> Result<Option<Repository>>;
    async fn get_repo_by_id(&self, id: Uuid) -> Result<Option<Repository>>;
    async fn list_repos_by_owner(&self, owner: &str) -> Result<Vec<Repository>>;
    async fn list_all_repos(&self) -> Result<Vec<Repository>>;
    async fn delete_repo(&self, owner: &str, name: &str) -> Result<()>;

    // Membership & Permissions
    async fn add_or_update_member(
        &self,
        repo_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<RepositoryMember>;
    async fn get_member(&self, repo_id: Uuid, user_id: Uuid) -> Result<Option<RepositoryMember>>;
    async fn list_members(&self, repo_id: Uuid) -> Result<Vec<RepositoryMember>>;
    async fn remove_member(&self, repo_id: Uuid, user_id: Uuid) -> Result<()>;

    async fn add_permission(
        &self,
        repo_id: Uuid,
        user_id: Option<Uuid>,
        perm: &str,
        granted_by: Option<Uuid>,
    ) -> Result<RepositoryPermission>;
    async fn list_permissions(&self, repo_id: Uuid) -> Result<Vec<RepositoryPermission>>;
}
