use anyhow::{Context, Result};
use reqwest::StatusCode;
use thiserror::Error;

use super::misc::{CheckSuite, Reference};
use super::{CommitSha, GithubRepo, PullRequest, PullRequestNumber};
use crate::github::misc::github_pr_to_pr;
use crate::models::RunId;
mod app;
mod auto;
mod token;
pub use app::AppClient;
pub use auto::AutoGitHubClient;
pub use token::TokenClient;

/// Provides functionality for working with a (authorized) client.
#[async_trait::async_trait(?Send)]
pub trait GitHubClient {
    fn is_available() -> bool;

    async fn get(&mut self, end: &str) -> Result<reqwest::Response>;
    async fn post<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> Result<reqwest::Response>;
    async fn patch<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> Result<reqwest::Response>;

    /// Post a comment to the pull request with the given number.
    async fn post_comment(
        &mut self,
        repo: &GithubRepo,
        pr: PullRequestNumber,
        text: &str,
    ) -> Result<()> {
        tracing::debug!("/repos/{repo}/issues/{pr}/comments with body: {text}");
        let res = self
            .post(
                &format!("/repos/{repo}/issues/{pr}/comments"),
                &serde_json::json!({
                    "body": text,
                }),
            )
            .await
            .with_context(|| format!("Cannot post comment to {}", pr))?;
        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Got {}", res.status()));
        }
        Ok(())
    }

    /// Resolve a pull request from this repository by it's number.
    async fn get_pull_request(
        &mut self,
        repo: &GithubRepo,
        pull_number: PullRequestNumber,
    ) -> Result<PullRequest> {
        let pr = self
            .get(&format!("/repos/{repo}/pulls/{pull_number}"))
            .await
            .map_err(|error| {
                anyhow::anyhow!("Could not get PR {}/{}: {error:?}", repo, pull_number)
            })?
            .json()
            .await
            .map_err(|error| {
                anyhow::anyhow!("Could not parse PR {}/{}: {error:?}", repo, pull_number)
            })?;
        Ok(github_pr_to_pr(pr))
    }

    /// Set the given branch to a commit with the given `sha`.
    ///
    /// Forcefully updates the branch to the given commit `sha`.
    /// If the branch does not exist yet, it instead attempts to create it.
    async fn set_branch_to_sha(
        &mut self,
        repo: &GithubRepo,
        branch: &str,
        sha: &CommitSha,
    ) -> Result<()> {
        // Fast-path: assume that the branch exists
        match self.update_branch(repo, branch, sha).await {
            Ok(_) => Ok(()),
            Err(error) => match error.downcast_ref() {
                Some(BranchUpdateError::BranchNotFound(_)) => {
                    // Branch does not exist yet, try to create it
                    match self.create_branch(repo, branch, sha).await {
                        Ok(_) => Ok(()),
                        Err(err) => Err(BranchUpdateError::Custom(err).into()),
                    }
                }
                _ => Err(error),
            },
        }
    }

    async fn create_branch(
        &mut self,
        repo: &GithubRepo,
        name: &str,
        sha: &CommitSha,
    ) -> Result<()> {
        /*repo.client
        .repos(repo.repo_name.owner(), repo.repo_name.name())
        .create_ref(&Reference::Branch(name), sha.as_ref())
        .await
        .map_err(|error| format!("Cannot create branch: {error}"))?;*/
        Ok(())
    }

    /// Force update the branch with the given `branch_name` to the given `sha`.
    async fn update_branch(
        &mut self,
        repo: &GithubRepo,
        branch_name: &str,
        sha: &CommitSha,
    ) -> Result<()> {
        let res: reqwest::Response = self
            .patch(
                &format!(
                    "/repos/{repo}/git/refs/{}",
                    Reference::Branch(branch_name.to_owned()).ref_url()
                ),
                &serde_json::json!({
                    "sha": sha.as_ref(),
                    "force": true
                }),
            )
            .await?;

        let status = res.status();
        tracing::trace!(
            "Updating branch response: status={}, text={:?}",
            status,
            res.text().await
        );

        match status {
            StatusCode::OK => Ok(()),
            _ => Err(BranchUpdateError::BranchNotFound(branch_name.to_owned()).into()),
        }
    }

    /// Creates a merge commit on the given repository.
    /// Merge `head` into `base`. Returns the SHA of the merge commit.
    // Documentation: https://docs.github.com/en/rest/branches/branches?apiVersion=2022-11-28#merge-a-branch
    async fn merge_branches(
        &mut self,
        repo: &GithubRepo,
        base: &str,
        head: &CommitSha,
        commit_message: &str,
    ) -> Result<CommitSha> {
        let request = MergeRequest {
            base: base,
            head: head.as_ref(),
            commit_message,
        };
        let response = self.post(&format!("/repos/{repo}/merges"), &request).await;

        match response {
            Ok(response) => {
                let status = response.status();
                let text = Into::<reqwest::Response>::into(response)
                    .text()
                    .await
                    .unwrap_or_default();

                tracing::trace!(
                    "Response from merging `{head}` into `{base}` in `{}`: {status} ({text})",
                    repo,
                );

                match status {
                    StatusCode::CREATED => {
                        let response: MergeResponse =
                            serde_json::from_str(&text).map_err(|error| MergeError::Unknown {
                                status,
                                text: format!("{error:?}"),
                            })?;
                        let sha: CommitSha = response.sha.into();
                        Ok(sha)
                    }
                    StatusCode::NOT_FOUND => Err(MergeError::NotFound.into()),
                    StatusCode::CONFLICT => Err(MergeError::Conflict.into()),
                    StatusCode::NO_CONTENT => Err(MergeError::AlreadyMerged.into()),
                    _ => Err(MergeError::Unknown { status, text }.into()),
                }
            }
            Err(error) => {
                tracing::debug!(
                    "Merging `{head}` into `{base}` in `{}` failed: {error:?}",
                    repo.name()
                );
                Err(MergeError::NetworkError(error.into()).into())
            }
        }
    }

    /// Find all check suites attached to the given commit and branch.
    async fn get_check_suites_for_commit(
        &mut self,
        branch: &str,
        sha: &CommitSha,
    ) -> Result<Vec<CheckSuite>> {
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

    /// Cancels Github Actions workflows.
    async fn cancel_workflows(&mut self, run_ids: Vec<RunId>) -> Result<()> {
        todo!()
        /*let actions = reqwest::Client::new().actions();

        // Cancel all workflows in parallel
        futures::future::join_all(run_ids.into_iter().map(|run_id| {
            actions.cancel_workflow_run(self.repo_name.owner(), self.repo_name.name(), run_id)
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        Ok(())*/
    }

    // IDK
    /*
    /// Add a set of labels to a PR.
    async fn add_labels(&mut self, pr: PullRequestNumber, labels: &[String]) -> anyhow::Result<()>;

    /// Remove a set of labels from a PR.
    async fn remove_labels(
        &mut self,
        pr: PullRequestNumber,
        labels: &[String],
    ) -> anyhow::Result<()>;
    */
}

#[derive(Error, Debug)]
pub enum MergeError {
    #[error("Branch not found")]
    NotFound,
    #[error("Merge conflict")]
    Conflict,
    #[error("Branch was already merged")]
    AlreadyMerged,
    #[error("Unknown error ({status}): {text}")]
    Unknown { status: StatusCode, text: String },
    #[error("Network error: {0}")]
    NetworkError(#[from] anyhow::Error),
}

#[derive(serde::Serialize)]
struct MergeRequest<'a, 'b, 'c> {
    base: &'a str,
    head: &'b str,
    commit_message: &'c str,
}

#[derive(serde::Deserialize)]
struct MergeResponse {
    sha: String,
}

#[derive(Error, Debug)]
pub enum BranchUpdateError {
    #[error("Branch {0} was not found")]
    BranchNotFound(String),
    #[error("Unknown error: {0}")]
    Custom(#[from] anyhow::Error),
}
