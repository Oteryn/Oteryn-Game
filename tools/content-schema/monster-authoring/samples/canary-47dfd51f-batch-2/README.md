# Canary second test batch — uncommon mechanics

Source: `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Classification: **OTS_HYPOTHESIS_ONLY**. All keys and assets remain provisional
source references; nothing here is admitted game content or runtime qualification.

| Monster | Additional coverage |
|---|---|
| Hydra | Paralysis and haste with exact speed modifier formulas |
| Warlock | Invisibility, haste, addon binding; registered skill reducer remains unresolved |
| Dworc Voodoomaster | Drunkenness and invisibility |
| The Halloween Hare | Creature and Item outfit transformations, several external appearance dependencies |
| Wyrm | Presentation-only effect; registered wave remains unresolved |
| Zushuka | Item outfit transformation and Nemesis Bosstiary |
| Black Knight | Nemesis Bosstiary and addon binding; death event remains unresolved |
| The Enraged Thorn Knight | Archfoe Bosstiary, unique loot and healing from damage; two events remain unresolved |
| Sand Vortex | Item appearance, damage reflection, drunkenness and haste |
| Paladin Familiar | Familiar flag/profile, base appearance and configuration-dependent duration |

## Findings

- All 10 bundles pass structure, semantic and declared-reference validation.
- Five manifests have every declared row resolved: Hydra, Dworc Voodoomaster,
  The Halloween Hare, Sand Vortex and Zushuka. Five deliberately fail readiness;
  see `validation-report.json` for the exact six unresolved source fields.
- Quest decision D6 is not a blanket script omission rule. The three inspected
  first-batch quest/task events retain their prior disposition. New events and
  registered spells require qualification instead of being silently dropped.
- `IOBosstiary::levelInfos` awards **additional** points at each unlock:
  Bane `(25,5), (100,15), (300,30)`; Archfoe `(5,10), (20,30), (60,60)`;
  Nemesis `(1,10), (3,30), (5,60)`. `addBosstiaryKill` adds the new tier's award.
  `points_per_unlock` preserves all awards; `boss_points` is their completed total
  (50 for Bane, 100 for Archfoe/Nemesis). Existing scalar-only candidate records remain valid.
- Familiar duration is `60 * familiarTime / 2` seconds. With pinned
  `config.lua.dist` `familiarTime=30`, the source baseline is **900000 ms**.
  The Paladin spell costs 2000 mana. Its declared external Ability is not imported Lua.
  Base appearance 992 comes from `FAMILIAR_ID`; `familiars.xml` also offers other skins.
  Actual selection is player state. `max(owner speed - base speed, 0)` cannot be
  represented by a fixed `owner_speed_bonus`, so that rule remains unresolved.
- The literal source census covers **1656** Lua paths under the pinned Global monster
  directory. It finds no `child =` or `combat = true` assignment. This proves only
  the bounded textual search, not arbitrary Lua or engine behavior absence.
  Nested container loot and damage immunity remain synthetic validation coverage.
- Source blob identities come from the pinned Git tree, preserving provenance even
  on a Windows checkout with CRLF conversion. Every source checkout must have the
  exact pinned HEAD and unchanged tracked files.

## Reproduction

```text
python canary_batch.py --canary <clean pinned checkout> --batch second
python verify_canary_batch.py
python verify_sample_batches.py
```

`sources.json` records every consumed helper's Git blob and the literal path/blob
census digest. No Wiki or Global claim is made for this second batch.
