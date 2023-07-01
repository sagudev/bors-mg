//! Permission parsing go as follows:
//! configs, (org not impl yet)

use crate::config::Config;

pub enum PermissionType {
    /// Can perform commands like r+.
    Review,
    /// Can start a try build.
    Try,
}

/// Decides if a GitHub user can perform various actions using the bot.
#[async_trait::async_trait]
pub trait PermissionResolver {
    async fn has_permission(&self, username: &str, permission: PermissionType) -> bool;
}

#[async_trait::async_trait]
impl PermissionResolver for Config {
    async fn has_permission(&self, username: &str, permission: PermissionType) -> bool {
        match permission {
            PermissionType::Review => self.reviewers.contains(username),
            PermissionType::Try => {
                self.reviewers.contains(username) || self.try_users.contains(username)
            }
        }
    }
}
