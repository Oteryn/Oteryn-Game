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
base_sha: 5c502d24557621efc798def87b68f137ba23fad8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-12T13:35:00+02:00
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
  - FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, ANL-01 and NET-TRANSPORT-01 accepted/lifecycle-closed
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

This checkpoint refresh is part of GAME-ITEM lifecycle closeout PR #206 and becomes canonical only when that closeout merges.

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

Older prose that conflicts with the current-status overlay or later accepted baselines is historical, not executable instruction.

## PROVEN accepted architecture

Accepted/lifecycle-closed architecture includes:

- canonical 19-member Rust workspace and client migration/cutover;
- `FND-ID-01` foundation identifiers;
- `FND-02` `protocol-oteryn` architecture;
- `FND-03` runtime execution architecture;
- `FND-04` admission/GameSession/CharacterLease/reconnect architecture;
- `DUR-01` durable identifier representation;
- `DUR-02` profile-neutral Character persistence partial baseline and whole Persistence-v1 common architecture;
- `ANL-01` event/audit foundation;
- TCP-default/future-QUIC architecture direction under ADR-0014..0016 while gameplay transport runtime remains unavailable;
- `GAME-VISION-01` minimum product direction and immutable first Reference target;
- `GAME-CHAR-01` Stage A + Stage B semantics;
- `GAME-ITEM-01` typed item definition/instance/equipment/container/revision semantics from PR #205, final head `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a`, squash merge `5c502d24557621efc798def87b68f137ba23fad8`.

Do not restart FND-01, VSL-02, GAME-CHAR, DUR-02 or GAME-ITEM architecture work unless an explicit superseding decision is accepted.

## Accepted GAME-ITEM boundary — consume, do not reopen

`GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md` freezes:

- stable versioned `ItemType` versus concrete DUR-01 `ItemInstance` versus authored `StaticItemPlacement`;
- typed bounded capabilities for stack quantity, charges, durability, temporal/decay, equipment, container, binding/restrictions and upgrades/modifiers;
- rejection of arbitrary authoritative JSON/EAV/free-form item state;
- server-authoritative atomic equipment occupancy and bounded acyclic container legality;
- deterministic modifier contribution ordering requirements without taking SIM arithmetic ownership;
- explicit item-definition compatibility/migration and no silent reinterpretation;
- Reference-sensitive unknowns as fail-closed `PARITY_PENDING_EVIDENCE`;
- `PROD-ENTITLEMENTS-01` as a separately unaccepted Oteryn-v2 consumer/enforcement gate.

It does **not** own create/destroy/split/merge/transform ItemInstanceId transition mechanics, atomic location, idempotency/retry/crash behavior or item/currency/value conservation. Those remain `DUR-03`.

## Binding DUR ownership split

Preserve:

- `GAME-ITEM-01` — item semantic legality and definition/instance state;
- `DUR-03` — item/currency/value transaction, single-location, conservation, provenance and anti-duplication semantics;
- `EXP-ECONOMY-01` — market/economy semantics;
- `EXP-SOCIAL-01` — guild/social semantics;
- `EXP-HOUSES-01` — houses;
- `GAME-META-01` — recurring/meta rewards;
- `EXP-EVENTS-01` — encounter/event rewards;
- `ANL-01` — event/audit semantics;
- PERF/implementation evidence — partitioning and exact Rust DB/migration-library choices unless correctness evidence requires an architecture decision.

## Implementation boundary

Architecture prerequisites permit a **later separately authorized** server/persistence foundation implementation programme for already accepted common scopes. A safe decomposition hypothesis remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
```

This checkpoint grants **no implementation authority**. Do not create server runtime code, DDL/migrations, item mutation runtime or production configuration unless a new explicit owner instruction authorizes an implementation package.

## Current ordered architecture work

After GAME-ITEM lifecycle closeout, the immediate safe paper-only sequence is:

1. `DUR-03 — Item Transaction and Anti-Duplication Invariants` — consume accepted GAME-ITEM, DUR-01, DUR-02 and ANL-01 and freeze item/currency/value conservation, single-location, identity-transition, idempotency, stale-writer, retry/crash and provenance semantics without runtime implementation;
2. `GAME-CHANNEL-01`, Reference evidence/parity tooling, `DUR-04` minimum headless content path and `SIM-DETERMINISM-01` may proceed in parallel only under separate explicit path ownership;
3. real-boundary VSL implementation expands only after each owning architecture and implementation authorization is ready.

QUIC remains later evidence-gated work, not the current implementation priority.

## Repository and production authority

Routine writes: `blakinio/Oteryn-v2` only.

External repositories remain read-only unless the owner separately authorizes an exact write task. This checkpoint grants no production deployment, protected-environment, secret, live account/session/data/database, entitlement activation or asset-copying authority.

## Context checkpoint

```yaml
last_progress: GAME-ITEM-01 delivery PR #205 was accepted and squash-merged as 5c502d24557621efc798def87b68f137ba23fad8; lifecycle closeout #206 reconciles canonical status/handoff and releases GAME-ITEM ownership, after which DUR-03 is the next bounded paper-only architecture gate.
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
next_action: From live main after GAME-ITEM lifecycle closeout, create one bounded paper-only `DUR-03` architecture task that consumes accepted GAME-ITEM, DUR-01, DUR-02 and ANL-01; do not implement runtime/DDL/production behavior.
```
