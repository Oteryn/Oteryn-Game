# OTV2-20260928-cw1-world-object-revert-progression

```yaml
task_id: OTV2-20260928-cw1-world-object-revert-progression
title: World-object revert_after progression owner and D38 follow-up model deltas
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-world-object-revert-progression
pr: null
base_sha: ac8395b885e82d564d70968d6cae066fbc276e13
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world architecture (CW1)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/agents/tasks/active/OTV2-20260928-cw1-world-object-revert-progression.md
public_contracts: []
depends_on:
  - D38 (owner decision, 2026-09-27, OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md §6)
  - independent review ACCEPT_WITH_CONDITIONS (2026-09-27/28) on the same proposal
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

One decision delta recorded in `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`,
still `DecisionStatus: CANDIDATE` for the new part:

1. A new §8 answers, for `revert_after` only, which existing owner supplies a scope-owned logical
   progression input: the minimum real options grounded in code facts (no existing Foundation/global
   tick; the scope runtime's own new monotonic step; a purely reactive re-evaluation), the
   must-decide-now test, a recommended option, the exact delta the owning lane (the scope-runtime/
   Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`, not this documentation task)
   must supply, and exact test obligations. The owner question itself (who accepts this decision) is
   left explicit and unresolved by this task.
2. §4/§5/§7 record, without designing them, the separate CW3/CW4 worker's Content model delta
   (1a per-state collision presence, 1b authored initial state validated fail-closed, 1c RETAG
   decision) and the C3 hardening clarification separating the supported fixed, bind-time-reserved
   collision footprint from out-of-scope dynamically materialized geometry. The Follow-up list gets
   the new CW3/CW4 children.

No code, Foundation/runtime/protocol/registry, `content/**` or `tools/**` change. No claim that any
of this is `ACCEPTED`. D37 relocation and `SCOPE_HANDOFF` are untouched.

## Architecture and source of truth

- `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` (owned doc, CANDIDATE with
  D37/D38 taken) — PROVEN, read in full.
- `apps/game-server/src/world_runtime.rs` `LocalObjectRuntime::bind` (~570-712) and `::prepare`
  (~932-997) — PROVEN by file:line: no tick/time parameter anywhere in bind or prepare; fencing is
  placement/incarnation/content_generation/expected_revision only; collision is hard-wired to the
  Open/Close two-state pair (`next_blocking` match at ~978-981); `absolute_collision_cells` computes
  a fixed footprint once at bind time (~698).
- `apps/game-server/src/foundation/runtime_actor_carrier.rs` `ChannelRuntimeV1` (~695-700) and
  `advance_owner` (~2169-2176) — PROVEN: the only progression `ChannelRuntimeV1` exposes is
  `ScopeOwnershipGeneration` advancement (scope reassignment), not a time/tick/step counter.
- `apps/game-server/src/gameplay_transport/connection.rs` `Liveness::tick` (~296-320) — PROVEN: a
  per-connection transport keepalive cadence (probe/ack), unrelated to world/scope simulation state.
- `apps/game-server/src/content/project/v2/creature.rs` `tick_profile`/`tick_interval_ms`/
  `tick_counts` (~571-604, ~1189-1231) — PROVEN: an imported-content authoring schema for
  damage-over-time, not a runtime scheduler; no consumer in `apps/game-server/src` was found driving
  it as a live clock.
- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` lines 224 ("No
  universal fixed global tick is required.") and 419 ("global tick rate ... deliberately
  deferred.") — PROVEN: architecture-level, no committed global simulation tick exists today.
- Grep of `apps/game-server/src` and `crates/` for `tick|Tick|logical_time|logical_step|Instant|
  Clock|SimulationStep` found no scope-owned simulation-step concept beyond the two above —
  DERIVED (absence evidence, bounded to the read tree).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Documentation-only architecture decision delta; no production mutation, authority grant,
  PREPARE/COMMIT, controller install/restore or persisted recovery evidence is touched.
```

## Acceptance criteria

- [ ] §8 states the revert_after progression options, must-decide-now test, recommendation, owning
      lane's exact delta and test obligations, with `DecisionStatus` for the new part left CANDIDATE
      and the owner-acceptance question explicit.
- [ ] §4/§5/§7 record the 1a/1b/1c model delta and the C3 fixed-footprint-vs-dynamic-geometry
      clarification, and §7 lists the new CW3/CW4 follow-up children.
- [ ] `python3 tools/agents/validate_governance.py` passes.
- [ ] `python3 tools/repository/validate_repository_policy.py` passes.

## Excluded scope

- Any code change (`apps/game-server`, `crates/`, `content/**`, `tools/**`).
- Foundation/runtime/protocol/registry design or acceptance.
- Declaring the new §8 decision, or the 1a/1b/1c model delta, `ACCEPTED`.
- D37 relocation and `SCOPE_HANDOFF`.
- Jira; bulk Issue #162 history.

## Implementation / findings

The owned architecture doc already records D37/D38 as owner-accepted-pending-review. This task adds
one new decision delta (§8, revert_after progression owner/options) and records, without designing,
a separate worker's Content model delta (1a/1b/1c) plus a hardening clarification (C3) that the
independent review asked for. See the diff for exact wording; see "Architecture and source of truth"
above for the file:line evidence behind every option and rejection.

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: pending

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: pending

### E2E

- scenario: NOT_APPLICABLE — documentation-only, no runtime behavior to exercise.
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent (CW1), mandatory, not delegated
- material findings: pending
- verdict: pending

## Independent review

- required: YES — the coordination allocation routes this through the same independent-review path
  as the rest of the proposal; this task does not itself accept anything.
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: not requested by this task
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: task record created; owned architecture doc not yet edited
status: implementing
branch: claude/cw1-world-object-revert-progression
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Edit the owned architecture doc (§4/§5/§7/§8), then run both validators.
```
