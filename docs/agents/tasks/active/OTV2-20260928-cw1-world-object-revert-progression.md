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
   progression input, bound to the existing FND-03 §10 authoritative-timer contract rather than a
   client command: the minimum real options grounded in code facts (no existing Foundation/global
   tick; an FND-03 timer over a monotonic `Deadline` from the authored `revert_after_ms`, RECOMMENDED;
   a synthetic scope-owned step counter, considered and superseded; a purely reactive re-evaluation,
   rejected), the must-decide-now test, the exact delta the owning lane (the scope-runtime/Foundation
   carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`, not this documentation task) must
   supply, and exact test obligations covering firing, fencing, capacity atomicity, equal-deadline
   ordering, clock-origin safety and occupied-target-cell handling (decided, not deferred). The owner
   question itself (who accepts this decision) is left explicit and unresolved by this task.
2. §4/§5 record, without designing it, the CW3 Content-model worker's delta (allocation
   `OTV2-20260928-cw3-local-object-state-model`: 1a per-state collision presence, 1b authored
   initial state validated fail-closed, 1c RETAG decision) and the C3 hardening clarification
   separating the supported fixed, bind-time-reserved collision footprint from out-of-scope
   dynamically materialized geometry. §8 Follow-up gets the new children, correctly split between
   CW3 (Content model), CW4 (runtime, ships without `revert_after` for now) and the scope-runtime/
   Foundation carrier lane (owns §7's progression-input decision).

No code, Foundation/runtime/protocol/registry, `content/**` or `tools/**` change. No claim that any
of this is `ACCEPTED`. D37 relocation and `SCOPE_HANDOFF` are untouched. §4/§8 now cite CW3 (PR
#1046) and CW4 (PR #1055) as merged, not pending, after merging `origin/main`.

## Architecture and source of truth

Full file:line evidence lives in §7 of the owned doc (Evidence subsection); this is the index.

- `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` — PROVEN, read in full.
- `world_runtime.rs` `bind`/`prepare` (~590-1055), `apply`/`resume_pending`/`terminalize_current`
  (~817-888) — PROVEN, post-#1055-merge line numbers: no tick/time parameter; every prepared outcome
  terminalizes immediately and replays on retry; collision no longer hard-wired to Open/Close.
- `foundation/runtime_actor_carrier.rs` `from_committed_assignment` (~702-750) — PROVEN,
  production-only: pins `scope_generation` once; `advance_owner` (~2169-2176) is test-only.
- `gameplay_transport/connection.rs` `Liveness::tick` (~296-320), `content/project/v2/creature.rs`
  `tick_profile` (~571-604), SIM-DETERMINISM-01 lines 224/419 — PROVEN: no scope/global clock.
  Bounded grep (`tokio::time::interval|sleep|select!\{|loop \{`) — PROVEN: every `ChannelRuntimeV1`
  call is reactive.
- `crates/foundation/src/time.rs` `Deadline`/`MonotonicClock`/`ManualClock`/`SystemClock::new()`
  (~50-166) — PROVEN: tested, unused in `apps/game-server/src`, fresh incomparable origin per call.
  `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144 (was 143) — PROVEN: authors `revert_after_ms`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §7/§9/§10/§14/§15.4/§28 — PROVEN: the ordering/scheduling/
  firing/cancellation/capacity/error-mapping contract §7 binds `revert_after` to. `foundation/mod.rs`
  `ScopeRuntimeFence::accept_input` (~1040-1050) — PROVEN: mints an ordinal per generation only,
  tracks no timer identity. `movement.rs` `MovementOwnerTurn` (~201-249) — PROVEN: existing bounded
  `max_inputs` precedent for due-timer admission bounding.
- PR #1055 (merged, `070d119`) generalized `LocalObjectRuntime` off Open/Close per D38; PR #1046
  (merged) shipped 1a/1b/1c. `git diff ac8395b8 origin/main` — PROVEN: neither touches
  `foundation/{mod,admission,runtime_actor_carrier}.rs`, `connection.rs`, `movement.rs` or
  `crates/foundation/src/time.rs`; only `world_runtime.rs` and the encounter doc's line numbers
  shifted.
- `world_runtime.rs` `LocalObjectCommand`/`transition_for` (~397-419, ~787-796),
  `TransitionBinding` (`content/reference_playable.rs` ~1334-1342) — PROVEN: every mutation needs a
  bound `TransitionKey`; `TransitionBinding` has no `revert_after_ms` field yet.

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

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3 in §4/§5/§8 without designing them.

Round 1 (coordinator): fixed §-numbering; CW3 attribution; §7 option 2 made honest (no proven
scope cadence). Round 2 (Codex, bbb3b4cd): added `Deadline` option as the recommendation; corrected
`advance_owner` to production-only evidence; decided occupied-revert refuses permanently. Round 3
(Codex, 350dca59): rebound §7 to an FND-03 §10 authoritative timer (own `RuntimeExecutionOrdinal`,
not `apply`/`resume_pending`/`CommandIngress`); staged timer-capacity atomicity; equal-deadline
tie-break; one shared clock instance per scope. Round 4 (owner-authorized, c76bf9b9): merged
`origin/main` (clean) picking up PR #1055 (CW4 runtime) and PR #1046 (CW3 1a/1b/1c), so §4/§8 cite
the merged state; re-verified every `world_runtime.rs` line citation; resolved 3 Codex P2s on FND-03
timer internals (ordinal-based equal-deadline tie-break in the timer key, not derived identity;
atomic pending-removal *before* `accept_input` since it tracks no timer identity; bounded due-work
admission per FND-03 §7/§14, mirroring `movement.rs`'s `MovementOwnerTurn`).

Round 5 (owner-authorized, cf3dd8e6, merged `origin/main` again — clean, no relevant file changed):
verified a new P1 — firing a revert calls `prepare`, which needs a concrete *bound* `TransitionKey`
(`LocalObjectCommand` ~412-419, `transition_for` ~787-796); nothing said which key restores an
arbitrary `TRANSFORM`/`RETAG`. Fixed, fail-closed, reusing the merged CW4 model: `revert_after_ms`
is admissible only on a transition whose *inverse* (a bound transition with swapped source/target
states for the same definition — covers TRANSFORM/CREATE/REMOVE/RETAG uniformly) is itself bound;
`bind` rejects (`InvalidBinding`) otherwise. The staged commit stores that inverse `TransitionKey`
plus the expected post-operation state/revision `prepare` already computes; firing calls `prepare`
with that key and expected revision, so an intervening change hits the existing `STALE_STATE` path
instead of a guess. Added 3 test obligations: missing inverse rejected at bind; revert restores
exactly the pre-operation state; intervening change ⇒ STALE_STATE, no mutation.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-5; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 5, after merging origin/main again; workflow
  count rose from 46 with the merge, unrelated to this task's own doc-only changes)

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
  PR #1045 pre-freeze round 5 (owner-authorized): merged origin/main again (clean); fixed a P1 —
  revert_after_ms now requires a bound inverse transition (fail-closed InvalidBinding at bind time),
  with the staged commit storing that inverse key plus the expected post-op state/revision so
  firing replays a validated delta instead of guessing; both validators re-run and still pass.
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
