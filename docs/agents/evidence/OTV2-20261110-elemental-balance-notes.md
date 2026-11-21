# Elemental resistance — audited, non-destructive first slice (2026-11-10)

Source snapshot: Oteryn-Game `main@c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f`. This proposal is **not** an Item migration or permission to change canonical Tibia-derived stats.

## Delivered now

- A derived `ItemAbility::ElementalProtection` recognizes exactly four **identical, positive, direct** Fire/Ice/Earth/Energy resistances. It expands into four independent effects in `derive_equipment_effects`; field handling also expands the derived ability for its selected element. No wire enum, canonical Item schema, persisted values, damage type or content pin changes.
- A repeatable, read-only analyzer `tools/content-schema/item-authoring/elemental_balance.py` reads the 34,111 indexed Item records and the pinned equipment abilities source packet; it returns candidate decisions and before/after four-element budgets. It never writes source Items.
- The report lists 16 candidates out of 402 records with known resistance vectors. Fourteen are exact semantic aliases (same effective protection). Two are discounted **manual review candidates**; no changes have been made to them.

## Discount proposal, deliberately not applied

For an Item with 2–3 positive direct resistances and no negative elemental or field-specific protection, the proposed percent is `floor(0.9 × sum(Fire + Ice + Earth + Energy) / 4)`, with a minimum useful output of 1%. It is not applied to single-element specialist Items, vulnerabilities, fractional results, or special field-only protections. For four equal positive direct resistances, use that same percent, which is lossless.

| Item | Before (direct) | Proposed | Budget before → after | Worst new gain | Worst specialization loss |
|---|---|---|---|---|---|
| Fur armor (`i22085`) | Earth 5%, Ice 5% | Elemental 2% | 10 → 8 | +2 pp (Fire/Energy) | −3 pp (Earth/Ice) |
| Swan feather cloak (`i25779`) | Earth 12%, Energy 12% | Elemental 5% | 24 → 20 | +5 pp (Fire/Ice) | −7 pp (Earth/Energy) |

The four-element sum is an **equal exposure proxy**, not an estimate of actual player survivability. Any universal gain against Fire or Energy can be too strong for a hunt even when the total budget decreases. Review zone exposure, equipment combinations, imbuements, field damage and actual mob damage mixes before changing either canonical Item.

## Scope and follow-up

This first slice preserves all current damage reduction numbers. It provides a single derived typed bonus for existing equal-four entries and an analysis tool, **not a new authorable Item property**, and does not claim the game client already displays the word `Elemental`. New authored unified resistance would need an accepted content contract and UI projection, plus balancing on full Item profiles and native runtime content overlays. The analyzer does not model those overlays or player/creature encounter frequency.

Quick checks: `python3 tools/content-schema/item-authoring/test_elemental_balance.py`, `python3 tools/content-schema/item-authoring/elemental_balance.py`, focused Rust equipment-effects and item-abilities tests, `cargo fmt --all -- --check`.
