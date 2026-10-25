# ITEM-STORAGE-1 — item storage: depot, supply stash, writable items (consolidated task card)

```yaml
task_id: ITEM-STORAGE-1
title: "ITEM-STORAGE-1: item storage: depot, supply stash, writable items"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ITEM-STORAGE-1; §7 'ITEM-STORAGE-1' (DEPOT0, SUPPLY_STASH0, WRITE0 child graphs)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Carried-item/Inbox transfer exists; the Depot/Stash/write owners scoped by this card are not retained. Evidence: `crates/gameplay/durability/src/durability/item_transfer.rs`.

Remaining: Installed Depot proof/schema, locker/view/session/Premium composition, Stash and writing children remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2156, branch skeleton/item-storage-1

Status note: the draft covers the DEPOT-1 slice only (Depot TRANSFER shapes, private
storage, map depot placement facts). Its branch also carries a composed copy of
ITEM-LOOP-1 (`docs/agents/tasks/archive/ITEM-LOOP-1.md`, migration 0113 and capacity/weight work):
ignore that part — ITEM-LOOP-1 merged as #2190. The draft is closing; the mechanic is
re-implemented fresh on the current layout. Stash and writing were never started.

## Goal

Characters store items durably in a World-wide per-Character depot and a supply stash,
and read/write books, scrolls and blackboards, all through the ITEM-MOVE-2 transaction
family.

Children (allocate from the brief's child graph; the plan row is only the family entry
gate; no child of a CANDIDATE brief before the brief is ACCEPTED):
DEPOT-CONTENT-1 -> DEPOT-1 -> DEPOT-WIRE-1; STASH-CONTENT-1 -> STASH-1 -> STASH-WIRE-1;
WRITE-CONTENT-1 -> WRITE-1 -> WRITE-MAP-1 -> WRITE-MOD-1. WRITE-WIRE-1 is excluded.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | MERGED (#2190) |
| DEPOT0 acceptance | ACCEPTED (DEPOT-0 #1359; protected main 3ef900c) |
| SUPPLY_STASH0 / WRITE0 acceptance | check at allocation (outside the draft's scope) |
| HOUSE-CUSTODY-1 exclusivity guard | MERGED (#1419, migration 0025); depot join still needed |
| ITEM-MOVE-2a | MERGED (EQUIP-WIRE-1 #2009 + ITEM-LOOP-1 #2190) |
| ITEM-MOVE-2b (STASH-1, WRITE-1) | BLOCKING (no record on main; ITEM-LOOP-1 child) |
| ITEM-VIEW-1b, MAP-LOAD-1, MAP-WIRE-1 | MERGED (archive records, #1810) |
| WO-1, BANK-RET-0 | MERGED (#1748) |
| MARKET-1 / MARKET-CONTENT-1 (STASH-CONTENT-1) | MARKET-1 skeleton MERGED (#2134); MARKET-CONTENT-1 BLOCKING |
| MAIL-TEXT-1, BAGS-1 (WRITE-1) | BLOCKING (no record on main) |
| HOUSE-RUNTIME-1a / 1b / 1c (WRITE-1 house items) | 1a MERGED (#2163); 1b/1c BLOCKING |
| LEVER-1 (WRITE-MAP-1) | BLOCKING (no record on main) |
| Current Locker / USE / session-bound view owner | BLOCKING for any live depot move (none on main) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to extend (do not fork; extensions wiring-sized):

- `crates/gameplay/items/src/item_transfer.rs` — pure `TransferShape` / planner
  (`PlanInput`, `plan_transfer`, `stack_maximum`); add the two depot shapes here.
- `crates/gameplay/durability/src/durability/item_transfer.rs` — `commit_item_transfer`
  (l.~497) and `character_item_fence_is_current` (l.~885, Channel-scoped). 2,201 lines:
  put depot composition in a new child module, not inline.
- `crates/gameplay/durability/src/durability/item_transfer_audit.rs` — TRANSFER audit
  fields/shapes/source/destination kinds.
- `crates/gameplay/items/src/item_capacity.rs`, `item_weight_lookup.rs` and
  `crates/gameplay/durability/src/durability/item_weight_lookup.rs` — capacity/weight
  (already on main via ITEM-LOOP-1).
- `crates/gameplay/world/src/map/facts.rs` — map visitor; `attrs.depot` is parsed (l.~284)
  but there is no `BundleFacts::depot_placement` yet.
- `crates/gameplay/durability/src/durability/character_inbox.rs` — see P2 finding below.
- Migrations `0025_house_item_custody.sql` (exclusivity), `0076_character_inbox.sql`,
  `0083_item_use.sql` (latest complete union bodies at draft time; re-check at allocation).
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`,
  `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/items/src/depot.rs` (new) + `depot_tests.rs` (new) — L3 depot custody
  model, box/ordinal rules, withdraw/deposit shape planning over native item models.
- `crates/gameplay/durability/src/durability/character_depot.rs` (new) +
  `character_depot_tests.rs` (new) — L4 bounded summary/page reads, capacity under root lock.
- `crates/gameplay/durability/src/durability/item_transfer_depot.rs` (new) — L4 depot
  TRANSFER composition called from `commit_item_transfer` (wiring-only hook).
- `crates/gameplay/world/src/map/depot_placement.rs` (new) + tests (new) — L4 runtime
  depot placement facts (or a wiring-only addition to `facts.rs` if it stays ≤~150 lines).
- `crates/gameplay/dispatch/src/gameplay_transport/depot.rs` (new) — DEPOT-WIRE-1 handler only.
- `apps/game-server/tests/item_storage_postgres.rs` (new) +
  `apps/game-server/tests/support/depot_postgres_cases.rs` (new).
- `apps/game-server/migrations/<next>_character_depot.sql` (new) — migration number:
  next free number at allocation time (only one migration PR in the queue).
- Stash and writing (`stash`, `item_text`) follow the same layering in later children.

## Accepted design (DEPOT0 §§3,5,6,8 and CP decisions, as built in the draft)

Depot custody:

- One World + Character custody; no Channel/town/runtime scope; shared across towns/channels.
- Boxes 1..17; positive never-renumbered ordinal (native B3 u64, checked overflow refusal;
  DEPOT0 names NUMERIC(20) — keep the owning representation, a wider domain needs an adapter).
- Empty items only, one row per item regardless of quantity.
- 15,000 hard entries; 2,000 Free / 15,000 current Premium; 32 entries per page; new
  ordinal = max+1. Taking out is allowed after Premium lapses, with no stack merges.
- Each TRANSFER shape is 1 item / 2 locations / 3 work units.
- Native `PremiumGate` is a raw planning input, not a current-authority proof.

TRANSFER composition (draft built it on existing operation 3):

- Two whole-item shapes: main-backpack -> Depot box, and Depot -> one new main-backpack
  entry (or the empty container slot when eligible). Withdrawal never scatters or merges.
- Binds Character, World, source custody, destination, numeric placement ordinal.
- Lock order: Character root -> items in ID order -> container-slot locks.
- Draft extended TRANSFER fields 8–10, shapes 8–9, source kinds 5–6, destination 6; no new
  outer operation tag. Re-derive the numbering at allocation against current main.
- Depot joins exclusive live custody (0025 exclusivity function, later amendments) and the
  backpack-removal proof union; preserve every prior admitted shape and body.
- Positive slot withdrawal still requires the existing HouseExit admission.

Reads/planners confer no session, locker reach or open-view authority. Recheck reach
(Chebyshev 1, same floor) and the generation/session-bound live view at command execution,
including after reconnect. Same-World integrity binds both the native item FK and the
Character root; the capacity guard locks the root and counts rows; reads refuse
inconsistent counts and bound page size.

Map facts (D1142): preserve the top-level depot attribute as `Option<u16>` (including 0);
`depot_placement` resolves only an explicitly observed materialized Item placement (strict
key, Item palette family, exact compact ID, qualified definition). Missing/wrong/nested/
Terrain/duplicate cases refuse; no palette-ID fallback. Raw data: callers bind it to the
active bundle generation.

## cross_domain_findings (from the draft)

- `character_item_fence_is_current` is Channel-scoped; depot custody is World-scoped.
  Reuse the right current fence boundary (session/lease/root) without manufacturing scope.
- `item_transfer::{PlanInput, plan_transfer, stack_maximum}` visibility: on main these live
  in `crates/gameplay/items/src/item_transfer.rs`; check the crate API instead of
  path-loading test-bearing sources (duplicate-source guard).
- REPORT_ONLY P2: `character_inbox.rs` readers (`ORDER BY ordinal` at l.~143 and l.~192) sort
  a `ordinal::text` projection lexically (1,10,2). Fix: qualify the numeric table column
  (`game_item_character_inbox_locations.ordinal`, `game_character_inbox_deliveries.ordinal`).
  The draft's own depot reader had the same bug and was fixed the same way.
- Unused depot declarations tripped `dead_code` under `-D warnings`; land the caller with
  the module instead of suppressing the lint.

## Review dispositions (from the draft)

- Inconsistent withdrawal snapshot (empty container slot + equipped backpack facts) now
  refuses `InvalidSnapshot` (was `Ok(BoxToBackpack)`).
- Contention test uses a real competing insert that must time out on the winner's root lock.
- Root-order and exclusivity SQL invariants each reproduced RED, then GREEN.

## Remaining work

- Installed Depot schema/proof migration and grants; Locker/USE/session-bound view, Content
  (DEPOT-CONTENT-1) and Premium admission; DEPOT-WIRE-1.
- STASH-CONTENT-1 / STASH-1 / STASH-WIRE-1; WRITE-CONTENT-1 / WRITE-1 / WRITE-MAP-1 / WRITE-MOD-1.

## Acceptance criteria

- [ ] Depot contents survive restart and are scoped to Character + World, shared across towns and channels (DEPOT0 §3).
- [ ] Every store/withdraw is one ITEM-MOVE-2 transaction; no item duplication under concurrent moves.
- [ ] Depot access excluded while the same item is under house custody.
- [ ] Written text respects the RESOURCE_LIMITS cap and is sanitized.
- [ ] Older clients receive no depot/stash domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers (server_version_num 170006).
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`item_transfer.rs`
      durability is 2,201 lines).

## Inseparable bundles

- DEPOT-1 / STASH-1: migration + container writer + move transaction + audit
- WRITE-1: text payload limit (RESOURCE_LIMITS) + writer + tests
- each *-WIRE-1: wire + registry + capability + tests

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-gameplay-world depot
cargo test --locked -p oteryn-game-server --test item_storage_postgres -- --test-threads=1  # PG 17.6 admin URL
python3 tools/content-migration/regenerate_content.py --check   # content children only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- DEPOT0, SUPPLY_STASH0, WRITE0 briefs; DUR-03 one-item transaction contract
- Completion plan §4, §5 (Wave 3), §7; coordination #1622 (D1120, D1122, D1142, D1147, D1158, D1160, D1182, D1329, D1379)
- PRs #1810, #1748, #1359, #1419
