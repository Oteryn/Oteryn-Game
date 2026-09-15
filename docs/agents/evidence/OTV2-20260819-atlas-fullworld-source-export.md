# OTV2-20260819 Atlas full-world source export evidence

## Status

**PASS for the Game-owned full-world offline source-projection adapter used by ATLAS-FULLWORLD-LOCAL-GENERATION-FABRIC.**

## Exact revisions and inputs

- Game main revalidated base: `9e594ceb292cb2a54bf968fb057501b743443728`.
- Game generation code revision: `f79fd3b5c239fa13810338f1380539c4eac67d7d`.
- `producer.py` SHA-256: `b3fcb59a8a5df3f5e9acb25036086215a60b8d66c8a01985172707559edf1a2f`.
- Legacy importer: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- `world.otbm` SHA-256: `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`.
- `15.32.zip` SHA-256: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`.
- catalog SHA-256: `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`.
- appearance SHA-256: `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`.

## Semantic boundary

Resolved visible items delegate to the already-qualified DYN producer path. The adapter broadens spatial selection only; it does not define the final Atlas physical publication format or infer new gameplay semantics.

## Unresolved appearance finding

The first complete-world generation attempt failed closed on server ID `2141` at legacy position `(33572,32528,14)`. A scan of all **18,997,668** cached source tiles proved:

- unique visible appearance source IDs: **25,198**;
- exact pinned 15.32 object appearances: **43,514**;
- missing visible appearance IDs: exactly **one**, `2141`;
- occurrences of `2141`: exactly **one**;
- that occurrence has no subtype, AID, UID, house-door ID, teleport destination or custom attributes.

The repair preserves that visible presentation explicitly with `presentation_resolution_state=UNRESOLVED_APPEARANCE` and `resolved_primitives=[]`. It does not delete the source record or substitute/infer another appearance or sprite.

## Validation

Focused validation:

- `python3 -m py_compile tools/game-atlas-fullworld-source/producer.py tools/game-atlas-fullworld-source/self_test.py` — **PASS**.
- `python3 tools/game-atlas-fullworld-source/self_test.py` — **PASS**; resolved path delegates and unresolved path is explicit/non-substituting.
- `git diff --check` — **PASS** before evidence finalization.

Integration qualification against the exact pinned inputs:

- bounded Thais Z7 replay after the unresolved-appearance repair: **PASS byte-identical**;
- tiles: `24,311`;
- presentations/primitives: `39,282 / 39,282`;
- exact `tiles.jsonl` SHA-256: `ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984`.

Complete-world consumer result:

- floors: **16** (`-15..0`);
- tiles: **18,997,668**;
- presentation records: **24,502,036**;
- resolved primitives: **24,502,035**;
- unresolved presentations: **1** (`appearance_source_id=2141`);
- unique appearance source IDs: **25,198**;
- unique sprite source IDs: **27,394**;
- Atlas fabric root: `sha256:ef72ccea156283eea1efd103577e2933b15b38d1a67aa05c89594cc3a731ea6f`;
- independent full-shard handoff verification: **PASS**.

## Review classification

Mandatory implementing-agent full-diff self-review is required before the delivery head is frozen. A separate independent review is **NOT_REQUIRED** by the trusted-base risk triggers: this change introduces no protocol/session/admission/persistence/value/production/security authority, does not weaken governance, and does not create a new untrusted parser; the only semantic delta is fail-closed preservation of one otherwise missing static presentation with no inferred replacement.
