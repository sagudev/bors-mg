use crate::models::RunId;

use crate::github::{CommitSha, GithubRepo, MergeError, PullRequest, PullRequestNumber};

mod command;
pub mod event;
pub mod handlers;

pub use command::CommandParser;
pub use handlers::handle_bors_event;

/// Provides functionality for working with a remote repository.
#[async_trait::async_trait(?Send)]
pub trait RepositoryClient {
    fn repository(&self) -> &GithubRepo;
    async fn config(&self) -> crate::config::Config;

    async fn get<U: reqwest::IntoUrl>(&self, url: U) -> anyhow::Result<reqwest::Response>;
    async fn post<D: serde::Serialize + Sized>(
        &self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response>;
    async fn patch<D: serde::Serialize + Sized>(
        &self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response>;

    /// Post a comment to the pull request with the given number.
    async fn post_comment(&self, pr: PullRequestNumber, text: &str) -> anyhow::Result<()>;

    /*/// Resolve a pull request from this repository by it's number.
        async fn get_pull_request(&mut self, pr: &PullRequestNumber) -> anyhow::Result<PullRequest>;
    */
    /// Cancels Github Actions workflows.
    async fn cancel_workflows(&self, run_ids: Vec<RunId>) -> anyhow::Result<()>;

    /// Set the given branch to a commit with the given `sha`.
    async fn set_branch_to_sha(&self, branch: &str, sha: &CommitSha) -> anyhow::Result<()>;

    /// Merge `head` into `base`. Returns the SHA of the merge commit.
    async fn merge_branches(
        &self,
        base: &str,
        head: &CommitSha,
        commit_message: &str,
    ) -> Result<CommitSha, MergeError>;

    /// Find all check suites attached to the given commit and branch.
    async fn get_check_suites_for_commit(
        &self,
        branch: &str,
        sha: &CommitSha,
    ) -> anyhow::Result<Vec<CheckSuite>>;

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

#[derive(Clone)]
pub enum CheckSuiteStatus {
    Pending,
    Failure,
    Success,
}

/// A GitHub check suite.
/// Corresponds to a single GitHub actions workflow run, or to a single external CI check run.
#[derive(Clone)]
pub struct CheckSuite {
    pub(crate) status: CheckSuiteStatus,
}
