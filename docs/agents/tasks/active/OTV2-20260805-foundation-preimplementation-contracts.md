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
base_sha: 1e16b32069868f14aa1761a512b6cd8b1024e277
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-13T09:24:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation/game/content/determinism architecture and remaining Reference/vertical-slice gates; executable packages remain separately bounded.
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
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, DUR-04, ANL-01, NET-TRANSPORT-01 and SIM-DETERMINISM-01 accepted/lifecycle-closed after recorded closeouts
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

This refresh is part of SIM-DETERMINISM lifecycle closeout PR #215 and becomes canonical only after that closeout merges.

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

Do not restart these gates merely because older backlog/predecision text describes an earlier state:

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
- DUR-04 typed semantic content graph, exact package locking, deterministic compilation, immutable bundle staging/activation/migration, bounded loader/provenance and capability-oriented deterministic scripting;
- SIM-DETERMINISM-01 deterministic arithmetic/RNG/order/replay/state-hash/supported-target architecture from delivery PR #214 exact final head `4c6684328123aebd657696808372a5855980d34e`, squash merge `1e16b32069868f14aa1761a512b6cd8b1024e277`.

## Accepted SIM-DETERMINISM boundary — consume, do not reopen

SIM-DETERMINISM freezes architecture for:

- reproducibility from canonical future-determining state + exact owner-local normalized input order + exact semantic revision/profile set + normalized external facts;
- FND-03 RuntimeExecutionOrdinal remaining owner-local authority/evidence with no global total order or second runtime commit ordinal;
- `SimulationDeterminismProfileRevision` for numeric/RNG/tie-break/hash/supported-target semantics without replacing content/ruleset/world-policy/DUR-04 script profiles;
- exact semantic revision binding for retryable/delayed occurrences, preventing silent reinterpretation under newer incompatible rules;
- explicit numeric semantic classes and formula descriptors with named rounding/invalid-state behavior;
- DUR-03 exact conservation remaining exact and non-floating;
- purpose-isolated deterministic gameplay RNG with retry/failover stability, authoritative stream advancement, no process-global mutable RNG and anti-prediction protection for exploit-sensitive seed/root state;
- separation of wall clock, monotonic elapsed time and authoritative execution order with no universal fixed tick;
- deterministic simultaneous/conflict policy using commutative semantics, stable tie-breaks or exact retained FND-03 order;
- typed normalization of external nondeterminism before it influences authoritative gameplay;
- replay envelopes retaining exact server/build executable identity, protocol revision/profile, World Bundle artifact/digest, semantic revisions, input/order evidence, formula/script profiles, RNG evidence and normalized external/time facts;
- optional NodeId/process-incarnation forensic attribution without making original placement a replay prerequisite;
- canonical deterministic state/hash coverage of active revisions, gameplay state, RNG state, pending accepted work, occurrence identities and semantically relevant fences/revisions;
- hierarchical first-divergence evidence that is read-only and cannot repair live authority;
- identical normalized authoritative results across supported server targets, with deterministic floating allowed only under explicit cross-target proof;
- explicit replay/hash/RNG/formula/pending-state resource limits before implementation acceptance.

SIM-DETERMINISM intentionally does **not** choose a Rust numeric/RNG/hash crate, exact gameplay RNG algorithm, exact fixed scale or gameplay formula, global tick rate, scheduler/thread counts, replay storage backend or production hash cadence.

## Implementation boundary

Architecture acceptance grants **no executable authority**. A future task may not create or claim Rust GameNode/Channel/item/content/SIM runtime, combat/AI/progression/scripts, compiler/loader/Studio/WIT host, PostgreSQL DDL/migrations, Platform/Gateway/World Registry changes, broad legacy content import, production traffic/configuration or entitlement activation without separate explicit owner implementation authority and its own evidence.

## Current ordered paper-only architecture work

After SIM-DETERMINISM lifecycle closeout, the remaining named pre-VSL paper-only programme action is:

```text
Build the versioned Reference evidence/parity manifest under its owning contract.
```

Do not invent a new stable gate ID unless the repository owner explicitly creates one. The bounded task must preserve the accepted first Reference target and evidence hierarchy, make unresolved behavior fail closed, record provenance/status per exercised mechanic and avoid promoting OTS implementations or search absence into Global truth.

That manifest work is architecture/evidence-only and does not authorize runtime implementation, DDL, production behavior, proprietary code/assets or any external-repository mutation.

## Repository and production authority

Routine writes remain limited to `blakinio/Oteryn-v2`. External repositories remain read-only unless explicitly authorized for an exact write task.

No production deployment, protected-environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

## Context checkpoint

```yaml
last_progress: SIM-DETERMINISM delivery PR #214 repaired the review P1 in owner-authorized cycle 4, passed exact-head self-review and Governance/Dependency/CodeQL, then squash-merged exact final head 4c6684328123aebd657696808372a5855980d34e as 1e16b32069868f14aa1761a512b6cd8b1024e277 after the owner explicitly overrode the fresh-independent-review-after-repair gate for that exact head; lifecycle closeout #215 reconciles canonical status and releases SIM ownership.
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
next_action: From live main after SIM-DETERMINISM lifecycle closeout, create one bounded paper-only task to build the versioned Reference evidence/parity manifest under its owning contract; do not invent a stable gate ID or implement runtime/DDL/production behavior.
```
