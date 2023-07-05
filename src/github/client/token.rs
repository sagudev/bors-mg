use super::GitHubClient;
use crate::config::PAT;
use crate::github::API_ENDPOINT;

/// Provides access to GitHub API using PAT
pub struct TokenClient;

#[async_trait::async_trait(?Send)]
impl GitHubClient for TokenClient {
    fn is_available() -> bool {
        PAT.get().is_some()
    }

    async fn get<U: reqwest::IntoUrl>(&mut self, url: U) -> anyhow::Result<reqwest::Response> {
        reqwest::Client::new()
            .get(url)
            .bearer_auth(PAT.get().unwrap())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn post<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        reqwest::Client::new()
            .post(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn patch<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        reqwest::Client::new()
            .patch(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
