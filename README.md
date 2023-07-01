# Bo

Bors bot without real work. It handles webhooks (commands) and dispatches GH actions to Merge Queueu

## Design priciples

- Quasi stateless (to be run in cloudflare workers)
- Wasm
- Using Merge Queue

## Workflow automata

When we recive webhook it can be one off three types:

1. Command (classic bors command)
2. GH command (egg. reviews in GH ui) curenttly unhandled
3. Workflow status (comeback?? or should this be dead)
4. app status (registered, updated, removed) (if applicateable)

## Commands

Before running commands we need to verify premissions (org, repo (cache?)).

- `r+ (SHA)`: Accept a PR. Optionally, the SHA of the last commit in the PR can be provided as a guard against synchronization issues or malicious users. Regardless of the form used, PRs will automatically be unaccepted if the contents are changed.
- `r=NAME (SHA)`: Accept a PR on the behalf of NAME.
- `r-`: Unacccept a PR.
- `p=NUMBER`: Set the priority of the accepted PR (defaults to 0).
- `rollup`: Mark the PR as likely to merge without issue, implies p=-1.
- `rollup-`: Unmark the PR as rollup.
- `retry`: Signal that the PR is not bad, and should be retried by buildbot.
- `try(=runner)`: Request that the PR be tested by buildbot, without accepting it.
- `force`: Stop all the builds on the configured builders, and proceed to the next PR.
- `clean`: Clean up the previous build results.
- `delegate=NAME`: Allow NAME to issue ALL homu commands for this PR
- `delegate+`: Delegate to the PR owner
- `delegate-`: Remove the delegatee
from rust:
- `rollup=maybe|always|iffy|never`: Mark the PR as "always", "maybe", "iffy", and "never" rollup-able.
