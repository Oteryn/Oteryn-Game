# Reward chest playable slice: owner decisions v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE with owner decisions D39-D42 taken (§4). The owner agreed on the
  condition that the work is done correctly ("jeśli na pewno jesteś pewien, że zrobisz to dobrze,
  to masz zgodę").
- Acceptance: the owning contracts take these decisions into their text after the independent
  review that authority, persistence and value changes require. Until then nothing here gives
  runtime, schema or production authority.
- DeliveryStatus: OPEN
- ImplementationStatus: NOT_STARTED
- Relation: the smallest playable quest content (`OTERYN_QUEST_AUTHORING_FORMAT_V1.md` §8, runtime
  readiness), built on D32 (a chest is a `RewardClaim`) and D35 (the quest domain owns quest state).

## 1. The slice

A player uses a placed reward chest. The chest hands out its reward once per character, or once
per cooldown. When the player has no room, nothing happens and the player can try again.

Other parts of a quest wait for their own owners (§6):
- teleports, walls and levers (D37, D38);
- summons;
- NPC dialogue;
- quest missions.

## 2. Reference behaviour (Canary and CrystalServer)

Both servers share one implementation (`scripts/actions/system|other/quest_reward_common.lua` and
`startup/tables/chest.lua` at the pinned revisions):

- **Trigger.**
  - At startup the chest table assigns a unique id to the map item at each chest position.
  - One action handles the unique-id ranges.
  - The common handler has no level or premium check.
- **Claimed state.**
  - A per-character storage or KV entry, written after the hand-out.
  - Cooldown chests keep a next-allowed timestamp.
  - A second use reports the chest as empty.
- **Hand-out.**
  - Directly into the player's backpack.
  - A container reward is a fresh container holding the items.
  - Keys get their action id; written items get their text.
  - A random reward picks one entry.
- **Room.** Checked before the hand-out: the total weight must fit the free capacity, and the main
  backpack needs at least one free slot. Otherwise the player is told why, and nothing is written.
- **Atomicity.** None.
  - If the engine cannot place an item, the script still marks the chest as claimed, so the player
    loses the reward.
  - An error part-way can deliver the item without the claim.
- **Differences.** CrystalServer hands out a container reward in one call, with one message and
  one write. Canary repeats the message and the write per item.

## 3. What Oteryn keeps and changes

Oteryn keeps the player-visible behaviour: the trigger, per-character claims, cooldowns, the room
rule, a retry after a refusal, keys, written items and random rewards. It takes the single-call
container hand-out from CrystalServer. It changes two things:
- The item and the claim commit together or not at all.
- The room rule needs a free slot for every top-level reward item. Both servers check one slot and
  let the engine drop the rest to the ground; Oteryn never drops a reward to the ground.

## 4. Decisions

| # | Decision | Owner |
|---|---|---|
| D39 | The GAME-INTERACTION-01 successor candidate is accepted for the `USE` edge on a placed object. Its trigger identity, child identity and retry rules govern the chest interaction. The dependencies it leaves open (GAME-ABILITY whole gate, movement handoff, writable text) stay open and do not affect this slice. | GAME-INTERACTION-01 (coordinator audit and independent review take it into the contract text) |
| D40 | DUR-03 accepts a second MINT source cause beside the creature-death occurrence: a GAME-INTERACTION child occurrence of a `USE` on a reward-claim placement. The mint is idempotent per (claim, character): a repeated request with the same identity returns the first outcome and never creates a second item. | DUR-03 |
| D41 | The destination is the character's inventory under the reference room rule: the total reward weight fits the free capacity, and the main backpack has a free slot for each top-level reward item or for the reward container. When it does not fit, the request is refused with a reason. No item and no claim are written, the player may retry, and nothing falls to the ground. | DUR-03 with the Character owner |
| D42 | A `RewardClaim` record is its own per-character durable record, unique on (character, claim). It is not a quest-progress track and does not reuse the XP tables. A cooldown claim stores the next allowed time. The record commits in the same DUR-03 transaction as the minted items, fenced by the character session generation like the XP award. The item and the claim commit together or not at all. | DUR-03 with the Character owner |

## 5. Open point before implementation

DUR-03 §39.3 records a conflict. The character progression migration (0009) ties the character
revision to XP receipts only, and does not say how a non-XP inventory change or a claim record
moves that revision. D41 and D42 need that composition written in the DUR-03 and Character text
before any migration or runtime code. If the answer does not fit a single transaction, the work
stops and returns to the owner rather than choosing a weaker rule.

## 6. Not in this slice

- Quest missions and transitions (D35).
- Door gates.
- Teleports and map objects (D37, D38).
- Summons (GAME-AI-01).
- NPC dialogue.
- Achievements granted by a chest: they follow once an Achievement owner exists; the claim records
  the grant request.

## 7. Order of work

1. Independent review of D37-D42 on one pull request head.
2. DUR-03 and Character text for §5, D40, D41 and D42; GAME-INTERACTION-01 text for D39.
3. Migration and durability code for the `RewardClaim` record with the MINT transaction, with
   tests for:
   - refusal and retry;
   - idempotent repeat;
   - fence rejection;
   - no partial commit.
4. The `USE` trigger on a chest placement in the interaction runtime.
5. Loading one chest from content, then playing it in the server.
