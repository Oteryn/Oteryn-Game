# Oteryn First Reference NPC Dialogue / Service Boundary

- Date: 2026-09-09
- Repository: `Oteryn/Oteryn-Game`
- Architecture escalation: #499
- Parent programme: #486
- Parent control plane: #162
- Architecture coordination: #220
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Status: `ARCHITECTURE DECISION CANDIDATE`
- DecisionStatus: `PROPOSED_FOR_PROTECTED_REVIEW`
- Runtime implementation authority: `NONE`
- Production authority: `NONE`
- Merge authority: `REPOSITORY_CONTROL_PLANE_ONLY`

## 1. Decision purpose

The first Reference Playable service loop needs one NPC conversation and one representative trader flow without turning AI, generic Interaction, Content scripts or the client into a second business/value authority.

Current accepted architecture already separates the relevant responsibilities:

- GAME-AI may own/reuse bounded NPC-local idle, movement and perception behavior, but explicitly does not own dialogue, trade, quest, bank, economy or durable business state;
- GAME-INTERACTION owns bounded child occurrence/proposal/reconciliation semantics and does not become the foreign business owner merely because an interaction starts at an NPC;
- GAME-ITEM owns item/equipment/container legality;
- DUR-03 owns durable item/value conservation, transaction identity, idempotency and runtime-to-durable commit/reconciliation;
- immutable content/ruleset revisions own authored definitions and Reference-backed values;
- the current `ChannelRuntime` / `InstanceRuntime` remains the physical authoritative gameplay owner under FND-03.

The unresolved gap is the owner of **NPC conversation/service lifecycle semantics** between those domains.

## 2. Official Reference evidence used for the first slice

Current official Tibia manuals establish the observable product shape used by this decision:

- talking to NPCs uses a dedicated NPC conversation window/channel;
- an NPC may expose specialized roles such as trader or banker;
- asking an NPC for `trade` opens a Buy/Sell interface;
- selecting item/quantity is still intent until the deal is confirmed;
- a confirmed trade can fail for insufficient money, capacity or space;
- the client UI displays the result but does not establish item/value authority.

Official source locators:

- `https://www.tibia.com/gameguides/?section=world&subtopic=manual`
- `https://www.tibia.com/gameguides/?section=controls_trading&subtopic=manual`
- `https://www.tibia.com/gameguides/?section=controls&subtopic=manual`

These current pages are product-shape/continuity evidence, not automatic proof of every exact 2026-07-28 keyword, price, catalogue or reconnect behavior. Exact Reference-sensitive values remain under #483 evidence discipline.

## 3. Selected ownership model

**Selected candidate: Option A from #499.**

Introduce a logical **`GAME-NPC-SERVICE` domain role inside the current FND-03 runtime owner**.

This is not a new process, microservice, database owner, scheduler or control plane.

```text
public Channel scope -> current ChannelRuntime
instance scope       -> current InstanceRuntime

inside that current owner:
NPC actor-local behavior -> GAME-AI role where applicable
NPC dialogue/service     -> GAME-NPC-SERVICE role
item legality            -> GAME-ITEM
value mutation           -> DUR-03 + owning economy/value rule
quest durable state      -> later named quest/domain owner
```

The runtime owner remains the only local authoritative writer. `GAME-NPC-SERVICE` names the semantic role that owns bounded conversation/service state inside that writer.

## 4. What GAME-NPC-SERVICE owns

For the first bounded slice it owns only:

1. current NPC conversation lifecycle state;
2. conversation participant binding to one NPC and one current Character/GameSession authority context;
3. normalized bounded dialogue intent such as greet/select-topic/select-service/close;
4. service capability selection from an exact immutable NPC/service definition revision;
5. typed service invocation occurrence identity;
6. pending/accepted/rejected service-invocation reconciliation state;
7. authoritative NPC dialogue/service observations projected to the client.

It does **not** own:

- NPC movement/pathfinding/perception;
- damage/healing/ability legality;
- item location/equipment/container legality;
- currency or item conservation;
- quest durable state or reward settlement;
- bank/market/depot business policy;
- protocol IDs;
- persistence schema;
- player authentication/session issuance;
- production deployment.

## 5. Authored definition boundary

Immutable content/ruleset definitions provide the authored facts interpreted by GAME-NPC-SERVICE.

A first-slice `NpcServiceDefinition` is semantically equivalent to:

```text
NpcSemanticKey
NpcDefinitionRevision
DialogueDefinitionRevision
ServiceDefinitionRevision
ServiceCapabilityKey
optional CatalogueDefinitionRevision
optional PriceRuleRevision
PresentationDefinitionRef
```

The exact physical schema is deliberately not selected here.

Content definitions may describe dialogue text/keys, service capability, catalogue entries and Reference-backed price data. They are immutable input, not runtime mutation authority.

Scripts/components, if introduced later, remain bounded proposal producers. They cannot directly mutate inventory, currency, Character progression, quest durability or other foreign-owned state.

## 6. Conversation occurrence and authority binding

One accepted conversation is bound to enough current authority and revision context to prevent stale or cross-owner continuation.

Semantic shape:

```text
NpcConversationRef = (
  RuntimeScopeRef,
  RuntimeOwnershipGeneration,
  NpcSemanticIdentity,
  NpcLocalGeneration,
  CharacterId,
  GameSession authority context,
  ConversationOccurrenceDiscriminator,
  ExactNpcDialogueServiceRevisionContext
)
```

The exact compact Rust representation is deferred.

Required properties:

- the client does not choose an authoritative NPC identity or revision;
- stale runtime generation/NPC generation fails closed;
- a conversation for one Character/NPC cannot be replayed as another Character/NPC;
- duplicate source input cannot create two semantic service invocations;
- content activation cannot silently reinterpret an already-started invocation under incompatible dialogue/catalogue/price revisions;
- pointer address, worker/thread identity and wall-clock order are not semantic conversation identity.

## 7. Conversation lifecycle

The minimum lifecycle is equivalent to:

```text
CLOSED
-> OPEN
-> ACTIVE
-> SERVICE_PENDING (optional)
-> ACTIVE or CLOSED
```

A service invocation may complete accepted or rejected; the conversation owner consumes the normalized result and produces the next dialogue/service observation.

No generic distributed transaction is created between conversation state and a foreign business transaction.

Conversation state is runtime-local unless a later mechanic proves durable continuity is required. The first Reference slice makes **no parity claim** about preserving an NPC conversation through disconnect, server restart or scope handoff. Such transitions must close/fail/reconcile safely rather than silently fabricate continuation. Exact Global reconnect behavior remains evidence-gated.

## 8. Dialogue intent versus service invocation

Dialogue selection and business mutation are separate stages.

```text
normalized player dialogue intent
-> GAME-NPC-SERVICE resolves exact dialogue/service capability
-> zero or one typed service invocation proposal
-> owning downstream domain validates/commits/rejects
-> normalized result returns to GAME-NPC-SERVICE
```

Opening a trader interface is an observation/projection event and does not itself mutate item/value state.

## 9. First Reference trader slice

The first implementation candidate is one target-evidenced Newhaven trader flow.

Minimum observable sequence:

```text
TALK/GREET
-> select TRADE capability
-> server projects bounded exact-revision Buy/Sell catalogue
-> client displays trade widget
-> player selects one item + quantity
-> server normalizes one BUY or SELL intent
-> downstream item/value owners validate and commit/reject
-> server projects committed/rejected result
```

### 9.1 Read-only trade-widget proof

This sub-slice requires no DUR-03 mutation:

- current authority can talk to the correct NPC;
- exact service definition authorizes `trade`;
- exact catalogue revision is projected;
- stale/unknown NPC/service/catalogue revision fails closed;
- client cannot invent catalogue entries or prices.

### 9.2 Mutating trade proof

Actual BUY/SELL is a later composition with the accepted value owners:

- GAME-NPC-SERVICE supplies the normalized service occurrence and exact catalogue/price context;
- GAME-ITEM validates item-definition/location/equipment/container constraints where applicable;
- DUR-03 owns TransactionId, conservation, idempotency, runtime/durable handoff and ambiguous-result reconciliation;
- the appropriate bank/economy owner is required if the exercised transaction consumes a bank-ledger fallback.

The smallest first trade fixture SHOULD use sufficient explicit physical-currency/value state so it does not need to exercise bank fallback unless that bank path is separately ready and target-evidenced.

## 10. Interaction boundary

GAME-INTERACTION remains a generic occurrence/proposal/reconciliation helper, not the NPC business owner.

The existing Interaction foreign-owner vocabulary must not be widened merely by string/tag convenience to make NPC service appear implemented.

A future implementation may:

- consume Interaction child-occurrence identity where an NPC service is triggered from a generic world interaction; or
- add a reviewed typed adapter if the exact child slice proves it is necessary.

Any such adapter remains proposal/reconciliation infrastructure. It does not transfer conversation/service/value ownership to Interaction.

## 11. AI boundary

NPC-local idle/movement/perception may reuse GAME-AI under its accepted limits and future integration slices.

GAME-AI does not:

- own dialogue state;
- choose a price/catalogue as runtime truth;
- mutate inventory/currency;
- complete a trade;
- mutate quest/bank/economy state.

AI may at most supply typed local actor behavior/proposals that are independently revalidated by their owning domains.

## 12. Item/value boundary

A confirmed trade is never committed inside NPC dialogue code.

Value mutation uses the accepted ownership chain:

```text
NpcServiceInvocationRef
+ exact catalogue/price rule revision
+ current Character/GameSession authority
+ exact item/value intent
-> GAME-ITEM legality where applicable
-> DUR-03 prepare / durable commit / reconcile
-> normalized business result
-> GAME-NPC-SERVICE dialogue/result projection
```

Duplicate dialogue input, duplicate client command or lost response cannot authorize an unrelated second business transaction.

## 13. Quest boundary

This decision does not solve the broader quest/dialogue runtime gap.

The first trader needs no durable quest graph.

Quest state, branching, repeatability, schedules, rewards and migration remain a later separately bounded owner decision, compatible with the existing `CONTENT-QUEST-01` refinement identified in the architecture gap register.

A future quest system may reuse the NPC conversation presentation/orchestration boundary but cannot hide durable quest state inside ephemeral NPC conversation state.

## 14. Client boundary

The native client is non-authoritative.

It may:

- request talk/greet/topic/service actions;
- display NPC dialogue;
- display service capability icons;
- display the exact server-projected trade catalogue and price;
- submit bounded buy/sell intents;
- display accepted/rejected outcomes.

It may not:

- create service availability;
- invent authoritative price/catalogue state;
- decide whether capacity/money/item constraints pass;
- commit item/currency/quest state;
- advance authoritative conversation/service revisions.

## 15. Reference evidence discipline

Every Reference-sensitive authored field retains independent evidence classification.

Current official manuals may establish product shape and current observable behavior, but they do not automatically prove the exact 2026-07-28 state.

For the first Newhaven trader:

- NPC conversation + trade-widget product shape: official continuity candidate;
- exact selected trader identity for the target journey: #483 evidence package / target continuity required;
- exact keywords: evidence-gated;
- exact catalogue entries: evidence-gated;
- exact prices: evidence-gated / controlled target observation required where no dated primary source exists;
- bank fallback: may remain excluded from the first trade fixture;
- current post-target behavior never silently overwrites dated target evidence.

Canary/Crystal/other OTS data remains `OTS_HYPOTHESIS_ONLY` and may only suggest fields, edge cases and tests.

## 16. Resource dimensions before implementation

No numeric hard maximum is selected by this decision.

Before a mutating NPC-service implementation can be accepted, the exact child plan must classify every exercised variable-size dimension and obtain accepted finite hard maxima or explicit fail-closed exclusion.

Candidate dimensions include at minimum:

- active NPC conversations per Character/GameSession;
- active conversations per NPC/runtime scope;
- dialogue choices/options processed per input;
- dialogue text/projection bytes per response;
- service capabilities per NPC definition;
- catalogue entries projected per service response;
- pending service invocations per conversation/Character;
- retained conversation/service reconciliation state;
- diagnostic/evidence volume under dialogue spam/failure.

Do not reuse unrelated Foundation/Interaction/Ability ceilings merely because their units are counts/bytes.

## 17. Minimum future implementation evidence

A future exact `GAME-NPC-SERVICE` child must prove at least:

1. current Character talks to one exact-revision NPC and receives deterministic dialogue/service projection;
2. stale runtime/NPC/content/service revision cannot continue the conversation;
3. duplicate talk/service input does not create duplicate service invocation;
4. one `trade` selection opens the exact server-projected catalogue without value mutation;
5. client-modified price/catalogue/service capability is ignored/rejected;
6. one accepted BUY/SELL intent delegates to the owning item/value transaction boundary rather than mutating value locally;
7. downstream reject returns one normalized dialogue/service result with no partial value mutation;
8. pending/ambiguous downstream business result is reconciled under the same operation, never retried as a fresh unrelated trade;
9. no AI/Interaction/Content/client code path gains direct item/currency/quest mutation authority;
10. all exercised resource max/max+1 and overflow cases fail closed;
11. exact Reference claims are limited to fields with admissible evidence;
12. Tier 1/Tier 2 final proof waits for Server Seam/native-client readiness and required Item/DUR-03 composition.

## 18. Explicit non-decisions

This decision does not select:

- full quest runtime/graph;
- bank/market/depot business architecture;
- permanent NPC scripting language or VM;
- NPC pathfinding/schedule algorithm;
- broad NPC authoring UI;
- exact Global target prices/catalogues/keywords without evidence;
- dialogue localization architecture;
- wire protocol IDs;
- PostgreSQL schema;
- numeric resource limits;
- production deployment topology;
- any Oteryn Evolved behavior.

## 19. Coordinator handoff

While this decision is unprotected:

- R8 evidence/readiness may continue read-only;
- no NPC dialogue/service runtime worker may be allocated from this document;
- #162 remains the sole allocation/integration control plane;
- #499 remains the architecture owner for this bounded decision;
- #494/#492 Content repair, WP2/WP3/WP4/WP5 and Server Seam remain independent.

After protected acceptance/readback, #162 may prepare one exact path-bounded first NPC-service implementation allocation only after fresh overlap/resource/dependency preflight.

The first implementation should preserve two sub-gates:

```text
NPC_DIALOGUE_TRADE_WIDGET_V1
-> does not require value mutation

NPC_SINGLE_TRADE_COMMIT_V1
-> requires GAME-ITEM + transaction-capable DUR-03 + exact target price/catalogue evidence
```

This keeps early client/service plumbing useful without weakening the final Reference readiness gate.
