# VSL-COMBAT-01 — Minimal Combat, Creature Death, Loot and Pickup Contract Candidate

- Date: 2026-08-16
- Gate: `VSL-COMBAT-01`
- DecisionStatus: `ACCEPTED` (Stage-C owner acceptance; reaffirmed by live #506)
- DeliveryStatus: `IMPLEMENTATION_ADMISSION_OPEN` (A-F dependencies not yet complete)
- ImplementationStatus: `NOT_STARTED` (no playable Combat claim)
- Scope: first real-boundary combat/death/loot/pickup vertical slice only
- Task authority: #162 allocation for this two-path admission only; no runtime, client, server, protocol, Content, DDL, Platform or production write authority
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

The original 2026-08-16 `CANDIDATE` / `IN_REVIEW` labels are historical metadata,
now superseded by Stage-C owner acceptance recorded by #506. The acceptance and
authority model below are binding. This status correction and delivery admission
do not reopen that architecture or claim executable implementation.

## 1. Problem

The first native combat slice must prove the real authority chain from client/AI intent through combat result, creature death, durable loot creation, XP settlement and retry-safe pickup. The accepted domain contracts already own most semantics, but the slice still needs one explicit integration contract so implementation does not invent:

- a second combat engine outside GAME-ABILITY;
- death or loot identity from transient callbacks;
- duplicated loot/XP after crash/retry;
- a generic distributed transaction across death, loot and progression;
- guessed Reference formulas or drop rates;
- GAME-AI value authority;
- client-authored damage/death/loot truth.

## 2. Accepted constraints

This candidate consumes without replacing:

- FND-02 CommandRef/order/idempotency, connection-generation and client reconciliation;
- FND-03 current runtime owner/order/fencing/async completion semantics;
- FND-04 GameSession/CharacterLease/admission/recovery authority;
- SIM deterministic numeric/RNG/order/revision/replay semantics;
- GAME-ABILITY owner-accepted targeting/legality/cast/effect/damage/heal/reaction architecture;
- GAME-AI typed action intent and no-value-authority boundary;
- GAME-INTERACTION stable child occurrence / pending-reconciliation semantics;
- GAME-ITEM item definition/instance/container legality;
- DUR-03 value creation/location/transaction/idempotency/ground↔durable handoff;
- GAME-CHAR durable Character progression/death consequence ownership;
- GAME-CHANNEL source multiplicity/eligibility boundaries;
- DUR-04 immutable content/loot/creature/ability revision binding;
- ALPHA-CLIENT non-authoritative projection/presentation;
- QA-E2E real-boundary evidence requirements.

## 3. First-slice product boundary

The minimum structural proof is intentionally narrow:

```text
one admitted player
+ one server-authoritative creature in one Channel/Instance scope
+ one accepted attack/ability intent
-> deterministic GAME-ABILITY damage outcome
-> creature death when applicable
-> stable corpse/loot source occurrence
-> durable loot materialization
-> single-principal XP settlement
-> retry-safe item pickup into Character inventory
-> client observes committed outcomes through FND-02 reconciliation
```

This proves authority, recovery and value safety. It is not a claim of full Reference combat, full playable alpha combat or content breadth.

## 4. Combat authority

There is exactly one authoritative ability/effect mutation pipeline: accepted `GAME-ABILITY-01`.

A client or GAME-AI source may propose a typed action/ability intent. It must not directly:

- write HP/mana/effects;
- decide target/range/line-of-sight legality;
- consume cooldown/charge/condition authority outside GAME-ABILITY;
- declare damage/heal result;
- declare death;
- mint XP/items/currency;
- reroll or bypass a downstream rejection.

The current FND-03 scope owner accepts the normalized source occurrence and routes it through the accepted GAME-ABILITY contract.

## 5. Combat occurrence and revision binding

VSL-COMBAT introduces no competing global CombatId.

The root authoritative combat occurrence remains the accepted source/ability occurrence identity and exact semantic revision set from GAME-ABILITY/SIM, including as applicable:

- source CommandRef / AI occurrence / timer or interaction source;
- attacker and target semantic identity/local generation;
- exact ability/effect definition revision;
- ruleset/formula/content/world-policy revision;
- SIM determinism profile revision;
- exact runtime scope and current ownership generation for commit eligibility.

Retry/replay/recovery cannot reinterpret one logical combat occurrence under incompatible newer formulas/content merely because activation advanced.

## 6. Creature lifecycle owner for the slice

For an ordinary creature whose lifecycle is local to the current public Channel/Instance, the **current ChannelRuntime/InstanceRuntime remains the authoritative creature lifecycle owner**.

VSL-COMBAT names a logical creature-combat lifecycle role inside that current owner. This is not a new process/service/global authority.

The current runtime owner owns:

- creature live/dead local state;
- local HP/effect observation as committed by GAME-ABILITY;
- creature removal/despawn/corpse runtime projection after death;
- stable linkage from the lethal committed combat outcome to one creature death occurrence.

Player Character durable death/protection consequences remain GAME-CHAR/profile owned and are **not** required for the first creature-death slice.

## 7. Death occurrence semantics

Creature death is a deterministic **post-commit descendant occurrence** of the first accepted authoritative state transition that makes the creature terminally dead under the active creature/ruleset policy.

A semantic death reference is equivalent to:

```text
CreatureDeathOccurrenceRef = (
  CreatureSemanticIdentity,
  CreatureLocalGeneration,
  LethalCommittedEffectOccurrenceRef,
  BoundSemanticRevisionContext
)
```

The exact compact representation is deferred.

Required properties:

- one creature lifecycle generation produces at most one logical death occurrence;
- replay/duplicate delivery of the lethal effect cannot create a second death;
- a stale old-generation creature handle cannot kill/reward a recycled actor;
- death identity survives retry/recovery enough to reconcile descendant loot/reward work;
- NodeId, pointer address, worker completion order and wall time are not death identity.

If a creature is removed without a semantic death (administrative despawn, scope retirement, incompatible recovery policy), the implementation must not manufacture a death/loot source occurrence.

## 8. Death commit boundary

The creature lifecycle owner consumes the committed GAME-ABILITY result as a normalized owner input/outcome and, if the active profile says the creature is terminally dead:

1. verifies current actor/local generation and relevant revisions;
2. creates/recognizes the one stable death occurrence;
3. marks the creature non-actionable/dead exactly once in local authoritative state;
4. stops future ordinary creature action generation;
5. creates bounded descendant workflow proposals for corpse/loot and XP/reward as declared below;
6. emits authoritative observation/evidence.

Death commit does **not** require loot or XP durability to complete synchronously while holding the owner lane.

## 9. Corpse and loot workflow

### 9.1 Runtime corpse projection

A corpse may exist as an immediate current-runtime world/container projection owned by the current FND-03 scope. That projection is not a second durable item/value store.

Corpse semantic identity is derived from the death occurrence and exact corpse/content definition revision. A corpse runtime slot/pointer is not durable identity.

### 9.2 Loot plan

Loot eligibility/selection is a deterministic bounded descendant of the death occurrence and exact loot/content/ruleset/SIM revisions.

A logical loot decision identity is equivalent to:

```text
LootDecisionRef = (
  CreatureDeathOccurrenceRef,
  LootTableDefinitionRef,
  LootEntryOrPurposeKey,
  DeterministicDrawOrdinal
)
```

GAME-ABILITY and GAME-AI do not mint value. SIM owns deterministic random-decision semantics. GAME-CHANNEL multiplicity/eligibility must already classify the value-producing source where applicable.

### 9.3 Durable materialization

A loot candidate becomes acknowledged durable item/value only through a DUR-03-conforming materialization transaction/workflow with stable operation/cause identity tied to the same death occurrence.

Required rules:

- retry of the same death reconciles the same semantic loot/materialization occurrence;
- the same death cannot mint the same logical loot source twice;
- newly created ItemInstanceIds follow DUR-03 transaction-scoped lifecycle/non-reuse rules;
- failed/ambiguous durable commit never authorizes a fresh unrelated mint attempt for the same semantic loot source;
- runtime projection becomes interactable as acknowledged durable loot only when DUR-03/current-owner reconciliation permits it;
- stale runtime completion cannot duplicate or resurrect value.

## 10. Loot availability state

The slice distinguishes local death from durable loot readiness.

A corpse/loot workflow must expose semantic state equivalent to:

```text
DEATH_COMMITTED
LOOT_SETTLEMENT_PENDING
LOOT_READY
LOOT_SETTLEMENT_REJECTED_OR_RECONCILIATION_REQUIRED
```

Exact names/representation are implementation details.

A player must never interact with/move item value that is still only a non-authoritative candidate. If persistence outcome is ambiguous, the same loot settlement is reconciled; no duplicate mint is permitted.

This architecture intentionally allows the runtime writer lane to continue while durable loot is pending.

## 11. XP / progression reward workflow

The first slice proves only a **single eligible principal** reward path. Party/shared-XP/multi-contributor attribution is deliberately deferred.

One stable combat reward occurrence derives from the same creature death:

```text
CombatProgressionRewardRef = (
  CreatureDeathOccurrenceRef,
  EligibleCharacterId,
  RewardDefinitionRevision,
  SemanticRevisionContext
)
```

VSL-COMBAT owns the slice integration/attribution rule; **GAME-CHAR remains authoritative owner of persistent Character XP/progression mutation** and DUR-02 owns physical persistence/idempotency mechanics.

Required rules:

- one death produces at most one logical XP reward occurrence for the single eligible Character in this slice;
- retry/recovery reconciles the same occurrence;
- the Character progression owner validates current accepted Character/ruleset authority before commit;
- loot and XP are separate named descendant workflows; no generic cross-domain atomicity is invented;
- failure/pending of loot does not imply XP rollback, and vice versa;
- the final result/evidence must expose which descendant workflow is pending/committed/rejected without fabricating all-or-nothing semantics.

Exact XP value/formula is not frozen here.

## 12. Retry-safe pickup

Picking up acknowledged durable loot from ground/corpse into Character inventory uses:

```text
client CommandRef
-> GAME-INTERACTION child occurrence
-> GAME-ITEM legality / target resolution
-> DUR-03 runtime PREPARE/reservation
-> durable transaction COMMIT/ABORT/ambiguity
-> current runtime completion/reconciliation
-> FND-02 result/state projection
```

Rules:

- duplicate CommandRef/interaction child never transfers the same item twice;
- while durable outcome is `PENDING`, a blind fresh same-intent operation is forbidden;
- stale ownership/connection/session evidence cannot transfer value;
- ambiguous commit reconciles the same DUR-03 transaction/operation identity;
- runtime ground/corpse projection and durable ItemLocationRef must converge to one semantic location;
- client inventory/corpse UI is observational only.

## 13. Test-only structural combat fixture profile

Exact first-Reference damage, healing, armor, resistance, attack cadence, XP, loot probability/quantity and death arithmetic are not all proven.

To prove the native authority pipeline before target evidence is complete, Tier 1/Tier 2 VSL tests MAY use an explicit versioned:

```text
VSL_COMBAT_FIXTURE_PROFILE
```

Properties:

- test/evidence only;
- deterministic;
- bounded;
- exact fixture values recorded in the test manifest/content revision;
- not selectable in ordinary product release configuration;
- not Reference behavior;
- not Evolved product policy unless separately owner-accepted;
- cannot contribute to `PARITY_CONFIRMED`.

This profile may define simple fixed damage/HP/XP and deterministic loot outcomes sufficient to exercise death/loot/pickup flows. It must still obey SIM numeric/RNG semantics, DUR-03 conservation/idempotency and all authority boundaries.

Reference implementation of an exercised mechanic remains blocked until target evidence + provenance/legal + exact implementation/fixture evidence satisfy the Reference manifest.

## 14. Minimal ability/content requirements

The first structural slice needs only bounded content sufficient to exercise the pipeline, such as:

- one player-usable ability/attack definition;
- one creature template with finite HP and one simple AI action/idle behavior as needed;
- one loot table/source definition;
- at least one item definition that can be materially instantiated;
- one XP/reward fixture definition;
- exact revisions/provenance compatible with the VSL content bundle.

This does not authorize broad spell/monster/item import.

## 15. Creature AI boundary

GAME-AI may propose a typed offensive action intent against the player under current target/legality facts.

The action then uses the same GAME-ABILITY pipeline as a player action. AI does not directly apply damage, declare player death, consume item value or bypass cooldown/legality.

For the first combat slice, sophisticated threat/path/spawn behavior is not required; a bounded deterministic fixture behavior is sufficient if it exercises the real owner/protocol path and is not reported as Reference AI parity.

## 16. Player damage/death scope

The first required terminal proof is creature death + durable loot + XP + pickup.

The slice MAY demonstrate player receiving damage and authoritative HP observation, but it does not need to freeze/implement full durable player death consequences before the creature-death slice proves the core pipeline.

If player death is exercised, exact persistent Character death/protection consequences must consume GAME-CHAR/profile semantics and remain Reference-evidence gated. VSL-COMBAT must not invent universal blessing/PvP/death-loss rules.

## 17. Corpse/loot lifetime and cleanup

Exact corpse lifetime, owner-only loot windows, decay timing and cleanup policy are deliberately not frozen here unless required by the first fixture scenario.

Any exercised VSL value must be an explicit fixture policy with no Reference claim.

Cleanup/recovery must never duplicate durable loot or retire live acknowledged item value without an accepted DUR-03/domain policy.

## 18. Failure and recovery semantics

| Condition | Required result |
|---|---|
| duplicate attack CommandRef | no second GAME-ABILITY occurrence/commit |
| stale connection/runtime generation | no combat mutation |
| stale actor local generation | result cannot target recycled creature |
| incompatible ability/content/SIM revision | fail/reconcile; no newest-revision reinterpretation |
| lethal effect replay | same death occurrence, never second death |
| crash after death before loot commit | recover/reconcile same death + same loot occurrence; no duplicate mint |
| durable loot commit succeeds but runtime completion lost | current owner reconstructs/reconciles committed item location; no second mint |
| loot persistence ambiguous | `PENDING`; same transaction/occurrence reconciled |
| XP commit response lost | same Character reward occurrence/idempotency reconciled |
| duplicate pickup | one DUR-03 transfer at most |
| stale pickup completion | cannot mutate new runtime owner; durable outcome reconciled |
| missing resource limit | affected executable feature fails acceptance/activation, not unlimited |
| Reference rule UNKNOWN/PENDING | no Reference claim; use fixture profile only for structural proof |

## 19. Resource-limit dimensions

Before executable acceptance, the applicable registries/profiles must define finite ceilings + failure/boundary tests for at least:

1. active combat/ability occurrences per actor/scope where not already covered by GAME-ABILITY;
2. combat descendant/reaction depth/work inherited/extended from GAME-ABILITY;
3. active creature death/loot-settlement workflows per scope;
4. loot entries/candidates/items and encoded plan bytes per death;
5. deterministic loot RNG draws/work per death;
6. corpse runtime projections per scope and items per corpse/container;
7. pending DUR-03 loot materialization operations;
8. pending pickup reservations/transactions;
9. XP/reward descendant operations per death;
10. eligible reward principals/contribution set when later expanded beyond the single-principal slice;
11. combat/result/state projection bytes/counts under FND-02;
12. diagnostic/replay evidence volume.

This contract chooses no numeric values.

## 20. Minimum first-slice scenarios

A terminal technical VSL implementation must prove at least:

1. admitted native client sends a semantic attack/ability command through production protocol;
2. server validates/commits GAME-ABILITY damage and client observes authoritative result;
3. duplicate command cannot apply damage twice;
4. AI-originated typed action uses the same GAME-ABILITY authority path;
5. lethal result creates exactly one creature death occurrence;
6. death creates one deterministic fixture loot plan/source occurrence;
7. crash/retry at pre/post durable loot commit cannot duplicate item value;
8. committed loot becomes one authoritative ground/corpse ItemLocation projection;
9. one single-principal XP reward commits at most once through Character authority;
10. player pickup uses GAME-INTERACTION + DUR-03 and cannot duplicate the item under retry/lost response;
11. client inventory/corpse view reconciles from authoritative result/state;
12. stale connection/runtime/actor generation cannot mutate combat/value;
13. shuffled backing collection order preserves normalized deterministic result;
14. exact server/build/protocol/World Bundle/SIM/fixture revisions are retained in evidence;
15. no result is labeled Reference parity merely because the structural fixture passes.

Tier 1 must cross Platform/Gateway/protocol/server/persistence boundaries applicable to the scenario. Tier 2 must exercise native semantic input and client projection. A direct mutation harness is component evidence only.

## 21. Explicit non-decisions

`DECISIONS_NOT_TAKEN`:

- exact Global damage/heal/armor/resistance/critical formulas;
- exact Global attack cadence/cooldown values;
- exact creature HP/XP/drop rates/loot distributions;
- full conditions/buffs/debuff catalogue;
- PvP/skull/blessing/death-loss behavior;
- party/shared XP/multi-contributor loot attribution;
- boss/raid/event reward semantics;
- corpse ownership/decay product rules;
- concrete combat/RNG libraries;
- physical Rust types/module layout;
- concrete protocol message IDs/fields;
- PostgreSQL schema/isolation implementation;
- numeric resource limits;
- production balance/content.

## 22. Decision timing

- **Must decide now?** `YES` for death identity, corpse/loot materialization boundary, XP owner integration, pickup retry workflow and fixture-vs-Reference proof separation.
- **Concrete downstream blocked:** minimal combat vertical slice, anti-dup loot/pickup implementation, Character XP integration, QA-E2E combat proof.
- **Harder later:** death callback identity or transient loot generation could become duplicate-value authority; XP/loot could be incorrectly coupled as one transaction; fixture values could accidentally become de facto Reference policy.
- **Superseding evidence:** representative combat requires different owner boundaries; crash/replay evidence shows the descendant workflow model cannot preserve value/ordering; later accepted reward/party/death architecture introduces stronger compatible semantics.
- **Deliberately not decided:** all exact formulas/content/product rules/technology/numeric values above.

## 23. Recommendation

`RECOMMENDATION: ACCEPT` this minimum structural combat/death/loot/pickup architecture for the first vertical slice.

This recommendation concerns architecture acceptance only; Stage-C owner acceptance is now recorded by #506. Runtime/persistence/client/content implementation and Reference parity remain separately gated.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE FROM THIS CONTRACT; LIVE #162 CHILD ALLOCATIONS ONLY`

## 24. Generic native vertical admission (2026-09-27)

This admission makes the accepted Stage-C contract executable as a bounded,
generic Game delivery chain. It does not add a Rat feature or make this document
runtime proof. The source of product and repository lifecycle truth remains the
live owner contracts, exact allocations, PR/check/Merge Queue state and protected
main. Stage-C authority in this contract is binding; an implementation child may
consume it but may not weaken or replace it.

### 24.1 Required chain and independent transactions

```text
admitted native player intent
-> accepted GAME-ABILITY legal commit
-> exactly one creature-death generation
-> deterministic, definition/revision/source/occurrence/state-driven loot output
-> separate one-item DUR-03 MINT to typed Ground + corpse association
-> eligible pickup intent and GAME-ITEM legality
-> separate one-item DUR-03 TRANSFER from Ground to native direct-root CharacterInventory
-> authoritative native-client observation of result and inventory/location
-> restart/retry readback proving the same single item and terminal state
```

Death is one post-commit descendant per creature local generation. Loot planning
is deterministic over the bound definitions and semantic revisions. MINT creates
one fresh ItemInstance in typed Ground custody with corpse association/provenance;
pickup later transfers that same item identity/type/quantity to a legal
CharacterInventory destination. The two operations have distinct transaction,
candidate and atomic boundaries, consistent with DUR-03 §§39.1-39.2. A corpse is
a runtime projection, never a second durable location. Unsupported shapes reject;
this admission adds no event/field IDs, source/root grammar, security purpose,
schema/API layout or production maximum.

XP is a separate descendant through integrated R7 P03 Character APIs. Reuse the
single accepted Character progression calculator and owning commit/reconciliation
semantics. Combat must not add an XP formula/engine, write XP directly, or make
XP and loot one distributed transaction. Failure or pending state in one
descendant does not roll back or fabricate the other.

Before D/E admission and before any generic Combat XP settlement, the
Character-owned progression initialization/readiness prerequisite must be proven
through its authorized owning route and bound to a fresh, separately allocated
Character owner revision/policy binding. Without that proof, generic Combat XP
settlement remains gated. R7 P03 applies awards to initialized progression; this
admission designs no initializer and selects no values, SQL, or runtime path.

### 24.2 Genericity, provenance and native entry scope

The same generic code must pass two deterministic definition/revision fixtures.
The existing Rat/cheese/Gold Coin may be admitted as fixtures, never as
identity-specific engine branches. A second distinct deterministic fixture is
required to prove genericity; it does not imply a second production roster or
Reference acceptance. Logic dispatches on accepted typed definitions, revision
bindings, authorized source/occurrence and current state, never names, fixture
IDs, or fixture bytes.

Canary `47dfd51f` and CrystalServer `ff7ede593` are owner-selected, pinned,
read-only behavioral/ordering references only. They may guide later evidence
comparisons and exact candidate presentation; they are not Oteryn numeric or
product authority, implementation-copy permission, or proof of Reference parity.
The exact product values remain subject to independent target evidence and
explicit owner acceptance.

Qualification reuses the protected native entry room and existing Movement /
Server Seam environment: `apps/game-server/src/content/project/native_entry_room.json`,
`native_entry.rs`, `content_native_entry_room.rs`,
`tools/qualification/native_entry_room/run.sh` and its workflow, the boot
activation pin, and integrated PR #961 Movement step. Preserve the accepted
start/east step-and-return proof cells. Any room/source revision belongs to the
Content/Seam owner and needs its own allocation. Do not create a second room or
harness. The room is preproduction qualification, not deployment or production
proof.

### 24.3 Dependency map and whole-unit admission

The following are serialized gates. Each child needs its own exact allocation,
bounded paths, lease and current-head qualification. Character-owned progression
initialization/readiness is a separate prerequisite with a fresh owning revision/
policy allocation; it must precede D/E admission and any Combat XP settlement.
Dependency-safe PRs are mechanics only: A stays open until F passes on protected
state.

| Child | Owner and required allocation / serialization | Exit evidence |
|---|---|---|
| A — this admission | Current #162 allocation; architecture author owns only this contract and its task packet. No runtime/schema/registry edits. | Accepted chain, dependency map, exact whole-unit DoD and owner-input boundaries are recorded. |
| B — native owning bindings | GAME-ITEM, Content, ANL/schema/profile, and resource owners each hold their required exact allocation. Serialize shared event registries, item/profile schema, `RESOURCE_LIMITS_REGISTRY.json`, migration numbering and native identity/custody decisions. Preserve item-specific P90D. | Accepted native typed binding and legal CharacterInventory placement; registered schema/profile/security purpose/resource limits with measured max/max+1/overflow and retained/encoded evidence. Unknowns remain closed. |
| C — DUR-03 physical MINT and TRANSFER | DUR-03 owner, separately leased from B/D and any shared durability writer. Serialize `durability/mod.rs`, physical DUR registries, audit/outbox schema and migration numbers. MINT and TRANSFER remain distinct operations. | Native audit/candidate binding, PostgreSQL transaction proof, exact receipts, audit/outbox contribution, ambiguous-commit recovery and separate MINT/TRANSFER restart/retry evidence under §§39.1-39.2. |
| D — generic death/loot/reward orchestration | Combat owner after B/C bindings, R7 P03, resource acceptance and authorized Character progression-readiness proof under a fresh separate owner revision/policy allocation. Exclusive lease over `foundation/mod.rs` and `runtime_actor_carrier.rs`; serialize shared edits. Exclude PR #1004 and R7 P03 paths absent a new shared lease. | Generic definition/revision/source/occurrence/state-driven path; two deterministic fixtures; death/loot workflow and XP through R7 P03 with no second XP engine or cross-domain transaction. |
| E — existing-room Seam/protocol/native-client composition | Server Seam, protocol and client composition owners each receive exact leases after the separate Character readiness gate. Serialize protocol schemas/IDs, server/client roots, and room/source changes with Content/Seam owner. Reuse existing room and #961 Movement integration. | Production protocol input/output and independent wire fixtures; retained start/east step-and-return; authoritative native-client observation of combat, item and inventory state. |
| F — terminal qualification | One exact-candidate integration/qualification owner after B-E; no concurrent candidate writes. | Real native desktop + Game PostgreSQL; restart/retry/anti-dup; proof admitted native Character progression was initialized through its authorized Character-owned route and bound to the fresh owner revision/policy; exact-head review/CI/MQ, merge-group `game-gate` and protected-main readback. Only whole-unit success closes A or qualifies playable Combat. |

### 24.4 Whole-unit terminal acceptance matrix

| Area | Required terminal evidence |
|---|---|
| End-to-end semantics | One native player intent commits through GAME-ABILITY legality; one death generation selects deterministic legal loot; separate one-item MINT and TRANSFER reach direct-root CharacterInventory; native client observes authority; XP settles once through R7 P03 after Character-owned initialization/readiness is proven. |
| Genericity and binding | Same code passes two deterministic fixtures with different definition/revision identity. Retain exact candidate bytes and semantic envelope; verify definition, content, ruleset, SIM, source occurrence and state bindings. No fixture-name branches or synthetic identity/bytes promoted as production. |
| Concurrency and replay | Exactly one concurrent pickup wins; duplicate command/death/source replay creates no second death, mint, transfer, XP or location. Source occurrence is non-reusable. One authoritative location remains and no corpse ghost survives reconciliation. |
| Ambiguity and recovery | Lost ACK, ambiguous commit, crash/restart before and after each MINT and TRANSFER, duplicate retry, committed-but-lost runtime completion and stale result reconcile the same operation and identity without fresh mint or partial custody. |
| Authority fences | Reject independently stale session, Character lease, runtime/scope, actor-local generation and content/revision evidence before mutation; independently test the applicable negative cases and one-winner race. |
| Resources and owner lane | Evidence for every accepted native resource/profile limit: max, max+1 and checked overflow before mutation/allocation; audit/outbox backpressure fails closed; bounded retry/reconciliation; no synchronous database wait in the owner lane. |
| Protocol and client | Independent exact-byte fixtures plus malformed, gap, duplicate and resync cases; server and native client composition are exercised, not only a shared headless codec or direct mutation harness. |
| Physical terminal proof | Real Game PostgreSQL and native desktop through existing room; restart/readback; admitted Character initialization via authorized owning route; existing start/east step-and-return retained. Preproduction only. |
| Repository terminal proof | Exact frozen candidate, complete owned-path review, required independent review, exact-head CI, native Merge Queue aggregate `game-gate`, and protected-main readback. Dependency PRs alone never close this row or A. |

No partial child, offline prototype, narrow green suite, schema/profile check,
PR merge without complete F evidence, or isolated death/corpse proof may claim
playable Combat.

### 24.5 Accepted facts, technical proposals and product inputs

**ACCEPTED** — Stage-C VSL-COMBAT-01 authority and death identity; GAME-ABILITY
is the only damage/effect authority; current Channel/Instance runtime owns
creature lifecycle and corpse projection; GAME-INTERACTION/GAME-ITEM provide
pickup planning and legality; DUR-03 owns durable item identity, custody,
conservation, idempotency and recovery; item MINT and TRANSFER are separate
one-item transactions under §§39.1-39.2; GAME-CHAR/DUR-02 own persistent XP and
R7 P03 is the integrated XP commit path; ANL-01 envelope/payload semantics and
existing item P90D remain binding. Character-owned progression initialization/
readiness through its authorized route, with a fresh separately allocated owner
revision/policy binding, is required before D/E and Combat XP settlement. Existing
room and Movement proof environment are reused. Canary/Crystal remain read-only.

**TECHNICAL decisions future owners may propose** — native typed bindings and
profiles, measured hard resource limits, implementation/module/API shapes,
transaction internals, physical schema/codec/receipt/audit/outbox/recovery,
protocol encoding, and composition details. Each requires exact owner authority,
serialization/lease checks, accepted contracts, required evidence and independent
review. This admission selects no SQL layout, protocol field/ID, event family or
field IDs, InventoryRootId, root/source grammar, security purpose, or new
Interaction/DUR/Character resource.

**PRODUCT inputs still unknown** — entry-room damage/lethality and HP math; loot
probability and quantity; XP amount/formula; client presentation assets; inventory
capacity and stack legality; and spawn/cell arrangement that preserves the
Movement proof. Do not invent values. Present exact candidate values with their
provenance and measurement to the owning product decision-maker. Owner selection
of Canary/Crystal as comparison references is not admission of their values.

The original evidence history and explicit non-decisions remain intact. Current
unknowns stay `UNKNOWN`; neither documentation nor fixtures assert Reference
parity, production readiness or deployment.
