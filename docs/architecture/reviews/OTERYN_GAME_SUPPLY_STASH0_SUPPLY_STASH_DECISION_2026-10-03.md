# SUPPLY-STASH-0 Supply Stash

- Decision: `SUPPLYSTASH0-SUPPLY-STASH-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  value conservation and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - DEPOT-0 §10 and its implementation brief ("Later: the Stash");
  - MARKET-0's implementation brief and §14 (the Stash as an offer source is later work);
  - IMBUE-FORGE-0's implementation brief, §3 and §13 (Stash materials, and tiered items in the
    Stash: "decided with the Stash"; §4 refuses them);
  - owner answer 2b (#162 5912593702: "depozyt i stash są dostępne z każdego miasta");
  - control-plane allocation D293 (#1622).
- Builds on:
  - DEPOT-0 §3-§5 (the locker, the depot capacity guard, the depot view, the one-item shapes);
  - DUR-03 §11.3, §14, §17, §18, §38 and §39.1;
  - BANK-0 §3-§5 (the §18 balance, ledger, operation and conversion idiom);
  - MARKET-0 §3.1 and its IMBUE-FORGE-0 amendment (wares and the default state);
  - TIMED-ITEM-0 §4 (the timed-row table);
  - PREMIUM-ACTIVATION and PREMIUM-DELIVERY-0 (the Premium evidence, fail closed);
  - the composition decision rules 1, 2 and 4;
  - owner rule 5905825574.
- Amends, each pending on acceptance of SUPPLY-STASH-0 and written by STASH-1 in its own docs
  commit:
  - DUR-03 §18 (the stash asset) and the §38 depot row;
  - DEPOT-0 §3 (the capacity count);
  - the composition decision rule 1.
- Runtime, migration and production authority: NONE. Each child needs its own #162/#1622
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| STASH-1 | hard (persistence), persistence and value review | the stash balance, ledger and operation tables (§3), the stow and withdraw conversions (§5), the counted depot capacity (§6), the DUR-03 lines and event (§7) | DEPOT-1; BANK-RET-0 (the `ECONOMY_LEDGER` retention profile); ITEM-MOVE-2a |
| STASH-WIRE-1 | impl, protocol review | capability `STASH_V1`, the Stash entry in the locker view, the paginated stash view, the `STASH` destination and the withdraw request (§8) | DEPOT-WIRE-1; STASH-1 |
| STASH-CONTENT-1 | content lane | the per-definition `stowable` fact (§4), derived from MARKET-CONTENT-1's wares | MARKET-CONTENT-1 |

Later, each with its own decision or amendment: stow from the Inbox, "stow all items of this
type", stow a container's contents, routing withdrawals through Manage Loot Containers, the
category, name and trader filters, the Market selling from the Stash, imbuing materials from the
Stash, the Steward hireling, the Cyclopedia item summary and depot search.

## 1. Question

How does a character keep large amounts of supplies in its locker, beyond the depot boxes?

## 2. Facts

**PROVEN**

- DEPOT-0 (candidate): the locker is a `container_fixture` world object; its view lists 17 fixed
  boxes; depot capacity is guarded per character (`DEPOT0-RL-01` = 15,000 hard, 2,000 free or
  15,000 Premium); the depot is Character + World, reachable from every locker of the World.
- DUR-03 §18: a non-item balance needs an owning domain, exact bounded arithmetic, explicit mint or
  burn or a versioned conversion rule. BANK-0 is the first such asset; its deposit and withdraw are
  `CONVERSION` lines with no BURN and no MINT.
- MARKET-0 §3.1: a ware is an Item definition with `trade.marketable = true`, offered only in its
  default state (no changed charges, duration, text or contents; tier 0 and no imbuement after
  IMBUE-FORGE-0).
- TIMED-ITEM-0 §4: an item with spent duration or charges has a row in `game_item_timed_states`.
- No Game table holds Premium yet; Premium-gated actions read PREMIUM-ACTIVATION evidence, fail
  closed, and are refused until PREM-3 delivers Premium (OFFLINE-0 §6).

**CIPSOFT_OFFICIAL** (the Tibia manual, `world.md` line 47)

- The Stash is the locker's third compartment: "unlimited-count storage for most Market-tradeable
  items". It excludes Store-purchased items, partially used items (for example time-limited rings)
  and altered items (for example imbued gear). It has search, sort and filter, including "sellable
  to a given NPC".
- Items go in by "Stow", "Stow all items of this type" or drag-and-drop, from backpacks, the depot
  chest and the Inbox.
- Imbuing materials may come from the backpack or the Stash (`characters.md` line 120). Stash
  retrieval follows the loot container routing for Premium (`controls.md` lines 83-85).

**TIBIAWIKI_STRUCTURED** ("Your Supply Stash" rev 1057911, item 28750, introduced in 11.80; Depot
rev 1057910)

- Only Premium players can add items; anyone can retrieve them after Premium ends.
- Each item type is shown as one stack, whatever its amount.
- Stash items count toward the depot item limit in increments of 100 per type: 100 units count 1,
  101 units count 2.
- On a Market sale, items are taken from the Inbox, then the depot boxes, then the Stash.
- Retrieval chooses an amount and goes to a Manage Loot Containers container or the main backpack.
- The page says "only items that can be stacked". The official manual says "most Market-tradeable
  items" (R1).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b5`)

- `player_stash` is keyed by (player, item id) with a count (`schema.sql:791-797`): a count per
  type, not item instances.
- `isItemStorable` refuses Store items, owned items, decaying items, and admits a market item with
  no tier that is not a container (`item.cpp:905-911`, `item.hpp:491-494`). A container with
  contents stows its contents.
- Withdraw is capped at 100,000 units per request (`stashCountByEachTime`) and needs 100 free
  capacity (`game.cpp:5706-5753`).

## 3. Storage (STASH-1)

- **Asset.** The Stash is DUR-03 §18 non-item value: an integer number of units per Item
  definition. A stowed item stops being an ItemInstance; a withdrawn item is a fresh one. The
  conversion rule is the identity on units, one unit per stack unit, under the definition's
  revision (DUR-03 §46).
- **The asset key** is the pair (`definition_key`, `definition_revision`): the ItemTypeKey and the
  GAME-ITEM definition revision of the stowed item. The balance, every ledger entry, every item
  line, the value line's asset (`stash:<definition_key>@<definition_revision>`) and every wire
  reference (§8) carry both. Two revisions of one key are two assets, never one row.
- **Revision compatibility (DUR-03 §46).** A stow credits the row of the item's own revision. A
  withdraw mints under the row's revision, and only while the active content declares that revision
  compatible for the key; otherwise the row is `MIGRATION_REQUIRED`: it is shown, but a withdraw
  refuses it (`NOT_SUPPORTED`) and nothing reinterprets it. Moving units from an old revision to a
  new one needs a migration decision that writes ledgered entries for both assets; STASH-1 adds no
  such path.
- **Balance.** `game_character_stash_balances`: one row per (`character_id`, `world_id`,
  `definition_key`, `definition_revision`) with `quantity` (BIGINT, 0 to `SUPPLYSTASH0-RL-01`) and
  `last_entry_id`. No row means 0. A row is inserted only by a committed stow. `character_id` references the Character root with RESTRICT, so a future deletion
  workflow must empty the Stash first.
- **Operation.** `game_character_stash_operations`: one row per operation, keyed by its occurrence,
  with the TransactionId, a SHA-256 binding of the whole request (kind, character, source or
  definition, amount, planned output slots) and the outcome. The same occurrence and binding
  replay the first outcome; a changed binding conflicts (BANK-0 §3).
- **Ledger.** `game_character_stash_entries`: one immutable row per balance change: `entry_id`
  (UUIDv7), operation, character, world, definition key and revision, kind (`STOW`, `WITHDRAW`), amount (> 0),
  quantity before and after.
- **Item lines.** `game_character_stash_item_lines`: one row per touched item: operation, ordinal,
  item, definition key and revision, quantity, direction (input or output). The entry-removal, mint-guard and
  placement proofs of `0010`-`0012`, `0023` and DEPOT-1 gain a branch that accepts a line of a
  committed stash operation.
- **Guards** (deferred constraint triggers): the balance equals the after value of its latest
  entry, and each entry's before equals the previous after; the units of an operation's input lines
  equal its credit, and the units of its output lines equal its debit; an operation has exactly the
  entries and lines its kind needs; the character is a live root of the entry's World.
- **No cross-asset conversion** (a commit-time guard): an operation has exactly one entry, on
  exactly one balance row; the entry's (`definition_key`, `definition_revision`) equals that row's;
  and every item line of the operation has that same pair. For a stow, the pair is also the input
  item's own definition and revision, read under its lock. A stow of one definition can never credit
  another, and a withdraw can never mint a definition or revision other than the debited one.
- **Grants.** `oteryn_game_runtime`: SELECT and INSERT on operations, entries and lines; SELECT,
  INSERT and UPDATE on balances; never DELETE. `oteryn_game_control`: SELECT.
- **Scope.** Character + World, with no channel and no runtime scope: every locker of the World
  opens the same Stash (owner answer 2b). Game owns it. Platform never writes it; Atlas may read an
  export.
- **Lifetime.** Never touched by death, `WorldReset`, channel changes or the end of Premium.

## 4. What can be stowed (STASH-CONTENT-1, STASH-1)

An item is stowable only when all of these hold:

- its definition is a Market ware (`trade.marketable = true`) and STASH-CONTENT-1 marks it
  `stowable`; containers are never `stowable`;
- the instance is in its default state (MARKET-0 §3.1 with the IMBUE-FORGE-0 amendment): tier 0,
  no imbuement, no text, no contents;
- it has no row in `game_item_timed_states` (no spent duration or charges);
- it is not Store-sourced. No Store delivery exists yet; the decision that adds one names its
  marker, and STASH-1's check reads it from then on.

Anything else is `NOT_SUPPORTED`. Stackable and non-stackable wares are both admitted (R1).

## 5. Operations (STASH-1)

### 5.1 Common rules

- Each operation is one PostgreSQL transaction with one TransactionId and its planned output
  slots fixed before the first attempt (DUR-03 §11.3, §20, §23.1).
- The locker view of DEPOT-0 §4.1 is open and the character stands next to that locker when the
  command runs (re-checked after a reconnect, FND-02 §13.3).
- **Fence.** Composition rule 2 with the acting Character's `character_root` lock; no runtime
  scope owns the Stash, so rule 2's §32 binding does not apply.
- **Lock order** (rule 4, extended): `character_root`, then the items in ItemInstanceId order, then
  the container-slot row, then the stash balance row.
  - A withdraw locks the existing row FOR UPDATE; an absent row is a refusal, and nothing is
    inserted.
  - A stow inserts the row if absent (`ON CONFLICT DO NOTHING`), then locks it FOR UPDATE; the
    insert is part of the stow and commits only with it.
- No `CharacterRevision` advance (§9).
- **A refusal writes nothing.** A refused operation rolls back its whole transaction, any
  provisional balance row included, and persists no zero-balance row. This departs from BANK-0
  §4.1's zero row on purpose: a Stash refusal has no balance to fence.

### 5.2 Stow

- **Source.** One whole item: a main backpack direct entry or a depot box entry (DEPOT-0 §3).
  The whole stack is stowed; partial counts come later.
- **Premium.** The character is Premium (PREMIUM-ACTIVATION evidence, fail closed; refused until
  PREM-3 delivers Premium). A non-Premium character gets `SEALED` with the Premium message id.
- **Capacity.** The counted depot total after the stow (§6) stays within the account limit,
  otherwise `NO_ROOM`.
- **Effect.** The item leaves its location and retires; the balance gains its quantity (`STOW`).

### 5.3 Withdraw

- **Request.** A row handle (§8) and an amount `n`, with `1 <= n <= balance`. Premium is not needed.
- **Outputs.** Fresh default-state ItemInstances in new main backpack direct entries: stacks of the
  definition's maximum stack size (100) for a stackable ware, one item per unit otherwise. At most
  `SUPPLYSTASH0-RL-02` (20) outputs, so at most 2,000 stackable units or 20 items per withdraw.
  Too few free entries refuses the whole withdraw (`NO_ROOM`). There is no top-up of existing
  stacks in this slice.
- **Effect.** The balance loses `n` (`WITHDRAW`).

## 6. Depot capacity (amends DEPOT-0 §3)

- The counted depot total of a character is its depot entries plus, for each stash balance,
  `ceil(quantity / 100)`.
- DEPOT-1's deferred guard counts both, under the `character_root` lock it already holds, against
  `DEPOT0-RL-01`. The runtime applies the account limit to stows into the depot and into the Stash
  alike.
- Withdrawing is always allowed, as taking items out of the depot is.

## 7. DUR-03 classes, cause and event (STASH-1)

- **Classes (§17).** A stow's input item and a withdraw's output items are `CONVERSION` lines; the
  balance side is a `CONVERSION` value line. There is no BURN and no MINT, so §15's closed list of
  sinks is unchanged.
- **Cause.** Closed `StashConversionCause {Stow | Withdraw, occurrence}`.
- **Value line.** A closed message: entry, asset (`stash:<definition_key>@<definition_revision>`), character, World,
  kind, class, amount, quantity before and after.
- **Event.** One stash event per operation, carrying its item and value lines, in a stash outbox,
  under the `ECONOMY_LEDGER` purpose that BANK-RET-0 defines.
- **Supersession.** For the stash shapes only, the §39.1 exclusions of non-item accounts and
  multiple touched items, and the §39.1 and §39.3 source limits (a depot box source). Every other
  §39 obligation is unchanged; `DUR03-RL-03` stays 0 for every existing shape.

## 8. Wire (STASH-WIRE-1)

- **Capability `STASH_V1`**, number reserved by the control plane before STASH-WIRE-1, requiring
  `DEPOT_V1`. Without it, the locker view shows no Stash.
- The locker view lists the Stash beside the 17 boxes, with its row count.
- **Pages.** A Stash target `StashPageTargetV1 {page}`, a new `UseIntentV1` field whose number
  STASH-WIRE-1 assigns under protocol review, opens one page in domain 11, as a box target does
  (DEPOT-0 §4.1).
  - A page holds at most `DEPOT0-RL-02` (32) rows, ordered by (`definition_key`,
    `definition_revision`); the server reads at most `DEPOT0-RL-03` (1) page per open.
  - Each row carries `{definition_key, definition_revision, quantity, migration_required}` and a
    handle bound to the row's `last_entry_id`.
  - A page beyond the last returns no rows; `has_more` is true exactly when a later page holds rows.
- **Stale handles.** A handle becomes `STALE` when the view closes (DEPOT-0 §4.1) and when its row's
  `last_entry_id` changes. A withdraw on a stale handle is refused `STALE` and writes nothing; the
  client reopens the page. The view's `expected_revision` is checked as for a box page.
- **Stow.** Command 9 gains the destination `STASH`; its source is a main backpack entry handle or
  an entry on the open box page.
- **Withdraw.** A new domain 11 request `StashWithdrawV1 {handle, amount}` while the Stash page is
  open. The handle resolves to the row's (`definition_key`, `definition_revision`) on the server;
  the client never names a definition on its own.
- Results: `NO_ROOM`, `NOT_SUPPORTED`, `SEALED`, `STALE` and the existing ones. The view closes as DEPOT-0
  §4.1 says.

## 9. Other amendments

- **DUR-03 §18.** After the BANK-0 amendment, the Stash is the second non-item asset, owned by Game
  per Character + World, with the §7 shapes and cause.
- **DUR-03 §38.** The depot row gains: "the Stash: §18 value per definition, converted to and from
  whole items by `StashConversionCause`".
- **Composition rule 1.** An operation between the acting character's backpack or depot and its
  Stash does not advance `CharacterRevision`.

## 10. Rows (registered by STASH-1 and STASH-WIRE-1)

| Row | Value |
|---|---|
| `SUPPLYSTASH0-RL-01` units per definition | 1,000,000,000 (an engineering bound; Tibia shows no limit) |
| `SUPPLYSTASH0-RL-02` outputs per withdraw | 20 |
| Stash units per depot count | 100 (`PARITY_PENDING`, TibiaWiki) |
| Stash rows per page | `DEPOT0-RL-02` (32) |
| Stash pages read per open | `DEPOT0-RL-03` (1) |

## 11. Rejected options

- **Stash entries as ItemInstances in a custody family.** Each stow would keep an identity that
  Tibia does not keep, rows would grow without bound, and "one stack per type" would need merges.
- **Stackable items only.** TibiaWiki says so; the official manual says "most Market-tradeable
  items", and Canary admits non-stackable wares (R1).
- **Stow from anywhere.** No evidence that Tibia allows it; the Steward hireling is the only
  official remote access (R3).
- **Unbounded withdraw.** Canary allows 100,000 units at once; a bounded transaction needs a fixed
  output count.
- **A Stash without the Premium gate until PREM-3.** It would differ from Tibia (R2).

## 12. Architect rulings (owner rule 5905825574)

- **R1, eligibility: a) Market wares in default state, stackable or not** (official manual and
  Canary). b) Stackables only (TibiaWiki). c) Every item. Recommendation and ruling: a).
- **R2, Premium: a) stow needs Premium, fail closed until PREM-3; withdraw never needs it**
  (TibiaWiki). b) Open to all until PREM-3. Recommendation and ruling: a). Until PREM-3 the Stash
  ships and is tested with a test Premium source but stays closed to stows in play.
- **R3, where: a) at a locker only**, `PARITY_PENDING`. b) From anywhere. Recommendation and
  ruling: a).
- **R4, capacity: a) `ceil(quantity / 100)` per definition counts toward the depot limit**
  (TibiaWiki, `PARITY_PENDING`). b) Not counted (the manual's "unlimited-count"). Recommendation
  and ruling: a); "unlimited" reads as "no per-type count limit".

## 13. Owner questions

None. R2 follows Tibia; PREM-3 opens stowing with no further decision.

## 14. Decision test

- **Must decide now:** YES. Control-plane allocation D293; DEPOT-0, MARKET-0 and IMBUE-FORGE-0
  each defer a rule to this decision.
- **Blocked without it:** STASH-1 and STASH-WIRE-1; the Market and imbuing Stash sources.
- **Harder later:** YES. Once items are stored as instances, moving them to per-type value is a
  data migration with identity loss.
- **Supersede if:** official evidence on stowing from anywhere, the exact capacity count, or a
  Stash limit per type.
- **Deliberately not decided:** stow from the Inbox, "stow all", container stow, loot container
  routing, filters and search, Market and imbuing sources, the Steward, the Cyclopedia summary,
  depot search.

## 15. Before-freeze checklist

1. **Contract amendments:** DUR-03 §18 and §38, DEPOT-0 §3, composition rule 1; each pending on
   acceptance, written by STASH-1.
2. **Serialization:** one transaction per operation, Character fence and root lock, balance row
   last.
3. **Restart:** balances and ledger are durable; a replayed occurrence returns the first outcome.
4. **Typed references:** CharacterId, WorldId, definition key, amount, handles.
5. **Wire:** §8, capability `STASH_V1`.
6. **Split work:** one source item per stow; at most 20 outputs per withdraw; one page per view.
