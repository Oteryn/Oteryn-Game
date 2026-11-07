# CipSoft client weapon proficiencies (15.30)

Source observations of the 443 weapon proficiency definitions in the owner-provided
15.30 client file `proficiencies-7fea90ec...json` (`content/assets/files/`, pinned by
SHA-256 in `manifest.json`). Nothing here is an Oteryn identity or gameplay definition.

The source is JSON with the exact keys `ProficiencyId`, `Name`, `Version` and `Levels`.
Each record carries `source_id` (= `ProficiencyId`, unique), `source_index` (array
position), `name`, `version`, `levels` (each level `{Perks: [...]}` exactly as given,
keys and numbers unchanged) and `source_record_sha256` (SHA-256 of the record's
canonical compact JSON, sorted keys).

Perk keys are the source's own names. The script accepts only these eight key sets
(observed, closed): `Type,Value`; `SkillId,Type,Value`; `AugmentType,SpellId,Type,Value`;
`BestiaryId,BestiaryName,Type,Value`; `ElementId,Type,Value`; `DamageType,Type,Value`;
`Range,Type,Value`; `ElementId,MissileId,Multiplier,Probability,Type` (the last has no
`Value`). Anything else, a duplicate id or a wrong type is rejected.

Not derived: the meaning of `Type`, `SkillId`, `AugmentType`, `ElementId`, `DamageType`
and `Version` codes; they are kept as raw numbers.

Regenerate or verify with `python tools/content-census/stage_proficiencies.py [--check]`
(shared with `../weapon-proficiency-bindings/`).
