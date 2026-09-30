# MARKET-0 World Market

- Decision: `MARKET0-WORLD-MARKET-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, security and protocol) and protected integration. It builds on BANK-0 (PR #1357),
  BANK-FEE-0 (PR #1361) and DEPOT-0 (PR #1359) and integrates after them. Owner questions Q1-Q3
  (§13) are answered (#162 5913348961).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to start the Market ("pełna zgodność handlu z Tibią",
  2026-09-30)
- Builds on: the scope matrix Market row ("Market service/domain, World, strong durable, one
  economy across all channels, shared"); DUR-03 §7.1, §11.3, §12, §15, §17, §18, §28, §34, §38,
  §39 and §46; the gold fee decision §4.2-§4.4 (D174-D178); BANK-0 (balance, ledger, guards,
  junior rule and Q1); BANK-FEE-0 (`FEE_DEBIT`); DEPOT-0 (`CharacterDepot`, `DEPOT_V1`, §6 deltas);
  the composition decision §3; FND-ID-01 (`MarketOfferId`); FND-02 §13.3; PROD-ENTITLEMENTS-01 §9;
  owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of MARKET-0 (#162 5912405163): DUR-03 (the Market custody
  families and shapes, a paragraph after the §38 table); the composition decision (a paragraph
  before its §7). The BANK-0, BANK-FEE-0 and DEPOT-0 amendments of §8 are written into those
  documents by MARKET-1 once they are on `main`. The DUR-03 §39.3 and gold fee §4.4 fee-source
  amendment is admitted by the Q1 answer and written by MARKET-1.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| MARKET-CONTENT-1 | content lane | `trade.marketable` and `trade.market_category` for every Item, from the client appearances' market data (§3.1) | none |
| MARKET-RET-0 | control plane routes; privacy review | the retention profile of the market event (purpose `ECONOMY_LEDGER`) | this decision |
| INBOX-1 | hard, persistence review | the `CharacterInbox` family, its counter and its two out-shapes (§5) | DEPOT-1 |
| MARKET-1 | hard, persistence, economy and security review | operations, offers, escrow, the book lock, place, accept, cancel, expiry and matching steps, the market event and every delta of §8 | BANK-1; GOLD-FEE-2; MARKET-RET-0; INBOX-1; MARKET-CONTENT-1 |
| MARKET-WIRE-1 | impl, protocol review | capability `MARKET_V1`, the market commands and domain, the Inbox view (§10) | DEPOT-WIRE-1; MARKET-1 |

Later, each with its own decision: Tibia Coin offers (**MARKET-COINS-0**, owner answer Q2b, with a
cross-repository contract with Oteryn-Platform), 30-day price statistics and the
unfair-offer highlight, `trade_as` ware merging, the Stash as an offer source, bags with contents.

## 1. Question

How do players of one World buy and sell items through the Market, safely across channels?

## 2. Facts

**PROVEN**

- The scope matrix: the Market is World-scoped, strong durable, one economy across all channels.
- DUR-03 §34: a workflow may span transactions only through explicit typed custody, each step
  conservation-safe, idempotent and restartable; §38: DUR-03 owns Market conservation and escrow,
  the Market owns offers, fills, fees and pricing; §12: a split gives the moved part a fresh
  transaction-scoped identity (§11.3), class `SPLIT_MERGE_QUANTITY`; §28: every shape needs hard
  ceilings before implementation; §15 and D178: a new fee source needs its own owner decision.
- BANK-0 (candidate): one balance per (Account, World) up to `BANK0-RL-01`; ledger entries belong
  to a bank operation whose acting character is a live root of the entry's account; junior
  characters without bank use (owner answer b, #162 5913348961). BANK-FEE-0 (candidate): fees pay coins
  first, the remainder as a `FEE_DEBIT` ledger entry referencing the fee record.
- DEPOT-0 (candidate): `CharacterDepot` rows are immutable and leave only by TRANSFER; whole-item
  shapes; the Inbox is left to a later decision.
- FND-ID-01 names `MarketOfferId` as a Game identity.
- Content: `trade.marketable` and `trade.market_category` exist in the item schema; five records
  know `marketable: false` and every other Item leaves it `UNKNOWN`.
- PROD-ENTITLEMENTS-01 §9: no Game table holds Premium; the Premium source answers Free until the
  consumer contract is accepted.

**CIPSOFT_OFFICIAL** (the Tibia manual, `controls_trading.md` §4.3.3; junior accounts: `world.md`)

- The Market is opened from the depot locker. Only Premium characters place offers; free ones may
  accept offers (and may place Tibia Coin offers).
- At most 100 open offers per character; 64,000 items per offer; a price per offer up to
  999,999,999,999.
- Placing an offer costs 2% of the offer price, at least 20 and at most 1,000,000 gold, taken from
  the bank at once and never refunded.
- A sell offer takes its items from the depot chest; a buy offer takes the fee and the full price
  from the bank. A crossing offer is matched at once.
- Accepting a sell offer needs the full price in the bank; accepting a buy offer needs the items
  in the depot chest. Items go to the buyer's Inbox, gold to the seller's bank.
- Offers end after 30 days; unsold items return to the Inbox, unspent gold to the bank.
- Refused wares: inscribed documents, liquid containers, keys, non-portable items and containers
  with contents. My Offers shows current offers and the last 600 ended ones. An offer may be
  anonymous.
- Junior characters cannot sell on the Market.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Wares are the client appearances' market ids; an item with changed attributes (charges,
  duration, text) is refused; a player cannot accept its own offer; accepted stackables are
  delivered as new stacks of at most 100.

## 3. Wares and prices

### 3.1 Wares (MARKET-CONTENT-1)

- A ware is one Item definition with `trade.marketable = true`. `UNKNOWN` and `false` are not
  wares: the Market lists a ware in its `market_category`, so a definition without one cannot be
  shown (stricter than PLAYER-TRADE-0's `tradeable: UNKNOWN`, which needs no category).
- MARKET-CONTENT-1 sets both fields from the client appearances' market data, with Canary's item
  table as corroboration. The manual's refused categories are never marketable.
- An item instance is offered only in the definition's default state (the gold fee §4.2 notion):
  no changed charges, duration, text or contents.
- `trade_as` merging is deferred; each definition key is its own ware.

### 3.2 Numbers

- `amount` 1 to 64,000 (`MARKET0-RL-02`); `piece_price` 1 to 999,999,999,999; `total =
  piece_price × amount` in checked 128-bit arithmetic, refused above 999,999,999,999 (the
  architect reads the manual's "max price per offer" as this total).
- Fee: `clamp(floor(total / 50), 20, 1,000,000)` (`MARKET0-RL-04`; rounding `PARITY_PENDING`).

## 4. Offers (MARKET-1)

- **Operation.** `game_market_operations`: one row per place, accept, cancel, expiry step or
  matching step, keyed by its occurrence: a player command's occurrence bound 1:1 to its
  CommandRef, or (offer, step kind, step number) for a job step; with the TransactionId, a SHA-256
  binding of the request and the outcome. The same occurrence replays the first outcome; a changed
  binding conflicts (the BANK-0 §3 pattern).
- **Offer.** `game_market_offers`: `offer_id` (`MarketOfferId`, UUIDv7), World, definition key,
  definition revision, side (`SELL` or `BUY`), Account, character, `piece_price`, `amount`,
  `remaining`, `anonymous`, `created_at`, `expires_at` (+30 days, `MARKET0-RL-05`), state (`OPEN`,
  `FILLED`, `CANCELLED`, `EXPIRED`), `escrow_gold` and `matching_pending`.
- **Placing** (the acting character next to the locker whose depot view is open, checked again at
  execution under FND-02 §13.3):
  - not junior; the Premium gate (owner answer Q3: everyone passes until the Premium consumer
    contract is delivered, then Premium only); fewer than 100 open offers (`MARKET0-RL-01`);
  - the fee is a `FEE_DEBIT` ledger entry referencing this Market operation, class BURN under the
    Market fee cause (owner answer Q1: yes, from the bank). It is taken from the bank only, as Tibia does: a declared
    exception to BANK-FEE-0's coins-first rule. An insufficient balance refuses the offer and
    writes nothing;
  - a **sell offer** moves `amount` units of default-state entries of the ware from the
    character's depot into escrow (§6): boxes ascending, then ordinal ascending. Every entry moves
    whole except possibly the last, which is split (§6). At most 100 touched items
    (`MARKET0-RL-03`), so one sell offer holds at most 100 depot entries: 10,000 units of a
    stackable in full stacks, 100 non-stackables. A larger amount is `TOO_MANY_ITEMS`, a declared
    Reference difference (`PARITY_PENDING`);
  - a **buy offer** debits `total` from the bank into `escrow_gold` (`MARKET_ESCROW`), after
    reserving Inbox room for its worst case (§5);
  - `matching_pending` is set when the book crosses (§7).
- **Cancel** (the owner, from the depot): escrowed items go to the owner's Inbox, `escrow_gold` to
  the bank (`MARKET_ESCROW_RETURN`). The fee is never refunded.
- **Anonymous.** Other players see no name; the offer still stores its Account and character.
- **Retention.** The expiry job deletes ended offers beyond each character's 600 newest
  (`MARKET0-RL-07`); the market event keeps the audit record under MARKET-RET-0's profile.

## 5. Inbox (INBOX-1)

- **Family.** A new DUR-03 custody family, the third after `HouseInterior` and `CharacterDepot`:

  ```text
  CharacterInbox { character_id: CharacterId, ordinal: NUMERIC(20) }
  ```

  Scope Character + World, no channel, owned by the Game item domain for the Character. Only whole
  items without contents. Never touched by death, `WorldReset` or channel changes. `character_id`
  references the Character root with RESTRICT.
- **In.** Market deliveries and returns (§6), and later other decisions' deliveries (house
  disposition). The recipient may be offline or on another channel and is never fenced; its
  `game_character_inbox_counters` row lock takes the place of rule 4's `character_root` lock
  (§7).
- **Out.** Two one-item shapes, run by the owner with the depot view open: Inbox to a new main
  backpack entry (or the empty container slot, as DEPOT-0 §5 says) and Inbox to a depot box.
- **Capacity.** The counter row holds `committed` = Inbox entries + entries reserved by the
  character's open offers (a buy offer reserves `remaining`, its worst case, since sellers' entries
  may be single units; a sell offer reserves its escrowed entries). A new offer or an accept that
  would raise it above `MARKET0-RL-06` (100,000) is `INBOX_FULL`. The ceiling refuses only new
  offers and accepts: no delivery is ever refused, whether a reserved Market delivery or return, or
  another decision's delivery (house disposition, HOUSE-OWN-0), which counts without a reservation. This bounds a character's open buy offers to 100,000 units in all (a declared
  Reference difference, `PARITY_PENDING`).

## 6. Escrow and fills (MARKET-1)

- **Item escrow.** A new custody family `MarketOfferEscrow { offer_id, ordinal }`, World-scoped,
  owned by the Market. Items in it are not spendable elsewhere (DUR-03 §7.1, §34).
- **Gold escrow.** `escrow_gold` is DUR-03 §18 non-item value in custody; a guard keeps
  `escrow_gold = remaining × piece_price` for an open buy offer and 0 for an ended one.
- **Fill price.** The maker is the older of the two offers; a fill is at the maker's price.
- **Accepting a sell offer** (`accept {offer_id, amount}`, the accepter next to its open locker):
  the buyer's bank pays `amount × piece_price` (`MARKET_PURCHASE`), the seller's bank receives it
  (`MARKET_SALE`), and the escrowed entries move to the buyer's Inbox as they are.
- **Accepting a buy offer:** the seller's depot entries move to the buyer's Inbox in §4's order;
  `escrow_gold` pays the seller (`MARKET_SALE`).
- **A matching step** where a new buy offer (taker) meets an older sell offer (maker) at a lower
  price also returns `(buy_price − sell_price) × n` from `escrow_gold` to the buyer's bank
  (`MARKET_ESCROW_RETURN`).
- **Splits.** Only the last touched entry may be split. The moved part is a new item with a
  planned output identity (§11.3), class `SPLIT_MERGE_QUANTITY` with §12 lineage; the source item
  keeps its identity, location row and the remaining quantity. No value is minted. Nothing is
  re-stacked (Canary creates new stacks; moving the items keeps their identity).
- **Refused:** an offer of the same Account (the balance is shared, as BANK-0 §4.3 refuses
  same-Account transfers; a declared Reference difference); a junior accepter (it has no bank under
  BANK-0's owner answer b, so a junior cannot buy either, a declared consequence); `amount` above `remaining`; an
  ended offer; missing units or gold; more than 100 touched items (larger amounts take several
  accepts, `PARITY_PENDING`); a book with a pending match (`BOOK_BUSY`, retryable, §7).
- **Balance ceiling.** Market credits (sales and escrow returns) are value already owned and are
  not refused at `BANK0-RL-01`: they may raise a balance up to the hard ceiling `MARKET0-RL-10`
  (9,000,000,000,000,000). Voluntary credits (deposit, transfer in) still stop at `BANK0-RL-01`.
  A credit past the hard ceiling leaves the step pending with an operator alert: nothing is lost.
- **Causes.** Closed `MarketCause {Place | Accept | Cancel | Expire | Match, offer_id,
  occurrence}`; the fee's BURN is under the Market variant of `FeeBurnCause` (Q1).

## 7. Serialization, matching and expiry (MARKET-1)

- **Book.** `game_market_books`: one row per (World, definition key), revision-free; offers keep
  their definition revision, and a fill needs DUR-03 §46 compatibility. Every place, accept,
  cancel, expiry and matching step upserts it, then locks it FOR UPDATE, so one ware's book is
  serialized across every channel process of the World.
- **Lock order** (composition rule 4, extended): the recovery fence and admission relations; the
  operation occurrence; for a player command, the acting Character's fence (rule 2),
  `character_root` and DEPOT-0's container-slot row; the book row; the offers by `offer_id`; the
  escrow and depot entries by ItemInstanceId; the Inbox counters by CharacterId; the balance rows
  by `account_id`.
- **Jobs.** Matching and expiry steps have no acting Character: they take the recovery fence and
  admission relations and no session fence. They pick candidates without a lock, lock the book
  (`FOR UPDATE SKIP LOCKED` on the book row), then lock the offer and check its state again. Any
  channel process of the World may run them; at most `MARKET0-RL-08` (100) steps per pass.
- **Matching.** The placing process runs the steps right after its commit; the job resumes them
  after a crash. On one book, pending offers are matched oldest first, each step filling the best
  counter-offer (best price, then oldest) at the maker's price, skipping same-Account offers, up to
  100 items. A step clears `matching_pending` when nothing crosses. While a book has a pending
  offer, accepts on it are `BOOK_BUSY`. Tibia matches at once; the short asynchronous window is a
  declared difference (`PARITY_PENDING`).
- **Expiry.** Offers past `expires_at`, oldest first, one per step, returning escrow as a cancel.
- **Revision.** No `CharacterRevision` advance (composition rule 1 as amended).

## 8. Persistence deltas (MARKET-1 and INBOX-1)

Each keeps the previous function body and adds one clause, as DEPOT-0 §6 does:

- **Tables:** operations, offers, books, the escrow and Inbox location tables (immutable rows, FK
  to item instances and Character roots with RESTRICT), the Inbox counters.
- **Exclusivity:** both location tables join the HOUSE-CUSTODY-1 item-level guard.
- **Placement and removal proofs:** escrow and Inbox clauses in `game_item_placement_proven`; rows
  deleted only by a TRANSFER of a committed Market or Inbox operation, with evidence captured in
  the `0014` idiom.
- **Splits:** the `0012` mint guard and the `0023` item-change guard gain a branch for a Market
  split line: a new item whose outbox row and lineage name the source, and a source whose quantity
  falls by exactly the moved part, in a depot or escrow row that stays.
- **Reservations and receipts:** source and destination kinds for depot, escrow and Inbox, their
  shape numbers, and a nullable `channel_id` for them.
- **Consistency guard:** the new kinds without Ground or corpse evidence; the two tables added to
  the counted location tables.
- **Audit:** additive escrow and Inbox location messages in `OneItemTransferV1`; one market event
  per transaction in a market outbox.
- **BANK-0 amendment:** a ledger entry references exactly one of a bank operation, a fee record
  (BANK-FEE-0) or a Market operation; kinds `MARKET_ESCROW`, `MARKET_ESCROW_RETURN`,
  `MARKET_PURCHASE`, `MARKET_SALE`, and `FEE_DEBIT` with a Market reference; a
  `counterparty_character_id` column; the acting character is NULL for a job step, and a
  counterparty entry names its own account's character as counterparty; the balance CHECK uses
  `MARKET0-RL-10` and the voluntary-credit rule of §6.
- **Conservation guard** (deferred, per Market operation): the sum of its ledger deltas, plus the
  change of `escrow_gold`, plus the fee burn, is 0; item lines conserve units per ware.

## 9. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `MARKET0-RL-01` open offers per character | 100 |
| `MARKET0-RL-02` amount per offer | 1 to 64,000 |
| `MARKET0-RL-03` touched items per transaction | 100 |
| `MARKET0-RL-04` fee | 2% of total, 20 to 1,000,000 gold |
| `MARKET0-RL-05` offer lifetime | 30 days |
| `MARKET0-RL-06` Inbox entries plus reservations | 100,000 |
| `MARKET0-RL-07` ended offers kept per character | 600 |
| `MARKET0-RL-08` job steps per pass | 100 |
| `MARKET0-RL-09` offers per query page | 32 |
| `MARKET0-RL-10` balance hard ceiling for Market credits | 9,000,000,000,000,000 |
| Place sell | 100 items, 200 location lines, 1 split, 1 value line, 302 work units, 1 event |
| Place buy | 0 items, 2 value lines (fee, escrow), 1 event |
| Accept or match | 100 items, 200 location lines, 1 split, 3 value lines, 304 work units, 1 event |
| Cancel or expire | 100 items, 200 location lines, 1 value line, 301 work units, 1 event |
| Inbox out | 1 item, 2 location lines, 3 work units, 1 event |
| `DUR03-RL-03-MARKET` value lines | 3; each ledger entry and each `escrow_gold` change is one line |

Participants per transaction: two Characters and two Accounts at most. The event payload ceiling
is measured by MARKET-1 against the audit envelope before implementation.

## 10. Wire (MARKET-WIRE-1)

- **Capability `MARKET_V1`**, requiring capability 5 `DEPOT_V1`; its number, two command types and
  one state domain are reserved on #162 at allocation.
- **`MARKET_QUERY`** (reads, only while the depot view is open): the category index; one ware's
  sell and buy offers by page (32, best price first); My Offers; offer history. Anonymous offers
  show no name.
- **`MARKET_INTENT`** (a oneof; an empty oneof is `REJECTED`): `place {side, definition, amount,
  piece_price, anonymous}`, `accept {offer_id, amount}`, `cancel {offer_id}`. Results: `OK`,
  `INSUFFICIENT_FUNDS`, `NOT_ENOUGH_ITEMS`, `INBOX_FULL`, `OFFER_LIMIT`, `NOT_PREMIUM`, `JUNIOR`,
  `NOT_MARKETABLE`, `OFFER_GONE`, `SAME_ACCOUNT`, `TOO_MANY_ITEMS`, `BOOK_BUSY`, plus the common
  results.
- **Inbox view.** The locker's box list gains the Inbox; USE field 7 `InboxTargetV1 {page}` opens
  one page (32 entries) in domain 11, and command 9 takes an Inbox entry handle as source, with the
  main backpack or `DEPOT {box}` as destination.
- The Market view closes with the depot view.

## 11. Declared Reference differences

All are architect applications of the minimum-sufficient doctrine, each reversible by a later
decision: sell offers of at most 100 entries and fills of at most 100 items; the 100,000 Inbox
reservation; asynchronous matching with `BOOK_BUSY`; same-Account offers refused; junior
characters cannot buy (from BANK-0 Q1b); no re-stacking on delivery; the total capped at
999,999,999,999.

## 12. Rejected options

- **An in-memory World Market service.** No such runtime exists; one book row per ware gives one
  serialized book across channels with no new process.
- **Matching inside the placing transaction.** It would touch an unbounded number of offers.
- **Delivering purchases into the depot.** Tibia uses the Inbox, and the depot has an account limit
  a delivery must not break.
- **Unknown wares as marketable.** The Market needs a category to list a ware.
- **Refusing a sale credit at `BANK0-RL-01`.** It would strand escrowed gold in a step that can
  never commit.
- **Tibia Coin offers in this slice.** The owner wants them (Q2b), but Tibia Coins are Platform's;
  MARKET-COINS-0 adds them with a cross-repository contract.
- **Refunding the fee on cancel.** Tibia never refunds it.

## 13. Owner questions (answered)

Owner answers, verbatim record on #162 5913348961: Q1 "tak z konta" (yes, from the bank); Q2 b;
Q3 yes.

**Q1. Admit the Market fee as a gold sink?** D178 needs an owner decision for every new fee source.
a) Yes, 2% (20 to 1,000,000) from the bank, as in Tibia (recommended); b) no fee.

**Q2. Tibia Coins on the Market?** Tibia lets even free accounts trade Tibia Coins; in Oteryn they
are Platform's, sold for real money. a) Not now: items for gold only; a later decision with
Platform (recommended); b) yes, as in Tibia.

**Q3. Premium before Premium exists?** Placing offers needs Premium in Tibia, but the Game has no
Premium source yet, so everyone reads as Free. a) Everyone may place offers until Premium is
delivered, then Premium only (recommended); b) nobody places offers until then.

## 14. Decision test

- **Must decide now:** YES. The owner asked for the Market now, with the bank and depot.
- **Minimum sufficient:** one operation, offer and book table, two custody families, bounded steps,
  three commands; no statistics, no coin offers yet, no ware merging.
- **Superseding evidence:** a later owner change of the answers in §13; official partial-fill or
  fee-rounding rules.
- **Deliberately not decided:** Tibia Coin offers, statistics and the unfair-offer highlight,
  `trade_as`, the Stash, bags with contents.

## 15. Before-freeze checklist

1. **Contract amendments:** DUR-03 (after the §38 table) and the composition decision (before its
   §7), each written "pending on acceptance of MARKET-0". The BANK-0, BANK-FEE-0, DEPOT-0, DUR-03
   §39.3 and gold fee §4.4 edits follow as stated in the header.
2. **Serialization:** the book lock and §7's lock order; jobs lock the book before the offer.
3. **Restart:** operations, offers, escrow and Inbox are durable; steps resume idempotently.
4. **Typed references:** `MarketOfferId`, AccountId, WorldId, CharacterId, definition key and
   revision, occurrence, TransactionId.
5. **Wire:** §10, capability `MARKET_V1`.
6. **Split work:** at most 100 touched items and 3 value lines per transaction.
