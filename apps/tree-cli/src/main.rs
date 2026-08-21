use clap::{Parser, Subcommand};
use reqwest::Client;
use tree_core::models::{
    CreateRepositoryRequest, CreateUserRequest, Repository, RepositorySummary,
};

#[derive(Parser, Debug)]
#[command(
    name = "tree",
    about = "Tree - Minimalist Git Hosting Platform CLI",
    version = "0.1.0"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        default_value = "http://localhost:8080",
        env = "TREE_SERVER_URL"
    )]
    server: String,

    #[arg(long, global = true, env = "TREE_AUTH_USER")]
    auth_user: Option<String>,

    #[arg(long, global = true, env = "TREE_AUTH_PASS")]
    auth_pass: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new Git repository
    Create {
        /// Repository name
        name: String,

        /// Repository owner (defaults to 'user')
        #[arg(short, long, default_value = "user")]
        owner: String,

        /// Repository description
        #[arg(short, long)]
        description: Option<String>,

        /// Make repository private
        #[arg(long)]
        private: bool,

        /// Default branch name (default: main)
        #[arg(long, default_value = "main")]
        default_branch: String,
    },

    /// Delete a Git repository
    Delete {
        /// Repository in 'owner/name' or 'name' format
        repo: String,
    },

    /// List repositories
    List {
        /// Filter by owner
        #[arg(short, long)]
        owner: Option<String>,
    },

    /// Show repository details
    Show {
        /// Repository in 'owner/name' format
        repo: String,
    },

    /// User management
    User {
        #[command(subcommand)]
        cmd: UserCommands,
    },
}

#[derive(Subcommand, Debug)]
enum UserCommands {
    /// Create a new user
    Create {
        username: String,
        email: String,
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long, default_value = "password")]
        password: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = Client::new();
    let server_url = cli.server.trim_end_matches('/');

    match cli.command {
        Commands::Create {
            name,
            owner,
            description,
            private,
            default_branch,
        } => {
            let req = CreateRepositoryRequest {
                owner: Some(owner.clone()),
                name: name.clone(),
                description,
                is_private: Some(private),
                default_branch: Some(default_branch),
            };

            let mut builder = client
                .post(format!("{}/repositories", server_url))
                .json(&req);

            if let (Some(u), Some(p)) = (&cli.auth_user, &cli.auth_pass) {
                builder = builder.basic_auth(u, Some(p));
            }

            let resp = builder.send().await?;
            if resp.status().is_success() {
                let repo: Repository = resp.json().await?;
                println!("✓ Repository created successfully!");
                println!("  Name:        {}/{}", repo.owner_name, repo.name);
                println!("  Default Ref: {}", repo.default_branch);
                println!(
                    "  Visibility:  {}",
                    if repo.is_private { "private" } else { "public" }
                );
                println!(
                    "  Clone URL:   {}/{}/{}.git",
                    server_url, repo.owner_name, repo.name
                );
            } else {
                let err_text = resp.text().await?;
                eprintln!("✗ Error creating repository: {}", err_text);
                std::process::exit(1);
            }
        }

        Commands::Delete { repo } => {
            let (owner, name) = parse_owner_repo(&repo);
            let mut builder =
                client.delete(format!("{}/repositories/{}/{}", server_url, owner, name));

            if let (Some(u), Some(p)) = (&cli.auth_user, &cli.auth_pass) {
                builder = builder.basic_auth(u, Some(p));
            }

            let resp = builder.send().await?;
            if resp.status().is_success() {
                println!("✓ Repository '{}/{}' deleted successfully.", owner, name);
            } else {
                let err_text = resp.text().await?;
                eprintln!("✗ Error deleting repository: {}", err_text);
                std::process::exit(1);
            }
        }

        Commands::List { owner } => {
            let mut url = format!("{}/repositories", server_url);
            if let Some(o) = owner {
                url.push_str(&format!("?owner={}", o));
            }

            let resp = client.get(&url).send().await?;
            if resp.status().is_success() {
                let repos: Vec<Repository> = resp.json().await?;
                if repos.is_empty() {
                    println!("No repositories found.");
                } else {
                    #[allow(clippy::print_literal)]
                    {
                        println!(
                            "{:<30} {:<10} {:<10} {}",
                            "REPOSITORY", "BRANCH", "ACCESS", "DESCRIPTION"
                        );
                    }
                    println!("{}", "-".repeat(70));
                    for r in repos {
                        let access = if r.is_private { "private" } else { "public" };
                        let desc = r.description.as_deref().unwrap_or("-");
                        let full_name = format!("{}/{}", r.owner_name, r.name);
                        println!(
                            "{:<30} {:<10} {:<10} {}",
                            full_name, r.default_branch, access, desc
                        );
                    }
                }
            } else {
                let err_text = resp.text().await?;
                eprintln!("✗ Error listing repositories: {}", err_text);
                std::process::exit(1);
            }
        }

        Commands::Show { repo } => {
            let (owner, name) = parse_owner_repo(&repo);
            let resp = client
                .get(format!("{}/repositories/{}/{}", server_url, owner, name))
                .send()
                .await?;

            if resp.status().is_success() {
                let summary: RepositorySummary = resp.json().await?;
                println!(
                    "Repository: {}/{}",
                    summary.repository.owner_name, summary.repository.name
                );
                println!("Default Branch: {}", summary.default_branch);
                println!("Commits:        {}", summary.commits_count);
                println!("Branches:       {}", summary.branches_count);
                println!("Tags:           {}", summary.tags_count);
                println!("HTTP Clone:     {}", summary.clone_url_http);
                println!("SSH Clone:      {}", summary.clone_url_ssh);
                if let Some(desc) = summary.repository.description {
                    println!("Description:    {}", desc);
                }
            } else {
                let err_text = resp.text().await?;
                eprintln!("✗ Error getting repository: {}", err_text);
                std::process::exit(1);
            }
        }

        Commands::User { cmd } => match cmd {
            UserCommands::Create {
                username,
                email,
                display_name,
                password,
            } => {
                let req = CreateUserRequest {
                    username,
                    email,
                    display_name,
                    password: Some(password),
                };

                let resp = client
                    .post(format!("{}/users", server_url))
                    .json(&req)
                    .send()
                    .await?;

                if resp.status().is_success() {
                    let user: serde_json::Value = resp.json().await?;
                    println!("✓ User created: {}", user["username"]);
                } else {
                    let err_text = resp.text().await?;
                    eprintln!("✗ Error creating user: {}", err_text);
                    std::process::exit(1);
                }
            }
        },
    }

    Ok(())
}

fn parse_owner_repo(input: &str) -> (String, String) {
    if let Some((owner, name)) = input.split_once('/') {
        (owner.to_string(), name.to_string())
    } else {
        ("user".to_string(), input.to_string())
    }
}
