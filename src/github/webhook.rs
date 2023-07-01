//! This module handles parsing webhooks and generating [`BorsEvent`]s from them.
use std::fmt::Debug;

use crate::models::events::payload::{
    IssueCommentEventAction, IssueCommentEventPayload, PullRequestReviewCommentEventAction,
    PullRequestReviewCommentEventPayload,
};
use crate::models::pulls::Review;
use crate::models::{apps::App, workflows, Author, CheckRun, Repository, RunId};
use hmac::{Hmac, Mac};
use http::StatusCode;
use sha2::Sha256;

use crate::bors::event::{
    BorsEvent, CheckSuiteCompleted, PullRequestComment, WorkflowCompleted, WorkflowStarted, PR,
};
use crate::cf::Req;
use crate::config::WEBHOOK_SECRET;
use crate::github::api::misc::{WorkflowStatus, WorkflowType};
use crate::github::{CommitSha, GithubRepo, GithubUser, PullRequestNumber};

/// This struct is used to extract the repository and user from a GitHub webhook event.
/// The wrapper exists because octocrab doesn't expose/parse the repository field.
#[derive(serde::Deserialize, Debug)]
pub struct WebhookRepository {
    repository: Repository,
}

#[derive(serde::Deserialize, Debug)]
pub struct WebhookWorkflowRun<'a> {
    action: &'a str,
    workflow_run: workflows::Run,
    repository: Repository,
}

#[derive(serde::Deserialize, Debug)]
pub struct CheckRunInner {
    #[serde(flatten)]
    check_run: CheckRun,
    name: String,
    check_suite: CheckSuiteInner,
    app: App,
}

#[derive(serde::Deserialize, Debug)]
pub struct WebhookCheckRun<'a> {
    action: &'a str,
    check_run: CheckRunInner,
    repository: Repository,
}

#[derive(serde::Deserialize, Debug)]
pub struct CheckSuiteInner {
    head_branch: String,
    head_sha: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct WebhookCheckSuite<'a> {
    action: &'a str,
    check_suite: CheckSuiteInner,
    repository: Repository,
}

#[derive(Debug, serde::Deserialize)]
pub struct WebhookPullRequestReviewEvent<'a> {
    action: &'a str,
    pull_request: crate::models::pulls::PullRequest,
    review: Review,
    repository: Repository,
    sender: Author,
}

/// extractor for GitHub webhook events.
#[derive(Debug)]
pub struct GitHubWebhook(pub BorsEvent);

/// Extracts a webhook event from a HTTP request.
impl GitHubWebhook {
    pub async fn from_request(req: &mut worker::Request) -> Result<Self, StatusCode> {
        let body = req.bytes().await.unwrap();
        // Verify that the request is valid
        if !verify_gh_signature(
            &req.get_header("x-hub-signature-256").unwrap(),
            &body,
            WEBHOOK_SECRET.get().expect("Webhook expected!"),
        ) {
            tracing::error!("Webhook request failed, could not authenticate webhook");
            return Err(StatusCode::BAD_REQUEST);
        }

        // Parse webhook content
        match parse_webhook_event(&req.get_header("x-github-event").unwrap(), &body) {
            Ok(Some(event)) => {
                tracing::trace!("Received webhook event {event:?}");
                Ok(GitHubWebhook(event))
            }
            Ok(None) => Err(StatusCode::OK),
            Err(error) => {
                tracing::error!("Cannot parse webhook event: {error:?}");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }
}

fn parse_webhook_event(event_type: &str, body: &[u8]) -> anyhow::Result<Option<BorsEvent>> {
    tracing::trace!(
        "Webhook: event_type `{}`, payload\n{}",
        event_type,
        std::str::from_utf8(&body).unwrap_or_default()
    );

    match event_type.as_bytes() {
        b"issue_comment" => {
            let repository: WebhookRepository = serde_json::from_slice(body)?;
            let repository_name = parse_repository_name(&repository.repository)?;

            let event: IssueCommentEventPayload = serde_json::from_slice(body)?;
            if event.action == IssueCommentEventAction::Created {
                let comment = parse_pr_comment(repository_name, event).map(BorsEvent::Comment);
                Ok(comment)
            } else {
                Ok(None)
            }
        }
        b"pull_request_review" => {
            let payload: WebhookPullRequestReviewEvent = serde_json::from_slice(body)?;
            if payload.action == "submitted" {
                let comment = parse_comment_from_pr_review(payload)?;
                Ok(Some(BorsEvent::Comment(comment)))
            } else {
                Ok(None)
            }
        }
        b"pull_request_review_comment" => {
            let repository: WebhookRepository = serde_json::from_slice(body)?;
            let repository_name = parse_repository_name(&repository.repository)?;

            let payload: PullRequestReviewCommentEventPayload = serde_json::from_slice(body)?;
            if payload.action == PullRequestReviewCommentEventAction::Created {
                let comment = parse_pr_review_comment(repository_name, payload);
                Ok(Some(BorsEvent::Comment(comment)))
            } else {
                Ok(None)
            }
        }
        b"installation_repositories" | b"installation" => Ok(Some(BorsEvent::InstallationsChanged)),
        b"workflow_run" => {
            let payload: WebhookWorkflowRun = serde_json::from_slice(body)?;
            let repository_name = parse_repository_name(&payload.repository)?;
            let result = match payload.action {
                "requested" => Some(BorsEvent::WorkflowStarted(WorkflowStarted {
                    repository: repository_name,
                    name: payload.workflow_run.name,
                    branch: payload.workflow_run.head_branch,
                    commit_sha: CommitSha(payload.workflow_run.head_sha),
                    run_id: RunId(payload.workflow_run.id.0),
                    workflow_type: WorkflowType::Github,
                    url: payload.workflow_run.html_url.into(),
                })),
                "completed" => Some(BorsEvent::WorkflowCompleted(WorkflowCompleted {
                    repository: repository_name,
                    branch: payload.workflow_run.head_branch,
                    commit_sha: CommitSha(payload.workflow_run.head_sha),
                    run_id: RunId(payload.workflow_run.id.0),
                    status: match payload.workflow_run.conclusion.unwrap_or_default().as_str() {
                        "success" => WorkflowStatus::Success,
                        _ => WorkflowStatus::Failure,
                    },
                })),
                _ => None,
            };
            Ok(result)
        }
        b"check_run" => {
            let payload: WebhookCheckRun = serde_json::from_slice(body).unwrap();

            // We are only interested in check runs from external CI services.
            // These basically correspond to workflow runs from GHA.
            if payload.check_run.app.owner.login == "github" {
                return Ok(None);
            }

            let repository_name = parse_repository_name(&payload.repository)?;
            if payload.action == "created" {
                Ok(Some(BorsEvent::WorkflowStarted(WorkflowStarted {
                    repository: repository_name,
                    name: payload.check_run.name.to_string(),
                    branch: payload.check_run.check_suite.head_branch,
                    commit_sha: CommitSha(payload.check_run.check_suite.head_sha),
                    run_id: RunId(payload.check_run.check_run.id.map(|v| v.0).unwrap_or(0)),
                    workflow_type: WorkflowType::External,
                    url: payload.check_run.check_run.html_url.unwrap_or_default(),
                })))
            } else {
                Ok(None)
            }
        }
        b"check_suite" => {
            let payload: WebhookCheckSuite = serde_json::from_slice(body)?;
            let repository_name = parse_repository_name(&payload.repository)?;
            if payload.action == "completed" {
                Ok(Some(BorsEvent::CheckSuiteCompleted(CheckSuiteCompleted {
                    repository: repository_name,
                    branch: payload.check_suite.head_branch,
                    commit_sha: CommitSha(payload.check_suite.head_sha),
                })))
            } else {
                Ok(None)
            }
        }
        _ => {
            tracing::debug!("Ignoring unknown event type {:?}", event_type);
            Ok(None)
        }
    }
}

fn parse_pr_review_comment(
    repo: GithubRepo,
    payload: PullRequestReviewCommentEventPayload,
) -> PullRequestComment {
    let user = parse_user(payload.comment.user);
    PullRequestComment {
        repository: repo,
        author: user,
        pr_number: PullRequestNumber(payload.pull_request.number),
        pr: parse_pr(payload.pull_request),
        text: payload.comment.body.unwrap_or_default(),
    }
}

fn parse_comment_from_pr_review(
    payload: WebhookPullRequestReviewEvent<'_>,
) -> anyhow::Result<PullRequestComment> {
    let repository_name = parse_repository_name(&payload.repository)?;
    let user = parse_user(payload.sender);

    Ok(PullRequestComment {
        repository: repository_name,
        author: user,
        pr_number: PullRequestNumber(payload.pull_request.number),
        pr: parse_pr(payload.pull_request),
        text: payload.review.body.unwrap_or_default(),
    })
}

fn parse_user(user: Author) -> GithubUser {
    GithubUser {
        username: user.login,
        html_url: user.html_url,
    }
}

fn parse_pr(pr: crate::models::pulls::PullRequest) -> PR {
    PR::PR(super::api::client::github_pr_to_pr(pr))
}

fn parse_pr_comment(
    repo: GithubRepo,
    payload: IssueCommentEventPayload,
) -> Option<PullRequestComment> {
    // We only care about pull request comments
    if payload.issue.pull_request.is_none() {
        tracing::debug!("Ignoring event {payload:?} because it does not belong to a pull request");
        return None;
    }

    Some(PullRequestComment {
        repository: repo,
        author: parse_user(payload.comment.user),
        text: payload.comment.body.unwrap_or_default(),
        pr_number: PullRequestNumber(payload.issue.number),
        pr: PR::PRUrl(payload.issue.pull_request.unwrap().url),
    })
}

fn parse_repository_name(repository: &Repository) -> anyhow::Result<GithubRepo> {
    let repo_name = &repository.name;
    let Some(repo_owner) = repository
        .owner
        .as_ref()
        .map(|u| &u.login) else {
        return Err(anyhow::anyhow!("Owner for repo {repo_name} is missing"));
    };
    Ok(GithubRepo::new(repo_owner, repo_name))
}

type HmacSha256 = Hmac<Sha256>;

/// Verifies that the request is properly signed by GitHub with SHA-256 and the passed `secret`.
fn verify_gh_signature(signature: &str, body: &[u8], secret: &str) -> bool {
    let Some(signature) = signature.get(b"sha256=".len()..).and_then(|v| hex::decode(v).ok()) else {
        return false;
    };

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("Cannot create HMAC key");
    mac.update(body);
    mac.verify_slice(&signature).is_ok()
}
