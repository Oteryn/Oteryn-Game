# DUR-03 — Item Transaction and Anti-Duplication Contract

- Date: 2026-08-12
- Gate: `DUR-03`
- Delivery task: `OTV2-20260812-dur-03-item-transaction-architecture`
- Delivery PR: #207
- Status on delivery branch: **CANDIDATE / NONBINDING**
- Canonical semantic effect: only after accepted delivery merge; programme `ACCEPTED / LIFECYCLE_CLOSED` promotion requires a separate lifecycle closeout
- ImplementationStatus: **NOT_STARTED**
- Runtime authority: **NONE**
- PostgreSQL DDL/migration authority: **NONE**
- Production authority: **NONE**
- Analysis source: `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md`

## 1. Purpose

Freeze the minimum native transaction and anti-duplication semantics required before any implementation may claim authoritative durable item/currency/value correctness.

This contract owns:

- one authoritative immediate semantic location for every live durable ItemInstance;
- create/retire/split/merge/quantity-transfer/transform ItemInstanceId transition rules;
- item/currency/value transfer, mint, burn, transform and conversion conservation semantics;
- stable transaction/operation identity through retry and ambiguous commit;
- durable idempotency and reconciliation obligations;
- mixed runtime-owned ground ↔ durable item/value handoff semantics;
- authority/fencing requirements for durable value mutation;
- bounded atomic participant/effect rules;
- safe typed custody for multi-transaction workflows;
- mandatory durable provenance/evidence where value/security policy requires it;
- restore/recovery item/value integrity gates;
- cross-world and cross-authority fail-closed behavior.

It does **not** own runtime code, physical PostgreSQL schema, wire payloads, item legality, market/trade business policy, exact Reference formulas/rates or production rollout.

## 2. Authority chain

```text
stable authored item definitions / content identity -> ADR-0005 + DUR-04
ItemInstance semantic legality/equipment/container  -> GAME-ITEM-01
ItemInstanceId durable representation/non-reuse     -> DUR-01
common migration/transaction/outbox/PITR substrate -> DUR-02
item/currency/value transactions + conservation     -> DUR-03
event/audit identity/evidence/privacy               -> ANL-01
CommandRef ordering/duplicate ingress               -> FND-02
runtime ownership/order/fencing                     -> FND-03
GameSession/CharacterLease/recovery authority       -> FND-04
exact formulas/ruleset arithmetic                   -> SIM-DETERMINISM-01 + owning gameplay gates
loot/trade/market/bank/depot/mail/reward/house policy -> owning domain gates
```

No layer may redefine another owner's semantics for persistence convenience.

Where older coordination prose conflicts with later accepted FND-02/FND-04 authority semantics, accepted component contracts and the current-status overlay govern; DUR-03 does not revive historical `session_generation` or Gateway-issued canonical GameSession assumptions.

## 3. Scope vocabulary

### 3.1 Durable ItemInstance

For this contract, a **durable ItemInstance** is a concrete GAME-ITEM ItemInstance whose authoritative value/state has crossed, or is crossing, a durable acknowledgement/transaction boundary and therefore must survive ordinary process/GameNode restart according to DUR-02.

A runtime-only transient object/loot candidate that has never become acknowledged durable gameplay value may remain under FND/gameplay runtime ownership until its owning gameplay/content contract defines a materialization operation. DUR-03 does not silently turn every transient runtime object into a persisted item.

### 3.2 Runtime projection

A runtime ground/corpse/instance representation may be authoritative for immediate simulation/interactability under FND-03 while still being a projection/reconstruction of the same semantic item location for durability purposes.

It is never a second peer item-location authority.

## 4. Core safety theorem

For every committed logical durable value transaction `T`, a conforming implementation must prove:

1. `T` has exactly one semantic TransactionId.
2. Every pre-existing live durable ItemInstance touched by `T` is accounted for exactly once in the committed effect set.
3. Every surviving live durable ItemInstance has exactly one authoritative immediate semantic location after `T`.
4. Every newly created concrete durable ItemInstance has a fresh ItemInstanceId allocated to `T` and never assigned to another logical transaction/lifecycle.
5. Every retired ItemInstanceId is never reused.
6. Pure transfer/split/merge creates or loses no conserved units/value.
7. Mint/burn/transform/conversion occurs only under explicit typed authorized cause/rule and complete lineage.
8. Mandatory durable receipt/audit evidence commits atomically with the authoritative mutation where policy requires it.
9. Retry, duplicate command, stale authority, crash, timeout or lost response cannot create a second authoritative effect for the same logical transaction/operation.
10. Failed/aborted transactions leave no partial authoritative durable value mutation.
11. Runtime projection/checkpoint disagreement after a durable commit cannot authorize a second value mutation.

If one property cannot be proven for an operation class, that class is not DUR-03-conforming.

## 5. Canonical immediate-location model

### 5.1 Exactly one semantic immediate location

Every live durable ItemInstance has exactly one authoritative immediate semantic location:

```text
ItemInstanceId -> exactly one ItemLocationRef
```

This is a semantic relation, not a SQL layout mandate.

### 5.2 Typed location families

The architecture must support typed families as accepted owners require:

```text
CharacterInventory {
  character_id: CharacterId
  position: typed inventory position
}

CharacterEquipment {
  character_id: CharacterId
  occupancy: complete semantic equipment claim
}

Container {
  parent_item_instance_id: ItemInstanceId
  entry: typed container entry/position
}

Ground {
  world_id: WorldId
  runtime_scope: ChannelRef or InstanceRef
  spatial_position: typed world position
}
```

Future world-shared spatial state or downstream custody such as house/trade/market/depot/mail/reward custody is introduced only as a separately typed/versioned family with named owner and explicit WorldId/scope semantics.

### 5.3 `TypedDomainCustody` is not one generic variant

`TypedDomainCustody` is an architecture registry concept. Each accepted custody family defines its own stable semantic type/key, owner, scope, legal reference shape, lifecycle/compatibility and authorization boundary.

It may not be implemented as arbitrary free-form strings/JSON/EAV that acquire transaction authority.

### 5.4 Prohibited competing authorities

Reject as canonical:

- arbitrary location strings;
- generic JSON location objects;
- free-form EAV location fields;
- multiple independent nullable owner/location columns treated as peer authorities;
- generic `owner_id` whose meaning changes by subsystem.

### 5.5 Binding, custody and authorization are distinct

```text
WorldId value scope
binding/restrictions
immediate location/custody
current gameplay authorization
presentation ownership
```

remain separate. Possession, CharacterId equality, binding or custody does not grant mutation authority.

## 6. Runtime simulation authority versus durable recoverability

ChannelRuntime/InstanceRuntime may own immediate ground/corpse/transient-item simulation under FND-03. DUR-03 preserves that owner while requiring durable value safety.

Binding separation:

```text
runtime simulation owner
!= durable transaction coordinator
!= second semantic item location
```

For an acknowledged durable ItemInstance located in runtime ground/instance state:

- current runtime owner governs visibility/interactability under current ownership generation;
- DUR-02/DUR-03 committed state/receipt/provenance must be sufficient to recover the same semantic item/value result after ordinary process/node restart;
- durable recovery representation does not grant a second live simulation writer;
- a stale runtime checkpoint/projection that conflicts with a committed DUR-03 result is non-authoritative until reconciled.

A runtime-only transient object that never crossed a durable value boundary is outside durable ItemInstance guarantees until its owning contract materializes it.

## 7. Mixed runtime↔durable transaction protocol

FND-03 prohibits blocking network/database work while holding the logical mutation lane. Therefore a transaction crossing runtime ground/instance state and durable Character/value state uses this semantic protocol.

### 7.1 Runtime PREPARE / reservation

Current runtime owner processes a normalized authoritative input and:

1. validates semantic runtime scope, current ownership generation, item/occurrence/runtime revision and GAME-ITEM legality;
2. binds the intended operation to CommandRef/OperationId/cause and one DUR-03 TransactionId as applicable;
3. reserves the affected runtime item/occurrence/destination under current ownership generation and relevant runtime/domain revision;
4. makes reserved value unavailable to competing runtime mutation while pending;
5. issues a bounded asynchronous persistence request and yields the writer lane.

The reservation is not durable success and cannot self-grant authority after fencing.

### 7.2 Durable COMMIT / linearization

The game-owned PostgreSQL transaction is the durable value linearization point for the mixed operation.

It validates/consumes as applicable:

- stable TransactionId/OperationId/CommandRef/cause;
- item/occurrence identity and expected durable value state;
- current CharacterLease/session authority requirements;
- current runtime scope ownership-generation fence or an equivalently accepted durable fence;
- source/destination semantic location/custody;
- item/value conservation and definition compatibility;
- durable receipt/idempotency state;
- mandatory ANL audit/publication state.

All required durable mutation/evidence commits or none does.

### 7.3 Runtime completion / reconcile

DB completion arrives as a new normalized authoritative input.

- known committed => current valid runtime owner finalizes/removes/materializes runtime projection to match committed semantic location/result;
- known abort => runtime reservation may release or same logical transaction may safely retry;
- ambiguous => reserved value remains non-spendable until DUR-03 reconciliation classifies durable outcome;
- stale completion under old ownership generation cannot mutate new runtime authority; replacement/current owner reconciles durable receipt/state.

### 7.4 Crash after durable commit before runtime completion

If durable commit succeeds and runtime fails before consuming the result:

- recovery/replacement inspects committed DUR-03 location/receipt/provenance before rematerializing item state from older checkpoint/replay;
- stale checkpoint ground presence is a ghost projection and cannot authorize another pickup/mutation;
- audit/runtime replay never re-executes the durable gameplay transaction.

### 7.5 Not distributed 2PC

Runtime memory is a fenced single-writer participant/projection around one game-owned durable DB commit point. This contract does not create a distributed two-phase commit protocol between memory and PostgreSQL.

Safety derives from reservation/fencing, one durable linearization point, idempotent reconciliation and fail-closed recovery.

## 8. Durable drop semantics

When an acknowledged durable ItemInstance moves from Character inventory/equipment/container to runtime ground and drop success becomes durable:

- runtime destination/location is validated/reserved under current ownership generation;
- durable transaction removes old durable spendability and establishes a durably recoverable Ground location/result with exact WorldId/runtime scope/spatial semantics or an equivalently accepted recoverable representation;
- runtime materializes/reconciles the committed ground projection after durable commit;
- ordinary GameNode crash after durable success cannot make the item disappear or reappear at old Character location;
- if runtime cannot safely establish/recover the ground projection, the operation fails/holds according to its transaction state rather than acknowledging a lossy drop.

## 9. Pickup semantics

### 9.1 Pickup of an already durable ground ItemInstance

- runtime owner reserves the ground item under current generation/revision;
- durable transaction changes the same ItemInstance's semantic location to the legal Character/container/custody destination;
- commit also records receipt/audit where required;
- runtime removes/reconciles stale ground projection after known commit;
- lost response/crash reconciles same TransactionId/committed location, never a second copy.

### 9.2 Pickup/materialization of runtime-only loot candidate

If the owning combat/loot/content contract declares the visible runtime loot has **not** yet become an acknowledged durable ItemInstance:

- pickup/materialization is a `MINT` transaction rather than transfer of a nonexistent durable item;
- stable authoritative loot/occurrence/output cause deduplicates retry;
- concrete output uses fresh transaction-scoped ItemInstanceId;
- same occurrence cannot materialize twice.

The owning combat/loot/content gate must explicitly choose materialization timing; DUR-03 does not silently choose it.

## 10. Container immediate-parent semantics

GAME-ITEM-01 owns containment legality. DUR-03 requires:

- contained item immediate location = parent container;
- moving container changes root's immediate location, not every descendant relation;
- affected capacity/weight/type/nesting constraints validate before commit;
- destroy/replace of a container with live descendants is invalid unless the same bounded transaction explicitly gives every affected descendant a legal disposition;
- no committed orphan/cycle.

## 11. Item lifecycle and identity transitions

### 11.1 Same concrete lifecycle preserves identity

Legal typed-state mutation preserves ItemInstanceId where it remains the same concrete lifecycle.

Examples: charge/durability/binding/compatible modifier changes, quantity adjustment of one existing stack, explicitly identity-preserving one-to-one transform.

A consumptive state mutation still has explicit owning cause/rule and required before/after evidence where durability/security policy requires it. `STATE_MUTATION` is not unexplained value drift.

### 11.2 New lifecycle gets fresh identity

Every new independently locatable concrete item/stack gets a fresh ItemInstanceId.

### 11.3 Transaction-scoped planned output identities

Every new ItemInstanceId planned by a logical TransactionId/output slot is allocated before the first durable commit attempt that could make it authoritative and remains assigned to that logical transaction/output slot across physical retry.

Rules:

- serialization/deadlock retry does not replace planned output IDs;
- ambiguous commit reconciles exact same output IDs;
- an output ID allocated to one TransactionId is never reassigned to another logical transaction, even if the first later terminates without commit;
- no uncommitted output is exposed as authoritative live item before commit.

### 11.4 Retirement

When concrete lifecycle ceases to exist, ID is terminal and never reused. Retained retirement/tombstone evidence follows DUR-01/ANL/privacy policy; DUR-03 does not require unlimited standalone tombstone history.

### 11.5 Quantity zero

A stack reduced to zero retires in the same atomic outcome.

## 12. Split semantics

For `S(q)` and `0 < x < q`:

```text
S: same ItemInstanceId, quantity q-x, remains live
N: fresh transaction-scoped ItemInstanceId, quantity x
```

`x == 0` invalid. `x >= q` is not split. Moving all quantity is moving existing S. Exact quantity and all legality/location constraints commit atomically.

## 13. Quantity transfer / merge

For exact `x` from compatible A to B:

- B keeps B ID and grows within bounds;
- A decreases x;
- A keeps ID if positive, retires if zero;
- no temporary item required solely for fungible units;
- exact conserved units unchanged.

UUID/client list ordering never selects survivor/receiver.

## 14. Create/mint semantics

New concrete item/stack uses transaction-scoped fresh ItemInstanceId.

A typed source may mint quantity into an existing compatible stack without temporary ItemInstance, but exact source/cause and quantity lineage are mandatory.

Every value-producing operation provides stable typed authoritative source/occurrence identity sufficient to prevent duplicate application. DUR-03 does not decide loot/reward/craft/business eligibility.

Same occurrence cannot mint twice; same occurrence with conflicting output intent is integrity conflict; repeatable sources use distinct occurrence identities.

## 15. Destroy/burn semantics

Burn/destruction identifies affected item/quantity/asset, typed sink/cause, survivor/retirement result and required lineage/evidence.

Silent row deletion, `quantity=0` live state or disappearance during recovery is not a valid sink.

## 16. Transform semantics

### 16.1 Explicit internal Oteryn identity policy

Each executable type-changing transform rule explicitly selects:

```text
PRESERVE_INSTANCE
REPLACE_INSTANCE
```

This UUID/lifecycle policy is internal Oteryn integrity semantics. External Global/Tibia behavior does not expose Oteryn ItemInstanceId and therefore cannot directly determine this UUID choice.

Reference evidence constrains observable transform semantics. If observable behavior is unknown, transform remains parity-pending/unsupported for claimed Reference execution.

### 16.2 `PRESERVE_INSTANCE`

Allowed only one-input/one-output when versioned Oteryn rule declares the same concrete lifecycle, resulting state is compatible/fully defined and one ID produces no second live output.

### 16.3 `REPLACE_INSTANCE`

Input retires; each concrete output uses fresh transaction-scoped ItemInstanceId.

### 16.4 Multi-input/multi-output

Every input disposition and output identity is explicit. One ItemInstanceId may never become two live outputs.

## 17. Conservation classifications

Every authoritative value mutation line has exactly one semantic class:

```text
TRANSFER
SPLIT_MERGE_QUANTITY
STATE_MUTATION
MINT
BURN
TRANSFORM
CONVERSION
```

No generic unclassified signed delta.

- `TRANSFER`: existing value changes immediate location/custody.
- `SPLIT_MERGE_QUANTITY`: exact units redistribute among compatible stacks.
- `STATE_MUTATION`: same lifecycle changes legal typed state under explicit cause/rule; it cannot silently alter stack quantity or non-item ledger outside owning classes.
- `MINT/BURN`: value enters/leaves under explicit source/sink.
- `TRANSFORM`: explicit versioned input/output/lifecycle rule.
- `CONVERSION`: explicit exact asset-A input/debit -> asset-B output/credit rule.

Market price/historical economy state is never conservation truth.

## 18. Non-item fungible value

Non-item balances are not forced into ItemInstance.

For each asset:

- owning domain defines asset identity/denomination and legal account/custody scopes;
- arithmetic is exact and bounded;
- binary floating point is not authoritative conservation basis;
- pure same-asset transfer balances exact debits/credits;
- net creation/destruction uses explicit mint/burn cause;
- conversion uses explicit versioned rule;
- retry/ambiguous commit follows TransactionId/OperationId contract.

Exact SQL scalar/business policy is deferred.

## 19. World-scope conservation

- each live ItemInstance stays within one WorldId value scope by default;
- direct cross-world item/currency/value transfer is forbidden;
- burn in world A plus mint in world B remains semantically cross-world transfer and cannot bypass policy;
- future world transfer requires explicit identity/value lineage, balance, custody, retry/recovery and authority contract.

## 20. Transaction identity

Every logical atomic durable item/currency/value mutation has exactly one ANL TransactionId allocated before first durable commit attempt.

TransactionId identifies logical atomic **intent**, not physical DB attempt.

Across attempts, stable intent includes as applicable source/cause, requested mutation class, logical participants/destination semantics and transaction-scoped planned output identities.

Same TransactionId with different business intent is integrity conflict.

A new TransactionId requires prior logical transaction proven terminal plus intentionally new logical transaction. Timeout alone is not terminal-abort proof.

## 21. CommandRef boundary

For player-originated commands, FND-02 `CommandRef=(GameSessionId,CommandId)` remains ingress identity/order and duplicate non-reexecution authority.

DUR-03 does not mint a new transaction merely because transport reconnects or connection_generation changes inside the same eligible GameSession continuity.

## 22. OperationId boundary

Use OperationId when logical value workflow spans multiple durable transactions, continues asynchronously, may resume/retry across GameSessions/processes or has durable workflow/custody lifecycle.

OperationId remains stable for same logical workflow. It is not required for every simple CommandRef-bounded transaction.

## 23. Retry: known abort versus ambiguous commit

### 23.1 Stable logical intent

Same TransactionId never changes business intent, source/cause, requested mutation class, destination semantics or planned output identity slots.

### 23.2 Known non-committed abort

For proven serialization/deadlock/non-commit:

- same TransactionId retained;
- current authoritative before-state may be reread and mutable effect rows rematerialized for the same intent;
- planned output IDs remain stable;
- if current state makes same intent illegal, reject/terminate rather than morphing into a different operation;
- no external side effect may escape aborted attempt.

### 23.3 Ambiguous commit

Once commit outcome is ambiguous:

- exact materialized candidate mutation/evidence/output-ID set freezes for reconciliation;
- no different candidate under same TransactionId until classification;
- committed => return/reconcile exact result;
- proven non-committed => retry under known-abort rules;
- unclassifiable => fail/hold; never guess a new TransactionId.

## 24. Durable idempotency / receipts

Where FND-02 ingress + current state cannot alone prove replay safety, durable receipt/reconciliation state distinguishes at least:

```text
NOT_APPLIED / safely retryable
COMMITTED / original result
TERMINAL_REJECTED where domain persists it
AMBIGUOUS / reconciliation required
CONFLICT / same identity with different intent
```

Receipt/source-cause retention covers owning replay/idempotency horizon. Exact storage/duration is downstream.

## 25. Ambiguous outcome algorithm

```text
execute logical T

known commit:
    reconcile/return committed result

proven abort and retry permitted:
    retry same logical T / same TransactionId

ambiguous:
    inspect durable receipt/state/evidence for frozen candidate
    committed -> return original
    proven non-committed -> retry same logical T
    unknown -> fail/hold
    never mint a guessed second TransactionId
```

## 26. Compensation after commit

Committed historical mutation/audit facts are immutable.

Correction uses new compensating transaction with new TransactionId, causation/reference to original, current authorization and complete source/sink/conservation evidence.

Raw row/audit rewrite is not compensation.

## 27. Atomic participant/effect set

Each physical commit attempt has bounded closed materialized participants/effects as needed:

- ItemInstances and immediate locations;
- item capability/quantity state;
- equipment/container claims;
- non-item asset accounts/lines;
- custody/workflow state;
- receipts/idempotency;
- authority/fence state;
- mandatory ANL audit/publication state.

Set cannot expand without bound during commit.

## 28. Resource bounds

Before implementation acceptance, absolute hard ceilings exist for externally influenced/amplification-prone DUR-03 structures, including touched ItemInstances, location/custody lines, value lines, transform I/O, container expansion, workflow participants, audit event/payload contribution and retry/reconciliation work.

Ruleset/product bounds may be lower. This architecture does not invent numeric values without evidence; missing ceilings block implementation and never mean unlimited.

## 29. Isolation, locks and anomaly closure

DUR-02 remains binding: name invariant, identify authority rows/constraints, prove anomaly closure under isolation/locks/constraints.

DUR-03 additionally requires:

- application-only check-then-write insufficient for location/conservation;
- deterministic lock/acquisition order or equivalent anomaly proof;
- READ COMMITTED only explicit proof, otherwise bounded SERIALIZABLE/stricter accepted mechanism;
- deadlock/serialization retry preserves TransactionId/OperationId/CommandRef;
- advisory locks not sole durable location/custody/uniqueness/conservation authority.

Exact SQL syntax deferred.

## 30. Player/character authority fencing

A transaction mutating Character-controlled value consumes applicable current authority:

- valid GameSession/CommandRef for player-originated intent;
- current CharacterLease authority/generation;
- current actor/domain preconditions;
- current runtime scope ownership generation for channel/instance participants.

Binding/location/ItemInstanceId/NodeId are not credentials. Client item/quantity/location claims are intent only and must be authoritatively revalidated.

## 31. Connection-generation nuance

A previously reserved CommandRef may survive eligible reconnect of same GameSession while connection_generation advances.

DUR-03 does not require the old transport generation itself at DB commit. It requires originally valid authoritative ingress, current logical GameSession/lease/runtime authority under FND continuation rules and current participant commit fences.

## 32. Channel/instance fencing

Durable mutation touching channel/instance-scoped ground/custody proves current runtime-scope ownership generation or equivalently accepted durable fence.

Stale former runtime owner cannot commit after authority moved. NodeId alone is not authority.

Future world-shared spatial owner uses separately typed location/authority family, not channel-local disguise.

## 33. Equipment atomicity

GAME-ITEM owns equip legality. DUR-03 requires all old location/claims, complete new occupancy, legal displacement result and required receipt/audit to commit all-or-none.

No half two-hand/mutually-exclusive claim.

## 34. Multi-transaction typed custody

Future workflow may span transactions only when every committed step is safe:

```text
stable OperationId where needed
-> move value into explicit typed custody
-> idempotent workflow steps
-> move/transform value out of custody
```

After custody commit, value is not spendable from prior location. Each step has own TransactionId and is conservation-safe. Workflow is restartable/idempotent. Compensation is new transaction. No hidden end-to-end atomicity across separate commits.

Owning domain defines business lifecycle/eligibility.

## 35. Current database authority boundary

Current atomic durable DUR-03 mutation uses one game-owned PostgreSQL transaction inside `oteryn_game` under ADR-0004/DUR-02.

Not authorized:

- Platform/game distributed 2PC;
- cross-database FK;
- mirrored dual item authority;
- implicit remote-service atomicity.

Mixed runtime/durable transaction from sections 7-9 is not cross-DB 2PC: runtime owner reserves/proposes under one generation; game DB transaction is durable linearization; runtime reconciles projection afterward.

A future external persistence/service custody boundary requires dedicated safe handoff/custody contract.

## 36. StaticItemPlacement materialization

Materialization occurrence has stable cause identity sufficient to deduplicate crash/retry. New concrete outputs use transaction-scoped ItemInstanceIds. Same one-shot occurrence cannot mint again; same occurrence with conflicting output intent is conflict; repeatable behavior uses distinct occurrences under owning content/gameplay policy.

## 37. Loot/reward/crafting boundary

DUR-03 does not decide loot drops, eligibility, rewards schedule, recipes, prices or entitlements.

Owning domain supplies stable authoritative cause/rule context so retry/recovery cannot apply same output twice.

Owning combat/loot/content architecture also declares whether visible runtime loot is already a durable ItemInstance or becomes durable only at a later materialization boundary; DUR-03 supports both only when the boundary is explicit and crash/retry-safe.

## 38. Surface ownership boundary

| Surface | DUR-03 owns | Downstream owner retains |
|---|---|---|
| pickup/drop/ground/inventory | atomic runtime↔durable handoff/location/conservation | movement/interaction/runtime presentation |
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

No surface becomes accepted merely because DUR-03 supplies transaction/custody primitives.

## 39. Mandatory durable evidence boundary

ADR-0006 requires durable audit for security-relevant durable item/currency mutation. DUR-03 therefore requires ANL-compatible durable transaction evidence sufficient to reconcile every effect whose owning value/security policy declares mandatory audit.

This does **not** require one durable event for every high-frequency non-security-critical item field tick. Typed item state mutations outside mandatory DUR-03 value/security audit remain under owning gameplay/ANL policy.

When mandatory:

- evidence may aggregate multiple mutation lines into one or more bounded transaction events/payloads;
- if evidence cannot fit hard ceilings, operation safely stages or rejects, never emits unbounded event;
- materialized evidence candidate is fixed before its physical commit can become ambiguous and commits with authoritative mutation.

Minimum semantic evidence as applicable:

- TransactionId;
- OperationId/CommandRef/cause;
- WorldId/runtime scope;
- interpretation revisions;
- touched ItemInstanceIds;
- lifecycle/location/type/quantity/value before/after lines;
- mutation class;
- typed source/sink/transform/conversion rule/cause;
- conservation summary;
- safe fence references without secrets.

Concrete ANL event IDs/protobuf payloads and numeric resource ceilings are registered before implementation conformance, not guessed here.

### 39.1 Closed one-item MINT/TRANSFER audit aggregate

This scoped representation decision resolves [#513 escalation 5854890321](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5854890321)
within the existing DUR-03/ANL-01 authority. It supplies the semantic basis for a
candidate typed schema and separately allocated offline deterministic measurement;
it does not register or activate a production event. The earlier
[decision packet](reviews/OTERYN_DUR03_REFERENCE_ONE_ITEM_AUDIT_RESOURCE_DECISION_PACKET_2026-09-27.md)
remains preserved as nonbinding proposal/evidence. General §39 remains binding.

The selected narrow shape is a small existing ANL envelope plus one closed typed
aggregate payload for each distinct logical transaction:

- `MINT`: one fresh transaction-scoped ItemInstance lifecycle, absent before and
  live after, established in typed `Ground` custody with applicable corpse
  association/provenance;
- `TRANSFER`: that already-existing live ItemInstance moves from typed `Ground`
  custody to direct-root `CharacterInventory`, preserving identity, type and
  quantity and leaving exactly one authoritative immediate location.

MINT and the later TRANSFER are separate transactions, with separate
TransactionIds, event candidates and atomic boundaries. Aggregation does not
combine their sequence into one commit. This child does not support mint into an
existing stack, multiple touched items, quantity redistribution, burn, transform,
non-item accounts, nested containers or additional custody families. Unsupported
shapes reject instead of acquiring meaning through a generic delta, metadata bag
or unbounded repeated effects. The quantity-one private fixture is not an accepted
Content definition or a production quantity ceiling.

Each aggregate covers the complete applicable §39 semantic evidence. This includes
typed item identity/lifecycle/type/quantity before and after; location/custody
before and after; authorized source/occurrence/cause and conservation summary;
WorldId and applicable concrete runtime scope; compatible interpretation and
definition revisions; and safe applicable fence references without secrets.
MINT absence is explicit semantic nonexistence, not a zero-quantity live item.
TRANSFER represents complete removal and establishment, or an equivalent typed
before/after pair, without partial or competing custody truth. Corpse association
does not introduce a new generic location authority.

Applicable ANL common correlation and durability fields stay in the envelope;
domain state and provenance stay in the typed payload. Player-originated pickup
uses its actual CommandRef. Server-originated mint does not invent a player command.
OperationId is present only when the owning durable workflow requires it. Evidence
references identify expected bindings; independently current session, lease,
runtime and content facts remain necessary to authorize any future mutation.

The selected design has one complete TransactionEventRef membership entry for
the aggregate, with `ordinal=1` and `count=1` for its own TransactionId. These are
design cardinality and complete-set semantics, not an accepted emitted count,
numeric resource maximum or measurement. No applicable mandatory effect/evidence
may be omitted to retain that cardinality. A shape needing additional mandatory
evidence must reject or obtain a later reviewed semantic/membership decision before
claiming conformance; it cannot hide an incomplete set under a separate count.

The candidate family uses `DURABLE_AUDIT` and a candidate privacy floor of at least
`RESTRICTED_PLAYER_LINKED`. Character/session linkage cannot be downgraded; an owning
security purpose may require `SECURITY_SENSITIVE`. This floor does not accept an
item purpose, retention profile, duration, roles, export/redaction, expiry or legal
hold policy. The Character bootstrap profile and its duration are not inherited.
Audit expiry and receipt/source-cause replay/non-reuse horizons remain distinct;
expiry cannot reopen mint eligibility or authorize identity reuse.

Before possible commit ambiguity, the EventId, complete TransactionEventRef,
immutable semantic envelope values and exact payload bytes are fixed. Ambiguous
retry/reconciliation reuses that frozen candidate and does not serialize mutable
domain state or mint a guessed new TransactionId. Proven noncommit follows §23.2
without weakening ANL immutable event-admission rules. Lost pickup acknowledgement
cannot restore Ground custody; duplicate delivery has no second consumer effect;
audit replay cannot mint, transfer or otherwise mutate gameplay.

Missing schema, profile or numeric acceptance keeps canonical admission and
implementation conformance gated. A fresh explicit allocation may measure an
unregistered candidate offline with synthetic fixtures only, while retaining those
gaps and separately reporting actual payload, envelope, aggregate count/bytes and
retained carriers. Existing ANL ceilings remain conjunctive; fixture identifiers,
synthetic budget probes and measured candidate sizes are not production acceptance.

This decision selects no event type ID, protobuf field number, accepted schema,
retention profile/policy, production hard maximum or registry entry. It grants no
SQL/outbox/runtime implementation, PREPARE/COMMIT, collection, Combat activation,
replay mutation, production or Reference-parity authority. Any successor requires
fresh exact allocation and applicable DUR/ANL/data-integrity/privacy review;
registry paths remain serialized under the live control plane.

### 39.2 Native MINT audit binding and staged admission

This bounded technical decision follows [the native-admission escalation
5857709826](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5857709826)
and [documentation allocation
5857774964](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5857774964).
It specializes the next native audit-binding gate under §39.1, GAME-ITEM-01 and
ANL-01; it neither supersedes their invariants nor admits a production event.

**Selected staging:** bind the native MINT snapshot/audit first, without requiring
an invented Inventory-root representation to start that work. The subsequent
TRANSFER remains a separate transaction under §39.1; this staging does not
combine commits, change materialization timing owned by combat/loot/content, or
accept pickup before its native destination and authority prerequisites exist.

The native MINT aggregate binds complete applicable evidence, not fixture aliases:

- Explicit semantic absence before and one fresh transaction-scoped live
  ItemInstance after, with ItemInstanceId and WorldId. Absence is not a nil
  identity or zero-quantity live state.
- The native stable namespaced ItemTypeKey and explicit compatible definition/
  ruleset/content revision context under GAME-ITEM-01 and DUR-04. Private numeric
  mappings and revision-scoped compiled handles cannot replace canonical type
  identity; this decision selects no key grammar, encoding or gameplay quantity.
- The complete definition-legal typed after-state and actual typed Ground,
  including its concrete World/runtime scope and world-position interpretation.
  Corpse association/provenance is not another immediate location authority.
- The owning materialization/loot domain's eligible stable occurrence/output-cause
  and compatible rule/revision binding, sufficient to deduplicate the same output
  across crash/retry. This records independently authorized source facts; an audit
  record, arbitrary UUID or synthetic source-kind value cannot create eligibility.
- Actual applicable ANL common context, complete TransactionEventRef membership
  under §39.1, safe expected fence references without secrets, and conservation
  evidence. Server-originated MINT invents no player command; OperationId exists
  only when the owning durable workflow requires it.

The owning source contract must supply the actual typed occurrence/output-cause
reference and eligibility meaning. The exact source reference/domain is not
defined here. Likewise, native Content identity alone does not accept its typed
state, stack semantics or placement legality. Unknown source or item legality
keeps admission closed; a synthetic fixture or caller assertion is not a substitute
for independent current owning facts. Immutable expected bindings cannot grant
current Character/session/lease/runtime authority.

TRANSFER additionally requires the legal native direct-root CharacterInventory
reference/position and applicable capacity/placement rules from GAME-ITEM-01 and
the owning pickup/current-authority contract. No new root identifier domain,
slot family or numerical ceiling is selected. Its before-state/source must bind
the actual existing item and Ground, not a freshly substituted internally
consistent snapshot; identity, type, quantity and one immediate location are
preserved. These destination prerequisites do not retroactively gate preceding
MINT schema/codec/profile design, but remain mandatory before TRANSFER admission.

Historical `oteryn.events.candidate.v1/v2`, their private numeric keys, synthetic
World/root/source facts and literal byte evidence remain unregistered evidence,
not compatible canonical production revisions. A separately accepted native
schema/registry binding must not reinterpret those bytes or reuse their fixture
identities as production authority. ANL-01 remains normative: same EventId fixes
exact payload bytes/hash and all immutable semantic envelope values. Equivalent
outer-envelope serialization ordering alone is not a content conflict; the v2
fixture's strict whole-envelope ordering is not a general ANL admission rule.

The accepted [item-specific P90D logical
profile](reviews/OTERYN_DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_DECISION_2026-09-27.md)
supplies purpose, privacy floor, authorized audited readers/export, immutable
timestamp/expiry, bounded deletion and explicit hold/evolution constraints.
Character retention is not substituted. The actual native event family must
bind that accepted item policy through reviewed registry admission; this paragraph
does not create a serialized profile entry or prove runtime retention enforcement.
Audit expiry stays independent of receipt/source-cause protection and non-reuse.

This decision concretely supplies native identity/snapshot and compatibility
inputs for separately allocated schema/codec and item-profile authoring. Resource
qualification must use the resulting actual native grammar and all applicable
copies/work; neither candidate widths nor wire integer widths become production
maxima. Production admission remains conjunctively gated on accepted actual
schema/event/profile bindings, complete mandatory evidence, registered applicable
hard resource bounds checked before allocation, legal current source/item/scope/
destination facts and applicable live fences, and proven owning atomic mutation/
audit/receipt/publication and recovery mechanisms. No SQL/runtime path, registry
mutation, PREPARE/COMMIT, collection or gameplay authority follows from this text.

Decision test: **must decide now YES**, to avoid freezing synthetic identity into
native encoding and unnecessarily coupling MINT to an unresolved root domain.
Promoting v2 is rejected because its source/type/root semantics are synthetic;
waiting for all TRANSFER representation choices is rejected as unnecessary for
preceding native MINT binding. Native MINT-first has the smaller implementation
surface but leaves TRANSFER admission and playability incomplete. Late identity/
source reinterpretation would require schema and historical-evidence migration;
explicit accepted domain changes, compatibility/security/privacy findings or
measured native encoding/resource evidence can justify reviewed supersession.
Field/event IDs, source/root domains, gameplay limits and physical mechanisms
remain deliberately undecided. Fresh exact allocations and independent
DUR/ANL/data-integrity/privacy review remain required for all successors.

### 39.3 Generic native one-item binding and staged destination admission

This generic Game specialization records the native semantic bindings needed by
the B1 allocation on #162. It supplies no production event or implementation
authority and applies across item families and gameplay domains. Rat, Gold Coin
and cheese may be named only as deterministic fixtures; no identity-specific
branch or production roster follows from them.

Evidence classification for this specialization:

- **PROVEN** — existing DUR-03 §§39.1-39.2, GAME-ITEM-01, ANL-01, VSL-COMBAT-01,
  and Content typed-definition rules supply the inherited invariants stated
  below; migration 0009 supplies the stated conditional Character progression
  guards.
- **DERIVED** — the native bindings and admission gates proposed below specialize
  those invariants for the allocated generic one-item path; they are not by
  themselves production admission or runtime permission.
- **RESOLVED (formerly CONFLICT)** — how a non-XP CharacterInventory transfer
  relates to the migration 0009 global CharacterRevision/XP-receipt chain is
  decided by `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`
  (`docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md`,
  protected on `main@74bb3fd`); see the composition paragraph below.
- **UNKNOWN** — source-reference/receipt grammar, position/capacity policy,
  physical schema and runtime composition remain with their owning allocations.

**Native item definition and state.** The canonical item definition uses the
existing Content typed-definition shape:

```text
TypedDefinitionRef = (DefinitionFamily, ProductionKey, DefinitionRevisionRef)
```

For an item, that reference resolves to the stable namespaced `ItemTypeKey` and
an explicit compatible immutable definition/revision context under GAME-ITEM-01
and DUR-04. A private numeric ID or compiled handle is revision-local and cannot
replace this identity. MINT binds semantic absence before and one fresh
transaction-scoped live ItemInstance after, including the complete typed
capability state allowed by that exact definition and compatible ruleset/content
revisions. Semantic absence is not a nil ID, zero quantity, or an incomplete
state. Non-stackable presence and stack quantity follow the accepted item
definition; this decision supplies no quantity value. Unsupported capability,
shape, incompatible revision, or unknown required state fails closed rather than
being accepted through a generic metadata field.

**Committed death and output cause.** A MINT source is a deterministic loot
output occurrence descended from one committed `CreatureDeathOccurrenceRef`,
bound to the applicable typed output definition and exact compatible loot,
content, ruleset and SIM revisions. The owning Combat/SIM source determines
eligibility and a stable semantic output occurrence; a caller-provided UUID,
audit EventId, or arbitrary source label cannot create that authority. Retry,
reconciliation and process restart must resolve the same committed death and
output cause to the same terminal mint result. The same one-shot output cannot
mint again after restart or after audit expiry. Event and item IDs remain owned
by their existing authorities and are not substitutes for the semantic cause.
`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` (owner decision D52,
`reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md`)
fixes the death key and output cause and defines the terminal result across a
restart: a MINT committed before the death's runtime-scope ownership generation
ended keeps that committed result, and a MINT not committed by then is
terminally not minted. No later generation or process attempts it again, and
the structurally non-reused key means none can. Receipt schema and retention
duration remain owner and registry gates.

**Actual native Ground context.** MINT establishes the item in the actual typed
Ground for the same `WorldId` and `ChannelId` that own the native runtime
placement. The binding includes the accepted map and content revisions, native
room/placement context, and typed spatial position under their owning Content,
FND and runtime contracts. Corpse association is provenance/projection only,
never a competing item location. A fixture coordinate, caller-selected room,
stale map/content binding, or unsupported runtime scope is not sufficient.
Current runtime-scope ownership and other applicable live fences are evaluated
independently at mutation time.

**CharacterInventory destination.** The later TRANSFER binds the real direct-root
`CharacterInventory` as the existing semantic pair `CharacterId + typed
inventory position`, with position legality supplied by GAME-ITEM-01 and the
Character/pickup owner. No `InventoryRootId`, new root domain, slot family, or
numeric capacity is introduced. MINT-first work remains admissible while
TRANSFER placement policy is unresolved, as §39.2 permits; TRANSFER itself is
not admissible until a legal destination position and all applicable capacity,
placement and current-authority rules are accepted and proven.

**Expected bindings versus current authority.** The immutable MINT/TRANSFER
candidate binds expected item definition/state, source occurrence, WorldId,
ChannelId, content/map/runtime context, destination and safe fence references.
Those expected values do not authorize a write. Admission independently checks
current compatible Content, Character/session and CharacterLease, runtime-scope
owner/generation, source eligibility, item legality and operation preconditions.
For player pickup, the current CommandRef and applicable current fences remain
required; no client claim or stored binding substitutes for them. ANL-01
immutability, exact payload-byte reuse for same-EventId ambiguity, complete
TransactionEventRef membership, and DUR-03 non-reuse/idempotency continue to
apply.

**Separate Character readiness and composition dependency.** Character-owned
progression initialization/readiness remains a separate prerequisite before
Combat D/E admission and any later Combat XP settlement, under its authorized
owner route and fresh, separately allocated Character revision/policy binding.
Migration `0009_character_progression.sql` is a
conditional composition dependency: it permits bootstrap-only revision one,
then requires every global CharacterRevision successor to match typed
progression state and an immutable XP receipt. How a non-XP inventory TRANSFER
composes with that XP-only receipt chain is decided by
`CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`: a DUR-03 transaction whose
only Character effects are CharacterInventory item locations and DUR-03 cause
records keyed by a Character does not advance CharacterRevision and writes no
Character root, progression or XP-receipt row; it is fenced like
`commit_character_experience` (recovery fence, admission-relation locks,
reconnect-session row, runtime-scope assignment, admission guards, cause and
source-scope binding) and serializes on the `character_root` row lock. No XP
receipt is invented for inventory work, and 0009 is unchanged. That decision
settles only the global-revision composition: destination position, capacity
and TRANSFER admission stay with their owners, and TRANSFER remains closed until
they are accepted and proven. MINT does not touch Character state and is not
blocked by this destination question.

**Decision test.** Must decide now: **YES** for definition/state, source, and
actual Ground bindings, so native MINT cannot inherit synthetic fixture
identity; **NO** for the unresolved TRANSFER destination position, capacity and
admission, which remain gated at the Character-owner boundary (the global-revision
composition is settled by `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`). The realistic alternatives for MINT are
to admit only with the complete typed native bindings above, or keep MINT closed
until every later Character destination question is settled. The first preserves
the already selected MINT-first staging without weakening any MINT invariant;
the second unnecessarily couples a Character-independent creation to pickup.
**Recommendation:** use the complete generic native bindings above for later
MINT qualification, while leaving TRANSFER closed until Character position,
capacity and TRANSFER admission are accepted and proven by their owners; the
global-revision composition follows `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`. The main
risk is that a weak or expiring source-cause record could permit a repeated mint;
late changes to definition/source semantics would require retained evidence and
receipt interpretation to migrate. No broad new identity, receipt, or authority
abstraction is selected.
Supersession requires accepted owner contracts, compatibility or security
findings, or measured native qualification evidence.

This section does not decide quantity, probability, stack maximum, inventory
capacity, loot or XP formulas, HP/damage, XP values, protocol/event IDs, registry
ceilings, SQL/runtime permission, production retention configuration, or
physical implementation. Unknown or unsupported native input remains closed.

## 40. Durable acknowledgement

For a durable DUR-03 mutation:

```text
success acknowledged
=> durable game DB transaction committed
=> required durable receipt/audit committed
=> ordinary process/GameNode restart can reconstruct the committed value result
```

Runtime checkpoint, network send or in-memory projection is not durable success.

For runtime-ground transactions, user-visible completion may additionally wait for current runtime reconciliation as required by owning gameplay/FND implementation, but no acknowledgement may precede durable commit or claim a result that cannot safely recover.

## 41. Restore and disaster recovery

Before authoritative item/value mutation resumes after restore/integrity incident, validate at least:

- supported schema/migration history;
- valid/unique live ItemInstanceIds consistent with retained non-reuse evidence required by policy;
- exactly one valid immediate semantic location per live durable item;
- valid parent container/custody graph;
- legal quantity/capability/definition revisions;
- no TransactionId/OperationId/source-cause receipt conflict;
- mandatory retained audit TransactionEventRef sets complete where required;
- non-item asset invariants;
- newer recovery fence blocks pre-loss GameSession/lease/runtime authority;
- audit replay cannot execute gameplay/remint;
- runtime recovery reconciles committed DUR-03 receipts/location before rematerializing ground/item projections from older checkpoints.

Integrity failure keeps affected mutation closed until explicit safe repair/compensation path.

## 42. Analytics/Game Intelligence boundary

Analytics may reconcile evidence, detect duplicate location/lineage/value patterns, raise cases and reconstruct provenance.

It may not mutate item/value authority, auto-delete/merge duplicates, mint compensation, rewrite history, autonomously sanction under DUR-03 or bypass domain authorization.

Correction uses new typed authorized transaction under owning gameplay/admin/security contract.

## 43. Fail-closed dispositions

| Condition | Category | Required effect |
|---|---|---|
| invalid item/location/capability/type state | `INVALID_INPUT` | no authoritative mutation |
| semantic item/equipment/container rule rejected | owning rejection category | no mutation |
| wrong WorldId/runtime scope | `CONFLICT` | no mutation |
| stale GameSession/CharacterLease/runtime owner | stale-authority `CONFLICT` subtype | no mutation |
| duplicate same semantic transaction/operation | idempotent reconciliation | no second mutation |
| same TransactionId/OperationId/source with conflicting intent | integrity `CONFLICT` | no overwrite/reinterpretation |
| proven serialization/deadlock abort | retryable internal/transient | bounded retry same logical identity |
| ambiguous commit | reconciliation required | no blind new TransactionId/candidate |
| runtime reservation pending ambiguity | pending/hold | no competing spend/mutation |
| participant/resource bound exceeded | `CAPACITY_EXCEEDED` | no partial mutation |
| unsupported transform/definition/ruleset revision | `UNSUPPORTED_REVISION` | fail closed |
| mandatory audit cannot commit | `DEPENDENCY_UNAVAILABLE` or owning internal | no mutation where audit mandatory |
| internal location/conservation/lineage violation | `INTERNAL_UNAVAILABLE` / integrity | stop affected path; preserve evidence |

Exact client-visible codes remain owning protocol/domain registry work.

## 44. Client presentation boundary

Client drag/drop slots, optimistic visuals or cached inventory/ground snapshots do not define transaction authority.

Server reconciles committed state through FND-02 domain revisions/snapshots/deltas. A stale-view rejection does not alter conservation.

## 45. Derived/materialized state

Derived inventory indexes, runtime ground projections, equipment views, weight summaries or client projections are either:

1. atomically consistent where they participate in correctness; or
2. explicitly rebuildable/non-authoritative from committed source/receipt state.

No projection/cache becomes a second item-location authority.

## 46. Definition revision compatibility

Every transaction interprets touched items under explicit compatible GAME-ITEM definition/ruleset/content revisions.

No silent reinterpretation on same ItemTypeKey. `MIGRATION_REQUIRED` state is not mutated under new meaning until migration/validation. Unsupported mixed revision fails closed. Historical transaction evidence retains enough revision context to interpret lineage.

## 47. Session/runtime recovery consequence

If durable transaction commits while transport/session/runtime completion fails:

- committed durable result remains authoritative;
- reconnect/recovery cannot replay it as new;
- retained command/operation/transaction receipt/evidence reconciles result;
- same-GameSession may terminate if FND command state cannot reconstruct safely, without rolling back committed value for convenience;
- replacement runtime owner resolves pending ground/custody reservation from durable outcome before reopening interaction.

## 48. Multichannel invariants

- Character-held durable value is not channel-owned solely because Character plays on one channel;
- channel/instance ground simulation remains runtime-scope fenced;
- stale channel owner cannot commit durable ground/value transaction after ownership generation changes;
- direct durable transfer between two independent live channel/instance ground authorities is unsupported without explicit one-winner handoff/custody coordinator;
- cross-channel relog/recovery cannot duplicate Character inventory because live durable item has one semantic location and current Character authority;
- world-shared item topology such as future houses requires separately typed world owner, not hidden channel-local duplication.

## 49. Security invariants

Mandatory:

- no client authority for identity/quantity/location/source/sink/balance;
- no arbitrary authoritative transaction/location JSON/EAV;
- no unbounded participant graph or audit payload;
- no cross-world laundering by burn+mint;
- no conflicting same cause/OperationId/TransactionId last-write-wins;
- no raw SQL/admin mutation as ordinary correction;
- no ItemInstanceId reuse/reassignment;
- no binding/location metadata as session authority;
- no stale GameNode/lease owner durable commit;
- no synchronous DB blocking inside runtime writer lane;
- no runtime checkpoint as durable success;
- no mandatory audit downgrade to best-effort;
- no audit replay as gameplay replay;
- no automatic analytics repair authority.

## 50. Required implementation evidence

Any future implementation claiming DUR-03 conformance proves on exact revisions at least:

### Identity/location

- fresh output ID/no cross-transaction reassignment;
- split/merge survivor/retirement;
- preserve/replace transform;
- one semantic location across inventory/equipment/container/ground/custody;
- container root move without descendant rewrite/orphan;
- cross-world rejection.

### Runtime↔durable handoff

- runtime reservation prevents competing pickup/drop while DB pending;
- ChannelRuntime writer never blocks synchronously on DB;
- current ownership-generation fence rejects stale persistence request/completion;
- drop durable commit is recoverable if runtime dies before materialization;
- pickup durable commit suppresses stale checkpoint ground ghost after recovery;
- ambiguous DB result keeps reserved runtime value non-spendable until classified;
- known abort releases/retries reservation safely;
- runtime-only loot materialization cause cannot mint twice.

### Idempotency/concurrency

- duplicate CommandRef no second execution;
- same TransactionId retry no second effect;
- planned output IDs stable through retry;
- conflicting TransactionId rejected;
- known abort rematerializes same logical intent safely;
- ambiguous commit freezes/reconciles exact candidate;
- lost response returns original result;
- stale CharacterLease/runtime generation rejected;
- same-GameSession reconnect does not duplicate pending command effect.

### Conservation

- pure move preserves item/value;
- split/merge exact quantity;
- mint/burn require cause;
- duplicate source occurrence rejected/reconciled;
- transform complete lineage;
- non-item ledger exact debit/credit;
- conversion exact rule;
- compensation new causally linked transaction.

### Atomicity/failure

- crash before commit => no durable mutation/mandatory audit;
- crash after commit before response/runtime completion => committed state/evidence recoverable;
- publication crash => EventId-stable at-least-once, no gameplay replay;
- equipment multi-slot all/none;
- custody transfer removes old spendability;
- participant/evidence overflow rejects without partial state.

### Restore

- duplicate location/receipt/cause conflicts detected;
- pre-restore authority fenced;
- runtime checkpoint ghosts reconciled against durable receipts;
- audit replay cannot remint;
- integrity failure keeps mutation closed.

### Evidence

- concrete registered ANL durable-audit schemas/types for mandatory classes;
- TransactionEventRef complete-set/gap/duplicate tests;
- bounded aggregated evidence event/payload count;
- privacy/retention profiles;
- no ItemInstanceId high-cardinality metrics labels.

Architecture acceptance alone proves none of these runtime outcomes.

## 51. Decision timing

### Must decide now?

**YES.**

### Downstream work blocked

- durable inventory/equipment/container/ground implementation;
- runtime↔durable loot/pickup/drop handoff;
- item/currency anti-duplication implementation;
- durable loot/pickup persistence slice;
- safe typed custody for later trade/market/depot/mail/reward/house flows;
- Game Intelligence item/value reconciliation;
- item/value concurrency/crash/recovery E2E.

### Future migration cost if changed late

Late change can require migration of semantic location authority, runtime/durable handoff, ItemInstanceId lineage, receipts/idempotency, source/sink provenance, audit interpretation, custody and restore validation.

### Supersession evidence

Reopen only with named evidence such as:

- proven externally observable Reference mechanic incompatible with typed model;
- PostgreSQL/runtime concurrency anomaly showing accepted handoff/atomicity cannot close a required operation;
- measured scale evidence requiring a different bounded partition/custody design;
- exploit/security evidence;
- future accepted cross-service/database custody protocol with equivalent one-authority/conservation guarantees;
- privacy/legal retention constraints;
- explicit later owner world-transfer/economy policy.

OTS schema layout, framework/library preference or convenience is insufficient.

### Deliberately not decided

- physical SQL schema/index/constraint/lock syntax;
- concrete Rust transaction/runtime APIs/crates;
- numeric transaction/resource ceilings without evidence;
- concrete ANL event IDs/protobuf payloads;
- exact unevidenced Reference source/sink/transform/crafting/decay/business rules;
- exact loot materialization timing under combat/content owner;
- downstream business state machines;
- cross-world transfer feature;
- cross-database/service atomic transfer protocol;
- production RPO/RTO/topology/backup cadence;
- automatic remediation.

## 52. Acceptance consequence

Only after:

1. candidate delivery passes exact-head implementing-agent self-review;
2. required genuinely independent review has zero open material findings;
3. exact-head governance/document CI passes;
4. review threads/ownership conflicts are clean;
5. PR #207 is squash-merged unchanged; and
6. separate lifecycle closeout atomically promotes maintained programme status/handoff,

may programme state become:

```text
DUR-03
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL/migration authority = NONE
```

Architecture acceptance does **not** authorize item/value runtime implementation or production mutation. A later implementation task requires separate owner authority and section 50 evidence.
