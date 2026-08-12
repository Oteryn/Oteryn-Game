# OTV2-20260812-dur-03-item-transaction-architecture

```yaml
task_id: OTV2-20260812-dur-03-item-transaction-architecture
title: DUR-03 item transaction and anti-duplication architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: agent/otv2-20260812-dur-03-item-transaction-architecture
pr: 207
base_sha: 2521882253b04287e1243c54692440120e0b6c8e
head_sha: 03162e512268b2776c9e726ccee9220a9fa4af46
final_head_sha: null
final_head_frozen_at: null
owner: architecture-coordinator/current-session
created_at: 2026-08-12T14:23:00+02:00
updated_at: 2026-08-12T14:59:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-dur-03-item-transaction-architecture.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
public_contracts:
  - DUR-03
depends_on:
  - GAME-ITEM-01
  - DUR-01
  - DUR-02
  - ANL-01
  - FND-02
  - FND-03
  - FND-04
blocks:
  - durable item/currency/value runtime implementation
  - item/value portion of later VSL persistence/loot/pickup proofs
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Produce one bounded, paper-only `DUR-03` architecture candidate that freezes authoritative item/currency/value transaction, single-location, identity-transition, runtime↔durable handoff, idempotency, retry/crash, provenance and anti-duplication invariants on top of accepted GAME-ITEM-01, DUR-01, DUR-02, ANL-01 and FND runtime/session authority.

The delivery implements nothing. Rust/runtime/client work, PostgreSQL DDL/migration execution, production changes and entitlement activation remain unauthorized. Shared canonical status/register/horizon/handoff files stay at live-main pre-acceptance state while PR #207 is open; any promotion is deferred to a separate post-merge lifecycle closeout.

## Architecture and source of truth

- `PROVEN`: `main@2521882253b04287e1243c54692440120e0b6c8e` records `GAME-ITEM-01 = ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` and `DUR-03 = PROPOSED / PLANNED / NOT_STARTED` as the next eligible paper-only item/value gate.
- `PROVEN`: GAME-ITEM owns typed item legality, ItemType/ItemInstance/StaticItemPlacement, equipment/container legality and definition compatibility; it assigns atomic location/conservation/identity transitions to DUR-03.
- `PROVEN`: DUR-01 defines ItemInstanceId as strong UUIDv7/full 128-bit/non-reused and forbids using UUID ordering as authority.
- `PROVEN`: DUR-02 owns common migration/transaction/outbox/durable-ack/PITR/evolution rules, requires invariant/anomaly proof and permits stricter DUR-03 transaction rules.
- `PROVEN`: ANL-01 owns EventId/OperationId/TransactionId/TransactionEventRef, mandatory durable evidence semantics and read-only replay.
- `PROVEN`: FND-02 owns `CommandRef=(GameSessionId,CommandId)` exact reservation/order/duplicate behavior.
- `PROVEN`: FND-03 owns one ChannelRuntime/InstanceRuntime writer, ownership-generation fencing and asynchronous `PENDING` external-work semantics; it explicitly forbids blocking DB/remote work in the writer lane.
- `PROVEN`: FND-04 owns GameSession/CharacterLease/recovery authority and same-session connection-generation changes.
- `PROVEN`: `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` classifies inventory/equipment as shared durable Character state and ground items/corpses as channel-local runtime authority with durable pickup boundary.
- `PROVEN`: ADR-0006 requires durable economy/security evidence for security-relevant item/currency create/destroy/split/merge/location/ownership/loot/pickup/trade/market/mail/depot/reward/currency/retry/transaction resolution.
- `PROVEN`: open PR #191 is GAME-CHAR provenance-only; #162 is CI/governance; neither overlaps the three DUR-03 owned paths.
- `UNKNOWN`: exact Reference-specific source/sink, transform, loot materialization timing, trade/market/bank/depot/mail/reward/crafting/decay edge behavior not established by accepted evidence remains parity-pending and may not be invented here.

## Acceptance criteria

- [x] Freeze one typed immediate semantic location invariant for every live durable ItemInstance without collapsing world scope, binding, custody, runtime simulation ownership or authorization.
- [x] Define runtime simulation owner versus durable recoverability so runtime ground/checkpoint state cannot become a second durable item authority.
- [x] Define non-blocking runtime PREPARE/reservation -> async game-DB durable COMMIT -> normalized runtime completion/reconciliation for ground/instance ↔ durable Character/value transfers.
- [x] Define crash windows for pickup/drop so committed DB state suppresses stale runtime/checkpoint ghosts and known abort/ambiguity cannot double-spend.
- [x] Preserve explicit downstream choice for runtime-only loot materialization versus already-durable ground ItemInstance; both require stable cause/idempotency.
- [x] Define create/destroy/split/merge/quantity-transfer/transform identity transitions preserving DUR-01 non-reuse.
- [x] Separate internal Oteryn transform UUID preserve/replace policy from externally observable Reference transform behavior.
- [x] Keep transaction-scoped planned output ItemInstanceIds stable across physical retries and never reassign them to another logical transaction.
- [x] Define exact conservation classes and lineage rather than market-value equality.
- [x] Define CommandRef/OperationId/TransactionId ownership, durable receipts and same-GameSession reconnect boundary.
- [x] Distinguish proven abort (same intent may rematerialize current legal before/effect rows) from ambiguous commit (exact candidate freezes until classification).
- [x] Define CharacterLease/GameSession/runtime ownership fences without using ItemInstanceId, binding or old transport generation as authorization.
- [x] Define bounded participant/evidence sets, anomaly-proof isolation/locking obligations and typed multi-transaction custody.
- [x] Define bounded ANL-compatible mandatory audit where value/security policy requires it without requiring one durable event for every high-frequency item field tick.
- [x] Define cross-world/cross-authority fail-closed behavior and reject implicit cross-database distributed transaction.
- [x] Define restore/recovery reconciliation including runtime checkpoint/ground ghost suppression.
- [x] Preserve downstream ownership for loot/business policy, trade, market, bank, depot, mail, rewards, houses, crafting/ruleset and entitlements.
- [x] Apply architecture decision discipline: must-decide-now, blocked work, migration cost, supersession evidence, deliberately deferred scope.
- [ ] Complete terminal exact-head self-review, genuinely independent review, required exact-head CI and zero unresolved material review threads.
- [ ] Squash-merge #207 unchanged from frozen validated head.
- [ ] After accepted merge, use one separate bounded lifecycle closeout to promote DUR-03 and refresh canonical handoff; do not promote status in open delivery PR.

## Excluded scope

- Rust/runtime/client implementation or gameplay command payload implementation.
- PostgreSQL DDL, migration files/execution, physical table/index/constraint/lock syntax or concrete ORM/driver selection.
- Production deployment, traffic, credentials, live data/session/database mutation, protected environments or backup execution.
- Exact unevidenced Reference values/formulas/source-sink rates/business behavior.
- Reopening accepted GAME-ITEM, DUR-01, DUR-02, ANL-01 or FND-02/FND-03/FND-04 except to document a verified conflict.
- Trade/market/bank/depot/mail/house/reward/crafting/loot generation business state machines.
- Concrete ANL protobuf event IDs/payload schemas or numeric DUR-03 resource ceilings without implementation/security/workload evidence.

## Implementation / findings

Preflight on `main@2521882253b04287e1243c54692440120e0b6c8e` found no active DUR-03 task and no overlapping open PR ownership. Draft PR #207 was opened early. Shared canonical overlays remain outside delivery ownership so the candidate cannot mark itself accepted before merge/closeout.

Candidate architecture now freezes:

1. **One location:** every live durable ItemInstance has exactly one typed immediate semantic location. Runtime ground projection and durable recovery record represent the same semantic location, not peer authorities.
2. **Runtime↔durable handoff:** current ChannelRuntime/InstanceRuntime reserves value under ownership generation, emits async persistence work and yields. The game-owned DB transaction is the durable value linearization point. Completion returns as a new normalized runtime input. Crash recovery checks committed receipt/location before rematerializing ground/checkpoint state.
3. **Durable drop/pickup:** an acknowledged dropped durable item remains recoverable on ground; pickup of durable ground item moves same ItemInstance. A runtime-only loot candidate instead materializes through one idempotent MINT occurrence when its owning gameplay gate chooses that model.
4. **Identity:** same concrete lifecycle preserves ID; new lifecycle gets transaction-scoped fresh ID; split keeps source + new output ID; merge keeps receiver and retires emptied source; transform uses explicit Oteryn preserve/replace policy.
5. **Retry identity:** TransactionId is logical atomic intent. Proven non-commit may reread/rematerialize same intent while keeping planned output IDs; ambiguous commit freezes exact candidate until classification.
6. **Conservation:** `TRANSFER`, `SPLIT_MERGE_QUANTITY`, `STATE_MUTATION`, `MINT`, `BURN`, `TRANSFORM`, `CONVERSION`; exact units/asset lines + cause/lineage, never market price.
7. **Custody/authority:** typed custody for multi-transaction workflows; current GameSession/CharacterLease/runtime fences; no binding/location/ItemInstanceId/NodeId authority shortcut.
8. **Atomicity:** current durable linearization uses one `oteryn_game` PostgreSQL transaction. Runtime memory reservation is not distributed 2PC; cross-database service transfer remains unsupported until separately contracted.
9. **Audit:** mandatory bounded durable evidence only where owning value/security policy requires it; aggregated transaction evidence allowed; no unbounded per-line event requirement.
10. **Recovery:** fail closed until identity/location/container/receipt/cause/audit/asset/runtime-fence reconciliation passes; analytics stays read-only.

## Repair history

### Repair cycle 1 — adversarial pre-freeze architecture self-audit

The first full candidate draft exposed three material clarity/correctness gaps before final-head freeze:

1. transform `PRESERVE_INSTANCE`/`REPLACE_INSTANCE` wording risked implying Oteryn UUID continuity could be discovered from Global; repaired by separating observable Reference transform behavior from internal Oteryn lifecycle identity policy;
2. retry wording over-froze transaction effects even after a **proven non-committed abort**; repaired by preserving stable logical TransactionId/intent/output IDs while permitting reread/rematerialization under current state after known abort, but freezing the exact candidate during ambiguous commit;
3. the initial draft did not explicitly close the accepted FND-03/multichannel boundary where ground items are runtime-owned but pickup/drop crosses durable value. Repaired with a non-blocking runtime reservation -> async DB durable linearization -> runtime completion/recovery protocol and explicit runtime-only loot materialization alternative.

The same audit also narrowed durable-audit wording so bounded mandatory audit applies according to accepted security/value policy rather than forcing one durable event for every high-frequency non-security-critical item state tick.

Process note: during this pre-freeze repair, one accidental temporary `.tmp` path was created by a tool-call mistake and immediately deleted. It never remained in the net PR diff, carried no architecture content, was not used to trigger CI or bypass validation, and the branch was not force-rewound. Final changed-file scope remains exactly the three declared owned paths.

## Validation

### Focused

- source/ownership audit against GAME-ITEM, DUR-01, DUR-02, ANL-01, FND-02, FND-03, FND-04, ADR-0006 and multichannel scope matrix: complete for candidate drafting
- full net diff scope check: pending after this task-record commit
- governance/document/link validation: pending exact frozen head

### Component/integration

- command/run: `NOT_APPLICABLE` — paper-only architecture candidate; no executable component changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/production outcome is introduced
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after this task-record commit
- trigger source: PR #207
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending after task/PR metadata freeze
- method/reviewer: implementing/coordinating agent
- material findings: repair cycle 1 addressed before freeze; terminal review pending
- verdict: pending

## Independent review

- required: `YES` — durable item/currency/value conservation, runtime↔durable handoff and anti-duplication are high-risk correctness/security boundaries
- exact head: pending
- method/auditor: genuinely independent mechanism on frozen exact head
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #207 draft
- changed-file review: expected exactly task + analysis + candidate contract
- unresolved review threads: pending
- related live PRs #191/#162: disjoint and untouched
- protected auto-merge: not configured
- merge commit/result: pending
- ownership release: only after separate terminal lifecycle closeout

## Context checkpoint

```yaml
last_progress: Completed repair cycle 1 from adversarial pre-freeze audit, including internal transform identity clarification, known-abort versus ambiguous retry semantics, mixed runtime-ground/durable handoff and bounded audit scope; task now awaits final diff/metadata freeze and terminal review/CI.
status: validating
branch: agent/otv2-20260812-dur-03-item-transaction-architecture
head_sha: 03162e512268b2776c9e726ccee9220a9fa4af46
pr: 207
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request #207
ci_check_generation: pending final freeze
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Verify the resulting exact head changes only the three owned paths, update PR #207 metadata without moving the head, mark ready, then run terminal implementing-agent self-review, independent review and exact-head CI.
```
