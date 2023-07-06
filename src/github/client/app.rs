use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{Algorithm, Header};

use super::GitHubClient;
use crate::config::{APP_ID, CMD_PREFIX, PRIVATE_KEY};
use crate::github::API_ENDPOINT;
use crate::models::AppId;

/// Provides access to a single app installation (repository) using the GitHub API.
pub struct AppClient(String);

impl AppClient {
    /// Create a JSON Web Token that can be used to authenticate an a GitHub application.
    ///
    /// See: https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#authenticating-as-a-github-app
    fn generate_bearer_token(app_id: AppId, private_key: &str) -> Result<String> {
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key.as_bytes())
            .context("Could not encode private key")?;

        #[derive(serde::Serialize)]
        struct Claims {
            iss: AppId,
            iat: usize,
            exp: usize,
        }

        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Time went backwards")?
            .as_secs() as usize;

        // Github only allows JWTs that expire in the next 10 minutes.
        // The token is issued 60 seconds in the past and expires in 9 minutes,
        // to allow some clock drift.
        let claims = Claims {
            iss: app_id,
            iat: now - 60,       //drift
            exp: now + (9 * 60), //10min expire
        };

        let header = Header::new(Algorithm::RS256);

        Ok(jsonwebtoken::encode(&header, &claims, &key)?)
    }

    pub fn new() -> Result<Self> {
        if Self::is_available() {
            return Err(anyhow!("APP NOt available!"));
        }
        Ok(Self(Self::generate_bearer_token(
            AppId(APP_ID.get().unwrap().parse()?),
            PRIVATE_KEY.get().unwrap(),
        )?))
    }
}

#[async_trait::async_trait(?Send)]
impl GitHubClient for AppClient {
    fn is_available() -> bool {
        APP_ID.get().is_some() && PRIVATE_KEY.get().is_some()
    }

    async fn get(&mut self, end: &str) -> Result<reqwest::Response> {
        reqwest::Client::new()
            .get(API_ENDPOINT.to_owned() + end)
            .bearer_auth(&self.0)
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
    ) -> Result<reqwest::Response> {
        reqwest::Client::new()
            .post(API_ENDPOINT.to_owned() + end)
            .bearer_auth(&self.0)
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", CMD_PREFIX.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn patch<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> Result<reqwest::Response> {
        reqwest::Client::new()
            .patch(API_ENDPOINT.to_owned() + end)
            .bearer_auth(&self.0)
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", CMD_PREFIX.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
