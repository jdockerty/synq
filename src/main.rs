use std::{
    fmt::Display,
    path::PathBuf,
    process::{Child, Command, Output},
};

#[derive(Debug, Default)]
enum GitService {
    #[default]
    Github,
    GitLab,
}

impl GitService {
    pub fn ssh(&self) -> String {
        match self {
            GitService::Github => "git@github.com".to_string(),
            GitService::GitLab => "git@gitlab.com".to_string(),
        }
    }
}

// A git repo contains an author, repository name and service.
//
// For example 'github.com/jdockerty/synq'.
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

    pub fn execute(&self) -> Output {
        let handle = Command::new("git")
            .args(&["clone", "--depth=1"])
            .arg(self.git_repo.url())
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
        std::fs::canonicalize(self.working_directory.join(self.git_repo.repository)).unwrap()
    }

    fn do_clone(&self) {
        let clone = GitClone::new(&self.git_repo);
        clone.execute();
    }

    /// Whether there is a detected diff between the local and remote repositories.
    pub fn diff(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let status = Command::new("git")
            .args(&["-C", &self.repo_dir().to_string_lossy(), "fetch"])
            .spawn()?
            .wait()?;
        if !status.success() {
            return Err(format!("unable to fetch {}", self.git_repo).into());
        }

        let local_output = Command::new("git")
            .args(&["-C", &self.repo_dir().to_string_lossy(), "rev-parse HEAD"])
            .spawn()?
            .wait_with_output()?;

        let remote_output = Command::new("git")
            .args(&[
                "-C",
                &self.repo_dir().to_string_lossy(),
                "rev-parse @{upstream}",
            ])
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
            .spawn()?
            .wait()?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watcher_1 = RepositoryWatcher::new(
        GitRepo::new("jdockerty", "gruglb", GitService::Github),
        "".into(),
    );

    if !PathBuf::from("clones").join(watcher_1.repo_dir()).exists() {
        eprintln!("Initial clone");
        println!("{:?}", watcher_1.repo_dir());
        watcher_1.do_clone();
    }

    if watcher_1.diff()? {
        watcher_1.update()?;
    }

    Ok(())
}

#[cfg(test)]
mod test {}
