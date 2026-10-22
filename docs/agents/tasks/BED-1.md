# BED-1 — sleeper table, bed USE/sleep, owner wake and offline settlement (consolidated task card)

```yaml
task_id: BED-1
title: "BED-1: sleeper table, bed USE and sleep, owner wake, offline settlement"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
wave: 5
class: "hard (persistence, session-generation fencing)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4, §5 (Wave 5), §7 BED-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md   # §4-§8
  - docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true   # only if a protocol registry entry is confirmed by CP
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2147, branch skeleton/bed-1

## Goal

Sleeper table, bed USE and sleep, owner wake and offline settlement per BED0 §4-§8. The
sleep USE ends the session through the fenced logout path; settlement runs inside the real
OFFLINE admission transaction.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| BED0 acceptance | ACCEPTED (ACCEPT_BATCH §1, owner item 5, #2209) |
| BED-CONTENT-1 (bed group facts, `item_bed_promotion.rs`) | MERGED (#1861) |
| WORLDINT-WIRE-1 (USE map_item arm) | MERGED (#2094) |
| HOUSE-RUNTIME-1a | MERGED (#2163) |
| LOOP-VITALS-1 (food/vitals/soul, #2196) | MERGED (#2196) as skeleton; the durable food/HP/mana residual writer was taken out of it — the same-TX food participant BED needs is still missing (`docs/agents/tasks/LOOP-VITALS-1.md`) |
| OFFLINE-1 (logout/offline_settlement build markers, activation, receipt chain; #2174) | NOT MERGED — **BLOCKING** |
| STATUE-1 (shared offline activation path) | NOT MERGED — **BLOCKING** |
| HOUSE-RUNTIME-1b | BLOCKING — re-implementation pending (draft #2165 closing) |
| HOUSE-RUNTIME-1c | BLOCKING — re-implementation pending (draft #2166 closing) |
| MAP-OVERLAY-1c (active-bundle House placement/currentness, head/foot pair) | BLOCKING — re-implementation pending (draft #2167 closing) |
| HOUSE-1b (property/disposition writer) | BLOCKING — re-implementation pending (draft #2141 closing) |
| HOUSE-1a (property/auction owner) | BLOCKING — re-implementation pending (draft #2137 closing) |
| HOUSE-VIEW-1 | NOT MERGED — **BLOCKING** |

Integration order from D1162: HOUSE-RUNTIME-1a (#2163) → HOUSE-1b → HOUSE-1a → BED-1.
Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/housing/src/house_bed.rs` (new, L3 pure rules: stored bed key, freed causes, sleep-duration cap)
- `crates/gameplay/housing/src/house_bed_tests.rs` (new)
- `crates/gameplay/durability/src/durability/bed_sleeper.rs` (new, sleeper storage, caller-TX read/lock, free/delete steps)
- `crates/gameplay/durability/src/durability/bed_sleeper_tests.rs` (new)
- `apps/game-server/tests/bed_sleeper_postgres.rs` (new)
- `docs/agents/tasks/BED-1.md`

Slot-only (final commit at the queue turn): the migration (number: next free number at
allocation time (only one migration PR in the queue)); module registration in
`crates/gameplay/housing/src/lib.rs`, `crates/gameplay/durability/src/durability/core_modules.rs`
/ `mod.rs`; the bed USE arm in `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs` /
`connection.rs`; `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`,
`GAME_EVENT_FOUNDATION_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json`,
`OTERYN_GAME_ERROR_CODE_REGISTRY.json` only if CP confirms an entry; `capabilities.rs`
wiring (no new capability offered until CP decides).

## Wiring points (verified on origin/main)

- `crates/gameplay/content/src/content/item_bed_promotion.rs` — BED-CONTENT-1 authoring facts (`ReferenceItemBed`); not proof of an active head/foot pair
- `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs` (619 lines) — USE map_item arm (WORLDINT-WIRE-1)
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` command chain `else if command.command_type == COMMAND_TYPE_...`; `capabilities.rs` offered sets
- `crates/gameplay/durability/src/durability/character_build.rs`, `crates/gameplay/domain/src/domain/character_build.rs` — `BuildCause` admits only `Training`, `VocationChoice`, `Promotion`; OFFLINE-1 adds logout/offline markers (D1381 moves the native marker writer here)
- `crates/gameplay/durability/src/durability/fresh_admission.rs` (`FreshAdmissionStore::release`) — terminal release, final position and lifecycle receipt; no sleeper hook yet
- `crates/gameplay/durability/src/durability/house_spell_acl.rs`, `crates/gameplay/housing/src/house_spell_acl.rs` — native ownership/ACL reads (`game_house_ownership`, `house_access_for_character`)
- Native `HouseId` (World + revision-free house content key) and `LogicalCell` — the bed identity; never a generated bed UUID or a Channel-local identity

## Accepted decisions carried from the draft

- **D1171:** OFFLINE/logout marker/activation belong to #2174 (OFFLINE-1); durable food/soul to
  #2196 (LOOP-VITALS-1); the caller-TX sleeper step and the BED-CONTENT-1 head/foot leaf to
  BED-1. A `map/facts.rs` accessor follows #2156.
- **D1370 / D1381:** the real OFFLINE receipt/build extension, chain guards and clock/predecessor
  source belong to #2174; their migration numbers are void — this card makes no number choice.
- **D1162:** registration order HOUSE-RUNTIME-1a → HOUSE-1b → HOUSE-1a → BED-1.
- **D1122:** the draft checkpoint was private storage only (NOT_FULLY_QUALIFIED); no runtime authority.
- Mandatory CP ordering: a disposition or other specific-cause free runs **before** the native
  ownership UPDATE in the same transaction; otherwise the generic owner-change fallback records
  `owner_change` first and the one-way guard refuses the real cause.
- No caller boolean, raw admin SQL, stored coordinate, separate bed training backend or new
  authority trait replaces a missing owner seam.

## Reference behaviour built by the draft (re-implement, do not copy)

- Sleeper row: native property/Character FKs, Character-to-World binding, one row per Character
  (including freed rows), one open sleeper per native bed key, nonzero u64 lease and revision,
  native `appearance::Sex` variant, four closed freed causes, paired timestamp-ordered free fields,
  immutable sleep identity and a one-way occupied→freed transition (no overwrite/reopen).
- Units: `slept_at`/`freed_at` are Unix **milliseconds** (as native `character_build`
  `committed_at`); the Logout anchor binds `slept_at == committed_at_us / 1000`. Second
  conversion belongs to the OFFLINE owner.
- Owner-change fallback: `AFTER UPDATE OF owner_character_id` on the native ownership table frees
  only open rows with `owner_change` at `clock_timestamp()` after taking the house revision row
  lock; same-owner revisions do not wake; first time/cause is preserved. Access revocation is not
  a wake cause.
- Insert/free/delete advance the house occupancy revision in the same transaction; rollback and
  rejected writes keep the prior state; revision overflow fails the mutation.
- Read: one snapshot including the house revision; the caller-TX variant takes `FOR UPDATE OF s`
  and neither commits nor frees/deletes/awards. Missing revision, bad lease/identity/variant/time
  or free pairing fail closed.
- PG test harness errors (never skips) on a missing or non-Unicode
  `OTERYN_TEST_POSTGRES_ADMIN_URL` and checks `server_version_num=170006`.
- Draft evidence (historical): 13 authored cases (1 pure, 12 PG) including competing free
  (`55P03` loser, zero-row retry) and competing inserts (`23505`, rollback isolation); current
  generation compilation, Clippy and PG execution NOT_RUN.

## Findings and open questions

Integration still owned elsewhere and required before activation:
1. Active-map head/foot resolver: USE, choice, reservation, insert, view and bed-removed checks
   share the real current bundle's reciprocal same-house resolver (MAP-OVERLAY-1c); stored
   coordinates are not proof.
2. Property/ACL currentness: the property row lacks BED0's OWNED / MOVE_OUT_PENDING / content
   fence; the writer takes property `FOR SHARE` plus granting guild/membership locks and
   revalidates session/lease/scope/Premium/block/reach. Owner-wake/bed-removed are
   house-runtime-only.
3. Logout bundle (with OFFLINE-1/STATUE-1): first-receipt replay before mutations; marker
   timestamp/lease/activation and native sex at insert; cleanup of the Character's older row;
   qualified head insert. A bed unique conflict commits the marker without sleeper or activation.
   Multi-house revision locking includes the old row's house.
4. Admission bundle: the OFFLINE settlement transaction locks the row; only the matching previous
   lease plus its latest logout receipt count; any row (including stale/freed) is deleted with a
   revision advance. `sleep_s = min(d, freed_duration_s if freed else d)`, cap 21 days; training
   still runs over OFFLINE0's `d` after wake. No food consumed and no regeneration coefficients
   promoted until DUR-02 durable vitals (BED-REGEN stays separate).
5. The slot installs schema + permissions + marker provenance guards + the real writer/hooks
   inseparably.

## Acceptance criteria

- [ ] Sleep ends the session through the fenced logout path.
- [ ] Owner wake and settlement are idempotent across restart.
- [ ] Only the house ACL may use a bed.
- [ ] Specific-cause free precedes the ownership UPDATE in the same transaction.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (pure rules in `housing`, storage in `durability`, USE arm in `dispatch`).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`connection.rs` is already over).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing house_bed
cargo test -p oteryn-gameplay-durability bed_sleeper
cargo test -p oteryn-game-server --test bed_sleeper_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md` §4-§8
- `docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md`
- `docs/agents/evidence/OTV2-20261005-bed-facts-v1.json`
- `docs/agents/tasks/archive/OTV2-20261005-item-sem-bed-1-bed-group.md`
- CP coordination: #2131 (D1122, D1162, D1171, D1370, D1381)
