# DUR-03 — Item Transaction and Anti-Duplication Analysis

- Date: 2026-08-12
- Gate: `DUR-03`
- Delivery task: `OTV2-20260812-dur-03-item-transaction-architecture`
- Delivery PR: #207
- Status: **ANALYSIS / CANDIDATE INPUT; nonbinding while PR #207 is open**
- Runtime authority: **NONE**
- PostgreSQL DDL/migration authority: **NONE**
- Production authority: **NONE**

## 1. Executive summary

`DUR-03` closes the correctness gap between accepted item semantics and any future implementation that moves, creates, destroys, transforms or exchanges durable gameplay value.

Accepted upstream architecture already fixes the necessary owners:

- `GAME-ITEM-01` defines ItemType/ItemInstance/StaticItemPlacement, legal typed item state, equipment/container legality, world scope and definition compatibility;
- `DUR-01` defines strong non-reused `ItemInstanceId`;
- `DUR-02` defines common PostgreSQL transaction/retry/outbox/durable-ack/PITR/evolution rules;
- `ANL-01` defines EventId/OperationId/TransactionId/TransactionEventRef and durable evidence semantics;
- `FND-02` defines ordered `CommandRef = (GameSessionId, CommandId)` reservation and duplicate non-reexecution;
- `FND-03` defines ChannelRuntime/InstanceRuntime one-writer ownership, async `PENDING` external-work semantics and ownership-generation fencing;
- `FND-04` defines GameSession/CharacterLease/recovery authority;
- ADR-0006 requires durable economy/security evidence for security-relevant item/currency mutations.

Recommended DUR-03 core:

```text
LIVE DURABLE ITEM
= one ItemInstanceId
+ one WorldId value scope
+ exactly one typed immediate authoritative location
+ legal GAME-ITEM state

ATOMIC DURABLE VALUE TRANSACTION
= one TransactionId
+ stable logical intent/cause
+ bounded materialized participant/effect set
+ current authority/fence proof
+ explicit TRANSFER / SPLIT_MERGE / STATE_MUTATION /
  MINT / BURN / TRANSFORM / CONVERSION class
+ durable receipt + mandatory ANL-compatible audit

KNOWN ABORT
= retry/rematerialize same logical intent under same TransactionId

AMBIGUOUS COMMIT
= freeze and reconcile the same materialized commit candidate
  before any new attempt

RUNTIME↔DURABLE ITEM HANDOFF
= current runtime owner reserves value under ownership generation
+ asynchronous durable transaction is the durable linearization point
+ runtime completion/recovery reconciles the durable result
+ no second runtime or durable authority exists
```

## 2. Verified source facts

### 2.1 GAME-ITEM-01

**PROVEN:** accepted GAME-ITEM establishes:

- ItemType is a stable versioned authored semantic definition;
- ItemInstance is one concrete mutable lifecycle using DUR-01 ItemInstanceId;
- StaticItemPlacement is not automatically a durable ItemInstance;
- one stack is one ItemInstance; quantity zero is not live stack state;
- equipment legality is a complete atomic occupancy claim;
- container legality is a bounded acyclic immediate-parent graph;
- item current location and create/destroy/split/merge/transform identity transitions are DUR-03-owned;
- world scope, binding, current location, authorization and presentation ownership remain distinct;
- physical currency items and non-item ledger value are distinct;
- unknown Reference item behavior remains fail-closed.

### 2.2 DUR-01

**PROVEN:** ItemInstanceId is a strongly typed full-width UUIDv7, nil invalid, never reused, internal/restricted by default. UUID ordering cannot establish authority, chronology or survivor choice.

### 2.3 DUR-02

**PROVEN:** accepted Persistence-v1 requires named invariant/anomaly proofs, semantic identity-preserving retry, atomic mandatory audit/outbox with mutation, durable-ack recovery, PITR/restore fencing and explicit schema evolution. It explicitly permits stricter DUR-03 transaction rules.

### 2.4 ANL-01

**PROVEN:** TransactionId identifies one logical atomic durable mutation; ambiguous commit/retry retains it. OperationId identifies a retry-capable multi-step logical operation where needed. Mandatory DURABLE_AUDIT commits with the owning mutation. Replay never replays gameplay mutation.

### 2.5 FND-02

**PROVEN:** CommandRef is `(GameSessionId, CommandId)`. A reserved command executes at most once; duplicate lower IDs never become a new command. Same-GameSession reconnect may preserve pending command identity while connection_generation advances.

### 2.6 FND-03 / multichannel scope

**PROVEN:** FND-03 owns one current authoritative ChannelRuntime/InstanceRuntime writer, ownership-generation fencing, normalized inputs, explicit async `PENDING` external work and non-authoritative callbacks.

**PROVEN:** the multichannel scope matrix classifies inventory/equipment as durable Character state, while ground items/corpses are channel-local runtime authority with a durable boundary on pickup; direct trade is channel-local interaction plus durable transaction.

This creates a required DUR-03 mixed runtime/durable handoff boundary rather than permission to block a ChannelRuntime on PostgreSQL or treat runtime ground memory as a second durable item authority.

### 2.7 ADR-0006

**PROVEN:** durable economy/security audit covers item create/destroy, split/merge, ownership/location changes, loot/pickup/trade/market/mail/depot/reward, currency and retry/transaction resolution where security-relevant.

### 2.8 Live programme state

**PROVEN:** `main@2521882253b04287e1243c54692440120e0b6c8e` records GAME-ITEM accepted/lifecycle-closed and DUR-03 proposed/planned/not-started as the next paper-only item/value architecture gate. Runtime/DDL/production authority is absent.

## 3. Problem

Without DUR-03, individually plausible designs still permit:

- inventory + ground duplicates;
- escrow side flags while old inventory remains spendable;
- stale channel/lease writers committing after authority changed;
- timeout-triggered second TransactionId/double mint;
- split reusing one ItemInstanceId for two live stacks;
- merge/transform identity resurrection;
- orphaned contained items;
- one-sided currency mutation;
- runtime crash after durable pickup/drop commit leaving a stale visible ghost;
- recovery rematerializing loot already committed to inventory;
- analytics detecting corruption but becoming an unsafe repair path.

## 4. Core constraints

- no live ItemInstanceId reuse or cross-transaction reassignment;
- exactly one immediate authoritative location per live durable ItemInstance;
- no client authority for identity/quantity/location/source/sink/balance;
- no binding/location identity as authorization;
- no stale GameSession/CharacterLease/runtime owner durable commit;
- no blind retry after ambiguous commit;
- no acknowledged durable item loss on ordinary GameNode restart;
- no cross-world gameplay value transfer by default;
- no implicit cross-database distributed transaction;
- no binary floating-point conservation arithmetic;
- no unbounded participant/custody/container/evidence graph;
- no synchronous DB wait inside the authoritative runtime writer lane.

Observable Reference behavior remains parity-gated. Oteryn-internal UUID preserve/replace policy is not inferred from nonexistent external UUIDs.

## 5. Decision — one typed immediate location

Reject subsystem-specific nullable peer location fields and generic owner/location JSON/EAV.

Recommended semantic model:

```text
ItemInstanceId -> exactly one ItemLocationRef
```

Typed families include as owned:

- CharacterInventory(CharacterId, typed position);
- CharacterEquipment(CharacterId, complete GAME-ITEM occupancy claim);
- Container(parent ItemInstanceId, typed entry);
- Ground(WorldId + ChannelRef/InstanceRef + typed spatial position);
- later separately typed world-shared spatial/custody families;
- later separately typed trade/market/depot/mail/reward/house custody families.

`TypedDomainCustody` is a registry concept only. Each family gets its own schema/type/owner/scope; no arbitrary generic location payload becomes authority.

## 6. Decision — runtime presence versus durable recoverability

A critical distinction is required:

```text
runtime simulation owner
!= second item location authority
!= durable recovery record
```

For channel/instance ground state:

- current ChannelRuntime/InstanceRuntime owns immediate gameplay visibility/interactability under its ownership generation;
- a durable item/value operation still needs enough committed durable location/receipt/provenance state to survive ordinary process/node restart after success is acknowledged;
- that durable recovery state records/reconstructs the same semantic location/custody; it does not become a competing live simulation owner;
- runtime projections/checkpoints that disagree with a committed DUR-03 transfer are stale projections and must reconcile before they can authorize another interaction.

Pure runtime/transient objects that have never crossed a durable value boundary may remain FND/gameplay-owned and rebuildable. They are not silently treated as acknowledged durable ItemInstances. The owning loot/content/combat contract must declare the materialization boundary explicitly.

## 7. Decision — mixed runtime↔durable transfer protocol

FND-03 forbids blocking remote/DB work inside the writer lane. Therefore pickup/drop and similar runtime-ground ↔ durable-character transfers use a semantic prepare/pending/commit/reconcile pattern.

### 7.1 Runtime prepare/reservation

The current runtime owner:

1. validates runtime scope, location, item/occurrence identity and GAME-ITEM legality;
2. allocates/consumes the stable DUR-03 TransactionId/operation/cause context;
3. reserves the affected runtime item/occurrence under current scope ownership generation and relevant runtime/domain revision;
4. makes the reserved value unavailable to competing mutation while pending;
5. issues a bounded asynchronous persistence request and yields the authoritative writer.

The reservation is not a second durable commit and cannot outlive authority without recovery/reconciliation semantics.

### 7.2 Durable commit

The game-owned PostgreSQL transaction is the **durable value linearization point** for a durable pickup/drop/materialization transaction.

It must validate/consume as applicable:

- stable TransactionId/OperationId/CommandRef/cause;
- item/occurrence identity and expected durable state;
- current CharacterLease/session requirements;
- current runtime scope ownership-generation fence or equivalent accepted durable fence;
- source/destination location/custody semantics;
- item/value conservation;
- receipt and mandatory audit evidence.

It commits all or none.

### 7.3 Runtime completion

The DB completion returns as a new normalized authoritative runtime input.

- known commit => current valid runtime owner finalizes/removes/materializes its runtime projection to match the committed semantic location;
- known abort => reservation can be released if the same logical operation terminates or safely retries;
- ambiguous commit => reservation remains non-spendable until DUR-03 reconciliation classifies the durable outcome;
- stale completion after ownership-generation change cannot mutate new runtime authority; the new owner/recovery path reconciles durable state/receipt instead.

### 7.4 Crash windows

If durable commit succeeds but runtime dies before applying completion:

- replacement/recovery must inspect committed location/receipt before rematerializing runtime state from an older checkpoint;
- a stale ground ghost cannot authorize a second pickup because durable eligibility/receipt and current generation reject it;
- runtime replay/checkpoint recovery never re-executes the gameplay value transaction.

This is not distributed 2PC. Runtime memory is a fenced single-writer participant/projection around one durable game-DB commit point.

## 8. Decision — durable drop and durable pickup consequences

### Durable item dropped to ground

If a previously acknowledged durable ItemInstance moves from Character inventory/equipment/container to ground and the drop is acknowledged:

- its new ground location/custody must be durably recoverable with exact WorldId/runtime scope/position semantics or an equivalent accepted recoverable representation;
- the runtime destination is reserved/validated before commit;
- durable commit removes old durable spendability and establishes the recoverable ground result atomically;
- runtime materializes/reconciles the committed ground projection;
- GameNode crash cannot make the acknowledged item disappear or reappear in old inventory.

### Pickup of an already durable ground ItemInstance

- runtime reserves the ground item;
- durable transaction changes the same ItemInstance's semantic location to Character/custody destination;
- after commit, runtime removes stale ground projection;
- lost response/crash reconciles same TransactionId and committed location.

### Pickup/materialization of runtime-only loot candidate

If owning gameplay declares the visible/runtime loot was not yet an acknowledged durable ItemInstance:

- pickup is a `MINT`/materialization transaction, not a transfer of a nonexistent durable item row;
- a stable loot/occurrence/output cause deduplicates the mint;
- fresh transaction-scoped ItemInstanceId is used for concrete output;
- retry/recovery cannot materialize the same occurrence twice.

The exact loot materialization point is owned by combat/loot architecture, but it must choose one model explicitly.

## 9. Decision — identity transitions

### Same lifecycle

Legal charge/durability/binding/compatible modifier/quantity state mutation preserves ItemInstanceId. A consumptive state change still requires typed cause/rule and before/after evidence where durability/security policy requires it.

### New lifecycle

New independently locatable item/stack gets a fresh ItemInstanceId allocated to the logical TransactionId/output slot before the first commit attempt.

That planned output ID remains stable across physical retry and is never reassigned to another logical transaction even if the original terminates without commit.

### Split

For S(q), `0 < x < q`: S keeps ID/quantity q-x; new stack N gets fresh transaction-scoped ID/quantity x. Moving all quantity is a move, not a split.

### Quantity transfer/merge

Receiver B keeps B ID; source A keeps ID while positive, retires when zero; total units exact; no temporary item needed solely for fungible units.

### Transform

Every Oteryn transform rule explicitly selects `PRESERVE_INSTANCE` or `REPLACE_INSTANCE`.

- preserve only one-input/one-output same-lifecycle transform;
- replacement retires old and uses fresh output IDs;
- one ID never becomes two live outputs;
- internal UUID policy is versioned Oteryn integrity semantics, not inferred from Global UUID behavior;
- unknown observable Reference transform remains parity-pending.

## 10. Decision — conservation classes

Every authoritative value mutation line is exactly one of:

```text
TRANSFER
SPLIT_MERGE_QUANTITY
STATE_MUTATION
MINT
BURN
TRANSFORM
CONVERSION
```

No unclassified signed authoritative delta.

Conservation means exact asset/item units plus complete input/output/source/sink lineage, not market-price equality.

Non-item fungible assets use exact bounded arithmetic; binary float is forbidden. Pure same-asset transfer has exact debits/credits; net creation/destruction requires mint/burn cause; conversion requires explicit versioned rule.

## 11. Decision — stable cause/provenance

Every mint/burn/transform/conversion must answer:

```text
what authoritative cause/rule allowed it?
which exact inputs participated?
which outputs resulted?
which TransactionId committed it?
was the same logical cause already applied?
```

Potential causes remain downstream-owned: loot resolution, reward grant, craft, static placement materialization, system source/sink, later admin compensation.

Same occurrence cannot mint twice; same cause with conflicting intent is an integrity conflict; repeatable sources require distinct authoritative occurrence identities.

## 12. Decision — CommandRef, TransactionId and OperationId

- FND-02 CommandRef remains player-command ingress identity/order.
- Every logical atomic durable item/value mutation uses one ANL TransactionId before first durable commit attempt.
- TransactionId identifies stable logical atomic intent, not a physical DB attempt.
- OperationId is used for logical workflows spanning multiple transactions, async continuation, cross-process/session retry or durable custody lifecycle.

Simple commands do not require OperationId when CommandRef + state preconditions fully bound retry semantics.

## 13. Decision — known abort versus ambiguous commit

### Stable intent

Same TransactionId preserves business intent, source/cause, requested mutation class, destination semantics and transaction-scoped planned output identities.

### Known non-committed abort

For a proven serialization/deadlock/non-commit:

- same TransactionId retained;
- implementation may reread current authoritative before-state and rematerialize legal mutable details/effect rows for the same intent;
- planned output IDs remain stable;
- if current state no longer permits the same intent, it rejects/terminates instead of adapting into another business operation;
- no external side effect may have escaped the aborted attempt.

### Ambiguous commit

Once commit outcome is ambiguous:

- exact materialized candidate/evidence set freezes;
- no different candidate under that TransactionId until classification;
- committed => return/reconcile exact result;
- proven non-committed => known-abort retry rules apply;
- unclassifiable => fail/hold; never guess another TransactionId.

## 14. Decision — durable receipts

Where CommandRef + current state alone cannot prove replay safety, durable receipt/reconciliation state distinguishes at least:

- not applied/safely retryable;
- committed/original result;
- terminal durable rejection where domain needs it;
- unresolved ambiguous outcome;
- conflicting identity/cause reuse.

Receipt/source-cause retention must cover the full replay/idempotency horizon. Exact physical storage/duration remains downstream.

## 15. Decision — atomic participant/effect set and isolation

Each physical commit attempt has a bounded closed materialized set including as needed item instances/locations/state, equipment/container claims, non-item asset lines, custody/workflow state, receipts, authority/fence state and mandatory audit/publication state.

Implementation requires hard ceilings for touched items, location/custody lines, value lines, transform I/O, container expansion, workflow participants, audit events/payload and reconciliation work.

DUR-02 anomaly discipline remains binding: READ COMMITTED only with proof, otherwise bounded SERIALIZABLE/stricter mechanism; deterministic acquisition or equivalent proof; advisory locks not sole authority; retry preserves semantic identities.

## 16. Decision — authority and reconnect nuance

Character-controlled mutation consumes valid GameSession/CommandRef where player-originated, current CharacterLease authority and current runtime ownership-generation fences for runtime participants.

Item binding/location/ItemInstanceId/NodeId are not credentials.

A command reserved under a valid same-GameSession binding may survive eligible connection_generation change. DUR-03 therefore requires current logical GameSession/lease/runtime authority and participant commit fences, not that the old transport generation itself remains current forever.

## 17. Decision — one current game DB atomicity boundary

Current atomic durable value mutation belongs to one game-owned PostgreSQL transaction inside `oteryn_game` under ADR-0004/DUR-02.

Reject default Platform/game distributed 2PC, cross-DB FK, mirrored dual item authority and implicit remote-service atomicity.

Mixed runtime/durable pickup/drop is not a cross-database distributed transaction: current runtime owner reserves/proposes under a fence, while the game DB transaction is the durable value linearization point and runtime later reconciles its projection.

## 18. Decision — multi-transaction typed custody

Multi-step workflows use stable OperationId where needed; each committed step independently conserves value; intermediate value moves to explicit typed immediate custody and is no longer spendable from old location; workflow is restartable/idempotent; compensation is new transaction; no hidden all-or-nothing across commits.

Trade/market/mail/reward business state machines remain downstream.

## 19. Decision — cross-world safety

Direct cross-world item/currency/value transfer is forbidden. Burn in world A + mint in world B is still semantically a transfer. Future world transfer needs explicit lineage, balance, custody, authority and recovery contract.

## 20. Decision — bounded durable audit

ADR-0006 makes security-relevant durable item/currency mutations durable audit. DUR-03 therefore requires ANL-compatible evidence sufficient to reconcile every transaction effect for which the owning security/value contract requires mandatory audit.

This is **not** an instruction to emit one durable event for every high-frequency item field tick. Non-security-critical typed item state mutations remain subject to their owning domain/ANL policy. When a state mutation participates in a DUR-03 value/security transaction, its required before/after cause/evidence is retained.

Evidence may aggregate multiple mutation lines into bounded transaction payload/events. If it cannot fit hard ceilings, operation stages safely or rejects; it never emits unbounded evidence.

Minimum semantic evidence as applicable: TransactionId; OperationId/CommandRef/cause; World/runtime scope; interpretation revisions; touched ItemInstanceIds; lifecycle/location/type/quantity/value before/after lines; mutation class; source/sink/rule; conservation summary; safe fence references.

Concrete event IDs/protobuf payloads/resource ceilings must be registered before implementation conformance.

## 21. Decision — restore/recovery

Before mutation resumes after restore/integrity incident verify supported schema, live item uniqueness/non-reuse consistency with retained policy evidence, one valid location, valid container/custody graph, legal quantities/revisions, no receipt/cause conflict, required retained audit complete sets, non-item asset invariants, newer recovery fencing and read-only audit replay.

For runtime scopes, recovery also reconciles committed DUR-03 transfer receipts/location state **before** rebuilding/interacting with ground items from older checkpoints. A checkpoint ghost cannot become a second pickup authority.

Failure keeps affected mutation closed; analytics diagnoses but does not auto-repair.

## 22. Decision — compensation

Committed history is immutable. Correction is a new authorized transaction with new TransactionId, causation to original, current authority and complete conservation/source-sink evidence. Raw row/history rewrite is not compensation.

## 23. Downstream surface boundaries

| Surface | DUR-03 owns | Downstream owner retains |
|---|---|---|
| pickup/drop/ground/inventory | atomic handoff/location/conservation | movement/interaction/runtime presentation |
| loot | materialization/mint/transfer idempotency/lineage | kill/loot generation/eligibility/materialization timing |
| trade | atomic exchange/custody safety | consent/lifecycle/policy |
| market | conservation/escrow safety | offer/fill/fee/pricing state machine |
| bank | exact item/ledger transfer/conversion | banking/economy policy |
| depot | item move/custody conservation | access/depot semantics |
| mail | item custody/move conservation | address/delivery lifecycle |
| rewards | mint/transfer idempotency | eligibility/schedule |
| houses | placement/move conservation | ownership/access/rent/topology |
| crafting/upgrades | input/output lineage | recipes/formulas/eligibility |
| entitlements | game-value delivery conservation if later accepted | Platform/payment/consumer activation policy |

## 24. Threat review

- duplicate CommandRef: FND-02 non-reexecution + durable receipt where needed;
- lost-response mint: same TransactionId + ambiguous reconciliation;
- stale writer: CharacterLease/runtime generation fence;
- runtime crash after DB pickup: recovery consults committed receipt/location before rematerialization;
- escrow double-spend: custody is immediate location, not side flag;
- split/merge ID duplication: explicit survivor/new-ID/retirement;
- cross-world laundering: burn+mint does not bypass transfer classification;
- restore remint: recovery fence + receipt/cause reconciliation + read-only replay;
- analytics repair authority: prohibited; compensation is new authoritative transaction.

## 25. Rejected alternatives

Reject:

- subsystem-specific peer location fields;
- generic authoritative JSON/EAV transaction/location payloads;
- market-price equality as conservation;
- CommandId alone for all multi-step/cross-session idempotency;
- event sourcing as gameplay mutation authority;
- blind new TransactionId on timeout;
- UUID ordering as survivor/authority;
- synchronous DB calls while holding runtime writer lane;
- runtime checkpoint as durable item commit;
- implicit Platform/game distributed transaction;
- analytics automatic repair.

## 26. Decision timing

### Must decide now?

**YES.**

### Blocked downstream work

- durable inventory/equipment/container/ground implementation;
- runtime↔durable loot/pickup/drop handoff;
- item/currency anti-duplication implementation;
- future safe custody for trade/market/depot/mail/rewards/houses;
- Game Intelligence item/value reconciliation;
- item/value crash/recovery/concurrency E2E.

### Hard migration cost if changed later

Late changes affect location authority, runtime/durable handoff, identity lineage, receipts, source/sink provenance, evidence interpretation, custody and restore proof.

### Supersession evidence

Valid: proven observable Reference mechanic incompatible with typed model; PostgreSQL anomaly/correctness evidence; measured scale evidence; accepted equivalent cross-service custody; exploit/security evidence; privacy/legal retention; explicit owner world-transfer/economy policy.

OTS schemas, framework preference and convenience are insufficient.

### Deliberately not decided

- SQL schema/index/lock syntax and Rust DB APIs;
- numeric participant/resource ceilings without evidence;
- concrete ANL event IDs/payloads;
- exact unevidenced Reference transform/crafting/loot/reward/decay/business rules;
- exact loot materialization timing before its owning combat/content contract;
- trade/market/bank/depot/mail/house state machines;
- cross-world transfer feature;
- external service/database custody protocol;
- production topology/RPO/RTO/backup cadence;
- automatic remediation.

## 27. Recommendation

Accept DUR-03 with this irreducible proof:

```text
one live durable item -> one immediate semantic location
new lifecycle -> fresh transaction-scoped ItemInstanceId
retired lifecycle -> ID never reused/reassigned

one atomic durable mutation -> one TransactionId
known abort -> retry/rematerialize same logical intent
ambiguous commit -> freeze/reconcile exact candidate

runtime-owned ground value
-> reserve under current ownership generation
-> async DB durable commit
-> reconcile runtime projection/recovery from committed result

pure movement -> no unauthorized source/sink
mint/burn/transform/conversion -> typed rule/cause + complete lineage

mandatory durable evidence where required -> commits with mutation
multi-step -> typed custody + OperationId where needed
restore -> fail closed until identity/location/receipt/audit/authority invariants reconcile
```

This unblocks a later separately authorized implementation without prematurely selecting physical schema or downstream gameplay business policy.
