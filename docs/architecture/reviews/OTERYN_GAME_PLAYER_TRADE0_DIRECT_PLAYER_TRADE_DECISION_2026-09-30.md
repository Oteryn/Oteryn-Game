# PLAYER-TRADE-0 Direct player trade

- Decision: `PLAYER-TRADE0-DIRECT-TRADE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence and security) and protected integration. It extends ITEM-MOVE-WIRE-0 (PR #1344) and
  ITEM-MOVE-WIRE-1 (PR #1354) and integrates after them.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to start player trade (2026-09-30, "handel między graczami",
  verbatim record on #162 5914137757)
- Builds on: the scope matrix row "Direct player trade: ChannelRuntime + durable transaction,
  Channel, same channel only"; DUR-03 §7, §11.3, §28, §31, §34 and §39; GAME-ITEM-01 §4.7; B3
  (D80-D83); ITEM-MOVE-WIRE-0 and -1; the composition decision §3 rules 2-4; FND-02 §13.3 and §15;
  FND-ID-01; MOVE-RL-11 (D85, VIS-2); owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of PLAYER-TRADE-0 (#162 5912405163): DUR-03 (the trade swap
  shape, a paragraph after §34); the composition decision (two Characters in one transaction, a
  paragraph before its §4)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| TRADE-WIRE-1 | impl, protocol review | the capability, the trade command and domain (§3) | ITEM-VIEW-1; VIS-2 |
| TRADE-1 | hard, persistence and security review | the trade session and its TRANSFERRING state (§4), the swap transaction and every persistence delta (§5, §6) | TRADE-WIRE-1; ITEM-MOVE-2a |

Later, with their own decisions: containers with contents (after bags), ground items, equipped
items as offers before ITEM-MOVE-2a, capacity (B3-3), trading on house tiles.

## 1. Question

How do two players exchange items safely?

## 2. Facts

**PROVEN**

- The scope matrix: direct trade is owned by the ChannelRuntime with a durable transaction, on one
  channel only; it is a channel-change gate ("rejected or delayed").
- DUR-03 §7.1: reserved value is unavailable while pending; §7.3: an ambiguous outcome stays
  non-spendable; §31: a reserved command survives a same-GameSession reconnect; §34: custody spanning
  transactions only when every step is safe.
- The composition decision binds one Character per writer: the cause's CharacterId equals the fenced
  Character (rule 2), ingress is proven by the owner still holding the CommandRef as pending
  (rule 3).
- Migrations assume one Character per transaction: receipts keyed by CommandRef with a unique
  TransactionId, reservations with one `character_id`, placement proofs matching the receipt's
  Character, a consistency guard deriving the slot from that Character (`0011`); item changes and
  entry removals proven only by TRANSFER, retire or fee receipts (`0023`); the audit outbox names one
  item in a 9,216-byte envelope (`0010`).
- FND-ID-01: one online character per Account, so the two traders are on different Accounts.
- Content: every Item's `tradeable` is `UNKNOWN`; five records know only `marketable: false`, which
  is a Market flag, not a trade flag.

**CIPSOFT_OFFICIAL** (the Tibia manual, `controls_trading.md` §4.3.2)

- Trade starts with "Trade with ..." on an item and a target; the partner answers with a
  counter-offer; both must accept; a change after the dialog opens cancels the deal; both need room
  (and capacity) for the incoming items; whole containers can be offered; at most 100 items per
  trade.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- One item per side; partners within 2 tiles on the same floor with a sight line, checked at the
  request and at the accept; the offered item within 1 tile; no trade with oneself; store, owned,
  unique-id and non-pickupable items refused (`game.cpp:5804-5925`, `player.cpp:12390`).
- A partner sees the offer after its own counter-offer (`game.cpp:5967-5976`).
- When both accept, the state becomes `TRADE_TRANSFER`; room is checked before anything is
  removed, so the giver's freed slot does not count; received items are not merged
  (`FLAG_IGNOREAUTOSTACK`); a full inventory fails the trade with no drop (`game.cpp:5986-6045`).

## 3. Wire (TRADE-WIRE-1)

- **Capability `PLAYER_TRADE_V1`**, which requires capability 4; its number, one command type and
  one state domain are reserved on #162 at allocation. An offer to a partner without it is
  `NOT_SUPPORTED`.
- **Command `PLAYER_TRADE_INTENT`**, a oneof (an empty oneof is `REJECTED`):
  - `offer {partner: {actor_id, generation}, item: handle}`: starts a trade, or answers a started
    one with the counter-offer;
  - `accept {trade_id}`: accepts the trade the server identified when both offers existed;
  - `cancel`.
- **`trade_id`.** The server issues one when the second offer exists. Any change cancels the
  trade, so a new trade has a new id; simultaneous accepts of the same id both count.
- **Domain `PLAYER_TRADE`**, on both sessions: the trade id, partner, the session's own offer, and
  the partner's offer only once the session has made its own (Canary parity: a partner sees the
  offer after its counter-offer; `game.cpp:5967-5976`), each offer as (definition, count, sub-type), each side's acceptance, the state (`OFFERED`, `READY`, `TRANSFERRING`, `CLOSED`)
  and, when closed, the reason (`COMPLETED`, `CANCELLED`, `NO_ROOM`, `STALE`, `REJECTED`).
  Runtime-local; revision monotonic per GameSession (FND-02 §15).
- **Results** (at most 4 bytes): `OFFERED`, `ACCEPTED` (waiting for the partner), `COMPLETED`,
  `CANCELLED`, `TOO_FAR`, `NOT_TRADEABLE`, `PARTNER_BUSY`, `NO_ROOM`, `NO_BACKPACK`, `STALE`,
  `NOT_SUPPORTED`, `REJECTED` (any other writer refusal or authority failure).

## 4. Trade session (TRADE-1, runtime)

- **Owner.** The channel runtime holds at most one trade per actor, both actors on the same
  channel. Nothing durable is written before the swap: the `TRANSFERRING` reservations below are
  in-memory runtime reservations, not DUR-03 reservation rows. After a crash they are gone; safety
  then rests on the swap transaction itself (§5 re-checks both sources and both free entries under
  lock) and on replaying the trade occurrence, which returns the first outcome.
- **Timeout.** A trade in `OFFERED` or `READY` closes as `CANCELLED` after `TRADE0-RL-02` (120 s)
  without a new offer or accept, so no player can hold another in `PARTNER_BUSY`.
- **Start.** The partner is another player, visible to the session, within 2 tiles on the same
  floor with a sight line, not oneself, and with no open trade (`PARITY_PENDING`).
- **Offers.** One whole item per side from the offerer's main backpack direct entries (equipped
  items after ITEM-MOVE-2a). Refused as `NOT_TRADEABLE`: the main backpack itself (the container
  slot item), an item with contents (bags come later), a corpse or ground item, a non-pickupable
  item, and an item whose content marks it untradeable. `tradeable: UNKNOWN` is tradeable, as most
  Tibia items are (Canary default); `marketable` is not read as a trade flag. Store-bound items do not
  exist yet; their decision adds the refusal. An offer is a reference, not a move.
- **Accept.** Allowed only in `READY` (both offers exist). Range and sight line are checked again
  at each accept.
- **Changes cancel** before `TRANSFERRING`: any committed or issued change to either offered item
  or to either destination backpack, from any cause (a move, use, drop, equip, fee, NPC sale, death,
  a reward mint that fills the last entry), and leaving range, logout, a reconnect or a channel
  transfer.
- **TRANSFERRING.** When the second valid accept of the trade id arrives, the runtime issues one
  trade occurrence bound 1:1 to that accept's CommandRef, reserves both offered items and one free
  entry in each destination backpack (DUR-03 §7.1), and runs the swap. While it runs, commands that
  would touch the reserved items or entries are refused, a channel transfer is delayed, a cancel is
  refused, and a reconnect keeps the trade (DUR-03 §31). Only the database outcome closes it; an
  ambiguous outcome keeps the reservations until reconciliation.
- **Logout, death or kick during `TRANSFERRING`.** The swap already holds both fences, taken at the
  accept, so it commits or aborts on its own. A logout or kick of either side waits for the outcome
  before the session closes (bounded by the transaction timeout); a death is processed after the
  outcome, serialized on `character_root`. Either way the other side sees the outcome.

**Amendment (pending on acceptance of BAGS-0;
`reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` §9).**
An offer may be a container tree of at most 100 items (BAGS-TRADE-1). The offer binds each item's
id, definition, quantity, state, immediate parent and entry ordinal; any change, a reparent or
reorder included, cancels, and the swap checks the binding under row locks. Every item of the tree, not only
the root, must pass the `NOT_TRADEABLE` and binding checks, rechecked under the swap locks. The main backpack
itself stays refused. Under `CONTAINER_TREE_V1` the §3 domain gives the recipient's session its
own handles for each offered container and each container nested in it; they open read-only
views only and become `STALE` on any offer change and when the trade ends. A swap with at least
one tree uses the BAGS-0 §9 two-tree shape and its rows (`DUR03-RL-0x-TREE-TRADE`: 200 touched
items, 2 participants, 4 location lines, at most 204 work units, both tree bindings in the receipt
and event), which override the §6 two-item rows for that swap only.

## 5. Swap (TRADE-1, persistence)

- **One transaction** moves both items or neither:
  - A's item into a new direct entry of B's main backpack;
  - B's item into a new direct entry of A's main backpack.
  - Received items are never merged (Canary parity, `PARITY_PENDING`). Room is checked before the
    sources are removed: each side needs one free entry of its own, so a full backpack cannot trade
    (Canary parity, `PARITY_PENDING`). Capacity waits for B3-3.
- **Fences.**
  - The triggering side (the second accepter) takes the composition rule 2 fence with its pending
    CommandRef.
  - The first accepter is authorized by its terminal accept: the cause record stores its accept
    CommandRef and the accepted trade id, and its fence is pinned to the GameSessionId, Character
    lease and scope ownership generation captured at its accept; only its connection generation may
    advance.
  - Both fences are checked in the transaction; a stale one on either side commits nothing. Replay
    waives the live checks for both only under the recovery fence.
- **Lock order.** The recovery fence and admission relations, the trade cause, both fence checks,
  both `character_root` rows in `character_id` order, the two items in ItemInstanceId order, then
  the container-slot rows in `character_id` order (the deferred `0011` placement check).
- **Sources checked.** Each item must still be at its offered location with the offered
  definition, count and state; otherwise `STALE`. The database check is the backstop for any change
  the runtime missed.
- **Cause and replay.** One trade cause record keyed by the trade occurrence, naming both
  Characters, both items and both fences; the same occurrence and binding replay the first outcome.
- **No custody family.** Items stay with their owners until the one swap, so DUR-03 §34 is not
  needed.
- **Revision.** Item-only for both Characters (composition rule 1): no `CharacterRevision` advance.

## 6. Persistence deltas and rows (TRADE-1)

- **Tables:** the trade cause row (occurrence, both Characters, both fences, both CommandRefs, the
  TransactionId); a per-line trade receipt table keyed by (occurrence, line) with source and
  destination Character per line.
- **Guards:** the trade receipt lines become an alternative proof in the placement proof, the
  consistency guard, the item-change proof and the entry-removal proof (`0011`, `0023`), each
  matching the line's own Character rather than a single receipt Character.
- **Audit:** one trade event with both TRANSFER lines, as a new audit shape with its own envelope
  row.
- **Rows** (fixed here, registered by TRADE-1 before implementation, DUR-03 §28):

| Row | Value |
|---|---|
| `DUR03-RL-01-TRADE` touched items | 2 |
| `DUR03-RL-02-TRADE` location lines | 4 |
| `DUR03-RL-06-TRADE` participants / effect work units | 2 / 6 |
| `DUR03-RL-07-TRADE` envelope / payload | measured for the two-line event within the ANL ceilings; a value above 9,216 / 7,936 bytes returns the shape for a new decision |
| `TRADE0-RL-01` trade commands per actor per second | measured and registered by TRADE-WIRE-1 |
| `TRADE0-RL-02` idle trade timeout | 120 s in `OFFERED` or `READY` |

## 7. Other amendments

- **DUR-03** (pending on acceptance): a paragraph after §34 admits the two-Character trade swap for
  its cause only, superseding the §39.1 one-item and one-Character limits for it.
- **Composition decision** (pending on acceptance): one transaction may fence two Characters for a
  trade swap, the first accepter authorized by its terminal accept with pinned session, lease and
  scope generations, both root locks in `character_id` order.

## 8. Rejected options

- **Moving offered items into a trade custody first.** It adds a family and a two-step workflow;
  reserving and swapping once is safe and simpler.
- **Several items per side in the first slice.** Tibia trades one item (or one container) per side;
  containers wait for bags.
- **Merging received stacks, or counting the freed entry.** Canary does neither.
- **Cross-channel trade.** The scope matrix limits trade to one channel.
- **A digest of the offers in the accept.** Per-session revisions would make simultaneous accepts
  stale; a trade id that any change retires does the same job.

## 9. Decision test

- **Must decide now:** YES. The owner asked for player trade now.
- **Minimum sufficient:** one command, one domain, one runtime session with a transfer state, one
  two-item swap transaction.
- **Superseding evidence:** official trade distance, room or item rules.
- **Deliberately not decided:** containers with contents, ground items, equipped offers before 2a,
  capacity, house tiles, trade logs for players.

## 10. Before-freeze checklist

1. **Contract amendments:** DUR-03 (after §34) and the composition decision (before its §4), both
   pending on acceptance of PLAYER-TRADE-0.
2. **Serialization:** one swap transaction; two fences; the lock order of §5.
3. **Restart:** sessions are runtime state; a trade in `TRANSFERRING` survives a reconnect and ends
   only with its database outcome; items are durable.
4. **Typed references:** D85 identities, handles, trade id, trade occurrence.
5. **Wire:** §3, capability `PLAYER_TRADE_V1`.
6. **Split work:** one trade, one transaction, two items.
