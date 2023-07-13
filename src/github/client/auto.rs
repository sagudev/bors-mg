//! Automatic chooser for api endpoints to balance API usage limits
//! also based upon availability

use std::cell::OnceCell;

use anyhow::Result;
use thiserror::Error;

use super::{AppClient, GitHubClient, TokenClient};
use crate::github::misc::{CheckSuite, Reference};
use crate::github::{CommitSha, GithubRepo, PullRequest, PullRequestNumber};
use crate::models::RunId;

#[derive(Error, Debug)]
pub enum AutoClientError {
    #[error("No authorized client available.")]
    NoClient,
}

// selectors
macro_rules! pat_app {
    ($sef:expr,$fn:ident($($arg:expr),*)) => {
        if TokenClient::is_available() {
            $sef.token.$fn($($arg),*).await
        } else if let Ok(app_cli) = $sef.app_get_or_init() {
            app_cli.$fn($($arg),*).await
        } else {
            Err(AutoClientError::NoClient.into())
        }
    };
}

macro_rules! app_pat {
    ($sef:expr,$fn:ident($($arg:expr),*)) => {
        if let Ok(app_cli) = $sef.app_get_or_init() {
            app_cli.$fn($($arg),*).await
        } else if TokenClient::is_available() {
            $sef.token.$fn($($arg),*).await
        } else {
            Err(AutoClientError::NoClient.into())
        }
    };
}

pub struct AutoGitHubClient {
    token: TokenClient,
    app: OnceCell<AppClient>,
}

impl AutoGitHubClient {
    pub fn new() -> Self {
        Self {
            token: TokenClient,
            app: OnceCell::new(),
        }
    }

    fn app_get_or_init(&mut self) -> Result<&mut AppClient> {
        self.app.get_or_init(|| AppClient::new().unwrap());
        self.app.get_mut().ok_or(AutoClientError::NoClient.into())
    }
}

#[async_trait::async_trait(?Send)]
impl GitHubClient for AutoGitHubClient {
    fn is_available() -> bool {
        TokenClient::is_available() || AppClient::is_available()
    }

    async fn get(&mut self, end: &str) -> anyhow::Result<reqwest::Response> {
        pat_app!(self, get(end))
    }

    async fn post<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        pat_app!(self, post(end, data))
    }

    async fn patch<D: serde::Serialize + Sized>(
        &mut self,
        end: &str,
        data: &D,
    ) -> anyhow::Result<reqwest::Response> {
        pat_app!(self, patch(end, data))
    }

    /// Post a comment to the pull request with the given number.
    async fn post_comment(
        &mut self,
        repo: &GithubRepo,
        pr: PullRequestNumber,
        text: &str,
    ) -> anyhow::Result<()> {
        pat_app!(self, post_comment(repo, pr, text))
    }

    async fn get_pull_request(
        &mut self,
        repo: &GithubRepo,
        pull_number: PullRequestNumber,
    ) -> Result<PullRequest> {
        app_pat!(self, get_pull_request(repo, pull_number))
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
    ) -> anyhow::Result<()> {
        app_pat!(self, set_branch_to_sha(repo, branch, sha))
    }

    async fn create_branch(
        &mut self,
        repo: &GithubRepo,
        refs: &Reference,
        sha: &CommitSha,
    ) -> Result<()> {
        app_pat!(self, create_branch(repo, refs, sha))
    }

    /// Force update the branch with the given `branch_name` to the given `sha`.
    async fn update_branch(
        &mut self,
        repo: &GithubRepo,
        refs: &Reference,
        sha: &CommitSha,
    ) -> Result<()> {
        app_pat!(self, update_branch(repo, refs, sha))
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
        app_pat!(self, merge_branches(repo, base, head, commit_message))
    }

    /// Find all check suites attached to the given commit and branch.
    async fn get_check_suites_for_commit(
        &mut self,
        branch: &str,
        sha: &CommitSha,
    ) -> anyhow::Result<Vec<CheckSuite>> {
        app_pat!(self, get_check_suites_for_commit(branch, sha))
    }

    /// Cancels Github Actions workflows.
    async fn cancel_workflows(&mut self, run_ids: Vec<RunId>) -> anyhow::Result<()> {
        app_pat!(self, cancel_workflows(run_ids))
    }
}
