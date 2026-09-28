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
   client command: minimum real options grounded in code facts (an FND-03 timer over a monotonic
   `Deadline` from authored `revert_after_ms`, RECOMMENDED; a step counter, superseded; reactive
   re-evaluation, rejected), the must-decide-now test, the exact delta the owning lane (the
   scope-runtime/Foundation carrier lane behind `ChannelRuntimeV1`/`InstanceRuntime`, not this task)
   must supply — including a canonical complete pending-entry field list (Round 11) — and exact test
   obligations. The owner question itself (who accepts this decision) stays explicit and unresolved.
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
  (~817-888), `LocalObjectCommand`/`transition_for` (~397-419/~787-796) — PROVEN: no tick/time
  parameter; every mutation needs a bound `TransitionKey` plus `placement`/`incarnation`/
  `content_generation`; `prepare`'s first check (~978-987) rejects a mismatch with
  `DISPOSITION_BINDING_MISMATCH`, distinct from `DISPOSITION_STALE_STATE`; every prepared outcome
  terminalizes immediately and replays on retry.
- `foundation/runtime_actor_carrier.rs` `from_committed_assignment` (~702-750) — PROVEN,
  production-only: pins `scope_generation` once. No scope/global clock (`gameplay_transport/
  connection.rs`, `content/project/v2/creature.rs`, SIM-DETERMINISM-01 224/419; bounded grep for
  interval/sleep/select!/loop) — PROVEN.
- `crates/foundation/src/time.rs` `Deadline`/`MonotonicClock`/`ManualClock`/`SystemClock::new()`
  (~50-166) — PROVEN: tested, unused in `apps/game-server/src`, fresh incomparable origin per call.
  `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 144 — PROVEN: authors `revert_after_ms`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §7/§9/§10/§14/§15.4/§28 — PROVEN: binds `revert_after`.
  `foundation/mod.rs` `ScopeRuntimeFence::accept_input` (~1040-1050) — PROVEN: mints an ordinal per
  generation only, tracks no timer identity. `movement.rs` `MovementOwnerTurn` (~201-249) — bounded
  `max_inputs` precedent.
- `TransitionBinding` (`content/reference_playable.rs` ~1334-1342) — no `revert_after_ms` field yet.
- `GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md` §4.1/§4.4/§5.1/§5.8 and
  `interaction/identity.rs` `RootSourceOccurrenceRef`/`ChildOccurrenceRef` (~15-157) — PROVEN, read
  for round 11: nested-cascade child identity; ordinal is authority-fence evidence, not identity.
- PR #1055/#1046 (merged) generalized `LocalObjectRuntime`, shipped 1a/1b/1c. `git diff ac8395b8
  origin/main` — only `world_runtime.rs`/encounter doc line numbers shifted.

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

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3. Rounds 1-2: numbering/CW3 attribution;
`Deadline` option recommended. Round 3: rebound §7 to an FND-03 §10 authoritative timer; staged
capacity atomicity; equal-deadline tie-break; one clock per scope. Round 4: merged `origin/main` (PR
#1055 CW4, #1046 CW3). Round 5: `revert_after_ms` admissible only with a bound inverse; staged commit
stores inverse key + expected state/revision. Round 6: inverse rule tightened to exactly one match;
pre-`prepare` discard restricted to `scope_generation`/`content_generation`. Round 7 (Codex
4119354894): a mutually timed pair would ping-pong; fixed — timer-origin execution never re-arms.
Round 8 (Codex 4119401513): round 7's wording would have starved the encounter-originated
`DepthWarzoneBossDeath` teleporter revert; restated as an origin test. Round 9 (Codex 4119452692):
pending entry lacked the target's `PlacementKey`/`incarnation`/`content_generation`; fixed — entry
retains exact target identity captured at scheduling; changed `incarnation` joins the pre-`prepare`
discard fences. Round 10 (Codex 4119516262, main→`74bb3fd3`, unrelated): Option 2's summary sentence
still said only `scope_generation`/`content_generation` invalidate the timer, contradicting round 9's
`incarnation` fence; fixed there and in the round-6 narrative after a whole-document grep.

Round 11 (Codex 4119565077, on frozen head `87b974b1`, main unmoved): every round asserted the
revert's own "derived child identity" (GAME-INTERACTION-01 §5.1 nested-cascade, implemented in
`interaction/identity.rs`) but never stored it, and it must never derive from the scheduling
`RuntimeExecutionOrdinal` (§4.4/§5.8: authority-fence evidence, not logical identity) — the fourth
one-field-per-round gap. Fixed structurally per coordinator instruction: one canonical "Pending
entry: complete field list" table (10 rows + sourcing) atop "Exact delta" covers every firing-path
input (addressing, fencing, ordering, `LocalObjectCommand` fields, terminal identity, replay/dedup;
capacity release needs no separate field); Option 2, Must-decide-now item 1, and the staging/firing/
occupancy/"Fenced, one path" text now reference it instead of re-enumerating fields. Firing
terminalizes every disposition under the entry's stored `InteractionChildOccurrenceRef`. Added the
redelivery-replays/distinct-occurrence-distinct-identity test obligation. Verified against
GAME-INTERACTION-01 §4.1/§4.4/§5.1/§5.8 and `interaction/identity.rs` directly before writing.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-11; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 11, main unmoved since round 10; unchanged)

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

- required: YES — routed through the same independent-review path as the rest of the proposal; this
  task does not itself accept anything.
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
  PR #1045 pre-freeze round 11 (Codex 4119565077 on frozen head 87b974b1): pending entry never stored
  the revert's own derived InteractionChildOccurrenceRef. Fixed structurally: one canonical "Pending
  entry: complete field list" table in Exact delta that every other place in §7 now references;
  firing terminalizes every disposition under the entry's stored identity, never the scheduling
  ordinal; added redelivery-replays/distinct-occurrence test obligation. Verified against
  GAME-INTERACTION-01 §4.1/§4.4/§5.1/§5.8 and interaction/identity.rs directly; validators pass.
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
