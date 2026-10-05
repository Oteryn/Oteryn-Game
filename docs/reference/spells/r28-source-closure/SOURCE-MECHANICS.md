# Offline source mechanics evidence

This capture retains immutable source evidence from locally available pinned Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` and Crystal `00ce02a57ca5a12e48f32a3476e37471167e4c3f`. It does not execute Lua, query the world, allocate Game identities or activate mechanics.

Population is every tracked Lua file under a spells/runes directory, every whole-tree file with a lexical `Spell(...)` constructor (including unreferenced quest spells), registered monster scripts referenced by the generated slot inventory, and locally available quoted `.lua` helper dependencies. Files without an executed registration remain source evidence, not missing player spells. Quoted dependency closure is conservative; dynamic/module-name resolution is not inferred. Unknown calls remain explicit references.

`source-mechanics-inventory.json.gz` is deterministic gzip of schema-validated JSON. Every file retains exact revision, Git blob, byte count and SHA256; calls retain start/end lines and source order. Known calls capture literals, symbols, ordered source expression tokens without length truncation, nested area/table expressions, parameters, formulas, condition damage schedules, callbacks and scheduling arguments. Opaque control/body expressions retain a hash, complete symbol dependencies and numeric terms. Anonymous bodies stay opaque; ordinary long source expressions preserve every ordered token. Anonymous function bodies are never dumped; their nested call references and anonymous declaration identities are captured separately.

Spell registrar method names come from the existing `spell-source-registrar.schema.json`. Argument count distinguishes getter/setter candidates. Each `Spell` constructor has its own file-local ordinal, including variable reuse. Method receiver constructor references are **textual preceding assignments**, not proven Lua object identity or lexical scope. Function context similarly names the preceding declaration only. This deliberately avoids asserting relationships the lexical extractor cannot prove.

Chained/indexed calls such as `creature:getPosition():sendMagicEffect(...)` and `combats[i]:execute(...)` are retained with receiver expressions. Return/assignment expression evidence captures numeric formula terms but marks each row as a line fragment, not a complete parsed Lua statement. Symbolic arithmetic is not evaluated. Branch conditions remain evidence, not runtime predicates.

`source-mechanics-coverage.json` supplies counts, per-file known/unknown capture, payload/artifact digests and population conservation. `source-mechanics-evidence.schema.json` rejects unknown structural fields. This source capture complements the authoring converter; it does not replace source semantics, native implementations, source precedence decisions or client presentation qualification.

Reproduce from the repository root:

```sh
python tools/content-schema/spell-authoring/source_mechanics_inventory.py \
  --source-root /workspace/spell-sources \
  --monster-slots /workspace/spell-source-closure/generated-monsters-r27-final/monster-spell-slots.json \
  --out docs/reference/spells/r28-source-closure/source-mechanics-inventory.json.gz
python -m unittest discover -s tools/content-schema/spell-authoring -p test_source_mechanics_inventory.py
```
