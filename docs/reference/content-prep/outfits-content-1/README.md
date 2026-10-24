# Monk addon source correction

Staged reference data: the 136 outfit records and their family index are in [admission-evidence/](admission-evidence/), with target paths and SHA-256 values in the [apply plan](content-apply-plan.json). `content/cosmetics/outfits/` stays `READY_UNPOPULATED` until the coordinator applies the plan.

Both addons are recorded as unavailable, awaiting a future CipSoft update. The invented100,000/200,000gp purchase recipes are withdrawn. This applies the owner’s explicit resolution, supported by TibiaQA’s April2025 answer quoting the official announcement. The historical source does not establish a future release date. See [source check](source-correction-check.json). Runtime admission remains blocked.

## Superseded materialization report

The following earlier report is retained as history; the source correction above takes precedence.

# OUTFITS-CONTENT-1: populated reference data

All 1 previously held parameter fields now contain explicit Oteryn working values in their target records. They are classified DERIVED and scoped OTERYN_REFERENCE_PARAMETER; source-backed facts retain their existing evidence classification. No runtime activation is authorized.

The [working parameter register](oteryn-parameters.json) records completion and shared policies. The [decision rationale](oteryn-proposals.json) retains every field path, source and assumption, and is marked materialized. Original unresolved Tibia observations are retained verbatim in the [Global source archive](tibia-source-archive.json), rather than being passed off as official facts.

Active evidence counts: {"DERIVED": 334, "PROVEN": 1030}. Remaining unfilled active parameter fields:0. [Field audit](research-audit.json); [source registry](sources.json).

The owner's instruction is to populate the data using explicit project assumptions. These are working reference defaults for native implementation; accepted architecture, concrete world/native bindings and gameplay activation remain coordinator responsibilities. OTS observations are pinned and labelled OTS_HYPOTHESIS_ONLY. Project choices do not establish current Global behavior.

The payloads are staged under admission-evidence/ with a content-apply-plan because content-tree seals and World Bundle inputs cannot be changed in this task. The staged record values themselves are populated.

Validation and the frozen remote candidate are recorded in the draft PR. No Rust, Cargo, migrations, protocol, CI, apps, generated slots, merge or ready transition.
