# SOCIAL-MAP packets: PARTY-1, GUILD-1, HOUSE-1a/1b and the map overlay and cutover

- Packet: `SOCIAL-MAP-PACKETS-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packets below. It needs
  ACCEPT-SOCIAL-MAP-0 (#1771) merged first: that PR accepts the decisions these packets build.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane order after D551: "PARTY-1, GUILD-1, HOUSE-1 and
  MAP-OVERLAY-CUTOVER-PACKET-1, all in one PR if they fit the batch limit". BED-0 and ECON-RET-0
  stay out.
- Builds on:
  - PARTY-PVP-0 §3, §4, §8.1 and §13;
  - GUILD-0 §3, §4 and §11;
  - HOUSE-OWN-0 §3-§9 and §12;
  - HOUSE-CUSTODY-0 §3.5 and §3.6;
  - ADR-0021 §4.4, §4.7 and §4.8;
  - DUR-03 "Map items and world reset";
  - MAP-LOAD-PACKET-1 (#1744) §2.2;
  - PREMIUM-ACTIVATION-0 (#1743) §1.2, §1.3 and §2.2;
  - ARCH-BATCH-PREMIUM-SOCIAL-PACKETS (#1738) §1.1, §2.4 and §3.
- **CANDIDATE bases.** Each packet names the candidate decisions it reads. Following the control
  plane's answer (a), none is treated as accepted here. Accepting one stays a separate owner
  decision, and a packet whose candidate base changes before allocation returns to the architect.
- Supersedes:
  - the thin PARTY-1 packet of #1738 §2.5;
  - the thin GUILD-1 packet of #1743 §2.2, for its owned paths, order and tests. Its Premium
    rules stay as written there;
  - the HOUSE-1 row of #1738 §3, which this decision splits (§1.3).
- Amends, in this PR:
  - HOUSE-RUNTIME-0's child table: HOUSE-RUNTIME-1 no longer waits on HOUSE-1 or HOUSE-ACL-1
    (§1.10);
  - PREMIUM-ACTIVATION-0 §1.3: one consumer row for house acquisition (§1.4);
- Corrects, by ruling and without editing that file: ACCEPT-SOCIAL-MAP-0 §3 (#1771), which says
  GUILD-1 follows INBOX-1 (§1.2).
- Runtime, migration, registry, protocol and production authority: NONE. Each packet needs its
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order, leases and shared files

### 0.1 Order

| Packet | Waits on |
|---|---|
| PARTY-1 | CHAT-2 (the relay carries the hint) |
| GUILD-1 | BANK-1, PREM-WIRE-1 |
| HOUSE-1a | BANK-1, HOUSE-RUNTIME-1a, PREM-WIRE-1 |
| HOUSE-1b | HOUSE-1a, INBOX-1a, MAP-OVERLAY-1c |
| MAP-OVERLAY-1a | MAP-LOAD-1 |
| MAP-OVERLAY-1b | MAP-OVERLAY-1a |
| MAP-OVERLAY-1c | MAP-OVERLAY-1b, HOUSE-RUNTIME-1a |
| MAP-CUTOVER-1 | MAP-OVERLAY-1c |

HOUSE-ACL-1, HOUSE-WIRE-1, GUILD-BANK-1, GUILDHALL-1 and every wire child stay held as #1738 §3
lists them. HOUSE-WIRE-1 waits on HOUSE-1b, and GUILDHALL-1 on HOUSE-1b and HOUSE-ACL-1.
HOUSE-RUNTIME-1 waits only on SCOPE-HANDOFF-1 and HOUSE-CUSTODY-1 (§1.10), so the house chain has
no cycle: HOUSE-RUNTIME-1, then HOUSE-1a, then HOUSE-ACL-1 and HOUSE-1b.

**Amendment (`OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md` §1.1, §1.2, §1.6).** HOUSE-RUNTIME-1
is split into 1a, 1b and 1c, and INBOX-1 into 1a and 1b. HOUSE-1a and MAP-OVERLAY-1c wait on
HOUSE-RUNTIME-1a only; HOUSE-1b waits on INBOX-1a only. HOUSE-1a also replaces the bodies of
`game_house_access` and `game_house_access_tile` in its own migration (owner only, its
`game_house_tiles`).

### 0.2 Leases

- **Migrations.** `0070` is the latest on `main`. BANK-1 holds `0071` and GOLD-FEE-2 holds
  `0072`. Each packet with a migration takes one number from the control plane at allocation. A
  packet merges only after every lower leased number is merged or released.
- **Event types.** GUILD-1 (the guild event, GUILD-0 §4.4) and HOUSE-1a (the house operation
  event) each take one `GAME_EVENT_FOUNDATION_REGISTRY.json` event type at allocation. Type 3
  (`BANK_OPERATION`) is leased to BANK-1. Each new entry carries every
  `required_event_type_fields` entry, including `payload_schema` and `payload_message`, with
  the schema file of §1.12.
- **One-item operation tags.** MAP-OVERLAY-1b takes the next free `OneItemTransactionV1`
  operation tag at allocation (tag 8 if still free; §1.11). MAP-OVERLAY-1c adds fields to
  operation tag 5 and takes no tag.
- **No capability and no wire.** None of these packets adds a protocol message or a capability.

### 0.3 Shared files

| File | Writers |
|---|---|
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | each packet edits only its own rows, in merge order; MAP-OVERLAY-1b and -1c also add their consumer line and new worst-case boundary test to the `DUR03-RL-07-*` rows |
| `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` | GUILD-1, HOUSE-1a: one entry each; HOUSE-1b: one notes clause for the house event |
| `apps/game-server/src/durability/mod.rs` | each persistence packet: its `mod` lines only |
| `apps/game-server/src/domain/mod.rs` | PARTY-1, GUILD-1, HOUSE-1a, HOUSE-1b: their `mod` lines only |
| `apps/game-server/src/lib.rs` | PARTY-1 (`mod party`); MAP-OVERLAY-1a adds nothing, since `map` is MAP-LOAD-1's |
| `apps/game-server/tests/character_authority_postgres.rs` | each persistence packet: its `#[path = "support/<x>_postgres_cases.rs"] mod` lines only |
| `apps/game-server/src/map/mod.rs` | MAP-LOAD-1, then MAP-OVERLAY-1a (`mod overlay`) and MAP-CUTOVER-1 (`mod boot`): their `mod` lines only |
| `apps/game-server/src/map/overlay.rs` | MAP-OVERLAY-1a, then MAP-OVERLAY-1b: the `mod pickup` line only |
| `apps/game-server/src/durability/item_mint_audit.rs` | MAP-OVERLAY-1b: the new `OneItemOperationV1` arm and the oneof tag list only |
| `apps/game-server/src/durability/bank.rs` | HOUSE-1a: the house ledger kinds and the `HOUSEOWN0-RL-15` term of the credit headroom check only (§1.12) |
| `docs/contracts/game-events/v2/house_operation.proto` | HOUSE-1a (new), then HOUSE-1b (additive operation kinds and lines; no existing field changes) |
| `docs/contracts/game-events/v1/native_one_item_transaction.proto` | MAP-OVERLAY-1b (new operation), then MAP-OVERLAY-1c (additive fields of tag 5); no existing field changes |
| `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` (type 2 text) | MAP-OVERLAY-1b, then MAP-OVERLAY-1c: one clause each |

## 1. Rulings

### 1.1 PARTY-1

- **The combat lock without PvP state.** PARTY-PVP-0 §8.1 makes leave and the cleanup job read
  the union of the ATTACK-0 in-fight deadline (`ATTACK0-RL-03`) and the PvP blocks of
  `game_character_pvp_state`. That table is PVP-1's and the blocks are PVP-RT-1's. Until PVP-1
  merges, PARTY-1 reads the in-fight deadline only, through one function
  `party_combat_locked(character)`. PVP-1 adds the PvP clause in that one function; a missing row
  then means no block. No PvP table or stub is created by PARTY-1.
- **The relay hint.** PARTY-1 adds only the party variant `{party_id, revision}` to the CHAT-2
  relay payload, through the payload enum CHAT-2 defines. It changes nothing else in
  `chat/**`. If CHAT-2 merges without a payload variant point, PARTY-1 stops and asks the
  architect.
- **No wire.** Party commands are built as server operations with their tests. The client
  messages are PVP-WIRE-1's and PARTY-CHAT-1's.
- **`spell/party.rs` stays.** The `PartyWorld` trait stub is PARTY-XP-1's. PARTY-1 does not
  implement it.

### 1.2 GUILD-1

- **Dependencies.** GUILD-1 needs:
  - BANK-1, for the junior predicate (BANK-0 §4.4: "until that island and fact exist, no
    character is junior");
  - PREM-WIRE-1, for the `PremiumStatus` seam (#1743 §1.2);
  - `GUILD_ACTIVITY_RETENTION_V1`, which is already on `main` (#1748).

  It does **not** wait on INBOX-1: no GUILD-0 §3 operation writes to a CharacterInbox. This
  corrects ACCEPT-SOCIAL-MAP-0 §3, which says GUILD-1 follows INBOX-1.
- **Disband steps 1 and 2.** The guildhall and balance steps of GUILD-0 §3.4 are no-ops until
  GUILDHALL-1 and GUILD-BANK-1 add them. Each step is a named function that returns "done" with a
  test that pins it. The balance clause of the deferred guard is GUILD-BANK-1's.
- **Rows.** GUILD-1 registers `GUILD0-RL-01` to `-09`, `-11`, `-15`, `-17` and `-18`.
  `GUILD0-RL-10` (the activity log window) and `-16` (the query page) are read on the wire and are
  GUILD-WIRE-1's. `-12`, `-14`, `-19` and `-20` belong to the bank and guildhall children.

### 1.3 HOUSE-1 is two packets

HOUSE-OWN-0's HOUSE-1 is larger than one reviewable batch: about ten tables, three jobs, four
ledger kinds and the disposition steps. It also needs INBOX-1 only for the disposition. So it is
split:

- **HOUSE-1a, acquisition:**
  - the property, tile and housing slot rows of §3;
  - the auction of §4: escrow of maximum plus first rent, proxy price, the 15 minute
    anti-sniping window, settlement and the release steps;
  - the ledger kinds `HOUSE_BID_RESERVE`, `HOUSE_BID_RELEASE`, `HOUSE_PRICE`, and `HOUSE_RENT`
    for the first rent, which settlement burns from the escrow (owner answer H1);
  - the `HOUSEOWN0-RL-13` ceiling on escrow returns. If MARKET-1 has not added it to the balance
    CHECK first, HOUSE-1a's migration does (HOUSE-OWN-0 §4, "Credits");
  - the escrow headroom guard `HOUSEOWN0-RL-15` (§1.9);
  - the §8 Ground tile guard, since it runs before any house becomes owned;
  - the auction and settlement job of §9 with its lock order;
  - the Premium consumer row (§1.4).
- **HOUSE-1b, tenancy:**
  - rent collection and grace of §5, from the second period on;
  - the warning for an unpaid rent, as a function the login path of HOUSE-WIRE-1 calls (§1.12);
  - the eviction ban;
  - move-out of §6;
  - the disposition fence and steps of §7, to the CharacterInbox;
  - catalogue revisions of §3, applied at a planned reset (§1.8).

  It needs INBOX-1, and through it MARKET-0's CharacterInbox (a CANDIDATE base), and
  MAP-OVERLAY-1c for the reset that applies catalogue revisions.

Pending amendments that neither half builds:

- the MAIL-0 rent letter: HOUSE-1b builds the login warning only, and `game_mail_system_letters`
  stays MAIL-1's;
- the BED-0 sleeper release;
- guildhall rows (`owner_kind` `GUILD`), which GUILDHALL-1 adds.

A house can be won in HOUSE-1a before rent collection exists. Settlement burns the first rent
from the escrow and sets `paid_until` to 30 days later, as HOUSE-OWN-0 §4 says. No job collects
rent before HOUSE-1b. HOUSE-1b's first collection pass treats a `paid_until` already in the past
as due at that pass: it charges one rent, and on success the next `paid_until` is 30 days after
the pass; on failure grace starts at the pass. No rent is charged for the gap, and no house is
evicted for a period in which collection did not exist.

### 1.4 Premium for house acquisition (amends PREMIUM-ACTIVATION-0 §1.3)

House bids and settlement follow the H2a ruling of HOUSE-OWN-0:

- `NotActivated`: Premium is not required;
- `Current`: allowed;
- `NotCurrent`: a bid is refused `NOT_PREMIUM`, and at settlement step 1 the bid is excluded like
  any bid that fails eligibility (HOUSE-OWN-0 §4);
- a house acquired while `NotActivated` is kept after activation (HOUSE-OWN-0 §5: a lapse
  changes nothing).

The settlement job calls `with_premium_batch_gate` before it takes any HOUSE-OWN-0 §9 lock.
HOUSE-1a wires it.

### 1.5 The map overlay is three packets

MAP-OVERLAY-1 of ADR-0021 holds the overlay, the MINT of map items, the Ground rebuild and the
World reset. That is three persistence concerns, so it is split:

- **MAP-OVERLAY-1a, the overlay:**
  - the per-channel overlay of §4.4: hidden base items and added items per tile;
  - the 1 s expiry index;
  - the 64 item tile limit;
  - the `MAP01-CHANNEL-OVERLAY-BYTES` budget;
  - the Ground rebuild of player items after a restart, fail-closed on a `map_revision` mismatch.

  It is in memory only and has no migration.
- **MAP-OVERLAY-1b, map item pickup:**
  - the `MapItemMaterialization` MINT cause, with its own reservation and receipt tables (the
    precedent is `0012` reward claim and `0031` change mint);
  - the eligibility rule;
  - hiding the origin at freeze;
  - re-hiding after a crash;
  - the reach check on a retried MINT.
- **MAP-OVERLAY-1c, the World reset:**
  - the durable World reset record;
  - steps 1-4 and the crash recovery of ADR-0021 §4.7;
  - the `WorldReset` retirement cause;
  - the HOUSE-CUSTODY-0 §3.6 preflight and step-4 recheck;
  - `MAP01-RESET-RETIRE-MS`, measured.

### 1.6 The step-4 recheck lock (HOUSE-CUSTODY-0 §3.6)

HOUSE-CUSTODY-0 §3.6 requires the step-4 recheck to run "under a lock that blocks HouseInterior
inserts". This decision fixes that lock:

1. The activation transaction locks the World reset record row `FOR UPDATE`.
2. It then takes `LOCK TABLE game_item_house_interior_locations IN SHARE MODE`.
3. It rechecks, then writes the new digest, epoch N+1 and `ACTIVATED`.

`SHARE` mode blocks every `INSERT`, `UPDATE` and `DELETE` on the table until commit, including a
move inside one house. That is wider than inserts alone, and it is accepted: the step is short,
and its own reads are not blocked.

**Why a table lock.** The house scope is World-scoped, so closing channel admission does not
close it. A row lock cannot cover an insert that has no row yet.

### 1.7 `WorldReset` shares the per-item retirement

`0015` keys `game_item_decay_retire_reservations` by item and generation, with a non-null
`corpse_item_instance_id` and a deadline of at least 60,000 ms. MAP-OVERLAY-1c widens both tables
in its migration:

- a `cause_kind` column with a CHECK of `CORPSE_DECAY` or `WORLD_RESET`, defaulting existing rows
  to `CORPSE_DECAY`;
- a `reset_epoch` column, required for `WORLD_RESET` and null for `CORPSE_DECAY`;
- the corpse fields and the deadline floor apply only to `CORPSE_DECAY`, by CHECK;
- the receipt keeps its `item_instance_id` primary key, so one item has at most one retirement
  across both causes (ADR-0021 §4.7 step 3).

**Stale `CorpseDecay` reservations are replaced, not adopted (#1773 P1 4178085304).** Reset
step 2 gives every channel scope a fresh ownership generation, and `0015` says a later
generation reserves afresh, because the former generation can no longer commit. So:
- the reset reserves its own `WORLD_RESET` row keyed by (item, new generation) for every live
  item, including one that has a `CorpseDecay` reservation of an older generation;
- that older row stays as history and can never commit, since its fence generation has ended;
- the receipt stays unique per item, so an item that already has a `CorpseDecay` receipt is no
  longer live and is not retired again.

The reset never reuses an older reservation and never writes a second receipt.

### 1.8 Catalogue revisions run inside the reset

HOUSE-OWN-0 §3 applies house catalogue revisions when a new bundle activates at a planned reset.
The reset is MAP-OVERLAY-1c's and the revision rules are HOUSE-1b's, so:

- the refusal of a bundle that retires, re-keys or breaks an owned house is part of the reset
  preflight, before step 1;
- the revision steps (new `VACANT` rows, `RETIRED`, cancelled auctions with release steps) run
  after step 3 and before activation, as World job steps keyed by (reset epoch, house), at most
  `HOUSEOWN0-RL-11` houses per pass;
- HOUSE-1b owns these functions and adds the two calls in `world_reset.rs`;
- until HOUSE-1b merges, the MAP-OVERLAY-1c preflight refuses a target bundle whose house
  catalogue differs from the active one on a World that has any property row. It fails closed.

### 1.9 Private house escrow headroom (#1771 P2 4178031083)

HOUSE-OWN-0 §4 lets escrow returns exceed `BANK0-RL-01` up to the hard ceiling `HOUSEOWN0-RL-13`.
It does not keep room for them. While a bid is `HELD`, other credits, such as Market credits, can
raise the balance to the ceiling, and then the return of that escrow cannot fit. GUILD-0 already
closes this for guildhall bids (`GUILD0-RL-20`). This decision adds the same guard for private
houses:

- **`HOUSEOWN0-RL-15`:** a deferred guard on each (Account, World) keeps `balance +
  sum(escrow_gold of HELD private house bids of that Account in that World)` at most the hard
  ceiling. One bid per (Account, World) means the sum has at most one term.
- A credit that would break the guard is refused `BALANCE_LIMIT` by its own system (bank, Market,
  house), as `GUILD0-RL-20` does. So every bid release, lowering and settlement return fits, and
  release steps never block.
- A bid reserve, a raise or a return only moves value between the two terms, so it never changes
  the sum.
- **One guard.** GUILDHALL-1 extends this same guard with its guildhall account escrow term, so
  `GUILD0-RL-20` and `HOUSEOWN0-RL-15` are checked as one sum, and the GUILD-0 §5.3 headroom
  subtracts both escrow terms. GUILDHALL-1 does not add a second guard.

### 1.10 HOUSE-RUNTIME-1 does not wait on HOUSE-1 (amends HOUSE-RUNTIME-0)

HOUSE-RUNTIME-0's child table lists "HOUSE-1 and HOUSE-ACL-1 (for owned houses)" among
HOUSE-RUNTIME-1's dependencies. HOUSE-1a waits on HOUSE-RUNTIME-1 and HOUSE-ACL-1 is held, so
that is a cycle (#1773 P1 4178085295). HOUSE-RUNTIME-0 itself already says HOUSE-RUNTIME-1 can
land first, tested on the §10 test house, and #1738 §3 rules it out of waiting on HOUSE-1.
So:
- HOUSE-RUNTIME-1 depends on SCOPE-HANDOFF-1 and HOUSE-CUSTODY-1 only;
- it reads ownership from `game_house_properties` when that table exists, and from the §10 test
  fixture before it does;
- **access before HOUSE-ACL-1.** A house with no ACL rows admits its owner only: §5.1 with empty
  subowner, guest and door lists. HOUSE-ACL-1 adds the list rows and their checks to the runtime
  path. So HOUSE-1a can open auctions before HOUSE-ACL-1, and an owner can enter and use their
  house alone;
- the HOUSE-RUNTIME-0 child table row is amended in this PR to match.

### 1.11 Audit schemas of the map packets

DUR-03 §39.3 requires every MINT and retirement cause to be admitted by the one-item audit
schema and the event registry. Both map packets therefore own their schema change:
- **MAP-OVERLAY-1b (#1773 P1 4178085301).** `item_mint_audit.rs` accepts only `loot_mint` with a
  creature death occurrence. Map item pickup follows the reward-claim precedent (operation tag 4):
  a new operation `map_item_mint` (`OneItemMapItemMintV1`) with the closed cause
  `OneItemMapItemMaterializationV1 {world, channel, base bundle digest, placement_key, reset
  epoch}` and a Ground destination. It has its own audit module, and the registry type 2 text
  gets one clause. No existing field changes. `OneItemOperationV1` and its oneof tag list live in
  `item_mint_audit.rs`, so MAP-OVERLAY-1b owns that file for the new arm and tag 8 only
  (#1773 P1 4178117645).
- **MAP-OVERLAY-1c (#1773 P1 4178085303).** `OneItemDecayRetireV1` gains additive fields:
  - `OneItemWorldResetV1 world_reset = 7` (`{world_id, reset_epoch, item_instance_id}`, the
    full `WorldReset` key of ADR-0021 §4.7 step 3 and DUR-03 "Map items and world reset";
    #1773 P1 4178117650). Its `item_instance_id` equals the retired item's. Exactly one of it and
    the existing `cause` = 5 is set;
  - `OneItemGroundContainerEntryV1 ground_container_entry = 8`, for an entry of any live Ground
    container, carrying the container's live Ground as scope authority, as `corpse_entry` does.
    It is only valid with `world_reset`;
  - the `deadline_unix_ms` of the corpse cause does not apply to `world_reset`.

  The registry type 2 text gets one clause, and no existing field changes.
- Both packets keep the type 2 payload within `DUR03-RL-07-PAYLOAD-BYTES` and
  `DUR03-RL-07-ENVELOPE-BYTES`: each measures its new worst case and adds it as a boundary test
  and consumer line on those rows.

### 1.12 Event schemas and owned paths (#1773 P1 4178117654)

- **Payload schemas.** Following BANK-1's `game-events/v2/bank_operation.proto`:
  - GUILD-1 adds `docs/contracts/game-events/v2/guild_event.proto` with
    `oteryn.events.v2.GuildEventV1`: the GUILD-0 §4.4 event and its activity entry messages;
  - HOUSE-1a adds `docs/contracts/game-events/v2/house_operation.proto` with
    `oteryn.events.v2.HouseOperationV1`: the HOUSE-OWN-0 §9 evidence, its operation kinds and
    its value lines;
  - HOUSE-1b adds its operation kinds (rent, grace, move-out, eviction, disposition step,
    release) and the disposition item `TRANSFER` line to that file, additively, and one notes
    clause to the registry. No existing field changes.

  Each registry entry names its file and message in `payload_schema` and `payload_message`.
- **House burns.** Price and rent burn no item, so their `BURN` lines are value lines of the
  house event under its own house variants of `FeeBurnCause` (`HOUSE_PRICE`, `HOUSE_RENT`). The
  one-item `FeeBurnCauseV1` of event type 2 is unchanged.
- **The `HOUSEOWN0-RL-15` credit check.** Every credit reads its headroom through BANK-1's one
  credit headroom check in `bank.rs`. HOUSE-1a adds the escrow term there, so a credit is refused
  `BALANCE_LIMIT` before any write and never by a deferred guard abort. If a credit path on
  `main` does not use that check, HOUSE-1a stops and asks the architect.
- **The Inbox delivery.** HOUSE-1b delivers through INBOX-1's delivery function for an unreserved
  delivery. If INBOX-1 merges without one, HOUSE-1b stops and asks the architect.
- **No runtime wiring.** The jobs of PARTY-1, GUILD-1, HOUSE-1a and HOUSE-1b are pass functions
  tested directly. The PartyView admission read and the HOUSE-1b rent warning are functions.
  Calling them from the channel loop, admission or login belongs to each family's wire child,
  so none of these packets edits `world_runtime.rs`, admission or session code.

## 2. Packets

### 2.1 PARTY-1

```yaml
task_id: OTV2-20261004-party-1
decision: PARTY-PVP-0 §3, §4.1, §4.2, §4.4, §8.1 (combat lock, §1.1 here); this decision §1.1
candidate_bases: [CHAT-0, ATTACK-0]
worker: oteryn-hard-worker
review: hard, persistence, security and privacy review (Codex, final frozen head)
branch: allocated by the control plane
base: main after CHAT-2 merges
migration_lease: one number from the control plane at allocation
depends_on: [CHAT-2]
owned_paths:
  - apps/game-server/migrations/NNNN_parties.sql             # §3 tables: parties, members, invitations, social blocks, social settings
  - apps/game-server/src/domain/party.rs                     # pure rules: invite, accept, succession, limits
  - apps/game-server/src/domain/mod.rs                       # the mod line only
  - apps/game-server/src/durability/party.rs                 # the §4.1 transactions and lock order
  - apps/game-server/src/durability/mod.rs                   # the mod line only
  - apps/game-server/src/party/**                            # the PartyView cache, full refresh, revision check, presence record, cleanup job
  - apps/game-server/src/lib.rs                              # the mod line only
  - apps/game-server/src/chat/relay_payload.rs               # the party hint variant only (§1.1), or the file CHAT-2 names for the payload enum
  - apps/game-server/tests/party_postgres.rs
  - apps/game-server/tests/support/party_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # the path-mod line only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # PARTYPVP0-RL-01 to -03, -08, -26, -28 to -30
  - docs/agents/tasks/archive/OTV2-20261004-party-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server party
  - python3 tools/agents/validate_governance.py
```

Builds PARTY-PVP-0 §3 and §4 except the party room (PARTY-CHAT-1) and the benefits (PARTY-XP-1):

- the §3 tables, including the social block and the party-invite and channel-visibility
  settings;
- invite with the consent check, accept, decline, revoke, leave, succession, pass leadership,
  the shared-XP toggle, block and the settings writes;
- invitation expiry;
- the node `PartyView` cache: full refresh at node start, at relay (re)connect and on a hint, the
  admission read, and the `PARTYPVP0-RL-26` revision check;
- the revisioned member presence record and its ordered invalidation (§4.4);
- the cleanup job, with removal after `PARTYPVP0-RL-08`.

Acceptance tests:

- each limit refuses at its value plus one: members, open invitations per party and per invitee,
  invitations per minute, blocked characters;
- an invite to a character who blocks the inviter, or whose setting refuses it, is refused
  without revealing which;
- leave and the cleanup job refuse a member inside the in-fight deadline and accept one after it;
- succession picks the §4.1 successor, and a party whose last member leaves is deleted;
- a lost hint is caught by the revision check within `PARTYPVP0-RL-26`, and a view not
  confirmed in time reads stale;
- a node restart rebuilds the view by full refresh, and no cached or hinted record is ever sent;
- two concurrent accepts for the last seat: exactly one commits;
- the session-generation fence refuses a stale writer.

Not in scope: PvP state and blocks (PVP-1, PVP-RT-1), the party room, shared experience, wire.

### 2.2 GUILD-1

```yaml
task_id: OTV2-20261004-guild-1
decision: GUILD-0 §3.1-§3.5, §4.1, §4.2, §4.4; PREMIUM-ACTIVATION-0 §1.3 and §2.2; this decision §1.2
candidate_bases: []
worker: oteryn-hard-worker
review: hard, persistence, security and privacy review (Codex, final frozen head)
branch: allocated by the control plane
base: main after BANK-1 and PREM-WIRE-1 merge
migration_lease: one number from the control plane at allocation
event_type_lease: one number from the control plane at allocation
depends_on: [BANK-1, PREM-WIRE-1]
owned_paths:
  - apps/game-server/migrations/NNNN_guilds.sql              # GUILD-0 §3.1 tables, leadership, disband claims
  - apps/game-server/src/domain/guild.rs                     # pure rules: names, ranks, state and deadline gates
  - apps/game-server/src/domain/mod.rs                       # the mod line only
  - apps/game-server/src/durability/guild.rs                 # §3.2 operations, §3.3 jobs, §3.4 disband steps, lock order GUILD0-LO-01
  - apps/game-server/src/durability/guild_audit.rs           # the §4.4 guild event and activity log
  - apps/game-server/src/durability/mod.rs                   # the mod lines only
  - apps/game-server/tests/guild_postgres.rs
  - apps/game-server/tests/support/guild_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # the path-mod line only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # the §1.2 rows; the measured envelope and payload rows of the guild event
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json       # the leased guild event type, with payload_schema and payload_message (§1.12)
  - docs/contracts/game-events/v2/guild_event.proto          # GuildEventV1 and its activity entry messages (§1.12)
  - docs/agents/tasks/archive/OTV2-20261004-guild-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server guild
  - python3 tools/agents/validate_governance.py
```

Builds GUILD-0 §3 and §4, with the Premium rules of #1743 §2.2:

- found, invite, accept, decline, revoke, leave, kick, rank edits, promote, demote, pass
  leadership and disband;
- the state gate `GUILD0-RL-15` and the deadline gate `GUILD0-RL-18`;
- the World jobs: formation deadline, vice and Premium deficit grace, invitation expiry and
  disband, `GUILD0-RL-09` guilds per pass. The daily job calls `with_premium_batch_gate` before
  any guild lock;
- the junior predicate from BANK-1 for founding and leadership;
- the guild event and the activity log, under `GUILD_ACTIVITY_RETENTION_V1`.

Acceptance tests:

- every §3.2 operation commits under GUILD0-LO-01 and refuses out of order;
- each limit refuses at its value plus one;
- `FORMING` past its deadline cannot be activated or cured, and disbands at the next pass;
- a guild that loses its fourth vice keeps running for `GUILD0-RL-04`, then disbands;
- `NotActivated` needs no Premium and the job writes nothing; `NotCurrent` founding is refused
  `NOT_PREMIUM`; a lapse keeps the rank;
- a junior character cannot found or lead;
- disband steps 1 and 2 return done and change nothing;
- the activity log returns entries within `GUILD0-RL-10` only and purges by its retention.

Not in scope: guild bank, guildhalls, guild chat, nameplates on the wire, every wire message.

### 2.3 HOUSE-1a

```yaml
task_id: OTV2-20261004-house-1a
decision: HOUSE-OWN-0 §3 (property, tiles, slot), §4, §8, §9 (auction and settlement); this decision §1.3, §1.4 and §1.9
candidate_bases: [D3 (item order)]
worker: oteryn-hard-worker
review: hard, persistence and security review (Codex, final frozen head)
branch: allocated by the control plane
base: main after BANK-1, HOUSE-RUNTIME-1a and PREM-WIRE-1 merge
migration_lease: one number from the control plane at allocation
event_type_lease: one number from the control plane at allocation
depends_on: [OTV2-20261004-house-runtime-1a, BANK-1, PREM-WIRE-1]
owned_paths:
  - apps/game-server/migrations/NNNN_house_ownership.sql     # properties, tiles, housing slots, operations, bids and escrow; the game_house_access and game_house_access_tile bodies
  - apps/game-server/src/domain/house_auction.rs             # pure rules: proxy price, anti-sniping, eligibility
  - apps/game-server/src/domain/mod.rs                       # the mod line only
  - apps/game-server/src/durability/house_ownership.rs       # bid, settlement, release steps, §8 guard, §9 job
  - apps/game-server/src/durability/house_ownership_audit.rs # the house operation event
  - apps/game-server/src/durability/bank.rs                  # the house ledger kinds and the HOUSEOWN0-RL-15 headroom term only (§1.12)
  - apps/game-server/src/durability/mod.rs                   # the mod lines only
  - apps/game-server/tests/house_ownership_postgres.rs
  - apps/game-server/tests/support/house_ownership_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # the path-mod line only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # HOUSEOWN0-RL-01 to -06, -11, -13, -15, DUR03-RL-03-HOUSE; the measured envelope and payload rows of the house event
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json       # the leased house event type, with payload_schema and payload_message (§1.12)
  - docs/contracts/game-events/v2/house_operation.proto      # HouseOperationV1, its kinds and value lines, the house FeeBurnCause variants (§1.12)
  - docs/agents/tasks/archive/OTV2-20261004-house-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server house_ownership
  - python3 tools/agents/validate_governance.py
```

Properties and tiles are created from the active `content/houses/houses-*.json` catalogue: key,
kind, tiles, doors and `rent_gold`. Catalogue revisions are HOUSE-1b's (§1.8).

Acceptance tests:

- a bid reserves maximum plus the snapshotted rent from the bank balance with
  `HOUSE_BID_RESERVE`; a raise reserves only the difference; a lower down to the current price
  returns the difference with `HOUSE_BID_RELEASE`; a bid is never withdrawn;
- one bid per Account and World across all auctions;
- the proxy price is the second-highest maximum plus 1 gold, capped at the leader's maximum, or 0
  with one bidder; on equal maxima the earlier bid leads; a bid inside the last 15 minutes sets
  the end to its time plus 15 minutes;
- the 257th bidder is refused;
- settlement re-reads eligibility, excludes failing bids regardless of their maximum, burns the
  price and the first rent (`HOUSE_PRICE`, `HOUSE_RENT`), sets `paid_until` 30 days ahead, and
  releases every other escrow by steps of at most `HOUSEOWN0-RL-06` bids; no transaction exceeds
  `DUR03-RL-03-HOUSE`, and each house operation's ledger deltas, escrow change and burns sum to 0;
- `HOUSEOWN0-RL-15`: with a `HELD` bid, a bank deposit and a Market-style credit that would push
  balance plus escrow over the ceiling are refused `BALANCE_LIMIT`; a balance one gold below
  the limit still takes the full escrow return at release and at settlement;
- a crash between any two release steps resumes without a double release;
- one housing slot per Account and World;
- the §8 tile guard refuses a new Ground row on a house tile in every channel, and a settlement
  that finds a Ground row on the house's tiles waits and retries;
- the Premium cases of §1.4, with the gate before every lock.

Not in scope: rent, move-out, eviction, disposition (HOUSE-1b); access lists (HOUSE-ACL-1); wire.

### 2.4 HOUSE-1b

```yaml
task_id: OTV2-20261004-house-1b
decision: HOUSE-OWN-0 §3 (catalogue revisions), §5, §6, §7, §9 (rent and disposition jobs); this decision §1.3 and §1.8
candidate_bases: [MARKET-0 (CharacterInbox), D3 (item order)]
worker: oteryn-hard-worker
review: hard, persistence and security review (Codex, final frozen head)
branch: allocated by the control plane
base: main after HOUSE-1a, INBOX-1a and MAP-OVERLAY-1c merge
migration_lease: one number from the control plane at allocation
depends_on: [OTV2-20261004-house-1a, OTV2-20261004-inbox-1a, MAP-OVERLAY-1c]
owned_paths:
  - apps/game-server/migrations/NNNN_house_tenancy.sql       # rent due, grace, bans, dispositions, catalogue revisions
  - apps/game-server/src/domain/house_tenancy.rs             # pure rules: rent period, grace, notice
  - apps/game-server/src/domain/mod.rs                       # the mod line only
  - apps/game-server/src/durability/house_tenancy.rs         # rent, grace, move-out, eviction, disposition steps
  - apps/game-server/src/durability/house_ownership_audit.rs # the new operation kinds only
  - docs/contracts/game-events/v2/house_operation.proto      # the additive operation kinds and the disposition TRANSFER line only (§1.12)
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json       # one notes clause for the house event only
  - apps/game-server/src/durability/world_reset.rs           # the two catalogue revision calls only (§1.8)
  - apps/game-server/src/durability/mod.rs                   # the mod line only
  - apps/game-server/tests/house_tenancy_postgres.rs
  - apps/game-server/tests/support/house_tenancy_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # the path-mod line only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # HOUSEOWN0-RL-07 to -10, -14; the house event rows re-measured for the new kinds
  - docs/agents/tasks/archive/OTV2-20261004-house-1b.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server house_tenancy
  - python3 tools/agents/validate_governance.py
```

Acceptance tests:

- rent is charged 30 days in advance with `HOUSE_RENT`; a missed charge starts the 7 day grace,
  retried daily and at its end;
- the rent warning function returns the warning for the owner while rent is unpaid (showing it at
  login is HOUSE-WIRE-1's, §1.12);
- grace end evicts and bans the Account on that World for 30 days;
- move-out with 1 to 30 days notice; outside that range it is refused;
- the disposition fence stops every interior change, then moves the interior to the owner's
  CharacterInbox by steps of at most `HOUSEOWN0-RL-06` items, in D3 order, and a crash between
  steps resumes without loss or duplication;
- each §3 catalogue revision case at a reset: a new house, a retired `VACANT` house, a cancelled
  auction with every escrow returned, and the preflight refusal for an owned house;
- a house won before HOUSE-1b whose `paid_until` has passed is charged once at the first pass and
  never evicted for the gap (§1.3).

Not in scope: the MAIL-0 rent letter, the BED-0 sleeper release, guildhalls, wire.

### 2.5 MAP-OVERLAY-1a

```yaml
task_id: MAP-OVERLAY-1a
decision: ADR-0021 §4.4 (overlay, tile limit, Ground rebuild, budget), §4.8; this decision §1.5
candidate_bases: []
worker: oteryn-hard-worker
review: persistence review (ADR-0021 §4.8)
branch: allocated by the control plane
base: main after MAP-LOAD-1 merges
migration_lease: none
depends_on: [MAP-LOAD-1]
owned_paths:
  - apps/game-server/src/map/overlay.rs                      # per-channel overlay and per-tile hidden bitmask
  - apps/game-server/src/map/overlay/**                      # expiry index, budget, Ground rebuild
  - apps/game-server/src/map/mod.rs                          # the mod line only
  - apps/game-server/tests/map_overlay_*.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # MAP01-CHANNEL-OVERLAY-BYTES: measured value and evidence
  - docs/agents/evidence/MAP-OVERLAY-1a-*.md
  - docs/agents/tasks/archive/MAP-OVERLAY-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server map_overlay
  - cargo run --locked -p oteryn-architecture-check
  - python3 tools/agents/validate_governance.py
```

Acceptance tests:

- two channels of one World share one base and keep separate overlays;
- hiding and adding items on a tile, up to the 64 item reach, and a 65th base item refused at
  load;
- a base stack is never merged in the overlay;
- a volatile item is removed by the expiry index within 1 s of its decay;
- a new volatile entry over the budget is refused atomically; a durable Ground item over it is
  counted and alarmed, never refused;
- after a restart the channel rebuilds every durable Ground item; one with a different
  `map_revision` fails the rebuild closed.

Not in scope: map item MINT, the reset record, live wiring into the channel loop (MAP-CUTOVER-1).

### 2.6 MAP-OVERLAY-1b

```yaml
task_id: MAP-OVERLAY-1b
decision: ADR-0021 §4.4 (picking up a map-authored item); DUR-03 "Map items and world reset" (MINT cause) and §39.3; this decision §1.5 and §1.11
candidate_bases: []
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main after MAP-OVERLAY-1a merges
migration_lease: one number from the control plane at allocation
operation_tag_lease: the next free OneItemTransactionV1 tag, from the control plane at allocation
depends_on: [MAP-OVERLAY-1a]
owned_paths:
  - apps/game-server/migrations/NNNN_map_item_materialization.sql  # MINT reservations and receipts keyed by the full cause
  - apps/game-server/src/durability/map_item_mint.rs
  - apps/game-server/src/durability/map_item_mint_audit.rs   # the map_item_mint event (§1.11)
  - apps/game-server/src/durability/mod.rs                   # the mod lines only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto # the new operation and its messages only
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json       # one type 2 clause only
  - apps/game-server/src/map/overlay/pickup.rs               # eligibility, hide at freeze, re-hide at rebuild
  - apps/game-server/src/map/overlay.rs                      # the mod pickup line only
  - apps/game-server/src/durability/item_mint_audit.rs       # the new OneItemOperationV1 arm and tag 8 in the oneof tag list only (§1.11)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # DUR03-RL-07-* consumer line and the new worst-case boundary test only
  - apps/game-server/tests/map_item_mint_postgres.rs
  - apps/game-server/tests/support/map_item_mint_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # the path-mod line only
  - docs/agents/tasks/archive/MAP-OVERLAY-1b.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server map_item_mint
  - cargo test --locked -p oteryn-game-server map_overlay
  - python3 tools/agents/validate_governance.py
```

Acceptance tests:

- an eligible entry is minted into Ground at its tile and then transferred; each ineligible kind
  of §4.4 stays in place;
- the `map_item_mint` event round-trips with its full cause, and the audit refuses it with a
  missing or extra cause field; the existing `loot_mint` and reward-claim tests still pass;
- the worst-case `map_item_mint` payload and envelope fit `DUR03-RL-07-*`;
- the same entry is taken once per channel and reset epoch; a second channel can take it too;
- a retried MINT returns the existing item and still checks reach before the TRANSFER;
- the origin hides at freeze and unhides only on proven non-commit;
- after a crash the rebuild re-hides every origin with a receipt for this channel, digest and
  epoch, even over the budget (counted and alarmed).

Not in scope: reset retirement, the reset record.

### 2.7 MAP-OVERLAY-1c

```yaml
task_id: MAP-OVERLAY-1c
decision: ADR-0021 §4.7; HOUSE-CUSTODY-0 §3.6; DUR-03 "Map items and world reset" (WorldReset retirement) and §39.3; this decision §1.6, §1.7 and §1.11
candidate_bases: []
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main after MAP-OVERLAY-1b and HOUSE-RUNTIME-1a merge
migration_lease: one number from the control plane at allocation
depends_on: [MAP-OVERLAY-1b, OTV2-20261004-house-runtime-1a]
owned_paths:
  - apps/game-server/migrations/NNNN_world_reset.sql         # reset record; widening of 0015's tables (§1.7)
  - apps/game-server/src/durability/world_reset.rs           # steps 1-4, crash recovery, preflight, step-4 recheck
  - apps/game-server/src/durability/item_decay_retire.rs     # the cause_kind column in its reads and writes only
  - apps/game-server/src/durability/item_decay_retire_audit.rs # the WorldReset cause and the Ground container entry only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto # the additive tag 5 fields only (§1.11)
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json       # one type 2 clause only
  - apps/game-server/src/durability/mod.rs                   # the mod line only
  - apps/game-server/tests/world_reset_postgres.rs
  - apps/game-server/tests/support/world_reset_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # the path-mod line only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # MAP01-RESET-RETIRE-MS: measured value and evidence; DUR03-RL-07-* consumer line and new worst case
  - docs/agents/evidence/MAP-OVERLAY-1c-*.md
  - docs/agents/tasks/archive/MAP-OVERLAY-1c.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server world_reset
  - cargo test --locked -p oteryn-game-server corpse_decay
  - python3 tools/agents/validate_governance.py
```

Acceptance tests:

- steps 1-4 in order: admission closes, each channel scope gets a fresh ownership generation,
  every live Ground item is retired (container entries first), then activation;
- an in-flight Ground, MINT or TRANSFER commit of the old generation fails its fence;
- a crash at each step resumes from step 2, and the old bundle never boots over a half-retired
  Ground;
- an item with a `CorpseDecay` reservation of the old generation gets a fresh `WORLD_RESET`
  reservation under the new generation; the old row cannot commit, and no item has two receipts;
- the retirement event round-trips with `world_reset` and with `ground_container_entry`; one with
  both causes, neither cause, a Ground container entry under the corpse cause, or a `world_reset`
  whose `item_instance_id` differs from the retired item is refused; every existing
  `CorpseDecay` event still decodes and validates unchanged; the worst case fits `DUR03-RL-07-*`;
- existing `0015` rows read as `CORPSE_DECAY` after the migration and keep their CHECKs;
- the preflight refuses a target bundle that fails HOUSE-CUSTODY-0 §3.6, before step 1, and,
  until HOUSE-1b, one that changes the house catalogue on a World with property rows (§1.8);
- the step-4 recheck runs under the §1.6 locks; a concurrent HouseInterior insert waits for the
  activation commit;
- an owned house's interior survives the reset; Ground on an unowned house tile is retired;
- `MAP01-RESET-RETIRE-MS` is measured and registered.

Not in scope: booting a World from a bundle, the CI artifact job.

### 2.8 MAP-CUTOVER-1

```yaml
task_id: MAP-CUTOVER-1
decision: ADR-0021 §4.7 (first cutover), §5; MAP-LOAD-PACKET-1 §2.2 (the pin)
candidate_bases: [CHAR-POSITION-0]
worker: oteryn-hard-worker
review: hard and persistence review (Codex, final frozen head)
branch: allocated by the control plane
base: main after MAP-OVERLAY-1c merges
migration_lease: none
depends_on: [MAP-OVERLAY-1c]
owned_paths:
  - apps/game-server/src/world_runtime.rs                    # boot from the pinned bundle; overlay wiring into the channel loop
  - apps/game-server/src/map/boot.rs                         # first boot runs the reset
  - apps/game-server/src/map/mod.rs                          # the mod boot line only
  - apps/game-server/tests/map_cutover_*.rs
  - docs/agents/tasks/archive/MAP-CUTOVER-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server map_cutover
  - cargo run --locked -p oteryn-architecture-check
  - python3 tools/agents/validate_governance.py
```

Acceptance tests:

- the first boot of a World from a bundle runs the §4.7 reset: every pre-cutover Ground item,
  with the legacy map revision, is retired, then the bundle activates;
- a restart outside a reset reuses the active bundle and never switches revisions;
- a durable Character position that the active bundle does not hold falls back at admission.

Not in scope: the CI artifact job and the production pin. Running a cutover on a protected or
production World is separate authority.

## 3. Rejected options

- **One HOUSE-1.** It would exceed the batch limit and hold acquisition on INBOX-1, which only
  the disposition needs.
- **One MAP-OVERLAY-1.** The overlay, the MINT cause and the reset are three persistence reviews
  of separate tables; one PR would be too large to review.
- **A row lock for the step-4 recheck.** A row lock cannot block an insert of a row that does not
  exist yet (§1.6).
- **Adopting an older `CorpseDecay` reservation at reset.** Its generation has ended, and `0015`
  requires a later generation to reserve afresh (§1.7).
- **Reusing the `loot_mint` operation for map items.** Its audit requires a creature death
  occurrence; a separate operation follows the reward-claim precedent (§1.11).
- **A new retirement table for `WorldReset`.** ADR-0021 §4.7 requires the reset to share per-item
  uniqueness with `CorpseDecay`; two tables could retire one item twice (§1.7).
- **A PvP stub table in PARTY-1.** It would create PVP-1's table under the wrong owner (§1.1).
- **GUILD-1 after INBOX-1.** No guild operation writes to the inbox, so the wait only delays it
  (§1.2).

## 4. Decision test

1. **What does this decide?** The detailed packets for PARTY-1, GUILD-1, HOUSE-1a, HOUSE-1b,
   MAP-OVERLAY-1a, -1b, -1c and MAP-CUTOVER-1; the HOUSE-1 and MAP-OVERLAY-1 splits; the step-4
   recheck lock; the private house escrow headroom; the first rent gap; catalogue revisions inside the reset; the shared retirement uniqueness; one Premium consumer row; the GUILD-1
   dependency correction; the guild and house event schemas and the owned-path sweep (§1.12).
2. **What does it not decide?** No candidate base is accepted. No code, migration, registry row,
   event type or wire is added by this PR.
3. **What is unblocked?** Each packet becomes allocatable when its §0.1 dependencies merge.
4. **What returns to the architect?** A packet whose candidate base changes, a CHAT-2 payload
   without a variant point (§1.1), or a measured budget above its ADR value.
