# Anti-stall and execution policy

```yaml
anti_stall_policy_version: 3.0-oteryn-v2
continuous_progress_execution: true
wall_clock_execution_windows: false
no_progress_minutes: 15
max_ordinary_ci_observations_per_exact_head: 2
ci_event_grace_minutes: 2
runner_assignment_stall_minutes: 10
max_ci_recovery_actions_per_exact_head: 1
terminal_ci_wait_budget_minutes: 45
terminal_ci_minimum_interval_minutes: 3
max_terminal_ci_observations_per_generation: 12
max_repair_cycles_per_gate: 3
max_identical_failure_retries_without_new_hypothesis: 1
max_additional_tasks_after_entry_task: 1
```

## Purpose

Autonomous work is progress-bounded and evidence driven. Productive implementation is **not** limited by a 60-minute, 120-minute, per-window or per-invocation wall-clock budget. A worker that is making material progress continues until the allocated task is complete, reaches a genuine evidence-backed blocker, is explicitly stopped by the owner, or hits one of the anti-stall conditions below.

The anti-stall controls exist to prevent endless polling, repeated identical failures, context reconstruction loops, PR/event regeneration and unproductive waiting. They MUST NOT be converted into periodic implementation stops, worker rotations, fresh execution-window grants or discarded productive minutes.

Any older prompt, task, plan, checkpoint or programme document that says `60-minute window`, `120-minute budget`, `foreground budget`, `remaining productive minutes`, `windowN`, or equivalent is historical execution bookkeeping only. It does not require a stop, rotation, new grant, budget reset or re-admission while the same authorized task can continue safely and productively.

## Measurable progress

Progress means at least one material event:

- coherent code/config/test/document/task state persisted;
- new validation evidence or a narrowed failure;
- a root cause repaired with a new proving result;
- branch/PR/CI/review/dependency state materially changed;
- a material review/audit finding opened, resolved or reclassified;
- task/PR reached an intentional terminal state.

Repeated reads, unchanged checks, duplicate summaries, waiting, activity-only commits, branch rewinds, close/reopen cycles and replacement PRs created only to regenerate CI are not progress.

There is no periodic checkpoint requirement based only on elapsed implementation time. Persist a durable checkpoint when it materially helps handover/recovery, before a genuine stop/rotation, or when the task becomes waiting/blocked.

## Required checkpoint fields

For autonomous or failure-prone work record when applicable:

```yaml
last_progress_at:
final_head_sha:
final_head_frozen_at:
ci_trigger_source:
ci_checks_for_current_head: 0
ci_check_generation:
ci_run_ids: []
runner_assignment_state:
terminal_ci_wait_started_at:
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required:
```

Reset counters only after the exact head, failure signature, hypothesis, external state or required-check generation materially changes. Elapsed productive time does not reset or exhaust task authority.

## Final-head freeze

Before final review and exact-head CI:

1. finish implementation, task record, PR title/body, changed-scope declaration and all known closeout metadata;
2. create the smallest coherent final commit or atomic commit set;
3. record `final_head_sha` and `final_head_frozen_at` in immutable PR/task-tracker evidence after the commit exists;
4. perform the mandatory full-diff self-review and run focused validation;
5. when the trusted-base risk policy/owner/contract requires independent review, perform it on the same unchanged SHA;
6. run required exact-head CI on that unchanged SHA.

After the freeze:

- do not commit a checkpoint, timestamp, review/audit verdict, CI status or PR number merely to document progress;
- place exact-head review/audit and CI evidence in the PR review, workflow run, artifact or external task tracker without moving the head;
- do not amend, force-push, rewind, close/reopen or replace the PR merely to create another event;
- move the head only for a material content repair based on an explicit finding or failed validation;
- every moved head invalidates the prior exact-head self-review, any required independent review and exact-head CI generation and requires a new freeze.

A commit cannot contain its own SHA. Do not create a self-referential follow-up commit merely to fill `final_head_sha` inside the repository task file.

Post-merge archive metadata is a separate bounded closeout change when it cannot be known before merge. It must not be smuggled into the delivery PR after the final-head freeze.

## CI state classification

Classify the observed condition before taking a recovery action:

- `EVENT_SUPPRESSED` — after the event grace period, the exact head has no check suite, check run or workflow run for the required workflow. A successful GitHub API write does not prove an Actions event was emitted.
- `RUNNER_STARVATION` — a run/job exists for the exact head, remains queued or pending beyond the assignment threshold, has no assigned runner (`runner_id = 0` or equivalent) and has started no steps.
- `WORKFLOW_FAILURE` — the job received a runner, executed at least one step and completed unsuccessfully.
- `WORKFLOW_CANCELLED` — a run reached a terminal cancelled state; determine whether concurrency, a moved head or an explicit cancellation caused it.
- `WAITING_NORMALLY` — a run exists, has an assigned runner or recent queue progress and remains within the bounded wait policy.

`EVENT_SUPPRESSED` and `RUNNER_STARVATION` are infrastructure/trigger states, not evidence that repository validation failed.

## Bounded CI recovery order

For one frozen exact head:

1. inspect the exact SHA, PR state, required context, check suites, workflow runs, job assignment and repository Actions permissions;
2. if a terminal failed/cancelled run exists and a new hypothesis justifies it, rerun it once;
3. otherwise use one trusted `workflow_dispatch` recovery run that validates the open PR number and exact frozen head;
4. if the active connector cannot dispatch or cancel Actions,