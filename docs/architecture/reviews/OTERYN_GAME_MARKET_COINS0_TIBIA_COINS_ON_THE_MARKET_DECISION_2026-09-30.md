# MARKET-COINS-0 Tibia Coins on the Market

- Decision: `MARKET-COINS0-COIN-OFFERS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (security,
  economy, persistence, protocol, privacy and cross-repository integration), protected
  integration here, and the matching Platform change accepted in `Oteryn/Oteryn-Platform`
  (MKTCOIN-P). It extends MARKET-0 and integrates after it. Owner questions C1-C3 (§12) are
  answered: C1 a, C2 b, C3 a; the owner confirmed the C2 chargeback rule (§5, §12) and the
  restore rule `MKTCOIN0-RESTORE` (§5; #162 Q9a, 2026-09-30).
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
  §3.1, §4 (the offer's ware key), §7 (the book key) and §10; the Store catalog owner decision
  (follow-up 1).
- Runtime, migration and production authority: NONE. Platform authority: NONE. Each child needs
  its own allocation (Game: #162; Platform: its own coordinator).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Repository, worker | Builds | Depends on |
|---|---|---|---|
| MKTCOIN-CONTRACT-1 | Game, control plane; security review | the consumer side of the coin custody contract, `docs/contracts/` (§5) | this decision |
| MKTCOIN-P | Oteryn-Platform (external) | the Wallet transferable balance and the market hold operations of §5; a test producer | Platform's own acceptance of §5 |
| MKTCOIN-1 | Game, hard; persistence, economy and security review | coin offer columns, operation states, the coin instruction outbox, deliverer and reconciler, every step of §6-§7 | MARKET-1; MKTCOIN-CONTRACT-1 |
| MKTCOIN-WIRE-1 | Game, impl; protocol review | capability `MARKET_COINS_V1`, the coin ware, two results, the balance line (§8) | MARKET-WIRE-1; MKTCOIN-1 |
| MKTCOIN-E2E-1 | Game and Platform; security and cross-repository review | one end-to-end test against the real producer, failure paths of §7 | MKTCOIN-1; MKTCOIN-P |

MKTCOIN-1 and MKTCOIN-P run in parallel; MKTCOIN-1 tests against a test producer that serves §5
exactly. Activation (owner answer C3 a): test Worlds, with operator-granted coins, once
MKTCOIN-E2E-1 passes; production Worlds only after Platform accepts its payment, refund and
chargeback policy (an external dependency, §5). Later, each with its own decision:
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
- **Ware key (`MKTCOIN0-WARE-KEY`, amends MARKET-0 §4 and §7).** A Market ware is a discriminated
  key: `ITEM {definition_key, definition_revision}` (MARKET-0 unchanged) or `COIN {kind}`, with
  `kind` a closed list holding only `TIBIA_COIN`. Offers store `ware_kind` (`ITEM` or `COIN`) with
  `definition_key` and `definition_revision` set only for `ITEM` and `coin_kind` set only for
  `COIN` (a database check refuses any other combination); books key by (World, `ware_kind`,
  `definition_key` or `coin_kind`). A `COIN` ware has no revision: DUR-03 §46 compatibility is
  not evaluated and a coin fill needs only equal `coin_kind`. An unknown `ware_kind` or
  `coin_kind` read from storage or the wire fails closed (`REJECTED`, no write). Existing
  MARKET-1 rows, if any, migrate as `ITEM` with their key and revision unchanged.
- **Tibia Coins** are the Oteryn Coins of Platform's Wallet, one balance for the Store and the
  Market, with a transferable part (owner answer C1 a). Only **transferable** coins are held;
  every coin is transferable (owner answer C2 b), so the transferable part is the whole balance.
  Game never decides transferability and never sees a non-transferable coin.
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
- **Buy offer (gold for coins):** the buyer's Account is claimed on Platform first (§6.1, CLAIM
  of `amount`), then MARKET-0's buy offer: the fee and `total` into `escrow_gold`. No Inbox
  reservation (coins never reach the Inbox).
- **Accept a sell offer** (the buyer pays gold): the buyer's Account is claimed for `n` coins
  first (§6.1), then one Game transaction: `MARKET_PURCHASE` and `MARKET_SALE` ledger entries,
  `remaining` falls, and one SETTLE instruction moves `n` coins from the offer's hold to the
  buyer's claim.
- **Accept a buy offer** (the seller gives coins): the seller's coins are held first (§6.1), then
  one Game transaction pays the seller from `escrow_gold` (`MARKET_SALE`) and queues SETTLE from
  that hold to the buy offer's claim.
- **Matching:** a coin sell offer already has its hold and a coin buy offer its claim, so every
  match step is Game-only: gold as MARKET-0 §6, plus one SETTLE naming both.
- **Cancel, expiry, `CREDIT_HELD`:** a sell offer's remaining coins are released by one RELEASE
  instruction in the same Game transaction (no item bound makes it wait for the next step, as
  MARKET-0 §6 does for items); a buy offer returns gold as MARKET-0 and queues ABORT of its
  claim with the exact unused amount and its settlement watermark (`MKTCOIN0-ABORT-ORDER`, §5).
- **Guard:** for a coin sell offer, `hold_amount = remaining + coins_settled + coins_released`,
  and `coins_released > 0` only in an ended state; for a coin buy offer, `claim_amount = remaining
  + coins_settled + coins_aborted`, and `coins_aborted > 0` only in an ended state, equal to its
  ABORT's `cancelled_amount` (§5).
- **Delivery:** bought coins reach the buyer's Platform balance when Platform applies SETTLE, as
  transferable coins (Canary; owner answer C2 b).

## 5. The coin custody contract (MKTCOIN-CONTRACT-1 and MKTCOIN-P)

A new cross-repository contract, written by MKTCOIN-CONTRACT-1 in `docs/contracts/` and accepted
by Platform for its Wallet. Game calls Platform over the PREMIUM-DELIVERY-0 §3 channel (mutual
TLS, a separate service identity scoped to Market coin custody). Every call is idempotent by its
key; the same key with other content is an integrity conflict.

| Call | Effect on Platform | Refusals |
|---|---|---|
| `HOLD {hold_key, account, amount, world, offer_id, operation, binding, issued_at}` | moves `amount` transferable coins to a hold owned by Game's Market, bound to (`offer_id`, `operation`, `binding`): `PLACE` for the coin sell offer `offer_id` placed by `operation`, `ACCEPT` for the accept `operation` of the buy offer `offer_id` | `NOT_ENOUGH_COINS`, `ACCOUNT_BLOCKED`, `ABORTED_KEY`, `STALE_REQUEST` |
| `CLAIM {claim_key, account, amount, world, offer_id, operation, binding, issued_at}` | records that `account` may receive up to `amount` coins, bound as HOLD: `PLACE` for its coin buy offer `offer_id`, `ACCEPT` for the accept `operation` of the sell offer `offer_id`; moves no coins | `ACCOUNT_BLOCKED`, `ABORTED_KEY`, `STALE_REQUEST` |
| `SETTLE {instruction_id, hold_key, claim_key, maker_offer_id, taker_operation_id, to_account, amount}` | moves coins from the hold to `to_account`, transferable | integrity faults only (the settlement fence below); over `MKTCOIN0-RL-11` it is deferred, not applied |
| `RELEASE {instruction_id, hold_key, amount}` | returns coins from the hold to its account | only an integrity fault |
| `ABORT {instruction_id, hold_key or claim_key, cancelled_amount, settle_count, settle_sum}` | after the key's `settle_count` SETTLEs are applied (`MKTCOIN0-ABORT-ORDER`), releases exactly `cancelled_amount` of a hold or ends exactly `cancelled_amount` of a claim, or records a tombstone so a late `HOLD` or `CLAIM` with that key is refused | an integrity fault only (`MKTCOIN0-ABORT-ORDER`); before its watermark it is deferred, not applied |
| `STATUS {key_kind HOLD or CLAIM, key, instruction_ids}` | for a hold: amount, settled, released, state, a recall flag; for a claim: amount, settled, ended (aborted) amount, state (`OPEN`, `SETTLED`, `ABORTED`, `UNKNOWN`), including a deferred ABORT; and the disposition of each named instruction id (at most 16 per call): `APPLIED`, `DEFERRED`, `REJECTED` or `UNKNOWN` (`MKTCOIN0-DISPOSITION`) | none |
| `BALANCE {account}` | total and transferable coins, for display only | unavailable |

- **No minting:** Platform refuses any SETTLE or RELEASE that would take a hold below zero, so
  Game can only move coins an owner put on the Market.
- **Settles cannot fail on business grounds:** a held coin is already sold. A closed or blocked
  buyer Account still gets the coins on Platform; Platform enforcement handles that Account.
- **Settlement fence (`MKTCOIN0-SETTLE-FENCE`, architect ruling: fail closed).** Platform applies
  a SETTLE only when every check holds, else refuses it as an integrity fault (a terminal
  `REJECTED`, `MKTCOIN0-DISPOSITION`), moves nothing and alarms: the hold and the claim exist and are open, in the same World; `amount` is at most the
  hold's remaining coins (`amount - settled - released`) and at most the claim's remaining
  coins; `to_account` is the claim's account and differs from the hold's account; and the
  offer fields match the bindings (next rule). Game never names a destination
  Platform has not bound: the buyer is registered by its own CLAIM before any fill can pay it.
  Platform also caps SETTLE volume per destination Account and per World per 24 h
  (`MKTCOIN0-RL-11`); a SETTLE over the cap is deferred and alarmed, not applied, until a
  Platform operator either applies it (within the fence) or rejects it (terminal `REJECTED`);
  it never expires, and the deliverer keeps retrying.
- **SETTLE offer fields (`MKTCOIN0-SETTLE-SIDES`, architect ruling).** Both fields are always
  present and typed; neither is ever empty or a stand-in: `maker_offer_id` is a `MarketOfferId`,
  the resting offer of the fill; `taker_operation_id` is a `MarketOperationId` (the taking Market
  operation's occurrence, §6.1), never a `MarketOfferId`. The fence requires, of the hold and the
  claim, that one (the maker key) is bound `PLACE` to `maker_offer_id`, and the other (the taker
  key) is bound with `operation = taker_operation_id` and either:
  - `ACCEPT` with `offer_id = maker_offer_id` (a direct accept: accepting a sell offer, the taker
    key is the accepter's claim; accepting a buy offer, it is the accepter's hold); or
  - `PLACE` to a different offer on the other side of the same coin book (a matching step: the
    taker key is the pending offer's own hold or claim, and `taker_operation_id` is the operation
    that placed it).

  Any other combination, a `binding` outside the closed list {`PLACE`, `ACCEPT`}, or a missing
  field is an integrity fault.
- **ABORT ordering (`MKTCOIN0-ABORT-ORDER`, architect ruling: fail closed).** Platform applies
  instructions for one key in this order: every SETTLE naming it before its ABORT. Game computes
  the ABORT in the transaction that ends the key: `settle_count` and `settle_sum` are the number
  and coin sum of the SETTLE instructions Game has committed naming that key, and
  `cancelled_amount` is the key's amount minus `settle_sum` (minus the coins released, for a
  hold), the exact unused amount; Game issues no instruction for a key after its ABORT. Platform
  defers the ABORT until it has applied exactly `settle_count` SETTLEs for the key totalling
  `settle_sum`, then applies it only if `cancelled_amount` equals the key's unused amount; any
  difference, more SETTLEs than `settle_count`, or a nonzero watermark on an unknown key is an
  integrity fault that moves nothing and alarms; Game's reconciler reports it as
  `MARKET_COIN_MISMATCH`. An ABORT of an unknown
  key with a zero watermark records the tombstone. A deferred ABORT is reported by `STATUS` (hold or claim) and is
  pending, not a mismatch (§7).
- **Terminal disposition (`MKTCOIN0-DISPOSITION`, fail closed).** Every SETTLE, RELEASE and
  ABORT Platform receives ends in exactly one terminal disposition, recorded as its receipt by
  `instruction_id`: `APPLIED`, or `REJECTED` (an integrity fault, or a Platform operator
  rejecting a deferred instruction within a reviewed procedure). `DEFERRED` (over `MKTCOIN0-RL-11`, or an ABORT before its watermark) is
  not terminal, and a deferred instruction never expires or lapses on either side. A terminal
  disposition is final: every later delivery of that `instruction_id` returns the recorded
  receipt, and a `REJECTED` instruction is never applied. Game treats an instruction as pending
  until it sees `APPLIED` or `REJECTED` (a delivery answer or `STATUS`); `DEFERRED` and `UNKNOWN`
  are pending. A key with a pending instruction cannot end on Platform (the SETTLE's coins stay
  unsettled in the hold and the claim; an ABORT waits behind its watermark), so the retention
  clock below does not start while Game may still deliver an instruction naming that key. An
  ABORT whose watermark counts a `REJECTED` SETTLE can never reach it; it stays `DEFERRED` and is
  resolved only within the reviewed compensation for that SETTLE (§7).
- **Defence in depth against a compromised Game server:** the service identity can only HOLD,
  CLAIM, SETTLE, RELEASE, ABORT and read; it can never credit coins without a hold and a claim.
  Platform caps HOLD and CLAIM volume per Account and per World per 24 h (`MKTCOIN0-RL-10`),
  refuses one above the cap with `ACCOUNT_BLOCKED`, caps settlements as above, and alarms on
  unusual volume, so a stolen Game credential can create, redirect or receive at most a bounded
  amount per 24 h, holds opened earlier included, before an operator stops it.
- **Key retention (`MKTCOIN0-KEY-RETENTION`).** Platform keeps every hold and claim record, every
  ABORT tombstone and every request receipt (by `hold_key`, `claim_key` and `instruction_id`,
  with the request binding) for at least `MKTCOIN0-RL-12` (90 days) after the key ends: longer
  than the offer lifetime (30 days, `MARKET0-RL-05`) plus the replay horizon. The replay horizon
  is bounded on both sides: Platform refuses a HOLD or CLAIM whose `issued_at` is older than
  `MKTCOIN0-RL-13` (10 minutes) with `STALE_REQUEST`; Game's deliverer resends an instruction
  only while it is pending, and a key with a pending instruction has not ended
  (`MKTCOIN0-DISPOSITION`), so its clock has not started. An instruction still pending after
  `MKTCOIN0-RL-14` (30 days) raises `MARKET_COIN_MISMATCH` and is still resent. A late resend of
  an instruction whose key ended and was compacted finds no open hold or claim and is `REJECTED`
  by the fence, never applied twice. Compaction never removes a record of a key that has not
  ended or inside that horizon. **Restore:** see `MKTCOIN0-RESTORE` below; the receipts are
  never re-created by a script.
- **Restore of Game or Platform (`MKTCOIN0-RESTORE`, fail closed; owner-confirmed 2026-09-30,
  #162 Q9a).** v1 defines no automated
  cross-system restore protocol. After any restore of the Game database or of the Platform wallet
  that crosses coin activity (its restore point is earlier than the newest HOLD, CLAIM, SETTLE,
  RELEASE or ABORT on the World, or the operator cannot show it is not), coin trading on every
  affected World stays closed: no coin offer placing, accepting or matching, no deliverer,
  and Platform refuses HOLD and CLAIM for the World with `COINS_UNAVAILABLE`. The closure is a
  MKTCOIN-P requirement (a per-World switch an operator can set and that a restore leaves set). It
  reopens only after a reviewed manual reconciliation, a declared operational procedure (a
  MKTCOIN-P dependency owned with the restore runbook), run by two operators and recorded. The
  procedure must: (1) inventory every active custody key on Platform (holds and claims, with
  offer, operation, binding, account and amounts) and every SETTLE, RELEASE and ABORT receipt
  applied since the restore point, and refuse to proceed if Platform's retained records do not
  reach back to the restore point minus `MKTCOIN0-RL-14` (30 days); (2) match each
  against restored Game offers, operations and instruction records; (3) correct every difference
  (an orphan hold, claim or settlement, a rolled-back Platform effect, or a receipt without its
  wallet effect) only by a reviewed compensating operation on the owning side (DUR-03 §26), and
  never by a direct edit, by re-creating a tombstone or receipt alone, or by reissuing an
  instruction; and (4) end with zero unexplained differences and balances validated. A restore
  older than Platform's retention is never reopened by this procedure; it stays closed until the
  owner decides (owner-confirmed, #162 Q9a). Because the closure holds until then, the restore cases (a Game restore rolling
  back a fill Platform applied, an orphan hold, a rolled-back receipt) cannot reach trading.
- **Recall:** Platform may flag an open hold (fraud, account closure). Game sees the flag through
  `STATUS` and runs a cancel step for that offer (fee kept). Coins already settled stay settled.
- **Platform-side coin history:** Platform writes "sold on Market" and "bought on Market" entries
  from SETTLE; Game sends AccountIds, amounts, the WorldId and the operation id, and no
  character name, CharacterId or counterparty name.
- **Chargeback (owner answer C2 b, confirmed by the owner 2026-09-30, #162):** chargeback
  handling stays with Platform, which acts on the paying Account; it never reverses an applied
  SETTLE or any Market trade, so the other party is kept whole.
- **External dependency (not edited here):** Platform's Wallet contract (its MODULE_CATALOG
  Wallet section, DATA_OWNERSHIP and ADR 0016 §5) must add the transferable balance (every coin,
  C2 b), the Game service principal and these hold operations; treat its Oteryn Coins as the
  Tibia Coins of the Store and the Market (C1 a); handle chargebacks without reversing Market
  trades; and accept its payment, refund and chargeback policy before production activation
  (C3 a). These are contract requirements on Platform; Platform accepts or amends §5.

## 6. The steps (MKTCOIN-1)

### 6.1 A step that needs a hold or a claim (placing either coin offer, accepting either)

A claim follows the same steps as a hold, with CLAIM for HOLD and `claim_key` for `hold_key`; a
refused CLAIM can only be `ACCOUNT_BLOCKED`, which ends the operation `FAILED`.


1. **Pre-check** without writing: gates, fee and gold, offer state. A refusal writes nothing.
2. **T1 (Game):** the operation row takes state `COIN_HOLD_PENDING`, with `hold_key` (UUIDv7,
   derived once from the occurrence) and the request binding.
3. **HOLD** on Platform, with `issued_at` from T1, one per Account in flight (`MKTCOIN0-RL-08`), within
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
- A deliverer sends them at least once with backoff (`MKTCOIN0-RL-04`) until Platform returns a
  terminal disposition (`MKTCOIN0-DISPOSITION`, §5); one still pending after `MKTCOIN0-RL-14`
  raises `MARKET_COIN_MISMATCH` and is still resent. Platform applies each at most once by
  `instruction_id`. SETTLE and RELEASE may arrive in any order: amounts are explicit and every one
  is bounded by the hold and the claim. An ABORT is the one ordered instruction: Platform defers it
  behind the SETTLEs of its watermark (`MKTCOIN0-ABORT-ORDER`, §5).
- A replay of a player command returns the first outcome (MARKET-0 §4); the hold key and
  instruction ids come from the occurrence, so a retry never holds or settles twice.

## 7. Failure, reconciliation, audit

- **Platform unavailable:** placing coin offers and accepting coin offers need a hold or a claim
  and are refused with `COINS_UNAVAILABLE` (retryable). Matching, cancel and expiry go on; their
  SETTLE, RELEASE and ABORT wait in the outbox. An instruction
  unacknowledged after `MKTCOIN0-RL-05` (15 minutes) raises an alarm. Nothing is lost or doubled.
- **Reconciliation (`MKTCOIN0-RECONCILE`):** a daily job (and one on demand) checks each open or
  recently ended hold and claim, `MKTCOIN0-RL-06` keys per pass. It asks `STATUS` with the
  key's kind and unacknowledged outbox instruction ids; each one Platform reports `APPLIED` is
  marked acknowledged first, and each one it reports `REJECTED` is marked rejected (terminal) and
  raises `MARKET_COIN_MISMATCH`. The expected Platform state is then Game's acknowledged snapshot: Game's
  `hold_amount` (or `claim_amount`), settled, released and ended (aborted) counts and state
  minus the effects of the instructions still
  unapplied. Only a difference from that snapshot raises `MARKET_COIN_MISMATCH`; a difference
  explained by pending outbox instructions is normal lag and does nothing. A claim Platform reports `UNKNOWN`, or
  not `OPEN` while its buy offer is open, is a mismatch found here before a further fill can
  commit its gold transfer; an ended claim must show `ABORTED` with the amount Game's ABORT
  named. A mismatch stops fills
  of that offer and is corrected only by a reviewed compensating operation on the owning side
  (DUR-03 §26), never by a direct edit. A compensation that touches the effect of an instruction
  starts only after Platform reports that instruction terminal (`APPLIED` or `REJECTED`,
  `MKTCOIN0-DISPOSITION`); while it is `DEFERRED` or `UNKNOWN` the operation stays pending and
  nothing is compensated, so a late apply can never duplicate a compensated effect.
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
| `MKTCOIN0-RL-06` holds and claims per reconciliation pass | 1,000 |
| `MKTCOIN0-RL-07` Platform response size | 1,024 bytes; larger or malformed fails closed |
| `MKTCOIN0-RL-08` HOLD or CLAIM calls in flight per Account | 1 |
| `MKTCOIN0-RL-09` coin instructions per Game transaction | 2 (a fill with a held credit: SETTLE, RELEASE) |
| `MKTCOIN0-RL-10` Platform HOLD and CLAIM volume cap | per Account and per World per 24 h, set by MKTCOIN-P and recorded on #162 before activation |
| `MKTCOIN0-RL-11` Platform SETTLE volume cap | per destination Account and per World per 24 h, set by MKTCOIN-P and recorded on #162 before activation; over it the SETTLE is deferred |
| `MKTCOIN0-RL-12` Platform key, tombstone and receipt retention | at least 90 days after the key ends; restore rule `MKTCOIN0-RESTORE` (§5) |
| `MKTCOIN0-RL-13` HOLD or CLAIM request age | 10 minutes after `issued_at`; older is `STALE_REQUEST` |
| `MKTCOIN0-RL-14` pending instruction escalation | 30 days without a terminal disposition; then `MARKET_COIN_MISMATCH`, resending continues |
| Place coin sell | 0 items, 1 value line (fee), 1 hold, 1 event |
| Place coin buy | MARKET-0 buy offer lines, 1 claim, 1 event |
| Accept a coin sell offer | 0 items, MARKET-0 gold lines (2 or 3), 1 claim, 1 SETTLE, 1 event |
| A match | 0 items, MARKET-0 gold lines (2 or 3), 1 SETTLE, 1 event |
| Accept a coin buy offer | 0 items, 2 value lines (escrow fall, sale), 1 hold, 1 SETTLE, 1 event |
| Cancel or expire a coin sell offer | 0 items, 0 value lines, 1 RELEASE, 1 event |
| Cancel or expire a coin buy offer | 0 items, MARKET-0 gold return, 1 ABORT of the claim, 1 event |

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
- **SETTLE to any Account Game names.** A compromised Game server could redirect every open
  hold; Platform binds the destination by CLAIM and caps settlements (§5).
- **Hold expiry on Platform as the crash cleanup.** An expiring hold could vanish under a
  committed offer; Game's ABORT with a tombstone is explicit.
- **Refusing all coin fills while Platform is down.** Held coins are already safe; only new
  holds need Platform.

## 11. Owner-rule applications

**Global parity kept:** coins for gold on the Market; free accounts may place coin offers; the
same 2% fee; one coin balance with a transferable part (C1 a); bought coins land on the account
balance, not the Inbox; anonymous offers; the 100 offer limit; coin trades in the coin history.

**Declared differences:**
- Placing and accepting coin offers are refused while Platform is down; bought coins may reach
  the balance late (R1).
- The coin amount step is 1 until sourced (R2).
- Junior characters cannot trade coins; same-Account offers are refused (from MARKET-0).
- A coin book per World while coins are per Account.
- Every coin is transferable, with no payment-risk lock; chargebacks never reverse a Market trade
  (C2 b, the chargeback part confirmed by the owner).
- Coin trading goes live on test Worlds first, on production Worlds only after Platform's payment
  policy is accepted (C3 a).

**Architect rulings (owner rule 5905825574):**

**R1. Platform outage.** Tibia has one server; here the coins are elsewhere. a) Refuse only the
steps that need a new hold or claim (every place and accept, so Platform binds each settlement
destination, §5) and deliver settlements late (recommended: no double spend, no unbounded
settlement authority, matching, cancel and expiry keep working); b) close coin trading during an outage; c) let Game fill against an
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

Owner answers (2026-09-30, #162, Q10-Q12):

- C1 — Owner answer (2026-09-30, #162): a — the Tibia Coins are the current Oteryn Coins of the
  Platform Wallet, with a transferable part.
- C2 — Owner answer (2026-09-30, #162): b — every coin is transferable. Chargeback handling stays with
  Platform and does not reverse Market trades (stated as an assumption, confirmed by the owner on
  2026-09-30, #162 A2).
- C3 — Owner answer (2026-09-30, #162): a — test Worlds first; production Worlds once Platform
  accepts the payment policy.

## 13. Decision test

- **Must decide now:** YES. The owner asked for coin offers (Q2b) and full parity now.
- **Minimum sufficient:** one virtual ware, one hold or claim per offer or accept, three
  instruction kinds, one outbox, one reconciler, one capability, two results; MARKET-0's book, fee, gold
  escrow, locks and jobs are reused, and Platform's existing Wallet reserve pattern is extended.
- **Superseding evidence:** official Global coin amount steps.
- **Deliberately not decided:** coin gifts, Store purchases, payments, prices, the world transfer
  rule, statistics, Tournament Coins.

## 14. Before-freeze checklist

1. **Contract amendments:** MARKET-0 §3.1, §4, §7 and §10; the Store catalog owner decision follow-up 1;
   each written "pending on acceptance of MARKET-COINS-0". The Platform Wallet change is an
   external dependency (MKTCOIN-P), not edited here. The capability number is reserved at
   allocation. Owner answers C1 a, C2 b and C3 a are recorded on #162 (2026-09-30).
2. **Serialization:** MARKET-0's book lock and lock order; the operation row lock decides commit
   or abort; one HOLD in flight per Account; Platform's settlement fence and ABORT ordering by
   settlement watermark (§5).
3. **Restart:** operations, offers and instructions are durable; the deliverer and reconciler
   resume; Platform applies each key once, records a terminal disposition per instruction
   (`MKTCOIN0-DISPOSITION`) and keeps keys, tombstones and receipts through
   `MKTCOIN0-RL-12` and the closed-after-restore rule `MKTCOIN0-RESTORE` (§5).
4. **Typed references:** `MarketOfferId`, AccountId, WorldId, the ware key, hold key, claim key,
   instruction id,
   operation occurrence (`MarketOperationId`, the SETTLE `taker_operation_id`), the binding kind,
   TransactionId.
5. **Wire:** §8, capability `MARKET_COINS_V1`.
6. **Split work:** 0 items per coin transaction; at most 3 gold lines and 2 coin instructions.
