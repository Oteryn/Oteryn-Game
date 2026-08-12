# OTV2-20260812-dur-02-owner-acceptance

```yaml
task_id: OTV2-20260812-dur-02-owner-acceptance
title: Persist owner-accepted whole DUR-02 Persistence-v1 baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260812-dur-02-owner-acceptance
pr: 201
base_sha: 4f4ac4f0891b9d37dcefd413d1baf724c20d301c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-12T09:41:00+02:00
updated_at: 2026-08-12T09:58:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-dur-02-owner-acceptance.md
  - docs/architecture/DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
public_contracts: []
depends_on:
  - docs/architecture/DUR-02_PERSISTENCE_V1_WHOLE_GATE_RECONCILIATION.md
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/ARCHITECTURE_STATUS_MODEL.md
blocks:
  - separately authorized server/persistence foundation implementation programme
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Persist the owner's explicit acceptance of the six-rule minimum whole-`DUR-02 — Persistence v1` closure package and the exhaustive fourteen-subject reconciliation disposition as the binding owner baseline for the whole gate.

After terminal delivery/closeout the intended architecture status is:

```text
DUR-02 — Persistence v1
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL/migrations       = NOT_AUTHORIZED by architecture acceptance alone
```

## Owner source

`USER_SOURCE`: on 2026-08-12 at 09:41 +02:00, immediately after presentation of the complete six-rule whole-DUR-02 closure package and the fourteen-subject reconciliation, the owner replied:

> tak

This task treats that reply as explicit acceptance of the complete recommended package in `DUR-02_PERSISTENCE_V1_WHOLE_GATE_RECONCILIATION.md`, including the dispositions, scope boundaries, implementation consequences, non-decisions and supersession rules.

## Acceptance criteria

- [x] Add `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` as the binding whole-gate owner baseline.
- [x] Bind the six remaining common Persistence-v1 rules without weakening the accepted Character persistence partial baseline.
- [x] Bind all fourteen historical subject dispositions and exact destination ownership where moved.
- [x] Set overall DUR-02 to `ACCEPTED` only in current coordination state; keep `ImplementationStatus=NOT_STARTED` and runtime/DDL authority `NONE`.
- [x] Explicitly preserve GAME-ITEM-01/DUR-03 as the authority for item/currency/value semantics and conservation.
- [x] Explicitly preserve ANL-01 event/audit semantic ownership, FND-03 runtime checkpoint/replay ownership and FND-04 session/lease/recovery authority.
- [x] Preserve exact library/tool/index/partition/RPO/RTO/cadence choices as downstream unless later evidence creates an architecture constraint.
- [x] State that accepted DUR-02 permits a separately authorized server/persistence foundation implementation programme without waiting for GAME-ITEM-01/DUR-03, while durable item/value mutation remains blocked on those gates.
- [x] Update current status, global register and gameplay/product horizon narrowly.
- [x] Do not implement SQL DDL, migrations, Rust persistence/server runtime, Platform writes or production actions.
- [ ] Complete exact-head self-review, mandatory independent exact-head review, repository-required CI, squash merge and lifecycle closeout.

## Excluded scope

- no PostgreSQL schema creation or migration execution;
- no Rust game-server or persistence implementation;
- no database credentials/provisioning;
- no item/currency/value persistence before GAME-ITEM-01 + DUR-03;
- no market/guild/house/reward semantic acceptance;
- no exact Rust DB/migration crate selection;
- no partitioning/sharding decision;
- no numeric RPO/RTO/backup cadence/retention values;
- no Platform database changes;
- no production rollout/traffic;
- no unresolved Reference behavior inference.

## Validation

### Focused

- binding owner baseline versus lifecycle-closed whole-gate reconciliation: **PASS**;
- Character persistence partial baseline preserved/consumed rather than superseded: **PASS**;
- ADR-0004/DUR-01/ANL-01/FND-03/FND-04 ownership boundaries: **PASS**;
- `MOVED` historical subjects do not become destination-gate acceptance: **PASS**;
- implementation/runtime/DDL authority remains absent: **PASS**.

### Repair cycle 1 — preserve accepted transport/GameNode policy

Full-diff review found one unrelated coordination drift in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`: while condensing the progressive execution policy for accepted DUR-02, the existing sentence preserving ADR-0014/ADR-0015/ADR-0016 transport/GameNode semantics was accidentally removed.

Repair:

- restored the exact accepted policy that dual transport remains architecture direction only, every gameplay transport mode remains unavailable until implemented/proven, and ADR-0009 one-process GameNode identity remains binding while only internal decomposition/genuinely distinct adjacent-service placement stays evidence-driven.

This repair does not change DUR-02 semantics. Repair budget used: `1/3`.

### Component/integration/runtime E2E

`NOT_APPLICABLE` — paper-only owner architecture acceptance; no executable database/runtime behavior changes.

### Independent review

Required: **YES**. Whole-gate acceptance makes persistence, migration, transaction, crash-recovery, publication-checkpoint and restore-safety guarantees binding architecture, so one genuinely independent exact-head review is mandatory before merge.

## PR and closeout

- delivery PR: #201;
- intended changed files: exactly five declared documentation/task paths;
- related PR #191 and #162: disjoint and untouched;
- merge: squash only after terminal exact-head self-review, independent exact-head review, exact-head CI, zero review threads and `behind_by=0`;
- lifecycle closeout: separate active->archive delivery after merge, setting DUR-02 DeliveryStatus `OPEN -> LIFECYCLE_CLOSED` and releasing ownership without semantic change.

## Context checkpoint

```yaml
last_progress: Owner baseline and coordination overlays are on PR #201; repair cycle 1 restored the pre-existing ADR-0014/0015/0016 transport/GameNode policy sentence removed by initial condensation.
status: validating
branch: docs/OTV2-20260812-dur-02-owner-acceptance
pr: 201
final_head_sha: null
final_head_frozen_at: null
ci_checks_for_current_head: 0
repair_cycles_for_current_gate: 1
owner_action_required: null
blocker: null
next_action: Perform terminal full-diff self-review on the unchanged candidate, then freeze exact head, run mandatory independent Codex review and repository-required CI before squash merge.
```
