# EVENTS-RAID-1 — scheduled raids: schedule core, firing store and runtime (consolidated task card)

```yaml
task_id: EVENTS-RAID-1
title: "EVENTS-RAID-1: raids (RAID-1 scheduled raid runtime and announcements; RAID-CONTENT-1 inputs)"
mode: IMPLEMENT
status: IN_PROGRESS   # schedule rules and metadata store; runtime activation blocked
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
needs_migration: true   # this pass: 0153_raid_schedule.sql (schedule + firing metadata only)
needs_registry: true   # resource entries 0650-0656; no protocol/event activation
needs_content_regeneration: false   # this pass changes no content; admission remains unfinished
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2177, branch skeleton/events-raid-1

## Goal

Scheduled raids per BOSS_RAID0: a deterministic per-World schedule (random checks and
fixed calendar slots), at most one RUNNING firing per `(WorldId, raid key)`, persistent
raids firing at most once per reset epoch, frozen target channels with per-channel run
rows, restart-safe; then the live runtime (raid actors, announcements) and the
RAID-CONTENT-1 inputs. The draft delivered only a private, data-only schedule/firing
component; production activation was never done.

Children: schedule core + store (this card's first slice), runtime/actors +
announcements, RAID-CONTENT-1 content admission (reference preparation is merged; see dependencies).

## Dependencies (verified against origin/main 65e40c57f, 2026-10-23)

Each original dependency was checked with `git log --oneline origin/main --grep <ID>`
and a bounded `rg` of `crates/`, `apps/` and archived task evidence. Commit-message
mentions alone do not prove that an owner API exists.

| Dependency | Actual state / evidence |
|---|---|
| BOSS_RAID0 acceptance | ACCEPTED: `e2da9805d` (#2209); owning decision `c3fce15f4` (#1407). |
| ENCOUNTER_RT0 acceptance | ACCEPTED: `b65298f2a` (#2108), archived ACCEPT-ENCOUNTER-RT-0 record. |
| ENC-RT-1 encounter runtime / admission (E4) | No generic admitted raid definition/anchor or raid admission API found; boss-specific owners exist under `encounters/src/bosses/`. BLOCKING for runtime. |
| CHAT-1 / CHAT-1b-2a / CHAT-1b-2b | MERGED: `b72d5e722` (#1739), `67cacb76e` (#2093), plus CHAT-WIRE-1 `0ded2eaac` (#1953). No raid occurrence delivery composition. |
| LOOP-MONSTER-1 | MERGED: `778a4b1bc` (#2323). `monster_owner_cycle/respawn_job.rs` drives admitted map sources and native summoning; it cannot create a raid occurrence `(firing_id, wave, entry, index)`. |
| MAP-OVERLAY-1c | BLOCKING: no merged implementation/WorldReset producer found. `0140_daily_reward.sql` already owns `game_world_reset_history` and `daily_reward_claim.rs` consumes it; this pass neither duplicates it nor fabricates epochs from wall time. |
| RAID-CONTENT-1 | Reference preparation is merged in `f93aecf6d` (#2293), with archived record and pinned source reconciliation. Content admission/area bindings are still absent. The former open-draft status was stale. |

### Step 0 inventory

No raid schedule/firing types, tables, dispatch handlers or protocol codes exist in
`crates/` or `apps/` on this main. The existing `boss_encounter.rs` owns boss clocks,
not raid schedules; it is preserved. Existing encounter-rules is the L3 owner, so
no new crate or parallel owner is created. Native `WorldId`, `ProductionKey` and
the simulation-determinism implementation are reused. The archived RAID-CONTENT-1
packet is reference evidence, not runtime admission. `MECHANICS_STATUS.md` does
not exist on main, and `crates/test-support` has no `property` module on main.

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
- **D1160 historical mount:** superseded for this pass by the owner's explicit FLOW-1 auto-mounted library case layout; no standalone PG target or duplicate mount.

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
- PG cases: auto-mounted only at `apps/game-server/src/lifted_tests/auto/raid_schedule_postgres_cases.rs`; migration `apps/game-server/migrations/0153_raid_schedule.sql`. No shared test-target mount.
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

## Delivered in this pass

- L3 `encounter-rules/src/raid/`: bounded definitions, absolute announcement sequence,
  random/fixed calendar policy, 60 s checks, one framed native RNG draw per due
  random check, ordinal advancement even while blocked, no missed-check catch-up,
  per-raid RUNNING guard, persistent once-per-epoch guard, monotonic reset cancellation,
  retirement and checked arithmetic. All facts remain proposed input.
- L4 `durability/raid_schedule.rs` and `raid_schedule/storage.rs`: two normalized
  metadata tables in **0153**, full-snapshot CAS under a named World/raid transaction
  advisory lock, recovery and admission locks first, exact after-state replay,
  typed concurrent loser, rollback-safe writes in the caller's transaction,
  retained history and numeric latest-ordinal load. Column grants cover every write;
  firing facts/terminal status and deletion are guarded. No channel run schema is
  assumed while its producer is blocked.
- Resource limit entries **0650-0656** use the accepted BOSSRAID0 limits, including
  separate wave/announcement dimensions of RL-04. A 1024-byte announcement text cap
  is a DERIVED local metadata bound. MAX_CANDIDATES is declared for the future job;
  this pass does not claim to implement or validate candidate enumeration.
- Auto-mounted real-PG case exercises runtime-role grants, concurrent CAS/replay,
  transaction rollback, independent World state, reload through ordinals 9/10/11,
  whole-expected-snapshot mismatch, stale recovery rejection, persistent epoch
  protection, reset cancellation, retained history and corruption refusal.

### Unfinished

Runtime activation remains BLOCKED by admitted raid content/anchors, WorldReset
production/epoch/calendar resolution, assigned-and-admitting target channels and
raid occurrence spawn identity. The merged map-spawn/summon API is not a raid
spawn API. The World job, <=100-candidate enumeration, frozen targets/channel-run
rows, generation/LOST reconciliation, live actor/announcement deduplication,
BOSS-WIRE and real-dispatch E2E/public-entrypoint production callers remain
unfinished. Do not expose these metadata APIs as a command, scheduler or authority
until the missing owners are composed and independent persistence review passes.
`Close` is only a metadata deadline transition; it does not assert terminal live
runs and cannot authorize delivery or spawning. This pass does not claim full
card acceptance or production readiness. Property tests are skipped because the
requested shared property module does not exist; arithmetic boundary tests run.

## Reference parity

Pinned Canary sources are the same revision as the merged RAID-CONTENT-1 packet:
[C++ rules](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/lua/creature/raids.cpp),
[constants](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/lua/creature/raids.hpp),
[raid index](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/raids/raids.xml).
Read-only source retrieval was refreshed for this pass; sources are evidence only.
Each row has a table-driven row in `raid_reference_parity_rows`.

| Rule / table row | Reference and native value | Test |
|---|---|---|
| Check | `CHECK_RAIDS_INTERVAL = 60` seconds; native 60000 ms. | `raid_reference_parity_rows::Check`; `raid_due_checks_are_once_and_downtime_is_not_caught_up` |
| IntervalUnits | `interval2` is minutes (`Badger = 500` => native mean 30000 s). Native chance **60 / mean seconds**, per accepted BOSS_RAID0 §4.2; exact 64-bit threshold tested. The pinned Canary/Crystal code instead multiplies mean by its random range before dividing by 60, with 32-bit overflow. That conflict is not adopted. No donor probability claim is made. | `raid_reference_parity_rows::IntervalUnits`; `raid_random_draw_is_framed_reproducible_and_bounded` |
| ZeroInterval | Canary rejects missing/zero interval; native random mean 0 is rejected. | `raid_reference_parity_rows::ZeroInterval` |
| Margin | Canary margin is minutes; native min-gap is seconds converted to ms and persists through resets/restarts. Native gap is per raid and from last firing (accepted §4.2), rather than donor global last raid end. | `raid_reference_parity_rows::Margin` |
| Sequence | Canary XML delay is ms, events sorted by delay. Native deadlines are `fired_at + delay_ms`, stable on ties (§4.3). The importer must normalize donor minimum tick (1000 ms); donor inter-event staggering is not native timing authority. | `raid_reference_parity_rows::Sequence` |
| Kinds | Canary warning and event/default map to native Warning / Advance; sequence preserves the admitted kind. Other donor message kinds need content qualification rather than silent reinterpretation. | `raid_reference_parity_rows::Kinds` |
| Persistent | Donor absent `repeat` removes a raid for that process lifetime. Accepted native persistent raids instead fire once per World reset epoch, even after DONE; reset cancels RUNNING and permits a later epoch. | `raid_reference_parity_rows::Persistent` |
| Bounds | Accepted BOSSRAID0 RL-03/RL-04: <=200 creatures, <=32 waves, <=16 announcements. These are native bounds, not inferred Global limits. | `raid_reference_parity_rows::Bounds`; `raid_definition_capacity_and_deadline_boundaries` |

## Validation

Run only this mechanic's PostgreSQL filter on the shared PC:

```sh
cargo fmt --all -- --check
rustfmt --edition 2024 --check apps/game-server/src/lifted_tests/auto/raid_schedule_postgres_cases.rs
cargo clippy --locked -p oteryn-gameplay-encounter-rules -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-encounter-rules --lib --quiet
cargo test --locked -p oteryn-gameplay-durability --lib --quiet raid_schedule
cargo test --locked -p oteryn-game-server --lib --quiet raid_schedule_postgres_cases::
cargo run --locked -p oteryn-architecture-check --quiet -- workspace .
python3 tools/repository/validate_repository_policy.py
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
```

### Local results (2026-10-23)

- Wrapper (`login=false`), toolchain from `rust-toolchain.toml`, locked/offline:
  encounter-rules `--lib`: **8 passed**; game-server
  `--lib raid_schedule_postgres_cases::`: **1 passed**, real PostgreSQL 18.6,
  **8.42 s**. The durability `--lib raid_schedule` filter compiled and passed
  with **0 matching tests**; its store is covered by the auto-mounted server PG case.
- `cargo fmt --all -- --check`: PASS. The generated auto-mount is not recursively
  formatted by rustfmt, so the PG file was also formatted and checked directly
  with the installed toolchain's rustfmt (login shell).
- Architecture boundary checker: PASS. Governance validation: PASS;
  governance Python unit tests: **124 passed**. Repository policy validation:
  PASS. `git diff --check`: PASS.
- `cargo clippy --locked -p oteryn-gameplay-encounter-rules -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings`: PASS through the queued wrapper.
- Whole-diff review against `~/oteryn-cache/CODEX-CHECKLIST.md` and a workspace
  `rg` of every new type's match/constructor/impl sites completed. The remaining
  production-caller/dispatch E2E gap is explicitly recorded under Unfinished;
  it is not claimed as satisfied by the metadata PG test.
- An initial login-shell Cargo invocation bypassed the queued wrapper and had
  no PG route; that route-less result is excluded from PostgreSQL evidence.
  The wrapper run above executed the database case. Existing vendored Tokio/SQLx
  dependency warnings are unchanged.

No commits, pushes, content generation, workflow edits or external writes were made.
