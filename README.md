# Bo

Bors bot without real work. It handles webhooks (commands) and dispatches API calls to GitHub for Merge Queue.

Well the only real work are try builds, using few aditional REST api calls.

## Design priciples

- Stateless
- Wasm (cloudflare workers)
- Using GitHub Merge Queue
- lazy (minimal work/apr req if possible)

## Workflow automata

When we recive webhook it can be one off three types:

1. Command (classic bors command)
2. Workflow status (comeback?? or should this be dead)
3. app status (registered, updated, removed) (if applicateable)

## Commands

- `r+ (SHA)`: Accept a PR. Optionally, the SHA of the last commit in the PR can be provided as a guard against synchronization issues or malicious users. Regardless of the form used, PRs will automatically be unaccepted if the contents are changed.
- `r=NAME (SHA)`: Accept a PR on the behalf of NAME. (PR NAME EDIT)
- `r-`: Unacccept a PR.
- `retry (failed)`: Signal that the PR is not bad, and should be retried.
- `try(=runner)`: Request that the PR be tested, without accepting it.

need KV store (todo):

- `delegate=NAME`: Allow NAME to issue ALL homu commands for this PR
- `delegate+`: Delegate to the PR owner
- `delegate-`: Remove the delegatee

not possible (yet):

- `p=NUMBER`: Set the priority of the accepted PR (defaults to 0).
- `force`: Stop all the builds on the configured builders, and proceed to the next PR.
- `clean`: Clean up the previous build results.
- `rollup`: Mark the PR as likely to merge without issue, implies p=-1.
- `rollup-`: Unmark the PR as rollup.
- `rollup=maybe|always|iffy|never`: Mark the PR as "always", "maybe", "iffy", and "never" rollup-able.
