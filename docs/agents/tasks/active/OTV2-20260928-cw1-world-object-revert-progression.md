# OTV2-20260928-cw1-world-object-revert-progression

```yaml
task_id: OTV2-20260928-cw1-world-object-revert-progression
title: World-object revert_after progression owner and D38 follow-up model deltas
mode: CONTRACT
status: ready
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
  (~817-888), `LocalObjectCommand`/`transition_for` (~397-419/~787-796) — PROVEN, post-#1055-merge
  line numbers: no tick/time parameter; every mutation needs a bound `TransitionKey`; every prepared
  outcome terminalizes immediately and replays on retry; collision no longer hard-wired to Open/Close.
- `foundation/runtime_actor_carrier.rs` `from_committed_assignment` (~702-750) — PROVEN,
  production-only: pins `scope_generation` once; `advance_owner` (~2169-2176) is test-only.
- `gameplay_transport/connection.rs` `Liveness::tick`, `content/project/v2/creature.rs`
  `tick_profile`, SIM-DETERMINISM-01 lines 224/419 — PROVEN: no scope/global clock. Bounded grep
  (`tokio::time::interval|sleep|select!\{|loop \{`) — PROVEN: every `ChannelRuntimeV1` call reactive.
- `crates/foundation/src/time.rs` `Deadline`/`MonotonicClock`/`ManualClock`/`SystemClock::new()`
  (~50-166) — PROVEN: tested, unused in `apps/game-server/src`, fresh incomparable origin per call.
  `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144 (was 143) — PROVEN: authors `revert_after_ms`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §7/§9/§10/§14/§15.4/§28 — PROVEN: binds `revert_after`.
  `foundation/mod.rs` `ScopeRuntimeFence::accept_input` (~1040-1050) — PROVEN: mints an ordinal per
  generation only, tracks no timer identity. `movement.rs` `MovementOwnerTurn` (~201-249) — PROVEN:
  existing bounded `max_inputs` precedent.
- `TransitionBinding` (`content/reference_playable.rs` ~1334-1342) — PROVEN: no `revert_after_ms`
  field yet; only `LOCAL_OBJECT_RETAG_INTENT_FAMILY` (~1195/~1646) is a named intent family today.
- PR #1055 (merged, `070d119`) generalized `LocalObjectRuntime` off Open/Close per D38; PR #1046
  (merged) shipped 1a/1b/1c. `git diff ac8395b8 origin/main` — PROVEN: only `world_runtime.rs` and
  the encounter doc's line numbers shifted; all other cited files are untouched.

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

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3 in §4/§5/§8. Round 1: §-numbering/CW3
attribution fixed; §7 option 2 made honest. Round 2 (Codex, bbb3b4cd): `Deadline` option recommended
over a step counter; occupied-revert refuses permanently. Round 3 (Codex, 350dca59): rebound §7 to
an FND-03 §10 authoritative timer, not `apply`/`resume_pending`/`CommandIngress`; staged capacity
atomicity; equal-deadline tie-break; one clock per scope. Round 4 (owner-authorized, c76bf9b9):
merged `origin/main` (PR #1055 CW4, #1046 CW3), §4/§8 cite the merged state; line citations
re-verified. Round 5 (owner-authorized, cf3dd8e6): `revert_after_ms` admissible only on a transition
with a bound inverse (swapped states, same definition); staged commit stores the inverse key plus
expected state/revision so firing replays instead of guessing. Round 6 (owner-authorized, f3d05f1f):
inverse rule tightened to exactly one match (plus matching intent family); pre-`prepare` discard
restricted to `scope_generation`/`content_generation` changing, so a changed object always resolves
via `prepare`'s `DISPOSITION_STALE_STATE`, one path.

Round 7 (owner-authorized, 53c46f0d, Codex 4119354894): a mutually timed pair (both a transition and
its inverse carrying `revert_after_ms`) would ping-pong forever, since firing one timer's inverse
also staged a new timer. Fixed: a timer-origin execution never registers a new timer for itself.
Added the "fires once and stops" test obligation. This round's fix wording said "only player/command
... schedules," which round 8 found too narrow.

Round 8 (owner-authorized, 9ec951d3, Codex 4119401513, no new upstream commits): verified —
`OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md:171-172`'s `DepthWarzoneBossDeath` `creature_died(boss)` →
`map_item(transform teleporter, revert_after_ms 1200000)` is encounter-originated, never
`apply`/`resume_pending`, so round 7's wording would have left it transformed forever. Fixed: restated
as an origin test ("is this the firing of a pending revert timer?"), not an allow-list — every
non-timer-origin execution (player/command, encounter/server-event, or otherwise) registers its
revert. Fixed the same wording in §4, Option 2, both Exact-delta bullets, Must-decide-now item 1 and
the timer-capacity obligation. Added the "encounter-originated timed transform registers and fires"
test obligation, citing lines 171-172 directly.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-8; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 8, unchanged — no new upstream commits)

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
  PR #1045 pre-freeze round 8 (Codex 4119401513): round 7's "player/command-only" wording was too
  narrow (the encounter-originated DepthWarzoneBossDeath teleporter transform never registered).
  Restated as an origin test: suppress only for the firing of a pending revert timer; every other
  origin registers its revert; fixed the same wording everywhere in §7; validators pass.
status: ready
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
next_action: coordinator freeze at the reported head + independent review
```
