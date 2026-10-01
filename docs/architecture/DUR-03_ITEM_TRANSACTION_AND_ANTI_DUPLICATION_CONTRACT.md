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

**Amendment (HOUSE-CUSTODY-0, 2026-09-30).** The first such family is admitted:

```text
HouseInterior {
  house_id: HouseId {world_id, house_key}   (revision-free)
  spatial_position: native WorldTilePosition (a tile of exactly this house in the active bundle)
  stack_ordinal: typed ordinal, unique per (world, house, position)
}
```

- It is World-scoped, with no `ChannelId`. Its owner is the Game housing domain (EXP-HOUSES-01).
- Only items without contents may enter it until a later decision admits containers.
- Every placement writes `HousingReclaimProvenance` in the same transaction, at most one live
  provenance per item.
- One deferred item-level guard across every location table keeps a live item in exactly one
  location.
- Its writers are fenced by one live generation per `HouseId`, delivered with the house interior
  runtime. Player writers stay closed until the house has an owner and an ACL storage grant, and
  until then the runtime role has no grant on the house tables.

See `reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md`.

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

**Amendment (DEPOT-0, 2026-09-30), pending on acceptance of DEPOT-0.**
`reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md`, once accepted, admits the
second custody family:

```text
CharacterDepot { character_id: CharacterId, box: 1..17, ordinal: NUMERIC(20) }
```

Character + World scope, no channel and no runtime scope; the 17 boxes are fixed compartments, not
items. Its DEPOT-1 child admits two one-item TRANSFER shapes, superseding the §39.1 and §39.3
source and destination limits for them only: a main backpack direct entry into a box, and a depot entry
out to the main backpack's container slot or a new direct entry (no merge). Corpse and Ground
sources are refused. The table joins the
HOUSE-CUSTODY-0 item-level exclusivity guard.

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

**Amendment (pending on acceptance of BAGS-0;
`reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` §3, §4).**
Containers with contents are admitted. Every entry is `Container {parent ItemInstanceId, ordinal}`
and stores no owner; the owner is the root's location. A tree has depth at most 8 and at most 500
items. Moving a container is one TRANSFER of the root; the descendants are locked `FOR SHARE` and
checked, never moved, and the receipt binds their count and a hash. For these shapes only, it
supersedes the §39.1 exclusion of nested containers and sets container expansion to 8 levels
(`DUR03-RL-05-TREE`). §39.3 step 3 keeps its one-item `DECAY_RETIRE` shape for a Ground tree:
one transaction per item, in post-order (every descendant before its parent, the root last), which
extends the D3 order to depth 8. Every other obligation is unchanged.

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

Besides the D3 `DECAY_RETIRE` cause, the admitted burn sink is the closed `FeeBurnCause` of the
gold fee amendment in §39.3 (owner decisions D174-D178), and, with the NPC service amendment in
§39.3, the closed `NpcTradeCause` of a SELL. Pending on acceptance of ITEM-USE-0, the item use
amendment in §39.3 admits the closed `ItemUseCause`.

**Amendment (pending on acceptance of QUEST-GATE-0; `reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md` §5.4).**
The quest exchange amendment in §39.3 admits the closed `QuestExchangeCause`.

**Amendment (pending on acceptance of RUNE-USE-0; `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`
§15).** The rune and conjure amendment in §39.3 adds the variant `Rune` to `ItemUseCause` and
admits the closed `ConjureCause` (a BURN of one reagent unit with a MINT of the conjured units).
Rune use is a caller-chosen one-unit BURN, not `DECAY_RETIRE`.

**Amendment (pending on acceptance of RANGED-0;
`reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §6).**
The weapon use amendment in §39.3 admits the closed `WeaponUseCause {Ammunition, Throwing}`.

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

**Amendment (BANK-0, 2026-09-30), pending on acceptance of BANK-0.**
`reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md` §5, once accepted, names
the first non-item asset: bank gold, an
integer balance per (Account, World) with an immutable ledger, converted to and from coins by the
gold fee worth table. Its BANK-1 child admits, for these shapes only: a deposit (coin inputs and up
to two change outputs as `CONVERSION` lines, and one credit `CONVERSION` value line), a withdrawal
(one debit `CONVERSION` value line and up to three coin `CONVERSION` outputs) and a transfer (two
`TRANSFER` value lines between two accounts, no item), under the closed causes
`BankConversionCause` and `BankTransferCause`. They supersede, for these shapes only, the §39.1
exclusions of non-item accounts, multiple touched items, burn and MINT combined with other lines;
they contain no BURN or MINT, so §15 is unchanged. Each is one transaction with one TransactionId,
replayed by its operation occurrence, with one bank event carrying its item and value lines.
`DUR03-RL-03` stays 0 for every existing shape.

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

**Amendment (ITEM-MOVE-WIRE-1, 2026-09-30), pending on acceptance of ITEM-MOVE-WIRE-1.**
`reviews/OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md` §6, once accepted, admits, each in its
child and for those shapes only, superseding the §39.1 and §39.3 limits named there: TRANSFER of a whole item
between the main backpack and the nine non-container `CharacterEquipment` slots, including a swap
with the target slot's occupant (two items, four location lines, with its own resource rows); and
TRANSFER of a whole item from a backpack entry or a slot to `Ground`, and from Ground back to the
main backpack, under §32. No new burn sink: dropped items are retired by `WorldReset` (D191).

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

**Amendment (PLAYER-TRADE-0, 2026-09-30), pending on acceptance of PLAYER-TRADE-0.**
`reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md` §5-§6, once accepted, admits one trade swap: two whole items
of two Characters on one channel, each into a new direct entry of the other's main backpack, in
one transaction keyed by the trade occurrence, with both Characters fenced. It supersedes the §39.1
one-item and one-Character limits for that cause only. Offered items stay in place until the swap,
so no custody family is used.

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

**Market amendment (pending on acceptance of MARKET-0, #162 5912405163;
`reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md`).** When MARKET-0 is accepted,
the market row is filled by two §5.2 custody families, `MarketOfferEscrow {offer_id, ordinal}`
(World-scoped, owned by the Market) and `CharacterInbox {character_id, ordinal}` (Character + World),
and by a buy offer's `escrow_gold` as §18 non-item value in custody. Its shapes (place, accept,
cancel, expire, match, Inbox out) move whole items as `TRANSFER` lines with at most one §12 split
(`SPLIT_MERGE_QUANTITY`, a §11.3 planned output identity), gold as `TRANSFER` value lines, and the
placing fee as one `BURN` value line under the Market variant of `FeeBurnCause`. The owner
admitted that fee source, paid from the bank (D178; #162 5913348961); MARKET-0 carries it into
§39.3 and the gold fee decision §4.4 as pending amendments. A held credit (MARKET-0 §6) is §18
non-item value in custody on the offer, like `escrow_gold`. For those
shapes only, it supersedes the §39.1 exclusions of non-item accounts, multiple touched items and
burn combined with other lines, and the §39.1 and §39.3 source and destination limits, within the
MARKET-0 §9 rows (100 touched items, 3 value lines). Every other obligation is unchanged.

**Amendment (pending on acceptance of MAIL-0; `reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md` §6).**
The mail row is filled with no new custody family: a posting moves a letter or a parcel root
from a main backpack entry into the recipient's `CharacterInbox` (`TRANSFER`) and stamps the same
item (`TRANSFORM`, `PRESERVE_INSTANCE`, §16.2) in one transaction, under the closed `MailCause`.
A parcel's children keep their `Container` location (§10). A child of an Inbox parcel leaves as a
one-item `TRANSFER`; a system letter, if the owner admits it, is one `MINT` into the Inbox. No
value lines. For those shapes only, it supersedes the §39.1 exclusions of transform combined with
transfer and of multiple touched items, and the §39.1 and §39.3 source and destination limits,
within the MAIL-0 §11 rows (11 touched items). Every other obligation is unchanged.

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
  association/provenance. (The reward-chest and D3 amendments in §39.3 each admit
  one named shape whose only location is a `Container` entry instead — no Ground
  custody, no separate TRANSFER for that placement. The starter grant amendment, pending on
  acceptance of STARTER-BACKPACK-0, admits one whose only location is the empty
  `CharacterEquipment` container slot.)
- `TRANSFER`: that already-existing live ItemInstance moves from typed `Ground`
  custody to direct-root `CharacterInventory`, preserving identity, type and
  quantity and leaving exactly one authoritative immediate location. (The D3
  amendment in §39.3 additionally admits a `Container { parent = a live corpse
  ItemInstance }` source, alongside Ground, for that one named shape.)
- `DECAY_RETIRE` (D3 amendment, §39.3 below): one already-existing live
  ItemInstance moves from its live location (typed `Ground`, for a corpse
  ItemInstance, or a `Container` entry, for a loot ItemInstance) to `RETIRED`
  with no location, under a named, non-caller `CorpseDecay` cause. The
  ADR-0021 amendment in §39.3 admits a second named cause, `WorldReset`, for
  Ground roots and their container entries. This is not
  `burn`: it exists only for the named causes these amendments give, admits no
  caller-chosen retire cause or reason code, and
  every other retire path (a TRANSFER full-merge source retiring per §11.4/
  §11.5, DUR-03's ordinary stack-to-zero retirement) is unamended by it.

MINT, the later TRANSFER, and (where admitted) `DECAY_RETIRE` are separate
transactions, with separate TransactionIds, event candidates and atomic
boundaries. Aggregation does not
combine their sequence into one commit. This child does not support mint into an
existing stack, multiple touched items, quantity redistribution, burn (outside
the named `DECAY_RETIRE` causes above), transform,
non-item accounts, nested containers or additional custody families. (The B3
amendment in §39.3 admits the two-item merge and top-up shapes and direct entries of
the equipped main backpack; the D3 amendment in §39.3 admits `DECAY_RETIRE`; the gold fee
amendment in §39.3 admits typed BURN of up to 20 coin stacks with up to 2 change MINTs, composed
with a Character change in one transaction; the NPC service amendment in §39.3 admits the NPC BUY,
SELL and travel shapes; pending on acceptance of ITEM-USE-0, the item use amendment in §39.3 admits
a one-unit BURN, or a one-unit TRANSFORM into a flask stack or a fresh flask, under
`ItemUseCause`.) Unsupported
shapes reject instead of acquiring meaning through a generic delta, metadata bag
or unbounded repeated effects. The quantity-one private fixture is not an accepted
Content definition or a production quantity ceiling.

**Amendment (pending on acceptance of RUNE-USE-0; `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`
§39.1).** The rune and conjure amendment in §39.3 also admits a one-unit BURN under
`ItemUseCause::Rune`, and one reagent-unit BURN with one MINT of the conjured units into an
existing stack or a fresh entry under `ConjureCause`.

**Amendment (pending on acceptance of RANGED-0;
`reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §6).**
The weapon use amendment in §39.3 also admits a one-unit BURN, or a one-unit split or whole-item
TRANSFER to Ground, under `WeaponUseCause`.

**Amendment (pending on acceptance of WORLD-INTERACTION-0;
`reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md`
§4.4 and §7.2).** A one-item TRANSFER from typed `Ground` custody on one tile to `Ground` on another
tile of the same channel scope (§32 fence), with the destination tile limit and both tile rows
locked after the item rows in tile key order; a container tree moves by its root (BAGS-0 §4.3).
Rows `DUR03-RL-0x-GROUND-MOVE`: 1 touched item, 2 location lines, 1 participant, 3 work units. A
key's immutable `key_number` is part of the MINT and TRANSFER typed evidence (`DUR03-RL-07-KEY`).

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

> **Amendment (2026-09-29).** The corpse container rules are those of the D3 decision (`reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`, the "Corpse container amendment (D3)" paragraph in this section). `reviews/OTERYN_GAME_A10_CORPSE_CONTAINER_DUR03_AMENDMENT_DECISION_2026-09-29.md` is superseded where it differs.

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

**B3 amendment.** Owner decisions D80-D83 in
`reviews/OTERYN_GAME_B3_INVENTORY_DESTINATION_CAPACITY_AND_STACKS_DECISION_2026-09-28.md`
(§4.6) admit, besides this destination, the character's `CharacterEquipment` container slot
and direct entries of the equipped main backpack (§5.2 `Container`), and the two-item merge
and top-up shapes of §13. For those shapes they supersede the §39.1 exclusions of multiple
touched items, quantity redistribution and nested containers, and resolve the "NO" of the
decision test below for destination, capacity and placement. Every other obligation of this
section is unchanged.

**Reward chest amendment.** `OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md` §5.1 (D40, D92)
admits, in the `CHEST-1` child after B3-1, one bounded MINT shape and for it supersedes these
statements of §39.1-§39.3: that every MINT descends from a committed creature-death output, and
that a MINT establishes typed Ground custody before a separate TRANSFER. For this shape:
- the source cause is the D40 GAME-INTERACTION child occurrence of a `USE` on a reward-claim
  placement, keyed by `(claim, character)` (and the cycle ordinal for a cooldown claim);
- the first and only location is a new entry of the equipped main backpack under the B3 placement
  rule; no Ground custody and no TRANSFER;
- the audit evidence is: before, explicit nonexistence of the item and the claim state; after, the
  live item in its `Container` entry, its type and quantity, and the committed `RewardClaim` row;
- mint into an existing stack stays excluded (§39.1).
Every other §39 obligation is unchanged.

**Starter grant amendment (STARTER-BACKPACK-0), pending on acceptance of STARTER-BACKPACK-0.**
`reviews/OTERYN_GAME_STARTER_BACKPACK0_STARTER_GRANT_DECISION_2026-09-30.md` §5-§6 admits, in the
`STARTER-1` child, one bounded MINT shape and for it supersedes the same §39.1-§39.3 statements as
the reward chest amendment above. For this shape:
- the source cause is `StarterGrant {character_id, template_key}`, server-originated with no
  CommandRef, whose occurrence identity is the `game_character_starter_grants` key: once per
  Character and template, forever; the template is the one named by the root's
  `starter_template_revision`;
- the first and only location is the Character's empty `CharacterEquipment` container slot (D80);
  no Ground custody and no TRANSFER; an occupied slot mints nothing;
- the audit evidence is a `OneItemTransactionV1` `starter_grant` operation: before, the nonexistence
  of the grant row and of the slot item; after, the live item in the slot, its type and quantity 1,
  and the committed grant row, all from one physical transaction;
- mint into an existing stack stays excluded (§39.1).
Every other §39 obligation is unchanged.

**Corpse container amendment (D3).** `D3-CORPSE-CONTAINER-LOOT-WINDOW-DECAY-V1` (owner decisions
D111-D113, `reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md` §4)
admits, for creature-death loot only, the following besides the destinations already listed
above; every other §39 obligation (fences, cause, evidence, idempotency, current authority,
conservation) is unchanged.

- **Corpse MINT is unamended.** A creature's corpse is one fresh, ordinary MINT under the
  existing §39.1/§39.2 shape: typed Ground custody, its own cause (the death key with a reserved
  `CORPSE_MATERIALIZATION` purpose key and `draw_ordinal = 0` in the existing loot-cause tuple
  shape, `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` §4.2). No new MINT destination is admitted for
  the corpse item itself.
- **Loot MINT into the corpse.** A loot entry's MINT establishes its first and only location as a
  fresh entry of `Container { parent_item_instance_id = <that same death's committed corpse
  `ItemInstance` >, entry }` — no Ground custody, never a separate TRANSFER for that placement.
  The parent must itself carry a live `CORPSE_MATERIALIZATION` receipt for the *same* death key;
  a loot MINT naming any other parent (including a character's own equipped container) is
  rejected. A loot entry's own MINT commits only after its death's corpse MINT has itself
  committed (retry-safe: an uncommitted corpse cause is retried first, exactly as any other MINT
  cause is). **Admitted audit aggregate (extends the reward-chest amendment's MINT-into-container
  shape above, the same way it extends §39.1's MINT bullet — no additional gap):** before, explicit
  semantic nonexistence of the item (same as every MINT); after, the live item in its `Container`
  entry (parent = the corpse), its type and quantity; cause, the same full loot-output cause
  §39.2/§4.2 (`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1`) already defines, unchanged. This needs no
  new cause shape (unlike `DECAY_RETIRE` below, which names one because none existed for a decay
  reason): it is the existing loot-MINT cause with its destination generalized from "an
  already-equipped backpack entry" (reward chest) to "this death's own corpse entry" (D3). The
  proto/registry change this needs — admitting a `Container` alternative beside
  `OneItemMintV1.destination` (`OneItemGroundV1`) — is not defined here; the D3-2 implementation
  child registers it as the additive `OneItemMintV1.corpse_container_entry` (field 5, exactly one of
  the two set), under the same non-candidate, no-`_fixture`-field-names conditions. `DECAY_RETIRE`'s
  own registration stays with D3-6.
- **Whole-plan preflight.** Before any entry of a death's accepted loot plan is frozen — corpse
  included — the composing caller checks the plan's full accepted entry count against
  `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (16, equal by construction to the already-accepted
  `COMBAT01-LOOT-PLAN-ITEMS`/`COMBAT01-ITEMS-PER-CORPSE` ceilings,
  `reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md` §4.1 row 6). The whole
  plan is admitted or the whole plan is refused up front — the same up-front-whole-plan posture
  `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE` already uses — never partially admitted mid-sequence.
  Because the ceilings are equal, an accepted plan can never itself exceed corpse capacity; the
  preflight exists to keep that invariant explicit and checked, not merely coincidental. The
  corpse's own entry-count check locks the corpse's row before counting (the same
  `FOR UPDATE`-before-count pattern `game_item_placement_proven()` already uses for the backpack,
  migration `0011_item_transfer_backpack.sql`), so concurrent loot MINTs for the same corpse
  serialize instead of racing the count. Each entry's MINT remains its own DUR-03 transaction
  (never combined into one commit, §39.1). **This is not cross-generation retry.** Per
  `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1` §4.2/§4.3 (D52): a loot entry's MINT retries only while
  the death's runtime-scope ownership generation is still the current assignment; if that
  generation ends (restart, crash, scope move) before every preflight-admitted entry has
  committed, the remaining entries are dropped terminally — no later generation retries them, and
  none can produce the same key. The corpse may therefore durably hold fewer live entries than its
  accepted plan's count; this is the same accepted D52 loss ("Przepadają, bez duplikatów"), never
  a duplicate and never silently completed by a later generation. The whole-plan preflight above
  prevents only a *capacity*-caused partial commit (an over-capacity plan is refused before any
  entry freezes, so it never partially lands); it does not and cannot prevent a generation-ending
  crash from leaving a already-admitted plan partially committed, which D52 already accepts.
- **`materialized_at`: the latest pre-commit anchor reachable, not the exact commit/visibility
  time.** The existing `occurred_at` column is written once, at `freeze_item_mint`
  (PREPARE/reservation) time, and reused unchanged through `commit_item_mint`
  (`item_mint.rs:410-447` reserves it; `item_mint.rs:620-628` reuses the frozen value without
  re-reading the clock) — a delayed commit would silently shrink both the D3 pickup window and
  decay if either were derived from it. `statement_timestamp()` is not a fix either: it is fixed
  per SQL statement, not per transaction, so an `INSERT`'s own `statement_timestamp()` can still
  precede the transaction's actual commit (and so its durable visibility to other transactions) by
  however long the transaction stays open afterward. The corpse's own `game_item_mint_receipts`
  row instead gets a new nullable column, `materialized_at BIGINT`, set from `NULL` to
  `clock_timestamp()` — which re-evaluates on every call, unlike `statement_timestamp()` — by a
  new `AFTER INSERT ... DEFERRABLE INITIALLY DEFERRED` constraint trigger (the same mechanism
  `game_item_mint_consistency_guard` already uses), scoped to a receipt whose
  `loot_purpose_key = 'CORPSE_MATERIALIZATION'`. A deferred constraint trigger fires immediately
  before the transaction's own commit finalizes — the latest point reachable from inside the
  transaction, regardless of how many statements the application issues before it, so this anchor
  does not depend on `insert_mint` being the transaction's last statement. The receipt-immutability
  guard (`game_item_mint_receipt_immutable`) gains one narrow, one-way exception for this — the
  same idiom the audit-outbox `publication_state` mark already uses: `materialized_at` may move
  from `NULL` to a value once, for a `CORPSE_MATERIALIZATION` receipt only, with every other column
  unchanged; only the trigger's own `SECURITY DEFINER` function performs this update (matching the
  `game_item_ground_removal_evidence_capture` idiom), so the runtime role needs no direct `UPDATE`
  grant on the column. This is still not the exact instant of durable visibility, only the closest
  anchor reachable from inside the transaction; the residual gap is bounded by however much of the
  transaction's configured `transaction_timeout`/`statement_timeout`/`lock_timeout` remains at that
  point — set, for every semantic transaction, to the remaining time of the already-registered
  `DFR-DB-PASS-MS` budget (2,000 ms, `durability/db.rs::begin_semantic_transaction`, lines
  1088-1096). **The D112 window and D113 decay may therefore be early by up to that bound (at most
  2,000 ms in the worst case), never late; this is accepted, not a correctness defect** (a 10 s
  window or a 60 s decay firing up to ~2 s early is immaterial to either's purpose).
  `materialized_at` is `NOT NULL`, after commit, exactly when `loot_purpose_key =
  'CORPSE_MATERIALIZATION'`, and `NULL` for every other MINT shape. The D112 exclusivity window and
  D113 decay are both derived from this column, never from `occurred_at`.
- **The corpse `ItemInstance` is never a legal TRANSFER source**, regardless of whether it
  currently has live entries (closing the gap `plan_transfer`'s existing `ContainerNotEmpty` check
  leaves open once a corpse's entries are all picked out or its loot plan was empty): a new
  refusal (`ItemTransferRefusal::CorpseNotPickupable`) rejects any TRANSFER whose source item
  carries a live `CORPSE_MATERIALIZATION` receipt, enforced both in the Rust admission and by a
  new deferred constraint trigger symmetric to `game_item_ground_removal_proven`, so a corpse's
  Ground row can never be deleted by a TRANSFER by construction. A corpse leaves Ground only
  through `DECAY_RETIRE` below. Corpse content declares no `container`-slot equip pattern, so a
  corpse is independently refused (`NotContainerSlotEquippable`) as a `ContainerSlot` destination
  even if the source check above were ever bypassed.
- **Pickup source.** TRANSFER additionally admits `Container { parent = a live corpse
  ItemInstance }` as a source (alongside the existing Ground source), gated by the D112/D133
  exclusivity window: before the window's deadline, only the death's captured top-damage
  `CharacterId` may transfer; at or after it, any character may. This reuses every other D80-D83
  destination, capacity, stack and merge rule unchanged and needs no new `DUR03-RL-*` row.
  **Admitted audit aggregate:** identical in kind to the existing TRANSFER shape (before: the
  item's live state and its exact source location — now a `Container` entry rather than Ground;
  after: the item's live state at its legal destination, as D80-D83 already define; cause: the
  same `typed_cause "ground_pickup_transfer"`-style player `CommandRef` provenance TRANSFER
  already carries, unchanged; receiver: the existing D83 merge/top-up shape, unchanged) — only the
  *source location's family* is new, not the aggregate's shape. The proto/registry change this
  needs — widening `OneItemTransferV1.source` from `OneItemGroundV1` only to admit a `Container`
  alternative — is not defined or registered here; it lands in the D3-6 implementation child
  alongside the corpse-loot MINT's and `DECAY_RETIRE`'s own registration, under the same
  non-candidate, no-`_fixture`-field-names conditions.
  **Allocation note (D3-4):** the control plane allocated this TRANSFER-source widening to D3-4, not
  D3-6. It landed additively as `OneItemTransferV1.corpse_source` (field 7,
  `OneItemCorpseSourceV1 { corpse_item_instance_id, placement_ordinal, corpse_ground }`), exactly one
  of `source`/`corpse_source` present, no existing field changed; the MINT `destination` widening and
  `DECAY_RETIRE` stay with D3-6.
- **`COMBAT01-CORPSES-PER-SCOPE` (already accepted at 64,
  `reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md` §4.1 row 6, "reject the
  projection; the death still commits and loot follows D52") is the one bound on concurrent
  corpses per scope; no competing or derived value is introduced here.** **The authoritative check
  is at the corpse's own MINT commit, under a per-scope lock, never a freeze-time-only count.** A
  count read at `freeze_item_mint` time alone cannot serialize against another death's concurrent
  freeze for the same scope: several deaths can each observe 63 live corpses, each pass that
  freeze-time check, and all commit, overshooting 64. The corpse's `commit_item_mint` pass instead
  takes a per-scope transaction advisory lock,
  `pg_advisory_xact_lock(hashtextextended('oteryn:corpse-cap:' || <scope_key hex>, 0))`, *before*
  `fence_is_live` and before its own insert (not `FOR UPDATE` on `game_runtime_scope_assignments`:
  `fence_is_live` already holds `FOR SHARE` on that row, so two corpse commits upgrading to `FOR
  UPDATE` would deadlock, and an exclusive row lock would also serialize every other MINT,
  TRANSFER and XP writer of the scope). Only corpse MINT commits take this key, so it serializes
  exactly the corpse-cap recount.
  It then recounts live corpses for that scope (live
  `game_item_ground_locations` rows joined to a `CORPSE_MATERIALIZATION` receipt, scoped to the
  same `world_id`/`channel_id`); if the count is already ≥ 64, the commit pass refuses
  (`CapacityExceeded`) and inserts nothing, all inside the same transaction as the corpse's own
  insert — so no two concurrent corpse commits for one scope can both observe room and both
  succeed. Any earlier `freeze_item_mint`-time count is **advisory only**: a cheap early rejection
  for the obvious case, never the authority; only the locked commit-time recount admits or refuses.
  On refusal, the new corpse's MINT (and therefore its whole loot plan, which has no destination
  without it under D111) is rejected; the creature's death itself still
  commits, and its loot is lost exactly as D52 already accepts loot loss (never duplicated). No
  corpse is retired early to make room, so no already-committed loot already inside an existing
  corpse is ever touched by another death's overflow. **D3-1 must prove a concurrency test**: N
  concurrent corpse-MINT commits for one scope already holding 63 live corpses produce exactly one
  success and N-1 `CapacityExceeded` refusals, never more than 64 live corpses and never a lost
  update.
- **Recovery is Ground-only and terminal-state-aware.** A corpse's own location is always Ground,
  never a `Container` entry of anything; the scope (re)admission query that reconstructs pending
  D113 decay timers (VSL-COMBAT-01 §17 above) reads only live `game_item_ground_locations` rows
  joined to a `CORPSE_MATERIALIZATION` receipt with `lifecycle = 1` (never a retired corpse, and
  never a `Container` row), so a corpse already retired by `DECAY_RETIRE` elsewhere is never
  rescheduled.
- **`DECAY_RETIRE` is N+1 separate one-item transactions, never one N-item transaction.** A full
  corpse (corpse item plus up to 16 live entries) is 17 `ItemInstance`s — far past
  `DUR03-RL-01`/`DUR03-RL-06`'s existing 1-2 touched-item/participant ceiling (§3.1). Rather than
  registering a new, corpse-sized resource row, decay is restructured to fit the existing default
  shape exactly: one new minimal DUR-03 transaction type, applied **once per live entry currently
  parented to the corpse, then once more for the now-empty corpse itself** — each application
  touches exactly **1** `ItemInstance`, with **1** location line (its removal) and the existing
  default `DUR03-RL-06` (1 participant / 3 work units), so **no new `DUR03-RL-*` row is needed**.
  Each entry-step is keyed by its own cause (that entry's `ItemInstanceId` under the corpse's decay
  marker), idempotent and non-duplicable exactly like any other DUR-03 cause. The final
  corpse-retirement step is keyed by the corpse's own `CORPSE_MATERIALIZATION` cause and is
  admitted only when zero live `Container` entries remain parented to it (checked the same way the
  entry-count preflight above is), so a corpse can never retire while orphaning a still-live entry.
  Every step commits under VSL-COMBAT-01 §17's "accepted DUR-03/domain policy" clause (§9.1/§17
  above). This is the only path by which a corpse's Ground row is ever removed (the ADR-0021
  amendment below adds the `WorldReset` path), and `CorpseNotPickupable`
  above applies throughout — a corpse mid-drain is exactly as unpickupable as a fresh one.
  **Resumable from durable state, not from an in-memory decay-progress marker:** the scope
  (re)admission recovery query above finds any corpse still live on Ground past its
  `materialized_at + 60_000` deadline — whether decay never started or was interrupted after
  retiring some but not all entries — and simply (re)issues the remaining entry-retirement steps
  followed by the corpse step; each step's own idempotent cause makes a repeated or resumed attempt
  safe, and an entry already removed from the corpse by a legitimate D133-gated pickup before decay
  reached it is not re-targeted (decay only ever retires entries it finds still live and still
  parented to the corpse at the moment each step runs).
- **`DECAY_RETIRE`'s admitted audit aggregate (closes the §39.1 gap: that section closed the
  aggregate to MINT and TRANSFER and excluded burn).** Each `DECAY_RETIRE` step (a corpse's own
  step or one of its entries', §5.2) is a third closed one-item aggregate, additive to §39.1,
  covering exactly the same complete applicable §39 semantic evidence as MINT/TRANSFER:
  - **Before:** the item's exact live state — identity, type, quantity — and its exact live
    location/custody: typed `Ground` (the corpse's own step) or the `Container { parent =
    <corpse>, entry }` it occupied (an entry's step). Not absence, not an already-retired state.
  - **After:** `RETIRED`, quantity 0, no location — the same terminal shape §11.4/§11.5 already
    define for a stack reduced to zero, now reached as this aggregate's own explicit after-state
    rather than folded into a TRANSFER receipt.
  - **Cause:** a new closed cause shape, `CorpseDecay { corpse_item, deadline }` — `corpse_item` is
    the corpse `ItemInstanceId` this retirement belongs to (the corpse's own id, for its own step;
    the parent corpse's id, for an entry's step) and `deadline` is the exact `materialized_at +
    60_000` value that authorized the retirement, so the evidence itself proves the retirement was
    not early. No caller-chosen retire reason, burn cause or free-form label is admitted; this is
    the one named decay cause only.
  - **Event:** one ANL-01 `OneItemTransactionV1` operation (the same envelope MINT/TRANSFER already
    use) with a new `oneof operation` member alongside the existing `mint`/`transfer` tags (the next
    unused tag number in sequence), carrying this before/after/cause payload. One event per logical
    `DECAY_RETIRE` step, exactly as MINT and TRANSFER are one event per logical transaction.
  - **Evidence obligations:** identical in kind to §39.1's closing paragraph — typed item
    identity/lifecycle/type/quantity before and after; location/custody before and after;
    authorized cause and conservation summary (before-quantity retires to exactly 0, never a
    partial reduction); WorldId and applicable concrete runtime scope; compatible interpretation
    and definition revisions; safe applicable fence references without secrets. Resource maxima are
    the existing defaults this document already states (§3.1: `DUR03-RL-01` = 1, `DUR03-RL-02` = 2,
    `DUR03-RL-06` = 1 participant / 3 work units); no new `DUR03-RL-*` row.
  - **Selects no schema, no field numbers and no production authority.** The exact protobuf message
    (`docs/contracts/game-events/v1/native_one_item_transaction.proto`) and its
    `GAME_EVENT_FOUNDATION_REGISTRY.json` event-type/profile registration are not defined or
    registered here; they land in the D3-6 implementation child (§6 of the decision), under its own
    fresh allocation and independent review, following exactly the same non-candidate,
    no-`_fixture`-field-names registration conditions §39.2 already states for the native MINT
    binding. Every other retire cause (burn, or any cause outside this one named `CorpseDecay`
    shape) stays excluded.

Every other §39 obligation is unchanged. Pointer notes are added to §39.1, §39.2 and §5.2.

**Gold fee amendment (D174-D178).** `CHARACTER-GOLD-FEE-BOUNDARY-V1` (owner decisions D174-D178,
`reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md` §4-§5) admits one fee
shape and, for it only, supersedes the §39.1 and D3 exclusions of burn, multiple touched items and
a MINT committed with other lines, and the DUR-02 schema packet §7.5 hold on a path changing both
Character and items. Every other §39 obligation (fences, cause, evidence, idempotency, current
authority, conservation) is unchanged.

- **Source (D174, stage 1).** Live coin stacks in direct entries of the character's equipped main
  backpack (B3). No Ground, nested bag, depot or bank source; a bank ledger needs its own economy
  contract, and `DUR03-RL-03` stays 0.
- **Coins (D175, D176).** Exactly `oteryn:item.tibia.i3031` (gold, worth 1), `i3035` (platinum,
  100) and `i3043` (crystal, 10,000), each with stack maximum 100. The source supplies the fee in
  gold units. The deterministic plan (decision §4.2) burns inputs by worth ascending, then display
  order; at most the last input is partly burned. Change `C` is minted back as at most 2 fresh
  stacks in new backpack entries, `floor(C / 100)` platinum and `C mod 100` gold (D175; the
  control-plane interpretation in decision §2 reads Q39's "+1 change output" as a wording error).
  Fewer free entries than change outputs after the burn, insufficient funds, or more than 20
  inputs rejects the whole transaction and writes nothing. Conservation: burned worth minus change
  equals the fee.
- **Composition (D177).** One transaction commits the fee source's Character change and receipt
  (`CharacterRevision` +1 exactly once), every BURN line, the change MINTs and the audit event, or
  none. One TransactionId; one receipt, the source's Character receipt keyed by its occurrence,
  which binds the cause, the fee and the change. The fence and lock order are the Character
  writer's (`CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1` §3 rules 2-4, with the expected
  `CharacterRevision`), then the backpack and its coin entries.
  An item-only fee source (NPC BUY and NPC travel, NPC service amendment below) has no Character
  change: its one record is its DUR-03 cause record, under the item writer's fence with the
  `character_root` lock and no expected revision (gold fee decision §4.3 as amended).
- **Cause (D178).** Closed `FeeBurnCause`. Variants: `CharmUnassign { charm, occurrence }`, and,
  with the NPC service amendment below, `NpcTrade(NpcTradeCause)` and `NpcTravel { npc, route,
  occurrence }`.
  No generic fee cause or reason code. A new fee source needs an amendment of this paragraph and
  the decision.
- **Evidence and rows.** One event: each BURN line (quantity before and after; a whole burn ends
  `RETIRED` with no location), each change MINT (absent before), cause, fee, conservation summary,
  WorldId, scope, Character revision and fence references. Fee-shape rows: `DUR03-RL-01` 22,
  `DUR03-RL-02` 22, `DUR03-RL-06` 22 participants / 64 work units, `DUR03-RL-07-EVENTS` 1, payload
  and envelope measured within the ANL ceilings, other rows unchanged. The rows, schema and field
  numbers are registered by GOLD-FEE-1, not here.


**Market fee amendment (MARKET-0), pending on acceptance of MARKET-0 (#1367).** The owner admitted the Market
placing fee as a sink paid from the bank (D238, #162 5913348961), as D178 requires. The
`FeeBurnCause` variant `MarketFee {offer_id, occurrence}` is one `FEE_DEBIT` value line of class
BURN on the placer's (Account, World) balance, with no item line
(`reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md` §4).

**Bank fee amendment (BANK-FEE-0), pending on acceptance of BANK-FEE-0.**
`reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md`, once accepted, admits a bank part for the
fee shapes above, for those shapes only: it supersedes the §39.1 exclusion of non-item accounts; the
conservation becomes `burned - change + bank_debit = F`; a non-junior payer whose coins are worth
less than `F` burns every eligible coin whole with no change and pays `F - T` from its bank balance
(BANK-0) instead of being rejected; the bank part is one `FEE_DEBIT` value line of class BURN
under the fee's own `FeeBurnCause` (§15, §18), counted by `DUR03-RL-03-FEE`; `DUR03-RL-03` stays 0
for coin-only fees; a fee paid wholly from the bank has no burn line.

**House amendment (HOUSE-OWN-0), pending on acceptance of HOUSE-OWN-0 (#1368).** The owner
admitted the house auction price and rent as sinks paid from the bank (D238, #162 5913348961), as
D178 requires. `reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md` §4-§9 adds
the `FeeBurnCause` variants `HousePrice {house, auction}` and `HouseRent {house, period}`, each one
`BURN` value line on the owner's (Account, World) bank balance with no item line; bid escrow is
§18 non-item value in custody, moved by `TRANSFER` value lines; house disposition moves items
from `HouseInterior` to `CharacterInbox` as one-item `TRANSFER` lines, up to 100 items from one
source to many destinations per step. For those shapes only, it supersedes the §39.1 exclusions of
non-item accounts, multiple touched items and burn combined with other lines, and the §39.1 and
§39.3 source and destination limits, within the HOUSE-OWN-0 §12 rows (100 touched items per step;
`DUR03-RL-03-HOUSE`: at most 200 value lines, each ledger entry and each escrow change counting as
one). Every other obligation is unchanged.

**NPC service amendment (NPC-0).** `NPC0-NPC-RUNTIME-SERVICE-V1`
(`reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md` §5-§6) admits three shapes
built on the gold fee amendment above. The owner admitted NPC buying, selling and travel as value
sources on 2026-09-30 (D208; decision §9, Q1a; #162 5909366267), as D178 requires. For these shapes
only, it supersedes:
- the §39.3 statements that every MINT descends from a committed creature-death output and that a
  MINT establishes typed Ground custody before a separate TRANSFER (the bought item and the SELL
  coins are minted straight into new backpack entries);
- the §39.1 exclusion of burn and of multiple touched items (the pointer list in §39.1 names this
  amendment);
- the gold fee decision's limit of change outputs to platinum and gold (a SELL mints crystal
  coins too) and its §4.5 sentence that burn stays excluded outside `CorpseDecay` and the
  `FeeBurnCause` variants (a SELL item burn is sunk by `NpcTradeCause`).

Every other §39 obligation (fences, evidence, idempotency, current authority, conservation) is
unchanged.

- **BUY (item-only).** The gold fee plan with `F = unit price x quantity`, plus one MINT of the
  bought item (one stack, or one non-stackable item) into a new direct entry of the main backpack.
- **SELL (item-only).** One BURN from one live direct backpack entry of the offer's item (no
  contents, default state apart from quantity): `quantity x count` units of a stackable item, or,
  with `quantity` 1, one whole non-stackable item, whose charges or sub-type equal the offer's
  `count` for a charged or fluid item; plus a MINT of
  `unit price x quantity` gold as at most 3 fresh coin stacks (crystal, platinum, gold; each at most
  100) in new backpack entries, counted after the burn.
- **Cause.** One closed `NpcTradeCause {npc, offer, side, occurrence}` covers every line of a BUY
  or SELL: the BUY coin burn as `FeeBurnCause::NpcTrade` (side always BUY), the SELL item burn as
  its sink, and every MINT as its source. The occurrence is issued by the runtime, bound 1:1 to the
  command's CommandRef. One audit event carries it.
- **Records and revision.** BUY, SELL and travel fall under the composition decision §3 rule 1
  and §3.1: no `CharacterRevision` advance. Each writes one DUR-03 cause record keyed by
  (occurrence, character) under the item writer's fence (rules 2-5, with the `character_root`
  lock); for BUY and travel this record, not a Character receipt, is the fee source record (gold
  fee decision §4.3 as amended).
- **Travel.** `F = route price` burned under `FeeBurnCause::NpcTravel {npc, route, occurrence}` (a
  price of 0 writes no fee lines), plus one pending arrival row, an obligation outside the
  revision chain like DEATH-0's pending respawn.
- **Common.** The price is read from the trade or travel service at the bound content revision; a
  mismatch with the client's expected price rejects. Insufficient funds, no free entry, a BUY stack
  above the definition's `max_stack`, or a SELL coin stack above 100 rejects the whole transaction
  and writes nothing.
- **Rows.** BUY `DUR03-RL-01` 23, `DUR03-RL-02` 23, `DUR03-RL-06` 23 participants / 66 work
  units; SELL 4, 4, 4 participants / 9 work units. The rows, schema, the `0023` widening (fee source kinds, the
  root-advance requirement for an item-only source, and the entry-removal proof) and field numbers are registered by NPC-TRADE-1 and NPC-TRAVEL-1, not
  here.

This amendment grants no runtime or DDL authority.

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
blocked by this destination question. The gold fee amendment above (D177) admits one
transaction that combines a Character change with BURN and change-MINT lines: the Character
part advances `CharacterRevision` once with its receipt, and the item lines add no advance.
Composition §3.6 (an XP award sharing a transaction with item effects) is unchanged.

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

**Map items and world reset (ADR-0021).** `ADR-0021-world-map-runtime-loading.md` §4.4 and §4.7
(owner answer 4a, 2026-09-30) admit the two named shapes below. For these shapes only, it
supersedes three sentences:
- the §39.3 sentence that a MINT source descends from a committed `CreatureDeathOccurrenceRef`;
- the D3 sentence that `CorpseDecay` is the only path that removes a corpse's Ground row;
- the D3 sentences that `CorpseDecay` is the only `DECAY_RETIRE` cause and every other retire
  cause stays excluded (`WorldReset` is admitted; burn stays excluded).

Every other §39 obligation (fences, evidence, idempotency, current authority, conservation) is
unchanged.

- **Map-item materialization.**
  - **Eligibility.** Only a top-level map-authored entry qualifies, and only if its definition
    is pickupable, it is not on a house tile, and it carries no `action`, `unique`, `door`,
    `depot` or `teleport` binding, no contents, and no `text`, `description`, `charges` or other
    attribute that the ItemInstance state cannot represent. Every other map-authored item is
    never pickupable.
  - **Provenance.** The pickup is the player's command (`CommandRef`), under the channel's live
    scope-ownership fence.
  - **Cause.** `MapItemMaterialization {world_id, channel_id, base_bundle_digest, placement_key,
    reset_epoch}`. `placement_key` is the compiler-emitted key of the origin entry; it is
    carried even after the overlay moved the item. The whole origin entry materializes, with the
    quantity of its stack.
  - **Shape.** A MINT into typed Ground at the item's current tile, under the §39.1
    MINT-to-Ground shape, then the existing Ground-to-`CharacterInventory` TRANSFER as a separate
    transaction. If the TRANSFER does not commit, the item stays an ordinary Ground item.
  - **Idempotency.**
    - One cause commits at most one MINT.
    - A retry after the commit returns that item. The returned item passes the normal Ground,
      reach and TRANSFER checks before any TRANSFER.
    - A reservation left by an ended ownership generation is abandoned. The same cause may be
      frozen again only while no receipt exists.
  - **Overlay.** The runtime hides the origin when the MINT is frozen and unhides it only on
    proven non-commit. After a crash, the Ground rebuild hides every origin that has a receipt
    for `(world_id, channel_id, base_bundle_digest, reset_epoch)`.
- **World-reset retirement.** A durable World reset record holds `{world_id, reset_epoch N,
  target bundle digest, state RETIRING | ACTIVATED}`. A reset runs in this order:
  1. Write the record as RETIRING and close admission for every channel of the World.
  2. For each channel scope of the World, the existing assignment writer assigns a fresh
     ownership generation to the resetting node, with admission still closed. This ends the old
     generation, so in-flight fenced commits fail `fence_is_live`. The retirements run under that
     ordinary live fence. No new fence kind or scope level is introduced.
  3. Retire every live Ground root of the World and its container entries, entries first and
     then the root, in the D3 order. Each is a one-item `DECAY_RETIRE` with the cause
     `WorldReset {world_id, reset_epoch, item_instance_id}`. `WorldReset` and `CorpseDecay`
     share one per-item retirement uniqueness (one retirement per item, ever). A later
     `CorpseDecay` step for an already retired item is refused as not live.
  4. When no live Ground item of the World remains, activation writes the new bundle digest,
     epoch N+1 and state ACTIVATED atomically, bound to the content activation record.

  If the process crashes while the record is RETIRING, boot refuses admission and resumes from
  step 2, which is idempotent, then step 3. The old bundle never boots over a half-retired
  Ground.

  Only Ground roots and their entries are retired. `HouseInterior` items (HOUSE-CUSTODY-0) are
  never touched. Ground items on the tiles of a house without an owner are retired like any other
  Ground item. Every live `HouseInterior` row's `(house_key, position)` must be a tile of the same
  house in the target bundle. That check runs as a preflight before step 1, where a failure
  aborts the reset without a record, and again inside the step-4 transaction under a lock that
  blocks `HouseInterior` inserts, where a failure keeps the record RETIRING until an
  EXP-HOUSES-01 §14.7 evacuation.
- **Registration.** MAP-OVERLAY-1 registers:
  - the proto and registry fields of both shapes;
  - the reset record;
  - the epoch storage;
  - the widening of both `0015` tables, `game_item_decay_retire_reservations` and
    `game_item_decay_retire_receipts`, with a cause discriminator (`CorpseDecay` or
    `WorldReset`), so both causes share their per-item primary key:
    - `corpse_item_instance_id` and `deadline` (with `deadline >= 60000`) are required only for
      `CorpseDecay`;
    - the `{world_id, reset_epoch}` binding is required only for `WorldReset`;
    - a discriminator CHECK on each table enforces both rules;
  - the matching extension of the Ground-removal proof triggers.

  This amendment grants no runtime or DDL authority.

**Item use amendment (ITEM-USE-0, 2026-09-30), pending on acceptance of ITEM-USE-0.**
`reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md` §4, once accepted,
admits, in its ITEM-USE-1 child and for these shapes only:

- **Burn.** One BURN line (§17) of exactly one unit from the used stack, which keeps its identity
  (§11.1) or retires at zero (§11.5), under the closed sink `ItemUseCause` (`Food`, `Potion`),
  keyed by the using command's CommandRef.
- **Flask.** For a potion whose content names an empty flask, one unit-level TRANSFORM line (§17)
  instead: one unit of the used stack in, one flask unit out, either as a quantity adjustment of a
  compatible flask stack in the main backpack (§11.1) or as a fresh flask item in a new entry,
  planned in the reservation (§11.3). §16.1's instance policy does not apply, because no whole
  instance changes type. At most two items. Nothing is minted.
- **Supersession.** For these shapes only, the §39.1 exclusions of burn, transform, mint into an
  existing stack and multiple touched items. Every other §39 obligation is unchanged. One audit
  event per use (a `OneItemTransactionV1` operation assigned by ITEM-USE-1), committed before the
  use's effect, with its own suffixed resource rows.

**Amendment (pending on acceptance of QUEST-GATE-0; `reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md` §5.4).**
Once accepted, in its NPC-QUEST-1 child:

- **Dialogue claim.** A `RewardClaim` MINT whose source cause is the confirming NPC talk occurrence
  and the claim, beside the D40 `USE` child, with the same claim rules (D40-D42).
- **Exchange.** One item-only transaction under the closed cause `QuestExchangeCause {npc, node,
  exchange_key, occurrence}`: at most 8 BURN lines (§17) from direct entries of the main backpack
  (§11.1, §11.5), the claim's MINT lines and its `RewardClaim` row (D42) when the node rewards, and
  one quest obligation row. Every check precedes every write: it is refused with nothing written
  when an item is missing, the claim is not allowed, or any QUEST-STATE-0 §4 validation of the
  transition fails (`STAGE_MISMATCH`, `REVISION_MISMATCH`, `OUT_OF_RANGE`, `NOT_SUPPORTED`), or
  the character already holds 64 pending quest obligations (`OBLIGATIONS_FULL`). An exchange always
  names a transition.
- **Gold hand-in** (owner Q1b). An exchange that takes gold is also a fee under the
  `FeeBurnCause` variant `QuestExchange(QuestExchangeCause)`, an item-only fee source in the same
  transaction: coins first, then the bank part of the bank fee amendment above (one `FEE_DEBIT`
  value line, `DUR03-RL-03-FEE`), refused with nothing written on insufficient funds or a junior
  payer whose coins are short.
- **Supersession.** For these shapes only, the §39.1 exclusions of burn, of multiple touched items
  and of a MINT committed with other lines: the rewarded exchange commits the claim's MINT lines
  together with the BURN lines, the `RewardClaim` row and the quest obligation row in one
  transaction, with one audit aggregate (one event, one cause record) covering all its lines. The
  MINT admission is bounded to the named claim's declared items, and the lines are never
  independent of the BURN lines. Every other §39 obligation is unchanged; its rows are suffixed
  `-QUEST-EXCHANGE`.

**Amendment (pending on acceptance of RUNE-USE-0; `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`
§39.3).** `reviews/OTERYN_GAME_RUNE_USE0_USING_RUNES_DECISION_2026-09-30.md` §5 and §10, once
accepted, admit for these shapes only: (a) **rune use**, the item use burn shape above under a
third `ItemUseCause` variant `Rune`, keyed by the using command's CommandRef and committed before
the rune's effect (RUNE-1); (b) **conjure**, in one transaction under the closed `ConjureCause`
keyed by the cast's CommandRef, one BURN line of one reagent unit (§11.1 or §11.5) and one MINT
line (§14) of the conjured units into a compatible stack or a fresh entry planned in the
reservation (§11.3), at most two items (RUNE-CONJ-1); its receipt and audit event record the
conjure's mana and soul debit as the MINT source, and the runtime settles or releases the
caster's holds only from that durable outcome (RUNE-USE-0 `RUNEUSE0-C2`). Both supersede the §39.1 exclusions of burn,
mint into an existing stack and multiple touched items for these shapes only, with one audit
event each and their own suffixed rows (`DUR03-RL-01-RUNE`, `DUR03-RL-01-CONJURE`).

**Amendment (pending on acceptance of RANGED-0;
`reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md`
§6).** Once accepted, in its RANGED-1 child and for these shapes only, under the closed cause
`WeaponUseCause {Ammunition, Throwing}` keyed by the swing `(WorldId, ChannelId, scope ownership
generation, runtime actor id, actor generation, swing sequence, CharacterId)`, fenced by the
composition decision's server-originated variant (the actor's current admitted session's
`CurrentCharacterItemFence`, no CommandRef) and, for a Ground drop, the §32 scope fence; one
transaction per swing, with the shot unit reserved under §7.1 at PREPARE and committed before the
swing's effect:

- **Ammunition burn.** One BURN line (§17) of exactly one unit from the shot direct entry of the
  equipped quiver, which keeps its identity (§11.1) or retires at zero (§11.5).
- **Throwing burn.** The same one-unit BURN from the right-hand stack when the frozen break draw
  breaks it or the drop was refused at PREPARE.
- **Throwing drop.** One unit of the right-hand stack to the landing tile's Ground as a new item: a §12
  split into a planned identity (§11.3), or a whole TRANSFER from the right-hand slot when it is the
  last unit; under the ITEM-MOVE-WIRE-1 §5 Ground rules (tile and channel limits, house tiles refused,
  the tile row lock and counter), its §6.2 scope fence, and D191 reset retirement. A database refusal
  is a refused commit under the same TransactionId (§23).
- **Supersession.** For these shapes only, as ITEM-MOVE-WIRE-1 §6.1 did for its own: the §39.1
  exclusions of burn, multiple touched items (at most 2), quantity redistribution (the split), nested
  containers (an entry of the equipped quiver, depth 1), the source custodies (the right-hand slot and
  quiver entries as BURN or split sources), Ground insertion by split, and a whole TRANSFER from a slot
  to Ground under `WeaponUseCause`. Every other §39 obligation is unchanged. One audit event per swing
  consequence (a `OneItemTransactionV1` operation assigned by RANGED-1); rows `DUR03-RL-0x-WEAPON` as
  RANGED-0 §6.3 (one participant per touched item).

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
