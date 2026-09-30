# NPC-0 NPC runtime service (talk, trade, travel)

- Decision: `NPC0-NPC-RUNTIME-SERVICE-V1`
- Status: **CANDIDATE**. Owner answer Q1a given (§9). Acceptance needs exact-head validation,
  independent review (persistence and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the NPC lane question packet (architect ruling on #162, 5909181928, answer 1a)
- Child of: `OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md` (the NPC boundary). This
  decision fills what the boundary left open (§18 there): wire IDs, value shapes, travel, the
  runtime content input and resource dimensions. It keeps the boundary's ownership rules. This PR
  also takes the boundary through protected acceptance (§10) and rules on its price evidence
  gate (§7).
- Builds on: `OTERYN_NPC_AUTHORING_SCHEMA_V1.md` (D4, D9), `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md`,
  `CHARACTER-GOLD-FEE-BOUNDARY-V1` (D174-D178), the Character and item composition decision
  (2026-09-27, §3 and §3.1), DUR-03 §39.3, ADR-0021 §4.3, FND-02 §15, `PREMIUM-ACTIVATION-V1`,
  DEATH-0 §3.4 (pending respawn)
- Amends: DUR-03 §15 and §39.3; the gold fee decision §4.3 and §4.4; the NPC boundary status and
  §15/§19 (all in this PR)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| NPC-CONTENT-1 | impl | Rust NPC, Dialogue, Trade and Travel families read from the content tree; offer and route classification and the validator rules of §3.4; generated minimal replies (§3.3) | this decision |
| NPC-PLACE-1 | impl, content review | NPC placements generated from the Canary/Crystal NPC spawn data into the World Project, checked against the bundle (§3.2) | NPC-CONTENT-1; MAP-BUNDLE-1 |
| NPC-WIRE-1 | impl, protocol review | registry and proto rows of §4, codecs, limits, client views | NPC-CONTENT-1 |
| NPC-TALK-1 | impl | conversation lifecycle and keyword matching in the channel runtime (§2.1), read-only trade window (boundary gate `NPC_DIALOGUE_TRADE_WIDGET_V1`) | NPC-WIRE-1; NPC-PLACE-1; MAP-CUTOVER-1 |
| NPC-TRADE-1 | hard, persistence review | BUY and SELL (§5), migration, cause records (boundary gate `NPC_SINGLE_TRADE_COMMIT_V1`) | NPC-TALK-1; GOLD-FEE-1a (merged); GOLD-FEE-1b |
| NPC-TRAVEL-1 | hard, persistence review | travel with fee and pending arrival (§6), migration | NPC-TALK-1; GOLD-FEE-1b; DEATH-1 admission consumption |

Every child keeps the boundary's rules: the client is not an authority, dialogue code never
commits value, and AI owns no dialogue or trade state.

## 1. Question

What must exist so a player can talk to an NPC, buy and sell, and travel, and which of those
shapes are new?

## 2. Facts

**PROVEN**

- The NPC boundary (on main, status `PROPOSED_FOR_PROTECTED_REVIEW`) places `GAME-NPC-SERVICE`
  inside the channel runtime. Conversation state is runtime-local and never durable (§7 there).
  Value moves only through GAME-ITEM and DUR-03 (§12 there). It names two gates:
  `NPC_DIALOGUE_TRADE_WIDGET_V1`, then `NPC_SINGLE_TRADE_COMMIT_V1` (§19 there). It forbids NPC
  runtime allocation while unprotected (§19) and gates exact prices on target evidence (§15).
- The content tree holds 1,094 NPCs, 707 Dialogue declarations, 322 trade services (10,723
  offers; the largest has 757) and 56 travel services (198 routes; 12 have price 0). NPC records
  carry no position, and `content/world/placements` is `READY_UNPOPULATED`. NPC keys are
  `oteryn:npc.<slug>`, travel services `oteryn:service.travel.<slug>` (schema D4). Tibia NPC text
  is admitted as reference data (D9). No Rust NPC family exists yet.
- Offers: 31 use a token currency (i22516, i22721) instead of gold; 141 carry `count` 500, 1,800
  or 14,400; 31 sell prices exceed 1,009,999 gold.
- Gold is only physical coins in direct entries of the main backpack (D174, D175). GOLD-FEE-1a
  (merged, `0023`) implements the gold coin fee BURN and pins worth 1 and no change output; the
  platinum and crystal admission and their stack maximum 100 (D176) land with GOLD-FEE-1b.
  `FeeBurnCause` is closed, its only variant `CharmUnassign`, and `0023` requires the fee record
  to commit with an advanced Character root. A new fee source needs an amendment of DUR-03 §39.3
  and the gold fee decision, with its own owner decision (D178).
- Composition decision §3 rule 1 and §3.1: a DUR-03 transaction whose only Character-related
  effects are item locations in the main backpack and cause records keyed by a Character does
  not advance `CharacterRevision` (as `0012` RewardClaim MINT).
- DEATH-0 keeps one pending respawn obligation per Character (`game_character_pending_respawns`,
  `0016`), consumed at placement. Admission consumption is not built yet (DEATH-1).
- The protocol registry holds command types 1-3 and state domains 1-3. The architect allocated 4-5
  with capability 1 (charms), reserved 6 with capability 2 (proficiency) and reserved 7-8 with
  capability 3 for NPC-0 (#162 5907282001, 5909366267).

**DERIVED**

- Travel and trade in Tibia are dialogue-driven: a travel keyword asks for confirmation, then
  moves the player and takes the fee; `trade` opens the Buy/Sell window (NPC boundary §2).

## 3. Content

### 3.1 Input (NPC-CONTENT-1)

- The runtime reads NPC, Dialogue, Trade and Travel records from the generated content tree
  (`content/npcs/`, `content/dialogues/`, `content/services/trade/`, `content/services/travel/`)
  as a validated input. As in the WO-2b ruling (5908515345), the input digests are part of the
  compiled content identity; an unpinned or mismatched input fails closed.
- Only admitted records load. Held records do not load.
- Every item reference resolves through the Item registry (A12 key); an unknown key holds the
  record.

### 3.2 Placement (NPC-PLACE-1)

- NPC positions come from the Canary/Crystal NPC spawn data, generated with provenance into the
  World Project in the project frame, and compiled into the World Bundle (ADR-0021 §4.3).
- An NPC whose position is not a walkable tile of the bundle is held; the rest of the World still
  compiles. A travel route whose destination is not walkable is held the same way.

### 3.3 Minimal replies (ruling answer 3b)

- An admitted NPC without a Dialogue gets generated replies only: a greeting, a farewell, and one
  line per service it has. The text is a fixed Oteryn template filled with data (NPC name,
  destination names and prices), never wiki text.
- An admitted Dialogue replaces the generated replies for that NPC.

### 3.4 Offer and route rules

- An offer whose currency is not gold is held (the 31 token offers).
- `count` on a stackable item is the quantity per purchase and must be 1 to 100. `count` on a
  charged or fluid item is its sub-type or charges. Any other `count` (for example the 141 offers
  above 100 today) holds the offer until classified.
- A sell price above 1,009,999 gold (the most three coin stacks can hold) is held.
- **Arbitrage rule.** For every item, the highest price any NPC pays for it must not exceed the
  lowest price any NPC sells it for. A violating offer is held.
- A travel route loads when it has no gate or only a level and Premium gate. Quest-gated routes
  are held. A route price of 0 is free.

## 4. Wire (NPC-WIRE-1)

| Kind | Id | Name | Content |
|---|---|---|---|
| capability | 3 | `NPC_SERVICE_V1` | gates everything below |
| command type | 7 | `NPC_TALK_INTENT` | `{npc_actor, text}` |
| command type | 8 | `NPC_TRADE_INTENT` | `{npc_actor, catalogue_revision, offer_index, side: BUY or SELL, quantity, expected_unit_price}` |
| state domain | 7 | `NPC_CONVERSATION` | the open conversation (NPC, state) and the reply lines addressed to this character |
| state domain | 8 | `NPC_TRADE_WINDOW` | the projected catalogue (NPC, catalogue revision, offers, prices) while the window is open |

- **Talk text.** UTF-8, at most `NPC0-RL-01` bytes; control characters are rejected. The server
  case-folds ASCII and matches whole keyword tokens. The text is never broadcast to other players
  and never persisted or logged verbatim. Intents are rate-limited by `NPC0-RL-04`.
- **Travel.** No command of its own. A travel keyword opens a confirmation bound to the route,
  price and content revision; `yes` through `NPC_TALK_INTENT` within `NPC0-RL-06` seconds confirms
  it. Anything else, or the timeout, cancels it.
- **Revisions.** Domains 7 and 8 are owned by the channel runtime. Their revision scope is the
  connection generation: every admission, reconnect and channel transfer starts them with a new
  snapshot, and revisions are never reused within a scope (FND-02 §15). Reconnect or transfer
  closes the conversation, and the new snapshot is empty (boundary §7).
- A client without capability 3 never receives domains 7 and 8, and a command 7 or 8 from it is
  refused as unsupported.
- **Limits** (measured and registered by NPC-WIRE-1): `NPC0-RL-01` talk text bytes, `NPC0-RL-02`
  reply lines per snapshot, `NPC0-RL-03` offers per catalogue projection (at least 757),
  `NPC0-RL-04` talk intents per second, `NPC0-RL-05` open conversations per NPC, `NPC0-RL-06`
  travel confirmation timeout.

### 4.1 Resource dimensions (boundary §16)

- Open conversations per character: 1. A new conversation closes the previous one.
- Open conversations per NPC: `NPC0-RL-05`.
- Pending service invocations per conversation: 1. A second command while one is pending is
  refused.
- Retained conversation state: none after close.

## 5. Trade (NPC-TRADE-1)

### 5.1 Common rules

- The server reads the offer and price from the trade service at the conversation's bound
  catalogue revision. The client's `catalogue_revision` and `expected_unit_price` are only stale
  checks: a mismatch rejects and re-projects the window.
- The character must be in an open conversation with that NPC, within talk range, under a live
  session. Every command carries one occurrence (UUIDv7).
- `unit_price x quantity` is computed in checked unsigned 64-bit arithmetic.
- Weight is not checked (B3 D81). The bank (stage 2), the Gold Pouch and ground overflow are out
  of scope: a result that does not fit the backpack is rejected and writes nothing.

### 5.2 Records and revision

- BUY and SELL are item-only transactions under composition §3 rule 1 and §3.1: they do not
  advance `CharacterRevision` and write no Character root, progression or receipt row, as `0012`
  RewardClaim.
- Each writes one DUR-03 cause record keyed by (occurrence, character). It binds the NPC, the
  offer (catalogue revision and index), side, quantity, unit price, every burn and mint line, and
  the TransactionId. A replay returns the first outcome; a changed binding conflicts.
- The fence and lock order are the item writer's (composition §3 rule 2, with the cause record in
  place of the reward claim), then the main backpack and its entries.
- One TransactionId fixes the output identity slots before the first attempt (D177).

### 5.3 BUY

- The gold fee plan of D175 with `F = unit_price x quantity`, burning coins and minting change,
  plus one MINT of the bought item (one stack of the offer's count, or one non-stackable item with
  its sub-type) into a new direct entry of the main backpack.
- `F` above 20,000,000 (the most 20 coin stacks can hold) is always insufficient funds.
- Insufficient funds, no free entry after the burn and change, or more than 20 burn inputs
  rejects the whole transaction.

### 5.4 SELL

- One BURN of `quantity` units from one live direct backpack entry of the offer's item (a whole
  non-stackable item, or part or all of a stack). The item must have no contents and its default
  state apart from quantity.
- A MINT of `V = unit_price x quantity` gold as fresh stacks in new backpack entries: `floor(V /
  10,000)` crystal, then platinum, then gold, each present only if positive and each at most 100.
- No free entries or a stack above 100 rejects the whole transaction.

### 5.5 Causes and evidence

- One closed cause covers every line of a trade transaction:
  `NpcTradeCause {npc, offer, side: BUY or SELL, occurrence}`.
  - BUY: the coin burn uses `FeeBurnCause::NpcTrade(NpcTradeCause)`; the change MINT and the item
    MINT use `NpcTradeCause` as their MINT source.
  - SELL: the item BURN uses `NpcTradeCause` as its burn sink; the coin MINT uses it as its MINT
    source.
- One audit event per transaction (`DUR03-RL-07-EVENTS` 1) carries the cause and every line.
- Rows (registered by NPC-TRADE-1):
  - BUY: `DUR03-RL-01` 23, `DUR03-RL-02` 23, `DUR03-RL-06` 23 participants / 66 work units;
  - SELL: `DUR03-RL-01` 4, `DUR03-RL-02` 4, `DUR03-RL-06` 4 participants, work units measured.
- A whole-item SELL retirement widens the `0023` entry-removal proof to that cause.

## 6. Travel (NPC-TRAVEL-1)

- Travel changes Character state (a pending arrival), so it is a Character transaction under D177:
  one Character receipt keyed by the occurrence, `CharacterRevision` +1 exactly once, the
  Character writer's fence. `F = route price` is burned with the gold fee plan under
  `FeeBurnCause::NpcTravel {npc, route, occurrence}`; a price of 0 writes no fee lines.
- **Eligibility,** checked in the same transaction from durable facts: level (`min_level`),
  Premium (`PREMIUM-ACTIVATION-V1`) when the route requires it, no pending respawn and no pending
  arrival. The runtime also refuses travel while the character is in combat (logout-blocked), as
  in Tibia.
- **Pending arrival.** The same transaction inserts `game_character_pending_arrivals`
  (one row per Character, primary key `character_id`; occurrence, destination), mirroring DEATH-0's
  pending respawn. The runtime then moves the character, and the placement deletes the row in its
  own fenced transaction.
- **After a crash** between the commit and the move, admission places the character at the
  pending arrival and deletes the row, the same path DEATH-1 builds for the pending respawn. A
  pending respawn and a pending arrival never coexist, because each refuses the other. An occupied
  destination tile uses the Character contract's placement fallback.
- The `0023` fee record gains the source kind `NpcTravel`, and the Character receipt chain guard
  gains the travel receipt (a new migration).

## 7. Price evidence (boundary §15 and §19)

The boundary requires exact target evidence for prices before `NPC_SINGLE_TRADE_COMMIT_V1`.
Architect ruling: for the playable slice, the admitted content prices satisfy that input. They
are the two-of-three wiki consensus (npc-majority-price task) or the Canary/Crystal price marked
`PARITY_PENDING` (ruling answer 5a), under the owner rule that owner-trusted fan sources are
allowed and an official source governs. Exact target evidence stays a parity gate before a
production release, not an implementation gate.

## 8. Rejected options

- **A generic NPC script or VM.** The boundary rejects it (§18 there).
- **One command per service.** Travel is a dialogue confirmation in Tibia; a separate command
  adds wire without a player benefit.
- **Merging bought items or coins into existing stacks.** It needs merge shapes and ordering
  rules. Fresh entries are enough for the first slice, as in the gold fee change.
- **A CharacterRevision advance for BUY and SELL.** Composition §3 rule 1 already covers
  item-only transactions; an advance would need a new receipt kind with no Character effect.
- **Travel as a runtime move without a durable record.** A crash would take the fee and lose the
  move.
- **"Newest position-bearing receipt" as the admission position.** Without a consumed
  obligation, every later login would teleport the character again.

## 9. Owner question

D178 requires an owner decision for every new fee source.

**Q1. Admit NPC trade and NPC travel as value sources?** a) Yes, as in Tibia (recommended);
b) buying and travel only; c) none now.

**Owner answer (2026-09-30, given directly in the architect session, recorded on #162
5909366267):** "tak a". Q1a: NPC buying, selling and travel are admitted as value sources (D178
satisfied). The control plane assigns the D-number.

## 10. Boundary acceptance

- This PR moves the NPC boundary from `PROPOSED_FOR_PROTECTED_REVIEW` to accepted, with the
  amendments of §7 and this decision, when the PR passes its independent review and protected
  integration. The boundary's §19 bar on allocating NPC runtime workers ends then.

## 11. Decision test

- **Must decide now:** YES. Without it NPCs cannot be used in play (ruling answer 1a).
- **Minimum sufficient:** two commands, two runtime domains, one capability, two item-only trade
  shapes and one travel shape, each reusing the gold fee plan.
- **Superseding evidence:** a Tibia-parity need for bank payment or stack merging, or dated target
  evidence contradicting an admitted price.
- **Deliberately not decided:** quest-conditioned dialogue (answer 4b), quest-gated routes, token
  currencies, the bank, spells, blessings and promotion services, NPC movement schedules, and
  localization.

## 12. Before-freeze checklist

1. **Contract amendments:** DUR-03 §15 and §39.3, the gold fee decision §4.3 and §4.4, and the NPC
   boundary are amended in this PR (owner answer Q1a).
2. **Serialization:** the item writer's fence for trade, the Character writer's fence for travel.
3. **Restart:** conversation state is runtime-local; trade cause records and the pending arrival
   are durable.
4. **Typed references:** NPC, offer (catalogue revision and index), route and occurrence.
5. **Wire:** §4, capability-gated.
6. **Split work:** every command is one transaction with one record.
