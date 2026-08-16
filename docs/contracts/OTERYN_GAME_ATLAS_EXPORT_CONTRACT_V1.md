# Oteryn Game -> Atlas Immutable Export Contract v1

- Contract ID: `oteryn-game-atlas-export-v1`
- Semantic revision: `1`
- Coordination ID: `OTERYN-GAME-ATLAS-V1`
- Canonical owner: `Oteryn-Game` (current source lineage: `blakinio/Oteryn-v2`)
- Consumer: future `Oteryn-Atlas`
- Status: **PROPOSED until this exact contract content is merged to protected `main`; ACCEPTED thereafter unless explicitly superseded**
- Runtime implementation status: `NOT_IMPLEMENTED`
- Production status: `NOT_ENABLED`

## 1. Purpose

This contract defines the semantic boundary by which `Oteryn-Game` may publish a deterministic, immutable, public-safe projection of canonical Oteryn World/Content for consumption by `Oteryn-Atlas`.

It intentionally freezes the smallest durable semantics required before Game exporter design, Atlas consumer design, legacy Atlas responsibility separation and selective history extraction can proceed safely. It does **not** freeze transport bytes or deployment technology.

## 2. Authority and pinned evidence

This contract is derived from the following exact repository evidence:

- `blakinio/Oteryn-Platform@b549e42041eda426bbf88db469862a92df930860`
  - `docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md`
  - current temporary cross-repository authority for this topology scope;
- `blakinio/Oteryn-v2@afcbf8585ba23c506242978c38b2b51f9ea6f1b6`
  - `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md`
  - accepted native World/Content, stable identity and bounded legacy-import authority;
- `blakinio/Otheryn@39cb2ce4ff427e7c3760eb6112b45efc0c1f73b8`
  - `docs/architecture/OTERYN_ATLAS_EXTRACTION_REVIEW_2026-08-15.md`
  - merged migration-source audit with verdict `EXTRACTABLE_WITH_REFACTOR`.

**FACT** — ADR 0041 assigns Game ownership of the Atlas export schema, public-field allowlist/classification, deterministic exporter, producer validation/golden fixtures and export provenance. It assigns Atlas ownership of consumer validation, limits, indexing, derived caches, render/publication and consumer compatibility evidence.

**FACT** — ADR 0041 requires the primary Game -> Atlas path to be an immutable artifact/snapshot rather than a synchronous Game Server API, and requires complete deterministic snapshots before any delta design.

**FACT** — ADR-0005 makes OTBM and other legacy formats bounded importer inputs, not canonical editable/runtime models, and defines stable namespaced content identity independently of legacy numeric identifiers.

**FACT** — the Otheryn extraction audit identifies current mixed Game/Atlas responsibilities and makes a formal versioned Game -> Atlas contract a P0 prerequisite before clean extraction.

## 3. Decision timing

- **Must decide now:** `YES`.
- **Blocked downstream work:** Game exporter design, Atlas consumer design, mixed legacy Atlas module separation, migration manifest finalization and selective Atlas extraction.
- **Risk of deferral:** an ad-hoc OTBM-shaped or repository-layout-shaped interface would create a second world truth model and make later extraction/compatibility materially harder.
- **Supersession evidence:** measured export/publication constraints, canonical World/Content evolution, security/privacy findings, consumer evidence or proven need for a new capability may justify a successor revision.
- **Deliberately not decided:** physical serialization, compression, exact spatial chunk dimensions, object storage/CDN, Atlas implementation framework, delta encoding, retention duration and repository migration mechanics.

## 4. Non-negotiable ownership boundary

### 4.1 Game owns authoritative export semantics

`Oteryn-Game` is the only canonical owner of:

- the semantic export contract and its revisions;
- classification of which canonical Game facts are eligible for public Atlas projection;
- the default-deny public-field allowlist;
- stable Game-domain identities exposed by the export;
- transformation from canonical World/Content into the public projection;
- deterministic producer behavior;
- producer-side validation and golden fixtures;
- source World/Content revision identity;
- exporter/contract revision identity;
- immutable artifact identity and producer provenance.

### 4.2 Atlas owns derived consumption and publication

`Oteryn-Atlas` owns:

- parsing/decoding the selected physical profile;
- consumer-side schema and semantic validation;
- bounded resource handling;
- search/spatial indexing;
- derived caches and publication shards;
- browser/runtime presentation;
- Atlas application and publication artifacts;
- declaration of the exact Game export revision/digest consumed;
- consumer compatibility evidence and rollback of Atlas publication.

### 4.3 Platform is not a world-data transit owner

`Oteryn-Platform` may provide discovery/entry policy for the public Map product, but it must not become the canonical producer, schema owner, data transit authority or repair path for missing Game export fields.

### 4.4 Meta does not duplicate provider schemas

A future `Oteryn` meta repository may record contract discovery, compatible producer/consumer combinations and release manifests. It must not copy this Game-owned schema as a second normative contract.

## 5. Forbidden source paths and authority shortcuts

An Atlas implementation conforming to v1 **MUST NOT** use any of the following as an alternate or fallback source of world truth:

- OTBM files;
- Legacy IR or legacy OTBM-shaped `Tile`/`Item` models;
- Canary/Crystal XML trees;
- Canary/Crystal Lua/scripts or factual-analysis tooling;
- the canonical editable Oteryn World Project wholesale;
- undocumented Game database tables;
- live GameNode memory/state;
- a synchronous Game Server API as the primary dataset path;
- Platform-owned database/API state to reconstruct omitted Game facts;
- viewer-generated identity strings as substitutes for canonical Game identities.

Missing, ambiguous, omitted or unsupported data remains missing/ambiguous/omitted/unsupported. Atlas must not infer authoritative facts from legacy or secondary sources to make the UI appear complete.

## 6. v1 artifact model

v1 is a **complete deterministic immutable snapshot**.

For identical canonical source revision, export policy revision, exporter revision and declared deterministic configuration, the producer must generate semantically identical output and, once a physical serialization profile is frozen, byte-identical canonical artifact content.

The artifact is immutable after publication. Correction requires a new export identity and digest; published bytes are never edited in place.

### 6.1 Required manifest semantics

Every v1 export must expose semantic equivalents of:

- `contract_id` = `oteryn-game-atlas-export-v1`;
- `semantic_revision` = `1`;
- `export_id` — immutable identity for one complete export;
- `world_id` — canonical logical world identity when the exported dataset is world-scoped;
- `world_schema_revision` — canonical World/Content schema revision used by the producer;
- `content_revision` — exact canonical content revision projected;
- `export_policy_revision` — exact public allowlist/classification policy revision;
- `exporter_revision` — exact exporter semantic/build revision;
- `producer_repository_sha` — exact Game source commit used to produce the export;
- `artifact_digest` — content identity for the immutable canonical artifact under the selected physical profile;
- `source_provenance` — bounded, public-safe immutable provenance sufficient to reproduce the source selection without leaking secret/editor/private details;
- `coordinate_profile` — explicit identifier/version for position, bounds and floor semantics;
- `capabilities` — explicit semantic capabilities present in the export;
- `asset_catalog_revision` or equivalent — when exported records reference appearance/outfit/publication assets;
- `required_consumer_capabilities` — capabilities without which the dataset cannot be interpreted safely.

The exact field spelling and byte representation are deferred to the physical schema profile. The semantics above are mandatory.

### 6.2 Deterministic metadata rule

Wall-clock publication timestamps, request IDs, machine paths, runner IDs or other nondeterministic execution metadata must not change the canonical artifact digest for otherwise identical canonical inputs.

Operational publication timestamps may exist as Atlas-side publication metadata or as non-canonical envelope metadata, but they are not Game world/content authority.

## 7. Public-safe projection policy

The export is **default deny**.

A canonical Game field or fact is absent from Atlas unless the Game-owned export policy explicitly classifies it for this contract revision/capability.

The producer must not expose server-only/editor-only/unreleased/security-sensitive material merely because the canonical World Project contains it. Examples of material that remains excluded by default include:

- hidden/server-only mechanics;
- private script inputs or execution details;
- admin/moderation metadata;
- editor-only notes/state;
- unreleased content;
- secret/anti-abuse data;
- private provenance such as workstation paths or credentials;
- mutable live gameplay/player state unless a later explicit contract makes a bounded derived fact public.

A consumer cannot request excluded fields through a secondary Atlas path.

Public classification is semantic, not repository-path-based. Moving a file into an export or Atlas directory does not make its content public.

## 8. Stable identity rules

### 8.1 Canonical entities

When a record represents a canonical Game entity with stable identity, the export must use the canonical stable Game identity. Namespaced stable keys from the Content Registry are preferred where ADR-0005 assigns them.

Legacy numeric IDs, OTBM IDs and build-local compact runtime IDs are not canonical public identity unless a separate contract explicitly promotes a scoped identifier.

### 8.2 Instances and placements

A placed/spawned/static record that requires independent addressability must receive a deterministic exported record identity derived from canonical stable inputs defined by the producer contract/profile. Atlas must not make a random/browser-local identifier authoritative.

### 8.3 Unresolved legacy identity

If migration evidence cannot establish a canonical stable identity, the producer must preserve that uncertainty explicitly. It must not invent a canonical-looking identifier from a display label or file path.

## 9. Coordinates, floors, bounds and ordering

v1 requires an explicit versioned `coordinate_profile` before executable producer/consumer implementation may claim compatibility.

That profile must define at minimum:

- horizontal axes and orientation;
- coordinate numeric domain and bounds;
- floor/level identity and ordering semantics;
- point versus area/bounds representation;
- inclusion/exclusion rules for bounds;
- position validity rules;
- deterministic ordering for multiple presentation records at the same position;
- stack/layer semantics when render primitives require them;
- displacement/anchor semantics when asset presentation depends on them.

No consumer may infer these semantics from Tibia/OTBM conventions, legacy `z` values, current viewer behavior or repository file layout.

The exact numeric limits and physical encoding remain deferred until the canonical coordinate/world contract and serialization profile are implementation-ready.

## 10. Semantic record families

The physical profile may package data differently, but v1 may expose only explicitly classified records in these semantic families.

### 10.1 Spatial presentation source

May contain public-safe canonical facts required to render/navigate the static world, including positions/bounds and deterministic presentation ordering or canonical render primitives where the export policy permits them.

Physical chunking is packaging, not authority. A chunk/shard/grid identifier must never become canonical entity identity.

### 10.2 Entity and overlay records

Candidate families include, only when explicitly public-allowlisted:

- NPC definitions/placements/spawns;
- monster definitions/placements/spawns;
- POIs;
- houses;
- towns;
- waypoints;
- teleports/transitions;
- public zones/areas/subareas;
- raids/events/encounter areas;
- deliberately public mechanics evidence.

This list is a semantic capability envelope, not permission to export every underlying Game field.

### 10.3 Search source records

Search-source records may include:

- stable canonical/export record identity;
- public label;
- public category/type;
- canonical position or bounds;
- reference to a public detail record.

Atlas owns the derived search index. Game does not own Atlas's index format.

### 10.4 Asset references

When public presentation requires asset references, the export may expose stable appearance/outfit/public asset identities and deterministic content-addressed or derivation references permitted by the export policy.

This contract does not grant redistribution rights for Tibia/CipSoft or other third-party assets. Asset licensing/provenance remains a separate mandatory gate.

## 11. Evidence and uncertainty vocabulary

When a record is derived from migration/reference evidence whose interpretation is not fully canonical, the producer must classify its semantic certainty using:

- `RESOLVED` — canonical identity/meaning is established by accepted Game semantics;
- `AMBIGUOUS` — two or more materially plausible interpretations remain;
- `UNRESOLVED` — evidence exists but cannot yet be mapped to a canonical meaning;
- `UNKNOWN` — required evidence is absent or insufficient.

Atlas must preserve these states truthfully when presenting or indexing such records. It must not promote `AMBIGUOUS`, `UNRESOLVED` or `UNKNOWN` to `RESOLVED` through display-name matching, legacy IDs or heuristic inference.

A later corrected interpretation produces a new Game export revision; Atlas does not mutate the source record in place.

## 12. Compatibility and capability negotiation

Compatibility is defined by contract/schema revision plus declared capabilities, not by repository name, branch, directory layout or current implementation language.

Rules:

1. Atlas must declare the exact export digest and semantic/schema revision it consumes.
2. An unknown **required** capability is a fail-closed incompatibility.
3. An optional capability may be ignored only if the artifact explicitly marks it optional and no required record semantics depend on it.
4. A consumer must reject a schema/profile revision whose mandatory semantics it cannot validate.
5. Producer ownership does not permit silent breaking changes to an already supported consumer pair.
6. A future meta compatibility matrix may mark a Game-export/Atlas-consumer pair ecosystem-supported only after both producer and consumer evidence exists.
7. Repository redirects or renames must not be used as compatibility/version signals.

A breaking semantic change requires a new contract/schema revision and an explicit compatibility/rollout decision.

## 13. Validation and resource-safety contract

### 13.1 Producer validation

Before an export is publishable, Game must prove at minimum:

- deterministic generation for identical declared inputs;
- manifest/source-revision consistency;
- stable identity validity and uniqueness within declared scopes;
- coordinate/floor/profile validity;
- public allowlist enforcement;
- absence of forbidden private/server/editor fields in representative negative tests;
- reference integrity;
- digest generation over the canonical physical artifact once that profile exists;
- golden fixtures for semantic contract behavior.

### 13.2 Consumer validation

Before Atlas accepts an export, it must validate at minimum:

- supported contract/schema/profile revision;
- required capabilities;
- artifact integrity/digest;
- bounded counts, lengths, nesting and allocation;
- bounded decompression/expansion if compression exists;
- coordinate/floor/profile constraints;
- identity/reference integrity required for indexing;
- malformed/duplicate/conflicting records;
- unsupported required fields/capabilities;
- publication build failure without partial promotion when validation fails.

### 13.3 Numeric limits

Exact numeric resource ceilings are intentionally not chosen by this semantic contract. They must be frozen by the implementation/schema profile before untrusted artifact parsing is enabled.

`unbounded` is not a valid implementation default.

## 14. Failure behavior

The producer and consumer fail closed.

- Invalid Game source/projection input => no publishable export.
- Export policy violation => no publishable export.
- Digest mismatch/corruption => Atlas rejects the artifact.
- Unsupported required capability/revision => Atlas rejects the artifact.
- Malformed or limit-exceeding record => Atlas rejects the affected candidate publication according to the later physical-profile atomicity rule; no silent truncation may create a dataset presented as complete.
- Missing public fact => remains missing; Atlas does not query legacy/Game internals as fallback.
- Atlas ingestion/publication failure => previous known-good Atlas publication remains independently rollbackable; Platform core availability is unaffected.

Failure categories may become machine-readable in the implementation profile, but the fail-closed semantics are mandatory now.

## 15. Publication identity and rollback

An Atlas publication must bind to exactly one validated immutable Game export identity/digest plus the exact Atlas application/index/publication revision that consumed it.

Rollback is selection of a previously validated immutable Game export + compatible Atlas publication, never mutation of an existing Game export.

The future ecosystem release manifest should be able to pin at minimum:

- Game repository SHA;
- relevant Game component/export version;
- World/Content schema/content revision;
- Game Atlas export semantic/schema revision;
- Game export artifact digest;
- Atlas repository SHA/version;
- Atlas publication/image/artifact digest;
- compatibility status/evidence identity.

A repository commit alone is provenance; it is not a substitute for all artifact/schema/component identities.

## 16. Snapshot and delta policy

v1 supports complete snapshots as the canonical recovery path.

Incremental/delta export is **not part of v1**. It may be introduced only after measured artifact size/build/publication evidence demonstrates that full snapshots are materially insufficient.

Any future delta contract must:

- bind an exact immutable base digest;
- bind an exact immutable target digest;
- fail if the base is not exact;
- retain complete-snapshot recovery;
- define atomic consumer/application semantics;
- not weaken provenance or public allowlist rules.

## 17. Legacy Atlas migration implications

This contract deliberately makes these current Otheryn responsibilities future Game-owned or rewrite targets before Atlas extraction:

- OTBM framing/semantic decoding;
- Legacy IR / map semantic interpretation;
- Crystal/Canary spawn/house/mechanics/NPC/monster/raid interpretation;
- canonical factual normalization;
- producer-side public projection/export.

It makes these concerns future Atlas-owned after adaptation to this contract:

- viewer/browser runtime;
- URL/deep-link behavior;
- spatial/search indexing;
- derived caches;
- publication verification;
- browser presentation/animation.

Mixed legacy modules such as the current Atlas orchestrator and factual-layer bridge must be split or rewritten around this boundary. They are not valid extraction units merely because they share a directory.

Generated `build/**`, temporary spools and generated caches are regenerated from immutable inputs and are not source-history migration material.

## 18. Rollout order

The safe rollout order is:

1. merge this semantic v1 contract in Game;
2. freeze the required coordinate/profile and public allowlist details needed by the first executable exporter without selecting unrelated deferred technologies;
3. implement bounded Game producer/exporter and producer fixtures;
4. create/adapt Atlas consumer parser/limits/validation against an immutable test export;
5. prove targeted producer/consumer compatibility on exact immutable revisions;
6. split legacy mixed Game/Atlas responsibilities around the proven contract;
7. prepare and review the exact selective history-extraction migration manifest;
8. perform any repository creation/extraction only under separately explicit authority;
9. prove reproducible Atlas publication and rollback from immutable Game exports;
10. record supported Game-export/Atlas-consumer pairs in the future meta compatibility/release manifest when that authority exists;
11. perform public Map integration/deployment only under separate deployment/production/DNS authority.

No step authorizes production activation by itself.

## 19. Explicitly deferred implementation profile

The following remain `UNKNOWN / DECISION_DEFERRED` and must not be inferred from this contract:

- JSON/JSONL/Protobuf/FlatBuffers/other byte format;
- compression algorithm and archive/container format;
- manifest file naming and directory layout;
- exact spatial chunk/grid dimensions;
- exact coordinate numeric ceilings until canonical model evidence is implementation-ready;
- exact hash algorithm/profile used for `artifact_digest`;
- object storage/CDN/package registry choice;
- Atlas frontend/runtime framework;
- whether raster/vector/render primitives or multiple capabilities are published;
- whether sprite/appearance raster derivation is Atlas-owned or another explicitly accepted build boundary;
- exact immutable artifact retention duration;
- delta encoding;
- exact history-filter command/path set;
- repository organization handle/transfer date;
- public hostname/origin;
- third-party asset redistribution permissions.

These are not omissions that consumers may fill by convention. Each must be resolved by the owning later contract/task when it becomes a concrete blocker.

## 20. Acceptance evidence required before implementation claims

This documentation contract alone proves architecture/semantic agreement only. It does **not** prove an exporter, consumer, Atlas repository, migrated history, deployment or production state.

Future implementation claims require named evidence for the exact implemented layers. At minimum, the first executable Game -> Atlas pair must demonstrate:

- producer deterministic/golden-fixture evidence;
- default-deny public projection negative tests;
- consumer malformed/resource-limit tests;
- exact digest/provenance binding;
- supported revision/capability negotiation;
- representative coordinate/floor/order rendering/indexing proof;
- explicit ambiguity-state preservation;
- independent rebuild of derived Atlas indexes/caches from one export;
- rollback to a prior immutable dataset;
- exact producer and consumer repository/artifact revisions.

Until those proofs exist, runtime state remains `NOT_IMPLEMENTED` / `NOT_PROVEN` even after this contract is accepted.

## 21. Supersession

A successor must identify this contract explicitly, preserve historical provenance, state the exact clauses changed, provide evidence satisfying the architecture decision discipline, and update compatible producer/consumer evidence accordingly.

Do not rewrite this accepted historical revision to make a later decision appear retroactive.
