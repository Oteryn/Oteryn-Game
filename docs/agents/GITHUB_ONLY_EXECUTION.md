# Game GitHub CI recovery extension

Use the execution and integration policy bound by `META_AGENT_POLICY_BINDING.json`. This Game-owned extension records failure modes of the repository's required GitHub workflows.

## Connector-triggered event limitations

A successful repository mutation does not prove that GitHub Actions emitted or accepted an event for that exact head. After the final branch/PR mutation, record the head, allow the configured event grace period, and query check suites or workflow runs for that SHA. Classify an absent run as `EVENT_SUPPRESSED`; classify a queued job with no runner or started step beyond the configured threshold as `RUNNER_STARVATION`. Do not mutate content merely to manufacture another event.

## Trusted manual dispatch recovery

A required workflow's manual recovery must bind the selected ref, open PR number and full expected head SHA. The workflow must verify the same-repository PR still targets `main` at that SHA, then check out and validate exactly that candidate while preserving the required check name.

If the active connector lacks dispatch or cancellation, record the exact unavailable action, PR, branch, head and required check. Do not weaken repository protection.

## Temporary workflows

A workflow removed before merge cannot prove its own removal commit. Final exact-head evidence must come from a retained trusted workflow or another repository-approved immutable validator. Do not use `pull_request_target` with PR-controlled code to recover a suppressed event.
