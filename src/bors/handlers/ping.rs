use super::PullRequestData;
use crate::github::client::GitHubClient;

pub(super) async fn command_ping<C: GitHubClient>(
    client: &mut C,
    pr_data: &PullRequestData,
) -> anyhow::Result<()> {
    let text = if pr_data.repository.owner() == "servo" {
        ":sleepy: I'm awake I'm awake"
    } else {
        "Pong 🏓!"
    };
    client
        .post_comment(&pr_data.repository, pr_data.number, text)
        .await?;
    Ok(())
}
