# OTV2-20260812-dur-02-whole-gate-reconciliation

```yaml
task_id: OTV2-20260812-dur-02-whole-gate-reconciliation
title: Reconcile the complete historical DUR-02 Persistence-v1 gate
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260812-dur-02-whole-gate-reconciliation
pr: 199
base_sha: 710c4b5e00de9f14224a6949c3bc7364f4c724a4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-12T09:12:00+02:00
updated_at: 2026-08-12T09:29:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-dur-02-whole-gate-reconciliation.md
  - docs/architecture/DUR-02_PERSISTENCE_V1_WHOLE_GATE_RECONCILIATION.md
public_contracts: []
depends_on:
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
  - docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
blocks:
  - owner decision on the minimum remaining whole-DUR-02 closure package
  - later separately authorized Persistence-v1 implementation design
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Reconcile every historical subject listed under stable gate `DUR-02 — Persistence v1` against the architecture accepted later. Deliver one nonbinding decision packet that distinguishes:

- already satisfied by binding architecture;
- still genuinely owned by DUR-02;
- moved to another named gate/domain;
- implementation/operations choices that should not remain architecture blockers;
- owner decisions still required before overall `DUR-02` may become `ACCEPTED`.

No SQL DDL, database migration, Rust persistence code, runtime behavior or production action is authorized.

## Architecture and source of truth

- `PROVEN`: current trusted base is `main@710c4b5e00de9f14224a6949c3bc7364f4c724a4`.
- `PROVEN`: profile-neutral Character persistence is already a binding `OWNER_ACCEPTED PARTIAL BASELINE / LIFECYCLE_CLOSED / NOT_STARTED`.
- `PROVEN`: overall `DUR-02` remains `PROPOSED / PLANNED / NOT_STARTED` specifically because historical Persistence-v1 scope is broader than that Character sub-scope.
- `PROVEN`: historical `FOUNDATION_DECISION_BACKLOG.md` lists fourteen material Persistence-v1 subjects: migration/schema ownership, Character state, lease state, item transfers, idempotency, isolation/retries, outbox, audit/telemetry, atomic value/security evidence, checkpoint/progress loss, market/guild/house/reward consistency, partitioning, backup/PITR/RPO/RTO and migration rollout/rollback.
- `PROVEN`: ADR-0004 already establishes PostgreSQL, separate Platform/game databases, one semantic/migration owner per persistent data set, no shared-table free-for-all, no cross-database FKs, and game-schema ownership by `blakinio/Oteryn-v2`.
- `PROVEN`: later architecture gives item/economy conservation to `GAME-ITEM-01` + `DUR-03`, event/audit semantics to `ANL-01`, Character/session persistence semantics to the accepted partial baseline + FND-04, runtime checkpoint/replay semantics to FND-03, and production capacity/operations objectives to PERF/OPS gates.
- `PROVEN`: open PR #191 is a disjoint GAME-CHAR factual provenance correction and PR #162 is disjoint CI/governance work; neither is owned or modified here.

## Acceptance criteria

- [x] Enumerate every historical DUR-02 subject without omission.
- [x] Assign exactly one primary disposition to each subject: `SATISFIED`, `RETAIN_DUR02`, `MOVED`, or `IMPLEMENTATION_DEFERRED`.
- [x] Name the canonical evidence/owner for every `SATISFIED` or `MOVED` subject.
- [x] Do not silently move item/currency/market/house/social semantics back into generic Persistence-v1.
- [x] Preserve the accepted Character persistence partial baseline unchanged.
- [x] Separate architecture requirements from exact library/tool/index/partition/backup cadence choices.
- [x] Define the smallest remaining owner decision package required for honest overall DUR-02 acceptance.
- [x] State precisely what whole-DUR-02 acceptance would and would not unblock.
- [x] Keep runtime/DDL/migrations/production unauthorized.
- [ ] Complete full exact-head self-review and repository-required documentation CI before merge.

## Findings

### Historical scope classification

All fourteen backlog subjects are accounted for in `DUR-02_PERSISTENCE_V1_WHOLE_GATE_RECONCILIATION.md`.

Primary disposition summary:

- `SATISFIED`: Character state/revision, Character lease ownership, idempotency identity/duplicate-command foundation, critical audit-vs-telemetry semantics;
- `MOVED`: item/inventory/ground transfer; atomic item/currency/security evidence semantics; market/guild/house/reward consistency;
- `IMPLEMENTATION_DEFERRED`: partitioning/sharding and exact Rust DB/migration library unless implementation evidence creates a correctness constraint;
- `RETAIN_DUR02`: migration artifact/authority model, common isolation/locking/retry principle, common audit/outbox substrate boundary, durable-ack/runtime-checkpoint/disaster-loss distinction, PITR/restore safety envelope, common schema-evolution discipline.

### Minimum remaining closure package

The nonbinding recommendation reduces the remaining whole gate to six rules:

1. one game-owned migration ledger/history for the current `oteryn_game` boundary with immutable explicit migration artifacts, dedicated least-privilege migrator and no production runtime auto-schema-sync;
2. READ COMMITTED only with explicit anomaly-closing locks/constraints, otherwise bounded SERIALIZABLE or stricter accepted domain mechanism, preserving semantic operation identity across retry;
3. one ANL-compatible durable journal + mutable crash-safe publication-state/checkpoint pattern for authoritative game domains, atomically committed when evidence is mandatory;
4. explicit separation between acknowledged durable commits, FND-03 runtime checkpoint/replay and disaster-recovery RPO;
5. PITR-capable, restore-tested, fail-closed recovery with non-rollback authority fencing while numeric objectives remain OPS/PERF-owned;
6. game-wide expand -> migrate/backfill -> validate -> cut over -> contract schema-evolution discipline with writer fencing and evidence-based recovery/rollback.

### Repair cycle 1 — status and future-topology scope

Pre-freeze self-review found two issues:

1. the packet described overall DUR-02 `DeliveryStatus=OPEN` even though this is a pre-decision analysis task rather than owner-baseline delivery, while the canonical overlay correctly remains `PLANNED`;
2. the one-migration-ledger recommendation needed to be bounded to the current `oteryn_game` database boundary so a future explicitly accepted separate game-domain database/service is not accidentally prohibited.

Repair:

- restored overall status to `PROPOSED / PLANNED / NOT_STARTED` during this nonbinding analysis;
- bounded one migration ledger/history to the current game database and required a later ADR for any genuinely separate persistence authority.

### Repair cycle 2 — exact domain ownership and crash-safe publication checkpoint

Terminal review found two additional issues:

1. the historical market/guild/house/reward consistency bucket was mapped only generically to "domain owners", which was too weak for an exhaustive ownership reconciliation;
2. the outbox recommendation stated at-least-once delivery but did not explicitly freeze crash-safe publication claim/checkpoint behavior after ambiguous submission or publisher failure.

Repair:

- mapped market/economy to `EXP-ECONOMY-01`, guild/social to `EXP-SOCIAL-01`, houses to `EXP-HOUSES-01`, recurring/meta rewards to `GAME-META-01`, encounter/event rewards to `EXP-EVENTS-01`, with `DUR-03` retaining item/currency conservation for all of them;
- required restart-safe publication claim/checkpoint state where claim is not proof of delivery, publisher crash leaves work retriable/reconcilable, ambiguous broker outcomes retain the same EventId/content, and immutable event evidence cannot be deleted merely because submission was attempted.

Repair budget used: `2/3`.

## Excluded scope

- no whole-DUR-02 owner acceptance in this task;
- no PostgreSQL DDL or migrations;
- no Rust persistence implementation;
- no selection of ORM/query builder/database crate;
- no item/currency conservation design;
- no market/guild/house/reward gameplay semantics;
- no numeric production RPO/RTO/capacity/backup cadence;
- no Platform writes or external-repository mutation;
- no production deployment.

## Validation

### Focused

- exact backlog-to-owner reconciliation: **PASS after repair cycle 2**;
- ADR-0004 ownership compatibility: **PASS**;
- accepted Character/FND-04/ANL/FND-03 boundaries preserved: **PASS**;
- explicit destination ownership for historical domain consistency bucket: **PASS**;
- crash-safe publication checkpoint/reconciliation semantics: **PASS**;
- no later gameplay gate pre-accepted: **PASS**.

### Component/integration/runtime E2E

`NOT_APPLICABLE` — nonbinding paper-only architecture reconciliation.

### Exact-head CI

Pending final immutable PR head.

## Self-review

Pending final immutable PR head.

## Independent review

- required: `NO` for the nonbinding reconciliation packet unless final review identifies material uncertainty or the delivery accidentally creates binding persistence/recovery authority; a later owner-acceptance delivery must reassess and is expected to require independent review.

## PR and closeout

- delivery PR: #199;
- intended changed files: exactly task + reconciliation packet;
- `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` remains unchanged because this task is pre-decision analysis rather than owner-baseline delivery;
- PR #191 and #162 remain untouched;
- after delivery merge, separate lifecycle closeout archives the task without changing overall DUR-02 status;
- owner decision follows only after the reconciliation packet is lifecycle-closed.

## Context checkpoint

```yaml
last_progress: Repair cycle 2 completed: exact owners assigned for the historical market/guild/house/reward bucket and the common outbox substrate now has explicit crash-safe publication checkpoint/reconciliation semantics.
status: validating
branch: docs/OTV2-20260812-dur-02-whole-gate-reconciliation
pr: 199
final_head_sha: null
final_head_frozen_at: null
ci_checks_for_current_head: 0
repair_cycles_for_current_gate: 2
owner_action_required: null
blocker: null
next_action: Perform terminal full-diff self-review on the unchanged candidate; if clean, freeze exact head and run documentation CI before merge/archive and owner decision.
```
