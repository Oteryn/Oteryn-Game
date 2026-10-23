# LOOP-MONSTER-1 — World respawn scheduler; monster ranged, spell and summon AI (consolidated task card)

```yaml
task_id: LOOP-MONSTER-1
title: "LOOP-MONSTER-1: session-independent World respawn scheduler; monster ranged, spell and one-pass summon AI"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 1 LOOP-MONSTER-1 (rows 94, 96)"
brief: "CREATURE-AI-0 (§4.4, §4.6, §7, §8.1; accepted in ACCEPT-BATCH #2209 §1, child gates not waived) and CREATURE-AI-1 (§1.2, §2); BUNDLE-MONSTER-AI-1 record"
class: "hard (child graph)"
base_branch: main
branch: null
pr: 2323
needs_migration: false
needs_registry: false  # existing native limits reused; no new bound
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2323](https://github.com/Oteryn/Oteryn-Game/pull/2323). Session-independent World respawn wake loop and native monster think/summon realization are merged. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/world_respawns.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2200, branch skeleton/loop-monster-1

## Goal

A World-owned respawn scheduler that runs with no session, plus monster ranged, spell
and summon AI on the Channel owner: one native think selects its target and runs the
existing profile schedule once, reporting ability and summon proposals; summons are
physically admitted, registered and replay-safe under the current owner; ordinary
creatures chase, flee, wander, walk back home and leash on the real Map.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| CREATURE-AI-0 decision / acceptance | MERGED (#1431); ACCEPTED (#2209 §1) |
| CREATURE-AI-1 targeting and think | MERGED (#1813; packet #1763) |
| BUNDLE-MONSTER-AI-1, SPAWN-CEILINGS-1 | MERGED (#2036, #1984) |
| MAGIC-CAST-1 | MERGED (#2072, #2112) |
| RANGED-CONTENT-1 PR1 (ranged facts) | MERGED (#2096); LOOP-RANGED-1 runtime: confirm residue at allocation |
| MAP-OVERLAY-1a/1b, MAP-CUTOVER-1b | MERGED (#1800, #1809, #1916) |
| ADMIT-CAP-1 (node/serve startup installation and wakeup, D1239) | MERGED (#2187) |
| CREATURE-MOVE-1a/1b | MERGED (#1977, #1987) |
| CREATURE-MOVE-1 remainder (chase caller; card `docs/agents/tasks/archive/CREATURE-MOVE-1.md`) | BLOCKING — re-implementation pending (draft #2184 closing); the draft composed both, re-implement together or #2184 first |

## Accepted decisions

- **D1123 / D1239a:** family-F assignment; the summon catalog stays source-only — without
  an admitted parent/child closure the result is `None = Unknown` and summons are
  refused; the document schema is not extended.
- **D1239:** persistent owner/admission composition here; `node/serve` startup
  installation and wakeup after #2187 (now merged).
- **D1423:** the World respawn wake loop is installed in `serve_gameplay` as a slot-only hunk.
- **D1329:** real PG cases run on shared PostgreSQL 17.6 (`server_version_num` 170006).
- No second scheduler, RNG, think pass, Movement owner or realization owner; native
  `profile_schedule.rs` owns the bounded attack/defence/summon clock, deterministic
  draw and occurrence replay/conflict; native `monster_summon.rs` owns catalog
  qualification, role/count observations, path/terrain policy and physical
  admission/replay; Foundation owns actor links, identity/generation and fences.

## Design findings

- Legacy `CreatureBehaviourProfile::project` removes summons and `CreatureAiState::think`
  calls `schedule.prepare` without facts; enabling summons there alone would hit
  `SummonClockDependency`, and `ThinkReport` had no summon proposals. The fix is one
  think call to `prepare_with_summons` with independently resolved optional
  `MonsterSummonFacts` for the **selected** target, returning `summon_proposals`
  (empty lists in refusal/over-budget paths), keeping replay/conflict binding and
  budgets. Never call legacy think and an explicit schedule for the same occurrence.
- Fresh changed facts for the same schedule occurrence are a native `OccurrenceConflict`,
  not permission to redraw or create twice.
- **Draft PG failure 1:** AcceptedEntryR1 carried no qualified ground speed, so Movement
  correctly refused `MissingSpeed`; fixture moved to the qualified Thalom source
  (`tools/content-schema/native-gameplay/canary-thalom-world.json`), no fallback speed.
- **Draft PG failure 2:** the outer binder left summon catalogs and nested admission specs
  pinned to the inner artifact (child count 0). Fix: requalify through the existing
  catalog qualifier before mutation; regression
  `activated_outer_source_summon_rejects_inner_catalog_and_registers_once`.
- **Fixture pin:** a synthetic position failed the current pin (speed `None`); use the
  pinned admission helpers for both actors.

## Reference behaviour from the draft (re-implement, do not copy)

- **Opt-in projection** `project_with_summons` (native limits, authored summon entries)
  beside the unchanged legacy projection; `CreatureAiState::prepare_owner_profile`
  checks the supplied scope fence/stamp and live actors and returns the native
  `SchedulePlan`. `prepare_monster_profile` gets path facts from current map policy
  and role/count from `NativeSummonCatalog::facts`; unknown path refuses before the
  clock advances. At most **8** proposals dispatched in authored order.
- **Catalog bounds:** one exact-ref index with duplicate refusal, ≤8 entries, max/count
  in 1..=16, positive intervals, chance ≤1,000,000.
- **Summon owner:** the Channel state retains `NativeSummonOwner`; physical admission,
  installed native policy, AI-table registration, registration-failure rollback and the
  replay memo finish synchronously under the current owner; replay neither readmits nor
  re-registers. Outer activation requalifies the admitted CreatureProfiles closure;
  catalog entries, source specs and nested admission specs share the checked outer
  artifact digest. Retirement/removal/death hooks connect to the same owners.
- **Live caller** holds the Item-owner transaction through current map, Item and
  LocalObject reads and the synchronous consumer; missing/unqualified Item movement
  flags → Unknown/refusal; an old empty-tile read cannot authorise a later transaction.
- **Chase/movement** (with CREATURE-MOVE-1): shared chase search budget and bounded
  search kernel; one retained route/request binds actor and target snapshots, pins,
  revisions and goal; carried requests run before fresh thinks; steps at native source
  speed and destination ground speed via owner deadlines; only `Moved` records cadence;
  each step re-checks its next cell; a retained path used as summon reachability rereads
  its suffix. Home comes from the authored spawn census (not admission position);
  target distance, fleeing, wander/dance, paced walkback and leash relocation use
  current Map and occupancy; intrinsic summons have no wild home. Cancellation during
  an awaited read keeps pending input, route and cadence and does not advance the
  schedule; dead future timer entries are removed. Path queries use a crate-private
  RPITIT reader (no per-tile Box); production boundaries require Send futures.
- **World respawn job** `MonsterRespawnJob::drive(runtime, current, wake, shutdown)`: one
  pass per wake under the runtime mutex, then sleep to the next **50 ms** owner window
  (`next_owner_window`); ends `Shutdown` (checked before each pass, raced against sleep),
  `StaleOwner` (retired/replaced fence) or `ClockExhausted`; a clock regression or carrier
  refusal keeps the last window. `RespawnWake: OwnerClock` adds `sleep_until`;
  `MonotonicRespawnWake::from_origin(ComposedFreshAdmission.clock_origin)` — the same
  origin the session pass uses for `drive_owner_spawns`. `ChannelRuntimeV1::owner_fence`
  checked under the runtime lock every pass. Installed as `run_world_respawns` in
  `serve_gameplay`, polled with the other seam cadences.
- **Tests kept as requirements:** opt-in/legacy projection; 8/9 entries and 16/17 bounds;
  due/retry/conflicting-path clocks; physical creation and replay; live count caps;
  unknown/blocked path; foreign scope; replaced ownership generation; stale actor;
  no free tile; owner-timer cancellation/dead entries; Movement rejects a non-pinned
  context before Map; shared-search distance and home route; respawn with no session at
  its due window, retired owner, shutdown before any pass, exhausted clock, installed job
  on an origin past its deadline (session pass then advances nothing).
- **Real PG cases** (`bestiary_postgres_harness::configured_admin`; unconfigured = SKIP,
  never PASS; harness cleanup awaited after `catch_unwind`, then the panic resumes):
  `current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope` (actual
  Item/Map reads, shared-budget deferral, SQL cancellation keeps the timer, retry moves
  without another think, one-pass registration/replay, wrong TX refusal, real Item MINT
  → Unknown) and `current_owner_spawn_home_walkback_and_leash_use_actual_source_cell`
  (Thalom cells (2,12,-7)/(3,12,-7)/(4,12,-7) ground speeds 150/150/110; home (4,12,-7)
  ≠ admission (2,12,-7); two paced steps; free-home leash relocation, occupied-home refusal).

No RL17 runtime/p99 result was measured.

## Target layout (ADR-0022)

- Think/schedule/projection: `crates/gameplay/monster-ai/src/ai_think.rs` (1,230 lines:
  call-site only), `ai_think/behaviour_profile.rs`, `ai_think/channel_owner.rs`,
  `ai_think/profile_schedule.rs`; new summon-planning logic in
  `crates/gameplay/monster-ai/src/ai_think/summon_plan.rs` (new) with
  `summon_plan_tests.rs` (new).
- Summon owner: `crates/gameplay/monster-ai/src/monster_summon.rs` (2,072 lines: no growth
  beyond ~150; split new owner/registration code into `monster_summon/owner.rs` (new)).
- Respawn job: `crates/gameplay/monster-ai/src/monster_owner_cycle.rs` (exists) or
  `monster_owner_cycle/respawn_job.rs` (new) with tests in a child file.
- Timer/schedule: `crates/gameplay/foundation/src/foundation/owner_timer.rs`,
  `foundation/profile_schedule.rs`; carrier `crates/gameplay/entities/src/carrier.rs`.
- Movement: `crates/gameplay/world/src/movement.rs`, `movement/path_search.rs`,
  `movement/speed.rs`; chase `crates/gameplay/combat/src/combat/attack/chase.rs`.
- Dispatch: `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs`
  (1,297 lines: call-site only; PG cases in a new child test file),
  `gameplay_transport/actor_spell.rs`, `gameplay_transport/attack.rs`.
- Content: `crates/gameplay/content/src/content/native_gameplay.rs`.
- Startup: `apps/game-server/src/node/serve.rs` (`serve_gameplay` hunk, wiring-only).
- PG harness: `apps/game-server/tests/support/bestiary_postgres_harness.rs` (exists).

## Acceptance criteria

- [x] Respawn continues with no session; deterministic schedule; ends on shutdown/stale owner.
- [x] One think pass per creature reports ability and summon proposals; no second schedule or RNG.
- [x] Summons admitted, registered and replayed once under the current owner; rollback on registration failure; Unknown closure refuses.
- [x] Chase/flee/wander/walkback/leash on the current Map at native speeds.
- [x] Both real-PG cases pass on configured PostgreSQL 17.6 (no skip counted).
- [x] Every concurrent transition serialized and the losing writer typed.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests live in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines (`ai_think.rs`, `monster_summon.rs`, `monster_ai_cycle.rs`).

## Validation

Owner D1466 scope overrides the original workspace/full-PG validation list. All cargo
commands used `/home/mole/oteryn-cache/codex-bin/cargo`; no dependencies downloaded.

```sh
cargo fmt --all -- --check  # pass
cargo clippy --locked --offline -p oteryn-gameplay-content -p oteryn-gameplay-entities -p oteryn-gameplay-world -p oteryn-gameplay-monster-ai -p oteryn-gameplay-dispatch --all-targets -- -D warnings  # pass
cargo clippy --locked --offline -p oteryn-game-server --lib --test guild_postgres --quiet -- -D warnings  # pass (compile only)
cargo test --locked -p oteryn-gameplay-monster-ai --lib --quiet  # pass: 92; 3 existing ignored
cargo test --locked -p oteryn-gameplay-world --lib movement::path_search --quiet  # pass: 17
cargo test --locked -p oteryn-gameplay-entities --features test-support --lib native_summon --quiet  # pass: 3
cargo test --locked -p oteryn-gameplay-content --lib caller_artifact_activates_full_book_under_outer_pin_without_changing_baseline --quiet  # pass: 1
cargo test --locked -p oteryn-gameplay-dispatch --lib world_respawns --quiet  # pass: 5
cargo test --locked -p oteryn-gameplay-dispatch --lib monster_ai_cycle --quiet  # pass: 12
cargo test --locked -p oteryn-gameplay-dispatch --lib loop_monster:: -- --test-threads=1 --nocapture  # first run: 3 pass; summon case repaired and rerun below
cargo test --locked --offline -p oteryn-gameplay-dispatch --lib current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope -- --test-threads=1 --nocapture  # pass: 1
cargo test --locked -p oteryn-gameplay-dispatch --lib creature_movement_dispatch_steps_between_thinks_on_postgres -- --test-threads=1 --nocapture  # pass: 1
python3 tools/agents/validate_governance.py  # pass
python3 -m unittest discover -s tools/agents/tests  # pass: 124
git diff --check  # pass
```

All four new PG cases actually ran with `server_version_num=170006`, zero skips:
`current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope`,
`current_owner_spawn_home_walkback_and_leash_use_actual_source_cell`,
`activated_outer_source_summon_rejects_inner_catalog_and_registers_once`, and
`current_owner_summon_registration_failure_rolls_back_and_replays_the_refusal`.
The group run passed the latter three; the corrected first case passed alone.
The combined setup now returns the parent from its completed budget walkback to the
pinned admission cell through existing Movement, preserving its schedule/occurrence.
Replay assertions bind the same occurrence rather than assuming SQL finishes within a think interval.
SQL cancellation/retry lives in the required home case; the summon case covers shared-budget retry.

## Planned module layout (recorded before implementation)

- monster-ai: `ai_think/summon_plan.rs`, projection/native-profile children, current-Map
  movement adapter, `monster_summon/{catalog,physical_admission,owner}.rs`, respawn window job.
- world: `movement/monster_map.rs` plus one resumable path kernel/current reader.
- entities: `carrier/monster_summon_admission.rs`, extracted physical admission/retirement.
- dispatch: `monster_profile_cycle.rs`, `world_respawns.rs`, dedicated child tests.
- content: extracted native activation fixture and source-profile test variants.

## Prerequisite audit — 2026-10-22

Baseline is detached HEAD `794fe3938fefb9df37d501e8083b4f8991a847f2` (#2305).
CREATURE-MOVE-1 goals/search budget/spawn home/step timers are present. Missing live
prerequisites were the admitted CreatureProfiles summon-catalog adapter, current
transaction Map/Item adapter, synchronous registration composition and independent wake loop.
No owner-only policy question arose. Existing production native ability bodies produce
proposals; absent executable bodies preserve the legacy targeting/melee projection.
No document schema or new spell realization capability was introduced.

## Integrator decision 2026-10-22: prerequisite implemented in this lane

Option B: `monster_summon/catalog.rs` reuses its original qualifier with one exact-ref index
and outer-pin requalification of nested specs; `movement/monster_map.rs` reuses existing
Durability scope/physical-transaction/native-world reads. `monster_summon/owner.rs` completes
admission/policy/AI registration/replay or rollback synchronously. The production
`monster_profile_cycle.rs` holds the transaction through consumption. Movement shares its
original goal adoption/route install/step timer and search kernel, including charged cancellation.
`monster_owner_cycle/respawn_job.rs` owns windows; `world_respawns.rs` composes wake/mutex/shutdown.
Re-located `serve_gameplay` is in dispatch `gameplay_transport/mod.rs`; its edit is slot wiring.

## File sizes and layer justification

Every existing code file grew by at most 34 lines; large owners shrink after extraction.
New files growing by more than 200 lines (all other new files are at most 198):

| File (under crates/gameplay/) | Lines | Why this layer |
|---|---:|---|
| monster-ai/src/ai_think/summon_plan.rs | 385 | Extracted targeting plus one native scheduling pass belongs to AI. |
| monster-ai/src/ai_think/creature_steps/creature_current_map.rs | 216 | Async current-Map adapter feeds the existing AI movement request/timer owner. |
| monster-ai/src/monster_summon/catalog.rs | 215 | Extracted qualifier and admitted summon closure belong to native summon AI. |
| entities/src/carrier/monster_summon_admission.rs | 240 | Extracted physical slots/links and current-fence wrappers belong to Entities. |
| world/src/movement/monster_map.rs | 295 | Current terrain/Item/LocalObject facts and Movement decisions belong to World. |
| dispatch/src/gameplay_transport/postgres_tests/loop_monster/summon_transaction.rs | 305 | Real composed PG admission/replay/rollback tests belong at dispatch. |

Shared slots: gameplay_transport/mod.rs (+7 wiring), actor_spell.rs (+2 owner field/default),
postgres_tests/loop_monster.rs (auto-mounted by the FLOW build.rs; no shared mount line), movement.rs (+1 module registration).
Other large owners contain extraction/call-site changes; no new logic bodies in slot files.

## Reused code and test-only seams

- Native profile schedule, deterministic draws, occurrence conflict/replay and existing limits.
- Existing behavior projection; existing summon qualifier, physical placement, memo and retirement.
- Foundation owner fence/stamps, actor links/slots, Movement snapshots and decision commits.
- CREATURE-MOVE-1 goal adoption, route install, step cadence/deadline lane, path budget,
  A* labels/order/bounds and occupancy census; one shared kernel, no duplicate implementation.
- Durability SpellItemScopeAuthority, scoped Item locks/reads, physical-XID proof,
  initialized native tile/static WorldObject qualification and semantic-pass deadlines.
- Existing spawn carrier/visibility and authored table_activation_spawns census.
- Bestiary configured-admin/runtime and revision Harness cleanup; Dependencies::new(),
  Dependencies::initialize()/owner(), prepare_current_session(), existing Item MINT API.
- Existing native activation helper extracted (deleted from parent) and re-exported;
  manifest/Thalom source and an existing source Item ranged Spell body reused.
- Existing summon_fixture and Channel movement harness reused by child unit tests.
- Small cfg(test/test-support) child wrappers: ai_think/summon_plan_test_support.rs exposes
  timer/report observations; monster_summon/catalog_test_support.rs changes only the pin;
  movement/monster_map/transaction_test_support.rs borrows the actual retained transaction.
  No copied harness/fixture body, duplicate mount, test weakening or new PG target.

## Changed files

```text
crates/gameplay/content/src/content/native_gameplay.rs
crates/gameplay/content/src/content/native_gameplay/test_support/activation_fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/item_scope.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/shared_budget.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/sql_cancellation.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/summon_transaction.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs
crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle.rs
crates/gameplay/dispatch/src/gameplay_transport/world_respawns.rs
crates/gameplay/dispatch/src/gameplay_transport/world_respawns/world_respawns_tests.rs
crates/gameplay/entities/src/carrier.rs
crates/gameplay/entities/src/carrier/monster_summon_admission.rs
crates/gameplay/monster-ai/src/ai_think.rs
crates/gameplay/monster-ai/src/ai_think/behaviour_profile.rs
crates/gameplay/monster-ai/src/ai_think/behaviour_profile/summon_projection.rs
crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests.rs
crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/creature_snapshot_tests.rs
crates/gameplay/monster-ai/src/ai_think/creature_steps.rs
crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_current_map.rs
crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_goal_adoption.rs
crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_paths.rs
crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_step_choice.rs
crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_step_timer.rs
crates/gameplay/monster-ai/src/ai_think/native_profile.rs
crates/gameplay/monster-ai/src/ai_think/profile_schedule/tests.rs
crates/gameplay/monster-ai/src/ai_think/profile_schedule/tests/summon_plan_tests.rs
crates/gameplay/monster-ai/src/ai_think/summon_plan.rs
crates/gameplay/monster-ai/src/ai_think/summon_plan/summon_path.rs
crates/gameplay/monster-ai/src/ai_think/summon_plan_test_support.rs
crates/gameplay/monster-ai/src/monster_owner_cycle.rs
crates/gameplay/monster-ai/src/monster_owner_cycle/respawn_job.rs
crates/gameplay/monster-ai/src/monster_summon.rs
crates/gameplay/monster-ai/src/monster_summon/catalog.rs
crates/gameplay/monster-ai/src/monster_summon/catalog_test_support.rs
crates/gameplay/monster-ai/src/monster_summon/owner.rs
crates/gameplay/monster-ai/src/monster_summon/physical_admission.rs
crates/gameplay/world/src/movement.rs
crates/gameplay/world/src/movement/monster_map.rs
crates/gameplay/world/src/movement/monster_map/transaction_test_support.rs
crates/gameplay/world/src/movement/path_search.rs
crates/gameplay/world/src/movement/path_search/current_path.rs
crates/gameplay/world/src/movement/path_search/current_path/current_path_tests.rs
crates/gameplay/world/src/movement/path_search/path_kernel.rs
crates/gameplay/world/src/movement/path_search/path_labels.rs
crates/gameplay/world/src/movement/path_search/path_occupancy.rs
docs/agents/tasks/archive/LOOP-MONSTER-1.md
```

## Review and unfinished

Whole diff re-read against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; all new
constructor/field/trait/variant and caller sites searched under apps/, crates/, tools/.
Each new production API has a live caller and real-dispatch coverage; test seams are cfg-gated.
No task-scope implementation remains. No RL17 runtime/p99 measurement. No CI-only failure
was deferred. Integrator retains independent review, freeze, CI/MQ and publication;
no commit/push/branch mutation was performed, no frozen candidate or PR created here.
Pending owner questions: none. Next step: integrator review of this local candidate.

## FLOW port — 2026-10-23

Ported onto `claude/happy-davinci-ijz3wy-6` (`49cdeaf5f`, main + FLOW-1/2). The dispatch PG
suite moved to the auto-mounted `gameplay_transport/postgres_tests/loop_monster.rs` (children in
`postgres_tests/loop_monster/`), so `monk_save_postgres_tests.rs` is unchanged. Per-session owner
fields are read through `ComposedFreshAdmission::states` (DISPATCH-WIRING-1). The lane's
CREATUREAI0 resource-limit edits already exist on the base as per-entry files 0496–0499 with
equal content; the aggregate registry was not restored. No migration and no ID renumbering.
