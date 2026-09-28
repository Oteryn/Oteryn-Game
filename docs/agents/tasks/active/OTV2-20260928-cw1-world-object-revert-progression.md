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
   must supply — including one canonical scope-owned lifecycle record per identity, `PENDING`→
   `IN_FLIGHT`→`TERMINAL` (Round 14) — and exact test obligations. The owner question itself (who
   accepts this decision) stays explicit and unresolved.
2. §4/§5 record, without designing it, the CW3 Content-model worker's delta (1a per-state collision
   presence, 1b authored initial state validated fail-closed, 1c RETAG decision) and the C3
   hardening clarification (fixed, bind-time-reserved collision footprint vs. out-of-scope dynamic
   geometry). §8 Follow-up splits children between CW3, CW4 and the scope-runtime/Foundation carrier
   lane (owns §7's progression-input decision).

No code, Foundation/runtime/protocol/registry, `content/**` or `tools/**` change. No claim that any
of this is `ACCEPTED`. D37 relocation and `SCOPE_HANDOFF` are untouched.

## Architecture and source of truth

Full file:line evidence lives in §7 of the owned doc (Evidence subsection); this is the index.

- `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` — PROVEN, read in full.
- `world_runtime.rs` `bind`/`prepare` (~590-1055), `apply`/`resume_pending`/`terminalize_current`
  (~817-888) — PROVEN: no tick/time parameter; every mutation needs a bound `TransitionKey` plus
  `placement`/`incarnation`/`content_generation`; `prepare`'s first check (~978-987) rejects a
  mismatch with `DISPOSITION_BINDING_MISMATCH`, distinct from `DISPOSITION_STALE_STATE`.
- `foundation/runtime_actor_carrier.rs` `from_committed_assignment` (~702-750) — production-only:
  pins `scope_generation` once. No scope/global clock — PROVEN.
- `crates/foundation/src/time.rs` `Deadline`/`ManualClock`/`SystemClock::new()` (~50-166) — unused
  in `apps/game-server/src`, fresh incomparable origin per call. Encounter doc line 144 — authors
  `revert_after_ms`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §7/§9/§10/§14/§15.4/§28 — binds `revert_after`.
  `foundation/mod.rs` `ScopeRuntimeFence::accept_input` (~1040-1050) — mints an ordinal per
  generation only, tracks no timer identity. `movement.rs` `MovementOwnerTurn` — bounded precedent.
- `GAME-INTERACTION-01_..._CANDIDATE.md` §4.1/§4.4/§5.1/§5.8/§7(265-280)/§5.9/§25, `interaction/
  identity.rs` `ChildOccurrenceRef` (~69-157) — nested-cascade identity; §7 mandates retention/
  convergence; §5.9/§25 leave the numeric bound unfrozen — the owning contract for round 14's record.
- `foundation/mod.rs` `CommandIngress` (~53-742) — `CommandId`-keyed, single-slot, session-gated.
- `world_runtime.rs` `prepare`/`PreparedTerminal::unchanged`/`PreparedMutation` (~973-1159) — round
  15: every `unchanged(...)` disposition builds `PreparedMutation::None`; only `COMMITTED` builds
  `Publish` — the complete, exhaustive set. `gameplay_transport/mod.rs` `ComposedFreshAdmission::step`
  (~546-585) — one lock `.await`, then synchronous; no panic/abort/task-supervision code found for
  scope-owner work.
- `content/reference_playable.rs` `LocalObjectStateDefinition` (~810-813) — `key`+`collision` only.
  `TransitionBinding` (~1334-1342) — no `revert_after_ms`, no attribute payload.
  `tools/content-schema/encounter-authoring/validate_encounter.py` (~188-201) + README ("the server
  does not read these files") — only boundary seeing the full action, offline, not wired.
- Rounds 18/19: `encounter-authoring/samples/*/encounter.json` exhaustive check — Depth trio (lines
  65-68/75-78/75-78) carry `destination`+`revert_destination`, as does every `transform`; the only
  `create` actions without them (`mazzinor`/`gaz_haragoth`/`cult_soul_remains`) carry `interaction`,
  now classified non-state (encounter-format doc line 144) — no authored sample admits
  `revert_after_ms` today.
- PR #1055/#1046 (merged) generalized `LocalObjectRuntime`, shipped 1a/1b/1c.

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
- Declaring §7 or the 1a/1b/1c model delta `ACCEPTED`.
- D37 relocation and `SCOPE_HANDOFF`.
- Jira; bulk Issue #162 history.

## Implementation / findings

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3. Rounds 1-16: numbering; `Deadline`
option; FND-03 §10 timer; capacity atomicity; equal-deadline tie-break; one clock/scope; exactly one
bound inverse; pre-`prepare` discard; never re-arms; origin test; exact target identity; fixed stale
summary; derived `InteractionChildOccurrenceRef`; no-op/nothing retained (12); superseded — violated
GAME-INTERACTION-01 §7, outcome retained (13); superseded — two P1 seam bugs, one lifecycle record
`PENDING|IN_FLIGHT|TERMINAL` (14); convergence — exhaustive `TERMINAL` mapping, removed unproven
"never..." claims, added "Open decisions" (15); scoped `revert_after_ms` to state+collision only,
wrongly said `bind` rejects an attribute-changing transition (16).

Round 17 (`a1cad472`): fixed round 16 — `TransitionBinding` (7 fields, no attribute payload) never
sees `destination`. Moved rejection to authoring/lowering (`validate_encounter.py`'s `map_item`
block, offline, not wired — an obligation). Fixed a stale §5 "no receipt store" summary.

Round 18 (Codex 4120357243, head `95f7db70`): `DepthWarzoneBossDeath` wrongly called state-only —
all three Depth encounters carry `destination`+`revert_destination` (lines 65-68/75-78/75-78); whole
sample corpus checked: only `mazzinor`/`gaz_haragoth`/`cult_soul_remains` creates omit both but carry
`interaction`. Fixed every state-only claim; replaced test-obligation fixture with labeled synthetic
transform. Merged `origin/main` (`8d320703`, unrelated); no cited file drifted.

Round 19 (Codex 4120487852/4120487841/4120487862, head `4e450a08`): **P1** — classified `interaction`
a non-state attribute (encounter-format line 144: content-domain binding, not state), the same class
as `destination`/`revert_destination`; consequence: no currently authored `map_item` action is
admissible for `revert_after_ms` at all (Depth trio + `mazzinor`/`gaz_haragoth`/`cult_soul_remains`);
synthetic fixture remains the only runnable case. Added an interaction-bearing-create-rejected test
obligation. **P2s (Open decisions only, no redesign):** item 4 — removed the claim that
`revert_after_ms` lives on `TransitionBinding` (home is the owning lane's choice, plausibly the
lowered operation occurrence). Item 5 — qualified compaction/tombstone text: a tombstone still costs
one entry per occurrence; a bounded duplicate-delivery horizon is needed before compaction reclaims
capacity.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-19; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 19; unchanged)

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

- required: YES.
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
  PR #1045 pre-freeze round 19 (Codex 4120487852/4120487841/4120487862 on frozen head 4e450a08):
  P1 -- classified interaction a non-state attribute, same class as destination/revert_destination;
  no authored map_item action is admissible for revert_after_ms today; added a test obligation.
  P2s (no redesign): Open decision 4 (TransitionBinding is not the confirmed home) and Open
  decision 5 (tombstone compaction needs a bounded duplicate-delivery horizon too). Validators pass.
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
