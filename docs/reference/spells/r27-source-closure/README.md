# Local source-first spell closure

This batch preserves Canary and Crystal as separate, immutable donor populations. It reuses existing local source snapshots and Git objects; it does not apply wiki, calculator or official-site values. Source-to-authoring and execution parity remain incomplete. No server activation, node wiring, remote publication or PR is part of this batch.

## Monster source import

The regenerated import covers 5,237 profiles and 20,742 attack/defence spell slots at Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` and Crystal `00ce02a57ca5a12e48f32a3476e37471167e4c3f`. It contains 20,552 mapped slots, 175 unresolved slots and 15 explicit approved omissions. These are source entries, not unique playable spell counts.

The converter now resolves canonical damage types rather than choosing the C++ iteration aliases `COMBAT_FIRST`/`COMBAT_LAST`. This restores 67 previously unresolved slot mappings. Source spelling mistakes remain recorded; mapping does not prove that malformed Lua executes correctly in the donor engine.

Pinned literal dependency extraction recovers Cake Golem loot item 12143 and the bosstiary records of six Dream Courts bosses. SoulWar's interval 7 now uses the same pinned, digest-checked dependency extraction. Encounter callbacks remain unexecuted; Chagorz's invalid resistance keys remain explicit omissions.

The complete regenerated tree is `/workspace/spell-source-closure/generated-monsters-r27-final`. Its independent verification passed 131,345 checks: source population conservation, Git/blob hashes, dependency provenance, bundle hashes, authoring schema outcomes and actual mapped Ability/Effect references. This is structural and provenance verification, not gameplay verification. Crystal still includes seven invalid structures and one unevaluated quest Action script located under its monster directory; neither is silently dropped.

`monster-remaining-gaps.json` classifies every unresolved slot: 35 have reference-capture limitations in addition to custom behavior, 115 require custom callback/cast behavior, and 25 have invalid or missing source damage types. Improving the 35 captures must not mark their custom gameplay implemented. The six `targetfirering` entries are source/API failures, not a missing fire mapping.

`monster-source-package.tar.gz` contains the split source populations and converted authoring bundles, excluding redundant combined arrays and original upstream Lua/XML/DAT inputs. Its manifest and checksums describe the packaged files. Historical r22/r25 archives and selected runtime catalogs remain unchanged.

## Player source facts

`player-source-schema-coverage.json` audits 979 registration records across four existing source snapshots. This includes disabled examples, removed spells and practice variants; it is not a count of unique active spells. A strict source registrar schema retains explicit values without inferred defaults. Transformations compare group cooldowns, vocation eligibility, rune properties, parameter precedence, monk roles and verified sound constants against the selected authoring catalog.

The source import retains Mentor Other and all source variants independently. Authoring discrepancies remain flagged. In particular, vocation display flags, vocation `none`, Crystal harmony costs, Canary element/stance fields and unavailable sound bindings need further authoring/runtime decisions. Preserving those values in a source schema does not implement their gameplay semantics.

`player-source-registrars.jsonl.gz` contains all 979 independently keyed, schema-valid source records; `player-source-import-proof.json` binds the artifact, payload, audit and schemas by SHA-256. Ordered call histories and complete callback graphs are not established by effective registrar values. External comparison must wait for the remaining source/schema closure work.

## Reproduction

Run from the repository root, using the existing Python environment with Lupa and jsonschema. Supply a fresh, nonexistent monster output directory:

```sh
/workspace/spell-tools/bin/python docs/reference/spells/r22-audit/monster-import-current/import_all_monster_spells.py --source-inputs /workspace/spells-r22-monster-import-current/source-inputs --out /workspace/spell-source-closure/generated-monsters-new
/workspace/spell-tools/bin/python docs/reference/spells/r22-audit/monster-import-current/verify_import.py --input /workspace/spell-source-closure/generated-monsters-new
/workspace/spell-tools/bin/python docs/reference/spells/r27-source-closure/monster-source-package.py --input /workspace/spell-source-closure/generated-monsters-new
/workspace/spell-tools/bin/python tools/content-schema/spell-authoring/source_schema_coverage.py --source-root /workspace/spell-sources --out /workspace/spell-source-closure/player-coverage-new.json --source-data-out /workspace/spell-source-closure/player-source-new.jsonl.gz --source-proof-out /workspace/spell-source-closure/player-proof-new.json
```

The package producer writes artifacts alongside this README, validates conserved split populations, verifies every archive member and emits a second deterministic stream to compare archive digests. The existing source repositories must contain the pinned Git objects. No library setup or quest execution is required to read literal dependencies. Modified staged dependency bytes are rejected rather than receiving false pinned provenance.

The focused combat, partial extraction and player registrar/export suites pass 33 tests. The final monster verification and packaged-file verification provide the broader checks for this batch.
