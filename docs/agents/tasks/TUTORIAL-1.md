# TUTORIAL-1 — Newhaven tutorial islet: progress store, notices frame, spawn switch (consolidated task card)

```yaml
task_id: TUTORIAL-1
title: "TUTORIAL-1: Newhaven tutorial (start islet, steps 1-7, notices frame, durable progress)"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 4 row TUTORIAL-1 (row 139); §5 Wave 5; §6.1"
brief: "CP D1426 scenario brief (#2186 comment 6241833560) + owner D1463 (2026-10-19), both reproduced below; audit evidence docs/agents/evidence/newhaven/tutorial-audit-2026-10-19.md"
class: "impl + protocol + persistence"
base_branch: main
branch: null
pr: null
needs_migration: true   # character tutorial progress; next free number at allocation time (only one migration PR in the queue). The draft's 0130 is reserved for MAP-PICKUP-1 — never use it
needs_registry: true    # tutorial notices capability + state domain; draft used capability 31 / domain 23 — re-check at the lease
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2186, branch skeleton/tutorial-1

## Goal

New Characters start on the Newhaven tutorial islet and walk the global tutorial
(steps 1–7) with durable, per-Character serialized progress; hints and the islet-hole
text reach the client through a capability-gated notices frame. The draft implemented
steps 1–4, the frame, the spawn switch and the progress store; steps 5–7 remained open.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| NEWHAVEN-TELEPORT-1 (forcefield script/bundle teleport) | MERGED (#2018) |
| Newhaven tutorial audit evidence | MERGED (#2288) |
| Wave 4 qualified interfaces (quest runtime, death/respawn, loot) | partial; confirm per step at allocation |
| Death respawn rule for step 6 | OWNER QUESTION (islet vs hometown) |
| Anna / Seagull NPC placements, Thais Peninsula landing | not on main (follow-up) |
| Item definitions 48843, 48845, 48847 | missing in `content/items/definitions/` |
| Newhaven audit draft #2255 | open (evidence only; not a consolidation draft) |

## Accepted decisions

- **CP D1156** (superseded in part): no invented island/route/reward without a CP brief.
- **CP D1426:** tutorial applies **only** on the Newhaven start island; StartThais is a
  temporary test start where the tutorial is inert.
- **Owner D1463 (2026-10-19):** supersedes D1410's "no spawn change": new Characters
  start on the islet temple (32534,32513,7); forcefield (32538,32514,7) leads to
  (32544,32515,7); Anna sails to **Thais Peninsula** (not Targuna) from level 8.
- **Owner Q3 (a):** a tutorial notices frame (protocol change) is authorised.
- The gate matches exactly the two committed Newhaven land components (islet + island)
  from B3 ground classes; other land inside the bounding box stays inert.
- Hint ids come from the client's tutorial table (`opentibiabr/otclient` 53d3a8ac
  `modules/game_tutorial/const/vocations.lua`: 3 Knight, 4 Paladin, 5 Sorcerer,
  6 Druid, 11 Monk); the server never invents a number. Crystal sends 3 to every
  vocation; we send the base-vocation hint instead; vocationless gets none and keeps
  the track armed. Hint 2 from `movement_select_vocation.lua` kept as is.
- Corruptor loot grants only defined Items; a vocation whose Items are missing gets
  nothing (no substitute) until definitions exist.

## Sources

- Island: `content/world/areas/islands/islands-00000-00059.json`, record
  `oteryn:area.island.newhaven` (TibiaWiki 109339 rev 1197143, frame
  `global-target-2026-09-27`): island x 32506–32634, y 32405–32522, z 7; islet
  component x 32532–32539, y 32511–32518, z 7.
- Rules: `zimbadev/crystalserver` 9f5a72c6, `data-global/scripts/quests/newhaven/`
  (donor corpus `tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz`):
  `on_login_newhaven.lua` b101507c, `movement_init_newhaven.lua` 9d4adb9b,
  `movement_select_vocation.lua` 8366ff4e, `movement_update_tutorial_hunting.lua` 4c3cb41f,
  `#ondroploot_the_corruptor.lua` a6984946, `on_death_newhaven.lua` 680ec85a.
- Tracks: `content/quests/missions/quest-state.json` quest `newhaven_quest`
  (`newhaven_citizen`, `newhaven_tutorial_hunting`, `newhaven_new_loot_the_corruptor`).
  The `oteryn:quest.newhaven_quest` stages are an approximation and are not used.

## Steps

| # | Trigger | Condition | Effect | Stored progress |
|---|---|---|---|---|
| 1 | Login | vocationless, no account access, not a citizen (no row or departed) | placed at islet temple (32534,32513,7) | step islet |
| 2 | Islet hole (32533,32513,7) | always | pushed back; text "I should not climb down again but rather venture forth." | unchanged |
| 3 | Islet forcefield (32538,32514,7) | always | moved to (32544,32515,7); hint 2 if vocationless | step island; hunting + Corruptor tracks armed |
| 4 | (32561,32485,7) or (32562,32485,7) | hunting armed | base-vocation hint | hunting cleared |
| 5 | The Corruptor drops loot | Corruptor armed | vocation loot | Corruptor cleared |
| 6 | Death | citizen, vocationless, no account access | placed at islet temple | step islet |
| 7 | `a_seagull` / `anna` take the Character away | any row | none | step departed; tracks cleared |

A citizen's login (islet/island) resumes with no placement and unchanged progress.

## Acceptance assertions (D1426)

- **A1** non-Newhaven start (StartThais): inert gate, no writes or effects on any cell.
- **A2** Newhaven-start vocationless Character placed on the islet at login; a GM or a
  Character with a vocation is not.
- **A3** steps 2–5 exactly as above; hint and loot once each.
- **A4** step 6 returns a vocationless citizen to the islet; step 7 clears every track.
- **A5** writes serialized per Character (row lock + CAS version); loser gets a typed
  `Conflict` with the stored progress and writes nothing, even when equal (retry or
  stale equal-state writer; no occurrence identity); replay writes nothing.
- **A6** durable progress; resume after reconnect (new connection generation) and node
  restart; stale connection/lease/session read and write nothing.

## Reference behaviour from the draft (re-implement, do not copy)

- **Notices frame:** snapshot type 1 = empty state `[0x08, 0x01]`; delta type 1 = one
  notice, `hint_id` 1..=255 or `status_text` 1..=255 UTF-8 bytes, ≤260 bytes, golden
  vectors; no frame without the capability (server behaviour unchanged).
- **Session:** `TutorialSessions` reads stored progress at playable admission, places a
  first-login vocationless Character, refuses the islet hole before the map check with
  its status text, writes forcefield/hunting transitions after a committed step with no
  lock held during the write (CAS conflict reapplies on the stored row). Notices sent as
  domain deltas after the step's frame. E2E through real dispatch:
  `tutorial_notices_follow_the_hole_and_the_forcefield_steps_through_dispatch`.
- **Review fixes kept as requirements (Codex 2026-10-19):** a failed admission read does
  not drop tracking — a Character with no/unread row is tracked on first island login or
  islet placement and re-read before the next write; a tracked Character beyond the
  islet whose stored step is still islet gets the forcefield transition at admission or
  next step; the client decodes its map window around the join position
  (`World::load_around`, `play.rs set_world`, `windows_shell.rs` resumed).
- **Movement profiles:** overlay `movement-item-profiles.json`
  (`OTERYN_NATIVE_MOVEMENT_ITEM_PROFILES/v1`) for the 17 plain Items on the temple →
  forcefield path and arrival cell (3892, 3893, 6216, 6217, 6373–6382, 6385, 19394,
  37001); each refused unless the Canary pin states it walkable; hole 51921 blocks.
  `register_spell_families.py` admits it (`MOVEMENT_ITEM_PROFILES_RECORD_SET`,
  `MOVEMENT_ITEM_PROFILES_NOT_WALKABLE`; 430 records).
- **Spawn:** pin `entry_start` = islet temple (32534,32513,-7); `world-bundle-compiler
  pin-check` refuses a start with non-walkable plain Items or no steppable orthogonal
  neighbour; node boot refuses a solid start; `preprod-topology.toml` and the client
  `START` follow the pin.
- **Designed values (not yet used):** Anna (32552,32497,7) dir 2, level 8
  (`world-npc.xml` ed65ac56, `anna.lua MIN_LEVEL_TO_TRAVEL`); Seagull (32535,32512,7);
  Anna landing (32472,32288,7) OTERYN_DESIGN beside Hector; Corruptor loot Knight
  3359/3285/3375, Paladin 10321/48843, Druid 3066/8871, Sorcerer 3074/8871, Monk 48847/48845.

## Open questions / remaining

- Step 6 respawn rule (islet or hometown) — owner question.
- Steps 5 and 7 at runtime; Anna/Seagull placements; missing Item definitions.
- 282 more walkable plain Item ids on the island are not profiled.
- Out of scope (own tasks): starter items (`lib/others/newhaven.lua`), map marks, Dawnport
  death branch, level-8 door (`on_use_newhaven.lua`). The death rule reads the current
  step where the reference reads the citizen track at last login.

## Target layout (ADR-0022)

- Pure step reducer: `crates/gameplay/quests/src/tutorial/` (new: `mod.rs`, `steps.rs`,
  `steps_tests.rs`); registration in `crates/gameplay/quests/src/lib.rs` (wiring-only).
- Progress store: `crates/gameplay/durability/src/durability/character_tutorial.rs` (new);
  PG cases `apps/game-server/tests/support/tutorial_progress_postgres_cases.rs` (new),
  included from `apps/game-server/tests/character_authority_postgres.rs`; migration
  under `apps/game-server/migrations/` (next free number).
- Protocol: `crates/protocol-oteryn/src/tutorial_notices.rs` (new) +
  `tutorial_notices_tests.rs` (new); `docs/contracts/protocol-oteryn/v1/tutorial_notices_v1.proto`
  (new); registry `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`.
- Session: `crates/gameplay/dispatch/src/gameplay_transport/tutorial_session.rs` (new) +
  `tutorial_session_tests.rs` (new); wiring in `connection.rs`, `mod.rs`,
  `capabilities.rs`; authority-field touch-ups in `kill_reward_postgres_e2e_tests.rs`,
  `monk_save_postgres_tests.rs`.
- Content/tools (exist): `tools/content-schema/native-gameplay/build_source_item_profiles.py`
  (+ `movement-item-profiles.json` new), `tools/content-migration/register_spell_families.py`,
  `crates/gameplay/content/src/content/native_gameplay.rs` (record count),
  `tools/world-bundle-compiler/src/main.rs`, `tools/world-bundle-compiler/tests/pin_check.rs`,
  `content/world/pins/oteryn.json`, `deploy/synology-game/preprod-topology.toml`,
  `apps/client/src/world.rs`, `apps/client/src/play.rs`, `apps/client/src/windows_shell.rs`.
- Scenario test: `apps/game-server/tests/tutorial_newhaven.rs` (new).

## Acceptance criteria

- [ ] A1–A6 above, each with a named test.
- [ ] Login and step-in (steps 1–4) wired through dispatch with notice frames, E2E test in a named CI job.
- [ ] Spawn switch to the islet with movement profiles and the pin-check neighbour rule.
- [ ] Steps 5–7 at runtime (after the owner answers the respawn question).
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-game-server --test tutorial_newhaven
cargo test --locked -p oteryn-gameplay-quests tutorial
cargo test --locked -p oteryn-protocol-oteryn tutorial_notices
cargo test --locked --release -p oteryn-world-bundle-compiler   # pin_check runs in release
cargo test --locked -p oteryn-client --lib
python3 tools/content-migration/register_spell_families.py --check
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test character_authority_postgres tutorial_progress -- --test-threads=1
python3 tools/agents/validate_governance.py
```
