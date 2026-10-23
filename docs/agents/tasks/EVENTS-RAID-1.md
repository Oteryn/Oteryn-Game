# EVENTS-RAID-1 — scheduled raids: schedule core, firing store and runtime (consolidated task card)

```yaml
task_id: EVENTS-RAID-1
title: "EVENTS-RAID-1: raids (RAID-1 scheduled raid runtime and announcements; RAID-CONTENT-1 inputs)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 4 row EVENTS-RAID-1 (row 102); App. 7 MONSTER-META-1, EVENTS-RAID-1, EVENTS-WORLD-1, ARENA-1; §5 Wave 5"
brief: "docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md (§4, §4.1, §4.2, §12, §14, §17; BOSSRAID0-RL-02)"
class: "hard (child graph), persistence review"
base_branch: main
branch: null
pr: null
needs_migration: true   # three raid tables; next free number at allocation time (only one migration PR in the queue)
needs_registry: maybe   # GAME_EVENT_FOUNDATION_REGISTRY rows for announcements
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Boss clocks exist, but no raid scheduler/firing writer from this card is retained on main. Evidence: `crates/gameplay/durability/src/durability/boss_encounter.rs`.

Remaining: Implement deterministic raid schedule, durable firing/deduplication and native spawn/announcement composition. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2177, branch skeleton/events-raid-1

## Goal

Scheduled raids per BOSS_RAID0: a deterministic per-World schedule (random checks and
fixed calendar slots), at most one RUNNING firing per `(WorldId, raid key)`, persistent
raids firing at most once per reset epoch, frozen target channels with per-channel run
rows, restart-safe; then the live runtime (raid actors, announcements) and the
RAID-CONTENT-1 inputs. The draft delivered only a private, data-only schedule/firing
component; production activation was never done.

Children: schedule core + store (this card's first slice), runtime/actors +
announcements, RAID-CONTENT-1 content population (open content draft #2243).

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| BOSS_RAID0 acceptance | ACCEPTED (#2209 ACCEPT-BATCH-2026-10-15; amended by EVENTS-WORLD-1 #2178) |
| ENCOUNTER_RT0 acceptance | ACCEPTED (#2108) |
| ENC-RT-1 encounter runtime / admission (E4) | not found on main — confirm owner at allocation |
| CHAT-1 / CHAT-1b-2a / CHAT-1b-2b (announcements) | MERGED (#1739, #2093; CHAT-1b-1 wire #1425) |
| LOOP-MONSTER-1 (raid monsters/summon catalog) | BLOCKING — re-implementation pending (draft #2200 closing) |
| MAP-OVERLAY-1c (WorldReset epoch, current-channel port) | BLOCKING — re-implementation pending (draft #2167 closing) |
| RAID-CONTENT-1 content | open draft #2243 (not a consolidation draft) |

## Accepted decisions

- **BOSS_RAID0 §4.1:** fixed rules by weekday or month-day with an offset after reset;
  **§4.2:** once-only firing, missed checks, at most one RUNNING firing **per raid**
  (the skeleton's broader one-per-World checkbox is *not* a new restriction).
- Random checks every **60 s**, one native deterministic RNG draw bound to framed
  `(WorldId, raid key, check ordinal)`; a due check advances the ordinal even when
  blocked; next check from recorded now; no downtime catch-up loop. Fixed slots older
  than the current check interval are missed, not caught up.
- Bounds (accepted): ≤100 candidate rows per World-job pass; ≤200 creatures, ≤32 waves,
  ≤16 announcements per raid.
- **ADR-0021 §3:** reset schedule is operational; calendar timezone, current reset and
  content authority are supplied by their real owners (no UTC/DST mapping invented).
- **HOUSE_OWN0 §9 / BOSS_RAID0 §12:** World jobs use the real recovery fence and
  admission-relation locks, without a Character session fence.
- **D1127:** content source inputs and regeneration by one author in the CP slot.
- **D1160:** the PG case lives under the canonical `character_authority_postgres.rs`
  root (PG-COVERAGE-1 thin-wrapper rule, #1352); no new standalone PG target.

## Design findings (cross_domain, CP seams)

- `assert_recovery_fence` and `lock_admission_relations` are durability-private; the
  production wrapper must call them before occurrence/World keys (keep visibility).
- The native assignment writer is Channel/House-scoped, not a WorldReset/raid producer.
- `world_clock.rs` supplies Tibian light/time, not a durable WorldReset epoch.
- Carrier `SpawnSourceId(u16)` is content-spawn identity, not the accepted raid
  occurrence `(firing_id, wave, entry, index)` — a new actor source is needed.
- Nothing on main establishes admitted raid content or the assigned-and-admitting
  channel set at fire time; these are mandatory integration seams.

## Review findings and dispositions

- Loader sorted `ordinal::text` lexically (9 outranked 10). Accepted and fixed with
  `ORDER BY game_world_raid_firings.ordinal DESC`; real-PG regression creates/reloads
  firings through ordinals 9/10/11 — keep this test.
- A controlled mutant omitting the expected-snapshot comparison failed the
  concurrent-loser test — keep that assertion as the CAS guard.
- A full-package RED (`progression_initialization_unavailable`) was seen on the draft
  and never classified; recheck, do not assume independence.

## Reference behaviour from the draft (re-implement, do not copy)

- **Pure transition aggregate** over one `(WorldId, raid ProductionKey)` schedule, its
  latest firing and exact target-channel/run rows, reusing native WorldId, ChannelId,
  ScopeOwnershipGeneration, ProductionKey and the simulation-determinism crate. Inputs
  are proposed facts, never authority. Private schedule revision (not CharacterRevision);
  check/firing ordinals preserved across reset; min-gap preserved through restart;
  retired rows retained.
- **Calendar conversion:** typed selector, nonnegative ms offset, caller-resolved reset
  timestamp/weekday/month-day; mismatch = explicit absent slot that consumes the due
  check without a synthetic slot. `check_fixed_calendar` rejects Random policy and a
  simultaneous raw slot; raw `check` rejects a missing fixed slot.
- **Firing rules:** persistent firings block another firing in the same epoch even after
  DONE; targets frozen (late channels cannot join); same-generation claim retry never
  creates another run; a newer generation marks an older claimed run LOST and never
  re-runs it. Close requires terminal rows for all targets or the deadline (missing →
  LOST); reset cancels RUNNING. Target membership is separate from run-row presence.
- **Store:** three tables keyed by `(WorldId, raid key)`; firings add ordinal + UUIDv7;
  runs add ChannelId. Strict bounded payloads validated on decode. CAS takes a per-key
  named transaction advisory lock, compares the **whole** expected snapshot, writes in
  the caller's transaction, never commits; exact after-state retry → `Replayed`,
  obsolete/different → `CompareFailed`. Loads only the latest firing and its runs.
  No outcome is a realization, announcement, owner or commit token.
- Draft evidence: 20 pure + 1 PG case (PG 17.6) incl. rollback, replay, concurrent
  loser, reload, World isolation, retained history, corruption refusal.

## Remaining production work (none done in the draft)

1. Semantic transaction/deadline wrapper with recovery check + admission locks first,
   bounded 100-row candidate enumeration in deterministic key order.
2. Resolve current WorldReset epoch, admitted raid definition/bundle, recorded time and
   calendar slot from real owners; validate waves/creatures/anchors and the bounds.
3. Target channels from current assigned-and-admitting scopes under admission locks;
   claims via scope-assignment high-water, incarnation possession and owner generation.
4. Raid actor sources `(firing_id, wave, entry, index)` and announcement occurrences
   under the live owner; same-generation retry reconciles without repeating; ambiguous
   commit reconciles metadata and live occurrence state first; reset actor removal stays
   with the overlay/reset owner.
5. Migration/grants/immutability guards, registry rows, content population/regeneration/
   pins, CHAT/BOSS-WIRE and encounter runtime binding.

## Target layout (ADR-0022)

- Pure schedule/firing aggregate and calendar: `crates/gameplay/encounter-rules/src/raid/`
  (new: `mod.rs`, `schedule.rs`, `calendar.rs`, tests in `schedule_tests.rs`,
  `calendar_tests.rs`); registration in `crates/gameplay/encounter-rules/src/lib.rs` (wiring-only).
- SQL store: `crates/gameplay/durability/src/durability/raid_schedule.rs` (new), using
  existing `runtime_scope_assignment.rs` seams.
- Runtime/actors/announcements: `crates/gameplay/encounters/src/raids/` (new); chat via
  `crates/gameplay/social/src/chat/`; summons via `crates/gameplay/monster-ai/src/monster_summon.rs`;
  clock `crates/gameplay/world/src/world_clock.rs`.
- PG cases: `apps/game-server/tests/support/raid_postgres_cases.rs` (new) registered in
  `apps/game-server/tests/character_authority_postgres.rs`; migration under
  `apps/game-server/migrations/`.
- Content: `content/encounters/raids/` (index exists).

## Acceptance criteria

- [ ] Raid schedule deterministic per World epoch (60 s checks, framed RNG, no catch-up).
- [ ] At most one RUNNING firing per (WorldId, raid key); persistent raids at most once per reset epoch; restart-safe (BOSSRAID0-RL-02).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Metadata CAS never authorises spawn/announcement; runtime occurrences deduplicated.
- [ ] Every BLOCKING dependency merged before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-encounter-rules raid
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test character_authority_postgres raid_postgres_cases::
python3 tools/content-migration/regenerate_content.py && git diff --exit-code
python3 tools/agents/validate_governance.py
```
