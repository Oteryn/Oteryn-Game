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
base_sha: 568236c33cd23da017bca1dbd1ed98afc8da71f4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-13T00:30:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation architecture and remaining paper-only/vertical-slice gates; executable packages remain separately bounded.
owned_paths: []
public_contracts:
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/architecture/README.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
handover_report: docs/agents/reports/OTV2-20260812-foundation-handover.md
depends_on:
  - accepted ADR-0001 through ADR-0016 as applicable to their named scopes
  - FND-01 and VSL-02 accepted/applied
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, DUR-04, ANL-01 and NET-TRANSPORT-01 accepted/lifecycle-closed after recorded closeouts
  - GAME-VISION-01, GAME-CHANNEL-01, GAME-CHAR-01 and GAME-ITEM-01 accepted/lifecycle-closed after recorded closeouts
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Maintain a truthful **non-owning** programme checkpoint. This record owns no architecture path and grants no implementation authority. Every substantial future gate still requires its own bounded task, branch, PR, validation and archive lifecycle.

This refresh is part of DUR-04 lifecycle closeout PR #213 and becomes canonical only after that closeout merges.

## Canonical continuation order

Use, in order:

1. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current DecisionStatus/DeliveryStatus/ImplementationStatus and selected successor;
2. accepted ADRs/contracts/owner baselines and machine-readable registries for semantic authority;
3. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` for stable gate definitions;
4. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` for global/gameplay staged ownership;
5. `docs/agents/reports/OTV2-20260812-foundation-handover.md` for the durable successor packet;
6. live GitHub main/PR/task/CI state for execution truth.

Older progress prose that conflicts with a later accepted contract or the current-status overlay is historical.

## Accepted/lifecycle-closed architecture to consume

Do not restart these gates merely because older backlog or predecision text describes an earlier state:

- native Rust workspace/client cutover and one project-owned `protocol-oteryn`;
- FND-ID-01 identity vocabulary;
- FND-02 protocol/CommandRef sequencing/reconciliation;
- FND-03 one-writer runtime ownership/generation/asynchronous-work boundary;
- FND-04 GameSession/CharacterLease/admission/recovery fencing;
- DUR-01 durable identifier representation;
- DUR-02 common Persistence-v1 migration/transaction/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit identity, durable evidence, privacy and read-only investigation boundary;
- GAME-VISION-01 product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR-01 Stage A/B semantic closure with unresolved Reference behavior remaining fail-closed;
- GAME-ITEM-01 typed item definition/instance/equipment/container/revision semantics;
- DUR-03 item/currency/value location, transaction, conservation and anti-duplication semantics;
- GAME-CHANNEL-01 selection/queue/co-location/anti-hopping/multiplicity/qualitative lifecycle/community policy;
- DUR-04 stable content/package identity, deterministic locked compilation, immutable bundle staging/activation/migration, bounded loader/provenance and authoritative scripting capability/determinism boundary from delivery PR #212 exact final head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23`, squash merge `568236c33cd23da017bca1dbd1ed98afc8da71f4`.

## Accepted DUR-04 boundary — consume, do not reopen

DUR-04 freezes architecture for:

- typed semantic content graph independent from YAML/RON/JSON5/custom physical serialization;
- stable `PackageKey`, immutable `PackageRevision`, stable `ContentKey`, exact immutable Content Lock and revision-scoped compact IDs;
- deterministic source/import -> typed model -> validation -> dependency/alias resolution -> normalization -> lowering -> client/server projection -> immutable bundle -> staging -> explicit activation;
- allowlisted client-safe projection with server-only authoritative data excluded;
- fail-closed loader validation with checked allocation/decompression/integrity/version/dependency/index/semantic validation and no partial authoritative publication;
- explicit content activation/rollback and durable-state migration classes;
- exact external-source provenance and `COPY|CONVERT|REWRITE|REFERENCE_ONLY|REJECT` dispositions with importer-boundary LIR;
- target WebAssembly Component Model + project-owned versioned WIT capability ABI, with Wasmtime only an implementation candidate;
- no ambient filesystem/network/process/environment/SQL/global-Game authority for scripts;
- snapshot-bound authoritative reads and proposal-only extension-state/domain mutations;
- no ActionPlan-created cross-owner atomicity; wider workflows retain owning OperationId/idempotency/compensation semantics;
- deterministic logical time, invocation-local RNG, stable query order, deterministic numeric/NaN policy, fuel/resource bounds and `script_execution_profile_revision`;
- typed/versioned bounded durable extension state, never VM memory persistence;
- explicit GAME-CHANNEL multiplicity/eligibility classification for relevant value-producing sources;
- Resource Limits Registry completeness before implementation acceptance;
- a bounded reversible physical-format/compiler/loader spike before final serializer/container/chunk/floor/compression choices.

DUR-04 intentionally does **not** freeze the authoring serializer, final bundle container, 32x32 versus 64x64 chunking, floor packing, compression codec, exact WIT function inventory, exact Wasmtime version/features or numeric resource ceilings.

## Implementation boundary

Architecture acceptance grants **no executable authority**. A future task may not create or claim Rust GameNode/Channel/item/content runtime, compiler/loader/Studio/scripting host, WIT implementation, PostgreSQL DDL/migrations, Platform/Gateway/World Registry changes, live item/currency/content/Channel mutation, broad legacy content import, production traffic/configuration or entitlement activation without separate explicit owner implementation authority and its own evidence.

## Current ordered paper-only architecture work

After DUR-04 lifecycle closeout, `SIM-DETERMINISM-01` and the Reference evidence/parity manifest remain independently ownable pre-VSL paper-only work.

To preserve singular ownership, the selected next action is exactly:

```text
SIM-DETERMINISM-01 — Authoritative Simulation Determinism Contract
```

The bounded gate should define only architecture needed now for authoritative arithmetic representation, rounding and overflow, deterministic RNG ownership/streams, simulation logical time and ordering, replay inputs, state hashing/divergence evidence, supported-target determinism and the relationship to DUR-04 `script_execution_profile_revision`. It must not implement combat, AI, scripts or runtime.

Reference evidence/parity tooling remains independently ownable and is not implicitly accepted by selecting SIM as next.

## Repository and production authority

Routine writes remain limited to `blakinio/Oteryn-v2`. External repositories remain read-only unless explicitly authorized for an exact write task.

No production deployment, protected-environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

## Context checkpoint

```yaml
last_progress: DUR-04 delivery PR #212 passed owner-directed exact-head self-review `4921665072` under the explicit owner review override, repair budget 3/3 and exact-head Governance/Dependency/CodeQL, then squash-merged as `568236c33cd23da017bca1dbd1ed98afc8da71f4`; lifecycle closeout #213 reconciles canonical status and releases DUR-04 ownership.
status: ready
branch: null
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: From live main after DUR-04 lifecycle closeout, create one bounded paper-only `SIM-DETERMINISM-01` architecture task; do not implement runtime/combat/AI/scripts/DDL/production behavior.
```
