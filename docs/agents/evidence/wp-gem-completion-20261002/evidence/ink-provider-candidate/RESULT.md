# Ink threshold provider candidate — scratch only

Status: prospective implementation, not provider admission. CP requests 5938233351 / 5938988776 remain pending. No Game repository mutation, commit, push, Cargo, Remote Desktop, paid API or new source research was performed. All implementation/output/report files are under this scratch directory. The normal repository generator and outputs remain UNKNOWN for Ink.

## Concrete proposed paths

1. `tools/content-schema/item-authoring/item_weapon_proficiency.py`
2. `tools/content-schema/item-authoring/test_item_weapon_proficiency.py`
3. `tools/content-schema/item-authoring/samples/weapon-threshold-tibiopedia-ink-manifest.json`
4. `tools/content-schema/item-authoring/samples/weapon-threshold-tibiopedia-ink-observations.json`

`candidate.patch` applies to the current snapshot (scratch `git apply --check` PASS). 322 additions / 2 deletions, including both factual JSON samples and tests; this leaves room within the 500 handwritten-line bound for the eventual own task record. `candidate-paths-and-sha256.json` contains exact source/base/patch hashes. Original HTML is excluded from the patch and samples.

## Implemented route

The actual `build()` and normal generator CLI take the optional triple `--supplement-manifest`, `--supplement-manifest-sha256`, `--supplement-source-html`. No argument is supplied by default; a partial triple fails closed. The expected manifest SHA must be selected externally by the owning control plane, which still must admit the exact provider and source-policy scope. A caller supplying a hash, a sample, or a self-declared flag establishes no owner authority; the candidate has no admission flag.

The manifest pins factual observations, the original HTML SHA, source URL/capture/locale and existing client/threshold source pins. The generator verifies actual retained HTML bytes and package bytes; matches the item/profile to the actual staged client crosswalk, names and full level sequence; normalizes the explicit positive `(pl, Każda)` literal; and requires exactly one existing threshold table to match observed Mastery at the actual profile's n+2 level. That table must agree with the ordinary classifier. Nothing infers vocation from absence or special-cases an Ink item ID. Known Wiki/Crystal facts win. Conflicting Wiki observations remain UNKNOWN. Only a genuinely unresolved vocation/class can consume the supplement.

The source reports profession Każda, four levels and Mastery 8,000,000. It does not report each experience threshold: these remain derived from the existing owner-selected standard table. Source HTML SHA `4e4668b92aae54cf6c29ce18125e6643da6291f67a1e3ba45bd527984e721ac0`; original supplement SHA `7c4bf5e24bfc33124ef33a07853fb96ef9e2709633b4caade73edc405dc35bb3`. Minimal samples retain capture 2026-10-01T17:23:19.085311+00:00, unavailable revision/publication timestamps and the unproven target-date continuity. The unrelated community mana-on-kill 5 / client 8 conflict authorizes no stat/perk promotion. Raw primary perks remain 8.

## Qualification

`python run_candidate.py`: 19 unittest cases PASS using the real retained source. Existing 13 cases remain passing; six new cases cover external pin/bytes rejection, schema/source/URL/client/profile mismatches, missing explicit vocation, level mismatch, ambiguous/conflicting Mastery, higher-priority and disagreeing Wiki/Crystal precedence, full 443-profile/raw-perk preservation and normal CLI write/check. Mutated semantic fixtures are re-pinned so those tests reach validation rather than stopping at an expected digest difference.

`default-staging.json` is byte-identical to the repository sample, SHA `b59ba0464c9449ca52cfae8918c0b3d960eb78e96e08342179fccaccbedfa36e`. The prospective explicit-pin rebuild changes exactly binding 51666 UNKNOWN→standard and its provenance, with derived class counts 37 crossbow / 158 knight / 471 standard. Every other binding and all profile/perk/table/global fields are unchanged. Prospective staging SHA `dd907cf892851a51cab5e7ece800e0104c5c7dff5cef2b8588a969dae53de043`. `candidate-proof.json` records these comparisons and confirms copied source originals remain unchanged.

Tests are portable with synthetic digest-pinned source bytes by default. The scratch runner sets `OTERYN_THRESHOLD_SUPPLEMENT_SOURCE_HTML` to the retained actual capture and injects repository input paths read-only. It emits outputs only in scratch. No source HTML redistribution is needed for unit tests.

## Remaining concrete CP decision

Admit or reject the exact Tibiopedia provider/package and explicit-vocation/Mastery-only scope, exact four handwritten paths, derived outputs and own task record. Integration must wire the approved explicit manifest/digest/source selection into the ordinary rebuild/check invocation; the unselected default intentionally remains UNKNOWN. Requalify all downstream derived outputs under the eventual exact branch/head allocation. This patch does not authorize those writes or publication, gameplay admission, target-date parity, runtime activation or production.

## Frozen Snowball qualification

Read-only HEAD `1f2c81cdef96a3fe6d47b1fa8eec2b8817a7134d` from `/workspace/Oteryn-WP-Snowball` (PR1484). `qualify_snowball.py` runs the actual candidate `build()` against this snapshot, then the existing `proficiency_authoring.build()`, schema/semantic validator and `content_files()` with in-memory source-byte injection. Only scratch receives returned outputs; no repository write function is invoked.

PASS: default staging equals frozen Snowball bytes (`a6a139e96b518f2a9e01420af193e04ff6198bcd9fe344dd2248dfb64ea487b9`), including Snowball53855 `item_defined:true`. Source counts remain 443 profiles / 666 bindings. Explicit-pin prospective staging SHA `aca59d3e525c79792196f77c52940023f732bed0bb4372f5cd472eb185590803`, with exactly one changed binding51666 and all raw profiles unchanged. Existing content generator emits 665→666 content bindings; every previous665 row is unchanged; only Ink `oteryn:proficiency.tibia.p371`, `definition-r1`, standard is added. Definition shards and index remain byte-identical. Existing catalogue validation reports zero errors. Baseline bindings SHA `ece454f39872fb805a54c8718152a406d537a372b87ecc5a108e13a5fd70fd47`; prospective bindings SHA `10c5e10064b608adf7216de68ea968c6e05a269c95b63b6247318330293a4950`. Exact output/proof paths are under `snowball-qualified/`.

Six supplement unittest cases also pass without the external real-source environment variable (`portable-test-report.log`), proving ordinary tests need no HTML redistribution. Both real and synthetic test runs use the actual generator route.

The CP admission digest to bind is **manifest SHA `54dcb65092d8c370adcaa99f03d1ad87b62cce3d679d7ccfcd8567347a0712d9`**, which pins observations SHA `252190a37ebbf26f2b8846431ad04250e22910fd09a5705c53885c031c929166` and source HTML SHA above. Current normal integration point is `main()` → `build(*supplement_args)` → `threshold_supplement(...)`; an eventual admitted ordinary regeneration and `--check` must select that exact manifest/pin and retained source bytes. Merely updating the generated standard sample while leaving ordinary CI invocation unselected would fail drift checking and is not a completed integration. Selecting a default admitted package or retaining original source bytes for CI requires the CP's exact source-policy decision; this scratch patch does neither by self-assertion.
