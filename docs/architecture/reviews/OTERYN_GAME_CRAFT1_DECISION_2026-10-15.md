# CRAFT-1 Item conversions: crafting, fishing, digging and cooking

- Decision: `CRAFT1-ITEM-CONVERSIONS-V1`
- Status: **CANDIDATE** (architect-authored). Acceptance needs exact-head validation, independent review (determinism, persistence and economy) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the mechanics completion plan rows 89 and 91 (`OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`, batch CRAFT-1: "crafting, fishing, digging, cooking"), and the owner's direction of 2026-10-15 (#1622 comment 6187301378, architect ruling A: write the CRAFT-1 brief).
- Builds on: ITEM-LOOP-1 (drop, pickup, stacks, capacity), ITEM-USE-0 (use-with, `UseItemEx`), the minted-item rules of ITEM-LOOP-1 and DUR-03 (every created item is a MINT, every consumed item a retirement), and the deterministic RNG purposes of ATTACK-0 §4 as the pattern for a new purpose.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE until accepted and allocated.

## 1. Scope

Item conversions the player triggers: "use X with Y" and a small set of NPC exchanges. Tibia parity, nothing more:
- **Use-with conversions** (crafting combos): one item or stack used on another, or on a map object, becomes one or more items. Reference example: water on flour gives dough (USED a water container, e.g. a vial or bucket, matched by `item_id` plus `sub_type` = the water fluid type, count 1, with `effect` = TRANSFORM_TO the same container at `sub_type` 0 with `lifecycle` = PRESERVE_INSTANCE (emptied; the container is not consumed); TARGET flour, count 1, TRANSFORM_TO dough with `lifecycle` = REPLACE_INSTANCE; the table lists both operands, so the target flour is never consumed implicitly), dough on an oven gives bread (`docs/reference/tibia-manual/controls.md`, "Use with", "crafting combos exist").
- **Fishing:** fishing rod plus worms on a water tile that holds fish. Each successful attempt consumes one worm. The catch chance scales with the Fishing skill. Not every water tile has fish, and a tile does not yield again right after a catch (`docs/reference/tibia-manual/characters.md` §Fishing; `world.md` water rules).
- **Digging:** shovel or pick used on a diggable ground item, which turns it into its opened form (a tile transformation with a timed return), as the upstream tools do (`data/scripts/actions/tools/shovel.lua`, `pick.lua` in the Canary reference).
- **Cooking:** food conversions (raw to cooked) by use-with a heat source; hireling cooking is a service of HIRE-1, not of this brief.
- **NPC exchanges:** an NPC that takes items and gives items under a fixed table (a conversion with no gold side).

Out of scope, each its own decision: imbuing and the Exaltation Forge (IMBUE-FORGE-0), runes made by spellcasters (MAGIC-ITEMS-1), the Store, the market, coin changing (BANK-0; coin stacks convert 100 to 1 by use, already a bank/item rule), and new recipes the reference does not have.

## 2. Reference grounding

Evidence read for this brief, under `docs/reference/`:
- `tibia-manual/controls.md` (use-with, crafting combos), `characters.md` (Fishing skill: rod, worms, catch chance by skill, one worm per success, not all tiles have fish), `world.md` (fishing tile cooldown and failure mechanic: "unspecified, needs a numeric model"), `interface.md` (the forge is separate).
- The numeric model is **not** in the manual. It comes from the Canary reference (`data/scripts/actions/`, `data/libs/`), which the repo already treats as upstream evidence. The brief fixes the shape; the numbers are content, taken from the reference at the content step (§4) and checked by a parity test.

Facts the brief relies on:
1. A conversion is an item-to-item rule keyed by (used item id, target item id or target tile kind).
2. Success is skill-gated for fishing only. All other conversions are certain.
3. Inputs are consumed on success. A fishing worm is consumed per successful catch, as the manual says.
4. Every conversion that creates an item mints it; it never moves a hidden item.

## 3. Rules

**C1. The conversion table is content.** One table in `content/` (family `item_conversion`, source-provenance per row), compiled into the bundle. A row: `input_item`, `target` (item id, tile kind or none), `operands` (one entry for each item the conversion touches, the used item and the target: `role` = USED, TARGET or INGREDIENT, `item_id`, an optional `sub_type`, `count`, and `effect` = CONSUME, TRANSFORM_TO (an item id, for an item that stays in place as another item) or KEEP (a tool such as a rod or shovel); a conversion with no TARGET operand targets a tile kind). **INGREDIENT is a consumed operand that is not the used item or the target:** it names an item the actor holds separately (the fishing worm), has `effect` = CONSUME only, and is resolved by `item_id` (plus `sub_type` when stated) and `count` from the actor's own containers in the server's fixed container order, never chosen by the client; it carries `consume_on` = ALWAYS or SUCCESS (a fishing worm is SUCCESS: retired in the C3 commit of a successful catch only, never on a failed roll or a refusal). An INGREDIENT the actor lacks in the required count is a refusal before any roll. **Every TRANSFORM_TO operand declares `lifecycle` = PRESERVE_INSTANCE or REPLACE_INSTANCE (DUR-03 §16), validated per row and never derived from the output:** PRESERVE_INSTANCE keeps the ItemInstance uid, lineage and container contents and changes only its definition or subtype (used for a fluid container emptied or filled); REPLACE_INSTANCE retires the old instance and mints a new one with a new uid in the same C3 transaction (flour to dough), each with the conversion cause for audit. The compile check rejects a TRANSFORM_TO operand with no `lifecycle`, a REPLACE_INSTANCE whose count differs from its operand, and a PRESERVE_INSTANCE that would change a stack count. **Fluid operands match by `item_id` plus `sub_type`.** Portable fluid contents are ItemInstance subtype state, not an item id (`OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`: current count, subtype, charges and container contents belong to ItemInstance; portable fluid containers stay Item capabilities), so a fluid recipe operand names the container `item_id` and the required fluid `sub_type` (the fluid type enum, e.g. water = 1), the same `sub_type` field NPC offers carry. An operand with no `sub_type` matches only an instance with subtype 0 (an empty container, or an item with no subtype); it never matches by `item_id` alone across fluid states. The effect on a fluid container is TRANSFORM_TO the same `item_id` at a stated output `sub_type` (an emptied container is `sub_type` 0), committed as an ItemInstance subtype change in the C3 transaction; the container is never retired or minted unless the row says CONSUME. The compile check rejects a row whose fluid operand lacks `sub_type` and a parity test covers wrong-fluid and empty-container refusals, `outputs` (item id and count list, or a weighted list), `skill` (none or Fishing), `chance_curve` (fishing only), `tile_cooldown_ms` (fishing only, integer, required for fishing rows, bounds 1 to 3600000, read by C4; the compile check rejects a fishing row without it and a non-fishing row with it), `tool` (rod, shovel, pick, none), `capacity_check`, `result_message`. Code holds no recipe. A row absent from the table is not a conversion.

**C2. Deterministic RNG.** The only random conversion is a weighted output or the fishing catch roll. It draws from a new registered RNG purpose `ConversionRngPurpose` (variants `FishingCatch`, `WeightedOutput`), seeded from the world seed, the character id, the item uid and the per-character conversion counter, following the ATTACK-0 §4 RNG-purpose pattern. Same inputs, same result. No wall-clock or global RNG. A replayed or retried intent returns the original result (idempotent on the intent id), so a retry cannot reroll. The conversion counter is defined in C2a.

**C2a. The conversion counter is durable and reserved per attempt.** The per-character `conversion_seq` is the number of conversion attempts the character has durably committed. Every attempt that passes the preconditions and the capacity check of C3 and then needs a draw (a fishing roll or a weighted output) is one attempt, and it commits exactly one `ConversionAttempt` record keyed by (character id, intent id) that advances `conversion_seq` by exactly 1, whether the roll succeeds or fails:
- The draw uses the value n = `conversion_seq` read under the `character_root` row lock inside the transaction that will commit, so the seed (world seed, character id, item uid, n) is bound to the durable counter and the intent, never to memory.
- A successful attempt commits the counter advance in the same C3 transaction as the retire and mint. A failed fishing roll commits a counter-only transaction (the `ConversionAttempt` record, plus the skill try when SKILLS-0 grants one); it retires and mints nothing and consumes no worm. A refusal before the roll (a precondition or C3 capacity refusal) is not an attempt and does not advance the counter.
- Because n advances on every committed attempt, the next fresh attempt with the same rod and tile draws from n+1 and is not a repeat of the failure. The `ConversionAttempt` record keyed by intent id makes a replayed or retried intent return the recorded outcome without a new draw. A crash before commit leaves n unchanged, so the retry redraws the same n and gets the same result, which is correct because nothing was committed; a crash after commit leaves n+1 and the record, so failover and restart cannot change an outcome.
- Classification under DUR-02: a transaction whose Character effects are only item locations, the `ConversionAttempt` cause record and the counter, is a DUR-03 value or cause-record transaction and does not advance `CharacterRevision` (the CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION rule); it is fenced on the session generation and serializes on the `character_root` lock. When the same transaction also advances Fishing skill progress (a Character semantic change, SKILLS-0), it advances `CharacterRevision` exactly once for the whole transaction. CRAFT-RT-1 confirms this classification in its persistence review.

**C3. Atomic mint and retire.** One durable item conversion is one persistence transaction (volatile MAP-OVERLAY transformations such as digging under C5 are exempt: they retire and mint no items and carry no persistence transaction): the consumed inputs (CONSUME operands, INGREDIENTs whose `consume_on` fires, and the old instance of a REPLACE_INSTANCE transform) are retired and the outputs (and the new instance of a REPLACE_INSTANCE transform) minted with a conversion cause, while a PRESERVE_INSTANCE transform changes the existing instance in place and retires and mints nothing, or nothing changes. The destination of every output is the actor's containers; no conversion places an output on the ground. The capacity check runs before the roll, against the row's worst-case output (the largest weighted output, or the single output). If the outputs do not fit (capacity, container full), the conversion is refused before anything is consumed and before any draw, with the ITEM-LOOP-1 refusal ("not enough room" or "too heavy"): no worm is spent, no counter advance, no skill try. If the state changes between that check and the commit so that the outputs no longer fit, the transaction rolls back whole with the same refusal. No output is dropped to the ground silently.

**C4. Fishing.**
- Preconditions: the fishing row is rod = USED (`effect` KEEP), water tile = the tile-kind target, and worm = an INGREDIENT operand (`effect` CONSUME, `consume_on` SUCCESS, count 1) resolved from the actor's containers per C1; a target water tile with fish. No worm means a refusal and no roll; the worm is retired in the same C3 transaction as the fish mint, on success only. The attempt then follows C3 (capacity before the roll) and C2a (counter).
- **Source multiplicity (`GAME-CHANNEL-01`): `ChannelLocalRepeatable`, eligibility scope `CharacterWorld`.** Fishing is a repeatable source, not a unique or shared-eligibility one. The simulation is channel-local: the fish presence and the tile cooldown are keyed by (`ChannelId`, tile), so the same water tile in two channels has two independent cooldowns and a catch in one channel does not cool the other. No durable eligibility is shared across channels, because the durable value is bounded without it: every successful catch consumes one worm (a retirement), the catch roll is skill-gated, and the per-character `conversion_seq` and the C3 transaction apply in every channel alike, so a character gains no extra attempt by changing channel beyond what its own worm supply and actions allow. The World stays the one economy boundary (worm and fish are World items); the owner can later re-class fishing as `ChannelLocalSharedEligibility` by amending this line, which changes only the cooldown key to (`WorldId`, tile).
- The catch roll uses the Fishing skill and the `chance_curve` of the row. The skill-try rule of SKILLS-0 applies (a fishing attempt may advance Fishing whether or not it succeeds, as the reference does); this brief only names the hook, SKILLS-0 owns the amount.
- A tile that just yielded does not yield again for the row's `tile_cooldown_ms`, measured on the deadline clock of the world (not wall time). The cooldown is channel-local volatile simulation state keyed by (`ChannelId`, tile), not persisted across a crash (like the ADR-0021 volatile map state). It is set only after a committed successful catch; a failed roll does not start it.
- **Destination policy: one outcome.** A fish is minted only into the player's container; there is no ground destination. When the container has no room for the worst-case catch, the attempt is refused under C3 before the roll: no roll, no worm consumed, no fish created, no counter advance, no skill try, and the tile cooldown is not started.

**C5. Digging and tile transformation.** A use-with on a diggable ground item replaces it with the opened form for `revert_ms` and returns it afterwards. The change is volatile map state under ADR-0021 (a crash resets it, as does the planned world reset). Items stacked on the tile move with it per the ITEM-LOOP-1 tile rules. No permanent tile edit is allowed.

**C6. NPC exchanges.** An exchange is an NPC-trade row (NPC-TRADE-1 writer) with `give` items and `take` items and no gold. It reuses the NPC trade validation (distance, focus, capacity, bank-free). It is one transaction under C3 (a durable item conversion).

**C7. Refusals are fixed messages** from the table's `result_message` set; no new free text in code.

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance; each its own PR.
1. **CRAFT-CONTENT-1** (content): the `item_conversion` family, schema, generator and CI check, with the Canary-derived rows and provenance. No runtime.
2. **CRAFT-RT-1** (hard, determinism and persistence review): the conversion engine, `ConversionRngPurpose`, C2 to C3, tests with fixed seeds, the idempotent retry, the C2a counter (a failed roll advances it, a crash before and after commit) and the DUR-02 revision classification.
3. **CRAFT-FISH-1** (impl): C4 (`ChannelLocalRepeatable` keyed by `ChannelId` and tile, the single full-container refusal), with the SKILLS-0 hook.
4. **CRAFT-DIG-1** (impl): C5 on the MAP-OVERLAY volatile layer.
5. **CRAFT-NPC-1** (impl): C6.
Wire: use-with is the existing ITEM-USE command. No new wire is expected; if a result needs a new message, that is a separate contract candidate.

## 5. Open points (not decided here)

- The Canary fishing curve and worm/bait variants: the numbers are content and take the reference's values; whether Oteryn adopts later bait types is an owner question for the content packet.
- Cooking foods that give timed buffs belong to TIMED-ITEM-0B and HIRE-1's Cook; CRAFT-CONTENT-1 lists only the conversion, not the buff.
- The exact set of NPC exchanges is whatever the reference NPC content already has; no new exchange is invented.

## 6. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "determinism, persistence, economy"
```
