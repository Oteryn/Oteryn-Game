# QCX56 Threatened Dreams Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

Global sources are the fetched TibiaWiki [quest (81362, revision 1070008)](https://tibia.fandom.com/?curid=81362&oldid=1070008) and [spoiler (81363, revision 1202454)](https://tibia.fandom.com/?curid=81363&oldid=1202454). The Crystal 201189f8 comparison uses the [116-row donor inventory](../evidence/quests-2-20261102/threatened_dreams_quest.json) and captured NPC source. Inventory dispositions are historical and need checking against main.

| Part | Global compared with main and Crystal |
| --- | --- |
| Entry | Global lists Premium and level 30 with level 100 recommended. Alkestios starts on `mission` in Global and `help` in the imported selector. |
| Troubled Animals | The chest claim binds unique 6100 and grants one item 25235. The imported discovery movements advance the wolf stage. Global also requires the 5,000 gold book exchange, table use, talisman, notes from the pile of bones, 1,000 gold fur purchase, stone use, and five separate feather pickups after Tereban. Global reports the laid fur with `whelp`, while the selector handles `mission`. Chief Grarkharok is optional. |
| Feyrist access | The Alkestios dialogue writes the access stage. Global then uses one corresponding small elemental gem on a shrine. The captured quest callback file has only Kroazur's exit and three wolf-discovery movements. |
| Fairy Treasure | Global uses three presents and three milk teeth. Optional teeth pay one white pearl per species and award Toothfairy Assistant after all six plus the main errand. Crystal bundles six teeth and six pearls. Global permits asking Tired Tree for `map` and telling The Seeds of Life without a book read. Five rake targets, the fourth fragment, map assembly and mosaic treasure are required. The mosaic gives one Butterfly Ring and five each of Rainbow Quartz and Ancient Coin (Crystal IDs 25698, 25737, 24390). |
| A Dream Come True | Global requires any melee weapon and two of each enchanted gem. The selector checks the combined kill threshold of 200 and Kroazur but leaves successful turn-in unbound. Global describes free lair access and a two-hour cooldown. Main has the exit callback. Global fairies return after one minute compared with Crystal's 30 seconds. Global grants a Blossom Bag after 15 repairs and equipment versions after recharging the emptied vessels. |
| An Unlikely Couple | Main writes Aurita's offer and Dancing Fairy's `spell` transition. Global also accepts `fishtail`, uses panpipes for the notes and gets a Raven Herb from a fern at night. Main has a separate qready1 Raven Herb clock callback. |
| Swan Feather Cloak | Global gates this on A Dream Come True, daytime, eight feather spots yielding five each per 20 hours, and a 20-hour craft after delivery of 100. Crystal uses a couple-stage gate, coordinates with shared cooldowns, and an immediate exchange. Main imports the offer only. |
| Sweet Dreams and Cherry | Global adds Candia access, cross-quest boss access, invisibility, the 1,000 gold illusion, A Friend in Need, the recipe with three ordered glazes, lipstick, four boss sweets, Sugar Daddy, three repairs, five honey captures, Carnival trade, three toothbrush deliveries and a Pegasus Feather. Cherry is required for overall completion and pays no reward. These stages are outside the supplied Crystal inventory and have no identified quest-owned bindings on main. |
| Other rewards | The Global pages specify no quest XP or outfit grants. The Swan Feather Cloak is an item. |

BLOCKER Kroazur entry room lifecycle with per-character boss cooldown and timed ejection data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua:3

OPEN-FACTS Exact Global runtime item IDs, numeric unique/action IDs, Raven Herb cooldown, and Candia donor binding lines are absent from the fetched pages. Global's level requirement is explicit, but the pages do not identify which interactions enforce it. The final barrier minimap and link give conflicting mountain sculpture coordinates.

PARTIAL Rewards, entry gates beyond dialogue, world item steps, kill turn-in, cloak and Candia remain under audit.
