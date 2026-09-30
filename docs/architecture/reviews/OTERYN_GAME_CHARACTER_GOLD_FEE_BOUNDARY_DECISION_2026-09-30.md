# Character gold fee boundary decision

- Decision: `CHARACTER-GOLD-FEE-BOUNDARY-V1`
- Status: **ACCEPTED FOR IMPLEMENTATION once protected-integrated**, with owner decisions D174-D178
  taken (§2). Acceptance requires exact-head validation, independent review and protected
  integration.
- Task: `OTV2-20260930-gold-fee-boundary-decision` (GOLD-FEE T0), allocated on #162
  ([5905498654](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5905498654))
- Source escalation: `ARCHITECTURE_ESCALATION_REQUIRED` for the CHARM-6 gold fee (D170), #162
  [5905416975](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5905416975)
- Amends: DUR-03 contract §15 and §39.1/§39.3 (typed BURN for named fee causes; the mixed
  Character + value composition); pointer notes in the DUR-02 schema packet §7.5 and
  GAME-ITEM-01 §9
- Admission baseline: `main@cf251bb2` (escalation prepared at `main@970e30a7`)
- Runtime, migration, registry, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

CHARM-6 (D170) needs the Tibia charm unassign fee: the Character changes its charm state and pays
gold in the same step. Three accepted texts block it (#162 5905416975):

- DUR-03 §39.1/§39.3 admit only MINT, TRANSFER and the named `DECAY_RETIRE`. Burn is excluded.
- DUR-02 schema packet §7.5: a path that changes both Character and items waits until
  GAME-ITEM-01/DUR-03 prove the cross-domain boundary.
- Gold exists only as the physical item `oteryn:item.tibia.i3031`, whose stack maximum B3 did not
  admit.

What does a fee debit, how is change paid, and how does the fee compose with the Character change?

## 2. Owner decisions

Questions Q35-Q39: #162 [5905416975](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5905416975).
Answers taken as D174-D178: #162 [5905498654](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5905498654)
(2026-09-30). Numbering continues after D165-D173 (#1310).

| # | Q | Decision | Owner choice (2026-09-30) |
|---|---|---|---|
| D174 | Q35 c + a | Fees are paid from coins and from the bank, as in Tibia, in stages. Stage 1 debits coin items in the character's inventory only. The bank ledger is a later, separate economy contract and task. Charm release to players may follow stage 1. | 35c, 35'a |
| D175 | Q36 b | Gold, platinum and crystal coins (`i3031`, `i3035`, `i3043`). Change is minted back as coins; if the change does not fit, the whole transaction is rejected. | 36b |
| D176 | Q37 a | `max_stack` = 100 for `i3031`, `i3035` and `i3043`, from Reference. | 37a |
| D177 | Q38 a | One atomic transaction carries the Character change and the BURN lines, with one TransactionId, one receipt and the full gameplay fence. | 38a |
| D178 | Q39 a | A closed typed BURN cause per fee source (`CharmUnassign { charm, occurrence }`, extendable only by contract amendment); at most 20 input stacks plus the change outputs (see the interpretation below); `DUR03-RL-03` stays 0. | 39a |

**Control-plane interpretation (2026-09-30, T0 repair).** D175 governs the change: change is
minted back, and the transaction is rejected only when the change does not fit. A change below one
crystal coin can need a platinum stack and a gold stack, so the "+ 1 change output" in the Q39
option text was a wording error, not an owner limit. The fee shape therefore admits **at most 2
change outputs** (one platinum stack and one gold stack) beside at most 20 input stacks.

## 3. Facts

**PROVEN** (`main@cf251bb2`)

- DUR-03 §15: a burn names the affected item and quantity, a typed sink/cause, the survivor or
  retirement result and its evidence; silent deletion or `quantity = 0` live state is not a sink.
  §17: `BURN` and `MINT` are conservation classes with an explicit sink/source. §39.1 and the D3
  paragraph of §39.3 exclude every burn except `DECAY_RETIRE`.
- DUR-03 §18 and GAME-ITEM-01 §9: a physical coin is an ItemInstance under DUR-03 conservation; a
  bank balance is non-item value owned by an economy contract and is not forced into ItemInstance.
- `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1` §3: an item-only DUR-03 transaction does not
  advance `CharacterRevision`; its fence is the Character writer's (rules 2-4). §3.6: a Character
  semantic write and DUR-03 item effects may share one transaction, the Character part advancing
  the revision once with its receipt; that decision did not allocate such a transaction.
- Migration `0020_character_charm_state.sql` (CHARM-3, #1307): each charm command is one fenced
  Character transaction that advances `CharacterRevision` by one with one immutable receipt keyed by
  the command occurrence (UUIDv7); assignment rows are immutable until CHARM-6.
- B3 (D80-D83): the reachable inventory is the equipped main backpack and its direct entries
  (`GAMEITEM01-CONTAINER-ENTRIES-MAX` = 20, `GAMEITEM01-PLACEMENT-DEPTH` = 1); stack ceiling
  `GAMEITEM01-STACK-QUANTITY-MAX` = 100; a placement takes the highest live ordinal plus one under
  the `character_root` lock; display order is highest ordinal first.
- `RESOURCE_LIMITS_REGISTRY.json`: `DUR03-RL-01` = 2, `DUR03-RL-02` = 2, `DUR03-RL-03` = 0,
  `DUR03-RL-06` = 2 participants / 6 work units, `DUR03-RL-07-EVENTS` = 1, payload 7,936 B,
  envelope 9,216 B (under ANL payload 196,608 B / envelope 262,144 B). The offline evidence test
  `apps/game-server/examples/dur03_native_one_item_audit.rs` asserts `DUR03-RL-01` = 2 against
  the registry and the runtime code checks a constant of 2.
- Content (`content/items/definitions/items-17500-17999.json`): `i3031` "gold coin", `i3035`
  "platinum coin", `i3043` "crystal coin" are `materializable`, `StackCapable`, with
  `semantics.stack` `UNKNOWN`. Each is bound `EXACT` to CrystalServer server id 3031/3035/3043
  (`imports/crystalserver/bindings/items.json`, source revision `ff7ede59`).
- Reference (tibia.com manual, capture 2026-09-28): `docs/reference/tibia-manual/world.md` §banks,
  "1 platinum = 100 gold; 1 crystal = 100 platinum = 10,000 gold"; official facts
  `imports/official/tibia-com/2026-09-28-160207Z/facts.json` `world.5-2-the-world-of-tibia.13`
  (1 platinum represents 100 gold) and `products.6-3-available-products.15`/`.16` (the Gold
  Converter changes "a stack of 100 gold pieces" into 1 platinum and "a stack of 100 platinum
  coins" into 1 crystal).

**DERIVED**

- Crystal coin stack of 100: not stated by a captured source; it follows the general Tibia ceiling
  (manual `magic.md`, runes "stack up to 100 per pile"), B3 D82 and the 100-platinum exchange.
  D176 admits it as an owner decision on this evidence.
- The payment plan, change rule and resource rows of §4.2-§4.6.

**UNKNOWN**

- Global's exact coin selection order when paying from inventory, and its behaviour when change
  does not fit. Oteryn fixes its own deterministic order (§4.2).

**CONFLICT**: none used. (Coin weights conflict across sources; weight is not checked, B3 D81.)

## 4. Decision

### 4.1 What a fee debits (D174)

- **Stage 1 (this decision):** a server fee debits physical coin stacks that are live direct
  entries of the character's equipped main backpack (B3 D80). Nothing else is a source: no Ground,
  no other character, no nested bag, no depot, no bank, no Gold Pouch.
- **Stage 2 (later):** a non-item bank ledger with the Tibia "coins first, then bank" order needs
  its own economy contract and task (DUR-03 §18, GAME-ITEM-01 §9). Nothing here defines a ledger,
  and `DUR03-RL-03` stays 0.
- Player-facing charm release may follow stage 1 (D170 gate, now satisfiable by stage 1).

**Amendment (BANK-FEE-0, 2026-09-30), pending on acceptance of BANK-FEE-0.** Stage 2 is decided in
`OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md`: coins first, then the bank.

### 4.2 Denominations, payment plan and change (D175, D176)

Coins are exactly three definitions, with a closed worth table in gold units:

| Coin | Key | Worth | `max_stack` (D176) |
|---|---|---|---|
| gold coin | `oteryn:item.tibia.i3031` | 1 | 100 |
| platinum coin | `oteryn:item.tibia.i3035` | 100 | 100 |
| crystal coin | `oteryn:item.tibia.i3043` | 10,000 | 100 |

Any other key is not a coin. Changing the table needs an amendment of this decision. An eligible
input is a live stack of one of these keys, at the compatible definition revision, with state equal
apart from quantity and `1 <= quantity <= 100`. It has no instance state other than its quantity:
its state is the definition's default state (review hardening).

The owning source supplies the fee `F` in gold units, computed inside the same transaction from
facts read after the `character_root` lock (for CharmUnassign, the rule owned by CHARM-6). `F >= 1`;
arithmetic is exact, unsigned 64-bit and checked (the reachable maximum is 20 x 100 x 10,000 =
20,000,000). The plan is a pure function of durable state, never of client order or UUID:

1. `T` = sum of `quantity x worth` over eligible inputs. If `T < F`, reject
   (`InsufficientFunds`); nothing is written, including the Character change.
   *Amendment (BANK-FEE-0, pending on acceptance):* for a non-junior payer, `T < F` instead burns every eligible input
   whole, mints no change, and debits `F - T` from the payer's bank balance; only a balance below
   `F - T` rejects. `F` is then bounded by `T` plus the bank maximum, not 20,000,000.
2. Order inputs by worth ascending, then by display order (highest placement ordinal first, B3
   §4.1). With remaining `R = F`, walk the inputs while `R > 0`: burn `k = min(quantity,
   ceil(R / worth))` units; if `k x worth >= R`, the change is `C = k x worth - R` and `R = 0`,
   otherwise `R -= k x worth`.
3. Consequences: at most one input is partly burned (the last); every earlier input is burned
   whole. Change arises only on the last input and `C < worth` of that input, so it is never a
   crystal coin.
4. **Change outputs (at most 2).** `C = 0`: no output. Otherwise the change is `floor(C / 100)`
   platinum coins and `C mod 100` gold coins, each output present only when its count is positive.
   Since `C < 10,000`, each count is at most 99, so each output is one stack within `max_stack`.
   All stacks of both change denominations were burned whole in step 2 (they have lower worth
   than the last input), so no merge target exists: each output is one fresh ItemInstance in a new
   backpack entry, placed after the burn lines, platinum first and then gold, each taking the next
   ordinal. If the backpack, counted after the burn lines, has fewer free entries than outputs,
   reject (`ChangeDoesNotFit`).
5. A plan needing more than 20 inputs is rejected (`CAPACITY_EXCEEDED`), even though B3 makes it
   unreachable today.

Every rejection writes nothing. Conservation: `sum(burned units x worth) - C = F` exactly; the net
value destroyed equals the fee. Each change output is a MINT under the same cause, not a CONVERSION rule
and not a bank exchange.

D176 is the typed stack maximum of those three definitions (GAME-ITEM-01 §4.1, within
`GAMEITEM01-STACK-QUANTITY-MAX`). The content write (`semantics.stack` known, 100) goes through
the item-authoring route in GOLD-FEE-1b, citing this decision (§6: GOLD-FEE-1a admits only gold
coins, whose stack maximum is not raised by it, so no change output exists there).

### 4.3 One atomic transaction (D177)

One PostgreSQL transaction in `oteryn_game` (DUR-03 §35) commits all of the following or none:

- the fee source's Character change and its Character receipt, advancing `CharacterRevision`
  exactly once (DUR-02 rule 2). The item lines add no second advance;
- every BURN line and the change MINT line of §4.2;
- the mandatory DUR-03 audit event (§4.5).

**One TransactionId and one receipt.** The transaction has one stable TransactionId, fixed before
commit can become ambiguous and reused on retry (DUR-03 §§20, 23). Its one receipt is the fee
source's Character receipt, keyed by the source occurrence; it binds the TransactionId, the typed
cause, `F`, and the change. The BURN and MINT lines are not a second receipt and have no idempotency
key of their own. A replay of the same occurrence and binding returns the first outcome (Character
change, burn and change together); a changed binding conflicts; the same occurrence can never burn
twice.

**Retry identity (review hardening).** The TransactionId also fixes two planned output identity
slots, one platinum and one gold ItemInstanceId, before the first attempt. A retry after a known
abort may rematerialize the burn set and the change counts from the then-current state, but reuses
the same TransactionId and the same two slots; a slot the plan does not need stays unused. After an
ambiguous commit, resolution is the occurrence replay above, never a new attempt.

**Full gameplay fence.** The Character writer's complete fence and lock order, as in
`commit_character_experience` and `commit_charm_command` (composition decision §3 rules 2-4): the
recovery fence and admission-relation locks; the occurrence lock and replay; the reconnect-session
row (GameSession, Character, World, runtime scope, `current_generation` = the fence's
`connection_generation`, lease generation, scope ownership generation, `session_state IN (1,2)`);
the runtime-scope assignment and node incarnation; the admission guards; `character_root FOR
UPDATE` with the expected `CharacterRevision`; then the domain rows: the fee source's rows, then
the main backpack's container row and its coin entries. The cause's CharacterId must equal the
fenced Character. A stale session generation, lease, scope or revision commits nothing.

**Amendment (NPC-0, 2026-09-30): item-only fee sources.** An NPC BUY and an NPC travel fee
(§4.4 as amended) have no Character change. For them the one receipt of this section is the
source's DUR-03 cause record keyed by (occurrence, character); nothing advances
`CharacterRevision` (Character and item composition decision §3 rule 1). The fence is the item
writer's (composition §3 rules 2-5): the full session fence and `character_root FOR UPDATE`
without an expected revision, then the source's rows, then the main backpack and its coin
entries. One transaction still carries every line, one TransactionId fixes the output slots, and
a replay of the same occurrence returns the first outcome. This is the architect's reading of D177
for sources the owner admitted with Q1a (#162 5909366267).

**Amendment (BANK-FEE-0, 2026-09-30), pending on acceptance of BANK-FEE-0.** The one receipt also
binds the bank debit, and the
transaction also writes the bank `FEE_DEBIT` ledger entry, locking the balance row after the coin
entries (`OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md` §4).

**Burn first with refund** (Q38 b) is rejected: it creates a window where gold is gone and the
Character change is not made, and a compensation path.

### 4.4 Typed BURN cause (D178)

The cause is a closed type, one variant per admitted fee source:

```text
FeeBurnCause = CharmUnassign { charm: CharmKey, occurrence: CharmCommandOccurrenceId }
```

- `charm` is the charm key (`oteryn:charm.<name>`, CHARM-3); `occurrence` is the UUIDv7 of the
  unassign command, the same occurrence that keys the charm receipt.
- No generic fee cause, reason code or free-form label. A new fee source (for example the D170
  full charm reset) is admitted only by an amendment of DUR-03 §39.3 and this decision, with its
  own owner decision.
- The fee amount, discounts and eligibility belong to the source (CHARM-6 for CharmUnassign);
  DUR-03 only conserves the value.

**Amendment (MARKET-0, owner decision D238), pending on acceptance of MARKET-0 (#1367).** The variant
`MarketFee {offer_id, occurrence}` is added: 2% of the offer total, 20 to 1,000,000 gold, paid only
from the bank as one `FEE_DEBIT` entry referencing a Market operation, never from coins and never a
`0023` fee record (`OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md` §3.2, §4).

**Amendment (NPC-0, 2026-09-30, owner decision D208, Q1a on #162 5909366267).** Two variants are added:
`NpcTrade(NpcTradeCause {npc, offer, side, occurrence})` for the coins of an NPC BUY (its `side`
is always BUY) and
`NpcTravel {npc, route, occurrence}` for an NPC travel fee
(`OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md` §5-§6). Both are item-only fee
sources under the §4.3 amendment above: a DUR-03 cause record keyed by (occurrence, character),
no `CharacterRevision` advance, the item writer's fence. NPC-TRADE-1 and
NPC-TRAVEL-1 widen `0023` (fee source kinds, the root-advance requirement for an item-only source,
and the entry-removal proof) in their own migrations.

### 4.5 Audit evidence

One event per logical transaction (`DUR03-RL-07-EVENTS` = 1), one closed aggregate with the §39
evidence:

- each BURN line: ItemInstanceId, type, quantity before and after, location before (the backpack
  `Container` entry) and after (the same entry for a partial burn; `RETIRED`, no location, for a
  whole burn, the §11.4/§11.5 terminal shape);
- each change MINT line (at most 2): explicit nonexistence before, the live item in its new entry
  after, type and quantity;
- cause, `F`, the worth table key set, and the conservation summary of §4.2;
- WorldId, runtime scope, the Character and its committed `CharacterRevision`, compatible
  definition revisions, and safe fence references without secrets.

*Amendment (BANK-FEE-0, pending on acceptance):* the event also carries one value line for a bank part (kind
`FEE_DEBIT`, class BURN), and an event of a fee paid wholly from the bank has no burn line.

The schema (a new closed operation of the native item transaction family), field numbers and
registry entries are not defined here; GOLD-FEE-1 registers them under the §39.2 non-candidate,
no-`_fixture` conditions. Burn stays excluded for every cause other than `CorpseDecay`
(`DECAY_RETIRE`), the `FeeBurnCause` variants and, with the NPC-0 amendment (D208), the SELL sink
`NpcTradeCause`.

### 4.6 Resource rows

Per-shape values for the fee transaction. The other shapes keep their values.

| Row | Fee shape | Derivation |
|---|---|---|
| `DUR03-RL-01` touched items | **22** | 20 inputs + 2 change outputs (amended) |
| `DUR03-RL-02` location lines | **22** | 20 whole-burn removals + 2 change placements (amended) |
| `DUR03-RL-03` value lines | **0** | unchanged; coin quantities are item state (B3 §4.4) |
| `DUR03-RL-04` transform | 0 | unchanged |
| `DUR03-RL-05` container expansion | 0 | unchanged; direct backpack entries only |
| `DUR03-RL-06-PARTICIPANTS` | **22** | one per touched item (amended) |
| `DUR03-RL-06-EFFECT-WORK-UNITS` | **64** | whole burn = participant + removal + retirement = 3; partial burn 2; change MINT 2; worst case 20 x 3 + 2 x 2 (amended) |
| `DUR03-RL-07-EVENTS` | 1 | unchanged |
| `DUR03-RL-07` payload / envelope | measured by GOLD-FEE-1 | exact protobuf worst case of the registered schema by the D50 method; must stay within ANL payload 196,608 B / envelope 262,144 B |
| `DUR03-RL-08` retry work | 3 | unchanged |

Rough, non-binding estimate: at the registered field bounds a line is under 2 KiB, so the payload
is about 45 KiB. If the measured worst case exceeds the ANL ceilings, the shape comes back for a
decision; it is never split into several events or truncated.

**Registration is deferred to GOLD-FEE-1**, under the registry single-writer lease, with max and
max+1 tests, as B3 §4.5 did through B3-1. This decision does not edit the registry: the offline
evidence test and runtime constants bind `DUR03-RL-01` = 2 today, so a registry-only change would
break them, and would widen the other shapes without their code.

## 5. DUR-03 amendment

For the `FeeBurnCause` shape only, this decision supersedes the following. Besides the D3
`DECAY_RETIRE` cause, the admitted burn sink is the closed `FeeBurnCause` of §4.4 (D174-D178); the
DUR-03 §15 pointer says so.

- §39.1 and the D3 paragraph of §39.3: "burn (outside the one named `DECAY_RETIRE` cause)" and
  "Every other retire cause (burn, …) stays excluded" — typed BURN under a `FeeBurnCause` is
  admitted, with whole-stack retirement and partial quantity reduction;
- §39.1: "mint into an existing stack, multiple touched items" exclusions — up to 20 BURN inputs
  and up to 2 change MINTs into fresh backpack entries are admitted (mint into an existing stack stays
  excluded);
- §39.1: "MINT, the later TRANSFER … are separate transactions … Aggregation does not combine
  their sequence into one commit" — the change MINTs commit in the fee transaction;
- §39.3 composition paragraph and DUR-02 schema packet §7.5 — for fee sources, the Character +
  value boundary is the §4.3 composition. GOLD-FEE-1 must prove it (§9 revalidation) before any
  such path is enabled.

Every other §39 obligation is unchanged (fences, cause, evidence, idempotency, current authority,
conservation). The amendment paragraph is in §39.3; pointer notes are in §15 and §39.1, DUR-02
(schema packet §7.5) and GAME-ITEM-01 §9. DUR-02 rule 2 needs no change: the Character change
advances the revision once, as for any Character semantic transaction.

## 6. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| GOLD-FEE-1a | Gold coins (`i3031`, worth 1) only: migration 0023 (D173) with the fee record, BURN lines and their composition guards; the in-transaction burn composed by a fee source inside its fenced Character transaction (change always 0, no MINT); the pure payment planner of §4.2 with its tests; the resource rows of §4.6 and the audit schema/registry; PostgreSQL tests | this decision accepted |
| GOLD-FEE-1b | Stack maximum 100 for the three coins (`i3031`, `i3035`, `i3043`; the item-authoring content write of §4.2); platinum and crystal inputs; the change MINT of §4.2 step 4 with its guards and PostgreSQL tests | 1a; must land before CHARM-6 |
| CHARM-6 | Charm unassign (and later reset, by amendment) composing the fee burn. Its migration replaces `game_item_fee_burn_consistency_guard` so a fee record requires the CharmUnassign receipt of the same occurrence, at the committed revision, bound to the record's TransactionId and fee `F` (1a accepts any Character receipt of that revision). CHARM-6 verifies the session-generation fence of the composed transaction | 1a, 1b, D170 |
| Later | Bank ledger stage 2 (D174); nested bags as sources | own decisions |

## 7. Rejected options

- **Bank ledger now** (Q35 b): no ledger exists; it needs an economy contract (D174 stage 2).
- **Gold only, no change** (Q36 a): with 20 stacks of 100 gold the level x 100 fee is unpayable
  from about level 20.
- **Burn first, refund on failure** (Q38 b): see §4.3.
- **Generic fee cause with a reason code** (Q39 b): any caller could burn value under a label;
  DUR-03 §14/§15 require a typed source/sink.
- **Change charged to `DUR03-RL-03`**: coins are items; widening the value-line row would admit a
  capability no shape uses (B3 §6).
- **Raise the registered `DUR03-RL-01` now**: breaks the bound code and tests; see §4.6.

## 8. Decision test

- **Must decide now:** YES. CHARM-6 and player-facing charm release (D170) wait on it, and
  GOLD-FEE-1 cannot start without a burn shape, a composition and a stack maximum.
- **Minimum sufficient:** backpack coins only, three denominations, at most two change stacks, one closed
  cause; no ledger, no exchange service, no generic burn.
- **Harder later:** receipts and audit events carry the cause type and the worth table; changing
  them after coins are burned needs a migration of retained evidence.
- **Superseding evidence:** a Global capture of the coin selection order or of change behaviour; an
  accepted bank contract; a measured RL-07 worst case above the ANL ceilings.
- **Deliberately not decided:** the charm fee amount and discounts (CHARM-6); the bank ledger;
  nested bags; field numbers and physical schema; client messages.
- **Example:** 2,350 paid from one crystal coin burns the crystal and mints 76 platinum and 50 gold
  (7,650), which needs two free entries after the burn.

## 9. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5905416975 (CHARM-6 gold fee, D170)"
owner_decisions: [D174, D175, D176, D177, D178]
amends:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §15, §39.1 pointers; §39.3 fee-burn paragraph
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md   # §7.5 pointer
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md   # §9 pointer
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md
resource_values_changed: true   # registered by GOLD-FEE-1a, not here
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # GOLD-FEE-1a after protected integration
required_fresh_allocation: true
required_independent_review: "exact-head independent review (typed BURN admission, payment plan and two-stack change conservation, Character + value atomic composition and fence, one-receipt idempotency, resource rows)"
required_revalidation:
  - "planner: exact conservation; insufficient funds writes nothing; ascending worth then display order; at most one partial input; change as floor(C/100) platinum plus C mod 100 gold, at most 2 fresh stacks, each <= 99; fewer free entries than outputs rejects; 21 inputs reject"
  - "transaction: Character change, burns, change MINT and audit commit together or not at all; CharacterRevision +1 exactly once; replay of the same occurrence returns the first outcome; a changed binding conflicts; each stale fence part (connection generation, GameSession, lease, scope generation, session_state, assignment, node incarnation, revision) commits nothing"
  - "rows: max and max+1 for RL-01 22, RL-02 22, RL-06 22/64; RL-03 still 0; the measured RL-07 worst case within the ANL ceilings"
remaining_unknowns:
  - Global coin selection order and change behaviour
  - bank ledger (stage 2)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates GOLD-FEE-1a."
```
