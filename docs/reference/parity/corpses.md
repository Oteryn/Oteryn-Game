# Creature corpse parity

Area: creature corpse Items, the containers a creature kill mints its loot into. Date: 2026-11-16. Format: `README.md` in this directory.

A kill settles (corpse, loot, experience) only when the creature's corpse Item is materializable with a known
container capacity in 1..=30 (`crates/gameplay/content/src/content/creature_reward.rs`, `row`) and every loot Item is
materializable. Without that the kill grants nothing (`NoSettlementReason::CorpseItemInadmissible` or
`LootItemInadmissible`).

Source: OTS = Canary `imports/canary/items-xml/items.xml` at Canary 04b83b512114bfd888000d6e1433ed8ecaec7c5b,
SHA256 `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2` (the pin `encounter_corpse_items.py` already
uses). Confidence REFERENCE for every row: no client measurement yet. Decay follows the donor's full chain
(`duration` and `decayTo` per stage), not a uniform 60 s. Weight is the donor's explicit `weight`, else 0 (the
non-pickupable Game corpse policy).

## A. Wave S1: spawned Newhaven creatures (`creature_corpse_items.py`)

Creatures: corrupted ghost, corrupted skeleton, dark carnisylvan, ghost, hulking carnisylvan, hunter, Muglex clan
assassin, Muglex clan footman, mummy, poisonous carnisylvan, smuggler, stone golem, wild warrior. Their missing loot
Items are admitted in the same slice through the existing encounter loot importer.

| Item | name | capacity | duration ms | decays to | weight | source (OTS) | status |
|---|---|---|---|---|---|---|---|
| i5972 | slain skeleton | 10 | 10000 | i4024 | 0 | items.xml:16186-16190 | FIXED |
| i4024 | slain skeleton | 10 | 300000 | i4156 | 40000 | items.xml:12089-12094 | FIXED |
| i4156 | slain skeleton | 10 | 300000 | i4157 | 10000 | items.xml:12765-12770 | FIXED |
| i4157 | slain skeleton | — | 60000 | — | 10000 | items.xml:12771-12775 | FIXED |
| i5993 | remains of a ghost | 10 | 10000 | i4094 | 0 | items.xml:16309-16313 | FIXED |
| i4094 | remains of a ghost | 10 | 300000 | i4158 | 1000 | items.xml:12434-12439 | FIXED |
| i4158 | remains of a ghost | 10 | 300000 | i4159 | 1000 | items.xml:12776-12781 | FIXED |
| i4159 | remains of a ghost | — | 60000 | — | 1000 | items.xml:12782-12786 | FIXED |
| i6002 | dead goblin | 10 | 10000 | i4121 | 0 | items.xml:16361-16366 | FIXED |
| i4121 | dead goblin | 10 | 300000 | i4122 | 4000 | items.xml:12583-12589 | FIXED |
| i4122 | dead goblin | 10 | 300000 | i4123 | 4000 | items.xml:12590-12595 | FIXED |
| i4123 | dead goblin | — | 60000 | — | 4000 | items.xml:12596-12600 | FIXED |
| i6004 | remains of a mummy | 10 | 10000 | i4130 | 0 | items.xml:16373-16377 | FIXED |
| i4130 | remains of a mummy | 10 | 300000 | i4131 | 1000 | items.xml:12632-12637 | FIXED |
| i4131 | remains of a mummy | — | 300000 | i4132 | 1000 | items.xml:12638-12642 | FIXED |
| i4132 | remains of a mummy | — | 60000 | — | 1000 | items.xml:12643-12647 | FIXED |
| i6005 | split stone golem | 15 | 10000 | i4133 | 0 | items.xml:16378-16382 | FIXED |
| i4133 | split stone golem | 10 | 300000 | i4134 | 0 | items.xml:12648-12652 | FIXED |
| i4134 | split stone golem | 10 | 300000 | i4135 | 0 | items.xml:12653-12657 | FIXED |
| i4135 | split stone golem | — | 60000 | — | 0 | items.xml:12658-12661 | FIXED |
| i18138 | dead hunter | 20 | 300000 | i18139 | 0 | items.xml:37089-37094 | FIXED |
| i18139 | dead hunter | 10 | 300000 | i18140 | 0 | items.xml:37095-37100 | FIXED |
| i18140 | dead hunter | — | 60000 | — | 0 | items.xml:37101-37104 | FIXED |
| i18226 | dead smuggler | 20 | 300000 | i18227 | 0 | items.xml:37568-37573 | FIXED |
| i18227 | dead smuggler | 10 | 300000 | i18228 | 0 | items.xml:37574-37579 | FIXED |
| i18228 | dead smuggler | — | 60000 | — | 0 | items.xml:37580-37583 | FIXED |
| i18250 | dead wild warrior | 20 | 300000 | i18251 | 0 | items.xml:37700-37705 | FIXED |
| i18251 | dead wild warrior | 10 | 300000 | i18252 | 0 | items.xml:37706-37711 | FIXED |
| i18252 | dead wild warrior | — | 60000 | — | 0 | items.xml:37712-37715 | FIXED |
| i36881 | dead hulking carnisylvan | 30 | 10000 | i36880 | 0 | items.xml:68495-68500 | FIXED |
| i36880 | dead hulking carnisylvan | 30 | 300000 | i36879 | 0 | items.xml:68489-68494 | FIXED |
| i36879 | dead hulking carnisylvan | 30 | 300000 | i36878 | 0 | items.xml:68483-68488 | FIXED |
| i36878 | dead hulking carnisylvan | — | 60000 | — | 0 | items.xml:68478-68482 | FIXED |
| i36890 | dead poisonous carnisylvan | 30 | 10000 | i36889 | 0 | items.xml:68540-68545 | FIXED |
| i36889 | dead poisonous carnisylvan | 30 | 300000 | i36888 | 0 | items.xml:68534-68539 | FIXED |
| i36888 | dead poisonous carnisylvan | 30 | 300000 | i36887 | 0 | items.xml:68528-68533 | FIXED |
| i36887 | dead poisonous carnisylvan | — | 60000 | — | 0 | items.xml:68523-68527 | FIXED |
| i36892 | dead dark carnisylvan | 30 | 300000 | i36891 | 0 | items.xml:68550-68555 | FIXED |
| i36891 | dead dark carnisylvan | — | 300000 | — | 0 | items.xml:68546-68549 | FIXED |

A `—` capacity is a late decay stage that is no longer a container; only the first Item of each chain is a creature's
corpse.

## B. Not in this slice

Corpses whose donor capacity is above 30 cannot settle until the runtime cap changes (a Server decision): i18046
assassin (40), i18050 bandit (36), i18082 dark apprentice (36), i18086 dark magician (40), i18281 dark monk (50),
i18254 witch (40). The full GAP list of unadmitted corpse Items follows in a later wave.
