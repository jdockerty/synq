use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use anyhow::{Result, ensure};
use run_script::run_script;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum GitService {
    #[default]
    GitHub,
    GitLab,
}

impl GitService {
    pub fn ssh(&self) -> String {
        match self {
            GitService::GitHub => "git@github.com".to_string(),
            GitService::GitLab => "git@gitlab.com".to_string(),
        }
    }
}

// A git repo contains an author, repository name, and service.
//
// For example 'github.com/jdockerty/synq', equates to:
//
// Author: 'jdockerty'
// Repository: 'synq'
// Service: 'github'
#[derive(Debug, Serialize, Deserialize)]
struct GitRepo {
    author: String,
    repository: String,
    service: GitService,
    post_sync: Option<String>,
}

impl GitRepo {
    pub fn new(
        author: String,
        repository: String,
        service: GitService,
        post_sync: Option<String>,
    ) -> Self {
        Self {
            author,
            repository,
            service,
            post_sync,
        }
    }

    fn url(&self) -> String {
        format!("{}:{}/{}", self.service.ssh(), self.author, self.repository)
    }
}

impl Display for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url())
    }
}

struct GitClone<'a> {
    git_repo: &'a GitRepo,
}

fn git_cmd(args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("can execute 'git' command")
        .wait_with_output()
        .unwrap()
}

impl<'a> GitClone<'a> {
    pub fn new(git_repo: &'a GitRepo) -> Self {
        Self { git_repo }
    }

    pub fn execute(&self, working_directory: &str) -> Output {
        git_cmd(&[
            "clone",
            "--depth=1",
            &self.git_repo.url(),
            &format!("{}/{}", working_directory, self.git_repo.repository),
        ])
    }
}

struct RepositoryWatcher {
    git_repo: GitRepo,
    working_directory: PathBuf,
}

impl RepositoryWatcher {
    pub fn new(git_repo: GitRepo, working_directory: PathBuf) -> Self {
        Self {
            git_repo,
            working_directory,
        }
    }

    /// Run a post sync action. If no action
    /// is provided, then this is a no-op.
    fn run_post_sync(&self) -> Result<()> {
        if let Some(post_sync) = &self.git_repo.post_sync {
            let (exit_code, output, error) = run_script!(post_sync)?;
            ensure!(
                exit_code == 0,
                format!(
                    "unable to run post sync script for {}/{}: {error}",
                    self.git_repo.author, self.git_repo.repository
                )
            );
            eprintln!("{output}");
        }

        Ok(())
    }

    fn repo_dir(&self) -> PathBuf {
        self.working_directory.join(&self.git_repo.repository)
    }

    fn do_clone(&self) {
        let clone = GitClone::new(&self.git_repo);
        clone.execute(self.working_directory.to_str().unwrap());
    }

    /// Whether there is a detected diff between the local and remote repositories.
    pub fn diff(&self) -> Result<bool> {
        let repo_dir = self.repo_dir().to_string_lossy().to_string();

        let fetch_output = git_cmd(&["-C", &repo_dir, "fetch"]);
        ensure!(
            fetch_output.status.success(),
            format!("unable to fetch {}", self.git_repo)
        );

        let local_output = git_cmd(&["-C", &repo_dir, "rev-parse", "HEAD"]);
        let remote_output = git_cmd(&["-C", &repo_dir, "rev-parse", "@{upstream}"]);

        Ok(local_output.stdout != remote_output.stdout)
    }

    pub fn update(&self) {
        let repo_dir = self.repo_dir().to_string_lossy().to_string();

        git_cmd(&[
            "-C",
            &repo_dir,
            "reset",
            "--hard",
            // TODO: non-'origin/main' remotes
            "origin/main",
        ]);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    repo_details: HashMap<String, GitRepo>,
    working_directory: PathBuf,
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    ensure!(args.len() == 2, format!("USAGE: {} <CONFIG_PATH>", args[0]));

    let config_path = args[1].clone();

    let config_data = &std::fs::read(&config_path)?;
    let config: Config = toml::from_slice(config_data)?;

    eprintln!("Reading config from {config_path}");

    for (name, repo) in config.repo_details {
        let watcher_1 = RepositoryWatcher::new(
            GitRepo::new(
                repo.author.clone(),
                repo.repository.clone(),
                repo.service,
                repo.post_sync,
            ),
            config.working_directory.clone(),
        );

        if !watcher_1.repo_dir().exists() {
            eprintln!("Cloning {}/{}", repo.author, repo.repository);
            watcher_1.do_clone();
            // First clone will have the latest info, so we can
            // skip some unnecessary work on checking diffs
            continue;
        }

        if watcher_1.diff()? {
            eprintln!(
                "Diff detected for {name} ({}/{}), updating.",
                repo.author, repo.repository
            );
            watcher_1.update();
            watcher_1.run_post_sync()?;
        } else {
            eprintln!(
                "No updates required for {}/{}",
                repo.author, repo.repository
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {}
