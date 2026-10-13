# SEC-1

```yaml
task_id: SEC-1
title: "SEC-1: server-side integrity checks (bot controls), gap record and proposed scope"
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/sec-1-record-20261013
issue: 2054
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session_015PVevC8C3J6ZTkLXAYy6b9 (SEC-1 worker)
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/SEC-1.md
public_contracts: []
depends_on:
  - PVP-RT-1b (or the live owner of field_step_ingress.rs; PVP-RT-1 is archived and handed the movement hook wiring to PVP-RT-1b; recheck at allocation)
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Sub-issue #2054, plan row 135 (PR #2047 §4): server-side integrity checks (bot controls), class impl, no migration, no registry.
This record only states the gap and the proposed scope; no code until the control plane decides.

## Architecture and source of truth

- `PROVEN`: plan row 135 names no concrete target. The only contract is the SEC-CLIENT-01 decision
  (`docs/architecture/reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md`):
  CLIENT_INTEGRITY_V1 and INPUT_TELEMETRY_V1 need protocol/registry work (outside SEC-1); Layer 2 says server-side timing weighs more.
- `PROVEN`: step pacing already exists server-side in `movement/pacing.rs` (`StepPacer`, TOO_EARLY, one-step buffer) and
  `connection.rs`; both are outside the owned paths (`movement/` belongs to LOOP-WORLD-1).
- `PROVEN`: no bot/anti-cheat code exists in `fresh_evidence.rs`, `field_step_ingress.rs` or `actor_movement.rs`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` for this record: docs only. Reassess if code is allocated.

## Acceptance criteria

- [ ] Control plane/architect names the bot-control target, or confirms the proposed scope below.

## Excluded scope

Protocol/registry, migrations, `movement/`, the client (#1942), sanctions (OPS-GM-01), CLIENT_INTEGRITY_V1/INPUT_TELEMETRY_V1.

## Implementation / findings

Gap: no localized bot-control target in the owned paths.
Proposed scope (b), narrowed to failures that reach the owned modules: server-authoritative step integrity in `actor_movement.rs`
and `field_step_ingress.rs`: a step is committed only when origin, destination and the SPEED-1 duration agree with server facts;
refusals there are counted as a bounded, non-sanctioning signal; negative cases in `authority_invariants.rs`, one invariant
changed per case. Sequence gaps and early/buffered steps are decided in `connection.rs` (about lines 2035-2051 and 2093-2113)
before `paced_step`; counting them needs `connection.rs` added to the allocation, which the control plane must decide.
Proposed paths, not owned until the control plane allocates them (after the PVP-RT-1b ownership check):
`apps/game-server/src/gameplay_transport/{fresh_evidence,field_step_ingress,actor_movement}.rs` and
`apps/game-server/tests/authority_invariants.rs`.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: NO for this record; YES if code is allocated
- exact head: NOT_APPLICABLE

## PR and closeout

- PR #2079 carries this record (follow-up to merged #2078, which carried the superseded earlier version at docs/agents/tasks/SEC-1.md); no code until the control plane answers

## Context checkpoint

```yaml
last_progress: gap record written; control plane chose option (c)
status: blocked
branch: claude/sec-1-record-20261013
head_sha: null
pr: null
owner_action_required: null
blocker: bot-control target not named; architect decision pending at control plane
next_action: wait for control plane decision, then implement scope (b) or the named target
```
