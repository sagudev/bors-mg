use url::Url;

use crate::github::client::GitHubClient;
use crate::github::misc::{WorkflowStatus, WorkflowType};
use crate::github::{CommitSha, GithubRepo, GithubUser, PullRequest, PullRequestNumber};
use crate::models::RunId;

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
    // PR needs to be fetched
    PRId((GithubRepo, PullRequestNumber)),
    // PR already fetched (or piggybacked from main response)
    PR(PullRequest),
}

impl PR {
    pub async fn get_pull<R: GitHubClient>(&mut self, client: &mut R) -> &PullRequest {
        match self {
            PR::PRId((repo, pull_number)) => {
                *self = PR::PR(client.get_pull_request(repo, *pull_number).await.unwrap());
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
