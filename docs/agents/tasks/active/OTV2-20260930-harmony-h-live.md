# OTV2-20260930-harmony-h-live

```yaml
task_id: OTV2-20260930-harmony-h-live
title: Monk Harmony and Serene in the live runtime actor, Harmony admission (SPELL-D8 H-live, SP-HARMONY-GATE folded in)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
pr: null
allocation_comment: "#162 5910691466 (point 3, H-live; SP-HARMONY-GATE folded in)"
base_branch: main
branch: claude/nifty-goldberg-omhdpr
base_sha: aeb13cf
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "Oteryn: spell state" (Claude Code)
created_at: 2026-09-30
execution_policy: continuous_progress
risk: high (fenced Character writes at actor end, admission of Harmony spells)
owned_paths:
  - apps/game-server/src/spell/mod.rs
  - apps/game-server/src/spell/authoring.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/cast_tests.rs
  - apps/game-server/src/spell/tests.rs
  - apps/game-server/src/spell/harmony_tests.rs
  - apps/game-server/src/spell/chain_tests.rs     # CasterState field only
  - apps/game-server/src/spell/part_d_tests.rs    # CasterState field only
  - apps/game-server/src/spell/party_tests.rs     # CasterState field only
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/src/gameplay_transport/monk_save.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/agents/tasks/active/OTV2-20260930-harmony-h-live.md
public_contracts: []
owning_contract: docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md §8.2 (SPELL-D8)
depends_on: ["H-1 #1360 (migration 0026, durability::monk_state)", "H-2 #1243 (spell::harmony)"]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Basis

- Owner decisions: D209 (the Serene flag is not durable), Q1=b (the remaining forced Serene time is durable), DEATH-1
  Harmony 0 at death (H-1, in `commit_character_death` and the 0026 guard).
- Carried LOWs of the #1360 KEEP review (5913880920): a negative PG case for a death transition that keeps Harmony,
  the rejecting rule named by its message, and a restart between save and load.

## Outcome

- **Runtime actor.** `PlayerSpellState` holds an `Option<MonkState>` (monks only), loaded from the durable values.
  `ChannelSpellStates::initialize` runs the Serene initialization evaluation in the first-entry owner step;
  `resume` runs it again after an FND-04B recovery; control loss detaches the actor. `serve_admitted` runs the
  evaluation every 1000 ms while the actor has `ACTOR_VITALS`, and a Serene change publishes a delta.
- **PRIMARY COMMIT.** `cast` refuses a monk command before the initialization evaluation, passes the spender
  multiplier (no virtue, §4 interim rule) into the damage bounds, and commits `commit_builder` / `commit_spender`
  in the same successor value as mana and cooldowns. `ACTOR_VITALS` carries the live Harmony and Serene.
- **Load and save.** The first entry reads `read_character_monk_state` before the owner lock and fails closed on a
  failed or corrupt load. Before the lease release (grace expiry and abandoned resume) `save_monk_state` writes
  `commit_character_monk_state_save` under a fence read from the current durable GameSession and Character
  root; a lost response is reconciled by occurrence; a fenced-out write lets the release go on; an unknown outcome
  holds the release back.
- **Admission.** The fail-closed `harmony_role` gate is replaced by the `HarmonyRole` field (builder, spender),
  admitted only for monk-only spells. `monk_focus` stays fail-closed.
- **Dormant path.** `character_cast_facts` still returns `None` (no GAME-CHAR vocation owner), so no production
  actor has spell state yet; the monk load and save run only once cast facts exist.

## Validation

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`,
  `cargo test --locked -p oteryn-game-server --lib --bins` (1167 passed).
- PostgreSQL 17.6 (pinned digest): `character_authority_postgres` (881 passed), `durability_postgres` (770 passed).

## Closeout

Pending: PR, freeze, independent review. The final authoring commit moves this record to `archive/`.
