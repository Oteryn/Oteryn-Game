# Anti-stall and execution budget

```yaml
anti_stall_policy_version: 2.2-oteryn-v2
normal_foreground_runtime_minutes: 60
large_foreground_runtime_minutes: 120
no_progress_minutes: 15
max_ordinary_ci_observations_per_exact_head: 2
terminal_ci_wait_budget_minutes: 45
terminal_ci_minimum_interval_minutes: 3
max_terminal_ci_observations_per_generation: 12
max_repair_cycles_per_gate: 3
max_identical_failure_retries_without_new_hypothesis: 1
max_additional_tasks_after_entry_task: 1
minimum_remaining_minutes_for_additional_task: 30
```

## Purpose

Autonomous work is foreground, bounded and evidence driven. It must not become an endless polling, retry, context reconstruction, PR creation or task-selection loop.

## Measurable progress

Progress means at least one material event:

- coherent code/config/test/document/task state persisted;
- new validation evidence or a narrowed failure;
- a root cause repaired with a new proving result;
- branch/PR/CI/review/dependency state materially changed;
- a material audit finding opened, resolved or reclassified;
- task/PR reached an intentional terminal state.

Repeated reads, unchanged checks, duplicate summaries, activity-only commits and waiting are not progress.

## Required checkpoint fields

For autonomous or failure-prone work record when applicable:

```yaml
invocation_started_at:
last_progress_at:
ci_checks_for_current_head: 0
ci_check_generation:
terminal_ci_wait_started_at:
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
```

Reset counters only after the exact head, failure signature, hypothesis, external state or required-check generation materially changes.

## Ordinary waiting

Outside final terminal CI:

1. observe required CI/external state once when expected;
2. allow at most one later unchanged observation;
3. configure protected auto-merge once when eligible;
4. persist exact head, run IDs, state and one next action;
5. stop/rotate or execute genuinely independent work already inside the same task.

Do not keep a worker active only to wait.

## Bounded terminal CI

A foreground invocation may remain active through final exact-head CI and merge only when implementation, audit, E2E, review hygiene and all non-CI gates are complete and the final head is frozen.

During this exception:

- total wait is capped at 45 minutes or remaining invocation budget;
- unchanged observations are at least three minutes apart;
- at most 12 observations are allowed per materially new required-check generation;
- new generations do not reset the total wait budget;
- a failure exits waiting and enters the repair loop;
- after success re-check head, checks, reviews, ownership and mergeability before merge.

## Failure loop

- Analyze the first actionable failure.
- Make one targeted repair based on an explicit hypothesis.
- An identical second failure requires a new hypothesis, instrumentation or narrower isolation.
- Never repeat the same failure again without new evidence.
- After three repair cycles for one gate, persist evidence and return `BLOCKED` or `ROTATE`.

## Stop handling

On budget/no-progress/retry/repair exhaustion:

1. stop polling and starting new work;
2. preserve coherent state;
3. record exact last progress, unchanged state, counters and attempted hypotheses;
4. set task `ready`, `waiting` or `blocked` accurately;
5. leave exactly one `next_action`;
6. return `WAITING`, `BLOCKED` or `ROTATE`.

`ROTATE` is an invocation result, never a task status.

## Canonical terminal report

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT:
CHANGED_PATHS:
VALIDATION:
AUDIT:
E2E:
PR_HYGIENE:
LAST_PROGRESS:
BUDGET:
UNCHANGED_STATE:
DURABLE_STATE:
BLOCKER:
NEXT_ACTION:
```
