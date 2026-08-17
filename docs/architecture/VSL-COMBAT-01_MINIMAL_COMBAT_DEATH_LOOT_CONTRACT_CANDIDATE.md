# VSL-COMBAT-01 — Minimal Combat, Creature Death, Loot and Pickup Contract Candidate

- Date: 2026-08-16
- Gate: `VSL-COMBAT-01`
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Scope: first real-boundary combat/death/loot/pickup vertical slice only
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

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

Acceptance would authorize architecture only. Runtime/persistence/client/content implementation and Reference parity remain separately gated.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`
