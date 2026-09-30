# BANK-FEE-0 Fees from the bank

- Decision: `BANK-FEE0-COINS-THEN-BANK-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and economy) and protected integration. It builds on BANK-0 (PR #1357) and integrates after it.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: stage 2 of the owner's D174 ("fees are paid from coins and from the bank, as in Tibia,
  in stages"), and the owner's direction "drugi etap opłat, pełna zgodność handlu z Tibią"
  (2026-09-30)
- Builds on: the gold fee decision (D174-D178, §4.1-§4.5 with the NPC-0 amendments), BANK-0 (balance,
  ledger, value-line message, junior rule and Q1, BANK-RET-0), NPC-0 (merged: BUY and travel), the
  NPC service boundary §9.2 and §15, DUR-03 §15, §17, §18, §39.1 and §39.3, migrations `0010` and
  `0023`, the composition decision §3, owner rule 5905825574
- Amends: the gold fee decision §4.1, §4.2, §4.3 and §4.5; DUR-03 §39.3 (a bank fee paragraph after
  the gold fee amendment); NPC-0 §5.1, §5.3 and §11; the NPC service boundary §9.2; BANK-0 §3, §5
  and §7.2 (pointers added when this PR integrates after #1357)
- Runtime, migration and production authority: NONE. The child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| GOLD-FEE-2 | hard, persistence review | the bank part of every fee (§3-§5): the `0023` and `0010` widening, the Rust writer and audit changes, the `FEE_DEBIT` ledger entry | BANK-1; BANK-RET-0; GOLD-FEE-1b; for BUY and travel, NPC-TRADE-1 and NPC-TRAVEL-1 (whichever lands later takes the bank part) |

## 1. Question

How does a fee fall back to the bank when the carried coins are not enough?

## 2. Facts

**PROVEN**

- D174 (owner): fees are paid from coins and from the bank, in stages; stage 1 uses coin stacks in
  the main backpack's direct entries only. Its stage 1 text keeps `DUR03-RL-03` at 0.
- Gold fee §4.2: the plan rejects when the eligible worth `T < F`; burns inputs by worth ascending,
  then display order; at most the last input is partly burned; change is at most two stacks; `F` is
  at most 20,000,000.
- Gold fee §4.3: one transaction, one receipt (the source's Character receipt, or the item-only
  source's DUR-03 cause record) binding the TransactionId, cause, `F` and change. §4.5: one event
  per transaction (`DUR03-RL-07-EVENTS` = 1).
- `0023`: `line_count`, `burned` and `fee` are at least 1; the backpack item is NOT NULL and must be
  the equipped one; `burned - change = fee`; the audit event is found through the first burn line.
  `0010`: the audit outbox needs an item. The Rust audit forces `value_lines` to 0 and checks
  `burned == fee` and the 20,000,000 cap.
- The fee sources are `CharmUnassign`, `NpcTrade` (BUY) and `NpcTravel` (D178, D208). CHARM-6 is not
  built yet.
- BANK-0 (candidate): a balance per (Account, World), an immutable ledger keyed by bank operations,
  a closed value-line message, junior characters without bank use (Q1 pending), the bank event's
  retention decided by BANK-RET-0.

**CIPSOFT_OFFICIAL** (the Tibia manual)

- An NPC trade shortfall is paid from the bank (`controls_trading.md`); a boat fare is taken from
  the bank when carried gold is not enough (`world.md`). This is current-manual continuity;
  target-dated evidence stays `PARITY_PENDING` (NPC boundary §9.2).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Carried stacks are removed whole while their worth is below the amount, and the rest is taken
  from the balance (`player.lua:133-142`, `game.cpp:3180-3186, 3343`); the same for charms
  (`iobestiary.cpp:540`) and travel (`modules.lua:545, 592`). Canary has no junior gate; Oteryn's
  is BANK-0's. Canary counts coins in nested bags; Oteryn's stage 1 source has none (B3 depth 1).

## 3. The rule (GOLD-FEE-2, amends gold fee §4.1 and §4.2)

- `T` = the worth of the eligible inputs (gold fee §4.2: the three coins, compatible revision,
  default state, direct entries of the main backpack).
- **`T >= F`:** gold fee §4.2 unchanged; the bank is not touched.
- **`T < F`** (replaces step 1's rejection for a non-junior payer): every eligible input is burned
  whole, in §4.2's order and line numbering, no change is minted, both change slots stay unused,
  and `F - T` is debited from the payer's (Account, World) balance. If the balance is lower, the
  fee is refused as `InsufficientFunds` and nothing is written. Ineligible stacks are never
  touched.
- A junior character (BANK-0 §4.4) keeps stage 1: `T < F` is refused.
- `F` is no longer capped at 20,000,000; it is bounded by `T` plus `BANK0-RL-01`. The backpack holds
  at most 20 entries, so every eligible input fits in one plan.
- The payer needs no backpack when `T` = 0.

## 4. Records (GOLD-FEE-2)

### 4.1 Fee record (`0023`)

- New column `bank_debit_gold_units` (0 when the bank is not used). The CHECK becomes
  `burned - change + bank_debit = fee`.
- `line_count` and `burned` may be 0 only when `bank_debit = fee`; the backpack item may be NULL
  only then.
- The "coins first" guard: `bank_debit > 0` requires every line whole (`quantity_after = 0`),
  `change = 0`, both change slots unused, and no eligible input left untouched. The guard branches
  never rely on a NULL first item or last ordinal.
- The root-advance guard composes with both the charm branch (Character receipt) and the
  NPC-TRADE-1 and NPC-TRAVEL-1 item-only branch.
- The bank part is an outcome, not part of `request_binding`: after a known abort it is
  recalculated with the rest of the plan; after an ambiguous commit the occurrence replay returns
  the first outcome.

### 4.2 Ledger entry (BANK-0 amendment)

- One ledger entry of kind `FEE_DEBIT` and amount `F - T`. It has no bank operation; instead it
  references the fee record's TransactionId. A CHECK allows exactly one of the two references.
- A same-transaction guard both ways: `bank_debit > 0` if and only if exactly one `FEE_DEBIT` entry
  exists for that fee, with an equal amount, the payer root's Account, the record's World, and the
  payer as acting character.
- The balance row is locked after the backpack and its coin entries, as BANK-0 §4.1 orders it.

### 4.3 Audit (`0010`, gold fee §4.5)

- The fee event stays the one event of the transaction; a `FEE_DEBIT` emits no BANK-0 bank event.
- It carries one value line for the bank part, in BANK-0's closed value-line message (kind
  `FEE_DEBIT`, class BURN, balance before and after). `DUR03-RL-03-FEE` = 1 when used, else 0; the
  coin-only shapes keep `DUR03-RL-03` = 0, as D174's stage 1 text says.
- A fee event with zero burn lines is linked through its fee record; the outbox's item column may be
  NULL only for it.
- The Rust audit (`item_fee_burn_audit.rs`) accepts the value line and the new conservation, drops
  the 20,000,000 cap, and the envelope ceiling is re-measured with the value line.
- Its retention is covered by BANK-RET-0 (the value line holds balances), a dependency of
  GOLD-FEE-2.

## 5. Per source

- **CharmUnassign:** the Character receipt binds the bank part. CHARM-6 builds on this directly.
- **NPC BUY** (NPC-0 §5.3): the result must still fit the backpack; the 20,000,000 line is replaced
  by §3.
- **NPC travel** (NPC-0 §6): the same, as the Tibia boat fare rule says.
- No wire change: the server decides; a later wire decision may show the bank balance in the trade
  window (the Tibia client shows two separate balances).

## 6. Amendments made by this decision

- **Gold fee decision:** pointers at §4.1 (stage 2), §4.2 step 1 (the bank remainder), §4.3 (the
  receipt binds the bank part) and §4.5 (the value line).
- **DUR-03 §39.3:** a "bank fee" paragraph after the gold fee amendment: for the fee shapes only, it
  supersedes the §39.1 exclusion of non-item accounts, sets `burned - change + bank_debit = F`,
  names the `FEE_DEBIT` value line (class BURN, under the fee's `FeeBurnCause`, consistent with §15
  and §18) and the `DUR03-RL-03-FEE` row, and replaces the Coins bullet's "insufficient funds"
  rule for a non-junior payer.
- **NPC-0:** pointers at §5.1 (the bank is no longer out of scope), §5.3 (the cap) and §11 (the bank
  is decided).
- **NPC service boundary §9.2:** the bank owner is BANK-0, and the fallback is this decision.
- **BANK-0** (after #1357): §3 gains the `FEE_DEBIT` kind and the fee reference; §5's "existing fee
  records unchanged" and "RL-03 stays 0" are narrowed to the bank shapes; §7.2's composition
  paragraph also covers `FEE_DEBIT` entries.
- **D178** is not touched: no new fee source, no new sink.

## 7. Rejected options

- **Bank first, then coins.** Tibia and Canary take coins first.
- **Partial coin use with change plus a bank remainder.** It would mint change while debiting the
  bank; burning every eligible coin whole matches Canary.
- **A separate bank event for the fee.** One transaction has one event.
- **A new fee cause for the bank part.** It is the same fee; its cause keys both parts.
- **A funds field in the NPC trade window now.** The server decides; a later wire decision can add
  the two balances as the Tibia client shows them.

**Amendment (pending on acceptance of MARKET-0 (#1367)).** The Market placing fee (owner decision D238) is taken from the bank
only, as Tibia does: a declared exception to §3's coins-first rule. It is one `FEE_DEBIT` ledger
entry that references a Market operation instead of a fee record (`OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md` §4, §8).

## 8. Decision test

- **Must decide now:** YES. The owner asked for the second fee stage now, and NPC trade and travel
  are about to be built.
- **Minimum sufficient:** one fallback rule, one column and CHECK change, one ledger kind, one value
  line.
- **Superseding evidence:** an owner answer to BANK-0 Q1 other than b (then juniors may use the
  bank for fees).
- **Deliberately not decided:** the Gold Pouch, blessing and promotion fees (their own sources),
  house rent and Market fees (their own decisions), the balance in the trade window.

## 9. Before-freeze checklist

1. **Contract amendments:** §6, each written pending on acceptance of BANK-FEE-0 (#162 5912405163).
2. **Serialization:** the fee transaction as today, then the balance row lock.
3. **Restart:** replay by the fee's occurrence, including the bank part.
4. **Typed references:** the existing fee causes; the payer's AccountId and WorldId; the fee
   TransactionId on the ledger entry.
5. **Wire:** none.
6. **Split work:** one fee, one transaction, at most one ledger entry.
