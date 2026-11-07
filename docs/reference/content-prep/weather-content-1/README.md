# WEATHER-CONTENT-1: populated reference data

All 5 previously held parameter fields now contain explicit Oteryn working values in their target records. They are classified DERIVED and scoped OTERYN_REFERENCE_PARAMETER; source-backed facts retain their existing evidence classification. No runtime activation is authorized.

The [working parameter register](oteryn-parameters.json) records completion and shared policies. The [decision rationale](oteryn-proposals.json) retains every field path, source and assumption, and is marked materialized. Original unresolved Tibia observations are retained verbatim in the [Global source archive](tibia-source-archive.json), rather than being passed off as official facts.

Active evidence counts: {"DERIVED": 8}. Remaining unfilled active parameter fields:0. [Field audit](research-audit.json); [source registry](sources.json).

The owner's instruction is to populate the data using explicit project assumptions. These are working reference defaults for native implementation; accepted architecture, concrete world/native bindings and gameplay activation remain coordinator responsibilities. OTS observations are pinned and labelled OTS_HYPOTHESIS_ONLY. Project choices do not establish current Global behavior.

This packet stages no `content/` or `rulesets/` payload: the target records are the reference documents here and in [weather presets](../../weather-presets.md).

Validation and the frozen remote candidate are recorded in the draft PR. No Rust, Cargo, migrations, protocol, CI, apps, generated slots, merge or ready transition.
