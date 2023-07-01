use crate::bors::event::{PullRequestComment, PR};
use crate::config::Config;
use anyhow::anyhow;

use crate::bors::RepositoryClient;
use crate::github::api::misc::{
    BuildModel, BuildStatus, PullRequestModel, WorkflowStatus, WorkflowType,
};
use crate::github::{GithubUser, LabelTrigger, MergeError, PullRequest, PullRequestNumber};
use crate::permissions::{PermissionResolver, PermissionType};

use super::PullRequestData;

// This branch serves for preparing the final commit.
// It will be reset to master and merged with the branch that should be tested.
// Because this action (reset + merge) is not atomic, this branch should not run CI checks to avoid
// starting them twice.
const TRY_MERGE_BRANCH_NAME: &str = "automation/bors/try-merge";

// This branch should run CI checks.
pub(super) const TRY_BRANCH_NAME: &str = "automation/bors/try";

/// Performs a so-called try build - merges the PR branch into a special branch designed
/// for running CI checks.
pub(super) async fn command_try_build<R: RepositoryClient>(
    repo: &R,
    pr_data: &mut PullRequestData,
) -> anyhow::Result<()> {
    let config = repo.config().await;
    if !check_try_permissions(repo, &pr_data.author, &config, pr_data.number).await? {
        return Ok(());
    }
    let pr = pr_data.pr.get_pull(repo).await;

    /*if let Some(ref build) = pr_model.try_build {
        if build.status == BuildStatus::Pending {
            tracing::warn!("Try build already in progress");
            repo.client
                .post_comment(
                    pr.number,
                    ":exclamation: A try build is currently in progress. You can cancel it using @bors try cancel.",
                )
                .await?;
            return Ok(());
        }
    }*/

    //let mut pull = comment.pr.clone();

    // main branch on try merge branch
    repo.set_branch_to_sha(TRY_MERGE_BRANCH_NAME, &pr.base.sha)
        .await
        .map_err(|error| anyhow!("Cannot set try merge branch to main branch: {error:?}"))?;
    // do a merge
    match repo
        .merge_branches(
            TRY_MERGE_BRANCH_NAME,
            &pr.head.sha,
            &auto_merge_commit_message(pr, "<try>"),
        )
        .await
    {
        Ok(merge_sha) => {
            tracing::debug!("Merge successful, SHA: {merge_sha}");
            // push to ci
            repo.set_branch_to_sha(TRY_BRANCH_NAME, &merge_sha)
                .await
                .map_err(|error| anyhow!("Cannot set try branch to main branch: {error:?}"))?;

            tracing::info!("Try build started");

            //handle_label_trigger(repo, pr.number, LabelTrigger::TryBuildStarted).await?;

            repo.post_comment(
                pr.number,
                &format!(
                    ":hourglass: Trying commit {} with merge {merge_sha}…",
                    pr.head.sha
                ),
            )
            .await?;
            Ok(())
        }
        Err(MergeError::Conflict) => {
            tracing::warn!("Merge conflict");
            repo.post_comment(pr.number, &merge_conflict_message(&pr.head.name))
                .await?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn command_try_cancel<R: RepositoryClient>(
    repo: &R,
    comment: &mut PullRequestData,
) -> anyhow::Result<()> {
    let config = repo.config().await;
    if !check_try_permissions(repo, &comment.author, &config, comment.number).await? {
        return Ok(());
    }

    let pr_number: PullRequestNumber = comment.number;
    let pr = comment.pr.get_pull(repo).await;

    todo!();

    /*
    let Some(build) = get_pending_build(pr) else {
        tracing::warn!("No build found");
        repo.client.post_comment(pr_number, ":exclamation: There is currently no try build in progress.").await?;
        return Ok(());
    };

    if let Err(error) = cancel_build_workflows(repo, db, &build).await {
        tracing::error!(
            "Could not cancel workflows for SHA {}: {error:?}",
            build.commit_sha
        );
    }

    db.update_build_status(&build, BuildStatus::Cancelled)
        .await?;

    tracing::info!("Try build cancelled");

    repo.client
        .post_comment(pr_number, "Try build cancelled.")
        .await?;

    Ok(())
    */
}
/*
pub async fn cancel_build_workflows<Client: RepositoryClient>(
    build: &BuildModel,
) -> anyhow::Result<()> {
    let pending_workflows = db
        .get_workflows_for_build(build)
        .await?
        .into_iter()
        .filter(|w| w.status == WorkflowStatus::Pending && w.workflow_type == WorkflowType::Github)
        .map(|w| w.run_id)
        .collect::<Vec<_>>();

    tracing::info!("Cancelling workflows {:?}", pending_workflows);
    repo.client.cancel_workflows(pending_workflows).await
}

fn get_pending_build(pr: PullRequestModel) -> Option<BuildModel> {
    pr.try_build
        .and_then(|b| (b.status == BuildStatus::Pending).then_some(b))
}*/

fn auto_merge_commit_message(pr: &PullRequest, reviewer: &str) -> String {
    let pr_number = pr.number;
    format!(
        r#"Auto merge of #{pr_number} - {pr_label}, r={reviewer}
{pr_title}

{pr_message}"#,
        pr_label = pr.head_label,
        pr_title = pr.title,
        pr_message = pr.message
    )
}

fn merge_conflict_message(branch: &str) -> String {
    format!(
        r#":lock: Merge conflict

This pull request and the master branch diverged in a way that cannot
 be automatically merged. Please rebase on top of the latest master
 branch, and let the reviewer approve again.

<details><summary>How do I rebase?</summary>

Assuming `self` is your fork and `upstream` is this repository,
 you can resolve the conflict following these steps:

1. `git checkout {branch}` *(switch to your branch)*
2. `git fetch upstream master` *(retrieve the latest master)*
3. `git rebase upstream/master -p` *(rebase on top of it)*
4. Follow the on-screen instruction to resolve conflicts (check `git status` if you got lost).
5. `git push self {branch} --force-with-lease` *(update this PR)*

You may also read
 [*Git Rebasing to Resolve Conflicts* by Drew Blessing](http://blessing.io/git/git-rebase/open-source/2015/08/23/git-rebasing-to-resolve-conflicts.html)
 for a short tutorial.

Please avoid the ["**Resolve conflicts**" button](https://help.github.com/articles/resolving-a-merge-conflict-on-github/) on GitHub.
 It uses `git merge` instead of `git rebase` which makes the PR commit history more difficult to read.

Sometimes step 4 will complete without asking for resolution. This is usually due to difference between how `Cargo.lock` conflict is
handled during merge and rebase. This is normal, and you should still perform step 5 to update this PR.

</details>  
"#
    )
}

async fn check_try_permissions<R: RepositoryClient>(
    repo: &R,
    author: &GithubUser,
    config: &Config,
    pr_number: PullRequestNumber,
) -> anyhow::Result<bool> {
    let result = if !config
        .has_permission(&author.username, PermissionType::Try)
        .await
    {
        tracing::info!("Permission denied");
        repo.post_comment(
            pr_number,
            &format!(
                "@{}: :key: Insufficient privileges: not in try users",
                author.username
            ),
        )
        .await?;
        false
    } else {
        true
    };
    Ok(result)
}
