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
  identity.rs` `ChildOccurrenceRef` (~69-157) — nested-cascade identity; §7: "duplicate delivery ...
  MUST converge to one lifecycle/outcome," "loss of a retained result payload MUST NOT re-enable
  execution"; §5.9/§25: retention window/count unfrozen — the owning contract for round 14's record.
- `foundation/mod.rs` `CommandIngress` (~53-742) — round 12: `CommandId`-keyed, single-slot,
  session-gated — rules out reuse.
- `world_runtime.rs` `prepare`/`PreparedTerminal::unchanged`/`PreparedMutation` (~973-1159) — round
  15: every `unchanged(...)` disposition builds `PreparedMutation::None`; only `COMMITTED` builds
  `Publish` — the complete, exhaustive set. `gameplay_transport/mod.rs` `ComposedFreshAdmission::step`
  (~546-585) — one lock `.await`, then synchronous; no panic/abort/task-supervision code found for
  scope-owner work.
- `content/reference_playable.rs` `LocalObjectStateDefinition` (~810-813) — round 16: `key`+
  `collision` only. Round 17: `TransitionBinding` (~1334-1342) — no `revert_after_ms`, no attribute
  payload; `bind` cannot see authored `destination`/`revert_destination`. Encounter doc line 144 +
  sample `the_lord_of_the_lice/encounter.json` lines 70/71/73/74.
  `tools/content-schema/encounter-authoring/validate_encounter.py` (~188-201) + README ("the server
  does not read these files") — only boundary seeing the full action, offline tooling not wired.
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
- Declaring the new §7 decision, or the 1a/1b/1c model delta, `ACCEPTED`.
- D37 relocation and `SCOPE_HANDOFF`.
- Jira; bulk Issue #162 history.

## Implementation / findings

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3. Rounds 1-6: numbering; `Deadline`
option; FND-03 §10 timer; capacity atomicity; equal-deadline tie-break; one clock/scope; exactly one
bound inverse; pre-`prepare` discard. Round 7: never re-arms. Round 8: origin test. Round 9: exact
target identity. Round 10: fixed stale summary. Round 11: derived `InteractionChildOccurrenceRef`.
Round 12: no-op, nothing retained. Round 13: superseded (violated GAME-INTERACTION-01 §7); outcome
retained. Round 14 (`a0519257`): superseded — two P1 seam bugs; one lifecycle record,
`PENDING|IN_FLIGHT|TERMINAL`. Round 15 (`a2aab063`): convergence — exhaustive `TERMINAL` mapping;
removed two unproven "never..." claims, added "Open decisions." Round 16 (`24141ed7`): a timed
`map_item` transform can change a teleporter's `destination`, unrestorable by
`LocalObjectStateDefinition`/`PreparedMutation::Publish` (state+collision only); scoped
`revert_after_ms` to those two types, said `bind` rejects an attribute-changing transition.

Round 17 (Codex 4120251303/4120251319, head `a1cad472`, owner's stop rule applies now): round 16's
"`bind` rejects it" was the wrong boundary — `TransitionBinding` (7 fields, no attribute payload)
never sees authored `destination`/`revert_destination`; `bind` cannot tell `the_lord_of_the_lice`
apart from `DepthWarzoneBossDeath`. Fixed: moved the rejection to authoring/lowering, fail-closed
with a named error. Verified the only existing boundary reading the full action —
`validate_encounter.py`'s `map_item` block (~188-201) — but its README says "the server does not
read these files" (offline `CANDIDATE` tooling); no server-side lowering exists yet. Stated as an
obligation on whichever lowering step is built, naming that file as host. Rewrote the affected
bullet/narrative/open-decision-3/test obligation. P2: fixed a stale §5 summary line ("no receipt
store") to describe the round-14 lifecycle record; annotated §7's founding C2 text too. Grepped
whole doc for other stale summaries; none found. Merged `origin/main` (`3b41c0f4`, unrelated); `git
diff a1cad472 HEAD` confirms no cited file drifted.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-17; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 17; unchanged)

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

- required: YES
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
  PR #1045 pre-freeze round 17 (Codex 4120251303/4120251319 on frozen head a1cad472, owner's stop
  rule applies now): round 16's "bind rejects it" was the wrong boundary -- TransitionBinding has no
  attribute payload and never sees destination/revert_destination. Moved rejection to
  authoring/lowering, fail-closed with a named error. validate_encounter.py's map_item block is the
  only existing boundary seeing the full action, but offline CANDIDATE tooling ("the server does not
  read these files"); no server-side lowering exists yet -- stated as an obligation on whichever
  lowering step is built. Also fixed a stale SS5 summary line ("no receipt store") to describe the
  round-14 lifecycle record. Grepped whole doc for stale summaries; validators pass.
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
