# DEPOT-1

```yaml
task_id: DEPOT-1
title: "DEPOT-1: per-town Character depot storage and shared Inbox foundation"
mode: IMPLEMENT
status: waiting   # DEPOT-1 slice merged (#2334); remaining slices per card
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-depot-1
pr: null
base_sha: ac3f7fbc6
owner: Codex standalone task worker
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
merge_authority: CP_ONLY
owned_paths:
  - crates/gameplay/items/src/depot.rs
  - crates/gameplay/items/src/depot_tests.rs
  - crates/gameplay/items/src/lib.rs
  - crates/gameplay/durability/src/durability/character_depot.rs
  - crates/gameplay/durability/src/durability/core_modules.rs
  - apps/game-server/migrations/0151_character_depot.sql
  - apps/game-server/src/lifted_tests/auto/depot_postgres_cases.rs
  - apps/game-server/src/lifted_tests/auto/depot_postgres_cases/
  - docs/agents/tasks/DEPOT-1.md
public_contracts: []
depends_on:
  - HOUSE-CUSTODY-1
  - ITEM-MOVE-2a
  - BAGS-1
  - INBOX-1a
  - DEPOT-CONTENT-1
  - DEPOT-WIRE-1
blocks:
  - BAGS-DEPOT-1
  - INBOX-1b
  - MAIL-1
  - MARKET-1
cross_repository_coordination_id: null
external_repositories: []
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@43914f183`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: #2334: pure depot bounds and dormant per-Character/per-town custody, box ordinals and hard capacity guards; existing Inbox delivery is reused. Evidence: `crates/gameplay/items/src/depot.rs`, `crates/gameplay/durability/src/durability/character_depot.rs`, `apps/game-server/migrations/0151_character_depot.sql`, `crates/gameplay/items/src/depot_tests.rs`, `apps/game-server/src/lifted_tests/auto/depot_postgres_cases.rs`.

Remaining: Admitted locker content/binding, session-bound paginated depot/Inbox views, fenced deposit/withdraw receipts/audit/replay, Free/Premium runtime admission, Inbox out-shapes and real-dispatch qualification remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Scope and authority

The owner's 2026-10-23 task explicitly requests **per-town** depot chests per
Character. This supersedes DEPOT-0's world-wide town sharing for this pass:
storage keys are Character + World + map depot/town id. Channel is never a
storage partition. Map depot ids retain the u16 range, including zero.
The durable layout therefore differs intentionally from current Tibia's
shared-town access; no historical decision text is silently changed.

Advance the storage portion that does not assume the absent locker/content/view
owners. Reuse native ItemInstances, existing container families and the existing
CharacterInbox; do not mint a movable chest or create a second Inbox owner.
ADR-0022 assigns the bounds to items (L3), storage models/SQL to durability
(L4), and future command composition to dispatch (L5).
This pass intentionally remains a dormant storage backend: no depot command,
capability or runtime writer is enabled while the live-access dependencies are absent.

Bound META policy 3.1.0 was read at immutable commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. This is local AUTHORING only;
the owner prohibits Git mutation, publication, credential access and writes
outside this worktree. No paid review was triggered.

## Dependencies (re-verified on origin/main ac3f7fbc6)

Each row was checked with `git log --oneline origin/main --grep <ID>` and
matching code under crates/ and apps/. A docs-only log hit is not implementation.

| Dependency | Actual state / evidence |
|---|---|
| DEPOT-0 acceptance | ACCEPTED: #1359, acceptance #2209; accepted depot decision on main |
| HOUSE-CUSTODY-1 | MERGED: #1419, `0025_house_item_custody.sql`; latest exclusivity union in 0128 |
| ITEM-MOVE-2a | MERGED: #2009 + #2190; main-backpack removal, native item loop and 0128 |
| ITEM-LOOP-1 | MERGED: #2190; `items/item_capacity.rs`, durability/item_loop_move and live dispatch |
| BAGS-1 / BAGS-0 acceptance | MERGED / ACCEPTED: #2190 / #1416; native nested entries and container-tree dispatch exist |
| INBOX-1a | MERGED: #1785, migration 0076, durability/character_inbox; reward delivery extensions in 0101 |
| ITEM-VIEW-1 | MERGED: #1713, #1720; existing corpse/carried views and handles, no depot views |
| MAP-LOAD-1 | MERGED: #1783; base map facts preserve depot placement classification |
| MAP-WIRE-1 | MERGED: #1906/#1909 and later fixes; native map observation exists |
| WO-1 | MERGED: #1169/#1179; closed WorldObject kinds include container_fixture |
| DEPOT-CONTENT-1 | BLOCKING live activation: log hits only DEPOT-0's brief; no admitted locker binding in production dispatch |
| DEPOT-WIRE-1 | BLOCKING live activation: log hits only DEPOT-0's brief; no capability, depot view, box target or session-bound locker owner |

Downstream, not assumed schemas: BAGS-DEPOT-1 has docs-only hits (#1416,
#1427); INBOX-1b has no log hit or runtime out-shape. MAIL-1 and MARKET-1
already name the existing CharacterInbox as their delivery owner; their
content/wire/transaction completion remains separate. ITEM-FORGE-1 consumes
MARKET-1 for MARKET-TIER-1, rather than granting depot write authority.

## Existing implementation reused (Step 0)

- Native items/backpack entries: items/item_transfer and durability/item_transfer;
  nested containers and ownership: durability/item_loop_move, migrations 0113/0128.
- The item-level exclusive-location guard: 0025, extended through 0128. This
  pass preserves all eight existing custody arms and adds the depot arm and
  empty-container restriction, including nested contents.
- CharacterInbox: migration 0076, its counters/delivery proof, MAIL-owned
  reward delivery extensions, and typed durability readers. No replacement
  schema or general runtime Inbox-write grant is introduced.
- Item-use dispatch exists for carried consumables, doors and the entry reward
  chest. Its production world-object route has no depot locker binding, virtual
  chest view or live locker authority. The existing item-use route alone cannot
  safely admit a depot transfer.
- Historical depot0 / supply-stash0 cards, BAGS-USE-1 and ITEM-STORAGE-1 were
  read. The latter's obsolete proposed standalone PG target is not followed.

## Delivered in this pass

- Migration 0151 (highest main migration 0144; owner minimum 0151) adds private
  town/box custody for native items and retained per-box ordinal counters.
  Seventeen fixed boxes are implicit, not item instances. No renumbering occurs.
- Character root -> item -> counter locking serializes placements. World and
  live-item checks precede placement; a native stack consumes one entry.
  The hard 15,000-entry ceiling is shared by all boxes of one town.
- Depot placement participates in existing deferred exclusive custody. Items
  with direct, corpse or nested contents cannot enter. Locations cannot be
  updated/deleted/truncated until a later migration supplies proven out-shapes.
- Runtime/control receive SELECT only. There is no callable Rust write API or
  runtime SQL placement/removal grant that bypasses missing current authority.
- Six auto-mounted library PG cases cover storage/reload/isolation, capacity
  and existing Inbox delivery, refusal rollback, all three contents families,
  role/immutability protection and concurrent ordinal allocation.
- Deferred counter proof rejects skipped conflicting inserts without leaking
  capacity/ordinals; the capacity case also races for the final available slot.
- L3 reference bounds and native L4 custody representation use existing item
  models. No new public functions, enums or trait methods are introduced.
- Continuation: fixed the six PG cases sharing one recovery-directory tag.
  The shared harness removes/recreates its directory by tag, so parallel cases
  previously raced and five failed with `NotFound`. Each case now has its own
  recovery tag and database; all six pass together without serializing tests.
- The storage/reload case also asserts the upper map depot id, 65,535, in box 17,
  alongside zero, town separation, Character separation and ordinal allocation.

## Reference parity

Reference constants have one independent row each in the L3 table-driven
`depot_reference_parity_limits` test. The manual is the authority for the
accepted 17 boxes and Premium limit; Canary's differing configured box/Premium
counts are not imported. Canary is reference evidence, never the runtime.

| Original rule | Value | Source | Assertion |
|---|---|---|---|
| Depot Chest fixed compartments | 17 | [Tibia manual, World](https://www.tibia.com/gameguides/?section=world&subtopic=manual) | L3 row `fixed depot boxes`; PG `depot_towns_boxes_and_characters_are_durable_and_isolated` |
| Free account admission limit | 2,000 entries | Same manual; [Canary constructor](https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/containers/depot/depotchest.cpp) | L3 row `free account entries`; runtime admission remains unfinished |
| Premium / database ceiling | 15,000 entries | Tibia manual, World | L3 row `Premium / database ceiling`; PG `depot_capacity_counts_stacks_once_across_boxes_and_does_not_cap_inbox` |
| One paginated chest page | 32 entries | Canary same file, `maxSize`, pagination | L3 row `one paginated chest page`; live pagination remains unfinished |
| Whole external stack cost, independent of quantity | 1 entry | Canary same file, `DepotChest::queryAdd` | L3 row `one whole item or stack`; PG capacity case uses quantity 100 per entry |

The live official-manual search result was readable; direct wiki/manual fetches
returned HTTP 403. Canary's exact-revision source was fetched read-only.
Town separation is the explicit owner deviation above. Empty-only storage is
the accepted DEPOT-0 first slice; nested depot trees await BAGS-DEPOT-1.

## Acceptance

- [x] Step 0 dependency/code sweep and existing Inbox reuse recorded.
- [x] Additive per-town custody, box ordinals, capacity and exclusive-location guards drafted.
- [x] PG cases auto-mounted once under the server library; no new PG target or CI edits.
- [x] Required Cargo validation passes on this worktree, including real PostgreSQL 18.6 cases.
- [ ] Live locker open, deposit/withdraw and Inbox out-shapes run through real dispatch.
- [ ] Independent persistence review and protected CI/integration (control plane).

## High-risk authority/recovery qualification

Storage-only foundation: no session admission, PREPARE, COMMIT API, receipt or
controller authority is added. Runtime mutations are denied, rather than
treating a CharacterId, stored location or box counter as current authority.
PG negatives cover World mismatch, duplicate custody, ordinal substitution,
invalid bounds, counter rollback, contents and missing role privileges.
The concurrent placement test observes an actual Character root lock wait.
Live authority/fencing, receipt replay and ambiguous-commit recovery remain
mandatory for the blocked writer; storage tests do not claim that evidence.

## Validation

- All Cargo commands used `/home/mole/oteryn-cache/codex-bin/cargo`, the repaired
  queued wrapper, with `CARGO_NET_OFFLINE=true`; no dependencies were downloaded.
- PASS: `cargo fmt --all -- --check`.
- PASS: `cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings`.
- PASS: `cargo test --locked -p oteryn-gameplay-items --lib --quiet`
  (36 passed, 0 failed).
- PASS: `cargo test --locked -p oteryn-gameplay-items --lib --quiet -- depot::tests::depot_reference_parity_limits`
  (1 passed, 35 filtered out; all five reference-table rows asserted).
- PASS: `cargo test --locked -p oteryn-gameplay-durability --lib --quiet -- schema::contract_tests::`
  (8 passed, 384 filtered out; no unfiltered durability PG suite).
- PASS: `cargo test --locked -p oteryn-game-server --lib --quiet -- depot_postgres_cases::`
  (6 passed, 415 filtered out, 10.47 seconds). The wrapper supplies the admin
  route, and the harness verifies PostgreSQL 18.6 before applying the actual
  embedded migrations to each case's own database. No whole PG target was run.
- PASS: `python3 tools/repository/validate_repository_policy.py` (23 files / 68 workflows).
- PASS: `python3 tools/agents/validate_governance.py` (22 policy documents / 9 lanes).
- PASS: `python3 -m unittest discover -s tools/agents/tests` (124 tests).
- PASS: whole-diff CODEX-CHECKLIST self-review, workspace constructor/field
  sweep, unchanged legacy custody-arm comparison, and `git diff --check`. All
  eight previous custody arms and contents guards are preserved verbatim.

## Unfinished

- DEPOT-CONTENT-1 / DEPOT-WIRE-1: admitted base-map locker binding, current
  session/generation-bound view, reach/revision/capability checks and paginated
  chest/box/Inbox view. No live depot-open implementation exists to extend.
- Fenced native TRANSFER deposit/withdraw receipts, audit, placement/removal
  proof union and restart/replay through real dispatch. Free/Premium runtime
  admission (2,000/15,000) needs the same current authority boundary; the database
  foundation only enforces the hard ceiling.
- INBOX-1b out-shapes, BAGS-DEPOT-1 trees, and downstream MAIL/MARKET/Forge
  completion. Stash, search, partial stacks and cross-box moves are out of scope.
- `crates/test-support` on origin/main has no property module: property tests
  skipped as instructed. No arithmetic API was added in this storage-only pass.
- `MECHANICS_STATUS.md` does not exist on origin/main; no index file created.
- Independent persistence review and protected CI/integration remain the
  control plane's responsibility. This worker performed no Git mutation,
  publication, external review trigger or credential access.

Next step: deliver the validated local diff to the control plane for independent
review and integration. Live activation waits for the named content/wire owners
and fenced transfer work. This card stays IN_PROGRESS and is not archived or
declared playable/merge-ready.
