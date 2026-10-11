# REGEN-FOOD-1 reference packet (2026-10-10)

Evidence for player health/mana regeneration while fed. Reference evidence only; the executable
values are owned by `apps/game-server/src/spell/food_regeneration.rs`.

## Rules used

- PROVEN (TibiaWiki `Regeneration`): players regenerate Mana and Hitpoints only while fed; any
  food adds its own length of regeneration time; the longest fed time is 1200 s; the food kind does
  not change the rate; promotion regenerates faster.
- PROVEN (`docs/reference/tibia-manual` characters/combat/houses): no regeneration in a protection
  zone while hunger still counts down.
- PROVEN (repo `ITEM-USE-0` §6.1, `FOOD_REGENERATION_CAP_MS`): cap 1,200,000 ms; a food that would
  reach the cap is refused (`Full`), nothing is burned.
- PROVEN (owner decision D5b in
  `tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json`,
  "Canary 15.30 decides"): per vocation, health +1 every `gainhpticks`, mana +2 every
  `gainmanaticks`:

| Vocation | Health +1 / | Mana +2 / |
|---|---|---|
| Sorcerer, Druid | 12 s | 3 s |
| Master Sorcerer, Elder Druid | 12 s | 2 s |
| Knight, Monk | 6 s | 6 s |
| Elite Knight, Exalted Monk | 4 s | 6 s |
| Paladin | 8 s | 4 s |
| Royal Paladin | 6 s | 3 s |

- CROSS-CHECK: TibiaWiki `Calculator/Mana Per Second` gives 2/6 (Knight), 2/4 (Paladin),
  2/3 (Royal Paladin, Druid, Sorcerer), 2/2 (Master Sorcerer, Elder Druid), 2/4 (Monk),
  2/3 (Exalted Monk). Exalted Monk mana differs from the owner decision (6 s vs 3 s): the owner
  decision governs.
- UNKNOWN/not implemented: Resting-area / daily-reward double rate, level dependence (none in
  these rates), offline (bed) regeneration.

## Slice

`ConditionValues::FoodRegeneration` (1 s tick) is applied by `eat_food`; each unsuppressed tick in
`actor_conditions::tick_inner` credits fed time toward the vocation periods. Not in this slice: the
item-use ingress that consumes a food item and calls `eat_food` (needs ITEM-USE-1 inventory
consumption and the durability owner), persistence of the accumulator (runtime only, resets on a
new actor), the `Recovery` condition.
