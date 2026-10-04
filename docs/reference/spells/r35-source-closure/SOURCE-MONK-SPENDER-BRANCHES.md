# Current Canary monk spender source programs

Three pinned source registrations are represented by real typed, schema-validated data: Devastating Knockout (power 62), Greater Tiger Clash (44), and Tiger Clash (15), Canary revision `04b83b512114bfd888000d6e1433ed8ecaec7c5b`. This is an additive source-only package; sealed r28–r34 and the current authoring cohort are unchanged.

Each callback retains `power * (skill / 100) * (attack / 10) + calculateFlatDamageHealing()`, the source subtraction/addition of one tenth, and its two-value `getHarmonyDamage(min,max)` return. Tiger Clash retains the actual conditional lower clamps 5 and 10. The `factor` callback argument is explicitly unused. Branches are neither evaluated using fake player values nor flattened into old formula substitutes.

The existing arithmetic expression AST is reused. The flat damage helper retains its current C++ recurrence, changing thresholds/factors and uint16 clamp. Harmony retains zero-charge and nonpositive-bonus exits, base 8, Harmony virtue/Serene bonus 4 or 8, Ascetic stage, raw Harmony buff minus 100, and `2^(harmony-1)` scaling. Every helper has exact file SHA, function span and body hash. Source helper shape guards reject unexpected changes instead of inventing behavior.

The existing native Formula schema can encode basic arithmetic and clamps, but not the full current helper recurrence/state and conversion boundaries. The additive strict `source-monk-spender-branches.schema.json` therefore represents bounded source statements and branches. It is not a general Lua/C++ AST or an executable Formula adapter.

The Lua binding reads both callback bounds as uint16, while the C++ helper returns a uint64 pair before pushing two Lua numbers on the valid-player success path. Missing player userdata reports `LUA_ERROR_PLAYER_NOT_FOUND` and returns count 1; that branch's stack value is explicitly unqualified and is never described as a pair. These boundaries are recorded without claiming qualified edge-case casting parity. Native helper/program consumers, live Harmony input ownership and numeric conversion parity remain explicit runtime gaps. No sign inversion, world-curve replacement, identity minting, source callback execution or activation was performed.

Artifacts: `source-monk-spender-branches.json.gz` and its qualification JSON. Gzip is deterministic, records conserve all three blocked registrations, and six tests cover clamp preservation, powers, bounded helper branches, changed-source refusal and strict evidence structure.

Reproduce offline:

```sh
python tools/content-schema/spell-authoring/source_monk_spender_branches.py \
  --out docs/reference/spells/r35-source-closure/source-monk-spender-branches.json.gz
python -m unittest discover -s tools/content-schema/spell-authoring -p test_source_monk_spender_branches.py
```
