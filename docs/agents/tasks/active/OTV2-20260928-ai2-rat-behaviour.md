# OTV2-20260928-ai2-rat-behaviour

```yaml
task_id: OTV2-20260928-ai2-rat-behaviour
title: GAME-AI-01 "AI-2 rat behaviour" -- think/decide/act over the owner timer lane (D53, D115)
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ai2-rat-behaviour
issue: 162
pr: null
allocation: "#162 comment 5879863781 (AI-2 rat behaviour, D115 values); D115/D116 in 5879404970"
base_sha: 86116adfda4b4e7c1dc505c2e2af486db2370378
head_sha: pending_push
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl ai (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/creature_think.rs   # new, exclusive
  - apps/game-server/src/foundation/mod.rs               # one `mod` line
  - apps/game-server/src/foundation/runtime_actor_carrier.rs   # lift Movement guard only
  - apps/game-server/src/movement.rs                     # test replaced/added; doc comment
  - docs/agents/tasks/active/OTV2-20260928-ai2-rat-behaviour.md
public_contracts:
  - GAME-AI-01
depends_on:
  - OTV2-20260928-ai1-owner-timer-lane
blocks:
  - "AI-3 (ai/** into lib.rs; perception snapshots; D116 spawn)"
  - "AI-4 (typed AI issuer via Ability; HP floor D54; real bite committer)"
cross_repository_coordination_id: null
external_repositories: []
```

## Plan (written before coding)

1. Read AGENTS.md, `apps/game-server/AGENTS.md`, the GAME-AI-01 contract (D53-D57), the
   AI-1 archive record, `owner_timer.rs`, `runtime_actor_carrier.rs`, `combat.rs`.
2. Resolve D115/D116 and the live "AI-2" scope from `#162`'s comments directly (not yet
   mirrored into `docs/`): the task prompt's "AI-2" (perceive/chase/bite/wander) reads
   differently from the written contract's §5 table (AI-2 = spawn/respawn/carrier
   envelope/Movement adoption; AI-3 = behaviour; AI-4 = bite via Ability there). Stop and
   report if this cannot be resolved from live evidence.
3. Design the smallest slice fitting the existing runtime: AI-1's `OwnerTimerLane` for the
   think tick; the existing Movement owner turn for chase/wander once the carrier's
   creature-rejection guard is lifted; bite/player-damage as an owner-injected commit
   boundary, since no player-vitals capability exists anywhere in this codebase.
4. Implement, test, validate, write this record, push. No PR, no GitHub comment.

## Outcome

`foundation::creature_think` (new) implements GAME-AI-01 §4.4's bounded think resolution
(attack -> chase -> wander -> idle) as a pure `decide` function, deterministic draws bound
to the think occurrence (`oteryn_simulation_determinism`), and `schedule_next_think` wiring
it to AI-1's `OwnerTimerLane`. The carrier's guard rejecting `CreatureOccupied` slots for
Movement is lifted (`runtime_actor_carrier.rs`), so a creature now steps through the
existing, unmodified `movement.rs::step_cardinal`/`MovementOwnerTurn` like a player (§4.5).
Bite/player damage stays a caller-injected `BiteCommitter` boundary: no player vitals exist
yet, so the only implementation is a bounded `#[cfg(test)]` vitals fixture, matching the
contract's own stated limitation (§4.7). No spawn/respawn, envelope or `ai/**` compilation:
routed to AI-3/AI-4.

## Architecture and source of truth

- PROVEN: the GAME-AI-01 decision (`docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md`)
  §4.2 (timer lane), §4.4 (behaviour), §4.5 (Movement adoption), §4.6/§4.7 (Ability
  boundary/player HP floor, gated on spell P3b-2/AI-4), §4.9 (resource rows, already
  registered by the bootstrap slice or AI-1).
- PROVEN (live, not yet in `docs/`): `#162` issuecomment-5879404970 posts D115 (rat AI:
  think 1000 ms; perception 7 tiles; 25% wander chance, radius 2; respawn 60 s; occupied
  tile retried 3x/5s; routed "AI-2 / AI-3") and D116 (2-rat spawn; routed "AI-3 /
  content"). issuecomment-5879863781 is the control plane's current allocation: "impl ai:
  AI-2 rat behaviour (D115 values, AI-1 binding items a/b)" on this branch, owning
  `runtime_actor_carrier.rs`, `owner_timer.rs`, "the new AI module", excluding
  `durability/item_transfer*`/migrations -- this task's direct source, followed here; it
  retargets the written §5 table's task *labels* for routing only, no accepted rule (§3,
  §6 rejected options) is relaxed.
- PROVEN: `owner_timer.rs` (AI-1, PR #1150): `OwnerTimerLane`, `TimerFamily`,
  `AI01_PENDING_TIMERS_PER_ACTOR`, and the two binding items left open.
- PROVEN: no player-vitals (HP) type/field exists in `apps/game-server/src` (exhaustive
  grep); `commit_damage` targets creature health only; §4.7 limits bite to "a test vitals
  fixture, never as a shipped behaviour" until spell P3b-2/AI-4.
- PROVEN: `compare_commit_position`/`initialize_position`/`read_position` already branch
  on `CreatureOccupied` identically to `Occupied`, incl. the existing dead-creature guard;
  only the two Movement-facing wrappers (`read_movement_position`,
  `commit_movement_position`) added a creature rejection. `movement.rs` was already fully
  generic over `ExactActorRef`; lifting the two wrapper guards is the whole change.
- DERIVED: several `tests/*.rs` binaries path-include `foundation/mod.rs` as an isolated
  crate root without `crate::movement`/`crate::content` (same constraint
  `owner_timer.rs`/`loot_plan.rs` document). `creature_think.rs` imports only `super::*`
  plus the external `oteryn_simulation_determinism`/`sha2` crates and defines its own
  `CardinalDirection`/`RatLocalPosition`; the caller converts trivially.

## Acceptance criteria

- [x] `decide` is pure: same inputs give the same decision, a shuffled candidate order
      gives the same decision, a retry of the same occurrence never redraws.
- [x] Attack requires cardinal adjacency, legality (caller-supplied) and an elapsed
      cooldown; a failed chance draw ends idle, never chase/wander (§4.4.1).
- [x] A re-entry-protected (illegal) target is chased, never bitten; nearest candidate
      wins, ties break by stable `placement_identity`.
- [x] Wander never leaves the radius from `home`; a boxed-in creature stays idle.
- [x] `AI01-PERCEPTION-CANDIDATES` (64): 64 accepted, 65 rejected before other work.
      `AI01-PENDING-TIMERS-PER-ACTOR` (1): a second think cannot schedule while pending;
      the rejection never advances the bounded sequence.
- [x] An occurrence identity is never scheduled twice across a lane's lifetime (binding
      (a)); `schedule_next_think` only forwards a caller-fence-issued `RuntimeWorkStamp`
      (binding (b)).
- [x] A bite commit clamps to the D54 floor (>= 1); a retry of the same occurrence returns
      the memoized result, never applying damage twice (test fixture).
- [x] A creature actor steps through the same `MovementDecision`/`step_cardinal` a player
      uses, with the same stale-snapshot revalidation.
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

Spawn realization, respawn scheduling, the multi-creature/D57 envelope, D116 content, the
Movement proof/room revision -- routed to AI-3/content. `ai/**` into `lib.rs`, the typed AI
issuer via Ability, the real (non-fixture) bite committer -- AI-4, gated on spell P3b-2
player vitals. `durability/item_transfer*`, migrations, `gameplay_transport/**` (B3-2's
lease). No production caller wires `creature_think` into `ChannelRuntimeV1`'s live owner
cycle yet: like `owner_timer.rs`/`loot_plan.rs`, it is complete, tested and uncalled,
ready for that wiring.

## Binding items carried over from AI-1

- **(a) bounded replay evidence.** `RatThinkState::next_sequence` is a monotonically
  increasing (`saturating_add`, never reset within one creature generation) per-creature
  counter; `schedule_next_think` mints a `ThinkOccurrence` only from its current value,
  advanced only after a successful `schedule`. `ExactActorRef` embeds the actor-local
  generation, never reused (D52), so `(creature, sequence)` can never repeat in the
  lane's lifetime: the evidence is this one `u64` high-water mark, O(1) per creature.
  Covered by `schedule_next_think_advances_the_bounded_sequence_and_never_repeats`.
- **(b) unforgeable stamp provenance.** Already enforced inside `OwnerTimerLane::schedule`
  (`accepts_stamp`, AI-1 repair generation 2); `schedule_next_think` only forwards a
  caller-supplied `RuntimeWorkStamp`, never fabricating one.

## Implementation / findings

- `foundation/creature_think.rs` (new, ~1040 lines incl. tests/docs): `RatBehaviorDefinition`
  (content-shaped, never hardcoded); `PerceivedPlayer`, `RatDecision`, `RatThinkError`;
  `decide` (pure); `step_toward` (fixed greedy rule, x-axis-first tie-break, no production
  pathfinding decided here, §7); `roll_bite_damage`; `BiteCommitter` trait (owner-applied
  commit boundary, §3: AI proposes, the owner commits); `RatTimerFamily`,
  `ThinkOccurrence`, `RatThinkState`, `schedule_next_think`, `occurrence_decision_id`.
  Chance/index draws reuse `domain::death::per_mille_roll`'s multiply-shift technique.
- `foundation/mod.rs`: one new line, `pub(crate) mod creature_think;` (`pub(crate)`, not
  private like `owner_timer`, since `movement.rs`'s test module is this slice's caller).
- `runtime_actor_carrier.rs`: removed the `CreatureOccupied` rejection from
  `read_movement_position`/`commit_movement_position` (2 sites); every other position
  primitive already handled it generically, including the existing health-0 guard.
- `movement.rs`: doc comment updated; the obsolete
  `real_movement_creature_and_revision_ceiling_reject_no_mutation` test split into
  `creature_actor_steps_through_the_same_movement_owner_turn_as_a_player` (new) and a
  renamed `real_movement_revision_ceiling_rejects_no_mutation`; one new
  `rat_decide_chase_step_applies_through_the_real_movement_owner_turn` end-to-end test.
- No `RESOURCE_LIMITS_REGISTRY.json` change: stays within already-registered ceilings; no
  new resource dimension.

## Validation

- `cargo fmt -p oteryn-game-server -- --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean, incl. the
  standalone Foundation test module and 9 standalone `tests/*.rs` composition/Postgres
  crates path-including `foundation/mod.rs` (checked individually).
- `cargo test -p oteryn-game-server --lib`: 795 passed, 0 failed, 2 ignored (whole-crate).
- `... -- foundation:: movement:: combat::`: 396 passed, 0 failed (14 new in
  `creature_think::tests`, 2 new/changed in `movement::tests`).
- `python3 tools/agents/validate_governance.py`: passed (22 policy docs, 9 lanes).
- `git diff --check`: clean.

## Self-review

- exact head: local candidate before freeze
- method/reviewer: implementing agent
- material findings: none open. D115/D116 and "AI-2 rat behaviour" initially looked
  contradicted by the written §5 table; resolved by reading `#162` live, not invented:
  D115/D116 are real and the control plane's latest allocation retargets "AI-2" to rat
  behaviour with these owned paths. No accepted rule was relaxed: bite routes through an
  owner-applied commit boundary, Movement adoption reuses the existing owner-turn path
  unchanged, no player-vitals capability was invented.
- verdict: READY_FOR_FREEZE (pending push and control-plane routing)

## Independent review

Required: YES (control plane routes per allocation `#162` issuecomment-5879863781).
Exact head / method / findings / verdict: pending.

## PR and closeout

Per allocation, this worker opens no PR, no GitHub comment, no review request; it pushes
`claude/ai2-rat-behaviour` and stops. Control plane owns freeze, review and integration.

## Context checkpoint

```yaml
last_progress: implementation, tests and validation complete on claude/ai2-rat-behaviour; pushing now
status: waiting
pr: null
head_sha: pending_push
final_head_sha: null
blocker: null
next_action: "control plane: freeze the pushed SHA, route independent review"
```
