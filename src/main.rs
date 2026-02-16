use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
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
// Service: 'GitHub'
#[derive(Debug, Serialize, Deserialize)]
struct GitRepo<'a> {
    author: &'a str,
    repository: &'a str,
    service: GitService,
}

impl<'a> GitRepo<'a> {
    pub fn new(author: &'a str, repository: &'a str, service: GitService) -> Self {
        Self {
            author,
            repository,
            service,
        }
    }

    fn url(&self) -> String {
        format!("{}:{}/{}", self.service.ssh(), self.author, self.repository)
    }
}

impl<'a> Display for GitRepo<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url())
    }
}

struct GitClone<'a> {
    git_repo: &'a GitRepo<'a>,
}

impl<'a> GitClone<'a> {
    pub fn new(git_repo: &'a GitRepo<'a>) -> Self {
        Self { git_repo }
    }

    pub fn execute(&self, working_directory: &str) -> Output {
        let handle = Command::new("git")
            .args(&["clone", "--depth=1"])
            .arg(self.git_repo.url())
            .arg(format!(
                "{}/{}",
                working_directory, self.git_repo.repository
            ))
            .stdout(Stdio::piped())
            .spawn()
            .expect("can execute 'git' command");
        handle.wait_with_output().unwrap()
    }
}

struct RepositoryWatcher<'a> {
    git_repo: GitRepo<'a>,
    working_directory: PathBuf,
}

impl<'a> RepositoryWatcher<'a> {
    pub fn new(git_repo: GitRepo<'a>, working_directory: PathBuf) -> Self {
        Self {
            git_repo,
            working_directory,
        }
    }

    fn repo_dir(&self) -> PathBuf {
        self.working_directory.join(self.git_repo.repository)
    }

    fn do_clone(&self) {
        let clone = GitClone::new(&self.git_repo);
        clone.execute(self.working_directory.to_str().unwrap());
    }

    /// Whether there is a detected diff between the local and remote repositories.
    pub fn diff(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let status = Command::new("git")
            .args(&["-C", &self.repo_dir().to_string_lossy(), "fetch"])
            .stdout(Stdio::piped())
            .spawn()?
            .wait()?;

        if !status.success() {
            return Err(format!("unable to fetch {}", self.git_repo).into());
        }

        let local_output = Command::new("git")
            .args(&[
                "-C",
                &self.repo_dir().to_string_lossy(),
                "rev-parse",
                "HEAD",
            ])
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;

        let remote_output = Command::new("git")
            .args(&[
                "-C",
                &self.repo_dir().to_string_lossy(),
                "rev-parse",
                "@{upstream}",
            ])
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;

        Ok(local_output.stdout != remote_output.stdout)
    }

    pub fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("git")
            .args(&[
                "-C",
                &self.repo_dir().to_string_lossy(),
                "reset",
                "--hard",
                // TODO: none 'origin/main' remotes
                "origin/main",
            ])
            .stdout(Stdio::piped())
            .spawn()?
            .wait()?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Config<'a> {
    #[serde(borrow)]
    repo_details: HashMap<String, GitRepo<'a>>,
    working_directory: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();

    if args.len() != 2 {
        return Err(format!("USAGE: {} <CONFIG_PATH>", args[0]).into());
    }

    let config_path = args[1].clone();

    let config_data = &std::fs::read(&config_path)?;
    let config: Config<'_> = toml::from_slice(&config_data)?;

    eprintln!("Reading config from {config_path}");

    for (name, repo) in config.repo_details {
        let watcher_1 = RepositoryWatcher::new(
            GitRepo::new(repo.author, repo.repository, repo.service),
            config.working_directory.clone(),
        );

        if !watcher_1.repo_dir().exists() {
            watcher_1.do_clone();
        }

        if watcher_1.diff()? {
            eprintln!(
                "Diff detected for {name} ({}/{}), updating.",
                repo.author, repo.repository
            );
            watcher_1.update()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {}
