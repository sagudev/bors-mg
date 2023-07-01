use url::Url;

use crate::github::api::misc::{WorkflowStatus, WorkflowType};
use crate::github::{CommitSha, GithubRepo, GithubUser, PullRequest, PullRequestNumber};
use crate::models::RunId;

use super::RepositoryClient;

#[derive(Debug)]
pub enum BorsEvent {
    /// A comment was posted on a pull request.
    Comment(PullRequestComment),
    /// A workflow run on Github Actions or a check run from external CI system has been started.
    WorkflowStarted(WorkflowStarted),
    /// A workflow run on Github Actions or a check run from external CI system has been completed.
    WorkflowCompleted(WorkflowCompleted),
    /// A check suite has been completed, either as a workflow run on Github Actions, or as a
    /// workflow from some external CI system.
    CheckSuiteCompleted(CheckSuiteCompleted),
    /// The configuration of some repository has been changed for the bot's Github App.
    InstallationsChanged,
    /// Periodic event that serves for checking e.g. timeouts.
    Refresh,
}

#[derive(Clone, Debug)]
pub enum PR {
    PRUrl(Url),
    PR(PullRequest),
}

impl PR {
    pub async fn get_pull<R: RepositoryClient>(&mut self, repo: &R) -> &PullRequest {
        match self {
            PR::PRUrl(url) => {
                let pr = crate::github::api::client::github_pr_to_pr(
                    repo.get(url.as_str()).await.unwrap().json().await.unwrap(),
                );
                *self = PR::PR(pr);
                if let PR::PR(pr) = self {
                    pr
                } else {
                    unreachable!()
                }
            }
            PR::PR(pr) => pr,
        }
    }
}

#[derive(Debug)]
pub struct PullRequestComment {
    pub repository: GithubRepo,
    pub author: GithubUser,
    pub pr_number: PullRequestNumber,
    pub pr: PR,
    pub text: String,
}

#[derive(Debug)]
pub struct WorkflowStarted {
    pub repository: GithubRepo,
    pub name: String,
    pub branch: String,
    pub commit_sha: CommitSha,
    pub run_id: RunId,
    pub workflow_type: WorkflowType,
    pub url: String,
}

#[derive(Debug)]
pub struct WorkflowCompleted {
    pub repository: GithubRepo,
    pub branch: String,
    pub commit_sha: CommitSha,
    pub run_id: RunId,
    pub status: WorkflowStatus,
}

#[derive(Debug)]
pub struct CheckSuiteCompleted {
    pub repository: GithubRepo,
    pub branch: String,
    pub commit_sha: CommitSha,
}
