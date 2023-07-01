use super::PullRequestData;
use crate::bors::RepositoryClient;

pub(super) async fn command_ping<R: RepositoryClient>(
    repo: &R,
    pr_data: &PullRequestData,
) -> anyhow::Result<()> {
    let text = if repo.repository().owner() == "servo" {
        ":sleepy: I'm awake I'm awake"
    } else {
        "Pong 🏓!"
    };
    repo.post_comment(pr_data.number, text).await?;
    Ok(())
}
