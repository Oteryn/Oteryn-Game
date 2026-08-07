# OTV2-20260805-foundation-preimplementation-contracts

```yaml
task_id: OTV2-20260805-foundation-preimplementation-contracts
title: Coordinate Oteryn v2 foundation contracts and staged implementation gates
mode: COORDINATE
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
pr: null
base_sha: 283fceeecc55c85f8b0d34459732f27c74a77de7
head_sha: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-07T09:17:00+02:00
execution_budget_minutes: 120
large_budget_reason: The programme checkpoint spans repository ownership, protocol, runtime, persistence, content, Platform integration, client migration and the global product architecture horizon, while each executable package remains separately bounded.
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/architecture/ADR-0008-protocol-canary-reference-only-migration-disposition.md
  - docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
  - docs/architecture/ADR-0010-reference-and-evolved-world-product-profiles.md
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
depends_on:
  - ADR-0001 through ADR-0011 accepted foundation
  - FND-01 and VSL-02 accepted and archived
  - atomic destination cutover merged as 78988f72a80cc904aa9176ae850c50d4efa0b0f0
  - source-only blakinio/otclient marker merged as 8c56c45c6c25147470ce3ca23e639a31d9085e47
  - source-marker lifecycle archive merged as 26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda
blocks:
  - protocol and Game Session identifier meanings until FND-ID-01 is accepted
  - canonical protocol implementation until FND-02 is accepted
  - authoritative runtime implementation until FND-03 is accepted
  - production admission and character lease implementation until FND-04 is accepted
  - authoritative durable gameplay mutation until DUR-01 through DUR-03 are accepted
  - final transactional event/outbox/audit boundaries until ANL-01 is accepted
  - production-grade balance/world and economy/security analytics claims until ANL-02 and ANL-03 are accepted
  - read-only AI investigation until ANL-04 is accepted
  - broad content import and durable scripting until DUR-04 is accepted
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Coordinate the minimum contracts and staged implementation gates required for the Oteryn v2 Rust workspace and first durable vertical slice without locking the project into an incorrect repository boundary, dependency graph, protocol, runtime topology, admission model, persistence design, world/content model or client migration path.

This is a **non-owning programme checkpoint**. It preserves current accepted state and exactly one next action; it does not implement all gates or own their files. Every substantial gate requires its own bounded task, branch, PR, validation, audit, merge and archive lifecycle.

## Canonical continuation sources

Read these in this order for foundation continuation:

1. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` — canonical current execution status and completed cross-repository closeouts;
2. accepted ADRs and dedicated owner baselines/contracts — architecture semantics;
3. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` — stable ordered gate definitions;
4. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` — complete staged architecture horizon;
5. `docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` and `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` — unresolved coverage;
6. this programme checkpoint — current coordination and one next action;
7. live GitHub branch/PR/CI state — final truth for execution status.

The original 2026-08-05 handoff remains preserved unchanged at `docs/agents/evidence/OTV2-20260805-foundation-original-handoff.md` for traceability.

Long-lived ADR/register/baseline text may contain historical progress sentences. For progression status, later exact evidence and `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` supersede stale `source marker pending` wording without silently changing the architecture decisions around it.

## Current repository and migration state

### PROVEN

- Oteryn-v2 `main` at this reconciliation baseline is `283fceeecc55c85f8b0d34459732f27c74a77de7`.
- `FND-01` and `VSL-02` are accepted and archived.
- PR #50 delivered the accepted 19-member canonical Rust workspace and squash-merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.
- The native Rust client is canonical in `blakinio/Oteryn-v2` and remains in ADR-0011 `pre-native-protocol` until later gates authorize real native gameplay capability.
- The destination production graph contains neither `protocol-canary` nor a speculative production `protocol-oteryn` adapter.
- `blakinio/otclient#274` marked `oteryn-client/**` historical/non-canonical and squash-merged as `8c56c45c6c25147470ce3ca23e639a31d9085e47` after exact-head Rust Client and repository CI passed.
- `blakinio/otclient#275` archived that source-marker task and squash-merged as `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`; source ownership is released.
- The source-marker prerequisite that previously blocked `FND-ID-01` is therefore satisfied.
- No complete `protocol-oteryn` Rust client-to-server gameplay implementation or native gameplay E2E is proven by the migration or marker work.
- Oteryn Platform remains the owner of Identity, OAuth/PKCE, Game Login Ticket, World Registry and the initial Go Game Gateway.

Every replacement agent must verify this baseline against live default branches before repeating it.

## Accepted foundation — do not silently redesign

The following directions are accepted and require a superseding owner-approved ADR to change materially:

- native Rust client and authoritative Rust game server;
- one project-owned gameplay protocol family: `protocol-oteryn`;
- multichannel-first logical world architecture with one logical authoritative mutation owner per channel;
- `blakinio/Oteryn-v2` as canonical Rust gameplay repository;
- `protocol-canary = REFERENCE_ONLY`, absent from production dependencies, negotiation, fallback and translation;
- Platform Identity/Game Gateway/World Registry as the external authentication/routing control plane;
- PostgreSQL target with separate Platform/game logical database ownership;
- project-owned native world/content model and integrated Oteryn Studio direction;
- Game Intelligence with separate operational observability, best-effort telemetry and durable economy/security audit classes;
- native three-tier E2E platform and exact evidence discipline;
- GameNode process/container/runtime and measured-capacity/recovery baseline;
- reference and evolved world profiles over one engine/client/protocol, with gameplay value isolated by logical world by default;
- fail-closed `pre-native-protocol` migrated-client state;
- accepted owner baselines for identifier semantics, UUIDv7 durable identity direction, world-scoped instance ownership/admission and privacy-first social presence.

## Ordered foundation state

### Completed

- `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract: **ACCEPTED AND APPLIED**.
- `VSL-02` — Exact Rust Client Migration and Cutover Contract: **ACCEPTED; DESTINATION AND SOURCE CLOSEOUT COMPLETE**.
- Atomic Oteryn-v2 destination cutover: **COMPLETE** at `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.
- Source-only historical/non-canonical marker: **COMPLETE** at `8c56c45c6c25147470ce3ca23e639a31d9085e47`.
- Source-marker lifecycle archive: **COMPLETE** at `26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda`.

### Next ordered gate

`FND-ID-01` — Foundation Identifier Vocabulary.

Its start condition is satisfied. Existing owner-accepted baselines are mandatory inputs but do not equal the completed contract. The dedicated `FND-ID-01` package must freeze the minimum cross-boundary identifier catalogue, owner/issuer, semantic scope, uniqueness/reuse/lifecycle, visibility/privacy, comparison/validation and minimum cross-language/wire encoding constraints without prematurely freezing the full PostgreSQL physical layout owned by `DUR-01`.

### Subsequent layer gates

- `FND-02` — `protocol-oteryn` v1: canonical transport/framing/schema/limits/revisions/capabilities/sequencing/snapshots/deltas/reconnect/golden-fixture contract.
- `FND-03` — Runtime Execution: runtime ownership, clocks/ticks, ordering, bounded queues, overload, parallel work return, lifecycle/checkpoint/recovery.
- `FND-04` — Identity, Game Session, Admission and Character Lease: session/token/admission/lease/fencing/reconnect/failure semantics.

These are separate gates. Completing `FND-ID-01` does not automatically complete or authorize `FND-02` through `FND-04`.

## Required before authoritative durable gameplay

The full definitions and ordering remain in `FOUNDATION_DECISION_BACKLOG.md` and the global register. At minimum:

- `DUR-01` — durable identifier representation and migration;
- `DUR-02` — Persistence v1 on PostgreSQL;
- `DUR-03` — item transaction and anti-duplication invariants;
- `DUR-04` — world/content detail and scripting before broad import/durable scripts;
- `ANL-01` — event/audit foundation before persistence/item contracts finalize transactional evidence;
- `GAME-CHAR-01` before final durable character schema;
- `GAME-ITEM-01` before final item transaction model.

No authoritative character, item or currency mutation is authorized merely because the workspace exists.

## Vertical-slice and later programme gates

`VSL-01` must eventually prove the complete native supported path with named E2E evidence, including Platform admission, Rust server entry, movement/visibility, combat/loot, retry-safe item mutation, durable state, analytics/audit evidence, safe logout/relogin, duplicate-session rejection and channel-local/world-shared isolation.

Later product/gameplay/operations gates remain registered rather than implicitly accepted, including movement, combat/abilities, character progression, items, AI, interactions, quests/content, rulesets, native client UX, LiveOps, compatibility/release train, capacity, orchestration/recovery, security, privacy, GM/support, accessibility/localization, social/economy/houses/events/instances/world lifecycle, creator tooling, integrations and deferred ecosystem/business decisions.

`GAME-VISION-01` analysis may proceed in parallel with `FND-ID-01` when it does not redefine accepted foundation boundaries.

## Package ownership model

- This programme checkpoint remains unassigned and owns no paths.
- Each substantial gate uses a separate task with its stable gate ID.
- Each package owns only declared paths/contracts and uses one dedicated branch/PR.
- Completion requires proportional focused checks, independent audit, exact-head CI, applicable E2E, squash merge, archive and ownership release.
- A package may not expand authority through governance edited on its own unmerged branch.
- Cross-repository writes require explicit authorization for each exact repository and one task/branch/PR per repository under the coordinated rollout order.

## Acceptance criteria

- [x] Repository ownership and early client migration direction accepted.
- [x] `FND-01` accepted and applied to the destination workspace.
- [x] `VSL-02` accepted with exact source, provenance, path mapping, rollback and one atomic destination cutover.
- [x] Atomic Oteryn-v2 destination migration/workspace PR merged and verified.
- [x] `blakinio/otclient` source-only marker merged and verified without destination implementation.
- [x] Source-marker task archived and source ownership released.
- [x] The source-marker prerequisite for `FND-ID-01` is satisfied.
- [ ] Complete `FND-ID-01` contract accepted before dependent protocol/admission identity meanings are frozen.
- [ ] `FND-02` accepted before canonical native protocol implementation/compatibility claims.
- [ ] `FND-03` accepted before authoritative runtime implementation claims.
- [ ] `FND-04` accepted before production Game Session admission/character lease behavior.
- [x] PostgreSQL and separate Platform/game ownership accepted in ADR-0004.
- [x] Native world/content/editor direction accepted in ADR-0005.
- [x] Game Intelligence direction accepted in ADR-0006.
- [x] Native E2E platform direction accepted in ADR-0007.
- [ ] `DUR-01` through `DUR-03` accepted before authoritative durable character/item/currency mutation.
- [ ] `ANL-01` accepted before final transactional outbox/audit boundaries are frozen.
- [ ] `DUR-04` accepted before broad content import/durable scripting.
- [ ] `VSL-01` completed with named E2E evidence before the native foundation slice is called complete.
- [ ] Every later contract names authority, producers/consumers, versions, failure semantics, limits and validation evidence appropriate to its boundary.

## Excluded scope

This programme checkpoint does not itself:

- implement or complete `FND-ID-01` or any later gate;
- create additional speculative workspace crates;
- implement `protocol-oteryn`;
- implement authoritative game runtime, admission, leases or persistence;
- modify external repositories;
- migrate Platform persistence;
- provision production infrastructure or databases;
- import proprietary assets/content;
- select final house topology or deferred product systems;
- claim native gameplay E2E or production readiness.

## Validation

This non-owning programme checkpoint is coordination state, not a substitute for package validation. Every package records its own exact-head evidence.

- current source-marker evidence: `PASS` through `blakinio/otclient#274` and `#275` as recorded in `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
- component/integration: `NOT_APPLICABLE` for this coordination checkpoint itself;
- E2E: `BLOCKED` until the relevant implementation packages and `VSL-01` exist;
- future package CI/audit: mandatory per package.

## Context checkpoint

```yaml
last_progress: The atomic destination cutover is complete; blakinio/otclient source marker #274 merged as 8c56c45c6c25147470ce3ca23e639a31d9085e47 and archive #275 merged as 26f7646ea26b27c9ac4bf617b8cb0d63c89bdfda, satisfying the final VSL-02 source-closeout prerequisite.
status: ready
branch: null
head_sha: null
pr: null
owned_paths: []
public_contracts:
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/architecture/ADR-0008-protocol-canary-reference-only-migration-disposition.md
  - docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
  - docs/architecture/ADR-0010-reference-and-evolved-world-product-profiles.md
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
validation_state: Destination workspace and source-only cutover closeout are complete and verified; every future gate still requires its own exact-head validation, audit and applicable E2E evidence.
audit_state: Source-marker closeout is complete; future gate audits pending.
e2e_state: BLOCKED until VSL-01 implementation; migration/marker completion does not prove native gameplay.
ci_generation: null
run_ids: []
counters:
  ci_checks_for_current_head: 0
  terminal_ci_checks_for_current_generation: 0
  unchanged_state_checks: 0
  identical_failure_retries: 0
  repair_cycles_for_current_gate: 0
  stall_warnings: 0
blocker: null
next_action: Create and execute one bounded FND-ID-01 contract package using all owner-accepted identifier baselines as mandatory inputs.
```
