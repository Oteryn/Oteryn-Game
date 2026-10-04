# Local Canary/Crystal spell data closure

This is the current local data work. It does not select a server manifest, activate
native implementations or publish a PR. Canary and Crystal variants remain
separate; a source registration is not a unique real-Tibia spell count.

Source order: use existing pinned local inputs, preserve source declarations and
mechanics, fit or extend authoring schemas, then compare external sources. The
source qualification batches retain upstream values. Once source capture and
syntax preservation are complete, cached wiki facts are compared in separate
enrichment batches without replacing those values.

## Materialized imports

| Set | Contents | Admission |
| --- | --- | --- |
| `imports/spells/r28/` | Base source profiles, registrations, mechanics and 297 player candidate bundles | Source only |
| `imports/spells/r29/` | Four Crystal companion candidates and explicit grade actions | Source only; Swift Foot legacy GREATER placeholder is not a source write |
| `imports/spells/r30/` | Three Canary speed candidates and current input-provider facts | Source only; current staff speed cap differs from the old model |
| `imports/spells/r31/` | 59 Condition declarations, 12 partial Effect prefixes, 47 explicit gaps | Source only; no complete candidates or application qualification |
| `imports/spells/r32/` | Typed current Canary Swift Foot program with source spans and grade branches | Source only; no new complete candidate |
| `imports/spells/r33/` | 59 Condition semantics records and 146 ordered parameter assignments | Source only; lifetime and persistence remain distinct |
| `imports/spells/r34/` | Four actual instant Conjure candidates for Chameleon/Destroy Field wrappers | Source only; source Item provider preserved and visual branch remains unbound |
| `imports/spells/r35/` | Three source Monk spender programs and actual source helper/conversion facts | Source only; helper runtime/input providers remain unqualified |
| `imports/spells/r36/` | 100 source conjure helper branch records | Source only; Item type and renderer remain unqualified |
| `imports/spells/r37/` | All 175 unresolved monster slot links, 156 registry links and 21 custom descriptor links | Source only; original unresolved status preserved |
| `imports/spells/r38/` | Full selected Lua syntax: 2346 records, 506755 nodes, 137 anonymous functions | Source only; parser facts do not qualify engine bindings or execution |
| `imports/spells/r39/` | 32 lexical operator corrections, with all 2346 per-file counts reconciled | Source only; 50286 actual calls and 11966 unsupported references, without implementation claims |
| `imports/spells/r40/` | 773 cached infoboxes, 483 source comparisons and 485 supplemental observations | Supplemental only; 469 context matches and explicit gaps; frozen 308-candidate comparison baseline |
| `imports/spells/r41/` | 1869 cached species facts, 5680 attack observations, all 5237 profile and 20742 slot comparisons | Supplemental only; no upstream overrides or fully Wiki-verified slot claims |
| `imports/spells/r42/` | One current Canary Heal Friend candidate with caster effect before combat | Source only; range conversion, engine providers and assets remain unqualified |
| `imports/spells/r43/` | Eight source variants of four house commands and exact local manual-note quotes | Secondary notes only; original manual provenance and fresh Wiki verification remain false |
| `imports/spells/r44/` | One current Crystal Restore Balance candidate with literal healing formula | Source only; unused basePower retained; engine providers and assets remain unqualified |
| `imports/spells/r45/` | Two source Paralyze partial Ability/Condition/Formula templates and exact Lua/C++ return-stage facts | No complete Spell candidates; both registrations remain blocked by the cast-return contract mismatch |

| `imports/spells/r59/` | 41 private source-complete v2 state/Wheel candidates; two removed Sap Strength references | Data only; authoring extensions and runtime consumers pending |
| `imports/spells/r60/` | 23 private source-complete v2 equipment/Monk candidates | Data only; canonical normalization and source differences explicit |
| `imports/spells/r61/` | 26 private source-complete v2 party/acquisition/summon candidates | Data only; controller parameters preserved, consumers pending |
| `imports/spells/r62/` | 31 private source-complete v2 world/control candidates; two retired Expose Weakness and four disabled references | Data only; no active reference examples or runtime admission |

Each set has its own manifest, schema snapshots and complete checksums. Existing
sets are immutable inputs to subsequent batches. The manifest's admission and
qualification flags govern; a passing schema does not enable execution.

The effective structural candidate population after r59-r62 is **475 of 483
current player registrations**. The **eight remaining BLOCKED records are
reference only**: two removed Sap Strength, two retired Expose Weakness and four
disabled examples. The 121 new private source-complete v2 candidates preserve
pending authoring contracts and unimplemented source consumers. Earlier 354
candidates retain their earlier provider and execution limits.
This count includes donor variants and source-only native descriptors. It is not
a playable spell count. Monster data separately preserves 5237 profiles and
20742 attack/defence entries. The immutable base has 175 unresolved entries;
r54 projects 10 completely and 62 partially, leaving 165 target-data holds.
All 175 selected slots still have unqualified runtime execution.

## Current completion queue

| Set | Concrete work | Current stage |
| --- | --- | --- |
| r32 | Canary Swift Foot NONE conditions, REGULAR modifier and GREATER no-op, familiar propagation and return order | Actual import passed independent audit |
| r33 | 59 Condition semantics records, 146 ordered parameters, source ConditionId/subID/enum namespaces and two indefinite lifetimes | Actual import passed independent audit |
| r34 | Four conjuring wrappers incorrectly dispatched by name to rune-use native behavior | Actual import passed independent audit |
| r35 | Three Canary Monk callback formulas with current Harmony/helper and conversion stages | Actual import passed independent audit |
| r36 | 100 source conjure helper records with conditional rune visual and failure/creation/decay order | Actual import passed independent audit |
| r37 | Exact source-mechanics and custom-descriptor links for all 175 unresolved monster slots | Actual import passed independent audit |
| r38 | Full source syntax for bodies/expressions previously retained only as opaque references | Actual import passed independent audit; 45 observed node kinds, schema covers 59 |
| r39 | 32 reserved operators incorrectly counted as calls by the old lexical inventory | Actual import passed independent audit |
| r40 | 773 cached infoboxes, 483 comparisons and 485 unselected supplemental observations; 469 context matches | Actual import passed independent audit |
| r41 | Cached monster spell references rebound to 5237 profiles and 20742 slots | Actual import passed independent audit; general bestiary fields do not prove spell verification |
| r42 | Current Canary Heal Friend caster effect before combat, healing formula and paralysis dispel | Actual import passed independent audit |
| r43 | Four house commands compared with existing local manual notes | Actual import passed independent audit; secondary notes, online sources unavailable |
| r44 | Current Crystal Restore Balance caster effect before combat and literal healing formula | Actual import passed independent audit |
| r45 | Two Paralyze partial templates, 6000 ms, raw speed coefficients and owner C++ normalization | Actual import passed independent audit; no full Spell projection or qualification |
| r46 | Two current Canary movement candidates: Levitate and Magic Rope; exact helper proof | Actual import passed independent audit; engine/world execution remains unqualified |
| r47 | Fourteen guard, state, target and parameter source evidence records | Actual import passed independent audit; zero complete candidates |
| r48 | Three ordered cast/callback source programs: Forked Glacier/Thorns and Sweeping Takedown | Actual import passed independent audit; zero complete candidates |
| r49 | Complete typed same-file source programs for all 21 other-cast variants | Actual import passed independent audit; external helpers and full Spell projection remain unqualified |
| r50 | Six source Monk Formula definitions and a closed reference to the existing tier helper | Actual import passed independent audit; bounded evaluator checked against original C++ helper |
| r51 | Source models for all 175 unresolved monster slots, including 19 inline cases | Actual import passed independent audit; original native projection statuses remain unchanged |
| r52 | Three player control/chain candidates; separate three native templates with four proposed bindings | Source only; aliases do not promote source receipts |
| r53 | Six standard Monk candidates using accepted S5/S16 | Source only; raw Canary formulas retained, providers unqualified |
| r54 | Ten full monster target-data candidates, 62 partial projections, 103 blocked | Source only; no native/runtime qualification |

The r33 condition model distinguishes `ticks=-1` (no countdown) from persistence
and removal on death. Unresolved subID symbols remain unresolved. The r36 helper
model preserves omitted fourth arguments as nil; it does not assign a universal
red default or qualify an Item type, renderer binding or native execution.

After these data batches, reconcile the overlays by registration key, run the
affected test suites and independently verify every new import. Continue with
actual source/schema gaps without regenerating or downloading immutable inputs.
Server/node wiring remains with the coordinator and architect for #162.

For Paralyze, the current Lua bridge returns true for VARIANT_NUMBER after calling
doCombat without propagating its health/condition result. Thus the caster green
effect cannot be gated on successful condition application as the old D.5.3
conformance example suggests. R45 records this contract correction and retains
both full spells as BLOCKED. The normalized speed Formula 40 is a source
uint16-base-domain projection; stacking, final entity speed and providers remain
unqualified. These templates contain no executable Spell or runtime admission.

All thirty-five r28-r62 sets are materialized. Each carries pinned schemas,
source proofs and inactive admission flags; source and import guards passed
independent review. Run
`tools/content-migration/build_source_spell_review_index.py` with the existing
Python environment to produce `source-closure-review-index.json`. This single
review view links the imported manifests and all 483 player registration
receipts. It refuses active sets and is not a server selection manifest.
The index is generated from actual imports: 483 records, 475 structural
data candidates and eight explicit reference-only blocked registrations. The separate monster target review
records 10 full data candidates, 62 partial and 103 blocked. Later syntax evidence must not upgrade
these statuses merely because it parses successfully.

The r38 extraction uses the pinned upstream LuaParser 4.2.0 grammar, without
executing source code. All 2346 source identities and semantic node fields are
retained. Every current player registration (483/483) has its exact donor, revision
and source path represented in these syntax records, including blocked casts.
Comments/trivia are omitted; exact source hashes remain available.
Decoded string values are parser-produced facts with explicit Lua fidelity
limits, while original literal spellings are retained. Reported spans are
upstream anchors rather than a claim of complete source ranges. C++ helper
qualification remains separate from parsing Lua syntax.

## Evidence

Detailed base counts and regeneration checks are in
`r28-source-closure/README.md`. New sets carry their own receipts, source pins,
schemas and tests. Current data comes from local immutable Git objects:

- Canary: `04b83b512114bfd888000d6e1433ed8ecaec7c5b`.
- Crystal: `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.

The expanded player-authoring suite ran 382 tests successfully with local source
configuration supplied: all passed, with zero skips.
All 45 affected import tests passed together; the five new r37 importer tests
then passed separately. The narrow repaired r35 metadata case has six passing
tests and independent review. The five new cached-reference, house-command,
precombat-healing and partial-Paralyze import modules also ran together in one
process: 30 tests passed. At that historical stage the index was independently rebuilt and checked:
18 manifest hashes, all 483 receipt/header links, all 310 candidate bundle links
and every metadata pointer resolve; the 173 blocked records have no executable
bundle pointer. No earlier sealed import was rewritten.

The next r46-r48 batch ran 36 affected producer/importer tests together: all
passed. Independent actual-copy and index audits verified all 21 sets and the
updated 483/312/171 population. R47/R48 preserve exact current source facts while
keeping their 17 full registrations BLOCKED. R46 qualifies source-data transfer
of two existing movement descriptors; it does not qualify server movement.
See `SOURCE-CLOSURE-NEXT-BATCH.md` for the bounded current main/#1534 comparison
and corrections to historical queue classifications.

The r49-r51 source-model implementation batch ran 35 tests together in one
process: all passed. Independent review reconstructed every source packet and
verified the actual imported bytes, source paths, schemas and checksums.
The index was rebuilt identically with 24 sets and unchanged 483/312/171 player
statuses. `SOURCE-MODELS-COMPLETION.md` explains the three concrete implemented
data models; `source-schema-progress.json` links their imported schemas and
manifests. Complete same-file programs, bounded formula evaluation and source
slot models are distinct from full native Spell/Ability projection.

The r28-r39 source-first batches used local immutable Git inputs, without wiki
requests, Remote Desktop or online calculators. The separate r40/r41 enrichment
batches read existing BR, Fandom and Tibiopedia caches; original capture methods
and unavailable timestamps remain explicit, rather than being reconstructed.
R43 uses local Oteryn-written manual notes as secondary evidence. New Tavily
research required reconnection, normal HTTP/wiki reads failed or redirected, and
Remote Desktop devices were offline. No browser read or desktop modification was
performed. Original Lua/assets remain reference inputs, not import-set payloads.

## Historical local unblocking batch: r55-r58

R55-r58 add33 actual target-data candidates. All162 prior blocked variants
have exact lane audits; at that stage129 structural holds remained, including4 disabled
examples and2 retired references, leaving123 gameplay data holds. Native
reader admission, providers and runtime execution stay separate and false.
See `UNBLOCK-162-RESULTS.md` and `unblocking-162-review-index.json`.


## Current source data completion: r59-r62

The remaining 129-row data cohort now has 121 concrete private source-complete v2
candidates and eight reference-only rows. `final-129-completion-review-index.json`
records this new result separately from the unchanged historical 33/129 audit.
`source-private-consumer-worklist.json` preserves all 121 pending-contract and
consumer records, including exact receipt, source-header and projection paths.

These are source DATA imports. They do not qualify source/numeric equivalence,
native execution, providers, renderer bindings, server selection or activation.
Private source-complete v2 authoring extensions still need accepted contracts and
actual consumers. Existing canonical Wiki facts are retained; this batch used
local pinned source captures and Git objects, without fresh network/Wiki research
or Remote Desktop. See `FINAL-129-DATA-COMPLETION.md` for scope and regeneration.
