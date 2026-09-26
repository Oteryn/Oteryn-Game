# Monster schema repair v2 — 2026-09-26

This repairs the proposal retained at `a355d7d35f42367453834931b93ff3df581a2e07`.
The branch predecessor for this repair is `1a165960ba0fcf19427067cef835ae8bd3bdbc8c`;
its two advisory review files are preserved. The owner requested implementation of
the repeat audit findings. Authoring resumed explicitly for this successor candidate.

## Corrections and source evidence

| Finding | v2 representation / disposition |
|---|---|
| Damage condition min/max were called damage per tick | `condition.damage_over_time.total_damage_range` is the nominal total budget range. Fixed totalDamage maps to equal endpoints. |
| Scorpion condition had no authored duration | `lifetime=damage_schedule` derives lifetime from the qualified schedule and forbids invented `duration_ms`; fixed-duration states require it. |
| Initial tick and timing were ambiguous | Typed automatic/fixed initial tick, decreasing tick profile and delayed first tick. Automatic is source startDamage=0. No runtime algorithm is implemented here. |
| Pure visual source ability could not be represented | `Effect.operation=presentation_only` requires a visual binding and forbids gameplay payloads. An inert `strength` branch is not invented as a buff. |
| Mixed length/radius lost facing semantics | `Ability.needs_direction` is independent of final area shape. The bounded decoded-field helper retains length-derived direction after positive radius replaces area. |
| Underground flag looked like general permission | `ignore_period_underground` means bypassing period restriction underground. False still permits matching-period underground spawns. |
| Existing Oteryn pass_through was missing | Explicit boolean `Behavior.movement.pass_through`, separate from walk/push flags. Its native field presence does not prove execution parity. |
| Two mana costs invented independent source settings | One shared `summoning.mana_cost` applies if summonable or convinceable; Familiar retains its own profile cost. |
| Bestiary source fields were reduced to metadata/labels | Optional integer `stars` (observed profile 0..5) and original `locations` text; taxonomy remains the normalized race classification. |
| Real zero loot minimum / zero target distance were rejected | Both zero values are retained explicitly. Native adapter support/parity must be proved before admission; never clamp them to one. |
| Zero-HP helper entities would require relaxed monster rules | Normal monsters retain positive HP. `health_disposition` returns unresolved semantics for health=maxHealth=0; resolve as an owned Interaction/Encounter/entity kind or explicitly approved omission. |

Source mechanics: [Canary monster parser](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/monsters/monsters.cpp#L47),
[Canary conditions](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/combat/condition.cpp#L1900),
[Canary cast geometry](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/combat/spells.cpp#L378),
[Crystal loot](https://github.com/zimbadev/crystalserver/blob/be61cdd3f12d197490cc50be1020f56cb1ff26bb/data/libs/functions/monstertype.lua#L98),
[Oteryn native authoring](https://github.com/Oteryn/Oteryn-Game/blob/de7c5499a2e6ad4ec533ea895754bdd82bda292e/apps/game-server/src/content/project/v2.rs#L631).

Concrete source cases: [Scorpion](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/vermins/scorpion.lua#L84),
[Wild Water Magic](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/wild_magics/wild_water_magic.lua#L73),
[Turbulent Elemental](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/quests/soul_war/normal_monsters/turbulent_elemental.lua#L107),
[Sir Baeloc](https://github.com/zimbadev/crystalserver/blob/be61cdd3f12d197490cc50be1020f56cb1ff26bb/data-crystal/monster/quests/grave_danger/bosses/sir_baeloc.lua#L78),
[Bone Overlord](https://github.com/zimbadev/crystalserver/blob/be61cdd3f12d197490cc50be1020f56cb1ff26bb/data-global/monster/winter_update_2025/bosses/bone_overlord.lua#L46),
[Pulsating Lava](https://github.com/zimbadev/crystalserver/blob/be61cdd3f12d197490cc50be1020f56cb1ff26bb/data-global/monster/winter_update_2025/quests/pulsating_lava.lua#L10).

## Advisory review disposition

- Exact decimals: retain numeric 0–100 authoring; publish the Decimal validator and regressions for 0.0003, 0.29, 1.4 and 4.93. Arbitrary binary-float validators are not the supported precision route.
- Linked/cross-field checks: publish the executable validator and synthetic verification fixtures. Syntax-only success remains insufficient.
- Item ownership: `$defs.item` is a verification projection of canonical Item capabilities, explicitly described in schema and README. It does not replace the full native Item authority.
- Bestiary lore/race: use existing `taxonomy` and `Creature.encyclopedia.description_document -> Document`; no duplicated race enum or second copy of Notes. Locations is editorial text, not a world placement contract.
- Loot suppression: authored entry order matters. A successful **positive-quantity** drop with the flag suppresses later entries with the same exact Item reference within this table invocation. Zero quantity does not mark the Item; nested/separate tables have separate invocation scope. Source filtering and global reward/loot modifiers still require their owners.
- Provenance: the current manifest is explicitly a Git-source resolution ledger. Wiki/Fandom/BR/Tibiopedia content remains in the collection dossier until an immutable repository archive records original URL, page revision when available, capture timestamp, content digest and original text, or an accepted native non-Git provenance route is selected. Point the ledger at that real archive file/commit; never invent a Git revision for a wiki or screenshot. A partial screenshot stays `partial_text`. This repair does not claim a universal non-Git provenance importer.
- Admission: CLI reports `declared_manifest_resolved` and `source_coverage_proven=false`. A manifest can only account for its declared rows; it does not prove that no source field or script dependency was omitted. The static 242-path census is an inventory, not automatic per-monster admission or arbitrary-Lua completeness.

## Compatibility and remaining qualification

The three schema IDs advance from proposal:1 to proposal:2 because several field
names and required branches change. Old data must be explicitly reauthored from
resolved source values; there is no silent v1 migration or runtime activation.

The decreasing schedule's nominal budget need not equal the exact summed ticks.
Its native execution, area center, zero draw/distance behavior, canonical Item
admission, asset rights and arbitrary custom boss scripts remain separately gated.
JSON validation does not establish Tibia Global parity or licensing permission.
All original-language lore, inspection and voices are kept unchanged.
