# Source data closure, local r28

This batch imports existing immutable Canary/Crystal source data before any external comparison. Donor populations remain separate. Source-only facts, candidate authoring bundles and active native gameplay are distinct stages. Runtime activation, canonical source switching and node wiring remain outside this data-only batch.

The verified local reference import is materialized at `imports/spells/r28/`, with 1883 artifact copies, 14 exact schema snapshots, a manifest and complete checksums. Its manifest SHA256 is `1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f`. All source copies, schema URI bindings and import membership were checked; active content manifests and catalogs remain byte-identical.

## Verified population and fields

- Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`: 839 named source registrations.
- Crystal `00ce02a57ca5a12e48f32a3476e37471167e4c3f`: 1507 named source registrations.
- Every variant has captured declaration facts or an explicit error. Four disabled symbolic examples remain disabled. Soulsnatcher variable reuse no longer generates duplicate registrations.
- 5237 monster profiles preserve 106582 decoded declarative fields, with zero unrecovered declared assignments. Source enum references and literal strings remain distinct.
- 20742 source attack/defence slots: 20552 mapped, 175 unresolved, 15 explicit approved omissions. These are source entries, not unique playable spell counts.
- All 979 player registration headers from four retained source snapshots validate against the extended candidate projection schema, with zero unrepresented header fields. This includes historical, removed and disabled variants.

The complete monster regeneration passed 136137 checks against exact Git objects, converter/schema fingerprints, source populations, bundle schemas, dependency provenance and mapped Ability/Effect references. Source profile evidence separately checks 173220 destination references with no broken pointers.

`source-mechanics-inventory.json.gz` retains evidence from 2346 selected source/helper files and 50318 calls. Known registrar/Combat/Condition calls, ordering, chained/indexed receivers, callback declarations, area expressions, schedules and numeric expression terms are retained. Opaque expressions and unsupported calls remain marked; this evidence does not claim a lossless executable Lua translation.

## Actual data repairs

The importer captures all registration variants rather than recording mechanics only for one selected file. Reference capture retains custom Combat declarations and ordered Spell/Combat calls, including explicit nil positions, false and zero. It reads bounded dependencies from verified pinned Git bytes. The 35 previously failing declaration captures now retain their source facts; their custom casts remain unexecuted.

Candidate schemas now preserve vocation `none`, vocation display flags, Crystal Harmony cost, Canary elemental cast type/stance slot, source rune item IDs, familiar cooldown zero and unbound source sound references. Missing cue bindings are not invented. Disabled symbolic carriers have a separate source projection field.

`player-source-bundles/` contains separate current-source records for all 483 player registrations: 297 structurally valid candidates and 186 explicit blocked records. These comprise 185 Ability executions, 94 Conjure executions and 18 source-qualified native descriptors. The native descriptors cover Food, Destroy Field, Chameleon, Creature Illusion, Crystal Levitate/Magic Rope and eight source variants of Aleta. Source-specific Item references preserve donor IDs and require the matching source revision. The package's hashed `import-summary.json` is authoritative.

Full candidates include source-matched Spell/Ability/Effect/Formula data where supported. A blocked record retains its header, callback facts and specific blockers, without placeholder executable effects. Rebuilding the package removes stale executable files when a candidate becomes blocked. External/wiki overrides and changed helper closures are excluded from source-qualified native reuse.

`source-formula-evidence.jsonl.gz` preserves a stable cohort of 87 source registrations: 54 normalized formula trees, 27 scalar-free item/condition Combats and six actual Canary C++ tier programs. The tier programs remain runtime-unimplemented. Source engine evidence prevents invented direct damage on scalar-free Combats and preserves Practise Healing's actual negative health branch instead of substituting an external correction. Inflict Wound preserves its condition, zero direct-health path and source selector, with a creature-variant routing gap recorded separately.

`source-custom-mechanics.json.gz` contains 21 typed source descriptors for ground transformation, boss form swaps, fixed-position named-monster healing, random absolute spawning and Gaz's area minion counting. Coordinates, item IDs, actions, timers and helper proofs remain donor-scoped. Gaz's undefined `sum` and unregistered `setSummon` are explicit unsupported bindings.

`player-source-guards.jsonl.gz` conserves 483 registrations from 481 source files, with 642 lexical conditions and 15735 source events. This is whole-file evidence with unresolved callback/control-flow binding, not executable guard qualification. Long ordinary mechanics expressions and symbol dependencies are retained without truncation; anonymous function bodies remain opaque references.

293 new provenance rows point at unchanged source healing/reflection values. Demon's source sound schedule is preserved in `audio.source_sound_schedule`, marked `source_only_unbound`. A failed legacy behavior probe now blocks its spell slot instead of losing the whole monster profile.

## Remaining semantic work

The 175 unresolved monster slots require custom behavior or a decision about invalid source values. Seven donor creature structures remain invalid and one Action script under Crystal's monster directory remains explicitly unevaluated. The source profile sidecar preserves their data independently of admission.

Captured source data and structurally valid candidates do not prove gameplay equivalence. Callback/world behavior, native admission, missing effect consumers and renderer/audio delivery still require their owning implementation. This batch does not overwrite the selected r25 native catalog or select a new server manifest.

## Reproduction and validation

Use the existing `/workspace/spell-tools/bin/python` and local pinned Git objects. Fresh output directory required:

```sh
/workspace/spell-tools/bin/python docs/reference/spells/r22-audit/monster-import-current/import_all_monster_spells.py --source-inputs /workspace/spells-r22-monster-import-current/source-inputs --out /workspace/spell-source-closure/generated-monsters-new
/workspace/spell-tools/bin/python docs/reference/spells/r22-audit/monster-import-current/verify_import.py --input /workspace/spell-source-closure/generated-monsters-new
/workspace/spell-tools/bin/python tools/content-schema/monster-authoring/package_source_import.py --input /workspace/spell-source-closure/generated-monsters-new --output-dir /workspace/spell-source-closure/package-new
```

Current regenerated tree: `/workspace/spell-source-closure/generated-monsters-r28-complete`. The package includes split donor populations and authoring bundles, retains all errors, and excludes original Lua/XML/DAT assets and redundant combined arrays. Its checksum file, receipt and internal inventory establish two identical deterministic emissions and exact member bytes. Historical r27 remains preserved.

Full monster tools: 66 tests, 59 passed and seven source-environment skips. Final full player tools: 318 passed, zero skips, with existing pinned local checkouts supplied as test source inputs. The initial default fixture paths were unavailable in this isolated worktree; no tests or hash checks were weakened to replace them. The expanded suite also caught a global Python import-path collision; the new importer now scopes and restores its dependency search path, with a fresh-process regression.

Source access in this batch: existing local Git objects and previously downloaded snapshots. No wiki, calculator, browser, Remote Desktop, source fetch, commit, push or PR was used.
