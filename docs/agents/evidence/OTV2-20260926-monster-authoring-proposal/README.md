# Updated monster authoring schema

Retained on the owner's request, 2026-09-26, on `codex/monster-authoring-schema-20260926`, without integration into `main`.

| File | Purpose |
|---|---|
| [monster.schema.json](monster.schema.json) | JSON Schema 2020-12 for Creature, Behavior, Presentation and Loot. |
| [monster-target-template.proposal.json](monster-target-template.proposal.json) | Empty monster template generated from the updated schema. |
| [monster-dependencies.schema.json](monster-dependencies.schema.json) | Direct Ability, Effect, Formula, Document, Item/corpse and nested Loot dependencies. |
| [monster-dependencies-template.proposal.json](monster-dependencies-template.proposal.json) | Empty template for those dependencies. |
| [monster-import-readiness.schema.json](monster-import-readiness.schema.json) | Source-field and dependency resolution during import. |

Authoring chances use **0–100 percent**, including fractions, with precision of 0.0001 percentage point. Native Loot ppm remains an internal integration detail. Original source-language text is retained; lore is not translated or invented.

The templates deliberately contain empty placeholders and do not validate as ready monster data. Omit optional sections that do not apply; use actual typed values for a real candidate. Structural JSON Schema validation alone does not prove reference availability, all cross-field rules, asset bindings or runtime behavior.

This is the updated **authoring proposal**, not an accepted replacement for WorldProject/v2 or an implemented Game importer. Original source research used Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`, Crystal `ac447fef0935e6df52dc6b6376ae4c1534ecd73f` and Oteryn `91fb3135a8a67d012bfe048b230f3501780d244b`. Earlier local authoring checks passed 68 focused cases and accounted for 242 inventoried standard source paths; neither is runtime qualification. This saved package is limited to the updated schemas and templates. The final publication receipt binds verification to the remote commit.
