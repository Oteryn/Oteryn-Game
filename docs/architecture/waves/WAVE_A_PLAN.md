# Wave A plan — class A mechanics plus one bundle

Status: **accepted by the owner 2026-10-29** (questions 1 and 2: A1, A2 and the A3 bundle, class B
fragments allowed in A3). No mechanic starts before #2703 is merged.

- Base: `origin/main` = `8ad7e013`. Line numbers are from that commit.
- Source of truth: `docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`
  (rows B at `e5d0bd58`, addendum D at `e78d93fd`). All 133 PARTIAL/MISSING rows were
  re-classified on `8ad7e013` (section 7). Donor file:line is copied from the audit row
  (verified there against the Crystal pin; Crystal source is not in this clone).
- Classes: **A** logic exists, only test callers; **A'** implemented but closed by a
  capability or gate (owner decision); **B** PARTIAL missing a piece; **C** MISSING;
  **STALE** the audit gap is closed on `8ad7e013`.
- Executor: the owner's local Claude Code CLI (PC/WSL pushes to GitHub directly), one packet
  per unit, written by the coordinator after acceptance.
- Mixing class B fragments into the wave A bundle departs from the "all A, then B" order and
  needs the owner's acceptance (question 2).

## 1. Class A rows on `8ad7e013`

| Row | Donor file:line | Logic (Oteryn) | Only callers today | Missing production caller |
|---|---|---|---|---|
| P08 kill credit | `src/creatures/players/player.cpp:12932` | `dispatch/.../weapon_proficiency.rs:239` `dispatch_kill`; writer `durability/.../character_proficiency.rs:848` | `weapon_proficiency.rs:383-385` (inline tests) | kill site `dispatch/.../kill_reward.rs:1619` (Task Board credit is already called there). Attack perks are wired: `attack.rs:1333`, `attack/drain.rs:313` |
| C20b death item loss, Amulet of Loss | `data/libs/systems/blessing.lua:101` | `gameplay/domain/src/domain/death.rs:223,233` | `apps/game-server/src/lifted_tests/domain_death_tests.rs:93,183,321,387` | `durability/.../character_death/commit.rs:287` writes `NULL, ARRAY[]::uuid[]` |
| C17d Challenge retarget | `data/scripts/spells/support/challenge.lua:5` | `monster-ai/src/ai_think.rs:279` `set_override`; cast stages `challenged_to` (`entities/src/carrier/runtime_actor_spell/staging.rs:386`) | `monster-ai/src/ai_think/think_tests.rs:619,636` | production table `monster-ai/src/ai_monster_melee.rs:46` never gets the override |
| C17c Creature Illusion | `data/scripts/spells/support/creature_illusion.lua:6` | planner accepts `creature_appearance`: `gameplay/spell/src/spell/native.rs:104` | — | dispatcher key gate `dispatch/.../parameter_cast.rs:850` admits only `locate_message`, `house_access`, `vertical_move` |

## 2. Units

| Unit | Content | Shape | Worker | Owned files (expected) |
|---|---|---|---|---|
| A1 | P08 kill credit | separate PR (PERS) | hard | `kill_reward.rs`, `weapon_proficiency.rs`, PG case in `gameplay_transport/postgres_tests/` |
| A2 | C20b death loss | separate PR (VAL, PERS) | hard | `domain/death.rs`, `durability/character_death/commit.rs`, `durability/character_death.rs`, the dispatch death caller, PG case in `apps/game-server/tests/support/authority_cases/` |
| A3 | bundle, 10 fragments (section 3) | one bundle PR | impl | section 3 |

## 3. Bundle A3 — fragments

Each fragment is ≤ ~100 lines, has no migration, protocol, persistence, auth/session or
durable value, and is its own commit with its own test. A fragment that turns out larger,
needs one of those, or fails is dropped from the bundle. Sizes are estimates.

| # | Row | Class | Fragment | Oteryn evidence | Donor | Test | Est. | Files |
|---|---|---|---|---|---|---|---|---|
| 1 | C17d | A | feed Challenge's `challenged_to` into `set_override` | `ai_think.rs:279`, `staging.rs:386` | `challenge.lua:5` | challenged monster targets the caster | S | `ai_monster_melee.rs`, `monster_ai_cycle.rs` |
| 2 | C17c | A | admit `creature_appearance` at the dispatcher gate | `parameter_cast.rs:850`, `native.rs:104` | `creature_illusion.lua:6` | cast changes appearance via real dispatch | M (drop if >100) | `parameter_cast.rs` |
| 3 | S13a | B | party-spell member range to the donor's | `spell/party.rs:66`, `ordinary_combat/prepare.rs:113` | `heal_party.lua:20` | member beyond 3 tiles is covered | S | `spell/party.rs` |
| 4 | P10 | B | daily streak feeds resting regeneration | `daily_reward.rs`, `bed_regen.rs` | `data/libs/functions/player.lua:775` | streak changes resting regen | S | `bed_regen.rs` |
| 5 | W01b | B | blockRespawn / unseen immediate spawn | `world_respawns.rs:34`, `entities/src/carrier/spawn.rs:21` | `spawn_monster.cpp:339` | spawn blocked while seen | S | `world_respawns.rs` |
| 6 | C16b | B | real-dispatch test, paralyze rune 3165 | `ordinary_combat.rs:84` | `paralyze_rune.lua:5` | test only | S | test file |
| 7 | C16c | B | real-dispatch test, Convince Creature ownership | `native_companion_item_cast.rs:11` | `convince_creature.lua:3` | test only | S | test file |
| 8 | C16d | B | real-dispatch test, Chameleon 200 s | `native_world_item_cast.rs:818` | `chameleon.lua:6` | test only | S | test file |
| 9 | C17b | B | test, Invisible 200 s and observer visibility | `ordinary_combat.rs:92`, `monster_ai_cycle.rs:610` | `invisible.lua:5` | test only | S | test file |
| 10 | C18b | B | position-cast tests, 5 crosshair spells | `ordinary_combat/prepare.rs:282` | `spells.cpp:1252` | test only | S | test file |

Not in the bundle on purpose: S13c (content and world pins regenerate), and C10, C11, C15,
C20a, S15b, W05a (a recorded divergence the owner has to adopt first).

## 4. Hot files and shared surfaces

- No unit touches connection/routing, `mechanics/`, registries, golden files,
  `connection.rs` or the capability capacity constant.
- A1 (`kill_reward.rs`, `weapon_proficiency.rs`), A2 (death files) and A3 (section 3 files)
  share no file. A2 and A1 are in different durability modules.
- A2's dispatch death caller is next to `composed_admission/death.rs`, which #2703 changes:
  A2 starts only from a `main` that contains #2703.
- A3 fragments 1 and 9 both touch `monster_ai_cycle.rs`, which is fine inside one PR.

## 5. Numbers

`scripts/next-free-numbers.py` on `8ad7e013` (open PRs checked by hand: #2711 CI only,
#2703 has no migration): next free migration 531, capability 172, command 341, mechanic
ordinal 448, offer order 61.

- A2: migration **531** reserved, used only if its spike shows the existing columns are not
  enough (expected: not needed).
- A1, A3: no numbers.

## 6. Batch

One batch, three heads (A1, A2, A3) on a fresh `main` after #2703 merges, labelled
`heavy-tests`, merged one after another per `.claude/skills/oteryn-dev/references/ci-merge.md`.
A unit that is not finished stays out; the others land without it.

## 7. Re-classification of the 133 PARTIAL/MISSING rows on `8ad7e013`

Counted from the row-by-row table: STALE 7, A 4, A' 9, B 70, C 43.

- **STALE (7):** E02 (`npc_command.rs:608-610`), E08 (`mechanics/0062-depot_search.mechanic`,
  command 182), E09 (`mechanics/0305-mail.mechanic` → `routing/mail.rs:11`), P06
  (`attack/drain.rs:228`, `composed_admission/wheel.rs:60`), P17
  (`mechanics/0333-exiva_settings.mechanic`, command 170), S07
  (`connection/authority/composed.rs:15`), W04 (`apps/game-server/src/node/raid_scheduler.rs`).
  These go to the wave docs PR for re-audit.
- **A (4):** section 1.
- **A' (9), owner decision:** C07 capability 91, M09 164, E03 171 (`production:false`,
  `bundle_world:true`); P03 140 (`production:false`, `bundle_world:false`); I08 Primal Bag
  `playable:false`; P14/P15 env gate `OTERYN_ENABLE_PROVISIONAL_TRAINING`
  (`loop_training/mod.rs:41`); M11a/M11b closed on bundle World (`seam_rooms.rs:65-67`).
- **S02:** `capabilities/0134-party-skull-v1.json` says `production:true`, but the audit says
  unoffered. Probably stale, to re-check in the docs PR (counted in B above).
- **B (70)** and **C (43):** later waves; class B small fragments feed future bundles.

## 8. Owner questions

1. Accept wave A as A1, A2 (separate) and the A3 bundle?
2. Mix class B fragments into A3 (departs from "all A first")?
3. A' gates: offer capabilities 91, 164, 171, 140; open Primal Bag; open provisional
   training; open vertical moves on bundle World — which ones, and in which wave?
4. Recorded divergences to adopt or keep: C10, C11, C15 (donor values), C20a (D59), S15b
   (4 s), W05a (server save 10:00 vs 06:00 Berlin).
5. P23a Hunting Tasks: replaced by the Task Board (numbered decision), or build them?

## 9. Owner answers, 2026-10-29

- Q1, Q2: accepted (A1, A2, A3 bundle with class B fragments).
- Q3: open every A' gate (capabilities 91, 164, 171, 140; Primal Bag; provisional training;
  vertical moves on bundle World). Each opening is its own small PR in a later wave.
- Q4:
  - W05a server save stays at 10:00; S15b reconnect protection stays 4 s.
  - C20a (replaces D59): levels 1-23 lose 10% as on Global, but of the experience needed for
    the next level, not of the character's total experience. Variant (whole level span or the
    rest to the next level) is pending.
  - C15 stamina: Global. Crystal at the pin drains stamina only inside
    `Player:onGainExperience` (`data/events/scripts/player.lua:484`, call at `:506`;
    `useStamina` at `:99`: 1 minute per 60 s, 2 minutes after a gap over 60 s). Never in a
    protection zone without a kill. Replaces the OFFLINE-0 R1 damage-activity signal.
  - C11 Monk Harmony: Global (news 8944: 7% + 0.005% per level, Virtue of Harmony x1.5 /
    x2 while Serene, Serene with 7 or fewer adjacent creatures).
  - C10 Monk and Exalted Monk regeneration: same as Paladin and Royal Paladin.
- Q5: build the Hunting Tasks (P23a), class C, later wave.
