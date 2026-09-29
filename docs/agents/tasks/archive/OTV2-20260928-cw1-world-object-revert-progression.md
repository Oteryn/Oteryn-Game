# OTV2-20260928-cw1-world-object-revert-progression

```yaml
task_id: OTV2-20260928-cw1-world-object-revert-progression
title: World-object revert_after progression owner and D38 follow-up model deltas
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-world-object-revert-progression
pr: 1045
base_sha: ac8395b885e82d564d70968d6cae066fbc276e13
head_sha: 687d42d2859bad8254abe3c51cc2d81743340f04
final_head_sha: 687d42d2859bad8254abe3c51cc2d81743340f04
final_head_frozen_at: 2026-09-28T09:44:00Z
owner: Oteryn: content world architecture (CW1)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T09:44:00Z
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
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §7/§9/§10/§14/§15.4/§28/(line 266 exhaustion recovery) —
  binds `revert_after`. `accept_input` (~1040-1050) mints an ordinal per generation, tracks no timer
  identity, returns `Result<_, GenerationError>`, can fail `Exhausted` (Round 20). `movement.rs`
  `MovementOwnerTurn` — bounded precedent.
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

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3. Rounds 1-18: numbering; `Deadline`
option; FND-03 §10 timer; capacity atomicity; equal-deadline tie-break; one clock/scope; exactly one
bound inverse; pre-`prepare` discard; never re-arms; origin test; exact target identity; fixed stale
summary; derived `InteractionChildOccurrenceRef`; no-op/nothing retained (12); superseded — violated
GAME-INTERACTION-01 §7, outcome retained (13); superseded — two P1 seam bugs, one lifecycle record
`PENDING|IN_FLIGHT|TERMINAL` (14); convergence — exhaustive `TERMINAL` mapping, removed unproven
claims, added "Open decisions" (15); scoped `revert_after_ms` to state+collision, wrongly said `bind`
rejects an attribute-changing transition (16); fixed — moved rejection to authoring/lowering
(17, `a1cad472`); fixed `DepthWarzoneBossDeath`'s wrongly-claimed state-only status, whole sample
corpus checked, replaced test fixture with synthetic transform (18, head `95f7db70`).

Round 19 (Codex 4120487852/4120487841/4120487862, head `4e450a08`): classified `interaction` a
non-state attribute, same class as `destination`/`revert_destination`; no authored `map_item` action
admissible for `revert_after_ms` today; added test obligation; Open decision 4 (`TransitionBinding`
not the confirmed home) and 5 (tombstone compaction needs a bounded duplicate-delivery horizon).

Round 20 (Codex 4120634397/4120634418/4120634408, head `3827d885`): **P1** — gated lifecycle-record
creation on the original operation's `prepare` result being `Publish`; every `unchanged` disposition
now registers no record, releasing reserved capacity; expected state/revision sourced from
`Publish`'s own fields. Fixed staging bullet, field list, Must-decide-now; added a test obligation.
**P2:** fixed a plain contradiction — a test obligation still called `interaction` disqualification
undecided after round 19 settled it. **P2 (no redesign):** `accept_input` can return
`GenerationError::Exhausted`; qualified the ordering text, added Open decision 6. Merged `origin/main`
(`13dadcaa`, unrelated); no drift.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-20; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 20; unchanged)

### E2E

- scenario: NOT_APPLICABLE — documentation-only, no runtime behavior to exercise.
- result: NOT_APPLICABLE

### Exact-head CI

- final head: `687d42d2859bad8254abe3c51cc2d81743340f04` (frozen final head)
- trigger source: protected-`main` merge of PR #1045
- workflow/run/job: `game-gate` and repository protected-branch checks
- runner assignment: complete
- classification: PASS
- result: PASS — merged as `ebacc5b8` on protected `main`; protected-main readback passed (see
  Terminal integration)

## Self-review

- exact head: `687d42d2859bad8254abe3c51cc2d81743340f04` (frozen final head)
- method/reviewer: implementing agent (CW1), mandatory, not delegated
- material findings: 20 Codex review rounds against PR #1045 (see Implementation/findings above),
  each verified directly against code/docs and fixed minimally or handed to "Open decisions for the
  owning lane"; the owner stop rule applied at round 20 (no open P1)
- verdict: PASS

## Independent review

- required: YES.
- exact head reviewed: `687d42d2859bad8254abe3c51cc2d81743340f04` (round 20, last review before
  merge)
- method/auditor: Codex review across PR #1045 (rounds 11-20 covered in this record's
  Implementation/findings; earlier rounds covered in prior context) plus protected-main readback for
  the frozen final head
- material findings: P0/P1 none open at merge; every P1 raised across rounds 11-20 was fixed in the
  round it was raised; P2s handed to "Open decisions for the owning lane" (§7, items 1-6) rather than
  redesigned. See Terminal integration for Open decision 7, raised after freeze.
- verdict: PASS; protected `main` admitted the frozen final head

## PR and closeout

- changed-file review: complete (see Terminal integration)
- unresolved review threads: none blocking at merge (Open decision 7, Codex 4120777222, raised on the
  PR thread after freeze — see Terminal integration)
- related/superseded PRs: none known
- protected auto-merge: not requested by this task
- merge commit/result: `ebacc5b8` on protected `main`
- ownership release: complete; owned paths released at archive

## Context checkpoint

```yaml
last_progress: terminal integration recorded; PR #1045 merged on protected main as ebacc5b8; record archived
status: completed
branch: claude/cw1-world-object-revert-progression
head_sha: 687d42d2859bad8254abe3c51cc2d81743340f04
pr: 1045
final_head_sha: 687d42d2859bad8254abe3c51cc2d81743340f04
final_head_frozen_at: 2026-09-28T09:44:00Z
ci_trigger_source: protected_main_merge
ci_check_generation: final
ci_checks_for_current_head: 1
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: complete
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 20
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none; task closed. Open follow-up: Open decision 7 (Codex 4120777222, PR-thread only) must be carried into §7 by the owning lane (see Terminal integration)
```

## Terminal integration

This section supersedes the historical `ready`/pending metadata and checkpoint above with the
frozen terminal outcome; the complete implementation record above remains verbatim as historical
evidence. This closeout performs no architecture, code or schema mutation of its own; it only moves
this record from `docs/agents/tasks/active/` to `docs/agents/tasks/archive/` and binds terminal
lifecycle fields. Coordination: issue #162 (control plane).

The owner stop rule was applied at round 20: PR #1045 merged with no open P1. Candidate head
`687d42d2859bad8254abe3c51cc2d81743340f04` (round 20: gated lifecycle-record creation on `prepare`'s
`DISPOSITION_COMMITTED`/`Publish` outcome, fixed a plain contradiction in the `interaction`
disqualification test obligation, qualified the ordinal-issuance-exhaustion ordering text and added
Open decision 6 — see Implementation/findings above) was frozen at 2026-09-28T09:44Z (issue #162
FREEZE comment for round 20). PR #1045 merged on protected `main` as commit `ebacc5b8`. Protected-main
readback passed: §7 is present with `DecisionStatus: CANDIDATE`, and the "Open decisions for the
owning lane" section (items 1-6) is present, matching the frozen head `687d42d2`.

**Open decision 7, carried forward — not yet reflected in §7 (Codex finding 4120777222).** After the
round-20 freeze and merge, Codex raised one further finding on the PR thread only, not incorporated
into a further round under the owner stop rule: lifecycle-record capacity should be reserved only
after the original operation's own `prepare` result is known to be `DISPOSITION_COMMITTED`/`Publish`,
rather than reserved before `prepare` runs and released on an `unchanged` outcome (round 20's own
shape — see the staging bullet near Open decision 6 in §7). This closeout does not edit §7 itself; it
records Open decision 7 here as a pointer so the owning lane carries it into §7 (alongside items 1-6)
in a follow-up edit before implementation.

Task status: `completed`. Aggregate issue #162 remains open for further work, including the owning
lane's implementation of §7's decision and the carry-forward of Open decision 7 into the document.
