# Rotten Blood mechanics evidence (news 7840 / 7852)

This bounded ledger records every numeric constant admitted by `NEWS-HIST-GLOOM-1` and
`NEWS-HIST-ROTTEN-CHARGE-1`. No persistence field is required: both are encounter-room events.

| mechanic | authoritative news evidence | Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` | Crystal `201189f871d23597b82caf88fd611454a2ddefb8` |
| --- | --- | --- | --- |
| Gloom chain | News 7840 reduces the initial distance from 4 to **3 tiles**, jump distance from 7 to **6 tiles**, and base damage from 3000 to **2250**; damage growth per jump rises from 30% to **50%**. | The pinned Rotten Blood scripts contain boss levers and kill callbacks but no Gloom-chain implementation or numeric constants: **UNKNOWN**. | The pinned tree has a distinct creature ability named `pillar chain` with a 9000 ms schedule and 1100–1380 damage; it does not encode news 7840's player-threshold chain: **UNKNOWN**. |
| Gloom chain continuity | News 7840 says leaving Chagorz' lever room no longer prevents the dark-energy charge from triggering its chain at the threshold. | No corresponding condition/room script exists at the pin: **UNKNOWN**. | The pin does not encode this continuity rule: **UNKNOWN**. |
| Bakragore Rotten Charge drop | News 7852 says dropping Bakragore's Rotten Charge deals an additional **1250 agony damage** to **all players in the boss room**. | The pinned Bakragore monster and lever scripts do not implement the dropped-charge room consequence: **UNKNOWN**. | The pinned community boss mechanic applies a different percentage-based explosion after timers, not news 7852's dropped-charge addition: **UNKNOWN**. |

Implementation policy: official news supplies the admitted constants and consequence scope. The
two pinned open-source trees are negative parity evidence only; their unrelated or absent behavior
is not used to invent fallback values.
