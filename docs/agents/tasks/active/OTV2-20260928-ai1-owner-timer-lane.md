# OTV2-20260928-ai1-owner-timer-lane

```yaml
task_id: OTV2-20260928-ai1-owner-timer-lane
title: GAME-AI-01 child AI-1, Channel owner timer lane and injectable clock (FND-03 §10)
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ai1-owner-timer-lane
issue: 162
pr: null
allocation: "#162 comment 5876068445 (GAME-AI-01 child AI-1)"
base_sha: 7d1134f090ac249f964fede017efabba91e22b90
head_sha: pending_push
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl ai (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/owner_timer.rs   # new module, exclusive
  - apps/game-server/src/foundation/mod.rs            # short lease: one `mod` declaration
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json      # AI-1's row only (AI01-PENDING-TIMERS-PER-ACTOR)
  - docs/agents/tasks/active/OTV2-20260928-ai1-owner-timer-lane.md
public_contracts:
  - FND-03
depends_on: []
blocks:
  - AI-2 (spawn realization, respawn, carrier creature envelope, Movement adoption)
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A generic, owner-scoped timer lane (`OwnerTimerLane<Family, Occurrence>`) implements the
FND-03 §10 timer contract for the first time in `ChannelRuntime`-adjacent code: one lane
instance per `ScopeOwnershipGeneration`, with an injectable `OwnerClock` (production monotonic
time and a deterministic `VirtualOwnerClock` for tests). It is generic over the caller's timer
family and occurrence identity so the same mechanism serves AI think timers now and respawn,
spell cooldowns and regeneration later (decision §4.2). The lane never mutates state itself:
`drain_due` only returns due timers in FND-03 §10.1 deterministic order (deadline, then
scheduling ordinal, then an internal within-resolution sequence) for the owner to apply as
normalized inputs.

## Architecture and source of truth

- PROVEN: `docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md`
  §4.2 (Channel owner timer lane), §4.9 (AI01-PENDING-TIMERS-PER-ACTOR / AI-RL-06), §5 (AI-1 scope
  and serialization).
- PROVEN: `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md` §8.4 (no universal fixed
  tick), §10 (timer execution contract: scheduling key, firing, cancellation/staleness,
  zero-delay recursion, catch-up taxonomy), §11 (deterministic test clocks).
- DERIVED: `ExactActorRef` (`foundation::runtime_actor_carrier`) is already an opaque,
  generation-binding equality type, so target-generation staleness (§10.3) is expressed as a
  caller-supplied `target_is_current: FnMut(ExactActorRef) -> bool` closure rather than the lane
  decoding actor internals it has no access to.

## Acceptance criteria

- [x] Stale timers (ownership or target generation changed) mutate nothing.
- [x] Equal deadlines from different resolutions order by scheduling ordinal, then sequence.
- [x] An occurrence identity is never scheduled twice.
- [x] The AI-1 pending-timer bound (AI01-PENDING-TIMERS-PER-ACTOR = 1) is enforced: max
      accepted, max+1 rejected.
- [x] A virtual clock drives the tests; no wall-clock dependency in the lane.
- [x] No busy loop: a fired timer is removed and never refires; only a fresh `schedule` call
      produces future work.
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

Spawn realization, respawn scheduling policy, the carrier's creature envelope, Movement
adoption, `ai/**` compilation, Ability wiring (all AI-2/AI-3/AI-4). No wiring of
`OwnerTimerLane` into `ChannelRuntimeV1`'s run loop: that owner lives in
`runtime_actor_carrier.rs`, a forbidden path for this task (see Spec gap below).
`AI01-SPAWN-*` and other AI01 §4.9 rows that belong to AI-2. `lib.rs`, `src/combat*`,
`src/ai/**`, `movement.rs`, `ability/**`, `durability/**`, migrations.

## Implementation / findings

- `apps/game-server/src/foundation/owner_timer.rs` (new): `SemanticTimeMicros`, `OwnerClock`
  trait, `VirtualOwnerClock`, `OwnerTimerError`, `OwnerTimerLane<Family, Occurrence>` with
  `schedule`, `cancel`, `pending_for_key`, `pending_len`, `drain_due`; `AI01_PENDING_TIMERS_PER_ACTOR`
  constant (= 1) mirroring the registry row.
- `apps/game-server/src/foundation/mod.rs`: one `#[allow(dead_code)] mod owner_timer;`
  declaration (module not yet consumed outside its own tests; no public re-export added, kept
  to the "short lease" the allocation asked for).
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`: added `AI01-PENDING-TIMERS-PER-ACTOR`
  (hard_maximum 1, `owner_contract` pointing at this decision's §4.9), the only AI-1-owned row;
  the four `AI01-SPAWN-*` rows are left for AI-2, which owns spawn realization.

### Spec gap / forbidden-path need

§4.2 is delivered as a standalone, fully tested lane. Wiring `OwnerTimerLane` into
`ChannelRuntimeV1`'s actual owner cycle (so a live Channel schedules/drains real think and
respawn timers) requires adding fields/calls inside `runtime_actor_carrier.rs`, which is
listed FORBIDDEN for this task (owned by AI-2/Combat D's exclusive lease). Per the
minimum-sufficient instruction, I stopped short of that integration rather than touch the
forbidden path. AI-2 (which already owns `runtime_actor_carrier.rs`) is the natural place to
add a `ChannelRuntimeV1`-held `OwnerTimerLane<AiTimerFamily, AiOccurrence>` field and call
`schedule`/`drain_due` from the owner cycle when it realizes spawns and thinks.

## Validation

- `cargo fmt --all --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean.
- `cargo test -p oteryn-game-server --lib foundation`: 358 passed, 0 failed (11 of those are
  `foundation::owner_timer::tests::*`, covering ordering, determinism, bounds, duplicate/
  cancellation, owner-generation staleness and target-generation staleness).
- `python3 tools/agents/validate_governance.py`: passed (22 policy documents, 9 project lanes).
- `python3 tools/repository/validate_repository_policy.py`: passed (23 files, 50 workflows).
- `git diff --check`: clean.

## Self-review

- exact head: local candidate before freeze
- method/reviewer: implementing agent
- material findings: none open
- verdict: READY_FOR_FREEZE (pending push and control-plane routing)

## Independent review

- required: YES (control plane routes per allocation `#162 comment 5876068445`)
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- The coordinator opens the PR from `claude/ai1-owner-timer-lane`; this worker does not open
  PRs, comment on GitHub, or request review.

## Context checkpoint

```yaml
last_progress: owner_timer.rs implemented and unit-tested (11/11), foundation lib suite green
  (358/358), registry row AI01-PENDING-TIMERS-PER-ACTOR added, governance/repository-policy
  validators pass; pushing claude/ai1-owner-timer-lane for the coordinator
status: waiting
branch: claude/ai1-owner-timer-lane
head_sha: pending_push
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: "control plane freezes the pushed head, opens the PR, and allocates AI-2 (which
  will wire OwnerTimerLane into ChannelRuntimeV1's owner cycle from runtime_actor_carrier.rs)"
```
