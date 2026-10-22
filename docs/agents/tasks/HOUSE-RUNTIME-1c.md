# HOUSE-RUNTIME-1c — house position persistence (consolidated task card)

```yaml
task_id: HOUSE-RUNTIME-1c
title: "HOUSE-RUNTIME-1c: house position persistence"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
class: "hard (persistence, session-generation fencing, admission/terminal transactions)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row HOUSE-RUNTIME-1c; §7 'HOUSE-RUNTIME-1a/1b/1c'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md   # §2.3
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: false   # the migrations directory is the only registry (D1347)
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2166, branch skeleton/house-runtime-1c

## Goal

A Character's position inside a House survives logout/restart through two nullable House
columns on the existing last-position row (CHAR-POSITION-1). Every exit and every rejected
saved-house login commits an outside row in the real handoff/admission transaction. Third
of HOUSE-RUNTIME-1a → 1b → 1c.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| CHAR-POSITION-1 (migrations 0080/0084, `character_last_position.rs`) | MERGED (#2006) |
| HOUSE-RUNTIME-1a (interior, migration 0095) | MERGED (#2163) |
| ADMIT-CAP-1 | MERGED (#2187) |
| SCOPE-HANDOFF-1 (migration 0074, `house_scope_handoff.rs`) | on main |
| HOUSE-RUNTIME-1b (House scope runtime, sealed claim / scope-transition authority per D1362, actor placement) | BLOCKING — re-implementation pending (draft #2165 closing) |
| MAP-OVERLAY-1c (qualified House catalogue: entrance mapping, retained entrance for removed Houses) | BLOCKING for exit/fallback activation — re-implementation pending (draft #2167 closing) |
| HOUSE-1a / HOUSE-ACL-1 / HOUSE-1b (`game_house_access` body, grants, disposition) | BLOCKING for real access re-check at login — re-implementation pending (drafts #2137, #2140, #2141 closing) |
| GM-1 (#2181) | open draft; recorded only as shared-file order `#2181 → #2166 → #2187`, no functional dependency — re-check the order at the slot |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/durability/src/durability/character_position_house.rs` (new: row-field adapter `decode_house_fields` and the House-branch consumer support only; no parallel loader/writer/sequence)
- `crates/gameplay/durability/src/durability/character_position_house_tests.rs` (new)
- `apps/game-server/tests/support/house_runtime_position_postgres_cases.rs` (new, mounted once from `apps/game-server/tests/durability_postgres.rs`)
- `docs/agents/tasks/HOUSE-RUNTIME-1c.md`

Slot-only (final commit at the queue turn): the migration adding `house_world_id UUID NULL`
and `house_key TEXT NULL` to `game_character_last_positions` (number: next free number at
allocation time (only one migration PR in the queue); the draft's reserved 0108 is void);
module registration in `crates/gameplay/durability/src/durability/core_modules.rs` / `lib.rs`;
the one include line in `apps/game-server/tests/durability_postgres.rs`.

## Wiring points (verified on origin/main)

Extend these native owners; do not create a second house-position store:

| Native source / symbol | Required delta |
|---|---|
| `apps/game-server/migrations/0080_character_last_position.sql`, `0084_character_last_vitals.sql` | Immutable. One row per Character, monotonic `(character_lease_generation, write_sequence)`; cell, House discriminator and vitals stay one capture/upsert/key |
| `crates/gameplay/durability/src/durability/character_last_position.rs` (`LastPositionRecord`, `upsert_last_position`, `load_last_position`, `decode_row`) | Add `house: Option<HouseId>`; the common upsert sets/clears both House columns in the same winning INSERT/UPDATE; lower/equal keys change nothing |
| `crates/gameplay/durability/src/durability/character_progression.rs::assert_session_fence` | Channel-only; do not loosen it globally. The House branch needs the real House admission/session owner |
| `crates/gameplay/dispatch/src/gameplay_transport/last_position.rs` (`LastPositions`, `Saved`, `capture_last_position`, `placed_record`, admission readers) | Add the House discriminator to the dirty identity (same cell/vitals in another context is dirty); reuse sequencing, cadence, batch 64 |
| `crates/gameplay/durability/src/durability/fresh_admission.rs::write_final_position` | Currently skips non-Channel; extend its House branch inside terminal release before session state 3, never after release |
| `crates/gameplay/durability/src/durability/character_death.rs` (`consume_pending_respawn_placed`, `place_and_consume_pending_respawn_placed`) | Placed respawn clears House state through the same upsert |
| `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` release/death/logout consumers | Preserve `settle_released_death`, terminal step and logout handling |
| `crates/gameplay/durability/src/durability/house_scope_handoff.rs` (`HouseId`), `crates/gameplay/housing/src/house_scope/handoff.rs::select_entry_tile`, `crates/gameplay/world/src/house_scope/mod.rs::exit_into_channel` | Real House identity = World + canonical key; pure tile selection is not a placement token |
| `crates/gameplay/durability/src/durability/world_house_instance.rs::read_current_world_house_admission_in_transaction` | Identity observation only, not a position-write token |
| `crates/gameplay/content/src/content/project/native_house_tiles.rs`, `crates/gameplay/spell/src/spell/house_execution.rs::current_house_presence` | Qualified placement and entry cell outside House cells; Unknown on omission |
| `crates/gameplay/housing/src/last_position.rs::house_cell` | Channel reader keeps refusing house-marked tiles for None-house rows |

## Accepted decisions carried from the draft

- **D1347:** the migrations directory is the only migration registry; no registry row needed.
- **D1353 / D1366:** focused PG cases live in a support leaf mounted once in the canonical
  `durability_postgres` aggregate. Missing/non-Unicode `OTERYN_TEST_POSTGRES_ADMIN_URL`
  returns before harness construction with a PRE-ROUTING / NONCANONICAL message and never
  counts as PostgreSQL qualification.
- **D1168:** single/batch/upsert callers, respawn and terminal release are 1c's callers;
  loader and placement/rebind belong to 1a/1b.
- **D1362:** HOUSE-RUNTIME-1b owns the House scope-transition authority variant and sealed
  claim predicate; native placement and reservation consumption precede bind/generation advance.
- **D1420:** native Cargo runs touch each changed crate's `src/lib.rs` inside the heavy-lock
  lease before every command (historical draft runs lacked it and are not qualification).

## Reference behaviour built by the draft (re-implement, do not copy)

Storage (qualified on the draft as 8/8 PG 17.6 cases; historical evidence only):
- Two nullable columns with one CHECK: both null or both set, `house_world_id` equals the row
  World, `house_key` passes the existing `game_house_scope_key_valid` / `HouseId::new` grammar.
  Old rows decode as `None`, no backfill. Half-null or malformed rows are invalid state, never
  silently an outside position. No FK cascade: the saved House must survive House removal so
  the fallback can run.
- `upsert_last_position` clears both columns in the same winning write as cell, pins and
  vitals; strict `(lease, sequence)` order and pending-respawn veto unchanged.
- Native single and batch periodic writers and both respawn placement APIs go through that
  common upsert. `HouseId` is not `Copy`; use real ownership changes, no truncated key/hash.
- Cases: the original four storage cases verify the migration's exact checksum; four caller
  cases cover same-cell clearing, equal/lower/newer-lease ordering, stale input, pending
  veto, deferred COMMIT rollback and replay without a second mutation. Each case cleans up its
  own database on error or panic before reporting.

Capture, load and exit rules (open in the draft; required here):
1. The 1b owner captures cell, exact `HouseId`, session/actor, scope generation and vitals
   under its owner-lock order. A Channel carrier never saves a House because a tile is
   house-marked. Dead actors are not captured.
2. The House write branch checks current session/connection/lease, named House and assigned
   Instance, node incarnation/scope generation and the physical owner. A historical row or
   retained handoff is never current proof.
3. Load reads the row only for a new runtime actor. Pending respawn (then pending arrival)
   wins over the saved row. Some-house rows go to the 1b/SCOPE admission owner before any
   Channel snapshot; `game_house_access` re-reads property, ACL and guild revisions under its
   locks. The saved tile is used only if it belongs to that House and is walkable, unblocked
   and unoccupied; otherwise the native same-floor Chebyshev radius-3 ring (north first,
   clockwise) inside the same House; otherwise outside fallback.
4. Every exit (door, leave, kick, access/guild revocation, disposition, shutdown, rejected or
   unrecoverable saved House) uses the real reverse-handoff/fresh-admission transaction to:
   pick the destination Channel (recorded origin first), reserve the entrance or its radius-3
   fallback, else CHAR_POSITION0 step 4; clear both columns and upsert the chosen tile under
   the destination lease with a new write sequence, in the same commit as placement. An
   occupied entrance never refuses an exit. An older in-flight House write cannot resurrect
   the row.
5. Open handoffs reconcile by real ID before either side resumes (PREPARED keeps source,
   COMMITTED keeps destination); no new handoff, no timeout-only release.

## Findings and open questions

- Entrance gap: the qualified `entry_cell` is outside the House cells, but its mapping to the
  catalogue `entrance` and active bundle coordinates must be qualified by the content/1b owner
  (MAP-OVERLAY-1c). No guessed exit tile.
- Removed-House gap: a historical `HouseId` has no retained entrance coordinate. The
  content/property lifecycle must supply a retained mapping or a reviewed retirement/fallback
  contract before activation. Unknown metadata is not no-access and is never a silent
  `entry_start` login.
- A new mandatory shared root lock or family fence change needs owning review, not this card.
- Tests must not use `tokio::join!`; reuse existing two-connection helpers.

## Acceptance criteria

- [ ] Logout inside a House restores the Character inside that House.
- [ ] If the House is gone or access is lost, login falls back to the House exit deterministically.
- [ ] Columns are nullable; old rows are unaffected.
- [ ] Every exit commits the outside row atomically with placement; no independent clearing transaction.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (House row adapter in `durability`; capture in `dispatch` only as a wiring delta).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`fresh_admission.rs` 3296, `character_death.rs` 2764, `last_position.rs` 1344, `gameplay_transport/mod.rs` 8167 and `tests/durability_postgres.rs` 6972 are already over).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-durability character_position_house
cargo test --locked -p oteryn-game-server --test durability_postgres house_runtime_position_postgres_cases:: -- --nocapture   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md` §2.3
- PR #2006 (CHAR-POSITION-1)
- CP coordination: #2131 (D1122, D1168, D1329, D1347, D1353, D1361, D1362, D1366, D1405, D1420)
