# Player spell presentation completion

This candidate preserves all 246 existing spell definitions and all mechanics,
identities, dependencies and source-selection choices. It corrects sound fields in
67 definitions: 61 lacked the engine's default cast cue; six had Crystal explicit
cues selected even though Canary defines the authoritative engine default. Both
pinned engines initialize `soundCastEffect = SPELL_OR_RUNE` (source enum ID 10)
and `soundImpactEffect = SILENCE` (ID 0). An absent registrar inherits these values.
Explicit silence, including an unregistered Lua sound constant evaluating to nil,
keeps its source-precedence vote and is omitted from audible presentation fields.

`executable-spell-catalog.json` is compiler input in the existing catalog schema;
its cast/impact fields are consumed by the existing compiler. The source-selection
file has an updated catalog digest and otherwise identical decisions. Historical
r21/r24 catalogs are unchanged. `sound-default-delta.json` lists every correction.

`source-presentation-supplement.json` covers all 246 definitions and both available
source scripts with exact revisions, SHA-256 hashes, line-numbered call sites and
source enum IDs. It finds 96 distinct script cue occurrences absent from the
existing authored aliases across 69 definitions, and direct/delayed calls in 42
definitions. These counts are NOT 96 missing success effects: they include refusal
branches, conditional effects, source alias alternatives and helper-dependent
behaviour. Source NONE/SILENCE values are marked no-op rather than missing effects.
67 definitions have no explicit primary-script sound constant; this is not silence,
because the engine defaults apply. Canary is the preferred reference when present,
not proof of which source execution a runtime owner has selected.

The supplement is reference evidence, not server-consumed activation data. Direct
calls retain expressions and unresolved trigger status; they must not be emitted
unconditionally. It does not interpret arbitrary Lua, prove dynamic helper closure,
redistribute sound files, or verify full client visual/audio playback. Numeric IDs
are original source enum values, not new native identities. Source sound membership
requires the precise `registerEnumNamespace` declaration, not an unrelated reference.

## Reproduce

Run from the repository root with the exact pinned checkouts under
`/workspace/spell-sources/{canary,crystal}`:

```sh
python tools/content-schema/spell-authoring/complete_player_sounds.py \
  --sources /workspace/spell-sources \
  --catalog tools/content-schema/spell-authoring/samples/executable-spell-catalog.json \
  --census tools/content-schema/spell-authoring/samples/spell-census-canary-99902524-crystal-ff7ede5.json \
  --out /tmp/player-catalog.json --delta /tmp/player-sound-delta.json
python tools/content-schema/spell-authoring/build_player_presentation.py \
  --sources /workspace/spell-sources --catalog /tmp/player-catalog.json \
  --out /tmp/player-source-presentation.json
python -m unittest discover -s tools/content-schema/spell-authoring -p test_player_presentation.py
python -m unittest discover -s tools/content-schema/spell-authoring -p test_spell_conversion.py
```

Eight new regression tests and 16 existing conversion tests pass. A broader local
227-test attempt initially encountered eight missing-fixture/setup errors; this is
not claimed as a passing full suite. After restoring fixtures and setting the
source-root environment, the coordinator reports all 232 tool tests passing with
zero skips/errors. Runtime cue-registry closure and full Rust manifest admission
are separate coordinator qualification receipts; the new catalog must be matched
by the qualified native-profile headers before native admission can succeed.
