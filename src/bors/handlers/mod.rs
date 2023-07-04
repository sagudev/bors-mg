use anyhow::Context;
use tracing::Instrument;

use crate::bors::command::BorsCommand;
use crate::bors::command::CommandParseError;
use crate::bors::event::{BorsEvent, PullRequestComment};
use crate::bors::handlers::ping::command_ping;
use crate::bors::handlers::trybuild::{command_try_build, command_try_cancel, TRY_BRANCH_NAME};
/*use crate::bors::handlers::workflow::{
    handle_check_suite_completed, handle_workflow_completed, handle_workflow_started,
};*/
use crate::config::CMD_PREFIX;
use crate::config::PAT;
use crate::github::client::AutoGitHubClient;
use crate::github::client::GitHubClient;
use crate::github::client::TokenClient;
use crate::github::GithubRepo;
use crate::github::GithubUser;
use crate::github::PullRequestNumber;
use crate::utils::logging::LogError;

use super::event::PR;
use super::CommandParser;

mod ping;
mod trybuild;
//mod workflow;

pub struct PullRequestData {
    pub repository: GithubRepo,
    pub author: GithubUser,
    pub number: PullRequestNumber,
    pub pr: PR,
}

/// This function performs a single BORS event, it is the main execution function of the bot.
pub async fn handle_bors_event(event: BorsEvent) -> anyhow::Result<()> {
    match event {
        BorsEvent::Comment(comment) => {
            // We want to ignore comments made by this bot
            /*if state.is_comment_internal(&comment) {
                tracing::trace!("Ignoring comment {comment:?} because it was authored by this bot");
                return Ok(());
            }*/

            let span = tracing::info_span!(
                "Comment",
                pr = format!("{}#{}", comment.repository, comment.pr_number),
                author = comment.author.username
            );
            if let Err(error) = handle_comment(comment).instrument(span.clone()).await {
                span.log_error(error);
            }
        }
        BorsEvent::InstallationsChanged => {
            let span = tracing::info_span!("Repository reload");
            todo!("Apper")
            // although we might want to make sure that we have hook
        }
        BorsEvent::WorkflowStarted(payload) => {
            /*if let Some((_, db)) = get_repo_state(state, &payload.repository) {
                let span = tracing::info_span!(
                    "Workflow started",
                    repo = payload.repository.to_string(),
                    id = payload.run_id.into_inner()
                );
                if let Err(error) = handle_workflow_started(db, payload)
                    .instrument(span.clone())
                    .await
                {
                    span.log_error(error);
                }
            }*/
        }
        BorsEvent::WorkflowCompleted(payload) => {
            /*if let Some((repo, db)) = get_repo_state(state, &payload.repository) {
                let span = tracing::info_span!(
                    "Workflow completed",
                    repo = payload.repository.to_string(),
                    id = payload.run_id.into_inner()
                );
                if let Err(error) = handle_workflow_completed(repo, db, payload)
                    .instrument(span.clone())
                    .await
                {
                    span.log_error(error);
                }
            }*/
        }
        BorsEvent::CheckSuiteCompleted(payload) => {
            /*if let Some((repo, db)) = get_repo_state(state, &payload.repository) {
                let span = tracing::info_span!(
                    "Check suite completed",
                    repo = payload.repository.to_string(),
                );
                if let Err(error) = handle_check_suite_completed(repo, db, payload)
                    .instrument(span.clone())
                    .await
                {
                    span.log_error(error);
                }
            }*/
        }
        BorsEvent::Refresh => {
            let span = tracing::info_span!("Refresh");
            // nop
        }
    }
    Ok(())
}

async fn handle_comment(comment: PullRequestComment) -> anyhow::Result<()> {
    let parser = CommandParser::new(CMD_PREFIX.get().unwrap());
    let commands = parser.parse_commands(&comment.text);
    let mut client = AutoGitHubClient::new();
    let mut pr_data = PullRequestData {
        repository: comment.repository,
        author: comment.author,
        number: comment.pr_number,
        pr: comment.pr,
    };

    tracing::debug!("Commands: {commands:?}");
    tracing::trace!("Text: {}", comment.text);

    for command in commands {
        match command {
            Ok(command) => {
                let result = match command {
                    BorsCommand::Ping => {
                        let span = tracing::info_span!("Ping");
                        command_ping(&mut client, &pr_data).instrument(span).await
                    }
                    BorsCommand::Try => {
                        let span = tracing::info_span!("Try");
                        command_try_build(&mut client, &mut pr_data)
                            .instrument(span)
                            .await
                    }
                    BorsCommand::TryCancel => {
                        let span = tracing::info_span!("Cancel try");
                        command_try_cancel(&mut client, &mut pr_data)
                            .instrument(span)
                            .await
                    }
                };
                if result.is_err() {
                    return result.context("Cannot execute Bors command");
                }
            }
            Err(error) => {
                let error_msg = match error {
                    CommandParseError::MissingCommand => "Missing command.".to_string(),
                    CommandParseError::UnknownCommand(command) => {
                        format!(r#"Unknown command "{command}"."#)
                    }
                };

                tracing::warn!("{error_msg}");

                client
                    .post_comment(&pr_data.repository, pr_data.number, &error_msg)
                    .await
                    .context("Could not reply to PR comment")?;
            }
        }
    }
    Ok(())
}
