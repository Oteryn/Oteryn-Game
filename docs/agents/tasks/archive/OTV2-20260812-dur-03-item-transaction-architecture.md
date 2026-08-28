# OTV2-20260812-dur-03-item-transaction-architecture — archived

```yaml
task_id: OTV2-20260812-dur-03-item-transaction-architecture
title: DUR-03 item transaction and anti-duplication architecture
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: agent/otv2-20260812-dur-03-item-transaction-architecture
delivery_pr: 207
base_sha: 2521882253b04287e1243c54692440120e0b6c8e
final_head_sha: a1d949362e219373a5d314c0e9ddf8de110362dd
delivery_merge_sha: 63380bcba469027e90677aaf4db571fa941be2f4
lifecycle_closeout_branch: docs/OTV2-20260812-dur-03-architecture-closeout
lifecycle_closeout_pr: 208
owner: released_after_closeout
created_at: 2026-08-12T14:23:00+02:00
completed_at: 2026-08-12T15:16:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
implementation_status: NOT_STARTED
runtime_authority: NONE
postgresql_ddl_migration_authority: NONE
production_authority: NONE
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260812-dur-03-item-transaction-architecture.md
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
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Delivered the bounded paper-only `DUR-03` architecture contract defining authoritative item/currency/value transaction, conservation and anti-duplication semantics. Delivery PR #207 is merged as `63380bcba469027e90677aaf4db571fa941be2f4`; lifecycle closeout PR #208 reconciles canonical programme status/handoff and releases DUR-03 path ownership only when it merges.

The delivery implements nothing. Rust/runtime/client implementation, PostgreSQL DDL/migration execution, production deployment/traffic and entitlement activation remain unauthorized.

## Binding sources consumed

- `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md` — ItemType/ItemInstance/StaticItemPlacement, item legality, equipment/container legality and definition compatibility; location/conservation/identity transition explicitly delegated to DUR-03.
- `DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md` — strong UUIDv7 ItemInstanceId, full 128-bit, non-reused, no UUID-order authority.
- `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` — common anomaly-proof transaction/retry/outbox/durable-ack/PITR/evolution discipline.
- `ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md` — EventId/OperationId/TransactionId/TransactionEventRef and mandatory durable-evidence semantics.
- `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` — `CommandRef=(GameSessionId,CommandId)`, ordered reservation and duplicate non-reexecution.
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` — one runtime writer, ownership-generation fencing and asynchronous `PENDING` external-work rules.
- accepted FND-04 session/lease/recovery contracts — current GameSession/CharacterLease authority and same-session reconnect semantics.
- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` — Character inventory/equipment durable shared state versus channel-local ground/corpse runtime authority and durable pickup boundary.
- ADR-0006 — durable economy/security evidence and read-only Game Intelligence boundary.

## Accepted DUR-03 semantic closure

### One semantic location

Every live **durable** ItemInstance has exactly one typed immediate semantic location. Typed families cover Character inventory/equipment, container immediate parent, channel/instance-scoped ground and separately contracted world-shared/domain custody families.

Runtime ground projection/checkpoint and durable recovery state do not become peer authorities; they represent/reconstruct the same semantic location under different runtime/durability ownership responsibilities. Arbitrary owner/location JSON/EAV and generic `owner_id` authority are rejected.

### Runtime↔durable handoff

Mixed runtime-ground/instance and durable Character/value operations consume FND-03 instead of blocking its writer lane:

```text
current runtime owner PREPARE/reserves under ownership generation
-> bounded async persistence request
-> one game-owned PostgreSQL transaction is durable value linearization point
-> completion returns as normalized runtime input
-> current valid runtime owner reconciles projection
```

Known abort may release/retry the reservation. Ambiguous commit keeps the affected value non-spendable until durable reconciliation. A completion from a stale ownership generation cannot mutate the new runtime owner.

If DB commit succeeds and the runtime fails before completion, replacement/recovery checks committed DUR-03 receipt/location before rebuilding older checkpoint state. Stale ground/checkpoint ghosts cannot authorize a second pickup or re-mint.

This is explicitly **not** distributed memory/DB 2PC.

### Durable drop/pickup and loot materialization boundary

- A previously acknowledged durable ItemInstance dropped to ground has a durably recoverable semantic ground result; ordinary GameNode failure cannot lose it or restore old inventory spendability.
- Pickup of an already durable ground ItemInstance changes the same ItemInstance's semantic location and reconciles any stale runtime projection after commit.
- If an owning combat/loot/content gate declares visible loot to be runtime-only until pickup, pickup/materialization is instead an idempotent `MINT` keyed by one stable authoritative occurrence/output cause and uses a transaction-scoped fresh ItemInstanceId.
- DUR-03 supports either explicit materialization model but does not choose loot generation/materialization timing for the combat/content owner.

### Item identity transitions

- same concrete lifecycle preserves ItemInstanceId for legal state mutation;
- every new independently locatable lifecycle uses a fresh ItemInstanceId;
- transaction-scoped planned output IDs remain stable across physical retry and are never reassigned to another logical transaction;
- split keeps source ID and gives the new stack a fresh output ID;
- merge/quantity transfer keeps the receiver ID and retires a source that reaches zero;
- type-changing transforms explicitly select internal Oteryn `PRESERVE_INSTANCE` or `REPLACE_INSTANCE` lifecycle policy;
- one ItemInstanceId can never become two live outputs;
- internal UUID preserve/replace policy is not falsely inferred from Global/Tibia because the external Reference does not expose Oteryn UUID identity; unknown observable transform behavior remains parity-pending.

### Conservation

Every authoritative value mutation uses one explicit semantic class:

```text
TRANSFER
SPLIT_MERGE_QUANTITY
STATE_MUTATION
MINT
BURN
TRANSFORM
CONVERSION
```

Conservation is exact item/asset units plus complete input/output/source/sink lineage, never market-price equality. Non-item fungible value uses exact bounded arithmetic; binary floating point is not authoritative conservation arithmetic.

### Command/operation/transaction identity

- FND-02 CommandRef remains player-command ingress/order identity.
- Every logical atomic durable value mutation has one ANL TransactionId allocated before the first durable commit attempt.
- OperationId is used where one logical workflow spans multiple transactions, asynchronous continuation or cross-process/session retry.
- Same TransactionId never changes business intent, source/cause, destination semantics or transaction-scoped planned output identity slots.

### Known abort versus ambiguous commit

A **proven non-committed** physical attempt may reread current authoritative before-state and rematerialize mutable effect rows for the same logical intent, keeping TransactionId and planned output IDs stable. If current state makes that same intent illegal, it rejects rather than morphing into a different business action.

Once commit outcome is **ambiguous**, the exact materialized candidate/evidence/output identities freeze until classification:

```text
committed -> return/reconcile exact result
proven non-committed -> retry same logical TransactionId
unknown -> fail/hold
never guess a second TransactionId
```

### Durable idempotency/receipts

Whenever CommandRef + current state alone cannot prove replay safety, durable receipt/reconciliation state distinguishes safely retryable/not-applied, committed/original result, durable terminal rejection where required, ambiguous state and conflicting identity reuse. Retention covers the owning replay/idempotency horizon.

### Authority/fencing

Value mutation consumes current accepted authority as applicable: valid player GameSession/CommandRef, current CharacterLease generation and current runtime ownership-generation fence for channel/instance participants. ItemInstanceId, binding, location, NodeId or an old transport generation are not credentials.

Same-GameSession reconnect may advance connection_generation while retaining a previously valid reserved CommandRef. DUR-03 therefore requires current logical session/lease/runtime authority and participant fences, not permanent equality to the old transport generation.

### Atomicity and typed custody

Current v1 durable value linearization is one game-owned `oteryn_game` PostgreSQL transaction under ADR-0004/DUR-02. Platform/game distributed 2PC, cross-database FK, mirrored dual authority and implicit remote-service atomicity are not accepted.

Multi-transaction workflows require explicit typed custody, stable OperationId where needed, independently conservation-safe commits, no old-location spendability after custody commit, restartable idempotency and explicit compensating transactions. Business state machines remain downstream-owned.

### Bounded durable evidence

Security/value classes requiring durable audit use ANL-compatible bounded transaction evidence sufficient to reconcile item/value effects. Evidence may aggregate multiple mutation lines and must obey hard resource ceilings; the contract does not force one durable event per every high-frequency non-security-critical item field tick.

Concrete ANL event IDs/payload schemas and numeric DUR-03 resource ceilings remain mandatory pre-implementation work and were intentionally not invented by the architecture gate.

### Restore/recovery

Before authoritative item/value mutation resumes after restore/integrity incident, implementation must validate supported schema, item identity/non-reuse consistency, one location, container/custody graph, legal quantity/revision state, receipt/source-cause uniqueness, required retained audit complete sets, non-item asset invariants and a newer recovery fence. Runtime recovery reconciles committed DUR-03 receipts/location before rematerializing older checkpoint ground/item projections.

Analytics/Game Intelligence remains read-only and cannot repair or mutate authoritative value.

### Cross-world and downstream boundaries

Direct cross-world value transfer remains forbidden; burn in world A plus mint in world B cannot bypass transfer policy.

DUR-03 owns conservation/transaction mechanics but does not absorb loot generation, trade consent, market/order-book policy, bank/depot/mail access/lifecycle, reward eligibility, houses, crafting/ruleset formulas or entitlement activation.

## Acceptance criteria — terminal delivery state

- [x] one typed semantic location per live durable ItemInstance;
- [x] runtime simulation ownership versus durable recoverability separated;
- [x] non-blocking runtime PREPARE/reservation -> async durable COMMIT -> runtime completion/recovery contract;
- [x] pickup/drop crash windows and stale checkpoint ghost suppression defined;
- [x] runtime-only loot materialization alternative kept downstream-owned and idempotent;
- [x] ItemInstanceId split/merge/transform/new/retirement semantics defined;
- [x] transaction-scoped output IDs stable through retries and non-reassigned;
- [x] exact conservation classes/source-sink lineage defined;
- [x] CommandRef/OperationId/TransactionId and durable receipt semantics defined;
- [x] known abort versus ambiguous commit retry behavior separated;
- [x] GameSession/CharacterLease/runtime fencing defined without identity/transport shortcuts;
- [x] bounded participant/evidence and anomaly-proof transaction obligations defined;
- [x] typed custody and cross-database/world fail-closed behavior defined;
- [x] bounded ANL-compatible durable audit boundary defined;
- [x] restore/recovery and read-only analytics boundary defined;
- [x] downstream product/business ownership preserved;
- [x] architecture decision timing, migration cost, supersession evidence and deliberate deferrals recorded;
- [x] frozen exact-head implementing-agent review passed;
- [x] genuinely independent Codex review completed without suggestions;
- [x] exact-head Agent Governance, Dependency Review and CodeQL passed;
- [x] unresolved material review threads before delivery merge: zero;
- [x] PR #207 squash-merged unchanged from frozen exact head;
- [ ] lifecycle closeout promotes canonical programme status/handoff and releases ownership; true only after PR #208 merges.

## Repair history

### Repair cycle 1 — adversarial pre-freeze architecture self-audit

The first full candidate draft exposed three material clarity/correctness gaps before frozen-head validation:

1. transform `PRESERVE_INSTANCE`/`REPLACE_INSTANCE` wording risked implying Oteryn UUID continuity could be discovered from Global; repaired by separating observable Reference transform behavior from internal Oteryn lifecycle identity policy;
2. retry wording over-froze transaction effects after a **proven non-committed abort**; repaired so stable logical TransactionId/intent/output IDs survive while current authoritative state may be reread/rematerialized after a known abort, but an ambiguous commit freezes the exact candidate;
3. the initial draft did not explicitly close the accepted FND-03/multichannel boundary where ground items are runtime-owned while pickup/drop crosses durable value; repaired with runtime reservation, asynchronous game-DB durable linearization, normalized completion/recovery and an explicit runtime-only loot materialization alternative.

The same repair narrowed durable-audit wording so bounded mandatory audit follows accepted security/value policy instead of requiring a durable event for every high-frequency non-security-critical item state tick.

Process evidence retained: one accidental temporary `.tmp` path was created by a tool-call mistake during pre-freeze editing and immediately deleted. It did not remain in the net PR diff, carried no semantic architecture, was not created for CI/check regeneration and the branch was not force-rewound. Final delivery diff remained exactly the three declared owned paths.

## Terminal delivery validation

Frozen exact delivery head: `a1d949362e219373a5d314c0e9ddf8de110362dd`.

- implementing-agent exact-head self-review `4916797999`: **PASS**, material findings `0`;
- independent Codex exact-head review request `5267211845`: completed without suggestions; PR 👍 reaction `450358534` recorded after the unchanged exact-head request;
- Agent Governance `31599369738`: **PASS**;
- Dependency Review `31599369737`: **PASS**;
- CodeQL `31599369780`: **PASS**;
- unresolved material review threads immediately before merge: `0`;
- final changed paths: exactly task + analysis + candidate contract;
- final compare to base main: `behind_by=0`;
- component/integration/runtime E2E: `NOT_APPLICABLE` — paper-only architecture delivery.

PR #207 was squash-merged unchanged from frozen exact head as `63380bcba469027e90677aaf4db571fa941be2f4`.

## Lifecycle closeout discipline

PR #208 must not change DUR-03 semantic content. It may only:

1. complete active -> archive movement and retain this full delivery/repair/validation history;
2. promote `DUR-03` to `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` in maintained programme/register/horizon/index sources;
3. keep runtime/client/PostgreSQL DDL/migration/production authority `NONE`/unauthorized;
4. preserve exact downstream business/domain ownership and all parity-pending behavior;
5. refresh the non-owning programme checkpoint and successor handoff to the next bounded paper-only architecture action selected from accepted programme ordering;
6. release DUR-03 path ownership only after closeout merge.

No closeout merge SHA or self-referential final head is invented before it exists.

## Context checkpoint

```yaml
last_progress: DUR-03 delivery PR #207 passed exact-head implementing-agent self-review, genuinely independent no-suggestion Codex review and all required exact-head CI, then squash-merged unchanged as 63380bcba469027e90677aaf4db571fa941be2f4; lifecycle closeout PR #208 is in progress.
status: completed
delivery_pr: 207
final_head_sha: a1d949362e219373a5d314c0e9ddf8de110362dd
delivery_merge_sha: 63380bcba469027e90677aaf4db571fa941be2f4
lifecycle_closeout_pr: 208
self_review: 4916797999
independent_review_request: 5267211845
independent_review_pr_reaction: 450358534
ci_run_ids:
  - 31599369738
  - 31599369737
  - 31599369780
repair_cycles_for_delivery_gate: 1
owner_action_required: false
blocker: null
next_action: Complete PR #208 lifecycle closeout; only after its merge is DUR-03 canonically ACCEPTED/LIFECYCLE_CLOSED and its ownership released.
```
