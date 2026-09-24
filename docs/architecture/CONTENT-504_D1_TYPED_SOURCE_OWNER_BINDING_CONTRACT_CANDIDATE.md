# CONTENT-504 D1 — Typed Source / Owner Binding Contract Candidate

- Date: 2026-09-17
- Parent: `#504`; control plane: `#162`
- Status: `CANDIDATE / OWNER_ACCEPTANCE_REQUIRED`
- Protected baseline: `main@b44fefe08f6aaf1b2c1c23dedd92bab0de87146e`
- Delivery: PR `#641`, branch `agent/content-world-design-dossier-20260917`
- Allocation: `#162` comment `5718999869`
- Implementation/runtime/registry/production authority: **NONE**
- Merge authority: **REPOSITORY_CONTROL_PLANE_ONLY**

## Purpose

Freeze only the smallest semantic producer-consumer delta needed before `#504` implementation. Consume protected `#511` Phase-A/#525 as migration/source evidence, `#483` evidence classification, `#486` Reference-first discipline, accepted ADR-0005/Stage-C boundaries and the current protected Content API. PR #641 revision-2 remains noncanonical supporting input.

Phase A is not rerun. OTS/migration geometry is not promoted to Global target truth.

## Selected contract

### Typed definition identity

Every successor reference is semantically:

```text
TypedDefinitionRef = (DefinitionFamily, ProductionKey, DefinitionRevisionRef)
```

`DefinitionFamily` is explicit; a key from another family does not satisfy the reference. `ProductionKey` keeps the existing namespaced-key semantics. `DefinitionRevisionRef` binds the exact immutable definition context; compatibility is never inferred from the key alone. Runtime compact IDs remain revision-local.

### Provenance and evidence

Do not create a second provenance system. A definition resolves through its exact package/content revision into existing `PackageManifestBinding` and `ContentLockBinding` provenance.

Target-sensitive fields additionally carry an `EvidenceBindingRef` into the existing Reference evidence authority. The concrete evidence-manifest schema remains owned elsewhere. `UNKNOWN`, `CONFLICT` and `OTS_HYPOTHESIS_ONLY` cannot silently become executable Reference truth.

### Placement identity and order

```text
PlacementRef = (PlacementKey, exact_map_revision, TypedDefinitionRef)
OrderedPlacements = (cell_or_field_ref, ordered sequence<PlacementRef>)
```

`PlacementKey` is stable semantic identity and is independent of coordinates, file path, source shard, chunk/section address, runtime sector, enumeration index and presentation ordinal. Ordered presentation is semantic data when qualified, but it is not placement identity and does not automatically define movement, use/look or item-custody precedence.

### Coordinates and footprints

A spatial address is semantically:

```text
SpatialAddress = (WorldId, CoordinateFrameRef, discrete logical cell coordinates)
```

`CoordinateFrameRef` prevents migration/OTS coordinates, controlled observations and canonical Oteryn target coordinates from being silently treated as the same frame.

D1 requires separate `PresentationFootprint` and `CollisionFootprint` relations. Presentation coverage never becomes Movement authority. Missing required footprint evidence is unresolved, not implicitly empty/walkable.

### Immutable Content vs mutable overlay

Content owns immutable definitions and authored placements for one exact generation. It may define finite state/transition vocabulary but not current live state.

Mutable state resolves through:

```text
RuntimeOverlayTarget = (RuntimeScopeRef, active_content_generation, PlacementKey)
```

Current `ChannelRuntime`/`InstanceRuntime` ownership remains unchanged. Cache reload, reimport or client disconnect cannot reset live scope state. Content chunks do not define mutation ownership.

### Current-owner capability / transition

A transition is semantically:

```text
TransitionBinding = (
  TransitionKey,
  definition_ref,
  source_state,
  normalized_intent_or_trigger_family,
  target_state,
  OwnerCapabilityRequirement,
  typed policy/guard refs
)
```

`OwnerCapabilityRequirement` means the already-selected current owner must support the operation. Authored content cannot select a new owner or mint authority. Movement/runtime composition is not implemented by this decision.

### Item/progression/value boundary

Content stores immutable definition references only. Item/progression/formula/loot/value-source references use the same typed-definition shape. They never create or own `ItemInstanceId`, current item location/custody, quantity/charges/durability, current XP, balances, mint/transfer/pickup transaction state or conservation authority. Static item placement is not a durable ItemInstance.

### Client-safe projection

Server-authoritative and client-safe views come from one locked semantic graph. Client projection is allowlisted and non-authoritative. It cannot own mutable gameplay state, movement/collision legality, transition guards, loot/value/RNG truth, durable item legality/custody or evidence acceptance.

## Profile compatibility

`FIRST_PRODUCTION_CONTENT_PROFILE/v1` remains unchanged. Do not widen or reinterpret it.

`REFERENCE_PLAYABLE_CONTENT_PROFILE/v1` is an explicit successor capability profile using the same production safety architecture plus required successor semantics.

```text
FIRST_PRODUCTION_CONTENT_PROFILE/v1 artifact
  != REFERENCE_PLAYABLE_CONTENT_PROFILE/v1 artifact
```

Cross-profile loading/activation must fail closed. Existing `FirstProductionContentSource` and `compile_first_production` behavior remains backward-compatible.

## Current protected API reused unchanged

Reuse rather than duplicate:

- `ProductionKey`;
- `ProductionAtom`;
- `Sha256HexDigest`;
- `PackageManifestBinding`;
- `ContentLockEntry` / `ContentLockBinding`;
- Foundation `WorldId`;
- existing content/map/ruleset/world-policy/compiler/canonicalization revision separation;
- current deterministic compile/projection discipline;
- current staging/activation exact-generation safety;
- existing first-production source/compiler semantics.

## Missing successor semantics

Implementation needs semantic equivalents of:

1. `DefinitionFamily` and `DefinitionRevisionRef`;
2. family-checked `TypedDefinitionRef`;
3. typed `EvidenceBindingRef`;
4. `PlacementKey` / `PlacementRef`;
5. `CoordinateFrameRef`;
6. ordered field/cell -> placement relation;
7. separate presentation/collision footprint relations;
8. finite state and `TransitionKey`/binding where exercised;
9. `OwnerCapabilityRequirement`;
10. fail-closed `REFERENCE_PLAYABLE_CONTENT_PROFILE/v1` profile/capability identity;
11. Reference-required effect vocabulary including typed Heal in addition to the current Damage-only Content effect family.

## `DEFERRED_REQUIRES_PHASE_B`

The following target values remain deferred:

- canonical Newhaven/Targuna coordinates and source->target transforms;
- exact target floor mapping where not independently established;
- exact ordered placement sequence for corridor cells;
- exact target collision/walkability;
- exact target presentation/collision footprint members;
- exact geometry-dependent floor-transition/local-relocation endpoints;
- geometry-derived target resource lower bounds not established by Phase A.

The type shapes above are not blocked by Phase B.

## Rejected alternatives

- widen first-production v1 in place — rejected: silent historical reinterpretation risk;
- wait for Phase B before all D1 decisions — rejected: identity/ownership/evidence boundaries are geometry-independent;
- use OTS geometry provisionally as executable target geometry — rejected by Reference evidence discipline;
- one generic untyped key/value record — rejected: loses family/compatibility checking;
- coordinates or ordered index as placement identity — rejected: couples identity to representation;
- store current state in Content — rejected: creates a second mutable authority;
- authored content selects owner — rejected: capability requirement is not authority;
- freeze serializer/compression/chunking now — rejected: not required to state this contract.

## Later mandatory tests

Successor implementation must prove at least:

1. wrong-family reference fails closed;
2. stale/incompatible definition revision fails;
3. provenance/Content-Lock mismatch fails;
4. required target field without accepted evidence fails Reference promotion;
5. `UNKNOWN`/`CONFLICT`/OTS-only claims do not promote;
6. duplicate `PlacementKey` fails;
7. source reorder/chunk repack does not change placement identity;
8. ordered placement output is deterministic;
9. presentation order cannot change collision/movement authority or item custody;
10. coordinate-frame mismatch fails;
11. presentation and collision footprint semantics remain independent;
12. mutable overlay does not leak between runtime scopes;
13. incompatible generation cannot reinterpret live overlay state;
14. unsupported owner capability fails before mutation;
15. content cannot mint a second owner;
16. static/item definition refs cannot create durable item/value state outside GAME-ITEM/DUR;
17. client projection does not leak forbidden server-only authority fields;
18. client-supplied state/collision result cannot become authoritative;
19. first-production and Reference artifacts reject cross-profile activation;
20. existing first-production regression behavior remains unchanged;
21. equivalent locked inputs remain deterministic under shuffled enumeration;
22. later registered resource maxima receive max/max+1 and overflow/bounds tests.

Phase-B geometry tests become mandatory only when those target values are admitted.

## `must decide now`

**YES now:** typed definition identity; revision/provenance/evidence binding; placement identity/order; coordinate-frame separation; presentation/collision footprint separation; immutable Content vs mutable overlay; owner capability/transition reference shape; item/progression/value definition-only boundary; client authority boundary; first-production/successor compatibility.

**Blocked without D1:** `#504` cannot safely allocate the shared successor model/compiler because importers and build code would otherwise invent incompatible source, placement, evidence and ownership semantics.

**Harder later:** imported data/artifacts could embed cross-family ambiguity, coordinate-derived identity, OTS-as-target assumptions, first-production reinterpretation, duplicate state owners or static-to-durable value coupling.

**Superseding evidence:** a later accepted owner contract, security/compatibility finding, representative successor implementation proving the relation insufficient, or Phase-B evidence requiring a stronger spatial semantic relation.

## Explicit non-decisions

No decision here selects `.omap/.owb`, JSON/JSONL/YAML/RON, FlatBuffers, Zstd, chunk/floor packing, source-shard layout, production maxima, target coordinates, unproved stack/collision truth, full-world import, Movement/runtime/Server-Seam implementation, protocol IDs/payloads, Studio/renderer technology, broad NPC/quest/economy/house/event content, scripting VM, deployment or production activation.

## Owner acceptance request

This candidate is ready for `#504` / repository owner acceptance. Acceptance freezes only this semantic D1 boundary and grants no implementation/runtime/registry/merge/production authority.

`TERMINAL_RECOMMENDATION: READY_FOR_504_D1_OWNER_ACCEPTANCE`
`PHASE_B_DEPENDENT_VALUES: DEFERRED_REQUIRES_PHASE_B`
`FIRST_PRODUCTION_V1_REINTERPRETATION: FORBIDDEN`
