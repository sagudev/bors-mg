use tokio::sync::OnceCell;

use anyhow::Context;
use http::Method;
use reqwest::{Request, RequestBuilder};
use tracing_subscriber::fmt::format;

use crate::config::{Config, PAT};
use crate::models::{Repository, RunId};
use tracing as log;

use crate::bors::{CheckSuite, CheckSuiteStatus, RepositoryClient};
use crate::github::api::operations::{merge_branches, set_branch_to_commit, MergeError};
use crate::github::{Branch, CommitSha, GithubRepo, PullRequest, PullRequestNumber};

/// We can run 5000req/h
use super::API_ENDPOINT;

/// Provides access to a single app installation (repository) using the GitHub API.
pub struct AppClient;

/// Provides access to GitHub API using PAT
pub struct PATClient {
    repo: GithubRepo,
    client: reqwest::Client,
}

impl PATClient {
    fn format_pr(&self, pr: PullRequestNumber) -> String {
        format!("{}/{}/{}", self.repo.owner(), self.repo.name(), pr)
    }

    pub(crate) fn new(repo: GithubRepo) -> Self {
        Self {
            repo,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl RepositoryClient for PATClient {
    fn repository(&self) -> &GithubRepo {
        &self.repo
    }

    async fn config(&self) -> Config {
        Config::get_all(&self.client, self.repository())
            .await
            .unwrap()
    }

    async fn get<U: reqwest::IntoUrl>(&self, url: U) -> anyhow::Result<reqwest::Response> {
        self.client
            .get(url)
            .bearer_auth(PAT.get().unwrap())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn post<D: serde::Serialize + Sized>(
        &self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        self.client
            .post(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn patch<D: serde::Serialize + Sized>(
        &self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        self.client
            .patch(API_ENDPOINT.to_owned() + end)
            .bearer_auth(PAT.get().unwrap())
            .json(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// The comment will be posted as the Github App user of the bot.
    async fn post_comment(&self, pr: PullRequestNumber, text: &str) -> anyhow::Result<()> {
        let res = self
            .client
            .patch(
                API_ENDPOINT.to_owned()
                    + &format!(
                        "/repos/{}/{}/issues/{pr}/comments",
                        self.repository().owner,
                        self.repository().name
                    ),
            )
            .bearer_auth(PAT.get().unwrap())
            .json(&serde_json::json!({
                "body": text,
            }))
            .send()
            .await
            .with_context(|| format!("Cannot post comment to {}", self.format_pr(pr)))?;
        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Got {}", res.status()));
        }
        Ok(())
    }

    /*async fn get_pull_request(&mut self, pr: PullRequestNumber) -> anyhow::Result<PullRequest> {
        todo!();
        /*let pr = self
            .client
            .pulls(self.repository().owner(), self.repository().name())
            .get(pr.0)
            .await
            .map_err(|error| {
                anyhow::anyhow!("Could not get PR {}/{}: {error:?}", self.repository(), pr.0)
            })?;
        Ok(github_pr_to_pr(pr))*/
    }*/

    async fn set_branch_to_sha(&self, branch: &str, sha: &CommitSha) -> anyhow::Result<()> {
        Ok(set_branch_to_commit(self, branch.to_string(), sha).await?)
    }

    async fn merge_branches(
        &self,
        base: &str,
        head: &CommitSha,
        commit_message: &str,
    ) -> Result<CommitSha, MergeError> {
        Ok(merge_branches(self, base, head, commit_message).await?)
    }

    async fn get_check_suites_for_commit(
        &self,
        branch: &str,
        sha: &CommitSha,
    ) -> anyhow::Result<Vec<CheckSuite>> {
        todo!();
        /*
        let response = self
            .get(format!(
                "/repos/{}/{}/commits/{}/check-suites",
                self.repo_name.owner(),
                self.repo_name.name(),
                sha.0
            ))
            .await?;

        #[derive(serde::Deserialize, Debug)]
        struct CheckSuitePayload<'a> {
            conclusion: Option<&'a str>,
            head_branch: &'a str,
        }

        #[derive(serde::Deserialize, Debug)]
        struct CheckSuiteResponse<'a> {
            #[serde(borrow)]
            check_suites: Vec<CheckSuitePayload<'a>>,
        }

        // `response.json()` is not used because of the 'a lifetime
        let text = Into::<reqwest::Response>::into(response).text().await?;
        let response: CheckSuiteResponse = serde_json::from_str(&text)?;
        let suites = response
            .check_suites
            .into_iter()
            .filter(|suite| suite.head_branch == branch)
            .map(|suite| CheckSuite {
                status: match suite.conclusion {
                    Some(status) => match status {
                        "success" => CheckSuiteStatus::Success,
                        "failure" | "neutral" | "cancelled" | "skipped" | "timed_out"
                        | "action_required" | "startup_failure" | "stale" => {
                            CheckSuiteStatus::Failure
                        }
                        _ => {
                            tracing::warn!(
                                "Received unknown check suite status for {}/{}: {status}",
                                self.repo_name,
                                sha
                            );
                            CheckSuiteStatus::Pending
                        }
                    },
                    None => CheckSuiteStatus::Pending,
                },
            })
            .collect();
        Ok(suites)*/
    }

    async fn cancel_workflows(&self, run_ids: Vec<RunId>) -> anyhow::Result<()> {
        todo!()
        /*let actions = self.client.actions();

        // Cancel all workflows in parallel
        futures::future::join_all(run_ids.into_iter().map(|run_id| {
            actions.cancel_workflow_run(self.repo_name.owner(), self.repo_name.name(), run_id)
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        Ok(())*/
    }
}

pub fn github_pr_to_pr(pr: crate::models::pulls::PullRequest) -> PullRequest {
    PullRequest {
        number: pr.number.into(),
        head_label: pr.head.label.unwrap_or_else(|| "<unknown>".to_string()),
        head: Branch {
            name: pr.head.ref_field,
            sha: pr.head.sha.into(),
        },
        base: Branch {
            name: pr.base.ref_field,
            sha: pr.base.sha.into(),
        },
        title: pr.title.unwrap_or_default(),
        message: pr.body.unwrap_or_default(),
    }
}
