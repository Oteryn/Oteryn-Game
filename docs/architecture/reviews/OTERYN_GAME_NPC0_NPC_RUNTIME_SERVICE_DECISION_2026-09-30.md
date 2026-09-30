# NPC-0 NPC runtime service (talk, trade, travel)

- Decision: `NPC0-NPC-RUNTIME-SERVICE-V1`
- Status: **CANDIDATE**. Owner answer Q1a given (§9). Acceptance needs exact-head validation,
  independent review (persistence and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the NPC lane question packet (architect ruling on #162, 5909181928, answer 1a)
- Child of: `OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md` (the NPC boundary). This
  decision fills what that boundary left open (§18 there): wire IDs, value shapes, travel and the
  runtime content input. It changes none of the boundary's ownership rules.
- Builds on: `OTERYN_NPC_AUTHORING_SCHEMA_V1.md` (D4, D9), `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`,
  `CHARACTER-GOLD-FEE-BOUNDARY-V1` (D174-D178), DUR-03 §39.3, ADR-0021 §4.3, FND-02 §15,
  `PREMIUM-ACTIVATION-V1`, the reward-chest MINT (`0012`) and the DEATH-0 respawn position
- Amends: DUR-03 §15 and §39.3 (the NPC service amendment, in this PR)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| NPC-CONTENT-1 | impl | Rust NPC, Dialogue, Trade and Travel definition families read from the content tree (§3), generated minimal replies (§3.3), fail-closed validation | this decision |
| NPC-WIRE-1 | impl, protocol review | registry and proto rows of §4, codecs, limits, client views | NPC-CONTENT-1 |
| NPC-TALK-1 | impl | conversation lifecycle and keyword resolution in the channel runtime (§2), read-only trade window (boundary gate `NPC_DIALOGUE_TRADE_WIDGET_V1`) | NPC-WIRE-1; MAP-CUTOVER-1 for placed NPCs |
| NPC-TRADE-1 | hard, persistence review | BUY and SELL value transactions (§5), migration, receipts (boundary gate `NPC_SINGLE_TRADE_COMMIT_V1`) | NPC-TALK-1; GOLD-FEE-1a (merged) |
| NPC-TRAVEL-1 | hard, persistence review | travel with fee and durable destination (§6) | NPC-TALK-1; GOLD-FEE-1a |

Every child keeps the boundary's rules: the client is not an authority, dialogue code never
commits value, and AI owns no dialogue or trade state.

## 1. Question

What must exist so a player can talk to an NPC, buy and sell, and travel, and which of those
shapes are new?

## 2. Facts

**PROVEN**

- The NPC boundary (main, candidate of #499) places `GAME-NPC-SERVICE` inside the channel runtime.
  Conversation state is runtime-local and never durable (§7 there). Value moves only through
  GAME-ITEM and DUR-03 (§12 there). It names two gates: `NPC_DIALOGUE_TRADE_WIDGET_V1`, then
  `NPC_SINGLE_TRADE_COMMIT_V1` (§19 there).
- The content tree holds 1,094 NPCs, 701 Dialogue declarations (6,313 static `say` keyword
  nodes), 322 trade services (10,723 offers) and 56 travel services (189 ungated routes), all
  generated from WorldProject/v2 (NPC admission §7.4g). NPC keys are `oteryn:npc.<slug>` and
  travel services `oteryn:service.travel.<slug>` (schema D4). Tibia NPC text is admitted as
  reference data (D9). No Rust NPC family exists yet.
- Gold is only physical coins (gold 1, platinum 100, crystal 10,000; stack maximum 100) in direct
  entries of the main backpack. GOLD-FEE-1a (merged, migration `0023`) implements the coin BURN
  plan with change (D174-D177). `FeeBurnCause` is closed and its only variant is `CharmUnassign`.
  A new fee source needs an amendment of DUR-03 §39.3 and the gold fee decision **with its own
  owner decision** (D178).
- The protocol registry holds command types 1-3 and state domains 1-3. The architect allocated
  command types and domains 4-5 with capability 1 (charms) and reserved 6 with capability 2
  (proficiency) (#162 5907282001).
- DEATH-0 writes a durable `respawn_position` in its receipt (`0016`). ADR-0021 §4.4 leaves the
  durable Character position to the Character contract.

**DERIVED**

- Travel and trade in Tibia are dialogue-driven: a travel keyword asks for confirmation, then
  moves the player and takes the fee; `trade` opens the Buy/Sell window (NPC boundary §2).

## 3. Content (NPC-CONTENT-1)

### 3.1 Input

- The runtime reads NPC, Dialogue, Trade and Travel records from the generated content tree
  (`content/npcs/`, `content/dialogues/`, `content/services/trade/`, `content/services/travel/`)
  as a validated input. The same rule as the WO-2b ruling (5908515345) applies: the input digests
  are part of the compiled content identity, and an unpinned or mismatched input fails closed.
- Only admitted records load. Held records (conflicting dialogue, conditional or scripted nodes,
  held NPCs) do not load.
- Every item reference resolves through the Item registry (A12 key); an unknown key fails
  validation.

### 3.2 Placement

- NPC positions are authored in the project frame and compiled into the World Bundle
  (ADR-0021 §4.3). A position that is not a walkable tile of the bundle fails compilation.
- Travel destinations use the same frame and the same check.

### 3.3 Minimal replies (ruling answer 3b)

- An admitted NPC without a Dialogue gets generated replies only: a greeting, a farewell, and one
  line per service it has (trade, travel). The text is a fixed Oteryn template filled with data
  (NPC name, destination names), never wiki text.
- A Dialogue, when admitted, replaces the generated replies for that NPC.

## 4. Wire (NPC-WIRE-1)

Allocated now by the protocol owner, after the charm and proficiency allocation:

| Kind | Id | Name | Direction and content |
|---|---|---|---|
| capability | 3 | `NPC_SERVICE_V1` | gates everything below |
| command type | 7 | `NPC_TALK_INTENT` | `{npc_actor, text}`; text at most `NPC0-RL-01` bytes, UTF-8, keyword matching is server-side |
| command type | 8 | `NPC_TRADE_INTENT` | `{npc_actor, catalogue_revision, offer_index, side: BUY or SELL, quantity, expected_unit_price}` |
| state domain | 7 | `NPC_CONVERSATION` | the open conversation (NPC, state) and the reply lines addressed to this character |
| state domain | 8 | `NPC_TRADE_WINDOW` | the projected catalogue (NPC, catalogue revision, offers, prices) while the window is open |

- Travel has no command of its own: it is a dialogue confirmation (`yes`) through
  `NPC_TALK_INTENT` (§6).
- Domains 7 and 8 are owned by the channel runtime, like domains 1-3. They are runtime-local:
  every admission, reconnect and channel transfer starts them with a snapshot, and no revision
  continuity crosses connections (FND-02 §15).
- Limits `NPC0-RL-01` (talk text bytes), `NPC0-RL-02` (reply lines per snapshot), `NPC0-RL-03`
  (offers per catalogue projection) and `NPC0-RL-04` (talk intents per second) are measured and
  registered by NPC-WIRE-1. The catalogue maximum must cover the largest admitted trade service.
- A client without capability 3 never receives domains 7 and 8, and a command 7 or 8 from it is
  refused as unsupported.

## 5. Trade (NPC-TRADE-1)

### 5.1 Common rules

- The server reads the offer and price from the trade service at the conversation's bound
  catalogue revision. The client's `expected_unit_price` must equal it; otherwise the command is
  rejected as stale and the window is re-projected.
- The character must be in an open conversation with that NPC, within its talk range, under a
  live session. Every command carries one occurrence (UUIDv7), which keys the receipt.
- Coins use the D175 table. Weight (B3-3), the bank (stage 2), the Gold Pouch and ground
  overflow are out of scope: a result that does not fit the backpack is rejected and writes
  nothing.

### 5.2 BUY

- One transaction: the gold fee plan of D175 with `F = unit_price x quantity` (burn and change),
  plus one MINT of the bought item into a new direct entry of the main backpack.
- Quantity: 1 for a non-stackable item; 1 to the stack maximum for a stackable item (one output
  stack). A fluid or charged item takes its offer's sub-type or count as its state.
- Insufficient funds, no free entry after the burn and change, or more than 20 burn inputs
  rejects the whole transaction.

### 5.3 SELL

- One transaction: one BURN of `quantity` units from one live direct backpack entry of the
  offer's item (a whole non-stackable item, or part or all of a stack), plus the gold MINT.
- The item must have no contents and its default state apart from quantity; anything else is not
  sellable in this slice.
- Gold `V = unit_price x quantity` is minted as fresh stacks in new backpack entries:
  `floor(V / 10,000)` crystal, then platinum, then gold, each present only if positive. Any stack
  above 100 coins or no free entries rejects the whole transaction.

### 5.4 Composition

- The receipt is a Character receipt keyed by the occurrence, advancing `CharacterRevision`
  exactly once (DUR-02 rule 2), under the Character writer's fence and lock order (D177). It
  binds the NPC, the offer (catalogue revision and index), side, quantity, unit price, the burn
  lines and the minted outputs. A replay of the same occurrence returns the first outcome; a
  changed binding conflicts.
- One TransactionId fixes the output identity slots before the first attempt, as in D177.
- Causes are closed: `FeeBurnCause::NpcBuy {npc, offer, occurrence}` for the gold of a BUY, and
  `NpcServiceCause::Sell {npc, offer, occurrence}` for the item BURN and the gold MINT of a SELL.
  The bought item's MINT uses `NpcServiceCause::Buy {npc, offer, occurrence}`.
- Rows: BUY touches at most 20 burn inputs, 2 change outputs and 1 bought item (`DUR03-RL-01`
  23); SELL touches 1 burn and at most 3 coin outputs. NPC-TRADE-1 registers the rows.

## 6. Travel (NPC-TRAVEL-1)

- A travel keyword of an admitted travel service asks for confirmation. The confirmation commits
  one transaction: the gold fee plan with `F = route price` and the travel receipt.
- Eligibility is checked in the same transaction from durable facts: level (`min_level`) and
  Premium (`PREMIUM-ACTIVATION-V1`) when the route requires it. Only ungated routes load in this
  slice.
- The receipt is a Character receipt keyed by the occurrence (`CharacterRevision` +1). It records
  the destination as a durable position, as DEATH-0 records its respawn position.
- After the commit the runtime moves the character to the destination. If the process fails
  between the commit and the move, admission places the character at the destination of the
  newest committed position-bearing receipt (DEATH-0 respawn or travel), so the paid move is not
  lost. NPC-TRAVEL-1 extends the admission position source that DEATH-0 uses; the Character
  contract still owns the fallback.
- Cause: `FeeBurnCause::NpcTravel {npc, route, occurrence}`.

## 7. Rejected options

- **A generic NPC script or VM.** The boundary rejects it (§18 there), and static keywords plus
  typed services cover the admitted content.
- **One command per service (talk, trade, travel).** Travel is a dialogue confirmation in Tibia;
  a separate travel command adds wire without a player benefit.
- **Merging bought items or coins into existing stacks.** It needs merge shapes and ordering
  rules. Fresh entries are enough for the first slice, as in the gold fee change.
- **Travel as a runtime move without a durable record.** A crash would take the fee and lose the
  move.

## 8. Decision test

- **Must decide now:** YES. Without it NPCs cannot be used in play (ruling answer 1a).
- **Minimum sufficient:** two commands, two runtime domains, one capability, two value shapes and
  one travel shape, each reusing the gold fee plan.
- **Superseding evidence:** a Tibia-parity need for bank payment or stack merging, or a trade
  catalogue larger than the wire limit.
- **Deliberately not decided:** quest-conditioned dialogue (answer 4b), the bank, spells,
  blessings and promotion services, NPC movement schedules, and localization.

## 9. Owner question

D178 requires an owner decision for every new fee source. This decision adds three value sources.

**Q1. Admit NPC trade and NPC travel as value sources?**
- **a)** Yes, as in Tibia: buying burns coins and mints the item; selling burns the item and
  mints coins; travel burns the route price. ← **recommended**: it is the smallest shape that
  makes NPCs usable, and it reuses the gold fee plan.
- **b)** Only buying and travel now; selling later.
- **c)** None now; NPCs talk and show the trade window only.

**Owner answer (2026-09-30, given directly in the architect session):** "tak a". Q1a: NPC
buying, selling and travel are admitted as value sources (D178 satisfied). The control plane
assigns the D-number.

## 10. Before-freeze checklist

1. **Contract amendments:** DUR-03 §15 and §39.3 are amended in this PR (owner answer Q1a).
2. **Serialization:** the Character writer's fence and `character_root` lock per transaction.
3. **Restart:** conversation state is runtime-local; value and travel destination are durable
   receipts.
4. **Typed references:** NPC, offer (catalogue revision and index), route and occurrence.
5. **Wire:** §4, capability-gated.
6. **Split work:** every command is one transaction with one receipt.
