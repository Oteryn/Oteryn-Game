# Static pilot batch 001

Observed source: Tibia Test Server Client **15.33.ad0955**, 2026-11-12.
Owner target decision and programme plan: [PLAN.md](PLAN.md).
Oteryn comparison revision: `3ce2305da05228e4f690e6195b01e921fe0274bf`.

## Collected scope

Ten base catalogue entries in Cyclopedia > Items > Armors:
albino plate, amazon armor, arcane dragon robe, stag plate, stag robe,
stoic iks chestplate, stoic iks cuirass, stoic iks robe, studded armor,
and summer dress. No equipped-item inspection or gameplay experiment is claimed.

| Result | Field rows |
|---|---:|
| MATCH | 81 |
| DIFFERENCE | 1 |
| UNKNOWN | 18 |
| PENDING | 14 |
| Total | 114 |

Eighteen item screenshots and two session-context screenshots are retained in
the access-controlled archive referenced by observations.json. Each image has
its own SHA-256 and archive-member locator; the archive also has a SHA-256.
The lead inspected the UI; the comparison worker independently rechecked item
images. The lead independently checked exact source pointers, values and status
logic for all 82 MATCH/DIFFERENCE rows and all six source shard Git blob hashes.

## Confirmed definition difference

**stoic iks robe: Mantra 16 in Global; 15 in the Oteryn family definition.**

- Observation: `gts-static-stoic-iks-robe-MANTRA`.
- Entity: `oteryn:item.tibia.i50255`.
- File: `content/items/definitions/items-27500-27999.json`.
- Exact pointer: `/records/406/definition/semantics/skill_modifiers/value/modifiers/value/0/parameter`.
- Global evidence: `frame-static-stoic-robe-bottom.png` in the retained archive.

This is a directly compared static definition difference, not proof of the
effective admitted runtime value. All ten inspected definitions have
materializable=false; runtime admission/profile reconciliation remains required.
No fix is included in this research batch.

## Explicit gaps

UNKNOWN includes classification, augments not qualified against the effective
native profile, the robe description and fields not displayed for summer dress.
Summer dress does not display Armor: this is not a verified zero. Augment donor
packets may disagree with the UI, but a donor packet alone is not runtime truth;
no augment runtime defect is asserted here.

Fourteen vendor price/residence observations remain PENDING comparison with the
owning Oteryn service records. Visible NPC lists are partial, not complete vendor
inventories. Blank lists do not prove absence of vendors. Market-average values,
loot preferences and action-bar configuration were not collected as item rules.

Specific world name, character skills, equipment names and the full active-modifier
set remain unknown. Build, character name, level and vocation were read from
Info/Character; name alone was not used to infer vocation. The character panel
shows Invalid Monk, level 757, Exalted Monk. Base catalogue comparisons do not
use character modifiers. No production-release timing is verified.

## Next static work

1. Resolve the exact world/session metadata and the 14 service comparisons.
2. Trace the Mantra difference to the effective profile before planning a fix.
3. Continue item categories, then spell definitions and visible creature/boss,
   charm, imbuement and proficiency entries, using the same evidence discipline.
4. Reconcile repository-wide references to the owner's new Winter Update target
   in a separately scoped follow-up; this batch preserves historical decisions.

The pilot is recorded with partial metadata; neither the entire static catalogue
nor dynamic behaviour has been verified. PR remains a research draft.
