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
of this is `ACCEPTED`. D37 relocation and `SCOPE_HANDOFF` are untouched.

## Architecture and source of truth

Full file:line evidence lives in §7 of the owned doc (Evidence subsection); this is the index.

- `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` — PROVEN, read in full.
- `apps/game-server/src/world_runtime.rs` `LocalObjectRuntime::bind`/`::prepare` (~570-997),
  `terminalize_current`/`resume_pending` (~806-846) — PROVEN: no tick/time parameter; collision
  hard-wired to Open/Close; every prepared outcome (incl. `DISPOSITION_OCCUPIED`) terminalizes
  immediately and replays on retry rather than re-evaluating (§7 test obligations).
- `apps/game-server/src/foundation/runtime_actor_carrier.rs`
  `ChannelRuntimeV1::from_committed_assignment` (~702-750) — PROVEN, production-only: construction
  pins `scope_generation` once; no production method advances it in place. `advance_owner`
  (~2169-2176) is test-only (`impl MovementActorFixture`, `#[cfg(test)]`) — round-1 evidence citing
  it as production was corrected in round 2. Two production doc comments (~760-762, ~773-774)
  independently say `ChannelRuntimeV1` grants no scheduler.
- `apps/game-server/src/gameplay_transport/connection.rs` `Liveness::tick` (~296-320) and
  `apps/game-server/src/content/project/v2/creature.rs` `tick_profile` (~571-604) — PROVEN:
  transport keepalive and content-authoring data respectively, neither a scope simulation clock.
- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` lines 224/419 —
  PROVEN: no committed Foundation/global simulation tick exists.
- Bounded grep (`tokio::time::interval|sleep|select!\{|loop \{`) against every `ChannelRuntimeV1`
  call site — PROVEN: every call is reactive; `movement.rs` ~197-200 names the missing "future
  owner scheduler" itself. UNKNOWN beyond this bounded read tree.
- `crates/foundation/src/time.rs` `Deadline`/`MonotonicClock`/`ManualClock` (~50-166, exported at
  `crates/foundation/src/lib.rs` line 5) — PROVEN: an already-tested monotonic-deadline primitive,
  unused in `apps/game-server/src` today. `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` line 143 —
  PROVEN: authors `revert_after_ms` directly (milliseconds).
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` §9/§10/§15.4/§28 (lines 376-436, 560-564, 891) — PROVEN:
  the timer scheduling/firing/cancellation/capacity/error-mapping contract §7 now binds
  `revert_after` to. `foundation/mod.rs` `RuntimeExecutionOrdinal`/`ScopeRuntimeFence` (~959-1058)
  — PROVEN: already implements §10.2's ordinal-on-accept, currently per-`GameSession`
  (`admission.rs` ~365-381/728). `world_runtime.rs` `apply`/`resume_pending` (~776-832) — PROVEN:
  need a live `GameSessionAuthoritySnapshot`/`CommandIngress`, which a disconnected timer lacks.

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

Initial delta: added §7 (revert_after owner/options); recorded CW3's 1a/1b/1c delta and C3 in
§4/§5/§8 without designing them.

Round 1 (coordinator): fixed §-numbering (decision §7, Follow-up §8); CW3 (not CW4) attribution;
§8 item 7 renamed to the scope-runtime/Foundation carrier lane; §7 option 2 made honest (no proven
scope cadence, grep evidence).

Round 2 (Codex, bbb3b4cd): added `Deadline` option (`crates/foundation/src/time.rs`, unused in
`apps/game-server/src`; `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md:143` authors `revert_after_ms`),
made it the recommendation over the round-1 step counter; corrected `advance_owner` (test-only,
~2169-2176) to production-only evidence (`from_committed_assignment` ~702-750); decided
occupied-revert refuses permanently (`terminalize_current`/`resume_pending`, ~817-846).

Round 3 (Codex, 350dca59, FINAL): all four findings trace to the existing FND-03 timer contract
(§9/§10/§15.4/§28, read before writing). Rebound §7 to an FND-03 §10 authoritative timer:
- P1 commit path: `apply`/`resume_pending` (~776-832) need a live `GameSessionAuthoritySnapshot`/
  `CommandIngress` a disconnected player's timer lacks. Fixed: due timer is now a normalized §10.2
  input minting its own `RuntimeExecutionOrdinal` via the scope's ordinal issuer
  (`RuntimeExecutionOrdinal`/`ScopeRuntimeFence`, `foundation/mod.rs` ~959-1058, currently
  per-`GameSession` in `admission.rs`), reusing only `prepare`'s pure delta logic.
- P1 timer capacity: FND-03 §15.4 requires the original operation to fail before commit if timer
  capacity is unavailable. Fixed: capacity reserved in the same staged commit; added the atomicity
  obligation and `CAPACITY_EXCEEDED` mapping (§28; numeric bound left to `RESOURCE_LIMITS_REGISTRY.json`).
- P2 equal-deadline: bound a stable (deadline, then derived identity) tie-break plus a new ordinal
  per accepted due timer (§10.1/§10.2); added the determinism obligation.
- P2 clock origin: `SystemClock::new()` (~98-104) starts a fresh, incomparable origin each call.
  Fixed: one shared clock instance per scope for schedule and wake; added a cross-clock obligation.

All validators re-run after each round's commit; unchanged pass (see Validation below).

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS — "Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy
  documents and 9 project lanes." (re-run after each pre-freeze fix commit, rounds 1-3; unchanged)

### Component/integration

- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS — "Post-merge exact-candidate routing regressions PASS / Repository policy
  validation passed (23 files, 45 workflows)." (re-run after each pre-freeze fix commit, rounds 1-3;
  unchanged)

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
  PR #1045 pre-freeze round 3 (FINAL, Codex review) rebound §7's revert_after recommendation to the
  existing FND-03 §10 authoritative-timer contract: due-timer input (not a client command), staged
  timer-capacity atomicity (§15.4), equal-deadline RuntimeExecutionOrdinal tie-break, one shared
  clock instance per scope; both validators re-run and still pass.
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
