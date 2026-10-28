# DONOR-F28-1 — House ACL at the mutation boundary (#710)

Tested base: `e6ce4e325ce3f988b6ca9edcd4615f93028738ba`.
Scope: test-only, owner decision 29 in
`docs/architecture/reviews/OTERYN_GAME_NIGHT_W1_OWNER_DECISIONS_2026-10-25.md`.

Donor locator: `otland/forgottenserver@99a26db7207a9e94099866678ae8457124139250`,
“Fix certain scenarios where the house protection would fail to apply (#3904)”.
The reported failure is checking protection at open/view time without checking it
again when a later mutation uses the open container. This is regression evidence,
not Reference truth; TFS was not fetched. Read-only comparison:
`canary-47dfd51f/src/items/containers/container.cpp:696` delegates removal to the
top House tile; `src/game/game.cpp:455` and `:499` check House invitation for use
and use-with.

Invariant: EXP-HOUSES-01; prior view, session or ACL cannot authorize a later item
mutation. Current ACL revision at the mutation boundary decides; projections are
presentation only.

## Owner and current authority

- `crates/gameplay/durability/src/durability/house_management/acl.rs::replace`
  owns canonical guest, subowner and door lists in `game_house_acl_entries`.
  `HouseManagementList::key` defines `guest`, `subowner`, `door:x:y:floor`.
  `game_house_management_acl_advance` (migration 0168) advances the property ACL
  revision under a same-transaction receipt (guildhall extension in 0228). The
  legacy Aleta/editor store in `house_spell_acl.rs` is explicitly not canonical
  management truth.
- Current `game_house_access` (0228) delegates personal houses to
  `game_house_personal_access` (the renamed 0143 function); that locks the current
  `game_house_properties` row `FOR SHARE`, resolves owner directly and calls
  `game_house_access_grant` (0168) for other grants. The guildhall branch locks
  current guild/member/property rows and returns current guild revisions too.
  Both return the current property ACL revision and content fence; neither trusts
  an open view.
- `DurabilityRoot::commit_house_item` calls `run_shape`, then
  `game_house_item_place` / `game_house_item_take` / `game_house_item_move`, all
  delegating to `game_house_item_apply` (0095). The SQL owner independently calls
  `game_house_access` in that mutation transaction: absent role yields
  `NO_ACCESS`; mismatched current ACL/guild revisions yield `ACCESS_CHANGED`.
  A committed command replay returns historical results without a new mutation.
  Workspace caller search finds no production caller of `freeze_house_item` or
  `commit_house_item`; their callers are existing native PostgreSQL tests.

## Requested conformance sequence

| Sub-fixture | Status | Missing owner path |
|---|---|---|
| Add into an already-open House container after R2 revokes A, plus permitted control | BLOCKED_OWNER | House container observation/open and House-container destination dispatch |
| Remove/move out under the same stale projection, plus permitted control | BLOCKED_OWNER | House-container child source and command 9 House mutation dispatch |
| Use under the same stale projection, plus permitted control | BLOCKED_OWNER | House-interior item use source/authority path |
| useEx/use-with under the same stale projection, plus permitted control | BLOCKED_OWNER | House-interior source/target owner for use-with |
| Trade offer under the same stale projection, plus permitted control | BLOCKED_OWNER | House-item trade source; current trade snapshots admit carried leaves |

These are missing paths, not observed failing ACL checks. No failing or ignored
fixture, production fix, invented House authority or claim of full conformance.
`HOUSE-RUNTIME-0` (2026-09-30) §6.4 explicitly records the missing command 9
HouseInterior sources/destinations and routes them to `HOUSE-ITEM-WIRE-1`;
§6.1 excludes containers. Its delivery table leaves containers placed in houses
to a later decision. Current `loop_move::observe_container` reads carried
containers or Channel Ground, and requires Channel player-control facts. `item_move.rs`
supports carried/Ground/corpse/equipment sources and destinations; `item_use.rs`
uses carried item authority. `connection/routing/use_item.rs` routes use-with
through existing tool/rune/mount paths. `player_trade.rs::observe_player_trade_offer`
uses `player_trade/snapshot.rs::read_leaves`; none supplies House custody.

## Supplemental native boundary fixtures

Locator:
`apps/game-server/tests/support/house_interior_postgres_cases/f28_house_acl.rs`,
mounted by the existing `house_interior_postgres_cases` in registered
`character_authority_postgres`. No new PostgreSQL target.

| Test filter | Scope | Status |
|---|---|---|
| `f28_native_add_rechecks_acl_at_mutation_boundary` | Native Place into House interior | PASS |
| `f28_native_remove_rechecks_acl_at_mutation_boundary` | Native Take from House interior | PASS |

Each uses three isolated databases: R1 guest permitted control; R1 candidate and
snapshot retained across atomic R2 revocation; R2 revision advance with permission
retained. Revocation must yield `NoAccess`, revision-only change `AccessChanged`,
and both must preserve item identities, quantities, container custody, House
locations/provenance, placement proofs, receipts and audit rows. The unchanged R1
control and a fresh R2 request with continued permission must commit.

These reuse the pre-existing `house_test_fixture::install` grant/tile SQL ports
and native mint/session/handoff/assignment/writer. They do not qualify canonical
ACL management, an open container, real dispatch, use, use-with or trade. The
retained read is a native interior snapshot, never represented as an open House
container. Full requested conformance remains BLOCKED_OWNER regardless of their
result.

## Validation

Focused PostgreSQL tests passed (2 tests, three isolated databases each). Final
commands/results, including required checks, are recorded in `F28-FINAL.md`.

Prior shape: F32 commit `fc99ad861` has five auto-mounted dispatch PG tests and a
per-case result table in its commit/PR evidence; F25 PR #2577 at
`f01115af045ece12ea3927743af7f2276b3d5a68` has one focused PG fixture plus three
unit fixtures and a per-case validation table. Neither adds an evidence file;
this retained record uses the repository's `docs/agents/evidence/` location.
