# Oteryn G4 multi-source identity binding decision

- Date: 2026-09-24
- Programme: Full Tibia Content Census / G4+
- Status: owner-directed architecture addendum; protected integration required
- Protected baseline inspected: `main@88efe885c1b9ff08dc3a33e061d9a63474d67824`
- Scope: identity/provenance crosswalk only; no runtime/wire ID allocation and no asset redistribution

## Decision

G4+ must preserve **external source identities as first-class crosswalk/provenance**, while keeping canonical Oteryn identity independent from every wiki, OTS, client or appearance numbering scheme.

The invariant is:

`canonical Oteryn identity != legacy/source numeric id != presentation/asset identity != runtime/wire compact id`.

A legacy/source ID is useful and must not be discarded, but it can never mint or replace the canonical Oteryn definition identity.

## Problem

Current protected WorldProject/v2 already provides:

- canonical definition identity as `(family, namespaced key, exact revision)`;
- dataset-level provenance through `ProjectV2Source` / `provenance/sources.json`;
- typed candidate `SourceId(u64)` observations;
- separate `Presentation -> Asset` binding and immutable asset identity/digest;
- family-specific crosswalk tooling/evidence.

That is enough to retain source facts, but it does **not** yet provide one generic first-class relation that says:

`this exact external entity identity from this exact source revision maps to this exact canonical Oteryn definition`.

Without that relation, mass G4 population could later lose exact comparability between Oteryn, TibiaWiki, Canary, Crystal and client/appearance identifiers, or force expensive rematching by names and fields.

## Identity layers

Keep these layers separate.

### 1. Canonical Oteryn definition identity

Authoritative Oteryn definition identity remains:

`family + ProductionKey + DefinitionRevisionRef`.

Examples:

- `Item / oteryn:item.dragon_scale_mail / definition-rN`;
- `Creature / oteryn:creature.rat / definition-rN`;
- `Presentation / oteryn:presentation.dragon_scale_mail.default / definition-rN`.

External numbering never participates in minting this key.

### 2. Source dataset identity

`ProjectV2Source` continues to bind the exact source dataset/import to:

- source key;
- exact source revision;
- source artifact digest;
- import batch;
- evidence class.

This identifies **which source snapshot** produced an observation.

### 3. Source entity identity binding

G4 requires a generic source-entity binding concept with this logical shape:

```text
target:
  family: <Oteryn family>
  key: <canonical Oteryn ProductionKey>
  revision: <exact Oteryn definition revision>

source:
  key: <exact ProjectV2Source key>
  revision: <exact source revision>

identity_namespace: <source-local id kind>
external_id: <verbatim source id>
binding_disposition: EXACT | ACCEPTED_ALIAS
```

Examples of `identity_namespace`:

- `mediawiki/page_id`;
- `ots/item_server_id`;
- `ots/item_client_id`;
- `ots/creature_id`;
- `client/appearance_id`;
- `client/object_id`;
- other bounded source-local identifiers proven necessary by a family importer.

`external_id` should be retained in the source's exact lexical representation. Do not coerce unrelated source systems into one shared integer namespace.

### 4. Presentation and asset identity

Rendering identity remains separate:

`definition -> Presentation -> Asset`.

A client/appearance/sprite identifier is therefore not automatically an Item identity.

Examples:

- OTS `server item id` may bind to an `Item` definition;
- client `appearance_id` should normally bind to `Presentation`, not directly become the Item key;
- asset digest/revision belongs to the Asset layer;
- changing a sprite/appearance must not remint the canonical Item/Creature/WorldObject definition.

### 5. Runtime/wire identity

Any compact runtime/wire identifier generated for a particular artifact, protocol revision or client projection is a delivery concern. It must not be reused as canonical source identity and is outside this decision.

## Binding rules

1. **No bare generic `id` in new G4 crosswalk contracts.** Every external identifier must be interpreted with its exact source and identifier namespace.
2. The comparison key is at least:
   `(source key, source revision, identity namespace, external id)`.
3. The same numeric value in TibiaWiki, Canary, Crystal or a client means nothing by itself.
4. Many distinct source identities may bind to one canonical Oteryn definition.
5. One exact source identity at one exact source revision may bind canonically to at most one Oteryn target. Competing targets remain `CONFLICT`.
6. `PROBABLE_MATCH`, `AMBIGUOUS`, `CONFLICT` and `NO_MATCH` stay crosswalk/evidence states and must not become canonical source bindings.
7. An explicitly proven duplicate/alias may become `ACCEPTED_ALIAS` without changing canonical Oteryn identity.
8. If a source changes its numeric ID between revisions, preserve the historical source identity and add the new revision binding; do not remint the Oteryn target.
9. Names are discovery/corroboration signals, not identity authority by themselves.
10. Source IDs must remain available in retained compact evidence even when the corresponding source payload remains artifact-only.
11. Placement/occurrence IDs, when present, identify source occurrences and must not silently become reusable definition identity.
12. Copyright/licence boundaries remain binding; retaining an identifier/provenance tuple does not authorize redistributing proprietary assets or source payloads.

## G4 promotion discipline

For each family batch:

1. collect/reuse exact source identities;
2. resolve the canonical Oteryn target using multi-signal evidence;
3. emit the crosswalk state;
4. promote a source-entity binding only for `EXACT` / explicitly accepted alias cases;
5. verify fields independently from identity;
6. populate canonical Oteryn definitions/relationships only with eligible verified data;
7. compile/test the affected content path.

Identity match and gameplay-field truth remain separate. A proven source-ID binding does not prove every field from that source.

## Current schema gap

The logical contract above is required now, because G4 mass population and later source-drift comparison depend on retaining exact external identities.

The **physical representation is not selected by this document**.

Current v2 gives us dataset provenance and candidate `SourceId` values, but no generic first-class `target <-> exact source entity identity` record. Before family-wide canonical population is committed at scale, the active G4 owner must do one bounded gap-closure step:

- either prove that an already accepted representation can express the complete contract above without ambiguity; or
- add the minimum sufficient typed v2 source-identity binding representation and tests.

Do not create `ProjectV3`, `ItemV2` or another global identity system for this gap.

Pilot/read-only G4 analysis may proceed immediately when its artifacts preserve the full tuple above. Mass canonical population must not discard source IDs while waiting for the physical carrier decision.

## Comparison use cases this enables

A later tool can deterministically answer:

- which Oteryn Item corresponds to TibiaWiki page X;
- which Crystal/Canary server IDs refer to the same canonical Item;
- whether two donors reuse the same numeric ID for different entities;
- whether a donor changed an ID between revisions;
- whether wiki has a new entity with no donor/Oteryn match;
- whether one Oteryn definition has conflicting donor field values;
- which Presentation/Asset corresponds to a client appearance ID;
- whether an Oteryn record drifted from a newer structured wiki snapshot.

This is also the basis for the future deferred live-Global verifier: live observations may attach to canonical identities without becoming the identity system themselves.

## Decision timing

**Must decide now: YES.**

### Concrete downstream work blocked

G4+ family crosswalk and canonical population need a durable rule for retaining source IDs before thousands of entities are promoted.

### What becomes harder later

If source IDs are dropped during population, later Oteryn↔wiki↔OTS comparison requires heuristic rematching by names/fields and cannot reliably detect source-ID churn or numeric collisions.

### Evidence that may supersede this decision

- a protected existing carrier is proven to satisfy every invariant above without extension;
- a real G4 family corpus proves the logical tuple is insufficient;
- a source system exposes a stronger immutable identity that requires a typed extension;
- measured client/protocol needs justify a separate delivery mapping while preserving this source/canonical separation.

### Deliberately not decided

- runtime compact-ID allocation;
- protocol wire IDs;
- client cache indexing;
- asset container/texture packing;
- World Bundle physical format;
- authenticated Global Tibia automation;
- a new project/schema major version.

## Acceptance

G4 identity handling is compliant when:

- canonical Oteryn identity never comes from a legacy/source numeric ID;
- exact source IDs remain recoverable with source/revision provenance;
- no cross-source numeric-ID equality is inferred;
- exact accepted mappings are separate from ambiguous/conflicting evidence;
- Presentation/Asset identity remains separate from Item/Creature/WorldObject identity;
- later comparison tooling can join Oteryn, wiki and donor records without name-only rematching.
