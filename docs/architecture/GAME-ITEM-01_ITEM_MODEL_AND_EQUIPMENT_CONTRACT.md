# GAME-ITEM-01 — Item Model and Equipment Contract

- Date: 2026-08-12
- Gate: `GAME-ITEM-01`
- Delivery task: `OTV2-20260812-game-item-01-architecture`
- Delivery PR: #205
- Status on delivery branch: **CANDIDATE; canonical when this delivery merges**
- DecisionStatus after accepted merge: **ACCEPTED**
- DeliveryStatus while PR is active: **IN_REVIEW**
- ImplementationStatus: **NOT_STARTED**
- Runtime authority: **NONE**
- PostgreSQL DDL/migration authority: **NONE**
- Owner authority source: explicit 2026-08-12 instruction to continue Oteryn-v2 architecture autonomously according to the canonical repository handoff
- Analysis source: `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md`

## 1. Purpose

Freeze the minimum native item semantic contract required before `DUR-03` can define authoritative item/currency/value transaction and anti-duplication invariants.

This contract owns:

- stable item-definition versus concrete item-instance semantics;
- legal typed item state;
- equipment legality and occupancy semantics;
- container legality and bounded containment;
- binding/transfer-restriction semantics;
- item-definition compatibility and migration meaning;
- deterministic item-modifier contribution ordering requirements;
- semantic boundaries with content, persistence, transactions, protocol, analytics and downstream gameplay domains.

It does not own runtime code, physical storage, wire encoding, content-bundle format, exact Reference formulas or `DUR-03` conservation/identity-transition mechanics.

## 2. Authority chain

Binding ownership is:

```text
stable content identity / authored definitions    -> ADR-0005 + DUR-04
first Reference target/evidence discipline        -> GAME-VISION-01
ItemInstanceId durable representation             -> DUR-01
item semantic legality / equipment / containers   -> GAME-ITEM-01
common PostgreSQL transaction/audit substrate     -> DUR-02
item/currency/value conservation + transactions   -> DUR-03
event/audit identity and evidence semantics       -> ANL-01
runtime execution/clocks/order                    -> FND-03
protocol wire/command semantics                    -> FND-02
simulation arithmetic/formula determinism          -> SIM-DETERMINISM-01 + owning gameplay/ruleset gates
```

No lower layer may redefine an upper semantic owner for convenience.

## 3. Canonical item concepts

### 3.1 ItemType

`ItemType` is an immutable/versioned authored semantic definition identified canonically by a stable namespaced `ItemTypeKey` owned by the Content Registry under ADR-0005.

Examples of key shape remain conceptually:

```text
oteryn:item.<namespace>.<name>
package.example:item.<name>
```

The exact grammar/encoding remains DUR-04-owned.

Rules:

- legacy numeric item IDs are mappings, not canonical type identity;
- compact runtime IDs are scoped to a compiled content revision unless a later contract explicitly guarantees otherwise;
- a stable ItemTypeKey does not imply that all revisions of its definition are semantically interchangeable;
- authoritative behavior is defined by server-owned typed semantics, never by client metadata alone.

### 3.2 ItemInstance

`ItemInstance` is one concrete mutable gameplay-item lifecycle identified by `ItemInstanceId` exactly as accepted by DUR-01:

```text
strong UUIDv7
full 128 bits
nil invalid
never reused
restricted/internal by default
```

Every live concrete mutable item instance has exactly one ItemInstanceId. ItemInstanceId does not encode type, quantity, owner, location, world, revision or time.

A live instance has, semantically:

```text
ItemInstanceId
WorldId scope
resolved ItemTypeKey + explicit compatible definition/revision context
typed capability state allowed by that definition
```

Current authoritative location is deliberately not folded into ItemInstance identity. `DUR-03` owns location and its atomic transitions.

### 3.3 StaticItemPlacement

An authored static map/world placement that references an item definition is not automatically a durable ItemInstance.

```text
StaticItemPlacement in immutable world/content bundle
!= durable mutable ItemInstance
```

If gameplay materializes an authored/static definition into mutable durable item state, that materialization is an authoritative lifecycle/transaction operation under the later runtime + DUR-03 contract.

### 3.4 Instance lifecycle envelope

GAME-ITEM-01 freezes only this lifecycle invariant:

```text
live concrete instance
-> may participate only in definition-legal state/placement/equipment/container operations
-> terminal retirement/destruction never permits ItemInstanceId reuse
```

Exact create/destroy/split/merge/transform identity transitions, survivor/new-ID rules and terminal evidence mechanics remain DUR-03-owned under DUR-01.

## 4. Typed capability model

A versioned ItemType definition declares the exact authoritative capability families an instance may possess. An instance may not carry authoritative state for an undeclared capability.

The minimum capability vocabulary is semantic, not a physical schema commitment.

### 4.1 Stack capability

A stack-capable instance carries one positive integer quantity.

Required invariants:

```text
1 <= quantity <= definition stack maximum <= absolute safety ceiling
quantity = 0 is not a live stack state
non-stackable item has no stack quantity state beyond semantic single-instance presence
```

One stack is one ItemInstance. Units inside a stack do not receive separate ItemInstanceIds solely because quantity is greater than one.

Exact split/merge ItemInstanceId transitions are DUR-03-owned.

### 4.2 Charge capability

Charge state is a separate typed bounded value.

- charge count is not stack quantity;
- charge consumption/restoration/transform eligibility is definition/ruleset-owned;
- invalid underflow/overflow fails closed;
- exact Reference behavior is parity-gated per applicable item family.

### 4.3 Durability capability

Durability is a separate typed bounded state where a definition enables it.

- durability is not charges, quantity or time-to-expiry;
- valid range/repair/degradation semantics are versioned;
- exact arithmetic and Reference behavior remain ruleset/SIM/evidence-owned.

### 4.4 Temporal/decay capability

A temporal item definition must state an explicit time-consumption semantic rather than use one ambiguous generic duration field.

At minimum, the model can distinguish:

- durable absolute deadline semantics;
- authoritative active-time budget semantics;
- future explicitly accepted time modes.

FND-03 owns runtime clock execution semantics. Persistence must retain enough typed state to avoid interpreting one time mode as another after restart/migration.

Exact Reference decay/expiration mapping remains parity-gated.

### 4.5 Container capability

A container-capable instance declares typed capacity/containment policy as defined in section 7.

### 4.6 Equipment capability

An equippable instance resolves typed equip patterns, requirements and modifier contributions as defined in section 6.

### 4.7 Binding and transfer-restriction capability

An item may carry versioned typed restriction state such as account/character binding or explicit transfer-surface restrictions where the owning product/ruleset definition supports them.

These restrictions are additional to mandatory logical-world value isolation and are not equivalent to current location or authorization.

### 4.8 Upgrade/modifier capability

Upgrade/tier/enchantment/modifier state is typed, bounded and resolves to stable versioned definitions.

No arbitrary free-form attribute map may define authoritative bonuses or item behavior.

## 5. Authoritative state guardrails

The following are rejected as canonical authoritative item-state design:

- arbitrary JSON item-attribute bags;
- free-form EAV semantic keys;
- serializer-specific opaque blobs without a versioned semantic schema;
- client-only fields that determine equip/container/transfer legality;
- unbounded modifier lists or nested extension objects;
- scripts with authority to add arbitrary state keys or write persistence directly.

A new authoritative item behavior category requires a typed schema/domain extension with explicit validation and compatibility rules.

Presentation-only metadata may be more flexible provided it is cryptographically/integrity-separated or otherwise impossible to become server authority accidentally.

## 6. Equipment contract

### 6.1 Semantic slot identity

Equipment slots use stable strongly typed semantic keys owned by the ruleset/content contract.

They are not:

- client UI indices;
- protocol opcode values;
- database column positions;
- implicit Rust enum discriminants exposed as durable identity.

Exact first-Reference slot vocabulary and special cases remain parity evidence until proven against the accepted target.

### 6.2 Equip patterns and occupancy claims

An equippable ItemType declares one or more valid equip patterns.

Each pattern contains the complete semantic occupancy claim:

```text
primary slot
+ every additional slot/resource reserved by that equipped item
+ mutually-exclusive occupancy groups where applicable
+ typed requirements
```

An equip operation is legal only when one complete pattern validates atomically against authoritative state.

Consequences:

- two-handed behavior is represented by one item claiming/reserving the relevant hand-slot resources atomically;
- mutually exclusive equipment cannot be committed through separate client-visible slot updates;
- one-half-success states are invalid;
- the client may request/present an equip choice but cannot decide legality.

The transaction that moves an item into/out of equipment remains DUR-03-owned.

### 6.3 Typed requirements

Equipment requirements consume authoritative typed inputs only, including where applicable:

- Character progression/build/vocation/profile facts owned by GAME-CHAR/ruleset contracts;
- Platform-owned entitlement facts only through a separately accepted Oteryn-v2 consumer/enforcement contract; GAME-ITEM-01 does not accept or activate `PROD-ENTITLEMENTS-01`;
- item capability/state;
- world/profile/ruleset capability state.

No arbitrary unversioned script predicate is part of the authoritative equipment contract.

### 6.4 Deterministic modifier contributions

Every authoritative item modifier resolves to a versioned typed modifier definition containing at minimum:

- target stat/effect domain;
- evaluation phase/category;
- explicit priority where that category permits ordering;
- bounded parameters;
- stable definition key.

Every ruleset that consumes item modifiers must publish one versioned deterministic evaluation plan.

The plan must make repeated evaluation over the same authoritative inputs produce the same ordered contribution sequence and result. Stable definition identity is used before any instance-level tie-break. If a non-commutative tie remains, a stable deterministic tie-break is mandatory; ItemInstanceId may be used only as identity, never as time/order/authority.

Exact numeric formulas, rounding and combat arithmetic remain SIM-DETERMINISM/gameplay-owned.

## 7. Container contract

A container-capable live ItemInstance participates in an authoritative directed containment graph.

Before any containment mutation may commit, legality must prove all applicable invariants:

```text
child != container
container is not reachable from child
no ancestor/self cycle after the proposed move
direct-entry/slot count within definition limit
nesting depth within absolute and definition limits
total reachable item count within absolute limit
serialized/loaded state within resource ceilings
capacity/weight rules satisfied
item-type allow/deny policy satisfied when present
```

### 7.1 Bound hierarchy

Every variable-size item structure uses two bound layers where applicable:

1. **absolute engine/security ceiling** — registered before implementation in the appropriate resource-limit contract/registry;
2. **ruleset/content limit** — may be lower but never exceed the absolute ceiling.

This applies at minimum to:

- stack quantity;
- direct container entries/slots;
- nesting depth;
- total reachable container graph size;
- modifier count/state size;
- transformation/trigger chain depth where item semantics can recurse.

Missing mandatory absolute ceilings are an implementation blocker; they are not interpreted as unlimited.

Numeric ceilings are deliberately not invented by this paper-only gate without workload/security evidence.

### 7.2 Weight/capacity

Weight/capacity legality is server-authoritative and deterministic over the resolved item definitions, quantities and containment state required by the active ruleset.

Exact Reference formula, inclusion/exclusion cases and limits remain parity-gated. The client may display a derived value but cannot make an otherwise-illegal move valid.

### 7.3 Ownership boundary

GAME-ITEM-01 owns container graph **legality**. DUR-03 owns:

- atomic old/new location transition;
- single-authoritative-location invariant;
- idempotency/retry;
- stale-session/writer rejection;
- crash/partial-failure recovery;
- conservation and mandatory durable evidence.

## 8. World scope, binding, ownership and authorization

Every live gameplay ItemInstance belongs to exactly one logical `WorldId` value scope unless a later explicit transfer architecture safely supersedes that rule.

The following concepts are distinct and must not be collapsed:

```text
World scope             where gameplay value is allowed to exist
binding/restrictions    which subject/surfaces may transfer/use it
current location         inventory/equipment/container/ground/etc.; DUR-03-owned
authorization            which current authority may act; session/domain-owned
display ownership        presentation only unless backed by an accepted rule
```

A generic `owner_id` field that ambiguously stands for more than one of these concepts is prohibited in canonical semantics.

Account or character binding never authorizes a stale GameSession/CharacterLease and never overrides FND-04 authority.

## 9. Physical currency versus non-item value

A physical currency object may be an ItemType/ItemInstance when the content/ruleset models it as a physical item.

A bank balance, account ledger balance, token balance or other non-item fungible value is not forced into ItemInstance merely to reuse this model.

```text
physical currency item
-> GAME-ITEM semantics + DUR-03 conservation

non-item currency/value ledger
-> owning economy/bank semantic contract + DUR-03 value conservation
```

This separation prevents one item abstraction from becoming the universal economy data model.

## 10. Definition revision and compatibility

Each authoritative ItemInstance must be interpretable under an explicit compatible item-definition context.

A stable ItemTypeKey alone is insufficient to silently reinterpret persisted state after content change.

Every authoritative semantic definition change is classified as one of:

### `PRESENTATION_COMPATIBLE`

Changes only presentation/non-authoritative metadata and cannot affect server legality, conservation, formulas, requirements, state interpretation or migration.

### `AUTHORITATIVE_COMPATIBLE`

New definition/runtime remains semantically compatible with already persisted authoritative state. Compatibility requires explicit proof, not assumption from identical field names.

### `MIGRATION_REQUIRED`

Authoritative meaning/state shape changes incompatibly. Existing state must undergo deterministic version-aware migration/backfill and validation before the new meaning becomes authoritative.

### `UNSUPPORTED`

No safe compatibility/migration path is accepted. Load/cutover fails closed.

DUR-02 expand/migrate/validate/cutover/contract discipline applies to physical persistence migration. DUR-04 owns item-definition source/bundle schema evolution.

No runtime may reinterpret incompatible historical item state under a newer definition merely because the ItemTypeKey is unchanged.

## 11. Transform, split and merge boundary

GAME-ITEM-01 may define whether an ItemType semantically permits operations such as:

- quantity change under stack rules;
- charge consumption/restoration;
- durability degradation/repair;
- decay/expiration;
- typed transform edges between item definitions;
- crafting/upgrade eligibility.

GAME-ITEM-01 does **not** define ItemInstanceId transition mechanics for those operations.

DUR-03 must decide and prove:

- create/destroy identity outcomes;
- split/merge survivor versus new IDs;
- transform preserve-versus-replace identity rules;
- multi-input/multi-output lineage;
- atomic commit/abort/rollback results;
- provenance/conservation evidence.

No GAME-ITEM implementation may invent these DUR-03 rules ahead of that gate.

## 12. Transfer-surface boundary

Item definitions/restrictions may state semantic eligibility for bounded transfer surfaces such as inventory, equipment, container, ground, trade, market, depot, mail, reward, house or other later accepted surfaces.

Eligibility is not a transaction implementation.

| Surface | GAME-ITEM-01 | Owning downstream authority |
|---|---|---|
| inventory/equipment/container/ground | item legality | DUR-03 atomic transfer/conservation |
| loot/corpse | resolved item legality | combat/loot cause + DUR-03 mint/transfer |
| trade | restriction eligibility | trade lifecycle + DUR-03 atomic exchange |
| market | market eligibility | EXP-ECONOMY-01 + DUR-03 value conservation |
| bank | physical item versus ledger distinction | bank/economy + DUR-03 value conservation |
| depot | item/container legality | depot access/domain + DUR-03 move |
| mail | transfer restriction | mail lifecycle + DUR-03 move |
| rewards | output item definition legality | reward owner + DUR-03 mint/transfer |
| houses | placement/container legality | EXP-HOUSES-01 + DUR-03 move |

Future transfer surfaces require an explicit typed semantic extension; arbitrary free-form surface names do not grant authority.

## 13. Protocol and client boundary

- FND-02 owns `protocol-oteryn` encoding, command sequencing and wire compatibility.
- The client may receive presentation-safe item definitions/state snapshots and submit bounded intents.
- The client never becomes authority for stack amount, charges, durability, equipment legality, modifiers, container capacity, binding or item transformation.
- Wire/runtime compact IDs do not replace stable ItemTypeKey/ItemInstanceId semantics.
- Unsupported revision/state fails closed rather than client/server guessing different meanings.

## 14. Content and scripting boundary

DUR-04 owns concrete item source schema, World Project/Bundle encoding, Content Registry package/version rules, compiler validation and scripting runtime/API.

DUR-04 must consume this contract and ensure:

- authored item definitions compile into typed capabilities;
- stable keys and revision compatibility are retained;
- server-authoritative fields are separated from client-safe presentation data;
- bounded values are validated before bundle acceptance;
- scripts cannot create arbitrary item state, bypass equipment/container legality, bypass DUR-03 transactions or access SQL directly;
- transform/trigger graphs cannot recurse without an explicit bounded policy.

## 15. Persistence and audit boundary

DUR-02 remains the common persistence substrate owner. DUR-03 will specialize item transaction correctness.

For any item mutation later declared to require durable audit:

```text
authoritative mutation
+ required DUR-03 conservation/provenance result
+ mandatory ANL-01 durable evidence
-> one accepted atomic transaction boundary
```

ANL-01 owns EventId/OperationId/TransactionId/event envelope/privacy/retention semantics. Analytics and Game Intelligence may reconcile item invariants but never mutate authoritative item state or substitute for DUR-03 prevention.

High-cardinality ItemInstanceId remains prohibited as an ordinary metrics label under ANL-01.

## 16. Reference parity contract

The accepted first Reference cut is unchanged: Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary.

This contract intentionally defines **representation capability without inventing exact target mechanics**.

For every exercised Reference item behavior, release/implementation evidence must classify it as:

- `PARITY_CONFIRMED`;
- `PARITY_PENDING_EVIDENCE`;
- `PARITY_CONFLICT`;
- `DECLARED_DIFFERENCE`;
- `OUT_OF_SCOPE`.

At this architecture cut, exact target-specific stack maxima, charge semantics, durability behavior, decay timing, slot vocabulary/requirements, two-hand cases, modifier/tier/enchantment arithmetic, weight/capacity limits, transfer restrictions and visible transform/split/merge edge cases remain `PARITY_PENDING_EVIDENCE` unless an existing separately accepted evidence record proves a narrower behavior.

No OTS implementation is proof of Reference behavior.

Security/integrity overrides remain binding: Reference fidelity never requires duplication, stale-authority writes, corrupt durable state or unsafe replay.

## 17. Determinism requirements

For the same accepted ruleset/content revisions and authoritative input state:

- capability validation must be deterministic;
- equipment pattern selection/legality must be deterministic for a specified intent;
- modifier contribution ordering must be deterministic;
- container legality/weight/capacity evaluation must be deterministic;
- definition revision resolution must not depend on map/hash iteration order;
- invalid/unsupported state must not be auto-normalized differently across builds.

Exact simulation formulas/rounding remain SIM-DETERMINISM-owned, but item semantics may not introduce nondeterministic input ordering.

## 18. Security and integrity invariants

Mandatory invariants:

- no unbounded item-controlled allocation or recursion;
- no client authority for item legality/state mutation;
- no arbitrary authoritative attribute/script escape hatch;
- no cross-world gameplay-value move by default;
- no ItemInstanceId reuse;
- no silent definition reinterpretation;
- no container cycle;
- no partial multi-slot equip claim;
- no unsupported capability state on an instance;
- no stale session/lease authorization inference from binding/ownership metadata;
- no analytics/remediation path that mutates item authority outside accepted gameplay/DUR contracts.

## 19. Decision timing

### Must decide now?

**YES.**

`DUR-03` and broad authoritative item/content modeling are blocked without a stable definition/instance/capability/equipment/container semantic contract.

### Downstream work unblocked after accepted merge and lifecycle closeout

`DUR-03` remains blocked during the interval after the GAME-ITEM delivery merges and before the separate GAME-ITEM lifecycle closeout is merged. It becomes eligible for its own paper-only architecture task only after that closeout atomically promotes maintained status overlays.

After accepted merge **and** lifecycle closeout, the following work is unblocked:

- `DUR-03` paper-only item transaction and anti-duplication architecture;
- bounded item-definition schema work under DUR-04 without inventing transaction semantics;
- durable vertical-slice loot/pickup transaction design;
- typed item/economy audit and later Game Intelligence reconciliation design.

### What becomes materially harder if changed later?

Changing concrete-item identity meaning, stack representation, capability typing, equipment occupancy, containment legality or revision interpretation after durable item data exists would require high-risk data/event/content migrations and could invalidate anti-duplication proofs.

### Supersession evidence

Reopen only with named evidence such as:

- a DUR-03 conservation proof that this semantic boundary cannot represent safely;
- a proven first-Reference mechanic that cannot be expressed within typed extensions;
- security/exploit evidence;
- measured runtime/content evidence demonstrating an unacceptable structural cost while preserving all invariants with a better typed design;
- an explicit later owner product/profile decision.

Convenience, OTS data layout or framework preference is insufficient.

### Deliberately not decided

- DUR-03 split/merge/transform ItemInstanceId transition rules;
- SQL schema/constraints/indexes/isolation/locking;
- Rust crate/module/type layout;
- exact numeric resource ceilings without evidence;
- exact Reference values/formulas without evidence;
- concrete content source/bundle encoding and scripting runtime;
- exact combat/ability/market/house/reward/economy policy;
- production topology or rollout.

## 20. Acceptance consequence

When this delivery is accepted and lifecycle-closed:

```text
GAME-ITEM-01
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL authority        = NONE

DUR-03
paper-only architecture = UNBLOCKED
runtime item mutation    = still NOT AUTHORIZED
```

Acceptance does not claim complete Reference item parity and does not authorize runtime/client/SQL/content implementation.
