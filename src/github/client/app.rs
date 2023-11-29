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
        if !Self::is_available() {
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

#[tokio::test]
async fn pong() {
    APP_ID.set("355259".to_string()).unwrap();
    PRIVATE_KEY
        .set(
            "-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAwjn8AcClx5lL/AfEGMUjpKwifIF5jkbepmFiMHTNjtHIegy6
O7xCgo2EjgnNDxs1P8ovJ46jM/kOVQbbrmiaU1iFgRPwBlTEkTxv3LxVnIYYBeAo
3BZT7mltbCUXyFB93RO8NQBbKXXJDrk4pG1rwO6eUwbLk/mT2Y0SJ7ldnHjBxmPT
JceYZhtHvZCMPzdFu/D8iBjZwQBYV6/r1NFNtS7C0wTaKeikPvghXpiWuRVBC7vj
FEMLUWet9XrS2O2/2sMhh3ctb80XwVbfV6EvSp5WLAvK3gGPq9RAu2KGco80NZY+
Nbd6PDrauOYw6SZVTi7sLKfis8GJpr4wUPD61wIDAQABAoIBABgHbHIbD2d+Q7RO
kagu25YH5cxZiFxd0DXtXmR4TuYYdiEDahbx15inQXsBI2l3CEN4FBVkxDQt3+QN
ESimsFEXZoztlohx/E+rlntMoZrXzCkN7oAsEv4v9OWoQST7MFclledIv/6FH/a+
W4+cKfSYkOXctVr9SZlkppZbjIVLBBmtGHxOkBgvY5Bb63wKL5RtXrt7TFO1LI2Q
8rfEdr/2zQ3PQ3b1L7TocneYTRccHeV38nJfztvT4HPEaStR6ZsM75majHyAn28L
W2gQUI+eNgwpV0UUTZ8ZWXyKS+bBNEUirn45XE85oqiZ27bbu4tJtOJk4GFG4Kfd
BFTFqYECgYEA5ozrInqgWVffYNfomSzPat7/hCFANCaBUU+Xlza2KnYoL9cvXa2q
pqjD+KN5/lflDl0PzrUpk2WyQQBFGgf9z60flgmQ1oBFNCLU8DCWSmWkZWiqOvHd
UmovfwbQQXyVupRgRIw2ihT0TcuVuvqdva9rP9E/MUXLjm1RD86dhocCgYEA16qY
Eb7CUuxnOqN+Z0O7+ruymhh6sH8K9EDhYhvLnl7NlTzdp7wAJe9c+N0huyEeW83v
ZcIrfVzosUtxLWUcPQ/Awvs+LMh4e7Bac+goDG1JlH4w1IztuIoLCYHvFQ/UgGAz
trJ5qMc63zNkR269j3Eh4kgoEZktXSSI599XrTECgYEAvNw3ShlV/ZpEPJrhyYix
qQRgICb050Obr7YZoh+JfqMoLHiELqMzJi8dyjJwnu/1jZyidFxnYH1wVlsYQEjH
nDZfp5LSeUS+bAUUlmXW178HuqLQwFSdTwP5QH+eXebm6N7fNYf+rYKY1pmtYGwo
h9iJbM/GimB4bYYTX3WMCUUCgYEArrkH4ICCYLoj287vLmwi1DzSsqMYavtR/Za+
wkQwj8rQlZKtJSJboGAvG3PTyw5G0SujQvavUy49Wr37IELlQNcNXSo9MfzsF5FF
htfT8lVsIkCmAN14DmTQElDRSGf9yk+mNeKcS8+083VoTbL7IkYOpIu+4psNtINP
40L/flECgYA2LukS9pu+lmNi7e3skFojuhJOIjcDaUmeAf/HbGKjckgQO5QB2S30
tLPcgANBANF9poWbzFxYVcchtW2wpC5eyLrztkxA3X+XKaN8DkHafdXJQHtdd2Pv
xCUhwhCTXk7zQJqY1UKGjVEOs/1F6GITwQm1ZCC7WSdLedfoZHpe9w==
-----END RSA PRIVATE KEY-----"
                .to_string(),
        )
        .unwrap();
    CMD_PREFIX.set("@bo-homu".to_string()).unwrap();
    let mut cli = AppClient::new().unwrap();
    println!("tok: {}", cli.0);
    cli.post_comment(
        &crate::github::GithubRepo {
            owner: "bo-playground".to_string(),
            name: "musical-enigma".to_string(),
        },
        6,
        "Test pong",
    )
    .await
    .unwrap();
    /*let req = reqwest::Client::new()
        .post("https://api.github.com/repos/bo-playground/musical-enigma/issues/6/comments")
        .bearer_auth(PAT.get().unwrap())
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", CMD_PREFIX.get().unwrap())
        .json(&serde_json::json!({
            "body": "Test pong 🏓",
        }));
    tracing::debug!("Reqq: {:#?}", req);
    let resp = req.send().await.map_err(|e| anyhow::anyhow!(e)).unwrap();
    println!("{resp:#?}");
    let stst = resp.status();
    println!("Body: {:#?}", resp.text().await);
    assert!(stst.is_success());*/
}
