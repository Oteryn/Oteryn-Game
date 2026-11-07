# Position-based source custom mechanics candidate data

`source-custom-mechanics.json.gz` contains 21 independently source-pinned descriptors:

- 6 ground transforms: Mazoran Fire and Charge Vortex from three donor packs.
- 6 boss form swaps: Time Guardian and Time Guardiann from three packs.
- 3 fixed-position named-monster healing declarations: Heal Brain Head.
- 3 random absolute-position spawns: Generator.
- 3 area-bound name-matched wild minion counting scripts: Gaz summon, each with an exact pinned `gaz_functions.lua` closure.

Source revisions are Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` and Crystal `00ce02a57ca5a12e48f32a3476e37471167e4c3f`. Each record retains immutable path, blob, SHA256 and bytes. Both Crystal packs remain separate records even when payloads match.

The strict schema is `tools/content-schema/spell-authoring/source-custom-mechanics.schema.json`. The existing encounter schema describes native identity-bound executable rules. This package instead preserves raw donor coordinates, item/action IDs, world lookup expressions and callback declarations without allocating native identities or inventing execution semantics; it therefore uses an additive source-only schema.

Typed facts include literal coordinates, item-position restore bindings, numeric tables, loops, random arguments, name filters, source actions and callback schedules. Mazoran has 887 item-position bindings per pack, alongside original ground exclusion/filter tables and area loops. Charge Vortex retains 11 positions and a 10000ms reset. Both Time Guardian variants retain their own predicates and health expressions; differences are not flattened. Brain Head retains its fixed coordinate and 300–500 random heal. Generator retains four coordinates and its 1–4 random selection. All captures have exact source lines and refer to the ordered call evidence.

Numeric literal multiplication in delay arguments can be normalized, while the original expression remains present. Dynamic expressions remain unsupported. Candidate action lists describe source calls, not branch reachability or execution order. Function context and receiver bindings have the explicit lexical limitations defined by the source-mechanics capture. No source Lua bodies are dumped.

`source-custom-mechanics-qualification.json` records schema validation, five regression tests, population counts and exact payload/schema digests. All records have `runtime_activation: false`, `identity_allocation: false`, and source-only execution status. Native identity mapping, world-coordinate projection, custom consumers, callback/control-flow semantics and scheduler activation remain separate work.

Reproduce offline:

```sh
python tools/content-schema/spell-authoring/source_custom_mechanics.py \
  --source-root /workspace/spell-sources \
  --out docs/reference/spells/r28-source-closure/source-custom-mechanics.json.gz
python -m unittest discover -s tools/content-schema/spell-authoring -p test_source_custom_mechanics.py
```

Gaz descriptors preserve initial `GazVariables.MinionsNow = 2`, `MaxSummons = 7`, the four 25-tile spectator range arguments, exact minion name matching, fill-to-current-state loop, inclusive random range 0–100 with `< 25`, and mutable state increment. Counting uses area spectators, not caster-owned summons. The two voice/action branches, visual effect and source statement chronology remain in typed evidence. The global `sum` is undeclared in the script/helper and `setSummon` has no registration in the pinned Lua/creature C++ sources; the unavailable binding is retained explicitly, never converted into native summon ownership. Source-specific counter/world/control-flow consumers remain unimplemented.
