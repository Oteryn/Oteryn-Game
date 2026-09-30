# BANK-0 Bank balance

- Decision: `BANK0-ACCOUNT-WORLD-BANK-BALANCE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and economy) and protected integration. Owner question Q1 (§11) is open; §4.4 applies its
  recommended answer as a reversible assumption.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to start the bank ("bank i depozyt (drugi etap opłat, pełna
  zgodność handlu z Tibią)", 2026-09-30) and the owner answer **1b** (verbatim record on #162
  5912593702: "Wspólne dla wszystkich postaci konta"; "within one World" is the architect's reading)
- Builds on: the gold fee decision (D174-D178, §4.2 worth table), migrations `0005`, `0010`,
  `0012`, `0022`, `0023`, DUR-03 §15, §17, §18, §20, §23.1, §28, §39.1 and §39.3, the game event
  foundation registry (retention profiles), GAME-ITEM-01 §9, the composition decision §3, NPC-0
  (PR #1332: dialogue confirmation, runtime occurrence), the multichannel scope matrix, ADR-0010 §6
  and the account-progress decision (World-scoped gameplay value; D45 as the Reference-difference
  precedent), D119 (starter island departure), owner rule 5905825574
- Amends: DUR-03 §18 and §39.1 (the bank shapes, one paragraph after §18); the composition
  decision §3 (an amendment paragraph before its §6); the scope matrix bank row
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| BANK-RET-0 | control plane routes; privacy review | the retention profile of the bank event (purpose `ECONOMY_LEDGER`), in the game event foundation registry | this decision |
| BANK-1 | hard, persistence review | the balance, operation, ledger and coin-line tables and guards, the bank event and outbox, and the writer for deposit, withdraw and transfer (§3-§5) | GOLD-FEE-1b (three coins, change guards; BANK-1 rewrites the same guard functions after it); BANK-RET-0 |
| BANK-NPC-1 | impl, content review | the `Bank` service in the NPC schema and content for the banker NPCs, and the confirmed operations (§6) | BANK-1; NPC-CONTENT-1; NPC-TALK-1 |

**BANK-FEE-0** (a separate decision after NPC-0 integrates) takes stage 2 of D174: every fee
source pays coins first, then the bank. It amends the gold fee decision §4.1 and §4.3, DUR-03's
gold fee "Source (D174, stage 1)" bullet, migration `0023`'s fee record (a bank part in its
conservation CHECK, a fee paid wholly from the bank, the "coins first" proof), NPC-0 §5.1-§5.3 and
§6, and the NPC boundary §15 and §19. Nothing in this decision debits the bank for a fee.

Later, each with its own decision: coin exchange at the banker, house rent from the bank
(HOUSE-OWN), Market (MARKET-0), the Gold Pouch, the balance in the NPC trade window.

## 1. Question

Where does a bank balance live, and how do deposit, withdraw and transfer work?

## 2. Facts

**PROVEN**

- `game_character_roots.account_id` (`0005`) gives a character's Account; Account and World are
  immutable on roots, and roots are never deleted. `0022` reserves names globally by `name_key`.
- D174: fees are paid from coins and from the bank, in stages; stage 1 uses backpack coins only.
  D175-D177: three coins (gold 1, platinum 100, crystal 10,000), stacks up to 100. GOLD-FEE-1a
  (`0023`) is gold only; GOLD-FEE-1b adds platinum, crystal and the change guards.
- `0023` admits only fee records (`cause_kind = 1`), whose coin burn equals the fee plus change,
  and which commit with a Character revision advance. Backpack entry removal and item changes are
  allowed only as its lines. The `0012` mint guard needs an item-outbox row for every new item.
- DUR-03 §18: non-item value is not an ItemInstance; the owning domain defines the asset, the
  account scope, exact bounded arithmetic, causes and a versioned conversion rule. §17: each line
  has exactly one class. §15 admits only `DECAY_RETIRE` and `FeeBurnCause` as burn sinks. §39.1
  excludes non-item accounts, multiple touched items, burn, and MINT combined with other lines.
- The game event registry needs a retention profile per event; the only DUR-03 profile excludes
  economy, market and trade purposes.
- ADR-0010 §6: gameplay value is World-scoped. Coins and the purchase ledger are Platform's.
- NPC-0 (candidate): dialogue never commits value; trade and travel confirmations issue a runtime
  occurrence bound 1:1 to the confirming CommandRef. The NPC schema has no bank service;
  `content/services/bank` is empty; 137 NPCs are bankers by profession text.
- D150 and D119: the vocation is chosen on the starter island before leaving it; until the island
  exists, characters are given a mainland home town.

**CIPSOFT_OFFICIAL** (the Tibia manual, `world.md`, banks)

- One bank account per character, usable from any city after the starter island;
  `deposit <amount> gold`, `deposit all` (confirmed), `withdraw <amount> gold` (no overdraft),
  `transfer <amount> gold to <name>`.
- Newhaven, Rookgaard and Island of Destiny characters have junior accounts: no house rental, no
  Market selling, no transfers.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- `deposit all` is resolved to an amount at the prompt (`bank_system.lua:99-101`); a transfer to
  oneself is refused (`:236`); transfers are gated by town (`bank.cpp:112-124`); a withdrawal
  gives any number of crystal, platinum and gold piles (`:183-199`).

**Owner decision**

- **1b** (#162 5912593702, verbatim: question "Bank: saldo na postać czy na całe konto?", answer
  b "Wspólne dla wszystkich postaci konta"): one balance shared by the Account's characters. The
  architect reads it as within one World, because Worlds are separate economies (ADR-0010 §6). It
  is a declared Reference difference from Global's per-character account, as D45 is for quest
  completion.

## 3. Storage (BANK-1)

- **Asset.** Gold, as an integer number of gold pieces; the only asset. The conversion rule
  between coins and balance is the gold fee §4.2 worth table (versioned), as §18 requires.
- **Balance.** `game_account_bank_balances`: one row per (`account_id`, `world_id`) with `balance`
  (BIGINT, 0 to `BANK0-RL-01`) and `last_entry_id`; `account_id` references the Account guard
  table. No row means 0.
- **Operation.** `game_account_bank_operations`: one row per operation, keyed by its occurrence
  (§6), with the TransactionId, a SHA-256 binding of the whole request (kind, acting character,
  amount, recipient character, planned output slots) and the outcome. The same occurrence and
  binding replay the first outcome; a changed binding conflicts.
- **Ledger.** `game_account_bank_entries`: one immutable row per balance change: `entry_id`
  (UUIDv7), operation, account, world, kind (`DEPOSIT`, `WITHDRAW`, `TRANSFER_OUT`,
  `TRANSFER_IN`), amount (> 0), balance before and after, `acting_character_id`,
  `recipient_character_id` (a transfer only), and the counterpart entry of a transfer.
- **Coin lines.** `game_account_bank_coin_lines`: one row per touched coin item of a deposit or
  withdrawal: operation, ordinal, item, coin definition, quantity before and after, and direction
  (input or output). The entry-removal, item-change, mint-guard and placement proofs of `0010`,
  `0011`, `0012` and `0023` gain a branch that accepts a line of a committed bank operation.
- **Guards** (deferred constraint triggers):
  - the balance row equals the after value and id of its latest entry; each entry's before equals
    the previous entry's after (by `last_entry_id`, constant time);
  - the acting character, and a transfer's recipient character, is a live root of that entry's
    account and World;
  - a transfer's two entries commit together, with equal amounts, on different accounts;
  - a deposit's input worth minus its output worth equals its credit, and a withdrawal's output
    worth equals its debit (the worth table);
  - an operation has exactly the entries and lines its kind needs.
- **Grants.** `oteryn_game_runtime`: SELECT and INSERT on operations, entries and lines; SELECT,
  INSERT and UPDATE on balances; never DELETE. `oteryn_game_control`: SELECT.
- **Scope.** Account + World; the same balance on every channel of the World; another World has
  its own. Game owns it as in-game value. Platform never writes it, and it never converts to money,
  Tibia Coins or another World's balance. Atlas may read an export.

## 4. Operations (BANK-1)

### 4.1 Common rules

- Each operation is one PostgreSQL transaction with one TransactionId, fixed with its planned
  output slots before the first attempt (DUR-03 §20, §23.1).
- Lock order (composition rule 4, extended): the recovery fence and admission relations, the
  operation occurrence, the acting Character's session and guard checks (rule 2), its
  `character_root` FOR UPDATE, the main backpack, then its coin entries, then the balance rows by
  `account_id` (upsert, then FOR UPDATE). A transfer recipient's root is read without a row lock:
  its Account and World cannot change. The balance row lock serializes every writer of one
  (account, world), including another Account's transfer into it.
- No `CharacterRevision` advance (the composition amendment of §7.2).
- One bank event per operation (§5).
- A credit above `BANK0-RL-01` is refused with a typed result (`BALANCE_LIMIT`), never by a CHECK
  abort.

### 4.2 Deposit and withdraw

- **Deposit `amount`:** coins from the main backpack's direct entries are consumed by the gold fee
  payment plan's selection (D175-D177), with change as at most two fresh stacks, and the ledger
  gains `DEPOSIT amount`. At most 20 input stacks, so at most 20,000,000 gold per deposit.
  `deposit all` is resolved to an exact amount at the prompt and bound in the confirmation.
- **Withdraw `amount`:** the ledger gains `WITHDRAW amount` (never below 0), and coins are created
  in the canonical split: `floor(amount / 10,000)` crystal, then platinum, then gold, each stack at
  most 100, at most three stacks, in new main backpack entries. So one withdrawal is at most
  1,009,999 gold (`PARITY_PENDING`: Canary gives any number of piles). No room refuses the whole
  withdrawal.
- Both are value-conserving conversions, not a new value source, so no D178-style owner decision is
  needed.

### 4.3 Transfer

- `transfer <amount> gold to <name>`: the recipient CharacterId is resolved at the prompt through
  the `0022` name reservation and bound in the confirmation. The ledger gains `TRANSFER_OUT` on the
  sender's (account, world) and `TRANSFER_IN` on the recipient's, in one transaction; the
  recipient may be offline and needs no fence, because only its Account's balance changes.
- Refused: an unknown name, another World, a recipient of the same Account (the balance is
  already shared; Canary also refuses a transfer to oneself), a junior sender or recipient, an
  amount above the balance, a credit above `BANK0-RL-01`.

### 4.4 Junior characters

- **Definition** (architect application of Global, which ties junior status to the island): a
  character is junior until it has left the starter island, the departure fact of D119 and
  DAWNPORT-1. Until that island and fact exist, no character is junior.
- **Use** (assumption pending Q1, answer b): a junior character cannot use the bank at all; it
  pays with coins only. So a new character cannot draw on the Account's shared gold, which is what
  Global's junior rule prevents.

## 5. DUR-03 classes, causes and event (BANK-1)

- **Classes (§17).** Every coin input of a deposit, its change outputs, and every coin output of a
  withdrawal is a `CONVERSION` line; the balance side is a `CONVERSION` value line. A transfer is
  two `TRANSFER` value lines. There is no BURN and no MINT in any bank shape.
- **Causes.** Closed `BankConversionCause {Deposit | Withdraw, occurrence}` and
  `BankTransferCause {occurrence}`.
- **Value line.** A closed message: entry, asset (`gold`), account, World, kind, class, amount,
  balance before and after.
- **Event.** One bank event per operation, carrying its item lines (for a deposit or withdrawal)
  and its value lines, in a bank outbox. Its retention profile (purpose `ECONOMY_LEDGER`) is
  decided by BANK-RET-0 before BANK-1.
- **Supersession.** For the bank shapes only, the §39.1 exclusions of non-item accounts, multiple
  touched items, burn and MINT combined with other lines. The bank shapes have no burn, so §15's
  closed list of sinks is unchanged. Every other §39 obligation is unchanged.
  `DUR03-RL-03` stays 0 for every existing shape.
- Existing fee records (`0023`) are unchanged.

**Amendment (pending on acceptance of MARKET-0 (#1367); `OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md` §6, §8).** The ledger gains `MARKET_ESCROW`,
`MARKET_ESCROW_RETURN`, `MARKET_PURCHASE`, `MARKET_SALE`, `MARKET_HELD_CREDIT` and `FEE_DEBIT` entries that reference a
Market operation; an entry references exactly one of a bank operation, a fee record or a Market
operation (a house operation after HOUSE-OWN-0), and gains `counterparty_character_id`; the acting
character is NULL for a job step. Market credits (sales and escrow returns) may exceed `BANK0-RL-01`
up to 9,000,000,000,000,000; §4.1's typed refusal still holds, now at that ceiling, checked before
the write, so no CHECK aborts. A deferred guard per Market operation sums ledger deltas, the change
of `escrow_gold` and the fee burn to 0. The Market fee is a burn of a Market shape; the bank shapes
still have none.

## 6. Banker NPCs (BANK-NPC-1)

- The NPC authoring schema gains a `Bank` service; BANK-NPC-1 generates it for the banker NPCs,
  with the Canary bank-module NPCs as provenance.
- The dialogue parses `balance`, `deposit <amount> gold`, `deposit all`, `withdraw <amount> gold`
  and `transfer <amount> gold to <name>`, within bounded lengths, refuses amounts above the
  operation's cap at the prompt, and asks for confirmation. The prompt binds the operation, the
  exact amount and the recipient CharacterId, with one pending invocation and its own timeout
  `BANK0-RL-04` (as travel's `NPC0-RL-06`).
- The confirming `yes` (NPC-0 command 7) issues one runtime occurrence bound 1:1 to its
  CommandRef, as travel does, and the NPC runtime calls the bank writer with it. This extends
  NPC-0 §5.1's occurrence rule to the bank service. The dialogue commits nothing.
- Any banker in any town serves the same balance, told in dialogue replies.

## 7. Other amendments

### 7.1 DUR-03

A paragraph after §18 names bank gold as the first non-item asset and admits the shapes of §5, with
the supersessions stated there.

### 7.2 Composition decision and scope matrix

- An amendment paragraph before the composition decision's §6: rule 1 also covers bank ledger entries, operations and coin lines: those of the acting
  character's (account, world) and a transfer's counterpart entry on another account. They are
  DUR-03 value records keyed by the operation, not Character receipts. Rule 4's lock order is
  extended as in §4.1.
- The scope matrix bank row becomes: Game bank ledger; Account + World; strong durable; the same
  balance on all channels of the World; shared.

## 8. Rows (registered by BANK-1 before implementation)

| Row | Value |
|---|---|
| `BANK0-RL-01` balance maximum | 999,999,999,999 gold |
| `BANK0-RL-02-DEPOSIT` | 1 to 20,000,000 gold, at most 20 input stacks and 2 change stacks |
| `BANK0-RL-02-WITHDRAW` | 1 to 1,009,999 gold, at most 3 output stacks |
| `BANK0-RL-02-TRANSFER` | 1 to 999,999,999,999 gold |
| `DUR03-RL-03-BANK` value lines | 1, or 2 for a transfer |
| `DUR03-RL-01-BANK` touched items | 22 for a deposit, 3 for a withdrawal, 0 for a transfer |
| `BANK0-RL-03` ledger entries per operation | 2 |
| `BANK0-RL-04` confirmation timeout | as `NPC0-RL-06` |

## 9. Rejected options

- **Reusing the `0023` fee record for a deposit.** A deposit is not a fee; its conservation
  equation and cause differ.
- **Burn and mint for deposit and withdraw.** They would be a sink and a new value source; a
  conversion conserves value.
- **Coins stored as items in a bank container.** DUR-03 §18 keeps non-item value out of
  ItemInstance.
- **A per-character balance (Global), or one across Worlds.** The owner chose 1b; value is
  World-scoped (ADR-0010 §6).
- **A balance column without a ledger.** Replay, audit and reconciliation need each change.
- **Junior by vocation.** The vocation is chosen on the island, so a vocation says nothing about
  having left it.
- **Fees from the bank in this decision.** They amend NPC-0 and `0023` in many places; BANK-FEE-0
  does it after NPC-0 integrates.

## 10. Decision test

- **Must decide now:** YES. The owner asked for the bank now, and BANK-FEE-0 builds on it.
- **Minimum sufficient:** one asset, balance, operation, ledger and coin-line tables, three
  operations, one event.
- **Superseding evidence:** an owner change of the scope, or the Q1 answer.
- **Deliberately not decided:** fees from the bank (BANK-FEE-0), coin exchange, house rent, Market,
  the Gold Pouch, a character moved to another Account or sold (the balance stays with the
  Account), interest (none in Tibia).

## 11. Owner question

**Q1. May a starter-island (junior) character use the Account's shared bank balance?**
Context: with 1b the balance is shared by all characters of the Account on a World; Global's junior
accounts exist so a new character cannot use the main character's gold.
a) Yes, fully; b) no bank use until it leaves the island, coins only (recommended); c) fees only.

## 12. Before-freeze checklist

1. **Contract amendments:** DUR-03 §18 and §39.1; the composition decision §3 (paragraph before
   its §6); the scope matrix bank row. Each is written "pending on acceptance of BANK-0" (#162
   5912405163). NPC-0 §5.1's occurrence rule is extended to the bank service (§6).
2. **Serialization:** one transaction per operation, the lock order of §4.1, replay by occurrence
   and binding.
3. **Restart:** balances, operations, ledger and lines are durable.
4. **Typed references:** AccountId, WorldId, CharacterId, occurrence, TransactionId.
5. **Wire:** none beyond NPC-0 dialogue.
6. **Split work:** one operation per transaction, at most two ledger entries.
