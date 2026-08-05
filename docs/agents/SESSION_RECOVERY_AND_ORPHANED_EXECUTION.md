# Session recovery and orphaned execution

## Principle

A UI spinner, old chat or claimed prior worker is not ownership evidence. Durable task/branch/PR/runner state is authoritative.

## Recovery checkpoint

Before a deliberate sleep, delayed recheck, runner job, terminal-CI wait or long command, persist:

```yaml
status:
branch:
head_sha:
pr:
owned_paths:
last_progress:
next_action:
wait_started_at:
wait_deadline:
ci_generation:
run_ids:
counters:
external_dependencies:
```

A continuation session preserves original wait start, deadline, generations and counters; it must not reset budgets by starting a new chat.

## Recovery procedure

1. Read trusted-base governance and checkpoint.
2. Verify live default branch, task branch/head, PR, CI, reviews and ownership.
3. Detect drift or conflicting ownership.
4. Confirm whether prior process/runner is still active using durable evidence.
5. Execute recorded safe `next_action` immediately when valid.
6. If stale, update checkpoint with the new exact state before continuing.

## Orphan determination

A task may be recovered as orphaned when the prior process is unavailable or its durable deadline expired and no conflicting agent owns the same branch, paths, PR, runner, deployment or protected operation.

Do not create duplicate branches/PRs merely because the previous chat is inaccessible.

## CI observations

One CI observation is one aggregate snapshot of all required checks for a PR/head. Querying workflows individually does not create extra allowed observations. Preserve run IDs and generation counters.

## Controlled interruption

When safe progress cannot continue, persist accurate `ready`, `waiting` or `blocked` state with exactly one next action and return the matching terminal result. Never claim hidden background continuation.
