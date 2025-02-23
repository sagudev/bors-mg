//! Contains definitions of common types (pull request, user, repository name) needed
//! for working with (GitHub) repositories.
use std::fmt::{Debug, Display, Formatter};

use url::Url;

pub mod client;
mod labels;
pub mod misc;
pub mod webhook;

pub use client::MergeError;
pub use labels::{LabelModification, LabelTrigger};
const API_ENDPOINT: &str = "https://api.github.com";

/// Unique identifier of a GitHub repository
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct GithubRepo {
    owner: String,
    name: String,
}

impl GithubRepo {
    pub fn new(owner: &str, name: &str) -> Self {
        Self {
            owner: owner.to_lowercase(),
            name: name.to_lowercase(),
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Display for GithubRepo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}/{}", self.owner, self.name))
    }
}

#[derive(Debug, PartialEq)]
pub struct GithubUser {
    pub username: String,
    pub html_url: Url,
}

#[derive(Clone, Debug)]
pub struct CommitSha(pub String);

impl From<String> for CommitSha {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl AsRef<str> for CommitSha {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
impl Display for CommitSha {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub name: String,
    pub sha: CommitSha,
}

#[derive(Clone, Debug)]
pub struct PullRequest {
    pub number: PullRequestNumber,
    /// <author>:<branch>
    pub head_label: String,
    pub head: Branch,
    pub base: Branch,
    pub title: String,
    pub message: String,
}

pub type PullRequestNumber = u64;
