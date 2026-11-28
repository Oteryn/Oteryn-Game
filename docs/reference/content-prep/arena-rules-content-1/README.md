# ARENA-RULES-CONTENT-1: populated reference data

Staged reference data: target `content/` files are in [admission-evidence/](admission-evidence/), with target paths and SHA-256 values in [content-apply-plan.json](content-apply-plan.json). `content/` is unchanged until the coordinator applies the plan.

All 1 previously held parameter fields now contain explicit Oteryn working values in their target records. They are classified DERIVED and scoped OTERYN_REFERENCE_PARAMETER; source-backed facts retain their existing evidence classification. No runtime activation is authorized.

The [working parameter register](oteryn-parameters.json) records completion and shared policies. The [decision rationale](oteryn-proposals.json) retains every field path, source and assumption, and is marked materialized. Original unresolved Tibia observations are retained verbatim in the [Global source archive](tibia-source-archive.json), rather than being passed off as official facts.

Active evidence counts: {"DERIVED": 1, "PROVEN": 58}. Remaining unfilled active parameter fields:0. [Field audit](research-audit.json); [source registry](sources.json).

The owner's instruction is to populate the data using explicit project assumptions. These are working reference defaults for native implementation; accepted architecture, concrete world/native bindings and gameplay activation remain coordinator responsibilities. OTS observations are pinned and labelled OTS_HYPOTHESIS_ONLY. Project choices do not establish current Global behavior.

Payloads are staged under admission-evidence/ with an apply plan because content-tree seals, generated slots and World Bundle inputs cannot be changed in this task. The staged record values themselves are populated.

Validation and the frozen remote candidate are recorded in the draft PR. No Rust, Cargo, migrations, protocol, CI, apps, generated slots, merge or ready transition.

## W17 creature candidates

The [five Creature authoring profiles](admission-evidence/tibiadrome-creature-candidates.json) record the Fandom archive values for HP, XP, conditions, targeting, summoning and observed attacks. Speed, armor, exact resistances and executable attack clocks require source values before the profiles can be materialized. The runtime candidate also needs an exact Presentation binding and an admitted Drome room.

The pinned Crystal tree at `201189f871d23597b82caf88fd611454a2ddefb8` supplies five initial corpse Item definitions. Their positive decay links identify the start of each named Item chain. The per-creature source IDs and XML lines are recorded as **Crystal (Global silent)**. The Crystal tree contains no corresponding monster registrations establishing which corpse, if any, the encounter places on a kill. The IDs remain source observations.
