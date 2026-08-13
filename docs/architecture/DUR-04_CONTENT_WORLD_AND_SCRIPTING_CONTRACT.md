# DUR-04 — Content, World Detail and Scripting Contract

- Status: `PROPOSED / IN_REVIEW / NOT_STARTED`
- Date: 2026-08-12
- Gate: `DUR-04`
- Scope: paper-only architecture contract.
- Runtime implementation authority: **NONE**.
- DDL/migration authority: **NONE**.
- Production activation authority: **NONE**.

## 1. Contract purpose

This contract completes the architecture intentionally left open by ADR-0005 for native content packaging, deterministic compilation, immutable World Bundle activation, migration/provenance and bounded scripted behavior.

It does not replace ADR-0005. It makes ADR-0005 implementable without freezing physical file encodings that still require the mandated spike/benchmark evidence.

## 2. Normative language and fail-closed rule

`MUST`, `MUST NOT`, `SHOULD` and `MAY` are normative.

Unknown physical-format details or numeric limits MUST fail closed at implementation review rather than be inferred as unlimited.

## 3. Canonical semantic identity

Every package MUST have a stable namespaced `PackageKey`, immutable `PackageRevision`, semantic schema version, declared dependencies, provenance/licensing metadata and source-manifest digest.

A runtime build MUST use a deterministic **Content Lock** resolving every dependency to an exact immutable package revision/digest. Runtime dependency resolution MUST NOT use floating versions, mutable branches or network-time “latest” selection.

Every canonical content definition MUST have one stable namespaced `ContentKey`. Legacy numeric IDs and compact runtime IDs MUST NOT become canonical semantic identity. Compiled numeric IDs MAY exist only within one compiled revision/artifact.

Aliases/deprecations MUST be explicit, deterministic and acyclic. Ambiguous or cyclic alias resolution MUST fail compilation.

## 4. Revision domains

The architecture MUST keep these concepts distinct:

- `content_revision`;
- `map_revision` where separately tracked;
- `ruleset_revision`;
- `world_policy_revision`;
- compiler identity/canonicalization profile;
- `script_execution_profile_revision` for authoritative script-engine semantics and deterministic execution configuration;
- exact bundle artifact digest.

A release/deployment record MAY bind them together, but one opaque revision number MUST NOT replace their semantic distinction.

`script_execution_profile_revision` MUST identify the authoritative execution semantics that can affect script outcome, including the selected engine/runtime version or compatibility identity, enabled Wasm features relevant to determinism, fuel operator-cost policy, floating/NaN policy, relaxed-SIMD policy where enabled, memory/table growth policy and other determinism-sensitive runtime configuration. It MUST NOT become the public WIT ABI identity and MUST NOT expose Wasmtime-specific types to scripts.

## 5. Source-model boundary

The canonical Oteryn content model is a typed semantic graph independent of its concrete serializer.

DUR-04 therefore does **not** accept YAML, RON, JSON5, a custom text syntax, `.omap`, `.owb` or any specific binary container as the permanent canonical physical encoding.

Any final physical source/bundle encoding MUST be selected only after the ADR-0005 bounded non-canonical spike proves deterministic hashes, random access, corruption/decompression handling, source-to-bundle-to-load equivalence and representative chunk/floor-packing behavior.

## 6. Deterministic compiler pipeline

The logical pipeline MUST be:

```text
source packages
 -> parse/import
 -> typed canonical model
 -> schema/semantic validation
 -> dependency/key/alias resolution
 -> migration normalization
 -> deterministic lowering
 -> client/server view projection
 -> deterministic artifact assembly
 -> integrity digests
 -> immutable artifact
 -> isolated staging verification
 -> explicit activation
```

For identical source bytes, Content Lock, compiler identity, canonicalization profile and target capability/profile inputs, compilation MUST produce the same logical output and exact artifact digest.

The compiler MUST either canonicalize or reject nondeterminism from filesystem order, unordered collections, locale/timezone, wall clock, machine paths, uncontrolled randomness, duplicate-key precedence and unstable serialization behavior.

## 7. Compile-time fail-closed rules

Compilation MUST fail on at least:

- duplicate canonical content keys;
- ambiguous/cyclic aliases;
- unresolved references;
- incompatible dependency/schema/runtime capability requirements;
- unsupported critical fields;
- invalid spatial references;
- script references lacking declared required capabilities;
- client/server visibility violations;
- relevant value-producing source/encounter definitions lacking explicit GAME-CHANNEL multiplicity and durable eligibility classification;
- missing mandatory resource-limit definitions required by the selected implementation format/runtime.

## 8. Client-safe versus server-authoritative projection

Client artifacts MUST be produced by explicit allowlisted projection from the locked semantic graph.

Server-only authoritative fields MUST NOT be serialized into client artifacts and then merely hidden by UI logic.

Client and server artifacts MUST carry compatible revision/capability metadata so incompatible pairs fail closed. Client data MUST remain non-authoritative even when it mirrors server definitions.

## 9. Immutable World Bundle

A published World Bundle or equivalent runtime artifact MUST be immutable and content-addressed by integrity digest.

It MUST identify at least format/schema version, content/map/ruleset/world-policy compatibility where applicable, compiler identity, Content Lock digest, artifact/manifest digest, required runtime/protocol capabilities, required WIT world/interface compatibility, `script_execution_profile_revision` where authoritative scripts are present, section/chunk integrity metadata, provenance summary and projection class.

Runtime MUST NOT deserialize unstable Rust memory layout as a public format contract.

## 10. Loader and staging safety

The loader MUST treat every artifact as untrusted until validation completes.

Before authoritative publication it MUST perform, in bounded order:

1. minimal header/metadata validation;
2. size/count/decompression-ratio checks before untrusted-size allocation;
3. checked arithmetic for offsets/counts/byte totals;
4. format/schema/runtime capability validation;
5. manifest/section integrity verification;
6. dependency/lock verification;
7. bounded index/section validation;
8. semantic consistency validation;
9. isolated staging construction.

No partially validated section MAY become authoritative runtime state. Unknown critical sections or required capabilities MUST fail closed.

All externally controlled counts, lengths, depths, compressed/decompressed sizes, ratios, memory/table counts and equivalent resources MUST have entries in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` before implementation acceptance.

## 11. Staging, activation and rollback

Loading and activation MUST be separate operations.

A staged revision MAY be parsed, verified, indexed and prewarmed without becoming authoritative.

Activation MUST bind an exact coherent revision/artifact set. A new authoritative simulation scope MUST NOT begin with a partial mixture of old/new definitions caused by incremental mutation. A currently active immutable revision MUST NOT be edited in place.

A later rollout contract MAY permit old and new compatible scopes to coexist temporarily, but every authoritative scope MUST identify the exact revision set governing it.

Rollback means activation of a previously verified compatible artifact. Rollback MUST NOT restore older content while leaving authoritative durable state in a semantically incompatible post-migration representation.

If activation requires durable migration, forward and rollback compatibility MUST be proven before production activation.

## 12. Durable-state migration classes

Every content change affecting durable semantic interpretation MUST be classified as exactly one of:

- `COMPATIBLE_NO_MIGRATION`;
- `READ_COMPATIBLE_NORMALIZE`;
- `EXPLICIT_DATA_MIGRATION`;
- `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION`;
- `REMOVED_WITH_EXPLICIT_POLICY`.

`READ_COMPATIBLE_NORMALIZE` MUST be deterministic and MUST NOT create/destroy player value or bypass domain validation.

`EXPLICIT_DATA_MIGRATION` MUST execute through accepted persistence/domain transaction ownership, never direct script/database mutation.

Player-value-bearing removals MUST NOT silently discard, reinterpret or duplicate durable value.

## 13. Legacy conversion and provenance

Each external source area used for migration/evidence MUST record exact source revision/archive digest, license/provenance status, one disposition (`COPY`, `CONVERT`, `REWRITE`, `REFERENCE_ONLY`, `REJECT`), importer identity, deterministic conversion report, unresolved/lossy-semantics report and legacy-ID mappings where relevant.

Legacy Intermediate Representation MUST remain an importer-boundary type and MUST NOT become canonical Oteryn content schema.

Otheryn/Canary/other OTS sources MUST NOT by themselves establish Global Tibia parity truth.

## 14. Scripting architecture

The target scripting boundary MUST use WebAssembly Component Model components with a **project-owned, versioned WIT capability ABI**, unless a later accepted ADR explicitly supersedes this decision.

Wasmtime is the initial implementation candidate, but Wasmtime APIs/types MUST NOT define the public Oteryn script contract.

A process-global Lua/game-object model is NOT accepted as the target authoritative architecture. A bounded compatibility adapter MAY be considered only in a separate migration contract.

The exact WIT package/world/interface revision required by a script component MUST be explicit and resolved before activation. WIT compatibility MUST NOT be inferred from mutable source paths or a host accepting “whatever imports happen to link.”

## 15. Script authority model

A script MUST be treated as a bounded decision/proposal component, not as an authoritative mutation owner:

```text
InvocationContext + bounded immutable read snapshot + explicit capabilities
 -> script component
 -> ProposedActionPlan / typed result
 -> host/domain validation
 -> accepted authoritative command/transaction
```

A script MUST NOT directly commit PostgreSQL transactions, mutate arbitrary server/domain objects, bypass DUR-03 conservation/idempotency/fencing, bypass GAME-ITEM legality, bypass GAME-CHANNEL multiplicity/eligibility or mint trusted audit/economy truth outside the authoritative mutation boundary.

All authoritative reads exposed during one invocation MUST be bound to the invocation's declared snapshot/revision context. A script MUST NOT observe a mixture of pre- and post-mutation live state because a host query happened later in the same invocation.

## 16. Capability security

Capabilities MUST be explicit versioned WIT imports.

The default authoritative script environment MUST provide no ambient filesystem, network/socket, process spawning, environment variable/secret access, unrestricted wall clock, OS randomness, direct SQL, global mutable Game object or mutable Rust/server object reference.

Capability absence is the default denial mechanism.

Candidate capability families MAY include bounded snapshot reads, scoped snapshot-bound spatial queries, deterministic simulation time/tick, deterministic invocation-local RNG, proposal of domain actions, bounded diagnostics and typed extension-state **read** access.

Authoritative extension-state writes MUST NOT be direct capability side effects. They MUST be represented as typed proposed mutations in the returned action/result plan and committed only after host/domain validation under the owning durable transaction/workflow.

Every capability call MUST remain subject to host/domain authorization and resource bounds.

## 17. Deterministic script execution

Authoritative script execution MUST use deterministic inputs/imports and an exact `script_execution_profile_revision`.

Deterministic semantic execution MUST use:

- simulation logical time instead of wall clock;
- host-supplied deterministic invocation-local RNG bound to accepted simulation/replay and invocation identity rather than a mutable global RNG stream;
- deterministic fuel accounting under the selected execution profile;
- bounded memory/table/instance resources;
- bounded host-call count;
- bounded query/result collection sizes;
- bounded action-plan size;
- deterministic error/trap mapping.

Host APIs returning collections or candidate sets MUST define stable ordering or canonicalize the result before exposing it to a script. A script MUST NOT observe hash-map iteration order, database default row order, scheduler order or another unspecified ordering source as gameplay input.

Authoritative WIT/domain interfaces SHOULD prefer integer, enum and project-owned fixed-point/decimal semantics where practical. If floating-point values are permitted in authoritative script logic, the selected execution profile MUST define and the implementation MUST prove deterministic floating behavior, including NaN canonicalization where observable; platform-dependent NaN payloads or unspecified floating serialization MUST NOT influence authoritative outcome.

Fuel exhaustion MUST terminate an invocation without committing its proposed action.

A fuel number is meaningful only together with the exact `script_execution_profile_revision`; engine/runtime upgrades or operator-cost-policy changes MUST NOT silently reinterpret an existing content revision's authoritative fuel budget. An incompatible execution profile requires explicit compatibility/migration/revision handling before activation.

Epoch/wall-clock interruption MAY exist as a secondary operational kill switch but MUST NOT define authoritative gameplay outcome or deterministic replay semantics.

Unconstrained memory/table growth is forbidden. Authoritative components MUST declare/enforce deterministic memory/table constraints, and implementation acceptance MUST prove that allocation/growth outcomes do not become host-resource-dependent gameplay nondeterminism.

## 18. Script failure isolation and transaction scope

Required script validation/instantiation failure MUST block activation of affected required content. Missing required capability MUST fail activation or invocation closed.

On invocation trap, fuel exhaustion or invalid result:

- no proposed action from that invocation is committed;
- no proposed extension-state write becomes authoritative;
- diagnostics remain bounded;
- the host returns a deterministic domain-level failure category;
- server-only content/secrets MUST NOT leak to clients.

An action plan containing any unauthorized/invalid authoritative action MUST be rejected atomically by default. Partial-plan semantics require a separate domain-specific accepted contract.

`ProposedActionPlan` MUST NOT become a generic cross-authority transaction escape hatch. Every plan type MUST declare the authoritative domain/transaction scope in which its all-or-nothing validation/commit semantics are valid. If a requested plan would require atomicity broader than an accepted owner/transaction boundary, the host MUST reject it or route it through a separately accepted multi-transaction workflow/orchestrator using the owning domain's OperationId/idempotency/compensation semantics. A script MUST NOT create distributed atomicity by returning a larger plan.

Repeated failures MAY feed a circuit-breaker/disable mechanism, but failover MUST NOT silently substitute different reward/quest/economy semantics.

## 19. Script persistent state

VM linear memory/table state MUST NOT be treated as durable gameplay persistence.

Persistent script-owned extension state MUST be typed, namespaced, schema-versioned, size-bounded, read through the invocation's bounded snapshot and persisted/fenced/migrated through accepted DUR-02/DUR-03 ownership.

A script MAY propose a typed extension-state mutation, but that proposal MUST remain non-authoritative until host/domain validation and commit. It MUST participate in the owning accepted transaction/workflow semantics, including idempotency and audit where applicable. A trap, fuel exhaustion, invalid plan or rejected transaction MUST leave authoritative extension state unchanged.

Opaque arbitrary binary blobs are forbidden for authoritative durable state unless a later accepted contract defines their schema/version/limit/migration/audit semantics.

## 20. Hot reload semantics

“Hot reload” MUST NOT mean mutating a live immutable revision in place.

Permitted conceptual workflow:

```text
compile new revision
 -> stage
 -> validate/prewarm
 -> explicit activation for eligible scopes
 -> drain/restart/migrate older scopes under accepted rollout policy
```

Every authoritative scope MUST be able to report which exact content/script artifact revision, WIT ABI requirement and `script_execution_profile_revision` governed it for replay, diagnostics and audit.

## 21. Package/runtime supply-chain rules

Production runtime MUST NOT fetch unresolved content packages from the network. Activation MUST use a reviewed immutable lock and verified artifact digests. Script component digests, exact WIT package/world/interface requirements and the required `script_execution_profile_revision` MUST be bound into the content revision/manifest. Unknown script imports/capabilities MUST fail closed.

Release signing/trust-root/CDN policy is deferred, but integrity hashes are mandatory now. Integrity digests alone MUST NOT be treated as proof of publisher authenticity; production trust/authenticity remains gated by the later release/signing contract.

## 22. Resource limits

DUR-04 deliberately does not invent numeric ceilings.

Before any parser/compiler/loader/script runtime is accepted, `RESOURCE_LIMITS_REGISTRY.json` MUST contain applicable hard maxima and boundary tests at least for source/package bytes and counts, dependency/key/reference counts and nesting, bundle/section/chunk compressed and decompressed bytes and ratio, indexes/densities where allocation risk exists, script component bytes, instance count, memory/table constraints, fuel, host calls, query/result/action-plan bounds and persistent extension-state bytes.

Absent required limits block implementation acceptance.

## 23. Required implementation evidence

A future implementation MUST prove at minimum:

1. identical locked source/compiler inputs produce identical artifact digest;
2. shuffled source enumeration does not change output;
3. duplicate key/ambiguous alias/unresolved dependency fails compilation;
4. client projection contains no server-only fixture;
5. corrupt or oversized/decompression-bomb fixture fails before activation/unbounded allocation;
6. unsupported required capability/schema/WIT requirement fails closed;
7. mandatory resource-limit registry is complete;
8. infinite-loop script terminates by deterministic fuel with zero committed mutation;
9. forbidden filesystem/network/wall-clock capability is inaccessible;
10. deterministic RNG/replay yields identical proposed action plan without consuming mutable global RNG state;
11. host-query result order is deterministic under intentionally shuffled underlying storage/enumeration;
12. one invocation cannot observe state changes outside its declared snapshot/revision context through later host queries;
13. authoritative floating-point fixture either uses accepted deterministic canonicalization or is rejected by policy, with identical replay outcome across supported targets;
14. the same script/input/fuel under the same `script_execution_profile_revision` yields the same exhaustion/completion outcome across supported targets;
15. incompatible script execution profile or WIT ABI requirement is rejected before activation rather than silently reinterpreting the content revision;
16. a script that proposes an extension-state write and then traps leaves authoritative extension state unchanged;
17. invalid action plan is rejected before mutation;
18. a plan requesting atomicity wider than its accepted owner/transaction boundary is rejected or routed through an explicitly accepted workflow rather than committed as one implicit transaction;
19. migration-required content cannot activate before migration proof;
20. incompatible rollback is rejected;
21. relevant channel-sensitive reward/encounter source lacking multiplicity/eligibility metadata fails compile;
22. incompatible content revisions cannot silently coexist under one claimed homogeneous world/channel revision set;
23. legacy conversion output/report is reproducible from exact source/importer revisions.

## 24. Physical-format spike gate

Before final World Project/World Bundle encoding is accepted, a bounded reversible spike MUST compare candidate encodings against deterministic byte identity, source-control diff/review behavior, random access, corruption detection, decompression/resource safety, source-to-bundle-to-load equivalence, editor save/recovery requirements, representative world scale, 32x32 versus 64x64 candidate chunking/floor packing, patch/download granularity and client/server load locality.

Spike artifacts MUST remain non-canonical and non-production until a subsequent accepted contract freezes the encoding.

## 25. Non-authority statements

Acceptance of DUR-04 does NOT authorize production compiler/loader/scripting crates, adding Wasmtime or scripting dependencies, WIT implementation files, PostgreSQL migrations, broad content import, production activation, Oteryn Studio implementation, final physical encoding, numeric limits, production signing/CDN or exact gameplay/NPC/quest/AI behavior.

## 26. Lifecycle rule

This document is a candidate while its delivery PR is open.

`DUR-04` becomes `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` only after:

1. this delivery candidate is reviewed and merged;
2. a separate lifecycle-closeout change reconciles maintained programme status/register/horizon/index/handoff and archives/releases the active task.

No downstream implementation or broad content-import authority may treat an open delivery PR as accepted architecture.
