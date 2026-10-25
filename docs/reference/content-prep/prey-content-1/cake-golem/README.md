# Cake Golem: recovered source and authoring bundle

The historical importer failed because CakeQuest.Items was not loaded. The table exists in the same CrystalServer revision. Isolated evaluation with that exact table reaches registration and preserves all21 declared monster fields. The product importer and historical deferred-admission guard are unchanged.

The bundle contains the converted Creature, Behavior, Presentation, Loot, abilities/effects/formulas and corpse Item dependencies. All structural, semantic and declared-reference checks pass. **Import readiness remains blocked** by one exact source entry: attacks[2] has no combat type. Its complete raw value is retained in source-evaluation.json; it is not silently discarded or guessed in the evidence. monster.json is a partial authoring projection until that manifest entry is resolved.

The pinned engine defaults this omitted type to COMBAT_UNDEFINEDDAMAGE. Fandom describes a poisoned cake attack but the fetched text does not specify its damage element; the visual effect alone cannot establish it. Wiki BR describes a different ability/damage range. No earth/physical type is invented. See wiki-source-comparison.json.

Catalog entries declare required source-scoped references and assets; they do not prove native availability. Native identity binding, asset/item materializability, event/encounter admission, the Prey source-pool conflict, and discrepant wiki combat values remain explicit implementation or source-reconciliation requirements. This package does not admit a native creature or activate gameplay.

Crystal source: zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f; Canary engine rules: opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446. Clean pinned checkouts and exact source bytes were verified. A temporary process-local loader wrapper supplies only the exact same-revision Items table to the unmodified converter. No wiki/official value overlay was applied. All donor-derived values remain OTS_HYPOTHESIS_ONLY.

Files: monster.json, dependencies.json, catalog.json, manifest.json, source-evaluation.json, validation.json and wiki-source-comparison.json. The first three pass validate_monster.validate without an import manifest; with manifest.json it correctly reports the unresolved second attack. This distinction is intentional.
