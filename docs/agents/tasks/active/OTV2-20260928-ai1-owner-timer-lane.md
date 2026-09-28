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
pr: 1150
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

### Repair generation 1 (Codex findings on `0d296c0`)

Three Codex findings against `0d296c0` were repaired in owner_timer.rs (no forbidden path
touched):

- **P1** (`drain_due` only compared the lane's stored generation with a caller-supplied
  generation value, both fixed at construction/call time, so a real handoff could never be
  detected): `schedule` and `drain_due` now take `owner_fence: &foundation::ScopeRuntimeFence`
  instead of a raw `ScopeOwnershipGeneration`. `ScopeRuntimeFence` is the existing FND-03
  §10/§8 single-owner, mutated-in-place owner-cycle authority already used for
  `RuntimeExecutionOrdinal` issuance; `mod.rs` gained one minimal public accessor,
  `ScopeRuntimeFence::is_current(generation)`. A superseded owner still holding this lane and a
  reference to the same fence now observes a real handoff (`apply_external_grant`) and is
  refused, rather than draining/scheduling against a generation that merely still equals the
  lane's own stored field.
- **P2** (`max_pending_for_key` was a per-call argument, so a caller could pass a value above
  the registered `AI01-PENDING-TIMERS-PER-ACTOR` hard maximum of 1): the cap is now a
  `family_caps: Vec<(Family, usize)>` policy fixed once at `OwnerTimerLane::for_generation`
  construction; `schedule` takes no cap argument at all, and an unregistered family fails
  closed (cap 0). Registered rows (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`) are
  unchanged; the fix makes the constructed lane structurally unable to exceed them from any
  call site.
- **P3** (a not-yet-due entry whose target had gone stale was never purged until its own
  deadline): `drain_due`'s single retain pass now checks `target_is_current` for every pending
  entry, due or not, and purges a stale-target entry immediately without returning it, so
  death/respawn churn cannot accumulate stale entries ahead of their deadline.

New/renamed tests (owner_timer.rs, all passing): `family_cap_is_fixed_at_construction_and_cannot_be_raised_per_call`,
`family_without_a_constructed_cap_fails_closed`,
`drain_due_requires_the_fence_to_still_be_current_after_a_real_handoff`,
`drain_due_purges_not_yet_due_stale_target_without_returning_it`; the prior
`schedule_rejects_stale_owner_generation`/`drain_due_fires_nothing_for_stale_lane_generation`
were renamed to `..._a_fence_that_never_matched_this_lane` and kept as the simple
never-matched-fence case alongside the new real-handoff test.

### Repair generation 2 (Codex findings on `f335b73`, review 5343444021)

Four Codex findings repaired in `owner_timer.rs` + a minimal `mod.rs` addition (no forbidden
path touched):

- **P1** (`OwnerTimerLane`/`ScopeRuntimeFence` carried only a generation, so Channel A's fence at
  generation 1 could authorize Channel B's lane): `ScopeRuntimeFence` gained an additive
  `scope: Option<RuntimeScopeRefV1>` field (`None` for every existing caller — `from_external_grant`
  is unchanged, so `admission.rs`/`admission_recovery_inner.rs` are unaffected) and a
  module-visible `with_scope` builder. `OwnerTimerLane` now takes `scope: RuntimeScopeRefV1` at
  `for_generation` and stores it. `is_current` (used only by `owner_timer.rs`, confirmed by
  repo-wide grep) was replaced by `ScopeRuntimeFence::is_current_for_scope(scope, generation)`,
  which `schedule`/`drain_due` both call instead: a fence bound to a different `RuntimeScopeRefV1`
  is refused even at a matching generation number. New tests:
  `schedule_rejects_a_fence_bound_to_a_different_scope_at_the_same_generation`,
  `drain_due_fires_nothing_for_a_fence_bound_to_a_different_scope`.
- **P2** (the raw `RuntimeExecutionOrdinal` passed to `schedule` was caller-supplied and not
  generation-bound): `schedule` now takes `scheduling_stamp: RuntimeWorkStamp` (already defined
  in `mod.rs`, issued via the caller's own `owner_fence.accept_input` + `.stamp()`, "the same
  fence issuance already uses" per this module's doc) and validates it with
  `owner_fence.accepts_stamp(scheduling_stamp)` alongside the scope/generation check; a raw,
  caller-fabricated ordinal can no longer stand in for a fence-issued one. All existing tests
  were converted from synthetic `ordinal(N)` values to real stamps issued from a live fence
  (`issue_stamp` test helper); the deliberately out-of-order tie-break test
  (`equal_deadline_orders_by_scheduling_ordinal_then_sequence`) now pre-issues two stamps from
  the same fence and applies them out of issuance order, which the fence's contract permits
  (`accepts_stamp` checks liveness/generation, not usage order).
- **P3** (`for_generation` accepted any per-family cap, so `[(AiThink, 2)]` bypassed the
  registered `AI01_PENDING_TIMERS_PER_ACTOR` = 1 maximum): `for_generation` now takes
  `(Family, FamilyPolicy, usize)` triples — the trailing `usize` is the registered hard maximum
  for that family — and returns `Result<Self, OwnerTimerError>`, rejecting with the new
  `FamilyCapExceedsRegisteredMaximum` variant when `policy.max_pending` exceeds it. New test:
  `construction_rejects_a_family_cap_above_its_registered_maximum`.
- **P4** (no catch-up policy was encoded): added `CatchUpPolicy` (`SkipToLatest` for AI think,
  `DeadlineState` for respawn, matching §4.9's table exactly) as part of the new `FamilyPolicy`
  struct, fixed at construction. `drain_due` now enforces it: for a `SkipToLatest` family it
  collapses every (family, target) key with more than one due entry down to the single latest
  one (earlier missed occurrences are dropped without being returned or mutated) and reports
  `clock.now()` as the fired timer's `due` instead of the stale original deadline, so a caller
  rescheduling relative to `due` cannot be driven to replay every missed interval.
  `DeadlineState` (respawn) fires every due entry unchanged, matching "at most the spawn's
  population pending". New tests:
  `skip_to_latest_reports_now_instead_of_the_stale_deadline_after_a_long_delay`,
  `skip_to_latest_collapses_multiple_due_occurrences_for_the_same_key_into_one_fire`.

`mod.rs` change: `ScopeRuntimeFence` gained the `scope` field, `with_scope`, and
`is_current_for_scope` (replacing `is_current`) as described above — additive/renamed, and the
only two pre-existing callers of the renamed method were both in `owner_timer.rs`.

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
- `cargo test -p oteryn-game-server --lib foundation`: 367 passed, 0 failed (20 of those are
  `foundation::owner_timer::tests::*`, covering ordering, determinism, the fixed per-family
  cap and its registered-maximum ceiling, scope- and generation-bound fence authority (including
  a real in-place handoff and a same-generation cross-scope refusal), fence-issued
  `RuntimeWorkStamp` validation, eager not-yet-due stale-target purging, and the
  `SkipToLatest`/`DeadlineState` catch-up policies).
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
last_progress: repair generation 2 on top of f335b73 (review 5343444021) - fixed 4 Codex
  findings in owner_timer.rs + a minimal additive mod.rs change (P1 scope-bound
  ScopeRuntimeFence.is_current_for_scope replacing generation-only is_current, P2
  fence-issued RuntimeWorkStamp replacing a raw caller-supplied ordinal, P3 per-family caps
  validated against a caller-supplied registered maximum at construction, P4 typed
  SkipToLatest/DeadlineState catch-up policy enforced in drain_due); owner_timer.rs unit tests
  20/20, foundation lib suite green (367/367), fmt/clippy/governance/repository-policy
  validators pass; pushing new commit to claude/ai1-owner-timer-lane
status: waiting
branch: claude/ai1-owner-timer-lane
head_sha: pending_push
pr: 1150
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
