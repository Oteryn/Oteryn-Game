# Foundation338 owning unexpected-control-loss implementation addendum

This is an exact implementation allocation addendum to the existing338 plan, not a new architecture decision. Governing authority: accepted `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` §§5–8/12–16, existing Foundation338 allocation and Work162. Concrete dependency: B329 finding5559409968; raw PREPARE cannot prove owning unexpected loss and must not originate continuity.

## Custody and dependency order

After protected allocation readback, same338 writer/branch/PR retains immutable admission4f35ec5a56f5e8b0c32db4503d2bd3503b8828ee, current bounded window and all counters. Existing six paths stay allocated; add only `apps/game-server/src/foundation/control_loss_durability_tests.rs`. Implement in already-owned `apps/game-server/src/foundation/admission_recovery_inner.rs` and include the new test file there. No additional module/facade/export path is needed for this semantic delivery. The worker updates its own existing task/plan acceptance at its next checkpoint. No new writer or history reset.

Qualify current post-grace claims/full flow alongside this additive boundary on the same branch. Protect the Foundation delivery before any separate B/SQL consumer integration; actual owning runtime source registration remains separately allocated. Independently testable semantic completion does not release Server Seam247.

## Required implementation

1. Implement a distinct split-phase durable unexpected-control-loss operation before reconnect PREPARE. PREPARE, fresh login, raw reconnect record, active-session snapshot, fresh receipt, socket close, peer report, graceful logout, healthy migration and process restart alone cannot authorize loss.
2. A crate-sealed owning runtime source independently resolves real unexpected playable-control loss and current session/actor/controller/runtime-owner fences. Foundation alone constructs private live authorization. Bind exact session, actor/account/character/world, observed authenticated transport and connection generation, runtime scope/ownership fence, and owner-issued loss epoch/origin, original grace, protection consumption and re-arm continuity. Do not derive authoritative facts from the request under validation.
3. Expose a pure bounded final validation predicate for a later durable adapter, requiring independently current fenced facts. Reject superseded ownership/generation or regained healthy controller authority. Return only the exact bound loss effect: same canonical session becomes reconnectable, only its exact controller binding is removed, genuine continuity retained. Account/Character holders, lease, provenance and protection history remain unchanged; loss neither acquires nor releases claims.
4. Retain complete immutable original operation for exact retry/reconciliation. Public historical data and receipts cannot reconstruct live authorization. Exact committed retry returns the original disposition with no second epoch, grace extension or protection reset. Ambiguity retains original binding. Completion/reconciliation cannot alter authority using stale historical facts; current-source resolution remains independent.
5. Preserve existing V1/V2 representations and behavior. No public caller flag, permissive source constructor, record-derived current-authority helper or receipt-to-live conversion. Follow accepted resource bounds; no invented numeric liveness or stable-control threshold. A future actual producer must satisfy registered policy before runtime activation.

## Qualification

Use independent owning-source fixtures and compile-fail checks for external seal implementation and raw/receipt construction. Demonstrate genuine first loss from fresh-origin continuity and later genuine loss after valid resumed control; one epoch per continuous loss; preserved original grace and consumed protection/re-arm history. Reject healthy controller, wrong/stale transport or generation, replaced runtime owner, wrong actor/account/character/world, terminal/superseded session, and source state changed between authorization and final validation. Exercise socket-only/restart-only/raw record/fresh receipt absence of authority, exact retry versus conflicting immutable operation, lost completion and historical restoration without live capability. Verify unchanged claims/provenance and current supersession blocking stale completion/reconciliation effects.

Run focused and affected Foundation tests, compile-fail tests, formatting and strict applicable linting. Require genuinely independent exact-head full-change review, canonical CI and normal protected Merge Queue/readback. Record concrete RED/GREEN evidence and preserve cumulative failures/repairs. Local skipped SQL tests provide no database qualification.

## Exclusions and later integration

No SQL, migration, shared harness, facade/lib/module export, Cargo, registry, workflow, actual source/transport/bootstrap/secret/live data, Platform/Atlas/META or deployment writes. Any concrete additional caller bridge must receive a separate exact allocation. PostgreSQL atomic mutation/current fences, bounded executor custody, owning runtime source registration and actual recovery/adoption remain later qualification obligations. B initial NULL continuity PREPARE remains closed until protected owning-loss semantics and its separately authorized durable adapter are available.

Single next action: Work independently qualifies and protects this allocation, then binds its readback to the existing338 writer.
