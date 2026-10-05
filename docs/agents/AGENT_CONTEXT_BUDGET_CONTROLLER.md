# Agent Context Budget Controller

Status: **DRAFT shared agent-operating mechanism**

Jira: `KAN-42`

## Purpose

Prevent long-lived Oteryn agent sessions from degrading because the active context becomes too large or too fragmented.

This mechanism is a continuation policy, not a wall-clock execution budget. It must not reintroduce fixed work windows prohibited by `ANTI_STALL_AND_EXECUTION_BUDGET.md`.

## Accounting source

Every report must name one source:

- `EXACT` — the execution runtime exposes used-context and context-window telemetry;
- `ESTIMATED` — a conservative estimator is fed the material loaded/emitted by the agent;
- `UNAVAILABLE` — neither exact nor calibrated estimated usage is available.

Never invent an exact percentage. If source is `UNAVAILABLE`, structural pressure signals drive rotation.

## Default pressure states

When exact or calibrated estimated percentage exists, the initial configurable defaults are:

| Effective pressure | State | Behavior |
| --- | --- | --- |
| < 65% | `NORMAL` | ordinary bounded work |
| 65–78% | `ECONOMY` | targeted reads, smaller outputs, delegate read-only discovery |
| 78–85% | `PREPARE_HANDOFF` | keep successor checkpoint current; avoid broad new investigation |
| 85–90% | `HANDOFF_REQUIRED` | finish the current safe boundary and persist handoff |
| > 90% | `NO_NEW_LARGE_WORK` | do not begin another material tranche in this chat |

These are operational defaults, not product/resource architecture maxima. They may be tuned from measured runtime behavior without changing gameplay semantics.

## Effective pressure

Token percentage is not the only signal. The lead also records:

- compaction count;
- unusually large tool/file outputs;
- number of simultaneously tracked tasks/PRs;
- unresolved material findings;
- frequent subsystem switching.

The controller may conservatively promote the state by one level when these structural signals make the remaining context materially harder to use.

A compaction is evidence of pressure, not proof of an exact percentage.

## ESTIMATED mode

Estimated mode may count known prompt/message/tool/file bytes or tokens when the execution host exposes them to the lead.

Rules:

1. state the estimator and reserve assumptions;
2. prefer overestimation to late rotation;
3. never present the result as provider billing/token telemetry;
4. do not count inaccessible hidden context as if it were measured;
5. if calibration is not trustworthy, report `UNAVAILABLE` and use structural signals.

## Required successor checkpoint

Before rotation, preserve the smallest state sufficient for a fresh chat to continue:

```yaml
programme: PLAYER_SWARM
main_sha:
jira_epic: KAN-34
current_story:
task_id:
branch:
pr:
head_sha:
completed: []
coverage_delta: {}
open_findings: []
architecture_escalations: []
waiting_dependencies: []
next_action:
lazy_refs: []
context_budget:
  source: EXACT | ESTIMATED | UNAVAILABLE
  usage_percent:
  pressure_state: NORMAL | ECONOMY | PREPARE_HANDOFF | HANDOFF_REQUIRED | NO_NEW_LARGE_WORK
  compactions:
```

The checkpoint belongs on an already-canonical programme/task/PR/control-plane surface. Do not create an activity-only commit merely to record a percentage.

If no writable canonical surface is available, the lead returns the compact successor packet and a fresh invocation reconstructs authoritative state from protected main, Jira and live task/PR facts instead of treating chat history as authority.

## Fresh-chat recovery

A new session invoked with:

```text
Oteryn: player swarm lead
```

must:

1. resolve the matching reusable lifecycle entry and prompt;
2. resolve protected `main`;
3. read the Player Swarm programme and KAN-34 child state;
4. find the current task/branch/PR/head/check state for the active tranche;
5. consume the latest canonical successor checkpoint when one exists;
6. continue the single recorded `next_action` unless live state proves it stale.

Do not replay completed work merely because the chat is new.

## Silent operation

Context rotation is routine operation and does not justify owner status spam.

Only owner decisions, owner-only blockers/material problems and the existing concise terminal completion convention may interrupt silent operation.

## Shared adoption

Player Swarm is the pilot consumer. After qualification, the same mechanism may be adopted by Work Coordinator, Supervising Architect and other long-lived leads without creating additional control planes.

