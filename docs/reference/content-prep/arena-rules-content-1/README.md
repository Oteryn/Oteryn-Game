# ARENA-RULES-CONTENT-1: populated reference data

Staged reference data: target `content/` files are in [admission-evidence/](admission-evidence/), with target paths and SHA-256 values in [content-apply-plan.json](content-apply-plan.json). `content/` is unchanged until the coordinator applies the plan.

All 1 previously held parameter fields now contain explicit Oteryn working values in their target records. They are classified DERIVED and scoped OTERYN_REFERENCE_PARAMETER; source-backed facts retain their existing evidence classification. No runtime activation is authorized.

The [working parameter register](oteryn-parameters.json) records completion and shared policies. The [decision rationale](oteryn-proposals.json) retains every field path, source and assumption, and is marked materialized. Original unresolved Tibia observations are retained verbatim in the [Global source archive](tibia-source-archive.json), rather than being passed off as official facts.

Active evidence counts: {"DERIVED": 1, "PROVEN": 58}. Remaining unfilled active parameter fields:0. [Field audit](research-audit.json); [source registry](sources.json).

The owner's instruction is to populate the data using explicit project assumptions. These are working reference defaults for native implementation; accepted architecture, concrete world/native bindings and gameplay activation remain coordinator responsibilities. OTS observations are pinned and labelled OTS_HYPOTHESIS_ONLY. Project choices do not establish current Global behavior.

Payloads are staged under admission-evidence/ with an apply plan because content-tree seals, generated slots and World Bundle inputs cannot be changed in this task. The staged record values themselves are populated.

Validation and the frozen remote candidate are recorded in the draft PR. No Rust, Cargo, migrations, protocol, CI, apps, generated slots, merge or ready transition.
