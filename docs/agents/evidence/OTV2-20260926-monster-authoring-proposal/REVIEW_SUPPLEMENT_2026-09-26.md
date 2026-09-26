# Supplement to the monster authoring proposal review

- Date: 2026-09-26.
- Reviewed schema: `monster.schema.json` at commit `a355d7d35f42367453834931b93ff3df581a2e07`, blob `cb47bc80bf30379d12b82fb0fa50867fd1bd8e0f`.
- This note supplements [REVIEW_NOTES.md](REVIEW_NOTES.md). It is an advisory review, not an accepted schema change, implementation, or permission to merge.

## Findings and bounded changes

1. **Bestiary facts are missing (PROVEN).** `/$defs/bestiary` defines `class`, `taxonomy`, `difficulty`, `occurrence`, `kill_thresholds` and `charm_points`, but not `locations`, `notes`, `stars` or typed Bestiary `race`. The pinned Canary/Crystal Cyclops defines `Bestiary.Locations`, `Stars`, `race` and `class`; `notes` is the separately proposed Oteryn player-facing narrative field in PR #933. **RECOMMENDATION:** add these optional authoring fields and corresponding template entries; make `notes` independently authored, distinguish locations text/area links from actual World spawns, and avoid bulk-copying copyrighted prose. Keep source Creature `race = "blood"` distinct from Bestiary Giant class/race.

2. **Two required mana costs are derived from one donor fact (PROVEN).** The Cyclops definition supplies a single `manaCost = 490`. In pinned Canary, the summon spell and convince rune both read `getManaCost()`. The schema requires `summon_mana_cost` and `convince_mana_cost` when both respective flags are true, but does not link their values. **RECOMMENDATION:** represent one shared `summoning.mana_cost` for this imported behavior. Separate values may be added only as explicitly defined Oteryn overrides with a documented migration/validation rule.

3. **An unexplained loot control is mandatory (PROVEN).** Each `/$defs/lootEntry` requires `skip_later_same_item_after_success` but the proposal does not specify how equality, success, ordering or the skip scope work. **RECOMMENDATION:** make the field optional until the algorithm and source mapping are defined. Do not manufacture `false` for every imported entry as an undocumented fallback.

4. **Admission is not demonstrated by a structurally valid manifest (PROVEN).** `monster-import-readiness.schema.json` allows `unresolved_dependency` and `unresolved_semantics`; `source_index` only has a lower bound and need not reference an actual `sources` member. **RECOMMENDATION:** keep unresolved statuses for truthful audit, but calculate admission separately and fail closed for unresolved required fields, nonexistent typed references, invalid source indexes, missing source coverage or conflicting authority. A self-declared `ready` value is insufficient.

5. **Dependency Item ownership must be explicit (DERIVED risk).** The proposal has a compact `/$defs/item` within the monster schema while the repository has an Item Master Schema and existing Item identities. **RECOMMENDATION:** bind `ItemRef` to the existing canonical Item; create any new corpse/loot Item through the Item owner, or prove an explicit compatibility mapping for a bounded projection. Do not create a second authority for Item capabilities or overrides.

6. **Cross-field checks remain outside JSON Schema (PROVEN by inspection).** Structural constraints do not establish `initial_health <= max_health`, `min_count <= max_count`, ordered Bestiary stages or actual target identity of a typed reference. **RECOMMENDATION:** put these in a deterministic semantic/linking gate with positive/negative fixtures and a complete Cyclops candidate. Keep the proposal distinct from runtime qualification.

7. **Exact percentage validation needs a chosen implementation (DERIVED risk).** The schema uses JSON `number` plus `multipleOf: 0.0001`; [REVIEW_NOTES.md](REVIEW_NOTES.md) records a narrow float-versus-Decimal validator reproduction. **RECOMMENDATION:** retain readable 0–100% authoring, choose and test an exact decimal parsing route (for example decimal strings parsed exactly or a supported Decimal validator), and require checked conversion to integer 0–1,000,000 ppm. Do not infer that all validators reject numeric percentages.

## Verification and scope

This supplement is grounded in the reviewed schema, the pinned Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal `ff7ede593c69d4c658b382c97443e8155926924a` Cyclops files, and the cited Oteryn code/contract surfaces. Their Cyclops blobs match at `811c1897400e4aa539c8a97bb6ed26cef7b58d37`. Five package JSON files parse and their local `#/$defs/...` targets exist. Full-schema validation of a complete monster, execution, CI and runtime qualification were not performed for this supplement.

No changes to `content/world/**`, Item identities, Wave-1 data, relations, provenance or UNKNOWN classifications are proposed here. This file records review findings only.
