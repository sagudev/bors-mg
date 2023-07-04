use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use reqwest::Client;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

use crate::github::{GithubRepo, LabelModification, LabelTrigger};

/// Prefix for bot (default `@bors-servo`)
pub static CMD_PREFIX: OnceLock<String> = OnceLock::new();
/// GitHub Webhook secret
pub static WEBHOOK_SECRET: OnceLock<String> = OnceLock::new();
/// Personal Access Token for BOT.
pub static PAT: OnceLock<String> = OnceLock::new();
/// Github App ID.
pub static APP_ID: OnceLock<String> = OnceLock::new();
/// Private key used to authenticate as a Github App.
pub static PRIVATE_KEY: OnceLock<String> = OnceLock::new();

/// Config file to search in repo or org config repo
const CONFIG_FILE_PATH: &str = "/bors.toml";
/// Organistaions global config repo
#[cfg(feature = "servo")]
pub const ORG_CONFIG_REPO: &str = "saltfs";

/// Configuration of a repository loaded from a `bors.toml`
/// file located in the root of the repository file tree.
#[derive(serde::Deserialize, Debug)]
pub struct Config {
    /// Currently unimplemented
    ///
    /// Inheritance: Merged
    #[serde(default, deserialize_with = "deserialize_labels")]
    pub labels: HashMap<LabelTrigger, Vec<LabelModification>>,
    /// List of reviewers
    ///
    /// Inheritance: Merged
    #[serde(default)]
    pub reviewers: HashSet<String>,
    /// List of try users
    ///
    /// Inheritance: Merged
    #[serde(default)]
    pub try_users: HashSet<String>,
    /// List of try choosers
    ///
    /// Inheritance: Override
    #[serde(default)]
    pub try_choosers: HashSet<String>,
    /// Run try on Bot's fork
    ///
    /// Inheritance: Override
    #[serde(default)]
    pub fork_try: bool,
}

impl Config {
    /// Merges two config
    ///
    /// global is org config, local config is repos config
    fn merge(mut global: Self, local: Self) -> Self {
        // this field is partiali merged
        global.labels.extend(local.labels.into_iter());
        // this field is merged
        global.reviewers.extend(local.reviewers.into_iter());
        // this field is merged
        global.try_users.extend(local.try_users.into_iter());
        // this field is overriden
        if !local.try_choosers.is_empty() {
            global.try_choosers = local.try_choosers;
        }
        // this field is overriden
        global.fork_try = local.fork_try;
        global
    }

    async fn get(client: &Client, repo: &str, branch: &str) -> Option<Self> {
        if let Ok(res) = client
            .get(format!(
                "https://raw.githubusercontent.com/{repo}/{branch}{CONFIG_FILE_PATH}"
            ))
            .send()
            .await
        {
            if let Ok(txt) = res.text().await {
                return toml::from_str(&txt).ok();
            }
        }
        None
    }

    pub async fn get_all(repo: &GithubRepo) -> Option<Config> {
        let client = Client::new();
        let mut local = if let Some(loc) = Config::get(&client, &repo.to_string(), "master").await {
            Some(loc)
        } else {
            Config::get(&client, &repo.to_string(), "main").await
        };

        #[cfg(feature = "servo")]
        if local.is_none() {
            Config::get(client, &repo.to_string(), "servo").await
        }

        let mut global = None;
        #[cfg(feature = "servo")]
        {
            let repo = format!("{ORG_CONFIG_REPO}/{}", repo.name());
            if let Some(loc) = Config::get(&client, &repo, "master").await {
                global = Some(loc)
            } else {
                global = Config::get(&client, &repo, "main").await
            };
        };
        match (local, global) {
            (Some(loc), Some(glob)) => Some(Config::merge(glob, loc)),
            (Some(loc), None) => Some(loc),
            (None, Some(glob)) => Some(glob),
            _ => None,
        }
    }
}

fn deserialize_labels<'de, D>(
    deserializer: D,
) -> Result<HashMap<LabelTrigger, Vec<LabelModification>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(serde::Deserialize, Eq, PartialEq, Hash)]
    #[serde(rename_all = "snake_case")]
    enum Trigger {
        Try,
        TrySucceed,
        TryFailed,
    }

    impl From<Trigger> for LabelTrigger {
        fn from(value: Trigger) -> Self {
            match value {
                Trigger::Try => LabelTrigger::TryBuildStarted,
                Trigger::TrySucceed => LabelTrigger::TryBuildSucceeded,
                Trigger::TryFailed => LabelTrigger::TryBuildFailed,
            }
        }
    }

    enum Modification {
        Add(String),
        Remove(String),
    }

    impl From<Modification> for LabelModification {
        fn from(value: Modification) -> Self {
            match value {
                Modification::Add(label) => LabelModification::Add(label),
                Modification::Remove(label) => LabelModification::Remove(label),
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for Modification {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value: String = String::deserialize(deserializer)?;
            if value.len() < 2 {
                return Err(Error::custom(
                    "Label modification have at least two characters and start with `+` or `-`",
                ));
            }

            let modification = if let Some(label) = value.strip_prefix('+') {
                Modification::Add(label.to_string())
            } else if let Some(label) = value.strip_prefix('-') {
                Modification::Remove(label.to_string())
            } else {
                return Err(Error::custom(
                    "Label modification must start with `+` or `-`",
                ));
            };

            Ok(modification)
        }
    }

    let triggers = HashMap::<Trigger, Vec<Modification>>::deserialize(deserializer)?;
    let triggers = triggers
        .into_iter()
        .map(|(k, v)| (k.into(), v.into_iter().map(|v| v.into()).collect()))
        .collect();
    Ok(triggers)
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use std::collections::BTreeMap;

    #[test]
    fn deserialize_empty() {
        let content = "";
        let config = load_config(content);
        assert!(config.labels.is_empty());
        assert!(config.reviewers.is_empty());
        assert!(config.try_users.is_empty());
        assert!(config.try_choosers.is_empty());
    }

    #[test]
    fn deserialize_labels() {
        let content = r#"[labels]
try = ["+foo", "-bar"]
try_succeed = ["+foobar", "+foo", "+baz"]
try_failed = []
"#;
        let config = load_config(content);
        insta::assert_debug_snapshot!(config.labels.into_iter().collect::<BTreeMap<_, _>>(), @r###"
        {
            TryBuildStarted: [
                Add(
                    "foo",
                ),
                Remove(
                    "bar",
                ),
            ],
            TryBuildSucceeded: [
                Add(
                    "foobar",
                ),
                Add(
                    "foo",
                ),
                Add(
                    "baz",
                ),
            ],
            TryBuildFailed: [],
        }
        "###);
    }

    #[test]
    #[should_panic(expected = "Label modification must start with `+` or `-`")]
    fn deserialize_labels_missing_prefix() {
        let content = r#"[labels]
try = ["foo"]
"#;
        load_config(content);
    }

    fn load_config(config: &str) -> Config {
        toml::from_str(config).unwrap()
    }
}
