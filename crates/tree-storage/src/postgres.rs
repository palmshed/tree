use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use tracing::info;
use tree_core::auth::hash_password;
use tree_core::error::{Result, TreeError};
use tree_core::models::{
    CreateOrgRequest, CreateRepositoryRequest, CreateUserRequest, Organization, OwnerType,
    PermissionType, Repository, RepositoryMember, RepositoryPermission, Role, User,
};
use tree_core::store::Store;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await
            .map_err(|e| TreeError::Storage(format!("Failed to connect to PostgreSQL: {}", e)))?;

        let store = Self { pool };
        store.run_migrations().await?;
        Ok(store)
    }

    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running PostgreSQL database migrations...");

        let migration_sql = include_str!("../../../migrations/0001_init.sql");
        sqlx::raw_sql(migration_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| TreeError::Storage(format!("Migration failed: {}", e)))?;

        info!("PostgreSQL database migrations applied successfully.");
        Ok(())
    }
}

#[async_trait]
impl Store for PgStore {
    async fn create_user(&self, req: CreateUserRequest) -> Result<User> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let pass = req.password.as_deref().unwrap_or("password");
        let password_hash = hash_password(pass);

        let row = sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, display_name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, username, email, password_hash, display_name, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&req.display_name)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                TreeError::UserAlreadyExists {
                    username: req.username.clone(),
                }
            } else {
                TreeError::Storage(format!("Failed to create user: {}", e))
            }
        })?;

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            display_name: row.get("display_name"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, display_name, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get user by id: {}", e)))?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            username: r.get("username"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            display_name: r.get("display_name"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, display_name, created_at, updated_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get user by username: {}", e)))?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            username: r.get("username"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            display_name: r.get("display_name"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, display_name, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get user by email: {}", e)))?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            username: r.get("username"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            display_name: r.get("display_name"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, email, password_hash, display_name, created_at, updated_at FROM users ORDER BY username ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to list users: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: r.get("id"),
                username: r.get("username"),
                email: r.get("email"),
                password_hash: r.get("password_hash"),
                display_name: r.get("display_name"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    async fn create_org(&self, req: CreateOrgRequest) -> Result<Organization> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO organizations (id, name, display_name, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, display_name, description, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.display_name)
        .bind(&req.description)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                TreeError::OrgAlreadyExists {
                    name: req.name.clone(),
                }
            } else {
                TreeError::Storage(format!("Failed to create org: {}", e))
            }
        })?;

        Ok(Organization {
            id: row.get("id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn get_org_by_name(&self, name: &str) -> Result<Option<Organization>> {
        let row = sqlx::query(
            "SELECT id, name, display_name, description, created_at, updated_at FROM organizations WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get org by name: {}", e)))?;

        Ok(row.map(|r| Organization {
            id: r.get("id"),
            name: r.get("name"),
            display_name: r.get("display_name"),
            description: r.get("description"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn list_orgs(&self) -> Result<Vec<Organization>> {
        let rows = sqlx::query(
            "SELECT id, name, display_name, description, created_at, updated_at FROM organizations ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to list orgs: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| Organization {
                id: r.get("id"),
                name: r.get("name"),
                display_name: r.get("display_name"),
                description: r.get("description"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    async fn create_repo(
        &self,
        owner_name: &str,
        req: CreateRepositoryRequest,
        disk_path: &str,
    ) -> Result<Repository> {
        let user_opt = self.get_user_by_username(owner_name).await?;
        let (owner_type, owner_id) = if let Some(u) = user_opt {
            (OwnerType::User, u.id)
        } else if let Some(o) = self.get_org_by_name(owner_name).await? {
            (OwnerType::Organization, o.id)
        } else {
            // Auto-create user if not exists for quick local onboarding
            let user = self
                .create_user(CreateUserRequest {
                    username: owner_name.to_string(),
                    email: format!("{}@tree.local", owner_name),
                    password: Some("password".into()),
                    display_name: Some(owner_name.to_string()),
                })
                .await?;
            (OwnerType::User, user.id)
        };

        let id = Uuid::new_v4();
        let now = Utc::now();
        let is_private = req.is_private.unwrap_or(false);
        let default_branch = req.default_branch.unwrap_or_else(|| "main".to_string());

        let row = sqlx::query(
            r#"
            INSERT INTO repositories (id, owner_type, owner_id, owner_name, name, description, is_private, default_branch, disk_path, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, owner_type, owner_id, owner_name, name, description, is_private, default_branch, disk_path, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(owner_type.to_string())
        .bind(owner_id)
        .bind(owner_name)
        .bind(&req.name)
        .bind(&req.description)
        .bind(is_private)
        .bind(&default_branch)
        .bind(disk_path)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                TreeError::RepositoryAlreadyExists {
                    owner: owner_name.to_string(),
                    name: req.name.clone(),
                }
            } else {
                TreeError::Storage(format!("Failed to create repository: {}", e))
            }
        })?;

        let owner_type_str: String = row.get("owner_type");
        let parsed_owner_type = OwnerType::from_str(&owner_type_str).unwrap_or(OwnerType::User);

        // Add owner as repository member with Owner role
        if parsed_owner_type == OwnerType::User {
            let _ = self.add_or_update_member(id, owner_id, Role::Owner).await;
        }

        Ok(Repository {
            id: row.get("id"),
            owner_type: parsed_owner_type,
            owner_id: row.get("owner_id"),
            owner_name: row.get("owner_name"),
            name: row.get("name"),
            description: row.get("description"),
            is_private: row.get("is_private"),
            default_branch: row.get("default_branch"),
            disk_path: row.get("disk_path"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn get_repo(&self, owner: &str, name: &str) -> Result<Option<Repository>> {
        let row = sqlx::query(
            r#"
            SELECT id, owner_type, owner_id, owner_name, name, description, is_private, default_branch, disk_path, created_at, updated_at
            FROM repositories
            WHERE owner_name = $1 AND name = $2
            "#,
        )
        .bind(owner)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get repository: {}", e)))?;

        Ok(row.map(|r| {
            let owner_type_str: String = r.get("owner_type");
            Repository {
                id: r.get("id"),
                owner_type: OwnerType::from_str(&owner_type_str).unwrap_or(OwnerType::User),
                owner_id: r.get("owner_id"),
                owner_name: r.get("owner_name"),
                name: r.get("name"),
                description: r.get("description"),
                is_private: r.get("is_private"),
                default_branch: r.get("default_branch"),
                disk_path: r.get("disk_path"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }
        }))
    }

    async fn get_repo_by_id(&self, id: Uuid) -> Result<Option<Repository>> {
        let row = sqlx::query(
            r#"
            SELECT id, owner_type, owner_id, owner_name, name, description, is_private, default_branch, disk_path, created_at, updated_at
            FROM repositories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get repository by id: {}", e)))?;

        Ok(row.map(|r| {
            let owner_type_str: String = r.get("owner_type");
            Repository {
                id: r.get("id"),
                owner_type: OwnerType::from_str(&owner_type_str).unwrap_or(OwnerType::User),
                owner_id: r.get("owner_id"),
                owner_name: r.get("owner_name"),
                name: r.get("name"),
                description: r.get("description"),
                is_private: r.get("is_private"),
                default_branch: r.get("default_branch"),
                disk_path: r.get("disk_path"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }
        }))
    }

    async fn list_repos_by_owner(&self, owner: &str) -> Result<Vec<Repository>> {
        let rows = sqlx::query(
            r#"
            SELECT id, owner_type, owner_id, owner_name, name, description, is_private, default_branch, disk_path, created_at, updated_at
            FROM repositories
            WHERE owner_name = $1
            ORDER BY name ASC
            "#,
        )
        .bind(owner)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to list repos by owner: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let owner_type_str: String = r.get("owner_type");
                Repository {
                    id: r.get("id"),
                    owner_type: OwnerType::from_str(&owner_type_str).unwrap_or(OwnerType::User),
                    owner_id: r.get("owner_id"),
                    owner_name: r.get("owner_name"),
                    name: r.get("name"),
                    description: r.get("description"),
                    is_private: r.get("is_private"),
                    default_branch: r.get("default_branch"),
                    disk_path: r.get("disk_path"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                }
            })
            .collect())
    }

    async fn list_all_repos(&self) -> Result<Vec<Repository>> {
        let rows = sqlx::query(
            r#"
            SELECT id, owner_type, owner_id, owner_name, name, description, is_private, default_branch, disk_path, created_at, updated_at
            FROM repositories
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to list all repos: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let owner_type_str: String = r.get("owner_type");
                Repository {
                    id: r.get("id"),
                    owner_type: OwnerType::from_str(&owner_type_str).unwrap_or(OwnerType::User),
                    owner_id: r.get("owner_id"),
                    owner_name: r.get("owner_name"),
                    name: r.get("name"),
                    description: r.get("description"),
                    is_private: r.get("is_private"),
                    default_branch: r.get("default_branch"),
                    disk_path: r.get("disk_path"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                }
            })
            .collect())
    }

    async fn delete_repo(&self, owner: &str, name: &str) -> Result<()> {
        let res = sqlx::query("DELETE FROM repositories WHERE owner_name = $1 AND name = $2")
            .bind(owner)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| TreeError::Storage(format!("Failed to delete repository: {}", e)))?;

        if res.rows_affected() == 0 {
            return Err(TreeError::RepositoryNotFound {
                owner: owner.to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    async fn add_or_update_member(
        &self,
        repo_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<RepositoryMember> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO repository_members (id, repository_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (repository_id, user_id)
            DO UPDATE SET role = EXCLUDED.role
            RETURNING id, repository_id, user_id, role, created_at
            "#,
        )
        .bind(id)
        .bind(repo_id)
        .bind(user_id)
        .bind(role.to_string())
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to add member: {}", e)))?;

        let role_str: String = row.get("role");
        let parsed_role = Role::from_str(&role_str).unwrap_or(Role::Read);

        Ok(RepositoryMember {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            user_id: row.get("user_id"),
            username: None,
            role: parsed_role,
            created_at: row.get("created_at"),
        })
    }

    async fn get_member(&self, repo_id: Uuid, user_id: Uuid) -> Result<Option<RepositoryMember>> {
        let row = sqlx::query(
            r#"
            SELECT m.id, m.repository_id, m.user_id, m.role, m.created_at, u.username
            FROM repository_members m
            LEFT JOIN users u ON u.id = m.user_id
            WHERE m.repository_id = $1 AND m.user_id = $2
            "#,
        )
        .bind(repo_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to get member: {}", e)))?;

        Ok(row.map(|r| {
            let role_str: String = r.get("role");
            RepositoryMember {
                id: r.get("id"),
                repository_id: r.get("repository_id"),
                user_id: r.get("user_id"),
                username: r.get("username"),
                role: Role::from_str(&role_str).unwrap_or(Role::Read),
                created_at: r.get("created_at"),
            }
        }))
    }

    async fn list_members(&self, repo_id: Uuid) -> Result<Vec<RepositoryMember>> {
        let rows = sqlx::query(
            r#"
            SELECT m.id, m.repository_id, m.user_id, m.role, m.created_at, u.username
            FROM repository_members m
            LEFT JOIN users u ON u.id = m.user_id
            WHERE m.repository_id = $1
            ORDER BY m.created_at ASC
            "#,
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to list members: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let role_str: String = r.get("role");
                RepositoryMember {
                    id: r.get("id"),
                    repository_id: r.get("repository_id"),
                    user_id: r.get("user_id"),
                    username: r.get("username"),
                    role: Role::from_str(&role_str).unwrap_or(Role::Read),
                    created_at: r.get("created_at"),
                }
            })
            .collect())
    }

    async fn remove_member(&self, repo_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM repository_members WHERE repository_id = $1 AND user_id = $2")
            .bind(repo_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| TreeError::Storage(format!("Failed to remove member: {}", e)))?;
        Ok(())
    }

    async fn add_permission(
        &self,
        repo_id: Uuid,
        user_id: Option<Uuid>,
        perm: &str,
        granted_by: Option<Uuid>,
    ) -> Result<RepositoryPermission> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let perm_type = PermissionType::from_str(perm).map_err(TreeError::BadRequest)?;

        let row = sqlx::query(
            r#"
            INSERT INTO repository_permissions (id, repository_id, user_id, permission_type, granted_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, repository_id, user_id, permission_type, granted_by, created_at
            "#,
        )
        .bind(id)
        .bind(repo_id)
        .bind(user_id)
        .bind(perm_type.to_string())
        .bind(granted_by)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to add permission: {}", e)))?;

        Ok(RepositoryPermission {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            user_id: row.get("user_id"),
            username: None,
            permission_type: perm_type,
            granted_by: row.get("granted_by"),
            created_at: row.get("created_at"),
        })
    }

    async fn list_permissions(&self, repo_id: Uuid) -> Result<Vec<RepositoryPermission>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.repository_id, p.user_id, p.permission_type, p.granted_by, p.created_at, u.username
            FROM repository_permissions p
            LEFT JOIN users u ON u.id = p.user_id
            WHERE p.repository_id = $1
            ORDER BY p.created_at ASC
            "#,
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TreeError::Storage(format!("Failed to list permissions: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let perm_str: String = r.get("permission_type");
                RepositoryPermission {
                    id: r.get("id"),
                    repository_id: r.get("repository_id"),
                    user_id: r.get("user_id"),
                    username: r.get("username"),
                    permission_type: PermissionType::from_str(&perm_str)
                        .unwrap_or(PermissionType::Read),
                    granted_by: r.get("granted_by"),
                    created_at: r.get("created_at"),
                }
            })
            .collect())
    }
}
