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
base_sha: 63380bcba469027e90677aaf4db571fa941be2f4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-12T16:16:00+02:00
execution_budget_minutes: 120
large_budget_reason: Non-owning programme checkpoint spanning accepted native foundation architecture and the remaining gameplay/durability/vertical-slice gates; executable packages remain separately bounded.
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
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, ANL-01 and NET-TRANSPORT-01 accepted/lifecycle-closed after their recorded closeouts
  - GAME-VISION-01, GAME-CHAR-01 and GAME-ITEM-01 accepted/lifecycle-closed
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Maintain a truthful **non-owning** programme checkpoint for Oteryn-v2. It coordinates accepted architecture and names exactly one next safe package without implementing gates, reserving their paths or treating architecture acceptance as runtime completion.

Every substantial architecture or implementation gate still requires its own bounded task, branch, PR, validation, review policy, merge and archive lifecycle.

This refresh is part of DUR-03 lifecycle closeout PR #208 and becomes canonical only when that closeout merges.

## Canonical continuation order

Use these sources in this order:

1. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` — current DecisionStatus/DeliveryStatus/ImplementationStatus truth;
2. accepted ADRs/contracts/owner baselines and exact registries — semantic authority;
3. `docs/architecture/FOUNDATION_DECISION_BACKLOG.md` — stable gate definitions/dependencies;
4. `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` — global staged horizon;
5. `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` — gameplay/product ownership and dependencies;
6. `docs/agents/reports/OTV2-20260812-foundation-handover.md` — current successor handoff;
7. this checkpoint — non-owning programme summary;
8. live GitHub branch/PR/CI/task ownership — execution truth.

Older progress prose that conflicts with current status or later accepted contracts is historical, not executable instruction.

## PROVEN accepted/lifecycle-closed architecture

Consume rather than redesign:

- canonical native Rust workspace/client cutover and one project-owned `protocol-oteryn`;
- FND-ID-01 typed identity vocabulary;
- FND-02 protocol/CommandRef sequencing/reconciliation;
- FND-03 one-writer runtime ownership, generations and asynchronous external-work boundary;
- FND-04 GameSession/CharacterLease/admission/recovery fencing;
- DUR-01 durable identifier representation including non-reused UUIDv7 ItemInstanceId;
- DUR-02 common Persistence-v1 transaction/migration/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit identity, durable evidence, privacy and read-only replay/investigation boundary;
- GAME-VISION-01 minimum product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR-01 Stage A + Stage B semantic closure with unresolved exact Reference behavior still fail-closed;
- GAME-ITEM-01 typed item definition/instance/equipment/container/revision semantics;
- DUR-03 item/currency/value transaction, conservation and anti-duplication semantics from delivery PR #207, frozen head `a1d949362e219373a5d314c0e9ddf8de110362dd`, squash merge `63380bcba469027e90677aaf4db571fa941be2f4`.

Do not restart these gates merely because older backlog prose describes an earlier progression state.

## Accepted DUR-03 boundary — consume, do not reopen

`DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` freezes architecture-level invariants for:

- exactly one typed immediate semantic location per live durable ItemInstance;
- runtime ground/checkpoint projection versus durable recoverability without dual authority;
- non-blocking runtime reservation -> asynchronous game-DB durable linearization -> normalized completion/recovery for pickup/drop and other mixed runtime/durable value operations;
- fresh transaction-scoped ItemInstanceId for new concrete lifecycles, explicit split/merge survivor/retirement and explicit internal transform preserve/replace identity policy;
- exact conservation classes `TRANSFER`, `SPLIT_MERGE_QUANTITY`, `STATE_MUTATION`, `MINT`, `BURN`, `TRANSFORM`, `CONVERSION` with complete source/sink/input/output lineage rather than market-price equality;
- CommandRef/OperationId/TransactionId separation, durable receipts, known-abort retry and ambiguous-commit candidate freeze/reconciliation;
- current GameSession/CharacterLease/runtime ownership fencing;
- bounded participant/evidence sets, anomaly-proof transaction obligations and typed multi-transaction custody;
- world-scope/cross-database fail-closed rules;
- bounded ANL-compatible durable evidence where owning security/value policy requires it;
- restore/recovery validation and stale checkpoint/ground ghost suppression;
- read-only Game Intelligence investigation and no automatic repair authority.

DUR-03 does **not** own trade consent, market/order-book policy, bank/depot/mail lifecycle, reward eligibility, houses, crafting/ruleset formulas, entitlement activation or exact unevidenced Reference behavior.

## Implementation boundary

Architecture acceptance still grants **no executable authority**. No task may create or claim:

- Rust GameNode/item transaction runtime;
- protocol gameplay payload implementation;
- PostgreSQL DDL/migration files or migration execution;
- live item/currency/value mutation;
- production traffic/deployment/configuration;
- Premium/VIP or other entitlement activation;

without a separate explicit owner implementation authorization and its own bounded task/evidence.

A future authorized server/persistence programme may consume the accepted contracts through bounded real-boundary slices. The existing safe decomposition hypothesis remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content slices
```

## Current ordered architecture work

After DUR-03 lifecycle closeout, the earliest still-unresolved gate in the owner-accepted recommended ordering is:

1. `GAME-CHANNEL-01` — bounded paper-only channel product/policy architecture covering player choice/assignment, co-location, queues/visibility, switching/anti-hopping, spawn/resource multiplication, world-global event/reward eligibility, PvP implications, social fragmentation and same-channel recovery safety.

Independent paper-only work may proceed under separate ownership for Reference evidence/parity tooling, `DUR-04` minimum headless content path and `SIM-DETERMINISM-01`. None of those gates is implicitly accepted by DUR-03.

QUIC remains a later evidence/profile-reconciliation gate and is not the current priority.

## Repository and production authority

Routine writes remain limited to `blakinio/Oteryn-v2`. External repositories remain read-only unless the owner separately authorizes an exact write task.

This checkpoint grants no production deployment, protected-environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying.

## Context checkpoint

```yaml
last_progress: DUR-03 delivery PR #207 passed terminal exact-head self-review, genuinely independent no-suggestion Codex review and Agent Governance/Dependency Review/CodeQL, then squash-merged unchanged as 63380bcba469027e90677aaf4db571fa941be2f4; lifecycle closeout #208 reconciles canonical status/handoff and releases DUR-03 ownership.
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
next_action: From live main after DUR-03 lifecycle closeout, create one bounded paper-only `GAME-CHANNEL-01` architecture task consuming accepted multichannel/runtime/product/value boundaries; do not implement runtime/DDL/production behavior.
```
