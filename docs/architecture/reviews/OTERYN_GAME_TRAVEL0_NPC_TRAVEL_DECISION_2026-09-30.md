# TRAVEL-0 NPC travel (ships, carpets and other transport NPCs)

- Decision: `TRAVEL0-NPC-TRAVEL-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and content) and protected integration. T1 (§15) is an architect ruling applying the owner's
  answers Q3a and H2a. The ambiguous-commit ruling (§7.5: the traveller stays frozen until
  reconciled, alert at 2 s) is owner-confirmed (#162 comment 5919525206); no owner question is
  open.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner direction of 2026-09-30 (build travel now, full Tibia Global parity); what
  NPC-0 §6 left open: discounts, the full refusal list, the arrival step, channel scope, the
  route content that is held today
- Builds on: NPC-0 (§3.1-§3.4, §4, §4.1, §5.1, §6, §6.1, §9 owner answer D208), QUEST-GATE-0
  (§3.4, §5.1; branch `claude/arch-quest-gate-0`), QUEST-STATE-0 §7 predicates, PARTY-PVP-0 §8.1
  (branch `claude/arch-party-pvp-0`), ATTACK-0 §4, CHAR-POSITION-0 §3.2-§3.3,
  PREMIUM-ACTIVATION-V1 §4.5 (D70, D73), PREMIUM-DELIVERY-0 §6, BANK-FEE-0 §5, the gold fee
  boundary (D174-D178), DUR-03 §7, §15 and §39.3 (the NPC service amendment), the composition
  decision §3 rules 1-5, the relocation and world object owners proposal §3 (D37 R1-R3),
  VSL-MOVE-01, ADR-0001 §7, the NPC admission v2 decision §6, ITEM-USE-0 and RUNE-USE-0 §5.2
  (burn before effect), owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of TRAVEL-0, in this PR: NPC-0 §6 (refusals, arrival,
  discounts); CHAR-POSITION-0 §3.2 (terminal release consumes an arrived obligation); the NPC
  admission v2 decision §6 (route fields)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| TRAVEL-CONTENT-1 | impl (NPC content lane), content review | route `keywords`, `gate`, `discounts`; the 65 held and 18 missing Canary routes classified; validator rules (§4) | NPC-CONTENT-1; QUEST-CONTENT-1 for gate and discount tracks |
| NPC-TRAVEL-1 (widened) | hard, persistence review | NPC-0 §6 plus: the price with discounts (§5), the refusal order (§6), the travel hold and arrival (§7), channel rules (§8) | NPC-TALK-1; GOLD-FEE-1b; DEATH-1 admission; QUEST-PRED-1; CHAR-POSITION-1 |
| NPC-TRAVEL-2 | impl, persistence review | Premium destinations: the route flag and the PREM-3 area flag read through `premium_current` (§6 step 3) | NPC-TRAVEL-1; PREM-1; PREM-3 |

No wire child: travel reuses the NPC-0 talk wire (§10). The bank part of the fee is GOLD-FEE-2
(BANK-FEE-0), whichever of it and NPC-TRAVEL-1 lands later takes it.

## 1. Question

How does a player pay an NPC and arrive at another place in the same World, with the fare taken
exactly once and only for a move that happens, at Global parity?

## 2. Facts

**PROVEN**

- NPC-0 (merged, #1332): a travel keyword opens a confirmation bound to the route, price and
  content revision; `yes` through `NPC_TALK_INTENT` within `NPC0-RL-06` confirms. The occurrence
  is bound 1:1 to the confirming command's CommandRef. Travel is one item-only DUR-03 transaction
  (no `CharacterRevision` advance): the gold fee plan under `FeeBurnCause::NpcTravel {npc, route,
  occurrence}`, one cause record keyed by (occurrence, character), and one
  `game_character_pending_arrivals` row. A crash after commit places the character at the arrival
  at the next admission. A death deletes the arrival; the fee stays spent. An occupied arrival tile
  uses the §6.1 spiral within Chebyshev 3, then the home temple.
- Owner answer D208 (NPC-0 §9, Q1a) admits NPC travel as a value source "as in Tibia", which D178
  requires. DUR-03 §15 and §39.3 already list `NpcTravel`. BANK-FEE-0 §5: travel pays coins
  first, then the bank.
- Content: 56 travel services, 198 routes (`content/services/travel/travel-00000-00055.json`).
  Route fields: `key`, `destination` (frame `global-target-2026-09-27`), `price` (0 to 400; 12 are
  0), `premium` (2 true, both on Kendra), `min_level` (4 routes, all 0). At most 11 routes per
  service; some keys are aliases of one place (`darama`/`darashia`, `kazor`/`kazordoon`,
  `hills`/`femor_hills` on the carpet pilots Uzon, Iyad, Pino and Ziyad). No gate or discount
  field exists. The NPC admission v2 decision (§2, §6) held 65 routes: 46 gated by Lua
  predicates, 15 source conflicts, 4 one-sided.
- QUEST-GATE-0 §3.4: a gated route loads; the talk runtime checks the gate before the
  confirmation; the travel transaction re-reads the gate's tracks under the `character_root` lock.
- PARTY-PVP-0 §8.1: logout block (ATTACK-0 60 s in-fight), PZ block (60 s) and kill block
  (15 min, durable). The PZ block and the kill block bar stepping onto a protection-zone tile.
- CHAR-POSITION-0 §3.2-§3.3: no last-position write while a pending arrival exists; admission
  consumes a pending arrival and upserts the last position in the same transaction.
- D37 (relocation proposal §3): the scope runtime owns relocation within its scope, fenced on
  WorldId, ChannelId or InstanceId, session generation and content generation; relocation to
  another Channel or Instance (`SCOPE_HANDOFF`) stays blocked. ADR-0001 §7: all channels of a
  World share the base map.
- PREMIUM-ACTIVATION-V1 §4.5 (D70, D73): entering a Premium area "a step, boat, carpet or
  teleport into it" is refused unless Premium is current. PREMIUM-DELIVERY-0 §6: every Premium
  check reads `premium_current`, false (Free) until PREM-1 is delivered. The Market (Q3) and
  houses (H2) each needed an owner answer for the time before delivery.
- The runtime keeps one creature per tile (`Slot::Occupied`, `Slot::CreatureOccupied` in
  `apps/game-server/src/foundation/runtime_actor_carrier.rs`).

**CIPSOFT_OFFICIAL** (the Tibia manual)

- Boat captains charge a fare; missing carried gold is taken from the bank. On crowded ships a
  captain can "kick" a passenger to a nearby place. Flying carpets exist at few places
  (`docs/reference/tibia-manual/world.md` lines 88-90).
- Premium areas are reachable only by boat or flying carpet (`world.md` line 35).
- Ships are protection zones; a character under a PZ block cannot enter one (`combat.md` lines 86
  and 89).

**OTS_HYPOTHESIS_ONLY** (Canary, `/home/user/opentibiabr/canary`)

- `StdModule.travel` (`data/npclib/npc_system/modules.lua` 198-260) checks, in order: Premium
  (223), level (225), PZ lock (227: "First get rid of those blood stains!"), money from carried
  gold then bank (229), then a 3 s re-travel cooldown (232-235, set at 245), then
  `teleportTo` (246). It does not check the logout block. The cooldown refusal comes after the
  money is taken, which loses the fare.
- Discounts: `travelDiscounts` has one entry, `postman`, 10 gold, keyed to a placeholder storage
  (`data/npclib/npc_system/custom_modules.lua` 18-40); cost is floored at 0. 50 of the 73
  Canary travel NPC scripts pass `discount = "postman"` (for example `data-otservbr-global/npc/
  kendra.lua` 67-68). The Postman rank track is `ThePostmanMissions.Rank`, values 1-5
  (`data-otservbr-global/lib/core/quests/catalog/024_the_postman_missions.lua` 149-156).
- 73 Canary scripts use `StdModule.travel` or `TravelModule`; 18 are not in the content (among
  them `buddel*`, `dalbrect`, `eremo`, `rapanaio_*`, `scrutinon`); 35 read storage values.
- Teleport uses `FLAG_NOLIMIT` (`src/game/game.cpp` 3581, `src/items/tile.cpp` 696), so
  travellers stack on the arrival tile. `StdModule.kick` moves a passenger to a random listed
  position (`custom_modules.lua` 42-63).
- With `teleportSummons = false` (`config.lua.dist` 333), summons more than 15 tiles or a floor
  away after a teleport are removed; familiars follow (`src/creatures/creature.cpp` 454-497).
- `toggleTravelsFree` (`config.lua.dist` 311) makes all travel free.

## 3. What NPC-0 already decides (kept)

Talk-driven confirmation; one item-only transaction with the fee, the cause record and the
pending arrival; the occurrence from the CommandRef; the crash, death and fallback rules; D208 as
the D178 owner decision. This decision adds to them and changes only NPC-0 §6's combat refusal
(§6 step 6).

## 4. Route content (TRAVEL-CONTENT-1)

- **Home.** Routes stay in `content/services/travel/` (NPC-0 §3.1), authored by the NPC content
  lane from Canary/Crystal scripts and the wiki prices (npc-majority-price, `PARITY_PENDING` where
  single-source). Not in `rulesets/`: a route is content of one NPC, not a World rule.
- **Fields added** to `ProjectV2TravelRoute` (NPC admission v2 §6, as amended):
  - `keywords`: the destination words that open this route; aliases collapse into one route.
    Each keyword is stored canonical: ASCII case-folded, one whole token (no whitespace or
    control characters), the form NPC-0 §4 talk matching compares; non-empty and at most
    `TRAVEL0-RL-07` bytes; a route has 1 to `TRAVEL0-RL-08` keywords, no duplicates;
  - `gate`: optional, a QUEST-GATE-0 condition (at most `QUESTGATE0-RL-01` predicates);
  - `discounts`: at most `TRAVEL0-RL-04` entries `{key, gate, amount}`, `amount` in gold, positive.
    Each `key` is canonical (ASCII case-folded, one whole token without whitespace or control
    characters), non-empty, at most `TRAVEL0-RL-07` bytes and unique within its route, so the
    cause record's applied-key list names exactly the authored entries that were summed;
- **Kinds.** Ships, carpets and other transport NPCs (ferrymen, the Rapanaio boats, the Buddel
  rafts) are the same route shape. The vehicle only changes the reply template text.
- **Validator rules** (compile time, a failure holds the route with a diagnostic):
  - destination in bounds, walkable in the active bundle, not a house tile, in the base scope
    (never an instance), and in the NPC's World (NPC-0 §3.2);
  - `price` 0 to 1,000,000; `min_level` 0 to 2,000; each discount `amount` 1 to 1,000,000 (the
    price cap); the route's aggregate discount sum, over all its entries, is computed at compile
    time with checked unsigned 64-bit addition, and an overflow fails the route closed; the
    discount sum may exceed the price;
  - discount keys are canonical, non-empty, bounded and unique within each route (above); a
    duplicate or malformed key holds the route, never merged or deduplicated at runtime;
  - gates and discount gates name known tracks (QUEST-GATE-0 §3.3);
  - keywords are canonical, non-empty and bounded (above), and disjoint across all routes of one
    travel service (NPC), so one keyword selects at most one route; a keyword violation fails the
    service's compile closed (none of its routes load), never resolved by traversal order;
  - a route whose Canary destination is computed by a function, or which needs a script effect
    other than the move, stays held.
- **Classification.** TRAVEL-CONTENT-1 lowers the 46 Lua-gated routes to `gate` where the
  predicate is a quest or level predicate, adds the 18 missing Canary scripts, and records the
  Postman discount on the 50 scripts that pass it. Unresolved conflicts stay held.
- **Not routes:** the Travora world transfer, the Kazordoon ore-wagon weekly ticket and
  `kick` (§14 R1, §16). They stay held.

## 5. Price and discounts (NPC-TRAVEL-1)

- **Fare.** `D` = sum of amounts of the discounts whose gate passes, in checked unsigned 64-bit
  arithmetic; the compile-time bounds (§4) keep it at most 4,000,000, so it cannot overflow at
  runtime. `F = price - D` if `D < price`, else `F = 0`: a saturating subtraction floored at 0,
  so `F` is unsigned and never negative, and a discount sum above the price never underflows or
  rejects. A fare of 0 writes no fee lines (NPC-0 §6).
- **Postman.** The Postman discount is 10 gold at the rank the Canary scripts read
  (`PARITY_PENDING`: the rank value and which routes carry it need a Global source; Canary's key is
  a placeholder). It is ordinary content, not code.
- **Binding.** The confirmation binds route, content revision and `F`. The reply states `F`.
- **Re-check.** The transaction re-reads the gate and discount tracks under the
  `character_root` lock (read only), as QUEST-GATE-0 §3.4 does, and recomputes `F`. A different
  `F` rejects as stale and writes nothing; the NPC offers the route again.
- **Cause record** binds the base price, the applied discount keys and `F`. The cause stays
  `FeeBurnCause::NpcTravel`; a discount lowers a sink the owner already admitted (D208), so D178
  needs no new owner decision. No free-travel switch exists (§12).

## 6. Eligibility and refusals

Checked in this order. Steps 1-7 run in the talk runtime before the confirmation and again when
`yes` arrives; steps 3-6, 8 and 9 are re-read in the transaction from durable facts (step 8 there
`checks the durable pending respawn and pending arrival only: the hold this invocation itself placed
at §7.1 is the expected one and never refuses its own commit; any other hold cannot exist, because
there is one hold per actor). Step 6 is
re-verified there because combat goes on during the travel hold (§7.1): a PZ block or kill block
gained between the request and the commit refuses with the same step-6 refusal cause, writes
nothing and releases the hold. PARTY-PVP-0 exposes both blocks as facts that transaction reads
under the `character_root` lock. The first failure answers with a template reply (NPC-0 §3.3) and
writes nothing.

1. **Conversation.** Open with this NPC, in talk range, alive, live session (NPC-0 §5.1).
2. **Route.** Loaded and not held; the actor is in a channel scope (not an instance, §8).
3. **Premium.** The route's `premium` flag, or a destination in a Premium area (PREM-3 region
   flag). The check uses the PREMIUM-DELIVERY-0 §6 switch-over, as the Market and house gates do:
   before PREM-1's activation record exists, the check is bypassed and the destination is open to
   every character (ruling T1, §15); once the activation record exists, `premium_current` must
   be true. Whether the activation record exists is read with the other step-3 facts, including
   in the transaction re-check; an unreadable activation state refuses (fail closed). Until
   NPC-TRAVEL-2 lands only the route flag applies.
4. **Level.** `min_level`.
5. **Quest gate.** The route `gate` (QUEST-GATE-0 §3.4).
6. **Blocks.** A PZ block or a kill block (PARTY-PVP-0 §8.1) refuses: the ship is a protection
   zone that a blocked character cannot enter. **The logout block alone does not refuse**
   (ruling R2). Until PARTY-PVP-0 lands no block refuses.
7. **Cooldown.** `TRAVEL0-RL-03` (3 s, `PARITY_PENDING`) since this character's last arrival;
   checked before any payment, unlike Canary.
8. **Obligations.** No pending respawn, no pending arrival, no travel hold (§7.1). In the
   transaction re-check the travel hold is excluded: only the durable obligations are re-read.
9. **Funds.** `F` from coins, then the bank (BANK-FEE-0 §5); insufficient funds refuses.

A summon or convinced creature never refuses travel; it stays behind and is removed (ruling R3).

## 7. Commit and arrival (NPC-TRAVEL-1)

### 7.1 Travel hold

On `yes`, the runtime freezes the travel (route, `F`, destination, content revision,
occurrence) and places a **travel hold** on the actor: its walk intents are refused and queued
steps dropped until the outcome. The hold is the conversation's one pending service invocation
(NPC-0 §4.1). Combat against and by the actor goes on.

### 7.2 Fee and obligation first

The one DUR-03 transaction of NPC-0 §6 commits: fee lines, cause record and pending arrival
together, or nothing. The runtime move follows the known commit. This is DUR-03 §7 (value
first) and the ITEM-USE-0 and RUNE-USE-0 §5.2 rule (burn before effect):

- moving first lets a crash or fence loss give a free trip;
- the move itself is runtime state and cannot share a database transaction; the pending arrival
  row is its durable half, so fee and move are atomic in effect: a committed fee always ends in an
  arrival (now, at a late commit, or at the next admission) or in a death that supersedes it.

A known abort releases the hold: nothing paid, refusal reply.

### 7.3 Arrival

On the known commit, in the owner's tick, the channel runtime relocates the actor within its
scope (D37 R1 shape, owner VSL-MOVE-01):

- **Fences:** WorldId, ChannelId, session generation, content generation, and the position
  revision observed at the hold. A mismatch leaves the obligation for admission.
- **Tile:** the pending arrival; if not free, the NPC-0 §6.1 spiral, then the home temple
  (ruling R1).
- **Effects of the move:** summons and convinced creatures are removed (R3); the attack target
  follows ATTACK-0 validity (lost when not visible); the logout block stays with the actor.
- The arrival is a relocation, not a step: it is sent through the existing spatial deltas.

### 7.4 Consuming the obligation

A follow-up transaction under the composition rule 2 session fence deletes the pending arrival
and upserts the last position with the placed tile, the same shape as admission
(CHAR-POSITION-0 §3.3). A stale session writes nothing and admission consumes the row. Terminal
release consumes an arrival whose actor has arrived, before its last-position write
(CHAR-POSITION-0 §3.2 as amended). Admission of a row whose actor never arrived places it (NPC-0).

### 7.5 Ambiguous commit

After `TRAVEL0-RL-02` (2,000 ms) the outcome is ambiguous: the invocation stays pending until
reconciliation by occurrence replay (FND-02 §13.3), and **the hold is kept** (architect ruling,
fail closed): the actor stays frozen, so its position revision cannot advance past the one
captured at the hold (§7.3) and a late commit can still place and consume the arrival. The hold
is released only at a known outcome: a commit places the actor and consumes the arrival (§7.3,
§7.4); an abort releases it and pays nothing. The 2,000 ms bound raises the ambiguity alert and
the long-hold metric, it does not release the hold. The 2,000 ms bound therefore never applies to
logout or channel transfer: while the hold is unresolved, terminal release and channel transfer
are deferred (the session and its generation stay alive, a transfer request waits or is refused
as busy) and complete only after the known outcome and, on a commit, the consume (§7.4), within
the normal logout and transfer bounds counted from that outcome. If the session ends anyway
(process loss, forced termination), the late outcome stays session-generation fenced: its consume
is stale and writes nothing, and the committed pending arrival is placed by admission (NPC-0).
No late outcome is ever applied to a newer session or channel.

## 8. Channel and World

- Travel **never changes channel or World**. The destination is a tile of the base map that every
  channel of the World shares (ADR-0001 §7); the arrival happens in the traveller's channel.
- The pending arrival row stores no channel (CHAR-POSITION-0 §3.1): after a crash, admission
  chooses the channel as usual.
- Travel from or to an instance scope is refused or held (`SCOPE_HANDOFF`, D37 R3).

## 9. Replay and idempotency

- The occurrence comes from the confirming `NPC_TALK_INTENT` CommandRef (NPC-0 §5.1). A replay
  of that command returns the first outcome while FND-02 retains it; it never burns or relocates
  twice. The TransactionId and cause record key come from the occurrence.
- The arrival is keyed by the pending arrival row: once consumed, nothing moves again.
- A later `yes` is a new command with no open confirmation: nothing happens.

## 10. Wire

None new. Confirmation, refusals and "Set the sails!" are NPC-0 reply lines in domain 7 under
capability 3 `NPC_SERVICE_V1`; the confirmation timeout is `NPC0-RL-06`. The move reaches the
client as a spatial relocation. The teleport effect is presentation and waits for an effect
domain (`PARITY_PENDING`).

## 11. Rows (registered by each child before implementation)

| Row | Value |
|---|---|
| `TRAVEL0-RL-01` travel holds per actor | 1 (the conversation's pending invocation) |
| `TRAVEL0-RL-02` hold to known outcome, ambiguity alert bound | 2,000 ms; p99 measured by NPC-TRAVEL-1 |
| `TRAVEL0-RL-03` re-travel cooldown | 3 s, `PARITY_PENDING` |
| `TRAVEL0-RL-04` discounts per route | 4 |
| `TRAVEL0-RL-05` routes per travel service | 32 (content today: 11) |
| `TRAVEL0-RL-06` travel commits per channel per second, p99 | measured by NPC-TRAVEL-1 |
| `TRAVEL0-RL-07` bytes per canonical travel keyword | 32 |
| `TRAVEL0-RL-08` keywords per route | 8 |
| DUR-03 rows, travel shape | gold fee rows + 1 cause record + 1 obligation row (NPC-TRAVEL-1) |

## 12. Rejected options

- **Move first, then charge.** A crash or fence loss gives a free trip; DUR-03 §7 forbids it.
- **Charge, then refund a failed move.** A refund is a new value source; with the obligation the
  move cannot fail after the commit.
- **A travel command or capability.** Travel is talk in Tibia; NPC-0 §8 rejected it.
- **A new fee cause for discounted fares.** The sink is the same; the record carries the discount.
- **Refusing travel under the logout block alone.** Not Global (R2).
- **Refusing travel with a summon.** Not Global (R3).
- **A free-travel switch** (`toggleTravelsFree`). A server option, not Global.
- **Routes in `rulesets/`.** A route belongs to one NPC's content, not to a World rule.

## 13. Owner-rule applications

**Global parity kept:** talk and confirm; fare from coins then bank; free routes; level,
Premium and quest gates; the Postman discount; the PZ-block refusal; ships and carpets as one
shape; a trip that never changes World.

**Declared differences:**
- No stacking on the arrival tile; the spiral fallback instead (R1).
- The travel hold freezes walking for about one commit (`TRAVEL0-RL-02`); an ambiguous commit
  keeps it until reconciliation.
- Cooldown value, Postman rank and the teleport effect are `PARITY_PENDING`.
- `kick`, the ore-wagon ticket and Travora are not built (§14 R1, §16).

## 14. Architect rulings (owner rule 5905825574)

**R1. Crowded arrival tiles.** Tibia stacks travellers on the arrival tile and offers `kick`.
The runtime keeps one creature per tile, and stacking would change movement, visibility and
pushing. Ruled: keep NPC-0 §6.1 (nearest free tile within 3). `kick` is then not needed and is
not built.

**R2. Logout block.** NPC-0 §6 refused travel while logout-blocked. In the manual, ships are
protection zones that a PZ-blocked character cannot enter; Canary refuses only the PZ lock.
Ruled: refuse the PZ block and the kill block; the logout block alone allows travel.
Superseding evidence: an official Global source.

**R3. Summons.** Canary removes a summon left far behind; familiars follow. Ruled: summons and
convinced creatures are removed at the arrival, with no value; familiars follow when a familiar
decision exists.

## 15. Architect ruling (applying owner answers Q3a and H2a)

**T1. Premium destinations before Premium exists?** Tibia keeps Premium areas (everything reached
only by boat or carpet) for Premium accounts; the Game reads everyone as Free until
PREMIUM-DELIVERY-0 is delivered. a) Everyone may travel to Premium destinations until Premium is
delivered, then Premium only, as the Market (Q3a) and houses (H2a) (recommended: the world is
playable now); b) Premium destinations stay closed until then. The answer also sets PREM-3's
area entry by step or teleport before delivery, so every way in agrees. **Ruled a)**: the owner
already answered the same question for the Market (Q3a) and houses (H2a); this applies those
answers and asks nothing new. The end of the open period is PREM-1's activation record, the
PREMIUM-DELIVERY-0 §6 switch-over (§6 step 3).

D178 needs no question: D208 already admits travel fees, and discounts only lower that sink.

## 16. Decision test

- **Must decide now:** YES. The owner asked for travel now; NPC-0 left discounts, blocks, the
  arrival step and the held routes open.
- **Minimum sufficient:** three route fields, one fare formula, one refusal order, one runtime
  hold, one consume transaction; the NPC-0 wire, fee plan, cause and obligation are reused.
- **Superseding evidence:** Global sources for the cooldown, the Postman rank, logout-block travel,
  or stacking.
- **Deliberately not decided:** `kick`, the ore-wagon ticket, Travora, familiars, the travel
  achievement, the teleport effect, travel between Worlds.

## 17. Before-freeze checklist

1. **Contract amendments:** NPC-0 §6, CHAR-POSITION-0 §3.2, the NPC admission v2 decision §6,
   each written "pending on acceptance of TRAVEL-0". DUR-03 is unchanged.
2. **Serialization:** the item writer's fence with the `character_root` lock; one travel hold per
   actor; the consume under the rule 2 session fence.
3. **Restart:** conversation and hold are runtime; the fee, cause record and pending arrival are
   durable; admission consumes a pending arrival.
4. **Typed references:** NPC, route key, discount keys, occurrence, World and tile position.
5. **Wire:** none new (§10).
6. **Split work:** one transaction per trip, one consume per arrival.
