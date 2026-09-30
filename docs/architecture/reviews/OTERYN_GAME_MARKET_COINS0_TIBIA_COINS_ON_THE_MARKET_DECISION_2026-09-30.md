# MARKET-COINS-0 Tibia Coins on the Market

- Decision: `MARKET-COINS0-COIN-OFFERS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  economy, persistence, protocol, privacy and cross-repository integration), protected
  integration here, and the matching Platform change accepted in `Oteryn/Oteryn-Platform`
  (MKTCOIN-P). It extends MARKET-0 and integrates after it.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: MARKET-0 owner answer Q2b (Tibia Coins on the Market, "as in Tibia", #162 5913348961)
  and the owner direction of 2026-09-30 (build now, full Tibia Global parity)
- Builds on: MARKET-0 (§3-§10); BANK-0 (§3, §4.4 junior rule); BANK-FEE-0; the gold fee
  decision §4.4 (D178, the `MarketFee` variant, D238); DUR-03 §18, §19, §26, §34, §35 and §38; the Store
  catalog owner decision (§2, follow-up 1); PROD-ENTITLEMENTS-01 (authority split §2);
  PREMIUM-DELIVERY-0 §3 (service transport); FND-ID-01 (`AccountId` is Platform's); owner rule
  5905825574 (Global parity)
- Cross-repository coordination id: `OTV2-MARKET-COINS`. Platform owns its side: this document
  never binds Platform; MKTCOIN-P accepts or amends §5 there.
- Amends, each pending on acceptance of MARKET-COINS-0 (#162 5912405163), in this PR: MARKET-0
  §3.1 and §10; the Store catalog owner decision (follow-up 1).
- Runtime, migration and production authority: NONE. Platform authority: NONE. Each child needs
  its own allocation (Game: #162; Platform: its own coordinator).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Repository, worker | Builds | Depends on |
|---|---|---|---|
| MKTCOIN-CONTRACT-1 | Game, control plane; security review | the consumer side of the coin custody contract, `docs/contracts/` (§5) | this decision; owner answers C1, C2 |
| MKTCOIN-P | Oteryn-Platform (external) | the Wallet transferable balance and the market hold operations of §5; a test producer | Platform's own acceptance of §5; C1, C2 |
| MKTCOIN-1 | Game, hard; persistence, economy and security review | coin offer columns, operation states, the coin instruction outbox, deliverer and reconciler, every step of §6-§7 | MARKET-1; MKTCOIN-CONTRACT-1 |
| MKTCOIN-WIRE-1 | Game, impl; protocol review | capability `MARKET_COINS_V1`, the coin ware, two results, the balance line (§8) | MARKET-WIRE-1; MKTCOIN-1 |
| MKTCOIN-E2E-1 | Game and Platform; security and cross-repository review | one end-to-end test against the real producer, failure paths of §7 | MKTCOIN-1; MKTCOIN-P |

MKTCOIN-1 and MKTCOIN-P run in parallel; MKTCOIN-1 tests against a test producer that serves §5
exactly. Activation on any World follows owner answer C3. Later, each with its own decision:
Store purchases with coins, coin gifts, the world transfer rule "no open coin offers", statistics.

## 1. Question

How do two Accounts trade transferable Tibia Coins for gold on the Market, when the gold lives in
the Game database and the coins live in Platform, with no double spend and no coin made by Game?

## 2. Facts

**PROVEN**

- MARKET-0 (candidate on `main`): offers, books, fills, the 2% fee from the bank, gold escrow as
  DUR-03 §18 value, the book lock, jobs, the market event; owner answer Q2b wants coin offers;
  §12 deferred them to this decision "with a cross-repository contract".
- The Store catalog owner decision §2: the Tibia Coin balance, payment flow and purchase ledger
  stay with Platform; follow-up 1 asks for a Game/Platform contract for balance reads and debits.
- PROD-ENTITLEMENTS-01 §2: payment truth and entitlement truth are Platform's, gameplay truth is
  Game's; neither becomes the other's authority.
- DUR-03 §35: no Platform/Game distributed 2PC, no implicit remote-service atomicity; "a future
  external persistence/service custody boundary requires dedicated safe handoff/custody
  contract". §34: a workflow spans transactions only through typed custody, each step
  idempotent and restartable. §26: correction is a new compensating transaction.
- Platform (`Oteryn/Oteryn-Platform` `c914564`, read only): the Wallet module holds "Oteryn Coins"
  per Platform Identity with `available_balance`, `reserved_balance`, an append-only ledger and
  idempotent reserve, release and settle (`docs/architecture/MODULE_CATALOG.md` Wallet;
  ADR 0016 §5-§7, which rejects a distributed transaction for a saga). The Wallet has no
  transferable split and says its coins "are not ... transferable coins"; funding is operator
  only; payments, refunds and chargebacks are not built. Its entitlement delivery contract
  Profile E keeps currency authority in the Wallet.
- BANK-0 §4.4 (owner answer b): a junior character has no bank use. MARKET-0 §6 refuses
  same-Account offers and junior accepters.
- PREMIUM-DELIVERY-0 §3: a pull-only private service call, mutual TLS, scoped service identity.

**CIPSOFT_OFFICIAL** (the Tibia manual)

- Coins belong to the account and serve all its characters; they can be gifted and traded on the
  Market by Premium and free players (`store.md` §6.1).
- Free accounts may place buy and sell offers for Tibia Coins, though other offers need Premium
  (`controls_trading.md` §4.3.3, line 30).
- "Restricted coins": depending on payment method, amount and account history, some bought
  coins are locked for 6 months and cannot be gifted or traded [platform] (`store.md` §6.1).
- The Store shows two balances: all coins and the transferable part (`store.md` §6.2).
- Coins History lists market trades (`accounts.md` line 113). A world transfer needs no open
  Market offers involving Tibia Coins (`products.md` line 134). Gifts go in steps of 25.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b5`, `src/game/game.cpp`)

- The coin ware is item 22118 (`ITEM_STORE_COIN`, `src/utils/utils_definitions.hpp:611`).
- Placing a sell offer checks and removes transferable coins at once (lines 10762-10772); cancel
  adds them back (10854-10856); a fill adds transferable coins to the buyer, "Purchased on
  Market" (11014-11015, 11056-11057). The fee is 2%, 20 to 1,000,000, for every ware (10739-10748).
- Canary applies its Premium switch to coin offers too (10703), unlike Global.

## 3. Who holds what

- **Platform holds every coin.** The balance, its transferable part, holds and the coin ledger
  are Platform's Wallet. Game stores no coin balance and never credits a coin that was not first
  held for a Market offer or accept (no MINT, no BURN of coins in Game).
- **Game holds the book and the gold.** Offers, fills, the fee and gold escrow stay MARKET-0's,
  in the Game database under DUR-03. The coin side of an offer is a *claim on a Platform hold*:
  the hold key, the coins in it, and how many were settled or released.
- **The coin ware** is a virtual Market ware `oteryn:market.tibia_coin`, one book per World. It is
  never an Item definition, an ItemInstance, a depot entry, an escrow entry or an Inbox entry.
  Its wire id is the client's coin market id (22118 in Canary, confirmed by MKTCOIN-WIRE-1).
- Only **transferable** coins are held. Platform decides which coins are transferable (C2); Game
  never sees non-transferable coins.
- Coins are per Account on every World; gold is per (Account, World). One coin book per World
  keeps each trade inside one World's gold (DUR-03 §19); the coins themselves are not World
  value.

## 4. Offers and fills (MKTCOIN-1)

- **Numbers:** `amount` in coins, 1 to 64,000 (`MARKET0-RL-02`; step size `PARITY_PENDING`, see
  R2); `piece_price` in gold per coin; `total` and the fee as MARKET-0 §3.2. The fee is the
  existing Market fee (D238 under D178), paid from the bank, never refunded: no new fee source.
- **Gates:** not junior; not same Account; coin offers count toward `MARKET0-RL-01`. The Premium
  gate does **not** apply to coin offers (Global: free accounts may place them).
- **Sell offer (coins for gold):** the coins are held on Platform first (§6.1), then the offer
  opens with the fee. Every later fill of it is Game-only.
- **Buy offer (gold for coins):** MARKET-0's buy offer: the fee and `total` into `escrow_gold`.
  No Inbox reservation (coins never reach the Inbox). No Platform call.
- **Accept a sell offer** (the buyer pays gold): one Game transaction: `MARKET_PURCHASE` and
  `MARKET_SALE` ledger entries, `remaining` falls, and one SETTLE instruction moves `n` coins from
  the offer's hold to the buyer's Account.
- **Accept a buy offer** (the seller gives coins): the seller's coins are held first (§6.1), then
  one Game transaction pays the seller from `escrow_gold` (`MARKET_SALE`) and queues SETTLE from
  that hold to the buyer.
- **Matching:** a coin sell offer already has its hold, so every match step is Game-only: gold as
  MARKET-0 §6, plus one SETTLE.
- **Cancel, expiry, `CREDIT_HELD`:** a sell offer's remaining coins are released by one RELEASE
  instruction in the same Game transaction (no item bound makes it wait for the next step, as
  MARKET-0 §6 does for items); a buy offer returns gold as MARKET-0.
- **Guard:** for a coin sell offer, `hold_amount = remaining + coins_settled + coins_released`,
  and `coins_released > 0` only in an ended state.
- **Delivery:** bought coins reach the buyer's Platform balance when Platform applies SETTLE, as
  transferable coins (Canary; Global `PARITY_PENDING`; C2).

## 5. The coin custody contract (MKTCOIN-CONTRACT-1 and MKTCOIN-P)

A new cross-repository contract, written by MKTCOIN-CONTRACT-1 in `docs/contracts/` and accepted
by Platform for its Wallet. Game calls Platform over the PREMIUM-DELIVERY-0 §3 channel (mutual
TLS, a separate service identity scoped to Market coin custody). Every call is idempotent by its
key; the same key with other content is an integrity conflict.

| Call | Effect on Platform | Refusals |
|---|---|---|
| `HOLD {hold_key, account, amount, world, operation}` | moves `amount` transferable coins to a hold owned by Game's Market | `NOT_ENOUGH_COINS`, `ACCOUNT_BLOCKED`, `ABORTED_KEY` |
| `SETTLE {instruction_id, hold_key, to_account, amount}` | moves coins from the hold to `to_account`, transferable | only an integrity fault (over the hold, unknown hold) |
| `RELEASE {instruction_id, hold_key, amount}` | returns coins from the hold to its account | only an integrity fault |
| `ABORT {hold_key}` | releases an unused hold, or records a tombstone so a late `HOLD` with that key is refused | none |
| `STATUS {hold_keys}` | the hold's amount, settled, released, state, and a recall flag | none |
| `BALANCE {account}` | total and transferable coins, for display only | unavailable |

- **No minting:** Platform refuses any SETTLE or RELEASE that would take a hold below zero, so
  Game can only move coins an owner put on the Market.
- **Settles cannot fail on business grounds:** a held coin is already sold. A closed or blocked
  buyer Account still gets the coins on Platform; Platform enforcement handles that Account.
- **Defence in depth against a compromised Game server:** the service identity can only HOLD,
  SETTLE, RELEASE, ABORT and read; it can never credit coins without a hold. Platform also caps
  HOLD volume per Account and per World per 24 h (`MKTCOIN0-RL-10`), refuses a HOLD above the cap
  with `ACCOUNT_BLOCKED`, and alarms on unusual hold volume, so a stolen Game credential can move
  at most a bounded amount before an operator stops it.
- **Recall:** Platform may flag an open hold (fraud, account closure). Game sees the flag through
  `STATUS` and runs a cancel step for that offer (fee kept). Coins already settled stay settled.
- **Platform-side coin history:** Platform writes "sold on Market" and "bought on Market" entries
  from SETTLE; Game sends AccountIds, amounts, the WorldId and the operation id, and no
  character name, CharacterId or counterparty name.
- **External dependency (not edited here):** Platform's Wallet contract (its MODULE_CATALOG
  Wallet section, DATA_OWNERSHIP and ADR 0016 §5) must add the transferable balance, the Game
  service principal and these hold operations, and resolve C1. Platform accepts or amends §5.

## 6. The steps (MKTCOIN-1)

### 6.1 A step that needs a hold (placing a sell offer, accepting a buy offer)

1. **Pre-check** without writing: gates, fee and gold, offer state. A refusal writes nothing.
2. **T1 (Game):** the operation row takes state `COIN_HOLD_PENDING`, with `hold_key` (UUIDv7,
   derived once from the occurrence) and the request binding.
3. **HOLD** on Platform, one per Account in flight (`MKTCOIN0-RL-08`), within
   `MKTCOIN0-RL-02` (2,000 ms).
4. **T2 (Game)** on a confirmed hold: under MARKET-0's lock order, check everything again, then
   commit the offer (fee BURN, `OPEN`) or the fill (gold lines and SETTLE). A refused hold ends
   the operation `FAILED` (`NOT_ENOUGH_COINS`). A timeout, an unavailable Platform, or a refusal
   in T2 ends it `ABORTED` and queues ABORT in the same T2: `COINS_UNAVAILABLE` or the T2
   refusal.
5. **Crash:** the reconciler finds operations in `COIN_HOLD_PENDING` older than
   `MKTCOIN0-RL-03` (60 s), and aborts them the same way. Game never commits a coin offer or fill
   without a confirmed hold, and never both commits and aborts one operation: both paths lock
   the operation row.

### 6.2 Instructions

- SETTLE, RELEASE and ABORT are rows of a coin instruction outbox written in the Game transaction
  that decides them (transactional outbox); the fill or cancel is final at that commit.
- A deliverer sends them at least once with backoff (`MKTCOIN0-RL-04`) until Platform
  acknowledges; Platform applies each once by `instruction_id`. Order does not matter: amounts are
  explicit and every one is bounded by the hold.
- A replay of a player command returns the first outcome (MARKET-0 §4); the hold key and
  instruction ids come from the occurrence, so a retry never holds or settles twice.

## 7. Failure, reconciliation, audit

- **Platform unavailable:** placing coin sell offers and accepting coin buy offers are refused
  with `COINS_UNAVAILABLE` (retryable). Buying from existing sell offers, coin buy offers,
  matching, cancel and expiry go on; their SETTLE and RELEASE wait in the outbox. An instruction
  unacknowledged after `MKTCOIN0-RL-05` (15 minutes) raises an alarm. Nothing is lost or doubled.
- **Reconciliation:** a daily job (and one on demand) compares each open or recently ended hold
  with `STATUS`, `MKTCOIN0-RL-06` holds per pass. A difference raises `MARKET_COIN_MISMATCH`,
  stops fills of that offer, and is corrected only by a reviewed compensating operation on the
  owning side (DUR-03 §26), never by a direct edit.
- **Audit:** the market event (MARKET-0 §8) gains the hold key, coins moved, instruction ids and
  the counterparty Account; retention under MARKET-RET-0's `ECONOMY_LEDGER` profile.
- **Conservation:** gold lines as MARKET-0 (sum 0 plus the fee burn). Coin conservation is
  Platform's, per hold; Game's guard (§4) and the reconciler check the mirror.
- **Revision:** no `CharacterRevision` advance (composition rule 1, as MARKET-0).

## 8. Wire (MKTCOIN-WIRE-1, amends MARKET-0 §10)

- **Capability `MARKET_COINS_V1`**, requiring `MARKET_V1`; its number is reserved on #162 at
  allocation. Without it the coin ware is not listed and coin offers cannot be placed.
- The coin ware is sent as its ware id in `place` and `MARKET_QUERY`; offers show coins as the
  amount and gold per coin as the price.
- `MARKET_QUERY` gains the Account's total and transferable coins, read by `BALANCE` when the
  Market opens; absent when Platform does not answer (display only, never a check).
- **New results** (only under `MARKET_COINS_V1`): `NOT_ENOUGH_COINS`, `COINS_UNAVAILABLE`.
  `NOT_PREMIUM` is never sent for the coin ware.

## 9. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `MKTCOIN0-RL-01` coins per offer | 1 to 64,000 (`MARKET0-RL-02`); step 1 until R2 evidence |
| `MKTCOIN0-RL-02` HOLD call in a player command | 2,000 ms |
| `MKTCOIN0-RL-03` `COIN_HOLD_PENDING` age before the reconciler aborts | 60 s |
| `MKTCOIN0-RL-04` instruction retry backoff | 1 s doubling to 5 minutes |
| `MKTCOIN0-RL-05` unacknowledged instruction alarm | 15 minutes |
| `MKTCOIN0-RL-06` holds per reconciliation pass | 1,000 |
| `MKTCOIN0-RL-07` Platform response size | 1,024 bytes; larger or malformed fails closed |
| `MKTCOIN0-RL-08` HOLD calls in flight per Account | 1 |
| `MKTCOIN0-RL-09` coin instructions per Game transaction | 2 (a fill with a held credit: SETTLE, RELEASE) |
| `MKTCOIN0-RL-10` Platform HOLD volume cap | per Account and per World per 24 h, set by MKTCOIN-P and recorded on #162 before activation |
| Place coin sell | 0 items, 1 value line (fee), 1 hold, 1 event |
| Accept a coin sell offer, or a match | 0 items, MARKET-0 gold lines (2 or 3), 1 SETTLE, 1 event |
| Accept a coin buy offer | 0 items, 2 value lines (escrow fall, sale), 1 hold, 1 SETTLE, 1 event |
| Cancel or expire a coin sell offer | 0 items, 0 value lines, 1 RELEASE, 1 event |

## 10. Rejected options

- **A Game coin balance mirrored from Platform.** Two authorities for one value (DUR-03 §35
  forbids mirrored authority); a stale mirror double spends.
- **Distributed 2PC across the two databases.** DUR-03 §35 and Platform ADR 0016 both reject it.
- **Debit coins at fill time from the seller's live balance.** A fill would need Platform online
  and could fail after the gold moved. Holding at placement keeps fills Game-only, as Canary
  removes the coins at placement.
- **Coins as an item (the 22118 ItemInstance).** It would put Platform value into DUR-03 item
  custody, the depot and the Inbox; Game would mint coins.
- **Platform runs the coin book.** Game owns offers and fills (MARKET-0; DUR-03 §38).
- **Hold expiry on Platform as the crash cleanup.** An expiring hold could vanish under a
  committed offer; Game's ABORT with a tombstone is explicit.
- **Refusing all coin fills while Platform is down.** Held coins are already safe; only new
  holds need Platform.

## 11. Owner-rule applications

**Global parity kept:** coins for gold on the Market; free accounts may place coin offers; the
same 2% fee; only transferable coins trade; bought coins land on the account balance, not the
Inbox; anonymous offers; the 100 offer limit; coin trades in the coin history.

**Declared differences:**
- Coin sell offers and coin buy accepts are refused while Platform is down; bought coins may
  reach the balance late (R1).
- The coin amount step is 1 until sourced (R2).
- Junior characters cannot trade coins; same-Account offers are refused (from MARKET-0).
- A coin book per World while coins are per Account.

**Architect rulings (owner rule 5905825574):**

**R1. Platform outage.** Tibia has one server; here the coins are elsewhere. a) Refuse only the
steps that need a new hold and deliver settlements late (recommended: no double spend, most of
the Market keeps working); b) close coin trading during an outage; c) let Game fill against an
estimated balance (double spend). **Ruled a).**

**R2. Coin amount step.** The manual gives 25-coin steps for gifts only. a) Step 1 until an
official source (recommended, Canary accepts any amount); b) 25 by analogy. **Ruled a)**; an
official source supersedes it.

## 12. Owner questions

**C1. Which coins are Tibia Coins?** Context: Platform's Wallet holds "Oteryn Coins" (Character
Bazaar, operator funded); the Store catalog prices in Tibia Coins. a) One currency: the Wallet's
coins are the Store's and the Market's Tibia Coins, with a transferable part added (recommended:
one balance, as Global has); b) a second, separate Tibia Coin balance on Platform.

**C2. Which coins are transferable (anti-fraud)?** Context: in Global some bought coins are locked
for up to 6 months by payment risk. a) Platform decides per purchase: coins are transferable
unless its payment-risk rule locks them for up to 6 months; operator grants and compensations are
not transferable; coins bought on the Market are transferable; a chargeback never reverses a
Market fill, Platform acts on the paying Account (recommended: Global's model, innocent buyers
kept whole); b) every coin transferable; c) only coins bought on the Market transferable.

**C3. When may coin trading go live?** Context: coins bought with money become tradeable for
gold, so gold gains a money price. a) Test Worlds with operator-granted coins once MKTCOIN-E2E-1
passes; production Worlds only after Platform's payment, refund and chargeback policy is
accepted (recommended); b) production Worlds with operator-granted coins at once; c) not before
paid coins exist anywhere.

## 13. Decision test

- **Must decide now:** YES. The owner asked for coin offers (Q2b) and full parity now.
- **Minimum sufficient:** one virtual ware, one hold per sell offer or accept, three instruction
  kinds, one outbox, one reconciler, one capability, two results; MARKET-0's book, fee, gold
  escrow, locks and jobs are reused, and Platform's existing Wallet reserve pattern is extended.
- **Superseding evidence:** official Global coin amount steps, whether Market-bought coins stay
  transferable, and any Global rule on coin offers of recently bought coins.
- **Deliberately not decided:** coin gifts, Store purchases, payments, prices, the world transfer
  rule, statistics, Tournament Coins.

## 14. Before-freeze checklist

1. **Contract amendments:** MARKET-0 §3.1 and §10; the Store catalog owner decision follow-up 1;
   each written "pending on acceptance of MARKET-COINS-0". The Platform Wallet change is an
   external dependency (MKTCOIN-P), not edited here. The capability number is reserved at
   allocation.
2. **Serialization:** MARKET-0's book lock and lock order; the operation row lock decides commit
   or abort; one HOLD in flight per Account.
3. **Restart:** operations, offers and instructions are durable; the deliverer and reconciler
   resume; Platform applies each key once.
4. **Typed references:** `MarketOfferId`, AccountId, WorldId, hold key, instruction id,
   operation occurrence, TransactionId.
5. **Wire:** §8, capability `MARKET_COINS_V1`.
6. **Split work:** 0 items per coin transaction; at most 3 gold lines and 2 coin instructions.
