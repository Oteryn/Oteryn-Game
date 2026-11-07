# GAME-ITEM-01 — Item Model and Equipment Architecture Analysis

- Date: 2026-08-12
- Gate: `GAME-ITEM-01`
- Delivery task: `OTV2-20260812-game-item-01-architecture`
- Delivery PR: #205
- Scope: native item semantics, equipment legality and item-definition compatibility only
- DecisionStatus at analysis start: `PROPOSED`
- ImplementationStatus: `NOT_STARTED`
- Runtime authority: `NONE`
- PostgreSQL DDL/migration authority: `NONE`

## 1. Problem

`GAME-ITEM-01` must define the semantic object that later item transactions conserve. Without this boundary, `DUR-03` cannot prove anti-duplication, single-location or transfer invariants because it would not know what constitutes one concrete item, which mutable fields are legal for a type, how equipment/container legality is evaluated, or how a persisted item is interpreted across content revisions.

The gate must be strong enough to unblock `DUR-03` and broad item/content modeling, but it must not capture `DUR-03` transaction identity-transition rules, `DUR-04` physical content encoding/scripting, SIM formula ownership, or exact unevidenced Reference behavior.

## 2. Binding constraints

### 2.1 First Reference target

`GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` fixes the first Reference target to Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary.

For Reference-sensitive item behavior:

```text
PROVEN / OBSERVED / DERIVED
-> may constrain a parity case with recorded evidence

UNKNOWN / CONFLICT
-> may NOT be filled from current Global, Canary, crystalserver, another OTS or implementation convenience
-> remains PARITY_PENDING_EVIDENCE / PARITY_CONFLICT
-> affected claimed Reference behavior stays disabled, excluded or otherwise fail-closed
```

### 2.2 Content identity

ADR-0005 makes stable namespaced Content Registry keys canonical. Legacy numeric IDs and compact runtime IDs are mappings. Compact IDs are scoped to a compiled content revision and are not durable item-type identity by default.

### 2.3 Durable concrete-item identity

`DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md` already defines:

```text
ItemInstanceId
  = strongly typed UUIDv7
  = globally unique for one concrete item-instance lifecycle
  = never reused
  = does not encode type, quantity, owner, location, scope or revision
```

DUR-01 explicitly leaves create/destroy/split/merge/transform identity-transition rules to `DUR-03`. GAME-ITEM-01 must not redefine that authority.

### 2.4 Persistence and audit

`DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` owns the common persistence substrate and common transaction-correctness rules. It moves inventory/equipment/ground-item transfer transaction boundaries to GAME-ITEM-01 + DUR-03.

`ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md` owns EventId/OperationId/TransactionId and durable audit semantics. Item/economy evidence may describe item semantics but does not become gameplay authority.

### 2.5 Static authored world versus dynamic item state

ADR-0005 keeps authored static world definitions separate from authoritative dynamic state. Therefore a static bundle placement that merely references an item definition is not automatically a durable `ItemInstance` row or lifecycle. Materialization into mutable gameplay state is an explicit authoritative operation owned by the later runtime/transaction contracts.

## 3. Realistic options

### Option A — generic attribute bag / EAV item model

One generic item entity stores arbitrary key/value attributes and scripts interpret them.

Benefits:
- fast to prototype;
- broad authoring flexibility.

Costs/risks:
- weak validation and type safety;
- hard-to-prove migration compatibility;
- ambiguous conservation semantics for quantity/charges/durability;
- hidden behavior in scripts/clients;
- poor deterministic ordering;
- large exploit and malformed-state surface.

**Disposition: reject for authoritative item semantics.** A bounded extension payload may exist only in a future separately accepted contract with a typed schema, owner and validation rules; it cannot be an authoritative escape hatch.

### Option B — typed capability composition over stable item definitions

A stable `ItemTypeKey` resolves a versioned item definition. The definition declares a bounded set of typed capabilities. A concrete `ItemInstance` stores only state allowed by those capabilities.

Benefits:
- strong validation;
- content-driven without deep engine subclassing;
- deterministic compatibility/migration;
- clear DUR-03 conservation surface;
- supports Reference parity plus later Evolved extension through versioned definitions.

Costs:
- capability schemas require disciplined evolution;
- some new mechanics require explicit schema/domain extensions rather than arbitrary data injection.

**Disposition: recommend.**

### Option C — closed Rust subtype/class hierarchy for every item family

Each item family becomes a dedicated engine subtype with hand-coded fields/behavior.

Benefits:
- strongest compile-time specialization.

Costs/risks:
- content changes require engine releases;
- poor fit for large data-driven item catalogues;
- encourages gameplay/content coupling to runtime implementation details;
- expensive migration and authoring workflow.

**Disposition: reject as the universal model.** Typed Rust logic remains appropriate for invariant-bearing operations, but content definitions should not require one code subtype per item family.

## 4. Recommended semantic model

## 4.1 Definition versus instance versus static placement

Use three distinct concepts:

```text
ItemType
  canonical identity: stable namespaced ItemTypeKey from Content Registry
  purpose: immutable/versioned semantic definition

ItemInstance
  canonical identity: ItemInstanceId from DUR-01
  purpose: one concrete mutable gameplay item lifecycle

StaticItemPlacement
  identity: authored world/bundle placement identity as owned by world/content architecture
  purpose: immutable authored placement/reference
  not automatically a durable ItemInstance
```

An `ItemInstance` references one stable `ItemTypeKey` plus enough explicit version context to interpret its authoritative state. The physical encoding of that version context is deferred to DUR-04/DUR-03 implementation contracts.

A stack is one concrete item instance carrying a positive quantity under a stack capability. Individual fungible units inside that stack do not acquire separate `ItemInstanceId` values merely because quantity is greater than one. Split/merge instance-ID transitions remain `DUR-03`-owned.

## 4.2 Typed capability composition

An item definition declares only applicable typed capabilities. A live instance may carry state only for capabilities its resolved definition permits.

Minimum semantic capability families required by this gate:

1. **Stack capability**
   - positive bounded quantity;
   - explicit maximum quantity from versioned ruleset/content policy;
   - zero quantity is not a live stack;
   - exact Reference stack maxima are parity data, not guessed architecture constants.

2. **Charge capability**
   - bounded current charge state;
   - definition/ruleset owns what consumes/restores/transforms charges;
   - charges are distinct from stack quantity and durability.

3. **Durability capability**
   - bounded typed durability state where the item family uses durability;
   - durability is distinct from charges and expiration;
   - exact loss/repair formulas remain ruleset/Reference evidence owned.

4. **Temporal/decay capability**
   - explicit typed temporal semantics rather than an ambiguous generic `duration` field;
   - the definition must distinguish an absolute durable deadline from an authoritative active-time budget or another later accepted time mode;
   - FND-03 owns runtime clock execution semantics;
   - exact Reference decay behavior remains parity-gated per item family.

5. **Equipment capability**
   - typed equip patterns/slot claims;
   - typed requirements and exclusions;
   - typed modifier contributions;
   - client presentation never grants equip authority.

6. **Container capability**
   - bounded direct-entry/slot capacity policy;
   - bounded nesting/reachable-node policy;
   - cycle prohibition;
   - deterministic capacity/weight legality contract.

7. **Binding/transfer-restriction capability**
   - typed world/account/character restrictions where applicable;
   - allowed/forbidden transfer surfaces are explicit policy, not UI convention.

8. **Upgrade/modifier capability**
   - typed bounded upgrade/tier/enchantment/modifier state;
   - each state entry resolves to a versioned definition, not an unbounded free-form map;
   - exact Reference upgrade systems and formulas remain their evidenced content/ruleset semantics.

Capabilities are orthogonal unless an accepted item definition declares an invariant between them. For example, stack quantity must not be reused as charges merely because both are integers.

## 4.3 Authoritative state shape

Authoritative item state must be structurally typed and versioned. Prohibited as the canonical escape hatch:

- arbitrary JSON object of item attributes;
- EAV rows with free-form semantic keys;
- client-only attributes that determine legality;
- unvalidated script-owned mutation of arbitrary fields;
- serializer-specific blobs whose meaning is not tied to a schema/revision.

A future extension may introduce new typed capability schemas through explicit compatibility/migration rules.

## 5. Equipment model

### 5.1 Slot identities

Equipment slots are typed ruleset/content semantic keys, not protocol opcodes, UI indices or database column positions. Their exact physical encoding is not decided here.

The first Reference slot vocabulary and special cases must be evidenced against the accepted target before a parity claim. The engine contract only requires stable semantic slot identities and bounded slot cardinality.

### 5.2 Atomic occupancy claims

Each equippable definition exposes one or more valid **equip patterns**. A pattern states:

- primary semantic slot;
- the complete set of slots/resources it occupies or reserves;
- mutually-exclusive occupancy groups where required;
- applicable typed requirements.

An equip attempt is legal only if one complete pattern validates atomically against the authoritative Character/ruleset/item state.

This models two-handed and other multi-slot/exclusive equipment without making the client or a UI slot the authority.

The item movement/ownership transaction that commits the equip remains `DUR-03`-owned.

### 5.3 Requirements

Requirements are typed predicates over authoritative inputs such as accepted Character progression/build/profile facts, item state and explicit entitlement/ruleset inputs. Free-form script predicates are not part of the GAME-ITEM-01 authoritative contract.

The exact Reference requirement catalogue and values remain parity/content/ruleset-owned.

### 5.4 Modifier ordering

GAME-ITEM-01 requires every item-contributed modifier to resolve to a versioned typed modifier definition with:

- target stat/effect domain;
- evaluation phase/category;
- explicit priority when the phase permits ordering;
- bounded parameters;
- stable definition key.

Each ruleset publishes one versioned deterministic modifier-evaluation plan. The plan defines phase ordering and the reduction semantics needed for reproducible results. Where an operation remains order-sensitive after phase/priority/key ordering, a stable final tie-break is mandatory and may use `ItemInstanceId` only as deterministic identity, never as chronology/authority.

`SIM-DETERMINISM-01` and the owning combat/ruleset gates own exact numeric formulas, rounding and authoritative simulation arithmetic. GAME-ITEM-01 owns the requirement that item contribution order is explicit and reproducible.

## 6. Container model

A container-capable item instance is a node in an authoritative containment graph.

Required legality invariants before any move may commit:

```text
no self containment
no ancestor cycle
bounded direct entries/slots
bounded nesting depth
bounded total reachable item count
bounded serialized/loaded state
capacity/weight policy satisfied
all child item types allowed by any explicit container policy
```

Limits are policy/resource-limit values, not unbounded implementation defaults. Exact Reference values remain parity-pending until evidenced.

GAME-ITEM-01 owns the **legality predicate**. `DUR-03` owns the atomic move, old/new location transition, idempotency, single-authoritative-location proof and crash recovery.

## 7. Binding, ownership and value scope

Distinguish:

- **World scope** — gameplay value remains within its logical world by default under accepted product architecture;
- **binding/restriction** — additional account/character/transfer-surface rules on an item;
- **authoritative location** — where the concrete instance currently exists; `DUR-03` owns this;
- **authorization** — who may currently act; session/lease/domain owners decide this;
- **display owner/name** — presentation only unless backed by an authoritative rule.

No `owner_id` convenience field may collapse these concepts.

A physical currency object may be modeled as an item when content semantics say it is an item. A ledger balance or non-item fungible value account is not an `ItemInstance`; its value-conservation rules remain `DUR-03` plus the owning economy/bank domain.

## 8. Definition revision and migration

A stable `ItemTypeKey` is not permission to reinterpret historical persisted state silently.

Each authoritative instance must be interpretable against an explicit compatible item-definition revision/context. A content rollout classifies each semantic item-definition change as one of:

1. **presentation/non-authoritative compatible** — may update without changing authoritative meaning;
2. **authoritative backward-compatible** — new reader/runtime accepts existing state with identical old semantics and explicit compatibility proof;
3. **authoritative migration-required** — deterministic migration/backfill transforms existing item state before incompatible semantics become authoritative;
4. **unsupported/incompatible** — rollout fails closed until a migration or declared compatibility path exists.

No current runtime may load persisted item state under a semantically incompatible newer definition merely because the stable ItemTypeKey is unchanged.

Migration follows accepted DUR-02 expand/migrate/validate/cutover/contract discipline. Exact bundle/schema encoding belongs to DUR-04/implementation.

## 9. Transformation boundary

GAME-ITEM-01 defines which **state transitions are semantically legal for an item definition**, including whether a type may stack, consume charges, lose durability, decay or transform into another item definition.

It does **not** decide:

- which input ItemInstanceId survives a split/merge;
- whether a transform preserves or replaces instance identity;
- how new IDs are minted within a multi-item operation;
- atomic create/destroy accounting;
- commit/abort/rollback identity outcomes.

Those are explicitly `DUR-03` decisions under DUR-01.

## 10. Domain boundaries

| Domain/surface | GAME-ITEM-01 owns | Other owner retains |
|---|---|---|
| loot/corpse | item semantic legality and resolved definitions | combat/loot cause + DUR-03 creation/transfer/conservation |
| trade | transfer restriction eligibility | trade lifecycle/offer policy + DUR-03 atomic exchange |
| market | item eligibility/definition semantics | EXP-ECONOMY-01 order/price/market policy + DUR-03 value conservation |
| bank | distinction between physical item and ledger value | bank/economy semantics + DUR-03 currency/value conservation |
| depot | container/item legality | depot access/domain policy + DUR-03 move |
| mail | item transfer restrictions | mail lifecycle/delivery policy + DUR-03 move |
| rewards | resolved item result legality | reward eligibility/source policy + DUR-03 mint/transfer |
| houses | item/container legality | EXP-HOUSES-01 ownership/placement policy + DUR-03 move |
| protocol/client | presentation-safe typed snapshots/intents | FND-02 wire semantics; server remains authority |
| content | semantic schema requirements | DUR-04 source/bundle encoding, compiler and scripting runtime |
| persistence | semantic invariants | DUR-02 common substrate; DUR-03 item transaction proof |
| analytics | typed evidence meaning | ANL-01 event semantics; ANL-02/03 read-only consumers |

## 11. Reference parity evidence matrix at this architecture cut

The accepted Reference target is fixed, but this task does not claim exhaustive target evidence for item mechanics.

| Behavior family | Architecture status | Reference parity status |
|---|---|---|
| definition vs concrete instance separation | native architecture decision | `NOT_A_DIRECT_PARITY_CLAIM` |
| exact stackable item catalogue/maxima | representable | `PARITY_PENDING_EVIDENCE` |
| exact charges semantics per item | representable | `PARITY_PENDING_EVIDENCE` |
| exact durability semantics, where any | representable | `PARITY_PENDING_EVIDENCE` |
| exact decay/expiration timing | representable | `PARITY_PENDING_EVIDENCE` |
| exact equipment slot vocabulary/requirements/two-hand cases | representable | `PARITY_PENDING_EVIDENCE` |
| exact bonuses/resistances/tier/enchantment ordering/formulas | representable | `PARITY_PENDING_EVIDENCE` |
| exact container/weight/capacity/nesting values | representable | `PARITY_PENDING_EVIDENCE` |
| exact binding/trade/market/depot/mail restrictions | representable | `PARITY_PENDING_EVIDENCE` |
| exact transform/split/merge visible behavior | semantic hooks present; identity transition deferred | `PARITY_PENDING_EVIDENCE` + `DUR-03` |

This is intentional: architecture accepts a safe semantic envelope without fabricating Global behavior.

## 12. Player and producer review

### Player-facing impact

Benefits:
- progress/value safety can be proven later because every concrete mutable item has explicit semantics and identity;
- equip/container behavior cannot depend on client-only state;
- deterministic modifier order reduces desync and unexplained stat differences;
- Reference claims remain evidence-backed rather than approximated.

Risks to control later:
- over-restrictive capability schemas could slow content iteration;
- poor migration tooling could delay content updates;
- strict fail-closed Reference gaps may temporarily narrow exercised parity scope.

### Producer/engineering impact

Benefits:
- one semantic model supports Reference and Evolved profiles without forks;
- typed capability composition avoids one Rust class per item while preserving validation;
- DUR-03 receives a precise conservation/legality input instead of a generic blob;
- migrations and analytics have explicit version/provenance boundaries.

Costs:
- content schema/compiler/editor validation must understand typed capabilities;
- new authoritative behavior categories require explicit schema/domain evolution.

The cost is justified because arbitrary item-state extensibility would make durable conservation and compatibility materially harder to prove.

## 13. Risks and mitigations

1. **Scope capture into DUR-03**
   - mitigation: GAME-ITEM defines semantics/legal transitions only; DUR-03 retains identity transitions, atomicity, location, idempotency and conservation.

2. **Generic-data escape hatch reappears through scripting**
   - mitigation: authoritative fields remain typed/versioned; DUR-04 scripting may call bounded APIs but cannot invent opaque item state or direct SQL.

3. **Content revision silently changes live items**
   - mitigation: explicit compatibility class and migration-required path; no silent reinterpretation.

4. **Container graph becomes a resource-exhaustion vector**
   - mitigation: depth, direct-entry, reachable-node and serialization/load bounds are mandatory.

5. **Modifier order diverges client/server or build-to-build**
   - mitigation: one versioned server-authoritative deterministic evaluation plan and stable keys/tie-breaks.

6. **Reference approximation becomes accidental product behavior**
   - mitigation: exact target-sensitive values remain `PARITY_PENDING_EVIDENCE` until proven or explicitly declared different.

## 14. Recommendation

Adopt typed capability composition over stable namespaced item definitions, with one durable `ItemInstanceId` per concrete mutable item instance and explicit separation from authored static placements and non-item value ledgers.

Freeze legality, typed state, equipment/container semantics, deterministic modifier evaluation requirements and content-revision compatibility in GAME-ITEM-01. Preserve transaction identity transitions and anti-duplication/conservation in DUR-03.

No runtime, SQL, serializer or exact Reference formula is required to close this architecture gate.

## 15. Decision timing

### Must decide now?

**YES.** `DUR-03` cannot safely define conservation, split/merge/transform transactions or single-location invariants without a stable item semantic model. Broad item/content modeling also needs the typed state/revision contract.

### Concrete downstream work blocked

- `DUR-03 — Item Transaction and Anti-Duplication Contract`;
- broad authoritative item/content import under `DUR-04`;
- vertical-slice durable loot/pickup proof;
- item/economy integrity event schemas and later reconciliation cases.

### What becomes harder if changed later?

Changing definition/instance separation, capability-state shape, content revision semantics or equipment/container legality after durable item data exists would require high-risk data migration, event-schema reconciliation, anti-duplication invariant changes and potentially incompatible content/client behavior.

### Evidence that could justify supersession

- DUR-03 proof showing this semantic split cannot express a required conservation invariant;
- evidenced Reference mechanic that cannot be represented without unsafe ambiguity;
- measured content/runtime evidence showing a capability boundary imposes unacceptable cost and a typed alternative preserves the same invariants;
- security exploit/failure evidence;
- an explicit later product/profile decision requiring a different item semantic domain.

### Deliberately not decided

- DUR-03 ItemInstanceId survivor/new-ID rules for split/merge/transform;
- physical database schema/index/lock/isolation design;
- Rust types/crates/APIs;
- world-bundle/item schema wire encoding;
- scripting language/runtime;
- exact equipment slot set and Reference numeric limits/formulas without evidence;
- combat, abilities, market, houses, rewards or economy domain policy;
- production resource-limit numbers until their owning evidence/registry work.

## 16. Implementation authority

This analysis is architecture-only. It authorizes no Rust/runtime/client code, PostgreSQL DDL/migrations, content import, production deployment or live data mutation.
