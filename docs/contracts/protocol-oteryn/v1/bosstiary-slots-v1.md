# Bosstiary personal slots (WB2/P02)

Command 491 extends capability 117 `BOSS_ENCOUNTER_V1`. Identity is the admitted character;
there is no character, price, category, point total, timestamp or Premium claim in the request.

Request bytes (strict; unknown actions, invalid slots, trailing query/remove bytes refused):

- `[0]`: query.
- `[1, slot, UTF-8 boss key...]`: equip into empty slot 1 or 2; key length 1–128.
- `[2, slot]`: remove. Reroll is remove followed by equip after the action cooldown.

Reply: status u8, unlocked bitmask u8, base loot bonus u16 BE, total points u32 BE,
next removal fee u64 BE, next action Unix milliseconds u64 BE, then two length-prefixed
UTF-8 keys (u8 length each, empty means unoccupied). Maximum 284 bytes. Status 0 is query
or applied; 1 locked/ineligible; 2 occupied/duplicate/empty; 3 cooldown; 4 insufficient
money; 5 change cannot fit. A rejection writes nothing. Exact occurrence replay returns
its retained reply; conflicting semantic reuse is refused.

The first slot requires one non-boosted boss at prowess; the second requires 1500 total
boss points. Equip requires prowess for the selected canonical boss, an empty slot,
and a boss absent from the other slot and daily boosted slot. Daily boosted boss selections
are cleared from the effective personal slots. Mastery adds 25 percentage points to the
point-derived bonus. Bonus rolls affect equipment only, excluding unique drops, for each
eligible personal-reward participant and the ordinary principal. Slots at death time come
from retained operations; subsequent equip cannot retroactively grant loot.

The initial removal counter is one: first removal free, next 100000, next 400000,
then `300000 * counter - 500000`. The counter resets at the durable World reset epoch.
Successful actions have a 490 ms cooldown (configured donor 500 minus 10). A paid removal
uses the native carried-coins-then-bank fee owner, new closed audit cause 15
`BosstiarySlot`, service key `oteryn:bosstiary.slot_remove`. State, immutable operation,
fee and fee audit commit atomically under the current session-generation fence and
character root lock. Insufficient funds/change failure rolls back the complete operation.

Sources: Crystal 201189f `src/server/network/protocol/protocolgame.cpp:10615-10639`,
`src/io/io_bosstiary.cpp:225-232,311-315`, `src/game/game.cpp:11062-11094`,
`config.lua.dist:469`, `data/scripts/creaturescripts/player/login.lua:179-181`,
`data/scripts/systems/reward_chest.lua:84-111`, `src/creatures/players/player.cpp:6567-6578`,
`config.lua.dist:240-244`. Boosted boss credits three Bosstiary kills; ordinary boss one.
