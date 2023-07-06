use super::GitHubClient;
use crate::config::{CMD_PREFIX, PAT};
use crate::github::{GithubRepo, API_ENDPOINT};

/// Provides access to GitHub API using PAT
pub struct TokenClient;

#[async_trait::async_trait(?Send)]
impl GitHubClient for TokenClient {
    fn is_available() -> bool {
        PAT.get().is_some()
    }

    async fn get(&mut self, end: &str) -> anyhow::Result<reqwest::Response> {
        tracing::debug!("Bearer {}", PAT.get().unwrap());
        reqwest::Client::new()
            .get(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", CMD_PREFIX.get().unwrap())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn post<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        let req = reqwest::Client::new()
            .post(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", CMD_PREFIX.get().unwrap())
            .json(data);
        tracing::debug!("Reqq: {:#?}", req);
        req.send().await.map_err(|e| anyhow::anyhow!(e))
    }

    async fn patch<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        tracing::debug!("Bearer {}", PAT.get().unwrap());
        reqwest::Client::new()
            .patch(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", CMD_PREFIX.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
