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
pr: 1045
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

1. A new §7 answers, for `revert_after` only, which existing owner supplies a scope-owned logical
   progression input: the minimum real options grounded in code facts (no existing Foundation/global
   tick; a new scope-owned monotonic step introduced by the scope runtime owner, honestly stated as
   a new mechanism since no existing scope cadence is proven; a purely reactive re-evaluation), the
   must-decide-now test, a recommended option, the exact delta the owning lane (the scope-runtime/
   Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`, not this documentation task)
   must supply, and exact test obligations. The owner question itself (who accepts this decision) is
   left explicit and unresolved by this task.
2. §4/§5 record, without designing it, the CW3 Content-model worker's delta (allocation
   `OTV2-20260928-cw3-local-object-state-model`: 1a per-state collision presence, 1b authored
   initial state validated fail-closed, 1c RETAG decision) and the C3 hardening clarification
   separating the supported fixed, bind-time-reserved collision footprint from out-of-scope
   dynamically materialized geometry. §8 Follow-up gets the new children, correctly split between
   CW3 (Content model), CW4 (runtime, ships without `revert_after` for now) and the scope-runtime/
   Foundation carrier lane (owns §7's progression-input decision).

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
- Bounded grep for the scope-runtime driver/cadence (`tokio::time::interval|tokio::time::sleep|
  select!\{|loop \{` cross-checked against every `ChannelRuntimeV1` use site:
  `gameplay_transport/{qualification,mod}.rs`, `movement.rs`, `node/serve.rs`,
  `foundation/{runtime_actor_carrier,mod}.rs`) — PROVEN: every located call into `ChannelRuntimeV1`
  is reactive (`gameplay_transport/mod.rs` `ComposedFreshAdmission::release_after_grace` ~473-514: a
  per-connection grace-expiry retry loop with its own backoff `sleep`; `movement.rs`
  `MovementOwnerTurn::begin`/`try_step` ~213-249: a bounded batch of movement inputs processed per
  invocation). `movement.rs` ~197-200 states explicitly in its own doc comment: "No production
  maximum, queue, command outcome, or scheduling authority is implied. Fairness remains an
  obligation of the future owner scheduler." No independent scope-wide cadence that advances
  regardless of command activity was found. UNKNOWN whether one exists outside this bounded read
  tree; §7 states this honestly rather than assuming a cadence to piggyback on.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Documentation-only architecture decision delta; no production mutation, authority grant,
  PREPARE/COMMIT, controller install/restore or persisted recovery evidence is touched.
```

## Acceptance criteria

- [x] §7 states the revert_after progression options, must-decide-now test, recommendation, owning
      lane's exact delta and test obligations, with `DecisionStatus` for the new part left CANDIDATE
      and the owner-acceptance question explicit.
- [x] §4/§5 record the 1a/1b/1c model delta (correctly attributed to CW3) and the C3
      fixed-footprint-vs-dynamic-geometry clarification, and §8 lists the new follow-up children
      (CW3 Content model, CW4 runtime, scope-runtime/Foundation carrier lane for §7's decision).
- [x] `python3 tools/agents/validate_governance.py` passes.
- [x] `python3 tools/repository/validate_repository_policy.py` passes.

## Excluded scope

- Any code change (`apps/game-server`, `crates/`, `content/**`, `tools/**`).
- Foundation/runtime/protocol/registry design or acceptance.
- Declaring the new §7 decision, or the 1a/1b/1c model delta, `ACCEPTED`.
- D37 relocation and `SCOPE_HANDOFF`.
- Jira; bulk Issue #162 history.

## Implementation / findings

The owned architecture doc already records D37/D38 as owner-accepted-pending-review. This task adds
one new decision delta (§7, revert_after progression owner/options) and records, without designing,
the CW3 Content-model worker's delta (1a/1b/1c) plus a hardening clarification (C3) that the
independent review asked for. See the diff for exact wording; see "Architecture and source of truth"
above for the file:line evidence behind every option and rejection.

Pre-freeze fix (coordinator, PR #1045 returned to AUTHORING): corrected §-numbering throughout this
task record (the decision is §7, Follow-up is §8, not the reverse); relabelled the Content-model
worker CW3 (not CW4) in §4/§8 of the owned doc and here; renamed §8 Follow-up item 7 from
"CW3/CW4 (scope-runtime lane)" to the actual owner, the scope-runtime/Foundation carrier lane
(`ChannelRuntimeV1`/`InstanceRuntime`), with CW4 adding `revert_after` on top once that lane
decides; and made §7's option 2 honest by adding the bounded scope-runtime-driver grep evidence
above (PROVEN: every located call is reactive; no proven scope-owned cadence) and rewriting option 2
so it states plainly that, absent a proven cadence, the owning lane's exact delta is introducing the
scope's own step driver (one per scope owner, not per object) as the one real new mechanism, still
CANDIDATE and not accepted here.

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after the pre-freeze fix commit; unchanged pass)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 45 workflows)." (re-run after the pre-freeze fix commit; unchanged
  pass)

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
last_progress: >
  PR #1045 returned to AUTHORING by the coordinator for pre-freeze fixes: task-record pr/§-numbering
  corrected, CW3/CW4 attribution fixed in the owned doc and this record, §7 option 2 made honest
  with new bounded scope-runtime-driver grep evidence; both validators re-run and still pass.
status: implementing
branch: claude/cw1-world-object-revert-progression
head_sha: null
pr: 1045
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
next_action: coordinator freeze + independent review
```
