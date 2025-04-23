use std::path::Path;
use git2::{Repository, Error, Oid, Commit};

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let repo = Repository::open(path)?;
        Ok(GitRepo { repo })
    }

    pub fn latest_commit(&self) -> Result<Commit, Error> {
        let head = self.repo.head()?;
        let oid = head.target().ok_or_else(|| Error::from_str("No HEAD target"))?;
        self.repo.find_commit(oid)
    }

    pub fn list_branches(&self) -> Result<Vec<String>, Error> {
        let mut branches = Vec::new();
        for branch in self.repo.branches(None)? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                branches.push(name.to_string());
            }
        }
        Ok(branches)
    }
}

// Add this to your Cargo.toml:
// git2 = "0.18"