# NEWS-HIST-REGEN-1

## Parity

| Evidence | Unfed regeneration | Fed (saturated) regeneration | Amounts and modifiers |
|---|---|---|---|
| Canary `47dfd51f` (verified correction) | No always-on clock in the food recipe | Vocation-specific legacy periods | HP +1, MP +2 per tick in `data/XML/vocations.xml`; PZ/daily-reward handling in `ConditionRegeneration` |
| tibia.com news 8541/8543 | Always on; HP 4 s, MP 6 s | HP 3 s, MP 5 s | Amount depends on vocation; no numeric amounts specified |
| Oteryn before | Off | Vocation-specific legacy periods | Existing vocation amounts; existing resting/equipment effects |
| Oteryn after | Always on; HP 4 s, MP 6 s | HP 3 s, MP 5 s | Existing vocation amounts; existing resting/equipment effects |

## Implementation

The existing player-condition periodic dispatch owns the one-second clock and selects its cadence
from the existing food condition. No second regeneration owner, durable clock, or migration is
introduced.

## Source correction during local validation

The new fixed intervals come from tibia.com news 8543, not the pinned Canary implementation.
At `47dfd51f45280a59a1d3e50ba7edd573d7234446`, `data/libs/functions/player.lua`
(`Player.feed`) installs a finite food condition using vocation periods. `src/creatures/players/player.cpp`
(`Player::updateRegeneration`) and `src/creatures/combat/condition.cpp` retain those periods.
This task preserves Oteryn's existing +1 HP/+2 MP amounts, as requested, rather than claiming
full parity with the vocation-dependent amounts announced by Tibia. Resting/daily-reward and
equipment behavior are not newly implemented here.
