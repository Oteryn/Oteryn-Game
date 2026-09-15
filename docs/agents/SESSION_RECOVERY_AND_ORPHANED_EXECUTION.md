# Session recovery and orphaned execution

## Principle

A UI spinner, old chat or claimed prior worker is not ownership evidence. Durable task/branch/PR/runner state is authoritative.

## Recovery checkpoint

Before a deliberate sleep, delayed recheck, runner job, terminal-CI wait or long command, persist:

```yaml
status:
branch:
head_sha:
final_head_sha:
final_head_frozen_at:
pr:
owned_paths:
last_progress:
next_action:
wait_started_at:
wait_deadline:
ci_trigger_source:
ci_generation:
check_suite_ids:
run_ids:
job_ids:
runner_id:
runner_labels:
steps_started:
runner_assignment_state:
counters:
owner_action_required:
external_dependencies:
```

A continuation session preserves original wait start, deadline, generations and counters; it must not reset budgets by starting a new chat.

## Recovery procedure

1. Read trusted-base governance and checkpoint.
2. Verify live default branch, task branch/head, frozen final head, PR, CI, reviews and ownership.
3. Detect drift or conflicting ownership.
4. Confirm whether the prior process/runner is still active using durable evidence.
5. Classify CI as normal waiting, event suppression, runner starvation, workflow failure or cancellation before any mutation.
6. Execute the recorded safe `next_action` immediately when valid.
7. If stale, update checkpoint or PR evidence without moving a frozen head merely to record status.

## Orphan determination

A task may be recovered as orphaned when the prior process is unavailable or its durable deadline expired and no conflicting agent owns the same branch, paths, PR, runner, deployment or protected operation.

Do not create duplicate branches/PRs merely because the previous chat is inaccessible.

A queued/pending Actions run is not proof of active execution when it has no assigned runner and no started steps. After the configured threshold, record `RUNNER_STARVATION`; do not call it a repository validation failure. Closing a PR does not prove the run was cancelled, and a replacement PR must not be created solely to escape the stale run.

## CI observations

One CI observation is one aggregate snapshot of all required checks for a PR/head. Querying workflows individually does not create extra allowed observations. Preserve check-suite, run and job IDs plus runner assignment state.

A missing run after the event grace period is `EVENT_SUPPRESSED`. Do not manufacture a new commit or metadata mutation to test the same hypothesis repeatedly. Use the single bounded trusted-dispatch path or record the exact owner action.

## Frozen-head continuation

When `final_head_sha` is set:

- verify the branch still points to that SHA before using prior audit or CI evidence;
- treat any head movement as invalidating the final generation;
- record new audit/CI evidence in immutable PR/workflow state instead of editing the task file;
- move the head only for a material repair;
- never reset wait/retry counters by replacing the PR or starting a new chat.

A commit cannot record its own SHA. The frozen exact head may therefore be recorded in the PR review, check run or external task tracker after the final repository commit exists.

## Controlled interruption

When safe progress cannot continue, persist accurate `ready`, `waiting` or `blocked` state with exactly one next action and return the matching terminal result. Never claim hidden background continuation.
