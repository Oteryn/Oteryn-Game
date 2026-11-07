# TUTORIAL-1 — Newhaven tutorial islet: progress store, notices frame, spawn switch (consolidated task card)

```yaml
task_id: TUTORIAL-1
title: "TUTORIAL-1: Newhaven tutorial (start islet, steps 1-7, notices frame, durable progress)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 4 row TUTORIAL-1 (row 139); §5 Wave 5; §6.1"
brief: "CP D1426 scenario brief (#2186 comment 6241833560) + owner D1463 (2026-10-19), both reproduced below; audit evidence docs/agents/evidence/newhaven/tutorial-audit-2026-10-19.md"
class: "impl + protocol + persistence"
base_branch: main
branch: w3/tutorial-1
pr: 2308
needs_migration: true   # allocated 0136 provisionally on 2026-10-22; integrator renumbers at merge; 0131 belongs to BAGS-USE-1
needs_registry: true    # tutorial notices capability + state domain; draft used capability 31 / domain 23 — re-check at the lease
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2308](https://github.com/Oteryn/Oteryn-Game/pull/2308), [#2311](https://github.com/Oteryn/Oteryn-Game/pull/2311). Newhaven islet start, seven-step progress/notices, Corruptor loot and free travel/death handling are merged; admission row-lock repair followed in #2311. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/tutorial_session.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

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
| Death respawn rule for step 6 | resolved: owner selected islet, 2026-10-22 |
| Anna / Seagull NPC placements, Thais Peninsula landing | authored locally in this continuation |
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

- Step 6 respawn rule is resolved by the Owner decision 2026-10-22 below.
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

## Prerequisite audit 2026-10-22

Local baseline: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`, branch
`w3/tutorial-1`, initially clean. No frozen candidate or PR was created. Work is
local only under the owner's prohibition on Git writes and external mutations.
The bound META organization policy was resolved at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb` before material work.

- **PROVEN:** tutorial reducer, progress adapter and notices codec are absent from
  the current gameplay/protocol owners. Capability 31 and domain 23 are unused in
  the local registry; they have not been registered or offered.
- **PROVEN:** `gameplay_transport/npc_travel.rs` explicitly remains a private
  component awaiting production integration. Workspace caller search found its
  `FrozenTravel`/`TravelHoldSlot` composition only in its child tests, with no talk
  dispatch caller. Step 7 therefore needs a production prerequisite in this lane,
  using the existing `oteryn-gameplay-npc::npc_travel` hold state and durability
  travel operations, rather than a second travel implementation.
- **PROVEN:** existing owners to reuse include
  `durability/character_progression.rs` gameplay fences,
  `durability/quest_state.rs` transaction/lock conventions,
  `durability/spell_familiar_group.rs` current admitted group observations,
  `gameplay_transport/last_position.rs` admission placement and
  `gameplay_transport/kill_reward.rs` committed creature reward settlement.
  No helper, fixture, harness or module registration was copied or added.
- **CONFLICT:** the steps table and A4 select the islet for step 6, but both
  Dependencies and Open questions explicitly reserve islet versus hometown for
  the owner, and Acceptance gates steps 5–7 on that answer. The supplied accepted
  decisions do not resolve that reservation. The continuation instruction says
  to stop at an owner-only policy decision; no respawn behavior was inferred.

## Owner decision 2026-10-22

OWNER DECISION (project owner, 2026-10-22), answering the question your previous run in this worktree stopped on: option (a): a character that dies during the tutorial respawns on the tutorial islet, exactly as step 6/A4 of the card specifies; steps 5-7 are unblocked.

This supersedes the pending respawn question above. Step 6 applies to a
vocationless, non-access citizen; other deaths retain the existing death contract.

## Integrator decision 2026-10-22: prerequisite implemented in this lane

The integrator assigns the missing production travel prerequisite to this lane.
The owner has now resolved the respawn question. Implementation is in progress;
the audit above is retained as historical evidence.

Planned layout before coding: keep the reducer in `quests/src/tutorial/steps.rs`,
the progress store in `durability/src/durability/character_tutorial.rs`, the codec
in `protocol-oteryn/src/tutorial_notices.rs` and tutorial composition in
`dispatch/src/gameplay_transport/tutorial_session.rs`. Add the missing production
travel composition as its own `newhaven_travel.rs` mechanic module beside the
existing dispatch travel adapter, reusing that adapter and its lower-layer owners.
Use dedicated child test files and the named PostgreSQL/scenario suites in the
Target layout. Shared slot files receive wiring only. Migration remains 0136.

The travel prerequisite now has production callers in `newhaven_travel.rs` and
`newhaven_arrival.rs` in dispatch, backed by durability's `newhaven_travel.rs` and
its existing `npc_travel.rs` consume owner. Other new mechanics are
`tutorial_movement.rs`, `tutorial_corruptor.rs`, `tutorial_islet_placement.rs`,
`tutorial_death.rs`, and the protocol/session notices modules. Bundle floors are
negative; actor positions, progress/death cells and client join centers use runtime
Tibia z (7 here), through the existing bundle conversion owner.

No acceptance checkbox is complete: the full composed scenario is still missing.

## Continuation boundary before Slice TUTORIAL-1c

The earlier admission prerequisite was that `CastFactsLoad::NoVocation` created no
`PlayerSpellState`, so tutorial combat could not reach death. SPELL-D5 already
provides the accepted “none” maxima and regeneration. BAGS-USE-1 (#2301,
migration 0131) is now merged; `item_use.rs::base_vocation` is available to this
slice and its ownership question is resolved.

The compiler `pin_check` fixture runs Git add/commit in scratch repositories. It
remains excluded under the owner's explicit Git prohibition. Final World pin
qualification belongs to the integrator. No frozen SHA, PR, external write, Git
write or paid review was made.

## Continuation validation and reuse 2026-10-22

- Full dispatch library: 904 passed; 14 existing ignored tests unchanged. Session:
  65 passed. Client library: 84 passed. Final coordinate repair: focused tutorial
  tests passed (content 1, dispatch 2, quests 4) and all five PG cases passed again.
- Full content/protocol/quests libraries passed earlier: 339/286/36 tests; the one
  existing ignored content test is unchanged. Notices golden vectors passed.
- Exact filters: `test --locked -p oteryn-gameplay-quests tutorial`;
  `test --locked -p oteryn-gameplay-content tutorial`;
  `test --locked -p oteryn-gameplay-dispatch tutorial`;
  `test --locked -p oteryn-protocol-oteryn tutorial_notices`;
  `test --locked -p oteryn-session tutorial_notices`;
  `test --locked -p oteryn-game-server --test character_authority_postgres tutorial -- --test-threads=1`;
  `test --locked --release -p oteryn-world-bundle-compiler --bin oteryn-world-bundle-compiler entry_start`;
  `test --locked -p oteryn-client --lib` (all use `cargo +1.95.0`).
- Package-scoped all-targets clippy `-D warnings` covers dispatch, durability, quests,
  content, protocol, session, client, game-server, compiler and synthetic-client-harness.
  Final all-targets clippy and `cargo fmt --all -- --check` both passed.
- World compile/equivalence passed: 19,373,519 tiles, bundle digest
  `95f05a4264909bd9b8ac15c40449aa6f51daf64b0debb2763c46964d02b233f6`.
  Pin `inputs_digest` is prospective worktree blob hashing, not index qualification.
- Spell regeneration `--check`, NPC placement `--check`, exact land mask `--check`,
  governance validation and all 124 governance unit tests passed.
- Reused: quest reducer across dispatch/death/travel/loot; root gameplay fences,
  semantic transactions and existing build/group reads; `TravelHoldSlot`/`FrozenTravel`,
  pending-arrival consume, admitted placement/fallback and last-position capture;
  existing corpse loot/MINT selection; moved-step spatial/map/container framing;
  existing protocol field codecs and selected-domain revision store.
- Reused authoring: `convert_islands` ground/component loader; pinned source Item
  importer/binding/fetch functions; NPC placement converter and WorldProject lowering;
  existing `git_blob_sha` for prospective input hashing. No dependency download.
- Reused tests: extracted the existing StepAuthority body (deleted its old body),
  connection driver, session join peer, client `repo()`, bestiary PG harness,
  last-position `run`/`record`/`death`, combat `death_fixture`/`capture`, and native
  travel fixture's `placed`/`occurrence`. Child files are mounted once per owner;
  no copied fixture body, duplicate module load or duplicate test registration.
- Whole-diff checklist re-read; new fields/variants/trait method and their match,
  constructor and implementation sites searched across `apps/`, `crates/`, `tools/`.
  Production callers exist; the full composed E2E requirement remains unfulfilled.
- Files growing >200 lines: dispatch `newhaven_travel.rs` (263),
  `newhaven_arrival.rs` (216), `tutorial_session.rs` (248): transport/composition only,
  with durable rules below them. `movement-item-profiles.json` (2416) and generated
  `spell-native-profiles.json` (+2398): normalized Content records, not runtime logic.
  No existing handwritten code file grows by ~150 lines; shared slots are wiring.

## Slice TUTORIAL-1c

Planned module layout, before coding: put vocationless admission facts in
`dispatch/src/gameplay_transport/spell_character_facts/vocationless_vitals.rs`;
keep the existing `PlayerSpellState` as the vitals owner, with explicit
vocationless saves in `spell/src/spell/cast/vocationless_vitals.rs` and first-choice
installation in `spell/src/spell/mana_training/first_vocation_vitals.rs`.
`dispatch/src/gameplay_transport/first_vocation_vitals.rs` only composes the
existing NPC/build writer and ordered last-position projection. Dedicated child
files hold unit and PostgreSQL tests. Shared owner files receive registrations
and owner calls; the obsolete admission skip is removed. No new schema is needed;
unmerged migration 0136 is unchanged.

The genuine `Vocation::None` now reaches Character-owned admission facts, item-use
requirements, food regeneration, familiar vocation selection and look projection.
A missing progression row uses the accepted level-one seed; an existing row and
build supply their real level and magic. Admission creates the existing vitals
owner, restores saved pools, and retains live pools on same-session replay.

First choice flushes training through its existing sequencer, saves absolute
health/mana under the existing session fence, and commits the existing build
receipt. Installation changes facts and owned training in place: pools,
conditions, cooldowns, regeneration progress, hit streaks and any newly accrued
tries survive. Exact installation replay changes no vitals revision; a concurrent
lethal hit stays at zero. Vocationless maximum snapshots remain explicit so a
prechoice capture cannot refill a larger class maximum on restart. Historical
receipts still require independently current actor/NPC/session ownership before
live installation.

Tests (all commands use the queued `cargo +1.95.0` wrapper):

- `test --locked -p oteryn-gameplay-dispatch --lib vocationless_vitals --quiet`:
  owned none facts, actual first-entry/re-entry, foreign-session hit refusal,
  lethal creature damage, real death receipt and respawn.
- `test --locked -p oteryn-gameplay-dispatch --lib first_vocation_vitals --quiet -- --test-threads=1`:
  actual NPC dispatch, training flush, preservation of wounded and full pools, one choice
  receipt, retained-receipt replay with and without prior live installation, and
  admission from a reopened durable root.
- `test --locked -p oteryn-gameplay-spell --lib first_vocation_vitals --quiet`:
  five base vocations, pools/conditions/cooldowns/regeneration, training retained
  during conversion, exact replay, and preservation of a concurrent zero-health state.

Reuse and fixture access:

- Reused `characters::vitals::maxima`, existing `PlayerSpellState` construction,
  build/stance loading, monk initialization, and food-regeneration rules.
- Reused `NpcConversations`, native Oressa dialogue/current-owner checks,
  `vocation_choice_request`, `commit_retained_npc_vocation`, build conversion,
  `CharacterRevisionSequencer`, and the existing receipt reconcile owner.
- Reused `placed_record`, the ordered last-position writer/reader and saved-vitals
  restore. No parallel vitals store, SQL helper, or migration was introduced.
- Reused the existing fresh-admission `Harness`, `Dependencies`, fresh evidence,
  and `death_ml_loss_composed::production_generation_with_pin` fixtures.
- Moved (deleted the original body) `prepare_current_session` to
  `monk_save_postgres_fixture/fresh_session.rs`, returning its actual originating
  request so the admission test does not reconstruct immutable authority.
- Extracted the existing kill-reward scenario thread to
  `kill_reward_postgres_fixture/scenario_thread.rs`; its original test and these
  two scenarios share the same large-stack runner. No fixture body was copied.
- Added test-only private access in
  `apps/game-server/tests/support/character_revision_sequencer_postgres_cases/tutorial_vitals_fixture.rs`
  to reopen the existing harness root, and
  `dispatch/src/gameplay_transport/npc_command/first_vocation_vitals_fixture.rs`
  to retain/restore the real NPC topic for lost-acknowledgement replay. Each is
  mounted once from its existing owner.

Files growing more than 200 lines in this slice:
`dispatch/src/gameplay_transport/first_vocation_vitals_postgres_tests.rs` qualifies
one mechanic's real dispatch, training flush, two receipt-replay states and root
restart. It belongs beside the dispatch owner and reuses the existing PG fixture;
production conversion remains in the spell owner below it. No existing file
grows by more than 150 lines.

### Unfinished / next slice

- Full production-composed Newhaven scenario and its named CI route.
- Classed Corruptor loot and both NPC-dialogue E2E scenarios.
- World pin qualification/refresh by the integrator; never run `pin_check` fixtures
  in this lane under the Git prohibition.

TUTORIAL-1 remains in AUTHORING with no frozen SHA or PR. These exclusions are the
integrator's slice boundary, not new owner questions.

Changed files in this slice (relative to the supplied HEAD):

```text
apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs
apps/game-server/tests/support/character_revision_sequencer_postgres_cases/tutorial_vitals_fixture.rs
crates/gameplay/content/src/content/spell_data/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/first_vocation_vitals.rs
crates/gameplay/dispatch/src/gameplay_transport/first_vocation_vitals_postgres_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/item_look.rs
crates/gameplay/dispatch/src/gameplay_transport/item_use.rs
crates/gameplay/dispatch/src/gameplay_transport/item_use/item_use_postgres_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/kill_reward_postgres_fixture/scenario_thread.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_fixture/fresh_session.rs
crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs
crates/gameplay/dispatch/src/gameplay_transport/npc_command/first_vocation_vitals_fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_character_facts.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_character_facts/tests/vocationless_vitals_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/spell_character_facts/vocationless_vitals.rs
crates/gameplay/dispatch/src/gameplay_transport/tutorial_vitals_postgres_fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/vocation_choice.rs
crates/gameplay/dispatch/src/gameplay_transport/vocationless_vitals_postgres_tests.rs
crates/gameplay/dispatch/src/lifted_tests/durability_world_party_tests.rs
crates/gameplay/progression/src/regeneration.rs
crates/gameplay/spell-rules/src/familiar_cast.rs
crates/gameplay/spell/src/spell/cast.rs
crates/gameplay/spell/src/spell/cast/vocationless_vitals.rs
crates/gameplay/spell/src/spell/mana_training.rs
crates/gameplay/spell/src/spell/mana_training/first_vocation_vitals.rs
crates/gameplay/spell/src/spell/mana_training/tests/first_vocation_vitals_tests.rs
docs/agents/tasks/archive/TUTORIAL-1.md
```

Final slice validation: three mechanic PostgreSQL scenarios passed (one
vocationless death case and two first-vocation cases: wounded/full pools), with
no new ignored tests. Content/progression/spell/spell-rules libraries passed
341/55/222/54 tests; the existing content ignore is unchanged. Dispatch
`spell_character_facts` passed 6 tests and
`gameplay_transport::actor_spell::tests::` passed 19. All-targets clippy
`-D warnings` passed for content, progression, spell-rules, spell, dispatch and
game-server; final dispatch/game-server clippy, rustfmt check and diff check
passed. Governance validation and all 124 governance tests passed. The complete
lane diff was re-read against the owner's CODEX-CHECKLIST; all None/NoVocation
matches, constructors and callers were searched across apps/crates/tools. Every
new production API has its real caller and mechanic dispatch qualification.
