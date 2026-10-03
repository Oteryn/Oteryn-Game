# The remaining 68 titles: chosen Oteryn quest recipes

This finite batch covers exactly the titles with `binding_scope=unbound` in the
frozen `40aa7c21428e9a8248e490b0e4b7aa06af27fdf1` rollout. The baseline and selection
are preserved so another authoring pass cannot silently change the denominator.

`recipes.json` contains original paraphrases of complete, linear Oteryn journeys:
entry requirements, ordered objectives with targets and counts, terminal stages,
reward intentions, repeat policy and an explicit list of adaptations. These are
chosen content specifications. They are not certified reconstructions of official
Tibia, executable Lua, or Native effect declarations. All behavioral choices,
including reward delivery and quantities, use `CHOSEN_OTERYN_APPROXIMATION`.
Source-backed entity names remain references; they do not certify a resolved ID.

The exact wiki provider/page/revision/hash references are checked against the
existing source specifications. Donor comparisons and fresh public-browser access
receipts record their own revisions and methods. Source facts and source gaps are
preserved separately; a chosen simplification does not erase an upstream conflict
or turn an unknown official solution into a proven one. No copied wiki dialogue or
third-party source bodies are redistributed in this batch.

The producer appends a distinct `oteryn_authored_v1` profile to the existing Quest
DATA shards. It preserves all donor-derived definitions and admits no invented
Canary/Crystal storage or script bindings. `data_complete=true` means the chosen
recipe has a finite start-to-finish path. Native trigger, placement, item/cosmetic
delivery and persistent-progress bindings remain explicit holds.

```sh
python tools/content-schema/quest-authoring/authored_quest_authoring.py --check
python tools/content-schema/quest-authoring/quest_tree_authoring.py content
python tools/content-schema/quest-authoring/quest_rollout_authoring.py
python tools/content-schema/quest-authoring/run_checks.py
```

The rollout distinguishes `authored` from donor `direct` and `family` bindings.
All authored rows retain `definition_fields_ready=false`, `runtime_enabled=false`
and gameplay smoke tests `NOT_RUN`. Migration of these DATA specifications does
not establish that a player can execute them on the server.
