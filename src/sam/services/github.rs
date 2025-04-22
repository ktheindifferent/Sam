use reqwest::{Client, Error, Response};
use serde::{Deserialize, Serialize};

pub struct GitHubClient {
    client: Client,
    token: String,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        GitHubClient {
            client: Client::new(),
            token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    pub async fn get_user(&self, username: &str) -> Result<GitHubUser, Error> {
        let url = format!("https://api.github.com/users/{}", username);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<GitHubUser>()
            .await?;
        Ok(resp)
    }

    pub async fn graphql_query<T: for<'de> Deserialize<'de>, V: Serialize>(
        &self,
        query: &str,
        variables: V,
    ) -> Result<T, Error> {
        let url = "https://api.github.com/graphql";
        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });
        let resp = self.client
            .post(url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .json(&body)
            .send()
            .await?
            .json::<T>()
            .await?;
        Ok(resp)
    }

    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<GitHubRepo, Error> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<GitHubRepo>()
            .await?;
        Ok(resp)
    }

    pub async fn list_repos_for_user(&self, username: &str) -> Result<Vec<GitHubRepo>, Error> {
        let url = format!("https://api.github.com/users/{}/repos", username);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<Vec<GitHubRepo>>()
            .await?;
        Ok(resp)
    }

    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: Option<&str>,
    ) -> Result<GitHubIssue, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
        let mut json_body = serde_json::json!({ "title": title });
        if let Some(b) = body {
            json_body["body"] = serde_json::Value::String(b.to_string());
        }
        let resp = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .json(&json_body)
            .send()
            .await?
            .json::<GitHubIssue>()
            .await?;
        Ok(resp)
    }

    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubIssue>, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<Vec<GitHubIssue>>()
            .await?;
        Ok(resp)
    }

    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<GitHubPullRequest, Error> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}",
            owner, repo, pr_number
        );
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<GitHubPullRequest>()
            .await?;
        Ok(resp)
    }

    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubPullRequest>, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<Vec<GitHubPullRequest>>()
            .await?;
        Ok(resp)
    }

    pub async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        commit_message: Option<&str>,
    ) -> Result<MergeResult, Error> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/merge",
            owner, repo, pr_number
        );
        let mut json_body = serde_json::Map::new();
        if let Some(msg) = commit_message {
            json_body.insert("commit_message".to_string(), serde_json::Value::String(msg.to_string()));
        }
        let resp = self.client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .json(&json_body)
            .send()
            .await?
            .json::<MergeResult>()
            .await?;
        Ok(resp)
    }

    pub async fn list_commits(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubCommit>, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/commits", owner, repo);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<Vec<GitHubCommit>>()
            .await?;
        Ok(resp)
    }

    pub async fn get_commit(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<GitHubCommit, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/commits/{}", owner, repo, sha);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<GitHubCommit>()
            .await?;
        Ok(resp)
    }

    pub async fn get_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<GitHubBranch, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/branches/{}", owner, repo, branch);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<GitHubBranch>()
            .await?;
        Ok(resp)
    }

    pub async fn list_branches(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubBranch>, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/branches", owner, repo);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<Vec<GitHubBranch>>()
            .await?;
        Ok(resp)
    }

    pub async fn get_release(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
    ) -> Result<GitHubRelease, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/releases/{}", owner, repo, release_id);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<GitHubRelease>()
            .await?;
        Ok(resp)
    }

    pub async fn list_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubRelease>, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
        let resp = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .json::<Vec<GitHubRelease>>()
            .await?;
        Ok(resp)
    }

    pub async fn create_release(
        &self,
        owner: &str,
        repo: &str,
        tag_name: &str,
        name: Option<&str>,
        body: Option<&str>,
        draft: bool,
        prerelease: bool,
    ) -> Result<GitHubRelease, Error> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
        let mut json_body = serde_json::json!({
            "tag_name": tag_name,
            "draft": draft,
            "prerelease": prerelease
        });
        if let Some(n) = name {
            json_body["name"] = serde_json::Value::String(n.to_string());
        }
        if let Some(b) = body {
            json_body["body"] = serde_json::Value::String(b.to_string());
        }
        let resp = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .json(&json_body)
            .send()
            .await?
            .json::<GitHubRelease>()
            .await?;
        Ok(resp)
    }

    pub async fn delete_repo(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<(), Error> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .header("User-Agent", "sam-github-client")
            .send()
            .await?
            .error_for_status()?; // Will return error if not 204
        Ok(())
    }

    // Add more methods for other endpoints as needed
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
    pub html_url: String,
    // Add more fields as needed
}

#[derive(Debug, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub private: bool,
    // Add more fields as needed
}

#[derive(Debug, Deserialize)]
pub struct GitHubIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub state: String,
    // Add more fields as needed
}

#[derive(Debug, Deserialize)]
pub struct GitHubPullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub state: String,
    // Add more fields as needed
}

#[derive(Debug, Deserialize)]
pub struct MergeResult {
    pub sha: String,
    pub merged: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubCommit {
    pub sha: String,
    pub commit: CommitDetails,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitDetails {
    pub message: String,
    pub author: CommitAuthor,
}

#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubBranch {
    pub name: String,
    pub commit: BranchCommit,
    // Add more fields as needed
}

#[derive(Debug, Deserialize)]
pub struct BranchCommit {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub html_url: String,
    // Add more fields as needed
}

// ...add more structs for other API responses as needed...
