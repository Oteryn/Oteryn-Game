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
  (~817-888) — PROVEN: no tick/time parameter; every mutation needs a bound `TransitionKey` plus
  `placement`/`incarnation`/`content_generation`; `prepare`'s first check (~978-987) rejects a
  mismatch with `DISPOSITION_BINDING_MISMATCH`, distinct from `DISPOSITION_STALE_STATE`.
- `foundation/runtime_actor_carrier.rs` `from_committed_assignment` (~702-750) — production-only:
  pins `scope_generation` once. No scope/global clock (bounded grep, evidence in §7) — PROVEN.
- `crates/foundation/src/time.rs` `Deadline`/`ManualClock`/`SystemClock::new()` (~50-166) — PROVEN:
  unused in `apps/game-server/src`, fresh incomparable origin per call. `OTERYN_ENCOUNTER_
  AUTHORING_FORMAT_V1.md` line 144 — authors `revert_after_ms`.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §7/§9/§10/§14/§15.4/§28 — binds `revert_after`.
  `foundation/mod.rs` `ScopeRuntimeFence::accept_input` (~1040-1050) — mints an ordinal per
  generation only, tracks no timer identity. `movement.rs` `MovementOwnerTurn` — bounded precedent.
- `GAME-INTERACTION-01_..._CANDIDATE.md` §4.1/§4.4/§5.1/§5.8, `interaction/identity.rs`
  `ChildOccurrenceRef` (~69-157) — nested-cascade identity; ordinal is fence evidence, not identity.
- `foundation/mod.rs` `CommandIngress`/`RetainedTerminalRecord`/`CommandId` (~53-55/202/498-742) —
  round 12, PROVEN: only terminal-retention mechanism in the repo; `CommandId`-keyed (not
  `InteractionChildOccurrenceRef`), single-slot (`MAX_RETAINED_TERMINAL_RECORDS = 1`), reachable only
  via session-gated `apply`/`resume_pending`; grep confirms no other retention path exists.
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

Initial delta: added §7; recorded CW3's 1a/1b/1c delta and C3. Rounds 1-6: numbering; `Deadline`
option recommended; rebound to FND-03 §10 authoritative timer with staged capacity atomicity,
equal-deadline tie-break, one clock per scope; merged origin/main (PR #1055/#1046); `revert_after_ms`
admissible only with exactly one bound inverse; pre-`prepare` discard restricted to
`scope_generation`/`content_generation`. Round 7 (Codex 4119354894): timer-origin execution never
re-arms (mutually timed pair fix). Round 8 (Codex 4119401513): restated as an origin test so
`DepthWarzoneBossDeath`'s revert isn't starved. Round 9 (Codex 4119452692): pending entry gained
exact target identity (`PlacementKey`/`incarnation`/`content_generation`); changed `incarnation`
joins the pre-`prepare` discard fences. Round 10 (Codex 4119516262, main→`74bb3fd3`, unrelated):
fixed a stale summary sentence still contradicting round 9.

Round 11 (Codex 4119565077, head `87b974b1`): pending entry never stored the revert's own derived
`InteractionChildOccurrenceRef` (GAME-INTERACTION-01 §5.1 nested-cascade; never the scheduling
ordinal per §4.4/§5.8). Fixed structurally: one canonical "Pending entry: complete field list" table
atop "Exact delta" that every other place in §7 now references; firing terminalizes every
disposition under that stored identity.

Round 12 (Codex 4119679783, head `8e12f6bf`): round 11's own text contradicted itself — the driver's
pending-removal-before-`accept_input` rule says a re-presentation is a no-op with nothing left, but
round 11's replay test said redelivery "finds that identity already terminal and replays it." Checked
`foundation/mod.rs` `CommandIngress`/`RetainedTerminalRecord` directly: the only terminal-retention
mechanism in the codebase is `CommandId`-keyed, single-slot (`MAX_RETAINED_TERMINAL_RECORDS = 1`),
reachable only through session-gated `apply`/`resume_pending` this timer already doesn't use — cannot
key by `InteractionChildOccurrenceRef`. Per coordinator decision (minimum-sufficient, no new ledger):
exactly-once *execution*, not outcome *replay*, already guaranteed by atomic pending-removal. Fixed:
rewrote the replay test obligation, firing/occupancy bullets, field list's closing paragraph and
round-11 narrative — redelivery is now a plain no-op, the single emitted outcome carries the stored
identity but is not retained; grepped the whole doc, fixed every timer-origin instance (client-command
replay via `CommandIngress` unaffected). Merged `origin/main` (`79b85ec6`, unrelated); `git diff
8e12f6bf HEAD` confirms every cited file is byte-identical to round 11's head, no citation drifted.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-12; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 47 workflows)." (round 12, main unmoved since round 10; unchanged)

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

- required: YES — same independent-review path as the proposal; not self-accepted.
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
  PR #1045 pre-freeze round 12 (Codex 4119679783 on frozen head 8e12f6bf): round 11's replay test
  contradicted the driver's pending-removal no-op rule. Checked foundation/mod.rs CommandIngress
  directly: CommandId-keyed, single-slot (MAX_RETAINED_TERMINAL_RECORDS=1), session-gated -- cannot
  key by InteractionChildOccurrenceRef. Per coordinator decision (no new ledger): timer-origin firing
  needs exactly-once execution (atomic pending-removal), not outcome replay. Rewrote the replay test
  obligation, firing/occupancy bullets, field-list closing paragraph; redelivery is now a plain
  no-op, outcome carries stored identity but is not retained. Grepped whole doc; validators pass.
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
