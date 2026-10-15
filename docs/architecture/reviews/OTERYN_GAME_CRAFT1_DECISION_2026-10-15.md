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
- **Use-with conversions** (crafting combos): one item or stack used on another, or on a map object, becomes one or more items. Reference example: water on flour gives dough, dough on an oven gives bread (`docs/reference/tibia-manual/controls.md`, "Use with", "crafting combos exist").
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

**C1. The conversion table is content.** One table in `content/` (family `item_conversion`, source-provenance per row), compiled into the bundle. A row: `input_item`, `target` (item id, tile kind or none), `consumes` (count of input), `outputs` (item id and count list, or a weighted list), `skill` (none or Fishing), `chance_curve` (fishing only), `tool` (rod, shovel, pick, none), `capacity_check`, `result_message`. Code holds no recipe. A row absent from the table is not a conversion.

**C2. Deterministic RNG.** The only random conversion is a weighted output or the fishing catch roll. It draws from a new registered RNG purpose `ConversionRngPurpose` (variants `FishingCatch`, `WeightedOutput`), seeded from the world seed, the character id, the item uid and the per-character conversion counter, following the ATTACK-0 §4 RNG-purpose pattern. Same inputs, same result. No wall-clock or global RNG. A replayed or retried intent returns the original result (idempotent on the intent id), so a retry cannot reroll.

**C3. Atomic mint and retire.** One conversion is one persistence transaction: the consumed inputs are retired and the outputs minted with a conversion cause, or nothing changes. If the outputs do not fit (capacity, container full, no room on the ground), the conversion is refused before anything is consumed, with the ITEM-LOOP-1 refusal ("not enough room" or "too heavy"). No output is dropped to the ground silently.

**C4. Fishing.**
- Preconditions: rod in hand or inventory, a worm stack, a target water tile with fish. No worm means a refusal and no roll.
- The catch roll uses the Fishing skill and the `chance_curve` of the row. The skill-try rule of SKILLS-0 applies (a fishing attempt may advance Fishing whether or not it succeeds, as the reference does); this brief only names the hook, SKILLS-0 owns the amount.
- A tile that just yielded does not yield again for the row's `tile_cooldown_ms`, measured on the deadline clock of the world (not wall time). The cooldown is world state, not persisted across a crash (like the ADR-0021 volatile map state).
- Fish items are minted into the player's container or onto the tile if the player has no room (then C3 refuses instead).

**C5. Digging and tile transformation.** A use-with on a diggable ground item replaces it with the opened form for `revert_ms` and returns it afterwards. The change is volatile map state under ADR-0021 (a crash resets it, as does the planned world reset). Items stacked on the tile move with it per the ITEM-LOOP-1 tile rules. No permanent tile edit is allowed.

**C6. NPC exchanges.** An exchange is an NPC-trade row (NPC-TRADE-1 writer) with `give` items and `take` items and no gold. It reuses the NPC trade validation (distance, focus, capacity, bank-free). It is one transaction under C3.

**C7. Refusals are fixed messages** from the table's `result_message` set; no new free text in code.

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance; each its own PR.
1. **CRAFT-CONTENT-1** (content): the `item_conversion` family, schema, generator and CI check, with the Canary-derived rows and provenance. No runtime.
2. **CRAFT-RT-1** (hard, determinism and persistence review): the conversion engine, `ConversionRngPurpose`, C2 to C3, tests with fixed seeds and the idempotent retry.
3. **CRAFT-FISH-1** (impl): C4, with the SKILLS-0 hook.
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
