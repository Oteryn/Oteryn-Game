# M20 behavior-source decision

Owner decision 2026-11-09: Crystal 201189f871d23597b82caf88fd611454a2ddefb8
has priority; where absent, current Tibia, TibiaWiki and repository evidence
are authorized. Unknown values may be authored per owner rule. Missing donor
scripts are consequently no longer an implementation blocker.

## Network attempt

The MediaWiki parse API was requested with an honest user agent
`Oteryn-Game content compatibility research (M20; automated agent)` for:

- The Cradle of Monsters Quest/Spoiler — HTTP 402; revid unavailable.
- The Scatterbrained Sorcerer Quest/Spoiler — HTTP 402; revid unavailable.
- The Cursed Crystal Quest/Spoiler — HTTP 402; revid unavailable.

No page content or revision was obtained; no revision is invented.

## Retained item evidence

Source: `imports/tibiawiki/facts/items-stats.json`, independently indexed in
`imports/tibiawiki/source-evidence/stack-default-successor8/global-own-itemid-index.json`.
These are item identity observations, not evidence for recipe quantities.

| Page title | revid | Item ID |
| --- | --- | --- |
| Bottled Yolk | 1116259 | 40598 |
| Empty Flask | 1116260 | 40599 |
| Nutrient Solution | 1116261 | 40601 |
| Flask with Anti-Mutagen | 1011469 | 40603 |

The existing report's donor-source absence remains accurate but no longer
requires another owner decision. Runtime route implementation and real-caller
PostgreSQL qualification remain unfinished. Any recipe values selected beyond
retained evidence must be explicitly recorded as **authored per owner rule**.
