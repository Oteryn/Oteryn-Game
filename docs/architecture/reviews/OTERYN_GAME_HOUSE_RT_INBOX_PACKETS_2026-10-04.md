# HOUSE-RT-INBOX packets: HOUSE-RUNTIME-1 and INBOX-1

- Packet: `ARCH-HOUSE-RT-INBOX-PACKETS-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packets below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane order after #1773 merged: write HOUSE-RUNTIME-1 (it unblocks
  HOUSE-1a and MAP-OVERLAY-1c) and INBOX-1 (it unblocks HOUSE-1b) as one packet PR.
- Builds on:
  - HOUSE-RUNTIME-0 §3, §5-§8 and §10 (ACCEPTED by ACCEPT-SOCIAL-MAP-0, #1771);
  - HOUSE-CUSTODY-0 §3.1-§3.5 and its child HOUSE-CUSTODY-1 (migration `0025`, merged);
  - HOUSE-OWN-0 §3 and §7;
  - SOCIAL-MAP-PACKETS-1 (#1773) §0, §1.10, §1.12 and §2.3-§2.4;
  - ARCH-BATCH-PREMIUM-SOCIAL-PACKETS (#1738) §2.3 (SCOPE-HANDOFF-1) and §3.
- **CANDIDATE bases.** INBOX-1 builds MARKET-0 §5, §7 and §8 (the `CharacterInbox`) and
  DEPOT-0 §5 (the depot box and container slot). Both decisions are CANDIDATE. As in #1773,
  neither is treated as accepted here. Accepting one stays a separate owner decision, and a
  packet whose candidate base changes before allocation returns to the architect.
  HOUSE-RUNTIME-1c reads CHAR-POSITION-0 (CANDIDATE) through CHAR-POSITION-1.
- Amends, in this PR:
  - HOUSE-RUNTIME-0's child table: HOUSE-RUNTIME-1 is split into 1a, 1b and 1c (§1.1);
  - #1738 §2.3: SCOPE-HANDOFF-1 also builds the house access function (§1.2);
  - #1773 §0.1 and §2.3-§2.4: HOUSE-1a waits on HOUSE-RUNTIME-1a, 1b and 1c, MAP-OVERLAY-1c on
    HOUSE-RUNTIME-1a, HOUSE-1b on INBOX-1a; HOUSE-1a replaces the house access function and
    creates its two parts, which HOUSE-ACL-1 and HOUSE-1b each replace (§1.2, §1.5).
- Runtime, migration, registry, protocol and production authority: NONE. Each packet needs its
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order, leases and shared files

### 0.1 Findings on `main` (ba8b8dfa)

1. **No house scope kind yet.** SCOPE-HANDOFF-1 is not allocated. Every house shape runs under
   the house scope generation and the session fence of a house scope (HOUSE-RUNTIME-0 §6.1), so
   all of HOUSE-RUNTIME-1 waits on it.
2. **No map runtime.** There is no `map` module (MAP-LOAD-1 is not merged). Presence, movement,
   doors and tile selection need the house tiles' walls and walkability from the active bundle
   (HOUSE-RUNTIME-0 §3). The item shapes do not.
3. **No last-position row.** CHAR-POSITION-1 is not merged, so §6.3 (logout and login inside a
   house) has no row to add columns to.
4. **The exit is gated.** SCOPE-HANDOFF-1 keeps the exit into a channel scope refused until
   ADMIT-0 is accepted and its §3.2 lifting conditions hold (#1738 §2.3). Leave, kick,
   revalidation and the disposition quiesce are all exits.
5. **No ownership row before HOUSE-1a.** `game_house_properties` is HOUSE-1a's. The admission
   commit (HOUSE-RUNTIME-0 §4.1) and every item write (§5.4) take FOR SHARE on that row.
6. **The Inbox has no client before MARKET-WIRE-1.** The Inbox view is MARKET-WIRE-1's
   (MARKET-0 §10), and the out-shapes run "with the depot view open" (DEPOT-WIRE-1).
7. **HouseInterior has no transfer event.** `OneItemTransferV1` has a Ground or corpse source and
   an inventory destination only.

### 0.2 Order

| Packet | Waits on |
|---|---|
| HOUSE-RUNTIME-1a | SCOPE-HANDOFF-1 (HOUSE-CUSTODY-1 is merged) |
| HOUSE-RUNTIME-1b | HOUSE-RUNTIME-1a, MAP-LOAD-1 |
| HOUSE-RUNTIME-1c | HOUSE-RUNTIME-1b, CHAR-POSITION-1 |
| INBOX-1a | none (HOUSE-CUSTODY-1 is merged) |
| INBOX-1b | INBOX-1a, DEPOT-1; allocated with the Inbox view child (§1.7) |

Downstream, as amended in this PR:
- HOUSE-1a waits on BANK-1, HOUSE-RUNTIME-1a, 1b and 1c, and PREM-WIRE-1. HOUSE-CUSTODY-0 §4
  and HOUSE-OWN-0 order ownership after the whole interior runtime: no house is sold before
  players can enter it;
- MAP-OVERLAY-1c waits on MAP-OVERLAY-1b and HOUSE-RUNTIME-1a;
- HOUSE-1b waits on HOUSE-1a, INBOX-1a and MAP-OVERLAY-1c.

Every other document that names HOUSE-RUNTIME-1 as a dependency means all three halves. That
covers HOUSE-1a, HOUSE-VIEW-1, BED-1 and the HOUSE-ITEM-WIRE-1 row of #1738 §3.

### 0.3 Leases

- **Migrations.** Each packet with a migration takes one number from the control plane at
  allocation. This decision names no number. HOUSE-RUNTIME-1b has no migration.
- **Note on PREM-WIRE-1.** Its packet (PREMIUM-ACTIVATION-0 §2) names migration `0073`, which
  #1534 now holds (D541). The control plane leases PREM-WIRE-1 a fresh number at allocation, so
  that packet needs no edit.
- **One-item operation tag.** HOUSE-RUNTIME-1a takes the next free `OneItemTransactionV1`
  operation tag at allocation, after MAP-OVERLAY-1b's (#1773 §0.2). INBOX-1b takes the one
  after. INBOX-1a takes none.
- **No capability and no wire.** None of these packets adds a protocol message or a capability.

### 0.4 Shared files

| File | Writers |
|---|---|
| `docs/contracts/game-events/v1/native_one_item_transaction.proto` | MAP-OVERLAY-1b, MAP-OVERLAY-1c (#1773), then HOUSE-RUNTIME-1a and INBOX-1b: each a new operation message under its leased tag, additive only; no existing field changes |
| `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` (type 2 text) | the #1773 writers, then HOUSE-RUNTIME-1a and INBOX-1b: one clause each |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | each packet its own rows; HOUSE-RUNTIME-1a and INBOX-1b also add their consumer line and worst case to the `DUR03-RL-07-*` rows |
| `apps/game-server/src/durability/mod.rs` | each persistence packet: its `mod` lines only |
| `apps/game-server/tests/character_authority_postgres.rs` | each persistence packet: its path-mod lines only |
| `apps/game-server/src/durability/item_mint_audit.rs` | HOUSE-RUNTIME-1a and INBOX-1b: the new operation arm and the oneof tag list only |
| the house access function `game_house_access` | SCOPE-HANDOFF-1 (stub), then HOUSE-1a (final body), by `CREATE OR REPLACE` in its own migration; no later child replaces it (§1.2) |
| the access parts `game_house_access_grant` and `game_house_access_fence` | HOUSE-1a (stubs); then HOUSE-ACL-1 replaces the grant part only and HOUSE-1b the fence part only, each by `CREATE OR REPLACE` in its own migration (§1.2) |
| the SCOPE-HANDOFF-1 handoff module | HOUSE-RUNTIME-1b (the exit callers) and HOUSE-RUNTIME-1c (the house-column clear in the exit commit): their call sites only |

## 1. Rulings

### 1.1 HOUSE-RUNTIME-1 is three packets

HOUSE-RUNTIME-0's child table gives HOUSE-RUNTIME-1 three things with different roots:

- the item shapes, which need only the house scope kind;
- the live interior, which also needs the map;
- the house position, which also needs the last-position row.

One packet would wait on all of MAP-LOAD-1 and CHAR-POSITION-1, and so would MAP-OVERLAY-1c,
which needs only a HouseInterior insert path for its step-4 recheck test. HOUSE-1a still waits
on all three halves: HOUSE-CUSTODY-0 §4 and HOUSE-OWN-0 order ownership after the whole interior
runtime, so no player buys a house they cannot enter. The split also keeps each half within one
reviewable batch.

So:
- **HOUSE-RUNTIME-1a** (§2.1): the three HOUSE-RUNTIME-0 §6.1 shapes with provenance, their
  grants, their event, and the §5.2 role rules at the write (§5.4).
- **HOUSE-RUNTIME-1b** (§2.2): activation and unload (§3), presence and movement, the rules
  inside, doors, leave, kick, revalidation (§5), map items and use (§6.2), and the disposition
  quiesce (§7).
- **HOUSE-RUNTIME-1c** (§2.3): logout and login inside a house (§6.3).

Each is on the house chain's playable path or a precondition of it. None adds a speculative layer.

### 1.2 One access function, replaced by each owner child

The admission commit and every item write lock the property row and re-read its `acl_revision`.
Before HOUSE-1a there is no property row. The database also has to keep the HOUSE-CUSTODY-0 §3.5
closure: a player writer is admitted only for a house with an owner and a grant. So:

- **The function.** One SQL function decides house access:
  `game_house_access(world_id, house_key, character_id)`. It returns:
  - the role (`OWNER`, `SUBOWNER`, `GUEST`) or no row;
  - the property state, which tells the caller whether the content fence is set;
  - the `acl_revision`;
  - for a guild-entry grant, the guild revisions it used.

  It takes the FOR SHARE locks of HOUSE-RUNTIME-0 §4.1 on whatever rows it reads.
- **Who calls it.** The SCOPE-HANDOFF-1 admission commit and every HOUSE-RUNTIME-1a shape. Each
  compares the result with the revisions its pre-check used and refuses on any difference.
- **The stub.** SCOPE-HANDOFF-1 creates the function as a stub that returns no row. With the stub,
  every house refuses `NO_ACCESS`, which is HOUSE-RUNTIME-0 §5.1 "a house with no owner admits
  nobody". This amends #1738 §2.3, which is not yet allocated.
- **One replacement, then composable parts.** HOUSE-1a replaces the stub body once, by
  `CREATE OR REPLACE` in its own migration with the same signature, and that body is final: no
  later child replaces `game_house_access`. The body:
  - reads `game_house_properties` FOR SHARE and returns `OWNER` for the owner. This is #1773
    §1.10's owner-only access;
  - for anyone else, calls `game_house_access_grant(world_id, house_key, character_id)`, which
    returns `SUBOWNER`, `GUEST`, an exclusion, or no row, with the guild revisions it used;
  - takes the property state from `game_house_access_fence(world_id, house_key)`, which returns
    the fenced or unfenced state.

  HOUSE-1a creates both parts as stubs: the grant part returns no row, and the fence part returns
  the state HOUSE-1a's own property row holds. Each part then has one owner, which replaces only
  that part by `CREATE OR REPLACE` with the same signature:
  - HOUSE-ACL-1 replaces the grant part with the subowner, guest, exclusion and guild-entry
    rows;
  - HOUSE-1b replaces the fence part so that `DISPOSITION` returns the fenced state.

  HOUSE-ACL-1 and HOUSE-1b stay parallel and may merge in either order: neither writes the other's
  part or the composed body, so neither can erase the other's behaviour. The locks keep the
  HOUSE-RUNTIME-0 §4.1 order inside the body: the property row, then the fence part's rows, then
  the grant part's rows. A change to the composed body or to a part's signature is a new
  architect decision.
- **The test house.** The HOUSE-RUNTIME-0 §10 test house replaces the `game_house_access` body in the test database
  from test code, reading a fixture table that the test creates. It is never a migration, and no
  production World can reach it.
- **Grants.** The runtime role gets EXECUTE on the function only through the SECURITY DEFINER
  shape functions and the admission commit. It never gets a grant on the fixture or property rows.

### 1.3 The shapes are SECURITY DEFINER functions

HOUSE-RUNTIME-1a opens the HOUSE-CUSTODY-0 §3.5 closure exactly as HOUSE-RUNTIME-0 §6.1 says. Each
shape is one SECURITY DEFINER function, and `oteryn_game_runtime` gets EXECUTE on those three
functions and on nothing else in the house tables. Each function does, in one transaction:

- **Fences.** It checks the house scope generation and the acting character's session fence
  (composition rule 2) in the one commit.
- **Locks.** It takes, in this order:
  - `character_root`;
  - `game_house_access` (§1.2);
  - the item rows by ItemInstanceId.
- **Refusals.** It refuses with a typed reason, and nothing is moved, when:
  - the tile is not a tile of this house (`NOT_HOUSE_TILE`). Before HOUSE-1a there are no
    `game_house_tiles` rows, so the function reads the tile set from the same replaceable source
    as the role, `game_house_access_tile(world_id, house_key, position)`. That function follows
    the stub, test and replacement rules of §1.2, and HOUSE-1a replaces it with a read of
    `game_house_tiles`;
  - `HOUSEOWN0-RL-14` is reached (`HOUSE_STORAGE_FULL`);
  - the role lacks the right in HOUSE-RUNTIME-0 §5.2 (`NO_ACCESS`);
  - the content fence is set (`HOUSE_CLOSED`);
  - the item has contents (`ITEM_HAS_CONTENTS`);
  - a revision differs from the one the authorization used (`ACCESS_CHANGED`).
- **Provenance.** It writes or deletes the provenance under the `0025` rules.

### 1.4 The house content revision is HOUSE-VIEW-1's

HOUSE-RUNTIME-0 §9 requires every `HouseInterior` mutation to advance a house content revision in
its own transaction. Its only reader is the HOUSE-VIEW-1 projection.

HOUSE-VIEW-1 builds it as a trigger on `game_item_house_interior_locations`. The trigger advances
the house's revision row on every insert, update or delete. So:
- every writer advances it without knowing it: the shapes, the HOUSE-1b disposition steps, and
  any write while the runtime is unloaded;
- no mutation can skip it, which is the §9 refusal rule.

HOUSE-RUNTIME-1a builds no revision. A table with no reader before HOUSE-VIEW-1 would be
speculative, and the trigger needs no backfill: a house with no revision row reads as revision 0.

### 1.5 The exit gate

Until SCOPE-HANDOFF-1's exit gate lifts (ADMIT-0 §3.2), a character could enter a house it cannot
leave. So, until then, HOUSE-RUNTIME-1b refuses every entry with `HOUSE_CLOSED`.

This is a check in the house runtime, not a test seam. HOUSE-RUNTIME-1b's exit tests use the exit
exactly as SCOPE-HANDOFF-1 ships it. While the gate is closed, those tests assert the typed
refusal and the closed entry. The exit-dependent acceptance items then stay open, so
HOUSE-RUNTIME-1b stays CANDIDATE after merge until they are met (#1738 §1.2). The packet adds no
way to open the gate.

### 1.6 The Inbox is two packets

INBOX-1 has two parts with different roots:

- the delivery into the Inbox, which HOUSE-1b's disposition needs (#1773 §1.12);
- the two out-shapes, which need DEPOT-1 and the depot view.

HOUSE-1b would otherwise wait on DEPOT-1, and so on DEPOT-WIRE-1, MAP-WIRE-1 and its own contract
acceptance. The disposition only has to land items somewhere value-safe. So:

- **INBOX-1a** (§2.4):
  - the `CharacterInbox` family, its counter, the guards and proofs of MARKET-0 §8 for Inbox rows;
  - one unreserved delivery function.

  It waits on nothing that is not merged.
- **INBOX-1b** (§2.5): the two out-shapes and their event.

Playable-first: INBOX-1a is a precondition of HOUSE-1b's eviction and move-out, which are on the
house chain. It stores only items a later out-shape returns to the player. It is not speculative.

### 1.7 INBOX-1b waits for its client

The out-shapes have no client before the Inbox view (MARKET-WIRE-1 §10, finding 6). Building them
before any player can call them would be infrastructure no player reaches. So INBOX-1b is
allocated together with the child that builds the Inbox view.

Whether that view stays in MARKET-WIRE-1 or becomes its own child after DEPOT-WIRE-1 is decided
when it is packeted. This decision does not amend MARKET-0 §10.

Until then, items delivered to an Inbox are durable and safe but not reachable. That is a value
delay, never a loss. HOUSE-1b's evictions and move-outs before then are value-safe.

## 2. Packets

### 2.1 HOUSE-RUNTIME-1a (house item shapes)

```yaml
task_id: OTV2-20261004-house-runtime-1a
decision: HOUSE-RUNTIME-0 §5.2, §5.4, §6.1, §6.4, §8 (item row); HOUSE-CUSTODY-0 §3.2, §3.4, §3.5; this decision §1.2-§1.4
candidate_bases: []
worker: oteryn-hard-worker
review: hard, persistence and security review (Codex, final frozen head)
branch: allocated by the control plane
base: main after SCOPE-HANDOFF-1 merges
migration_lease: one number from the control plane at allocation
one_item_operation_tag_lease: the next free tag at allocation (§0.3)
depends_on: [SCOPE-HANDOFF-1]
owned_paths:
  - apps/game-server/migrations/NNNN_house_interior_shapes.sql  # three SECURITY DEFINER shapes, game_house_access_tile stub, EXECUTE grants, the DUR-03 deltas below
  - apps/game-server/src/durability/house_interior.rs           # the shape writers and typed refusals
  - apps/game-server/src/durability/house_interior_audit.rs     # the house transfer event
  - apps/game-server/src/durability/item_mint_audit.rs          # the new operation arm and the oneof tag list only
  - apps/game-server/src/durability/mod.rs                      # the mod lines only
  - apps/game-server/tests/house_interior_postgres.rs
  - apps/game-server/tests/support/house_interior_postgres_cases.rs
  - apps/game-server/tests/support/house_test_fixture.rs        # the §10 test house: fixture table and the replaced access bodies, test database only
  - apps/game-server/tests/character_authority_postgres.rs      # the path-mod line only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto # one new operation message under the leased tag, additive only
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json          # one type 2 clause only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json                # the HOUSERT0 item row; DUR03-RL-07-* consumer line and new worst case
  - docs/agents/tasks/archive/OTV2-20261004-house-runtime-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server house_interior
  - cargo test --locked -p oteryn-game-server house_custody
  - cargo test --locked -p oteryn-game-server item_transfer
  - python3 tools/agents/validate_governance.py
```

Builds, as MARKET-0 §8 and DEPOT-0 §6 do (each guard keeps its body and gains one clause):

- **The three shapes** of HOUSE-RUNTIME-0 §6.1, as §1.3 describes:
  - `CharacterInventory` or `CharacterEquipment` to `HouseInterior`;
  - `HouseInterior` to `CharacterInventory`;
  - tile to tile in the same house.
- **The proofs.** A `HouseInterior` row is inserted or deleted only by a committed shape, with
  evidence in the `0014` idiom. HOUSE-1b adds its disposition steps as a further clause.
- **Reservations and receipts.** `HouseInterior` source and destination kinds, the shape numbers,
  and a nullable `channel_id`.
- **The consistency guard.** The house kinds, with no Ground or corpse evidence.
- **The event.** A new `OneItemTransactionV1` operation under the leased tag. Its house location
  message carries:
  - the `HouseId` (World and house key);
  - the position and stack ordinal;
  - the house scope generation;
  - on a placement, the reclaim subject and placement transaction.

  The source and destination are each exactly one of house and inventory (or equipment). A
  tile-to-tile move has both as house locations. The shapes call no existing operation, so
  `OneItemTransferV1` is unchanged.

Acceptance tests (on the §10 test house; the stub refuses everything):

- with the stub body, every shape refuses `NO_ACCESS` for every character, and nothing is
  written;
- `oteryn_game_runtime` can EXECUTE the three shapes and nothing else on the house tables. A
  direct INSERT, UPDATE or DELETE on `game_item_house_interior_locations` or the provenance table
  is refused for that role;
- each shape commits one item with its provenance: a placement writes the placer as reclaim
  subject; a take deletes it; a tile move keeps it with a +1 revision;
- the §5.2 table, with the owner, a subowner and a guest from the fixture. A guest takes only
  items it placed; a subowner and the owner take any item;
- an item with contents, a tile outside the house, `HOUSEOWN0-RL-14` and a set content fence each
  refuse with their typed reason;
- a concurrent revocation (the fixture's `acl_revision` raised in another transaction) either
  commits first, and the write refuses `ACCESS_CHANGED`, or commits after the item write;
- a command from an older session of the character writes nothing while the house scope
  generation is still current. A stale house scope generation writes nothing;
- the event round-trips with each source and destination pair. One with two sources, no
  destination or a Ground location is refused, and every existing type 2 event still decodes
  and validates unchanged. The worst case fits `DUR03-RL-07-*`;
- the `0025` exclusivity guard still refuses an item in two locations across the new shapes.

Not in scope: activation, presence and movement (1b); login (1c); the content revision (§1.4); the
client path (HOUSE-ITEM-WIRE-1; ITEM-MOVE-WIRE-1 §5's `BLOCKED` stays); stack merge and split;
containers; cross-house moves.

### 2.2 HOUSE-RUNTIME-1b (the live interior)

```yaml
task_id: OTV2-20261004-house-runtime-1b
decision: HOUSE-RUNTIME-0 §3, §4.2 (callers), §5, §6.2, §7, §8; this decision §1.5
candidate_bases: []
worker: oteryn-hard-worker
review: hard and security review (Codex, final frozen head)
branch: allocated by the control plane
base: main after HOUSE-RUNTIME-1a and MAP-LOAD-1 merge
migration_lease: none
depends_on: [OTV2-20261004-house-runtime-1a, MAP-LOAD-1]
owned_paths: the house scope runtime module and its tests, under a path the control plane fixes
  at allocation against the live MAP-LOAD-1 and SCOPE-HANDOFF-1 layout; the SCOPE-HANDOFF-1
  handoff module (the exit call sites only); docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  (HOUSERT0-RL-01 to -04); docs/agents/evidence/HOUSE-RUNTIME-1b-*.md;
  docs/agents/tasks/archive/OTV2-20261004-house-runtime-1b.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server house_runtime
  - python3 tools/agents/validate_governance.py
acceptance:
  - every exit-dependent item below stays open until SCOPE-HANDOFF-1's exit gate lifts (§1.5)
  - HOUSERT0-RL-02 and HOUSERT0-RL-04 are measured and registered, with evidence, before any
    activation outside a test World
```

Builds:

- **Activation and unload (§3).** Activation runs on the entering character's node if it has
  capacity, else on any node of the World, with one live assignment. Unload runs after
  `HOUSERT0-RL-01`.
- **The map and rules inside (§3).** The house's tiles from the active bundle, a protection zone,
  no creatures, and summons dismissed at entry.
- **Presence and movement** on house tiles, with `HOUSERT0-RL-03` and the single-writer tile
  reservation of §4.1 and §6.3.
- **Doors (§5.1).** An inner door with a door list admits that list and the owner. Before
  HOUSE-ACL-1 there are no lists, so only the owner passes.
- **Leave (§5.3)** and **kick (§5.5).** Each is a server function; HOUSE-WIRE-1 later adds the
  wire variant.
- **Revalidation (§5.4).** Re-check everyone inside when `acl_revision` or a granting guild
  revision changes, and move out anyone who lost access.
- **Use (§6.2).** Map-authored items are never pickupable. `USE` follows ITEM-USE-0, and beds stay
  inert. The depot locker opens nothing until DEPOT-1, so `USE` on it answers `NOT_AVAILABLE`.
- **Disposition quiesce (§7).** When `game_house_access` returns the fenced state, everyone is
  moved out and new entries refuse `HOUSE_CLOSED`. A release reopens entry.
- **The entry refusal of §1.5.**

Acceptance tests (on the §10 test house):

- activation on the entering character's node, then on another node at capacity; a second
  activation waits; unload after `HOUSERT0-RL-01` with nobody inside, with every item still
  there;
- the 201st character is refused `NO_ROOM`;
- no attack, regeneration or creature inside, and a summon is dismissed at entry;
- with the fixture's lists, an inner door admits its list and the owner only;
- an `acl_revision` change moves out a character that lost access. The fixture's fenced state
  moves everyone out and refuses entry `HOUSE_CLOSED`, and a release reopens it;
- a guest cannot kick; a subowner kicks guests only; the owner kicks anyone;
- with the exit gate closed, every entry refuses `HOUSE_CLOSED`. Leave, kick, revalidation and
  quiesce return the exit's typed refusal and write nothing;
- after the gate lifts (an open acceptance item): every exit lands on `entrance` or its
  fallback on the origin Channel.

Not in scope: entry and exit themselves (SCOPE-HANDOFF-1); the shapes (1a); login (1c); views
(HOUSE-VIEW-1); beds (BED-1); the client path.

### 2.3 HOUSE-RUNTIME-1c (logout and login in a house)

```yaml
task_id: OTV2-20261004-house-runtime-1c
decision: HOUSE-RUNTIME-0 §4.2 (position write), §4.3, §6.3; CHAR-POSITION-0 §3.2 and §3.3 as amended by HOUSE-RUNTIME-0
candidate_bases: [CHAR-POSITION-0]
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main after HOUSE-RUNTIME-1b and CHAR-POSITION-1 merge
migration_lease: one number from the control plane at allocation
depends_on: [OTV2-20261004-house-runtime-1b, CHAR-POSITION-1]
owned_paths: the leased migration (two nullable house columns on CHAR-POSITION-1's
  last-position table, with a CHECK that both are set or neither); the house branch of
  CHAR-POSITION-1's write and admission read; the house-column clear in the SCOPE-HANDOFF-1 exit
  commit and in the §4.3 recovery fallback (call sites only); their tests;
  docs/agents/tasks/archive/OTV2-20261004-house-runtime-1c.md. The control plane fixes the exact
  paths at allocation against the merged CHAR-POSITION-1 layout.
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server house_runtime
  - cargo test --locked -p oteryn-game-server character_position
  - python3 tools/agents/validate_governance.py
```

Until 1c merges, CHAR-POSITION-0's skip rule applies inside a house. A logout there writes no
house position, so a login never returns the character inside.

Acceptance tests:

- a logout inside writes the `HouseId` and tile;
- a login with access re-runs the §4.1 admission commit and lands on the saved tile, or on its
  in-house fallback within 3;
- a login refused for access, property state or tile is placed at `entrance` on a Channel. That
  admission clears the columns and writes the chosen tile;
- a concurrent revocation either commits first, and the login refuses, or commits after and finds
  the character inside;
- every exit and the recovery fallback clear the columns in their commit;
- a disconnect right after an exit never returns the character inside.

### 2.4 INBOX-1a (the CharacterInbox and its delivery)

```yaml
task_id: OTV2-20261004-inbox-1a
decision: MARKET-0 §5 (family, in, capacity), §7 (the counter lock in rule 4), §8 (Inbox tables, exclusivity, proofs, consistency); SOCIAL-MAP-PACKETS-1 §1.12; this decision §1.6
candidate_bases: [MARKET-0 (CharacterInbox)]
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main
migration_lease: one number from the control plane at allocation
depends_on: []
owned_paths:
  - apps/game-server/migrations/NNNN_character_inbox.sql   # location table, counters, delivery function, the guard clauses below
  - apps/game-server/src/durability/character_inbox.rs     # read helpers and the typed delivery error, for the callers' tests
  - apps/game-server/src/durability/mod.rs                 # the mod line only
  - apps/game-server/tests/character_inbox_postgres.rs
  - apps/game-server/tests/support/character_inbox_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs # the path-mod line only
  - docs/agents/tasks/archive/OTV2-20261004-inbox-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server character_inbox
  - cargo test --locked -p oteryn-game-server house_custody
  - python3 tools/agents/validate_governance.py
```

Builds:

- **The location table.** `game_item_character_inbox_locations` holds `CharacterInbox {
  character_id, ordinal NUMERIC(20) }`, one row per whole item without contents.
  - Its scope is Character + World, with no channel column.
  - It has a RESTRICT FK to the Character root and an FK to the item instance.
  - Its rows are immutable: an Inbox item never changes in place.
  - Death, `WorldReset` and channel changes never touch it. No existing writer gains a clause
    on it.
- **The counter.** `game_character_inbox_counters` has one row per Character, with `committed`
  and `next_ordinal`.
  - A trigger keeps `committed` equal to the entry count. MARKET-1 later adds its reservations
    to the same column, in its own delta.
  - INBOX-1a has no refusal on the counter. `MARKET0-RL-06` is registered by MARKET-1, its only
    refuser.
- **The exclusivity guard.** `game_item_location_exclusive` keeps its `0025` body and gains the
  Inbox table.
- **Proofs.** An Inbox row is inserted only by the delivery function. It is deleted only by a
  committed out-shape (INBOX-1b) or a later Market operation, each added as its own clause. Until
  then there is no delete path.
- **The consistency guard.** The Inbox table is added to the counted location tables.
- **The delivery function** (#1773 §1.12):
  - **Signature.** `game_character_inbox_deliver(item_instance_id, character_id, world_id,
    cause_kind, cause_ref)`. It returns `(character_id, ordinal)` for the caller's own event.
  - **Behaviour.** One call delivers one item. It is unreserved and never refused for capacity:
    the counter may exceed `MARKET0-RL-06`, and the only bound is the caller's own. It locks the
    counter row (upserted) FOR UPDATE in place of rule 4's `character_root` lock (MARKET-0 §7).
    It takes no session fence, so the recipient may be offline or on another channel.
  - **Errors.** It refuses only an item with contents, a Character of another World, and an
    item still in another location at commit (the exclusivity guard).
  - **Cause kinds.** `cause_kind` is a closed CHECK. INBOX-1a defines the column with no kind.
    Each caller adds its kind in its own migration: HOUSE-1b adds `HOUSE_DISPOSITION`, and
    MARKET-1 adds its own.
  - **Delivery record.** Each delivery writes a delivery record keyed by (item, cause), which is
    the placement proof.
  - **Grants.** The function is SECURITY DEFINER, and no runtime role gets EXECUTE on it. Only
    the callers' own SECURITY DEFINER functions, owned by the migration owner, can call it.
- **No event.** The delivery emits no event of its own. The caller's transaction audits it in the
  caller's event, from the returned location.

Acceptance tests (a test-only SECURITY DEFINER caller in the test database, with a test cause kind
added by test code, stands in for HOUSE-1b):

- a delivery to an offline Character and to one on another channel commits, without a session
  fence;
- the counter rises by 1 per delivery. Ordinals are increasing per Character and never reused;
- 100,001 deliveries are all accepted: the counter exceeds `MARKET0-RL-06`, and nothing refuses;
- an item with contents, a Character of another World, and an item still in another location at
  commit are each refused;
- the runtime role cannot execute the function, insert into or delete from the Inbox table, or
  update the counter;
- an Inbox row cannot be updated or deleted. Death, the `WorldReset` retirement and a channel
  change leave it untouched;
- two concurrent deliveries to one Character serialize on the counter row. A delivery and a
  `character_root` lock of the same Character do not deadlock under the MARKET-0 §7 order.

Not in scope: the out-shapes and their event (INBOX-1b); reservations and the ceiling refusal
(MARKET-1); mail and parcels (MAIL-0); the Inbox view (§1.7).

### 2.5 INBOX-1b (the out-shapes; held)

```yaml
task_id: OTV2-20261004-inbox-1b
decision: MARKET-0 §5 (out), §7, §8 (removal proof, reservations and receipts, audit), §9 (Inbox out row); DEPOT-0 §5
candidate_bases: [MARKET-0 (CharacterInbox), DEPOT-0]
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main after INBOX-1a and DEPOT-1 merge
migration_lease: one number from the control plane at allocation
one_item_operation_tag_lease: the next free tag at allocation (§0.3)
depends_on: [OTV2-20261004-inbox-1a, DEPOT-1]
owned_paths: the leased migration (the two shapes, the removal proof clause, the Inbox
  reservation and receipt kinds, the EXECUTE grants); apps/game-server/src/durability/character_inbox.rs;
  its audit module; apps/game-server/src/durability/item_mint_audit.rs (the new operation arm and
  the oneof tag list only); the proto, registry and resource-row edits of §0.4; its tests;
  docs/agents/tasks/archive/OTV2-20261004-inbox-1b.md. The control plane fixes the exact paths at
  allocation against the merged DEPOT-1 layout.
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server character_inbox
  - python3 tools/agents/validate_governance.py
acceptance:
  - allocated together with the Inbox view child (§1.7)
```

Builds the two one-item out-shapes, each run by the owner with the depot view open (DEPOT-1's
predicate):

- Inbox to a new main backpack entry, or to the empty container slot, as DEPOT-0 §5 says;
- Inbox to a depot box.

Each runs under the owner's session fence and `character_root`, then DEPOT-0 rule 4's items and
container-slot order, then the Inbox counter last (MARKET-0 §7). It lowers `committed` by 1. Its
event is a new `OneItemTransactionV1` operation under the leased tag. It registers the
MARKET-0 §9 "Inbox out" row.

Acceptance tests:
- both shapes commit one item;
- a stale session, a closed depot view, a full backpack and a depot at its limit each refuse;
- the event round-trips, and the worst case fits `DUR03-RL-07-*`.

## 3. Rejected options

- **One HOUSE-RUNTIME-1.** MAP-OVERLAY-1c would wait on MAP-LOAD-1 and CHAR-POSITION-1, which it
  does not need. It would also exceed the batch limit.
- **HOUSE-1a after HOUSE-RUNTIME-1a only.** It would sell houses before players can enter them,
  against HOUSE-CUSTODY-0 §4 and HOUSE-OWN-0's rejected "auctions before the interior runtime".
- **Each owner child replacing the whole access body.** HOUSE-ACL-1 and HOUSE-1b are parallel,
  so whichever merged last would erase the other's clause. One final body with two parts, each
  with one owner, composes in either order (§1.2).
- **A runtime-only role check before HOUSE-1a.** Granting EXECUTE on the shapes with the role
  checked only in Rust would open the HOUSE-CUSTODY-0 §3.5 closure in the database. The access
  function keeps it closed until an owner child replaces the body.
- **A test house in a migration.** HOUSE-RUNTIME-0 §10 makes it test code. A migration would put
  an owned house in every World.
- **The content revision in HOUSE-RUNTIME-1a.** It has no reader before HOUSE-VIEW-1. A trigger
  added then covers every writer with no backfill (§1.4).
- **Entry allowed while the exit is gated.** A character could be stuck inside (§1.5).
- **One INBOX-1 after DEPOT-1.** HOUSE-1b would then wait on the whole depot and map wire chain
  for a delivery that needs none of it (§1.6).
- **Building the out-shapes now.** No client can call them before the Inbox view (§1.7).
- **Extending `OneItemTransferV1` with house and Inbox locations.** Changing its exactly-one source
  and destination rules would touch every existing type 2 validator. A new operation under a
  leased tag is additive.

## 4. Decision test

| Question | Answer |
|---|---|
| Must decide now? | YES. HOUSE-1a, MAP-OVERLAY-1c and HOUSE-1b cannot be allocated without these packets. |
| Smallest sufficient? | Five packets, each with one root. 1b has no migration. No content revision, no out-shape before its client. |
| Any infrastructure no player reaches? | INBOX-1a, as the value-safe target of HOUSE-1b's disposition (§1.6). HOUSE-RUNTIME-1a, as the closure-preserving access path HOUSE-1a needs. |
| Does any packet lower a review, a test or a protection? | No. The exit gate stays, the §3.5 closure stays in the database, and every packet keeps its hard review. |
| Any wire, protocol or production change? | No. Two packets each lease one event operation tag, additive only. |
| Owner question? | None. Accepting MARKET-0, DEPOT-0 or CHAR-POSITION-0 stays a separate owner decision. |
