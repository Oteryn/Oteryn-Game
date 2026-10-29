# Wave A plan — class A mechanics (logic exists, no production caller)

Status: **proposal for owner acceptance**. Nothing is dispatched before the owner accepts it,
and no mechanic starts before #2703 is merged.

- Base: `origin/main` = `8ad7e013`.
- Source of truth: `docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`
  (rows B at `e5d0bd58`, addendum D at `e78d93fd`). Every row below was re-checked on
  `8ad7e013`; the line numbers are from that commit.
- Executor: the owner's local Claude Code CLI (PC/WSL pushes to GitHub directly), one packet
  per unit, written by the coordinator after acceptance.

## 1. Class A rows still open on `8ad7e013`

| Row | Donor file:line (Crystal pin) | Logic that exists (Oteryn) | Only callers today | Missing production caller |
|---|---|---|---|---|
| P08 — weapon proficiency kill credit | `src/creatures/players/player.cpp:12932` | `gameplay_transport/weapon_proficiency.rs:239` `dispatch_kill`; durable writer `durability/character_proficiency.rs:848` `commit_character_proficiency` | `weapon_proficiency.rs:383-385` (inline tests) | the production kill site `gameplay_transport/kill_reward.rs:1619` (where Task Board kill credit is already called) never calls `dispatch_kill` |
| C20b — PvE death item/backpack loss and Amulet of Loss | `data/libs/systems/blessing.lua:101` | `gameplay/domain/src/domain/death.rs:223` `calculate_pve_death`, `:233` `_with_bless` | `apps/game-server/src/lifted_tests/domain_death_tests.rs:93,183,321,387` | the death commit `durability/character_death/commit.rs:287` writes `NULL, ARRAY[]::uuid[]` for `amulet_of_loss_item_id, lost_item_ids`, so nothing drops |
| C17d — Challenge forces monster target | `data/scripts/spells/support/challenge.lua:5` | `gameplay/monster-ai/src/ai_think.rs:279` `CreatureAiTable::set_override`; the cast already stages `challenged_to` (`entities/src/carrier/runtime_actor_spell/staging.rs:386`) | `monster-ai/src/ai_think/think_tests.rs:619,636` | the production table (`monster-ai/src/ai_monster_melee.rs:46` `creature_ai`) never receives the override, so the monster does not retarget the caster |

Paths without a crate prefix are under `crates/gameplay/dispatch/src/` or
`crates/gameplay/durability/src/`.

## 2. Audit rows that are stale on `8ad7e013` (not wave A work)

These B.1 rows now have a production caller. They go to the wave docs PR for re-audit, not
to a mechanic PR. Remaining gaps, if any, are class B.

| Row | Production caller found on `8ad7e013` |
|---|---|
| E02 coin exchange | `dispatch/.../npc_command/coin_exchange.rs`, `coin_use.rs`; durable `durability/coin_exchange.rs:21` |
| E09 mail | `dispatch/mechanics/0305-mail.mechanic` (command 124) → `connection/routing/mail.rs:11` `authority.mail_command` |
| S07 bed sleep | `connection/authority/composed.rs:15` → `bed_sleep_production.rs:12` |
| W04 raids | `apps/game-server/src/node/raid_scheduler.rs:3` uses `durability/raid_runner.rs` (wave/spawn completeness unverified) |
| P06 wheel stat bonuses | `attack/drain.rs:228`, `composed_admission/wheel.rs:60` (max HP/mana, leech) |
| P07 atelier | `mechanics/0447-atelier.mechanic` (308, 309) → `routing/atelier.rs:11`; `dispatch_atelier` (`atelier.rs:9`) still test-only, needs re-audit |
| P08 attack perks | `attack.rs:1333` installs tracks, `attack/drain.rs:313` calls `dispatch_attack`; kill credit still open (section 1) |

## 3. Class A' — implemented, capability not offered (owner decision needed)

Logic is reachable only by fixtures because the capability is not offered. Offering changes
what the client sees, so it is not wave A work without an owner decision.

| Row | Capability | Declaration |
|---|---|---|
| C07 field runes | 91 | `crates/protocol-oteryn/src/world_object.rs:36` |
| S02 party skull | 134 | `crates/protocol-oteryn/src/party_skull.rs:6` |
| M09 fishing | 164 | `crates/protocol-oteryn/src/fishing.rs:2` |

## 4. Units, bundle vs separate

No bundle in wave A: a bundle needs 5-10 safe fragments and wave A has one (C17d). C17d goes
alone as a small PR; it can join a class B bundle in wave B instead if the owner prefers.

| Unit | Row | Shape | Why | Worker | Owned files (expected) |
|---|---|---|---|---|---|
| A1 | P08 kill credit | separate PR | persistence (durable proficiency write) | hard | `dispatch/.../kill_reward.rs`, `weapon_proficiency.rs`, `channel_states.rs` (read only if possible), PG case in `gameplay_transport/postgres_tests/` |
| A2 | C20b death loss | separate PR | durable value (items) and persistence | hard | `domain/src/domain/death.rs`, `durability/character_death/commit.rs`, `durability/character_death.rs`, the dispatch death caller, PG case in `apps/game-server/tests/support/authority_cases/` |
| A3 | C17d challenge | separate small PR | no migration, protocol, persistence or durable value; alone, not bundle-sized | impl | `dispatch/.../native_combat_cast.rs` or the commit apply site, `monster-ai/src/ai_monster_melee.rs`, test in `monster-ai` |

Hot-file check: no two units share a file above; none touches routing, `mechanics/`,
registries, golden files or `connection.rs`. A2's dispatch death caller lives near
`composed_admission/death.rs`, which #2703 changes: A2 starts only from a `main` that
contains #2703. A1 and A2 both touch durability, in different modules (`character_proficiency`
vs `character_death`); no shared file.

## 5. Numbers

`scripts/next-free-numbers.py` on `8ad7e013` (open PRs checked by hand: #2711 CI only,
#2703 has no migration): next free migration 531, capability 172, command 341, mechanic
ordinal 448, offer order 61.

- A2: migration **531** reserved, used only if its spike shows the existing columns are not
  enough (expected: not needed).
- A1, A3: no numbers expected.

## 6. Batches

One batch, three heads: A1, A2, A3 on a fresh `main` after #2703 merges, labelled
`heavy-tests`, merged one after another per `.claude/skills/oteryn-dev/references/ci-merge.md`.
A unit that is not finished does not enter the batch; the rest land without it.

## 7. Owner questions

1. Accept wave A as A1-A3, all separate PRs?
2. C17d now alone, or held for a wave B bundle?
3. Offer capabilities 91, 134, 164 (section 3): yes / no / later wave?
