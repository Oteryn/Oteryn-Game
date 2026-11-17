# LOTTERY donor findings and implementation plan

Status: research only; no production implementation or tests yet.

Crystal checkout: https://github.com/zimbadev/crystalserver at
201189f871d23597b82caf88fd611454a2ddefb8, retained at /workspace/crystal.

## Donor evidence

- data-global/npc/sandra.lua:108 removes 100 items of one kind, trying
  283, 284, then 285. Line 110 grants item 5957 deterministically:
  `player:addItem(5957, 1)`.
- data-global/npc/sandra.lua:117 consumes winning ticket 5958 for the
  outfit addon when the bonus-system storage is 1.
- data/scripts/actions/items/lottery_ticket.lua:4 draws
  `math.random(98) == 1`; line 6 transforms the used ticket into 5958.
  Lines 7-9 remove the ticket on loss. Weights are win 1, loss 97.
  The donor grants no blank item; its blank outcome consumes the ticket.

The requested NPC random exchange moves the draw into the NPC exchange
transaction. Preserve the donor weights and represent loss as no output
item, rather than inventing a blank item ID.

## Next implementation steps

1. Base update completed: fetched origin/main at
   e2152d050038cecdb3728c222af2ac184e1cac5a and merged into the task
   branch. The prior base mismatch is resolved.
2. Extend the existing exchange row with weighted outcomes, using the
   server RNG owner and the existing fenced atomic transaction.
3. Wire Sandra's dialogue through the production NPC programme caller.
4. Add mounted PostgreSQL cases for seeded win/loss, missing input,
   full inventory, command replay and exact donor odds. Run red then green.
5. Run formatting, touched-crate clippy and applicable content/check gates;
   regenerate derived content only through repository tools.

Reserved identifiers, only if needed: migration 2887, capability 540,
command/mechanic ordinal 6275.

The owner's latest instruction permits incremental WIP pushes after each
completed step. This commit records donor research only and claims no
runtime behavior or passing validation.

## Production integration points inspected

- crates/gameplay/quests/src/quest/npc_rules/item_exchange.rs:8 owns
  inventory key discovery and the bounded deterministic exchange planner.
- crates/gameplay/durability/src/durability/quest_state/npc_program.rs:300
  calls that planner during source evaluation; the durable transaction
  checks command replay before loading fenced inventory and reevaluating.
- Weighted selection must happen only after replay lookup and retain the
  existing inventory participant so refused outcomes roll back all inputs.

Bound META organization policy resolved from Oteryn/Oteryn at
1bfb5ff98c8aa156e73669a14e083a1d464c29fb through the GitHub contents API.
