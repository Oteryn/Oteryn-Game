# Canonical WorldId boundary for production CONTENT

- Date: 2026-09-09
- Tracking: #491
- Parent repair: #490
- Repair PR: #492
- Source merge requiring repair: #481
- Protected base used for this decision: `main@ece300c384aa1e53f208975f25e94635c2fcc7ce`
- Decision: `FOUNDATION_WORLD_ID_CANONICAL_AT_CROSS_BOUNDARY/v1`
- Status on this branch: `CANDIDATE / PENDING_PROTECTED_ACCEPTANCE`
- Runtime implementation authority granted by this document: **NONE**
- Live deployment / production-environment activation authority: **NONE**

## 1. Problem

Protected Game code currently contains two nominal Rust types named `WorldId`:

- `crate::domain::WorldId` in `apps/game-server/src/domain/mod.rs`;
- `crate::foundation::WorldId` in `apps/game-server/src/foundation/protocol.rs`.

Both validate a non-nil RFC-variant UUIDv7 and preserve all 128 bits, but FND-ID-01 defines one semantic cross-boundary `WorldId`, issued by Platform World Registry / the authoritative topology-control boundary. Equal physical representation does not make separate nominal types separate canonical identities or implicitly interchangeable types.

Merged #481 bound first-production CONTENT to `crate::domain::WorldId`. #490 therefore cannot truthfully close P1-2 until the canonical CONTENT/runtime boundary is explicit.

## 2. Constraints

Binding constraints are:

1. `WorldId` is one strongly typed UUIDv7, full 128 bits.
2. Platform World Registry / authoritative topology control remains the semantic issuer/owner; Game consumes assigned identity and does not mint a competing namespace.
3. `WorldId` and `ChannelId` remain distinct semantic types.
4. No raw string, generic UUID, display name, slug, ordinal or implementation-local integer may replace canonical `WorldId` at a typed cross-boundary API.
5. All 128 bits must round-trip losslessly.
6. Identity equality is not freshness, authority, ordering or fencing evidence.
7. #490 is a bounded post-merge CONTENT repair; it must not become a broad Foundation/domain identifier refactor.
8. No Platform mutation, protocol redesign, persistence migration, registry change or live activation is authorized here.

## 3. Options considered

### A. Foundation canonical — selected

Use existing `crate::foundation::WorldId` as the canonical Rust type at production CONTENT/runtime/admission/durability cross-boundaries.

### B. Domain canonical + Foundation adapter — rejected for this boundary

Declaring `crate::domain::WorldId` canonical would require a new accepted adapter contract and would invert the already-protected admission, recovery and durability usage of the Foundation type. It adds migration/bridge work without protecting a stronger invariant.

### C. Immediate single-type consolidation — deferred

Removing the duplicate nominal type across all domain consumers can be desirable, but it is broader than #490 and would expand paths, review surface and regression risk beyond the repair needed to restore first-production CONTENT acceptance.

## 4. Decision

**Option A is selected.**

`crate::foundation::WorldId` is the canonical Game Rust representation of the Platform-issued logical-world identity when `WorldId` crosses a production subsystem boundary, including:

- admission and recovery;
- runtime scope and authority-bearing Foundation APIs;
- durability/persistence adapters that bind world identity;
- production CONTENT source identity;
- production CONTENT artifact metadata and staged/active generation identity;
- first-production restart/LKG expectation identity;
- later composition comparisons between CONTENT and authoritative runtime scope.

This choice does not transfer semantic ownership to the Foundation module. Platform World Registry / authoritative topology control remains the issuer and lifecycle authority. The Foundation type is the canonical Game-side typed representation consumed at these cross-boundary surfaces.

## 5. Exact #490 / #492 implementation consequence

After this decision is protected, the existing #490/#492 repair lineage may replace the first-production use of `crate::domain::WorldId` with existing public `crate::foundation::WorldId` **without mutating Foundation or domain source files**.

The bounded repair may update only already-owned CONTENT/test surfaces necessary to apply the type change, expected to include:

- `apps/game-server/src/content/production.rs`;
- `apps/game-server/tests/content_first_production.rs`;
- the already-owned #490 P1-1 surfaces where compilation requires corresponding import/test adjustments.

No Cargo/lockfile, Foundation implementation, domain implementation, registry, protocol, DDL or Platform path is implied by this decision.

Required behavior:

1. `FirstProductionContentSource.world_id` uses `crate::foundation::WorldId`.
2. Production artifact metadata and `GenerationIdentity` preserve that same nominal type.
3. Restart/LKG expectation reconstruction preserves that same nominal type.
4. The temporary first-production carrier may serialize the exact 16 UUID bytes deterministically under its own versioned internal representation, but decoding must reconstruct through the Foundation `WorldId` validator and preserve all 128 bits.
5. The carrier encoding remains internal to `FIRST_PRODUCTION_CONTENT_PROFILE/v1`; it does not become the permanent FND-02 wire or World Bundle textual format.
6. `WorldId` comparison at composition boundaries is typed `foundation::WorldId` equality only; bytes/string labels are not accepted as an alternate identity API.

## 6. Domain WorldId disposition

`crate::domain::WorldId` is **not** a second cross-boundary world identity namespace.

For this bounded decision it may remain temporarily in existing GAME-CHAR/GAME-ITEM/domain-local code so #490 does not become a broad refactor. New production cross-boundary APIs must not adopt it as an alternative canonical `WorldId`.

This decision deliberately does **not** add blanket `From`/`Into` conversions between `domain::WorldId` and `foundation::WorldId`. Such a generic conversion would make accidental cross-context substitution easy and would obscure which boundary performed validation.

If a real pre-consolidation consumer later needs to cross between those bounded contexts, it requires one explicit named, lossless and type-directed bridge at the owning boundary, with all 16 bytes preserved and the destination validator applied. That bridge must be separately allocated if its paths are not already owned.

A later small consolidation may remove the duplicate domain nominal type and converge internal domain code on the canonical type when evidence shows the migration surface is bounded. That cleanup is not a prerequisite for #490.

## 7. Required repair evidence

The #492 P1-2 repair must prove at least:

- a public production CONTENT source requires `crate::foundation::WorldId`;
- a valid UUIDv7 round-trips source -> artifact -> staging -> `GenerationIdentity` without changing any of the 128 bits;
- nil, wrong-version, wrong-variant and malformed carrier representations fail closed;
- restart/LKG expectation reconstruction retains the same canonical `WorldId`;
- no raw string/generic UUID overload is added as a convenience path;
- where practical, compile-time/API evidence prevents passing `crate::domain::WorldId` directly to the production CONTENT boundary;
- all existing #490 P1-1 staged-generation ownership regressions remain green.

## 8. Trade-offs and risks

### Benefits

- aligns CONTENT with the already-protected admission/recovery/durability cross-boundary type;
- removes the immediate nominal-identity split at the first-production composition boundary;
- requires no Foundation/domain mutation for #490;
- avoids a third type or generic UUID/string bridge;
- keeps the repair small enough for focused independent review.

### Costs

- the duplicate `domain::WorldId` remains temporarily inside domain-local code;
- a later domain consolidation may still be required;
- callers that currently construct domain IDs for CONTENT tests must switch to the Foundation constructor/decoder.

### Main risk

The remaining duplicate type could be mistaken later for another canonical namespace. This is controlled by explicitly classifying it as domain-local/non-canonical for new cross-boundary APIs and by prohibiting blanket implicit conversions.

## 9. Decision timing

### Must decide now?

**YES.** #491 blocks P1-2 of #490, which blocks terminal first-production CONTENT acceptance in #54.

### Concrete downstream work blocked

The existing Draft PR #492 cannot apply the canonical P1-2 repair or enter terminal review until the cross-boundary type is resolved.

### What becomes harder if chosen incorrectly?

Keeping two peer canonical types would permit authority/content/runtime comparisons to depend on ad-hoc byte casts, create accidental second-namespace semantics and spread conversion code across later Movement/Combat/Server composition.

### Evidence that could justify supersession

A reviewed shared-identity-module consolidation, a Foundation/domain boundary refactor proving one different canonical location, or an FND-ID/FND-02 supersession that deliberately relocates the canonical type while preserving semantic ownership and lossless UUIDv7 identity.

### Deliberately not decided

- timing/path of full domain `WorldId` consolidation;
- permanent shared identity-module layout;
- Platform-side UUIDv7 rollout/migration details;
- FND-02 gameplay wire encoding beyond existing protected contracts;
- permanent World Project/World Bundle representation;
- database schema changes;
- any live deployment or activation policy.

## 10. Lifecycle / authority

This architecture record grants no direct runtime write, merge bypass or live deployment authority.

After protected acceptance and protected-main readback, #162 may resume the **existing** #490/#492 bounded repair lineage and apply only the exact CONTENT/test consequence described above. It must not create a second repair worker for the same paths.

Final #490/#54 acceptance still requires focused regressions, genuinely independent exact-head high-risk review with zero open material P0/P1/P2 findings, applicable repository CI/governance, normal FULL Merge Queue integration and protected-main readback.

`ARCHITECTURE_DECISION: FOUNDATION_WORLD_ID_CANONICAL_AT_CROSS_BOUNDARY`
`IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_DOCUMENT`
`RESUME_AUTHORITY_AFTER_PROTECTED_ACCEPTANCE: #162_EXISTING_#490/#492_LINEAGE_ONLY`
`LIVE_DEPLOYMENT_OR_PRODUCTION_ENVIRONMENT_ACTIVATION_AUTHORITY: NONE`
