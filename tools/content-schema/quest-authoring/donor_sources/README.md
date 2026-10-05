# Pinned donor sources, local authoring

This layer retains Canary/Crystal sources before external quest research. It does
not execute Lua, grant native admission or declare quests playable. The existing
canonical Quest data and its reviewed transcription guards remain authoritative.

The shared corpus is pinned to Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`
and Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`. Crystal summer-update
`00ce02a57ca5a12e48f32a3476e37471167e4c3f` is a separate comparison variant:
it diverges from the baseline (48 commits ahead, 7 behind).

The acquisition used public GitHub over ordinary HTTPS and existing local caches.
No wiki or Remote Desktop research was used in this stage. Upstream licenses and
exact full Git trees are retained inside `samples/donor-source/corpus.tar.gz`.
Every selected occurrence has both Git blob SHA-1 and SHA-256/length witnesses.
Scope includes runtime Lua/templates, configuration, shared helpers, descriptors
and engine text. The exclusion inventory records binary/assets/docs/build scope.

From this directory's parent:

```sh
python quest_donor_source_authoring.py
python quest_donor_source_authoring.py --check
python -m unittest discover -p 'test_donor_source_*.py'
```

The offline replay verifies the corpus, NPC and reward captures, NPC declared
include routes, all structural Lua AST shards and a fresh normalized-gap triage.
It rebuilds outputs without network access. `run_checks.py` invokes this replay.
Full AST regeneration additionally requires the pinned Python packages listed in
`requirements.lock`; archive verification does not install or import the parser.

NPC spans use UTF-8 decoded character offsets; reward records also retain raw byte
offsets. AST spans can be null where upstream parser nodes lack token bounds;
full tokens, raw bytes and child ordering remain retained. These omissions are
explicit and cannot certify semantic transcription completeness.

A static include route is a source declaration under a pinned distribution profile,
not proof of deployed process CWD, package search path or module execution. Runtime
`config.lua` is generated outside Git; `config.lua.dist` is retained as a template,
not silently substituted for that live file. Alternative datapacks stay separate.

`coverage.json` deliberately keeps `source_quest_semantic_1_to_1=NOT_ESTABLISHED`
and `external_quest_research_allowed=false`. Normalized opaque statements,
conditions and blocked effects measure converter limits, not missing quests or
missing donor files. Existing wiki evidence is preserved separately.

The inventory checks all primary journal declarations with exact file/line joins,
retains alternative/demo declarations, and lists 248 unjoined script components.
Typed Source supplements now cover 64 clock/position conditions and 7 reward
choices in 3 selected graphs. Their schemas/qualifiers preserve opaque siblings,
outer guards and unknown fallback. They do not change canonical opaque nodes.

Map structural receipts and the C++ decoder are in `maps/`; its standalone
`verify.py --cache-root <Source-cache>` performs full Source-backed qualification.
The ordinary portable replay uses STRUCTURAL_ONLY and says cache NOT_VERIFIED.
Raw map assets/indexes remain external to product assets. See the map README for
relative cache relocation and explicit blob/index-directory options.
