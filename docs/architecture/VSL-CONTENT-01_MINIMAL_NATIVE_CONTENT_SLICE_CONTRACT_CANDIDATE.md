# VSL-CONTENT-01 — Minimal Native Content Compiler/Loader Slice Contract Candidate

- Date: 2026-08-16
- Gate: `VSL-CONTENT-01`
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Scope: minimum content/model/compiler/loader evidence needed by the first movement/combat vertical slice
- Runtime/client/compiler/Studio/DDL/Platform/production authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Problem

The first real native movement/combat slice needs exact immutable world/content revisions and a real compiler/loader path, but the accepted DUR-04 contract deliberately forbids freezing the final Oteryn World Project / World Bundle physical encoding before the mandated bounded format spike.

The slice therefore needs a contract that lets engineering prove:

- typed native content identity and references;
- deterministic lowering/projection;
- bounded artifact loading;
- exact client/server revision compatibility;
- atomic staging/activation;
- movement/combat consumption of one immutable revision;

without turning a test serializer, Rust memory layout or convenience file format into the permanent Oteryn format.

## 2. Accepted constraints

This candidate consumes without replacing:

- ADR-0005 native Oteryn World/Content Model, editable project vs compiled bundle separation, stable ContentKey, Area/Subarea/Zone versus Region/Chunk separation and legacy-import boundary;
- DUR-04 PackageKey/PackageRevision/ContentKey/Content Lock, deterministic compiler, client-safe/server projection, immutable bundle, bounded loader/staging/activation and format-spike gate;
- SIM exact behavior-affecting revision/replay provenance;
- FND-03 immutable revision availability checks and runtime activation ownership;
- FND-02 protocol/content capability/reconciliation boundaries;
- GAME-ITEM item-definition authority;
- GAME-ABILITY ability/effect semantic ownership;
- GAME-AI creature/spawn provenance and bounded spawn activation;
- GAME-INTERACTION stable authored interaction definition identity;
- GAME-CHANNEL source multiplicity/eligibility classification;
- ALPHA-CLIENT client-safe projection and release/content compatibility;
- VSL-MOVE/VSL-COMBAT minimum slice content requirements.

## 3. The first-slice content principle

The first vertical slice proves the native semantic pipeline, not the final editor/file-format product.

```text
versioned VSL fixture source
-> typed canonical Oteryn semantic graph
-> full semantic validation
-> deterministic lowering
-> server-authoritative projection
+ client-safe projection
-> deterministic VSL evidence artifacts
-> bounded loader/staging verification
-> explicit activation
-> movement/combat runtime consumes exact revision
```

The fixture source and evidence artifact profile are explicitly **noncanonical / non-shipping** physical representations until the DUR-04 physical-format spike is completed and a later owner decision freezes final encodings.

## 4. Minimum semantic content set

The first movement/combat slice requires only enough content to exercise real ownership/revision boundaries.

A valid VSL content set includes, at minimum:

### World/spatial

- one bounded `WorldId`/fixture world context;
- one bounded Region/Chunk-like technical fixture footprint sufficient for loading/index tests;
- explicit floors/cells/tiles in the fixture footprint;
- static collision/terrain capability metadata required by VSL-MOVE;
- at least one semantic Area/Subarea/Zone reference where useful for proving semantic vs technical spatial separation;
- one pure same-scope local relocation/teleport edge for movement evidence;
- spawn/source placement sufficient for one creature;
- stable spatial references/index input.

### Creature / AI

- one stable creature definition/template;
- finite HP/fixture progression fields needed by VSL-COMBAT;
- one bounded spawn definition with explicit source/provenance/recovery/multiplicity classification;
- one minimal deterministic AI/idle/action fixture definition as needed by the scenario.

### Ability/combat

- one player/AI usable ability/attack definition compatible with accepted GAME-ABILITY;
- fixture formula/profile references rather than guessed Reference formulas;
- one loot table/source definition with stable entry/purpose keys;
- one XP/progression reward fixture definition.

### Items

- at least one stable GAME-ITEM item definition suitable for durable materialization/pickup;
- container/corpse presentation metadata only where required by the slice;
- explicit server-authoritative versus client-safe projection classification.

### Client-safe presentation

- bounded project-owned test visuals/metadata sufficient for Tier 2 state/presentation evidence;
- no proprietary asset requirement;
- no server-only formula/loot/security field leakage.

Scripts, NPCs, quests, houses, market, broad monsters/items/spells, Studio UI and legacy import breadth are not required for the first slice.

## 5. Stable semantic identity

Every authored definition in the slice uses stable namespaced semantic keys/revisions from ADR-0005/DUR-04.

Examples of required semantic identities include:

- package/revision;
- World Bundle/content/map/ruleset/world-policy revision context;
- tile/terrain/content keys;
- creature/template key;
- spawn/source key;
- ability/effect definition key;
- loot table/entry/purpose key;
- item type key;
- interaction/relocation definition key.

Runtime compact IDs MAY be generated inside one artifact revision but are never cross-build semantic identity.

## 6. Versioned noncanonical VSL fixture source

Before the permanent editable World Project encoding is selected, implementation MAY use a bounded versioned `VSLContentFixture` or equivalent source representation solely for automated/component/Tier-1/Tier-2 evidence.

Required properties:

- deterministic and reviewable;
- explicit schema version;
- contains only the minimum semantic input needed to build the canonical graph;
- bounded counts/bytes/depths;
- duplicate keys and unknown critical fields rejected;
- no direct runtime authority;
- cannot be accepted as the permanent World Project encoding merely because it is convenient;
- must be removable/adaptable when the final format is selected;
- ordinary product release tooling MUST NOT expose it as the canonical authoring contract.

A programmatic typed fixture builder is also permitted if it produces the same canonical graph and remains test-only. Tests must not rely on unstable Rust memory layout as serialized compatibility.

## 7. Canonical semantic graph remains the architecture boundary

The compiler pipeline must operate on the accepted typed canonical semantic graph rather than directly coupling runtime logic to the fixture-source syntax.

This gives the first slice a durable architecture seam:

```text
future final World Project parser/importer
                 |
VSL fixture ---->+--> canonical semantic graph --> validation/lowering --> artifacts
legacy importer ->+
```

A later physical source format may therefore change without rewriting gameplay authority/content semantics.

## 8. Deterministic compile pipeline

The first slice compiler must realize DUR-04 semantics equivalent to:

1. parse/build bounded fixture source;
2. canonicalize into typed semantic graph;
3. validate keys, references, spatial semantics and capabilities;
4. resolve exact Content Lock / dependency revisions;
5. validate GAME-CHANNEL multiplicity/eligibility for value-producing sources;
6. validate client/server field visibility;
7. normalize deterministic semantic order independent of source enumeration;
8. lower spatial/content indexes deterministically;
9. generate server-authoritative projection;
10. generate client-safe projection;
11. assemble deterministic VSL evidence artifacts;
12. compute manifest/section/artifact integrity digests;
13. emit exact compile/provenance report.

Same fixture bytes/semantic inputs + Content Lock + compiler/canonicalization profile + target profile must produce the same logical result and exact VSL evidence artifact digest.

## 9. VSL evidence artifact profile

The slice may use a versioned **non-production `VSL_BUNDLE_EVIDENCE_PROFILE`** to exercise actual bytes, corruption handling, loading and client/server compatibility before the final bundle format is selected.

This profile must be deliberately simple, bounded and disposable. It must provide at least:

```text
header / profile version
manifest
exact content/map/ruleset/world-policy revision metadata
compiler/canonicalization identity
Content Lock digest
projection class (server or client-safe)
section table with bounded offsets/counts/lengths
digest/integrity metadata
payload sections sufficient for the slice
```

It MUST NOT:

- become the permanent `.owb`/World Bundle contract;
- use raw Rust struct layout as compatibility format;
- be advertised as production-ready distribution format;
- make compression/chunking/signing/CDN decisions permanent;
- bypass the DUR-04 physical-format spike.

The implementation must isolate this evidence profile behind semantic artifact/loader interfaces so replacement is measurable and bounded.

## 10. Final physical format remains gated

DUR-04 remains binding: before final World Project/World Bundle encoding is owner-accepted, a bounded reversible format spike must compare candidates for:

- deterministic byte identity;
- source-control diff/review behavior;
- random access;
- corruption/decompression/resource safety;
- source-to-bundle-to-load equivalence;
- editor save/recovery;
- representative world scale;
- 32x32 versus 64x64 chunk/floor packing candidates;
- patch/download granularity;
- client/server load locality.

The VSL evidence profile is evidence infrastructure, not a shortcut around this gate.

A later **format selection owner decision** remains required before a production/broad-content executor may freeze the permanent physical encoding.

## 11. Client-safe versus server projection

The compiler must produce explicit separate projections from the same locked semantic graph.

Server-only examples include as applicable:

- authoritative collision details not intended for client exposure when security/product policy says so;
- hidden trigger/loot/reward/security fields;
- server-only AI/ruleset/formula/source metadata;
- secret/private provenance or RNG evidence.

Client-safe projection contains only allowlisted data needed for presentation/input UX and compatibility.

The client-safe artifact is still non-authoritative. It cannot grant movement legality, combat result, loot outcome or admission/session authority.

Negative fixtures must prove server-only fields do not leak into the client artifact.

## 12. Spatial model for the slice

The slice preserves ADR-0005's separate layers:

```text
semantic geography: Area / Subarea / Zone
technical loading:   Region / Chunk
precise encounters:  EncounterZone / RaidCell / RaidAnchor (not required for minimal slice unless used)
```

The first fixture may use only one or a few bounded technical chunks. It must not make the fixture chunk dimensions a permanent product decision.

Movement/collision reads immutable static spatial facts from the exact active artifact revision. Runtime dynamic occupancy remains FND-03 state, not content mutation.

## 13. Spawn and value-source validation

Any slice spawn/source capable of producing durable value must declare the accepted GAME-CHANNEL multiplicity/eligibility classification required by GAME-AI/GAME-CHANNEL/DUR contracts.

Compilation/staging fails closed if:

- source identity/provenance is missing;
- population/placement bounds are invalid;
- creature/AI/content references are unresolved;
- required recovery classification is missing;
- required value-source multiplicity/eligibility is missing;
- loot/item references are invalid;
- incompatible semantic revisions/capabilities are present.

## 14. Fixture formula and Reference discipline

The slice content may reference `VSL_COMBAT_FIXTURE_PROFILE` / `VSL_MOVEMENT_FIXTURE_PROFILE` or equivalent explicit test-only policy revisions.

These are not Reference/Evolved product policy. They exist only to make structural proof deterministic while target facts are unavailable.

Compiler/release validation must distinguish such fixture-only profiles from product-release profiles. A build/artifact intended for ordinary release must fail if it attempts to activate a test-only fixture profile unless a separate explicit test environment flag/profile makes the non-product status unambiguous.

## 15. Loader safety

The VSL loader implements the same security envelope required by DUR-04 even though the evidence profile is temporary:

1. bounded minimal header read;
2. hard size/count checks before peer-sized allocation;
3. checked arithmetic for offsets/counts/totals;
4. profile/schema/capability validation;
5. manifest/section digest verification;
6. Content Lock/revision verification;
7. bounded section/index validation;
8. semantic consistency checks;
9. isolated staging construction;
10. explicit activation only after complete validation.

Corrupt, truncated, overlapping, oversized, incompatible or unknown-critical data fails before authoritative publication.

## 16. Staging and activation

Loading and activation are separate.

A staged server/client artifact pair may be validated/prewarmed without becoming active.

Activation binds one coherent exact revision set. A runtime scope cannot begin with a partial mixture of old/new fixture content. An active immutable revision is never edited in place.

For the first slice, gameplay-relevant hot reload is not required.

Rollback means activating a previously verified compatible fixture artifact. No migration-required durable state is introduced by the minimal fixture unless separately accepted.

## 17. Runtime consumption

After activation:

- FND-03 runtime scope records exact active World Bundle/evidence artifact/content/map/ruleset/world-policy revisions;
- VSL-MOVE reads static collision/relocation/spatial facts from that revision;
- GAME-AI/spawn uses creature/source/AI definitions from that revision;
- GAME-ABILITY/VSL-COMBAT uses ability/fixture/loot definitions from that revision;
- DUR-03 materialization records applicable source/item/content revision provenance;
- ALPHA-CLIENT activates only a compatible client-safe projection;
- QA/replay evidence retains the exact artifact digest and compiler/build revisions.

No runtime subsystem may requery mutable source files as authoritative behavior after activation.

## 18. Resource-limit dimensions

Before implementation acceptance, explicit finite registry values/boundary tests are required as applicable for:

1. fixture source bytes/file/count/depth;
2. packages/dependencies/ContentKeys/references;
3. world dimensions/floors/cells/objects-per-cell;
4. technical Region/Chunk counts and cell density;
5. compiled section count/bytes/index count;
6. artifact total bytes;
7. compressed/decompressed bytes/ratio if compression is exercised (compression may remain absent in the VSL profile);
8. string/blob/asset metadata lengths;
9. creatures/spawns/abilities/loot entries/item definitions in one fixture artifact;
10. client-safe asset/projection counts and bytes;
11. loader staging memory/work;
12. compiler work/output/evidence-report size;
13. relocation/spatial index candidate counts where artifact-controlled;
14. malformed/corrupt fixture corpus work bounds.

This contract chooses no numeric values.

## 19. Minimum first-slice evidence

A conforming implementation must prove at least:

1. same locked VSL fixture + compiler profile produces byte-identical evidence artifact/digest;
2. shuffled input/collection/filesystem enumeration does not change semantic artifact output;
3. duplicate keys/unresolved references/invalid spatial references fail compilation;
4. missing GAME-CHANNEL source multiplicity classification blocks value-producing spawn activation;
5. client-safe projection contains no server-only fixture fields;
6. corrupt/truncated/offset-overflow/oversized artifact fails before activation/unbounded allocation;
7. incompatible client/server content/revision pair fails closed;
8. complete valid staged artifacts activate atomically for a new eligible runtime/client context;
9. VSL-MOVE consumes collision/relocation facts from the exact active artifact;
10. VSL-COMBAT/AI consumes creature/ability/loot/item definitions from the exact active artifact;
11. runtime/replay evidence retains exact server build/compiler/artifact/Content Lock/SIM/fixture revisions;
12. no live source-file mutation changes running authoritative semantics;
13. fixture-only rules cannot be mistaken for Reference parity or ordinary release profile;
14. the evidence profile can be replaced behind the semantic compiler/loader boundary without redefining canonical content identity.

## 20. Required follow-up format spike

The first executor package may include a dedicated **non-production content-format spike** after this architecture is accepted.

The spike:

- compares candidate source/bundle encodings under DUR-04 §24 criteria;
- produces benchmark/equivalence/corruption/recovery evidence;
- cannot merge a permanent compatibility format merely by implementation choice;
- returns one owner-decision package for final physical encoding;
- must not block unrelated protocol/runtime/persistence/movement/combat structural work that can consume the semantic graph + VSL evidence profile.

## 21. Explicit non-decisions

`DECISIONS_NOT_TAKEN`:

- final World Project source syntax/serializer;
- final World Bundle container/serializer;
- final file extensions;
- final chunk size/floor packing;
- compression/delta/CDN/signing format;
- Oteryn Studio UI/viewport/editor implementation;
- broad legacy import;
- WIT/Wasmtime implementation for the first slice;
- scripting requirement for movement/combat fixture;
- broad NPC/quest/shop/event/house/market content;
- proprietary assets;
- exact Global movement/combat/loot/XP values;
- numeric resource limits;
- production rollout/hot reload.

## 22. Decision timing

- **Must decide now?** `YES` for the minimum semantic content set, fixture-vs-product boundary, canonical graph/compiler/projection/loader/activation seam and noncanonical VSL evidence profile constraints.
- **Must decide final encoding now?** `NO`; accepted DUR-04 explicitly requires measured format-spike evidence first.
- **Concrete downstream blocked:** movement/combat structural implementation, real loader/activation tests, client/server revision compatibility and end-to-end content provenance.
- **Harder later:** test fixtures could become accidental product formats; runtime systems could couple directly to ad-hoc source structs; server-only content could leak into clients; immutable revision identity could be lost.
- **Superseding evidence:** format spike/representative content scale shows the semantic compiler/loader seam itself is inadequate, or a later accepted content architecture preserves stronger determinism/security/review properties.
- **Deliberately not decided:** all permanent physical format/tooling/technology choices listed above.

## 23. Recommendation

`RECOMMENDATION: ACCEPT` this minimum content slice architecture while preserving the mandatory post-architecture format spike before final physical encoding selection.

Acceptance would authorize architecture only. It does not authorize the format spike, compiler/loader implementation, Studio, runtime activation or production content.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`
