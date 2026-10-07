# Item source recovery and full source batches — 2026-10-05

This package preserves local item work and adds two complete source-observation batches. It is quarantined review evidence. It does not change active item definitions, server schemas, client projection, wire profiles or runtime behavior. Native import remains unfinished.

## Existing repository data was checked first

The checked main was `98ca4aeaf7d579af64d43269e9b3b9bd65d49915`, refreshed to `0ec917e6b3031974233e9f0119715aaef1a66a05` before publication. A final refresh to `f8c5e621c15cb3909ed6158c02feebf0b9689644` also left those paths unchanged. The item definitions, reference document and owning Item model/artifact files did not change between those heads.

Main already has 34,033 Item definitions, including item 40450; 127 Known charge entries were identical to the local entries and were not reimported. It also has 118 spell item profiles and the accepted consumption schema. Existing local ability/modifier observations were retained rather than recreated. Details and exact hashes are in `audit/`.

## Recovered local work

`recovery-manifest.json` inventories 317 files, including all 149 uncommitted working files and the latest committed Source batch and Source-specific support files. Each member has both compressed and original size/SHA-256. Original paths are logical recovery destinations, not active repository paths. Verification decompresses one bounded chunk at a time and writes no project files.

The archives include original data, model/schema work, migration gates, metadata and evidence. They preserve the original local bytes, including the 162 MB reference document, while each published gzip member stays below GitHub's individual-file limit. This is an exact recovery snapshot, not a claim that the archived code is ready to apply to main. In particular, the archived experimental V6 media group 18 conflicts with main's accepted consumption group 18.

## New source batches

| Batch | Item targets | Own source observations | Status |
|---|---:|---:|---|
| Charges and level doors | 33,975 | 45,721 | Prospective own-parser observations; zero held rows |
| Ability configuration declarations | 33,975 | 45,721 | Full literal census; numeric/final runtime results remain Unknown |
| Definition registry successor | 33,975 | 55,012 | Includes the previous 9,291 observations; exact inverse preserves the previous registry |

The definition successor measured at most 8 observations per target and 5 ordered assignments per observation. Ability rows measured at most 17 events, 18 direct assignments, 209 UTF-8 bytes per value lexeme and 15,839 serialized bytes per row. Only the measured array bounds were added to the supplied declaration schema.

`source-batches/` contains compressed data, schemas, constructor catalogs and receipts. `producer-evidence/` contains capture-time producer/catalog snapshots for review; these retain their capture-time filesystem paths and are not portable entry points. The portable verifier is the command below.

The Item cohort excludes protected item 901 and unbound identities. Its 33,975 targets are not the number of fully classified/completed items. Sources can have different values; source cuts and own identifiers are retained. No new Native ability members, instance charges, door access rules, sound/effect playback or runtime capability was inferred.

## Sources and method

Sources were the previously captured, hash-pinned repository inputs from:

- [Canary](https://github.com/opentibiabr/canary/tree/47dfd51f45280a59a1d3e50ba7edd573d7234446).
- [Crystal](https://github.com/zimbadev/crystalserver/tree/ff7ede593c69d4c658b382c97443e8155926924a).
- [Crystal comparison cut](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f).

Current Game repository state was read through normal Git/HTTP. Full source XML/prototype captures were read locally with their pinned digests; no Remote Desktop browsing or external wiki enrichment was used. Actual C++/Lua execution, stored ability allocation, final sparse ability members and runtime getters remain unverified/Unknown. Charges/leveldoor replay is conditional on normal return of the captured C++ loading phase; it is not an instance/runtime assertion.

## Validation and remaining work

From the repository root, with the existing `jsonschema` dependency installed:

```sh
python tools/content-migration/verify_solo_item_source_recovery.py
```

The source producer suites passed 10 charge/leveldoor and 3 ability tests. Recovery boundary tests reject path escapes, symlinks, truncation and decompression beyond the declared size. The full package verifier checks every archived original digest and both complete new source cohorts.

Identity-keyed reconciliation of local data against main produced 34,033 Items with zero held conflicts and preserved 213,745 local Known atoms. The reconciled reference was 86,767,665 bytes, below GitHub's individual-file limit. That candidate remains local and unadmitted; its receipts are included as evidence, not as a passed Native reader/build result.

Full server checking was blocked locally by disk exhaustion and then a cgroup memory SIGKILL. Targeted ad hoc Rust builds also failed dependency resolution after cache changes. None of those runs is reported as passed. The PR therefore remains a draft.

Before active import: reconcile and qualify the model against current main, preserve consumption's accepted V6 slot, install measured registry/schema/target-validation successors, run fresh Native readback and whole-parent/inverse checks on both views, update owning metadata and pass the affected server/content gates. Finish source coverage and schema parity before external wiki enrichment. Classification and runtime import are still incomplete.
