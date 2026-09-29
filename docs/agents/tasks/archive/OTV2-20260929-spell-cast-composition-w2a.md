# OTV2-20260929-spell-cast-composition-w2a

```yaml
task_id: OTV2-20260929-spell-cast-composition-w2a
title: Spell cast composition W2a - cast command, anchor payment, result and ACTOR_VITALS (§9 step 2, first slice)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1263
allocation_comment: "#162 5895497246"
base_branch: main
branch: claude/spell-cast-composition-w2a
base_sha: c91055f66dd2c5724f17e226c7b7da2430a1a914  # main 48de3868 merged in before freeze
head_sha: null  # the frozen head is the one in the FREEZE_SHA entry; a commit cannot hold its own SHA
owner: "Oteryn: spell cast lane" (Claude Code, oteryn-hard-worker)
created_at: 2026-09-29T19:00:00Z
updated_at: 2026-09-29T21:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/**
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs  # allocated; left unchanged (see Decisions)
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/content/activation.rs  # allocated; left unchanged
  - apps/game-server/src/ability/**  # allocated; left unchanged
  - docs/agents/tasks/archive/OTV2-20260929-spell-cast-composition-w2a.md
  - docs/agents/tasks/archive/OTV2-20260929-spell-cast-wire-w1.md  # closeout: main #1259 archived it; this adds merge_pr and merge_sha
public_contracts:
  - docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto  # consumed, unchanged
depends_on:
  - OTV2-20260929-spell-cast-wire-w1  # PR 1254, merge c91055f6
blocks:
  - "W2b: Character-sourced cast facts (vocation, magic level, level) and vitals maxima"
cross_repository_coordination_id: null
external_repositories: []
jira: null  # no spell Story is mapped; Jira sync pending
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

First reviewable slice of §9 step 2 of `OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md`, in the
first-control-step M2 pattern: a connected player's cast runs on the server from the command to the published
result and vitals.

- **Command.** `serve_admitted` dispatches command type 3 `WORLD_ACTOR_SPELL_CAST_INTENT` under the same FND-02
  CommandId discipline as a step: every command gets one sequenced `CommandResult` with a
  `WorldActorSpellCastResultV1`; a payload that does not decode is `REJECTED` before the Channel owner. A replayed
  CommandId is never executed again (`COMMAND_OUTCOME_EXPIRED`), so a retry never casts or pays twice (SPELL-D3).
- **Channel owner work item.** `ComposedFreshAdmission::cast_spell` runs one cast under the runtime lock
  (`SPELL-RL-01` = 1, bound to the registry row by a test). `actor_spell::cast_in_channel` reads the actor's state,
  resolves the cast through the existing core (`resolve_cast` then `effect_plan`), and compare-commits the successor
  state: the heal, mana, soul and both cooldowns are one value written once (§5, SPELL-D3). A failed cast writes
  nothing. The occurrence and every magnitude draw are bound to (placement, GameSession, CommandId) through
  `deterministic_decision_u64`; the ready times come from the process-local owner clock.
- **Vitals and cooldowns (§4, SPELL-D2).** `ChannelSpellStates` sits beside the Channel runtime, like the door
  runtime, and is locked only after it. An entry is keyed by the exact actor and its GameSession; each access first
  proves through `ChannelRuntimeV1::player_control_facts` that the actor is still that session's committed player.
  It is created at the maxima in the same owner step that positions the actor, and a present actor keeps it exactly
  (retry and same-GameSession reconnect: no refill, no cooldown reset). An ended actor's entry is unreachable and
  dropped at the next initialization.
- **Publication.** `ACTOR_VITALS` (domain 3) joins the join/resync snapshot at the owner's revision, and a committed
  cast is followed by one `StateDelta` (revision n to n+1). Harmony and Serene are 0 and false: the SPELL-D8
  owner (H-1, H-2) is not delivered.
- **Dispositions.** The §3 table (`spell::cast::disposition`), with `TargetNotAllowed` as a legality failure
  (`TARGET_ILLEGAL`) and any rejection the table does not name as `REJECTED`. An unknown index and a `POSITION`
  intent are `REJECTED` (no admitted spell is `cast_at_position`); `ATTACK_TARGET` resolves to no target because no
  attack-target owner exists, and `aim_at_target` is then ignored (SPELL-D7).
- **Spell book (SPELL-D1).** `spell::cast::v1_spell_book` loads the §6 V1 self set from the candidate starter
  bundles (`cure_poison`, `intense_healing`, `light_healing`), in `ProductionKey` byte order, and fails the whole
  book closed on any load error. `serve` loads it with the Content activation and refuses readiness without it.
- **Harmony spells** stay fail-closed in `authoring.rs`; the admission gate is unchanged.

## Decisions and assumptions

- **Casting is gated in production (SPELL-D4, §4).** Vocation and magic level have no GAME-CHAR owner, so
  `character_cast_facts` returns none for every Character: a production cast is `REJECTED` and no `ACTOR_VITALS`
  is published. The whole path is proven with supplied facts in tests; the WP5 seam and node-boot qualification
  prove the gated `REJECTED` answer over TLS. Opening the gate is W2b and needs the owner decision below.
- **State beside the runtime, not in the carrier.** `runtime_actor_carrier.rs` is path-included by the
  PostgreSQL integration test crates, which have no `crate::spell`, so it cannot hold spell types; the state follows
  the door-runtime pattern instead. The carrier, `ability/**` and `content/activation.rs` are unchanged.
- **Condition removal** (the `paralyze` removal of `exura`, `exana pox`) is satisfied for a caster without that
  condition, as in Canary: no condition owner exists (S8), so no runtime actor carries one. Any other side effect
  (conjure, unresolved) is `REJECTED`.
- **Draw root** is the pinned server artifact digest; the occurrence is per command. A dedicated per-World
  decision root is a later SIM decision.
- **V1 caster constants:** premium false (SPELL-D5, Premium not active), learned set empty (§4), skills and
  equipment zero (no V1 formula reads them).

## High-risk authority/recovery qualification

```yaml
applicable: true   # Channel-owner mutation gated by current session binding
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - "identity/binding: the actor is the committed player of exactly the admitted GameSession (player_control_facts)"
  - "identity/binding: the state entry is keyed by the exact actor (scope generation, slot, slot generation) and session"
  - "current liveness: an ended or replaced actor has no reachable state"
  - "temporal: cooldown ready times come only from the owner clock; the client sends no time"
  - "provenance: cast facts come only from the Character owner seam; the client supplies an index and target intent only"
consumer_boundaries:
  - ComposedFreshAdmission::cast_spell -> actor_spell::cast_in_channel
  - ChannelSpellStates::initialize (first-entry owner step)
  - ChannelSpellStates::commit (compare-commit)
  - serve_admitted command type 3 dispatch
mutation_operators:
  applicable:
    - "missing facts: no Character cast facts -> REJECTED, no vitals (tested)"
    - "mismatched binding: foreign GameSession -> REJECTED, no state (tested)"
    - "stale actor: removed actor -> REJECTED, entry dropped (tested)"
    - "stale successor / concurrent replay: non-successor revision -> commit refused (tested)"
    - "transport replay: repeated CommandId -> COMMAND_OUTCOME_EXPIRED, no second payment (tested)"
    - "malformed input: undecodable intent -> REJECTED before the owner (tested)"
    - "unknown or position-only index -> REJECTED (tested)"
    - "reconnect: re-initialization keeps vitals and cooldowns (tested)"
  considered_not_applicable:
    - "durable write: vitals and cooldowns are runtime-actor-local and never persisted (SPELL-D2)"
    - "client time: no time field exists on the intent"
one_invariant_per_negative_case: true
record_derived_matching_helper:
  allowed_for_positive_happy_path: false
  forbidden_for_negative_authority_or_provenance_cases: true
```

## Tests

- `spell::cast::tests` (9): canonical book order, maxima and wire bounds, heal plus payment in one successor,
  heal clamp, cooldown and shared group, caster-check dispositions, unknown index and position intent, condition
  removal, the full §3 mapping.
- `gameplay_transport::actor_spell::tests` (6): `SPELL-RL-01` registry binding, single payment and compare-commit,
  reconnect keeps state, foreign session and ended actor, gated without facts, draw bound to the command.
- `gameplay_transport::connection::tests::admitted_cast_pays_once_publishes_vitals_and_a_retry_expires`: bytes in
  and out of `serve_admitted` over a real `ChannelRuntimeV1` (snapshot with vitals, CAST plus delta, COOLING_DOWN,
  malformed and unknown REJECTED, replay expired, mana paid once).
- WP5 seam and node-boot qualification (`qualification.rs`, CI topology only): the first-control scenario adds a
  cast answered `REJECTED` while casting is gated; every other frame is unchanged.

## Validation

- `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --quiet -- -D warnings`: pass.
- `cargo test --quiet -p oteryn-game-server`: 9403 passed, 0 failed, 7 ignored (lib: 1021 passed, 2 ignored). The
  PostgreSQL targets ran without a local PostgreSQL 17.6 and are left to CI.
- `cargo test --quiet -p oteryn-protocol-oteryn`: 64 passed, 0 failed.
- `python3 tools/agents/validate_governance.py`, `python -m unittest discover -s tools/agents/tests` (36) and
  `python3 tools/repository/validate_repository_policy.py`: pass.

## Remaining for W2b

- Character-owned cast facts (vocation, magic level, level from progression) and vitals maxima per SPELL-D5, which
  opens the gate; the qualification then expects CAST, the vitals snapshot and a cooldown and mana rejection (§9
  step 4).
- `SPELL-RL-04` (measured spell book bound, admission fails closed) with the WorldProject/v2 spell lowering.
- Harmony and Serene in the runtime actor (SPELL-D8 H-1, H-2).

## Independent review

Required (§8.1: authority and state, high-risk qualification). The control plane triggers it on the frozen head.
