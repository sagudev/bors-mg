use chrono::{DateTime, Utc};

use super::{Branch, PullRequest};
use crate::github::PullRequestNumber;
use crate::models::RunId;

/// Status of a GitHub build.
#[derive(Debug, PartialEq)]
pub enum BuildStatus {
    /// The build is still waiting for results.
    Pending,
    /// The build has succeeded.
    Success,
    /// The build has failed.
    Failure,
    /// The build has been manually cancelled by a user.
    Cancelled,
    /// The build ran for too long and was timeouted by the bot.
    Timeouted,
}

/// Represents a single (merged) commit.
pub struct BuildModel {
    pub repository: String,
    pub branch: String,
    pub commit_sha: String,
    pub status: BuildStatus,
    pub created_at: DateTime<Utc>,
}

/// Represents a pull request.
pub struct PullRequestModel {
    pub repository: String,
    pub number: PullRequestNumber,
    pub try_build: Option<BuildModel>,
    pub created_at: DateTime<Utc>,
}

/// Describes whether a workflow is a Github Actions workflow or if it's a job from some external
/// CI.
#[derive(Debug, PartialEq)]
pub enum WorkflowType {
    Github,
    External,
}

/// Status of a workflow.
#[derive(Debug, PartialEq)]
pub enum WorkflowStatus {
    /// Workflow is running.
    Pending,
    /// Workflow has succeeded.
    Success,
    /// Workflow has failed.
    Failure,
}

/// Represents a workflow run, coming either from Github Actions or from some external CI.
pub struct WorkflowModel {
    pub build: BuildModel,
    pub name: String,
    pub url: String,
    pub run_id: RunId,
    pub workflow_type: WorkflowType,
    pub status: WorkflowStatus,
    pub created_at: DateTime<Utc>,
}

/// A Git reference, either a branch, tag, or rev.
#[derive(Debug, Clone)]
pub enum Reference {
    Branch(String),
    Tag(String),
    Commit(String),
}

impl Reference {
    pub fn internal(&self) -> &str {
        match self {
            Reference::Branch(x) => x,
            Reference::Tag(x) => x,
            Reference::Commit(x) => x,
        }
    }

    pub fn ref_url(&self) -> String {
        match self {
            Self::Branch(branch) => format!("heads/{branch}"),
            Self::Tag(tag) => format!("tags/{tag}"),
            Self::Commit(sha) => sha.clone(),
        }
    }

    pub fn full_ref_url(&self) -> String {
        match self {
            Self::Branch(_) | Self::Tag(_) => format!("refs/{}", self.ref_url()),
            Self::Commit(sha) => sha.clone(),
        }
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.full_ref_url())
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
