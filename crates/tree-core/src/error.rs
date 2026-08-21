use thiserror::Error;

#[derive(Error, Debug)]
pub enum TreeError {
    #[error("Repository '{owner}/{name}' not found")]
    RepositoryNotFound { owner: String, name: String },

    #[error("User '{username}' not found")]
    UserNotFound { username: String },

    #[error("Organization '{name}' not found")]
    OrgNotFound { name: String },

    #[error("Repository '{owner}/{name}' already exists")]
    RepositoryAlreadyExists { owner: String, name: String },

    #[error("User '{username}' already exists")]
    UserAlreadyExists { username: String },

    #[error("Organization '{name}' already exists")]
    OrgAlreadyExists { name: String },

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Invalid repository name: '{0}' (must contain only alphanumeric, '-', '_', '.', and not end with .git)")]
    InvalidRepositoryName(String),

    #[error("Invalid user/org name: '{0}'")]
    InvalidName(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Storage/Database error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),
}

pub type Result<T> = std::result::Result<T, TreeError>;
