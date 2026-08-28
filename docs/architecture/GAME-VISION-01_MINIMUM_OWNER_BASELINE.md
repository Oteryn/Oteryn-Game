# GAME-VISION-01 — Minimum Product Vision Owner Baseline

- Status: **OWNER_ACCEPTED MINIMUM BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: minimum product-vision closure required before downstream product-sensitive architecture proceeds
- Source type: `USER_SOURCE`
- DecisionStatus after canonical merge: **ACCEPTED — MINIMUM GATE SCOPE**
- ImplementationStatus: **NOT_STARTED**
- Does not authorize: runtime/client/server/protocol/persistence/content implementation, production rollout, exact Reference target selection, exact gameplay/economy formulas, monetization, branding, LiveOps activation or acceptance of downstream gates

## 1. Purpose

Persist the product owner's explicit acceptance of the complete recommended minimum closure package prepared in `GAME-VISION-01_MINIMUM_CLOSURE_DECISION_PACKET.md` sections 5-8.

This baseline closes the minimum product-vision ambiguity that previously blocked coherent `GAME-CHAR-01`, economy/item planning, representative content selection and milestone evidence design. It does **not** claim that every future Oteryn product decision is complete.

The seven earlier dedicated owner baselines remain binding and are incorporated by reference rather than rewritten:

- `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md`;
- `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md`;
- `GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md`;
- `GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md`.

## 2. Owner source and acceptance

### USER_SOURCE — accepted 2026-08-11

After the repository-delivered minimum-closure packet was summarized to the owner, the owner was asked whether the full recommended package should be accepted as the minimal closure of `GAME-VISION-01`.

The owner explicitly answered:

> tak

This acceptance applies to the complete recommended package in sections 5-8 of `GAME-VISION-01_MINIMUM_CLOSURE_DECISION_PACKET.md` as one coherent decision: core loop, economy/scarcity philosophy, success-measure categories and deliberate deferrals/hard downstream gates.

It does not retroactively accept material outside that packet or any downstream gate named as unresolved.

## 3. Decision timing record

### Must this decision be recorded now?

**YES.**

The accepted 2026-08-10 programme refinement requires a minimum product vision before broad gameplay/content production, and the seven earlier partial baselines had left three material gaps: core session/long-term loop, economic source/sink/scarcity intent and category-level success criteria.

### Concrete downstream work unblocked by this minimum baseline

This acceptance unblocks product-sensitive architecture work that can remain within the accepted minimum semantics, especially:

- `GAME-CHAR-01` character lifecycle/progression architecture;
- bounded `DUR-02` discovery already permitted by accepted DUR-01 + ANL-01;
- representative content/milestone analysis that consumes the accepted core-loop intent;
- later `GAME-ITEM-01` and economy-facing architecture consuming the accepted conservation/scarcity direction;
- alpha/milestone design that can now name the categories of evidence that matter.

It does not authorize implementation by itself.

### What remains expensive or unsafe to guess later?

- concrete Reference gameplay semantics without an exact named Reference baseline;
- character/item persistence assumptions that contradict later detailed gameplay gates;
- hidden Reference economy tuning;
- Evolved redesigns that bypass explicit versioned decisions;
- numeric KPI targets invented before a milestone has real evidence.

### Evidence that could justify superseding this minimum baseline

A later proposal may reopen part of this baseline only with named evidence such as:

- parity fixtures showing the Reference model cannot be represented or interpreted as defined;
- external playtests showing the accepted loop does not produce the intended product experience;
- economy telemetry showing inherited source/sink mechanics justify a later explicit Evolved change;
- progression/retention/session evidence showing an accepted product emphasis is materially wrong;
- legal/provenance constraints;
- explicit product-owner strategy change.

Implementation convenience or an isolated technical preference is not sufficient.

## 4. Accepted core interaction and session loop

### 4.1 Moment-to-moment interaction grammar

The accepted product-level low-level loop is:

```text
move / position
-> select target or world interaction
-> execute server-authoritative combat / ability / interaction intent
-> manage health, mana/resources, positioning and risk
-> obtain or lose progress/value according to the active ruleset
-> decide whether to continue, disengage, recover or change objective
```

This is a product loop, not a combat formula or client-authority contract.

For Reference, the selected named Reference revision remains authoritative for exact observable mechanics. The loop does not create an implicit Reference difference.

### 4.2 Normal session loop

A normal meaningful Oteryn session should support the following shape:

```text
choose a goal
-> prepare character / equipment / supplies / party as needed
-> travel or gain access to the activity
-> execute hunt / quest / boss / trade / exploration / social objective
-> secure committed progress and value
-> recover / restock / trade / reorganize
-> choose the next goal
```

Not every session must contain every activity or every step. The accepted product principle is that player-chosen goals plus risk/execution repeatedly produce durable progress, value, knowledge or social achievement without one mandatory universal activity type.

### 4.3 Solo/party interpretation

The previously accepted `solo viable, party rewarded` baseline remains binding:

- ordinary meaningful progression must remain possible without permanent party dependence;
- coordinated party play must provide legitimate gameplay value through efficiency, safety, synergy and/or access to harder objectives;
- not every boss, quest, event or endgame objective must be soloable;
- group rewards may never rely on duplicated eligibility or integrity failures.

Reference uses its named parity mechanics. Evolved may later change mechanics only through explicit versioned decisions.

### 4.4 PvP interpretation

The previously accepted `PvP = secondary pillar` baseline remains binding:

- PvP may materially affect risk, preparation, conflict and social goals where enabled;
- PvP is not required in every ordinary session and does not globally dominate progression/economy design;
- exact Reference PvP semantics remain parity-owned;
- Evolved PvP changes require explicit later decisions.

## 5. Accepted medium and long-term loop

Oteryn should preserve multiple durable reasons to return and progress:

```text
character capability progression
+ equipment / wealth / resource goals
+ exploration, access, quest and encounter mastery
+ social, party, guild and world relationships
+ increasingly difficult, rare or prestigious objectives
+ explicit Evolved systems only when separately accepted
```

The product should not collapse long-term progression into one scalar such as only level, only wealth or an automatic seasonal-reset loop. Persistent character/world identity remains part of the accepted player promise.

This does not freeze endgame duration, vocation roster, quest topology, achievements, crafting, seasons or content cadence.

## 6. Accepted economy and scarcity philosophy

### 6.1 Reference-rule economy first

For the first Reference product proof, the selected named Reference revision is the **mechanical source/sink rule oracle** for in-scope gameplay semantics where lawful observable evidence exists.

Reference parity concerns rules that create, transform and remove gameplay value. It does **not** require copying or fabricating Global Tibia's historical market prices, total currency supply, item quantities, liquidity or population history.

### 6.2 Accepted product-level economy principles

1. **Mechanical source/sink parity, not price parity.** Reference reproduces the selected in-scope value-creation/removal mechanics; market outcomes belong to Oteryn's own world history.
2. **Conservation before balance tuning.** Duplication, stale writers, transaction races, channel multiplication or technical defects are never accepted sources of value.
3. **No hidden macro tuning.** Intentional source, sink, reward-cadence or scarcity changes must be explicit profile/ruleset revisions.
4. **Measurable provenance.** Important item/currency sources and sinks should become classifiable by stable domain cause once their owning event/transaction contracts exist; Game Intelligence observes but does not mutate.
5. **Scarcity is semantic before numeric.** Rare/valuable rewards remain meaningfully scarce relative to the active ruleset/activity cadence; exact probabilities and quantities remain downstream game-design decisions.
6. **World age/population matter.** Price, liquidity, wealth concentration and supply are interpreted in world-age/population context rather than treated as Reference constants.
7. **Security/legal/integrity override defect compatibility.** Reference parity never requires reproducing a duplication exploit, stale-authority defect or unsafe mutation path.

### 6.3 Evolved economy boundary

Evolved retains the same conservation/provenance/audit foundations but may later change source/sink/scarcity policy through explicit, versioned and measurable economy decisions after Reference evidence exists.

The first Evolved differentiation remains reliability/UX-first and does not automatically include macroeconomic redesign.

## 7. Accepted measurable success categories

The minimum vision freezes the **categories of evidence that matter**, not their final numeric thresholds.

### 7.1 Reference correctness

Measure:

- bounded parity-matrix coverage against the selected Reference revision;
- `PARITY_CONFIRMED`, pending-evidence and declared-difference classifications;
- number/severity of undocumented divergences;
- reproducibility against the exact Reference revision under test.

### 7.2 Player interaction quality

Measure, when the relevant runtime exists:

- healthy launch/login/world-entry path;
- movement/combat/input responsiveness on supported conditions;
- reconnect/recovery success and clarity;
- session completion and abandonment reasons;
- actionable client-visible error/session/channel state.

### 7.3 Progress and value trust

Measure/prove:

- no known item/currency duplication path under accepted failure scenarios;
- committed progress/value survives required recovery cases;
- stale writers/obsolete session generations cannot overwrite newer state;
- important value sources/sinks become attributable once their owning event/transaction infrastructure exists.

### 7.4 Core-loop health

Measure whether:

- an ordinary meaningful solo session can produce legitimate progress;
- coordinated party play provides a real advantage or harder coordinated objective;
- PvP, where enabled/in scope, remains fair, authoritative and exploit-resistant without becoming mandatory for all progression;
- representative content supports preparation -> activity -> secure progress -> recovery/planning rather than accidental dead ends.

### 7.5 Economy health

When privacy-safe and transaction-safe instrumentation exists, measure at least:

- item/currency source and sink composition;
- money-supply/value growth in world-age/population context;
- concentration and liquidity where markets exist;
- suspicious value creation and duplication indicators;
- progression time-to-upgrade or equivalent scarcity outcomes when relevant systems exist.

Analytics informs human product decisions; it does not autonomously tune the economy.

### 7.6 Product and operational health

When external evaluation begins and privacy policy permits, track categories such as:

- session frequency/return behavior and bounded retention cohorts;
- severe player-visible incidents;
- support burden and common abandonment/failure causes;
- population distribution and later channel-friction metrics;
- content-production throughput once the headless toolchain is real.

Numeric targets remain milestone-owned.

## 8. Accepted deliberate deferrals and hard gates

The owner accepts the following decision timing policy.

| Topic | State after this baseline | Hard requirement before |
|---|---|---|
| Exact Global Tibia patch/date/behavior baseline for first Reference revision | `DEFERRED WITH HARD GATE` | Any broad Reference mechanics/content implementation or final parity fixtures that require concrete target semantics, and the first external Reference evaluation/release contract. |
| Exact Reference revision naming scheme | `DEFERRED` | Release/version tooling depends on it. |
| Complete numbered design-pillars/anti-pillars catalogue | `DEFERRED FORMALIZATION` | Revisit if the accepted player-promise/product filters cease to reject scope coherently. |
| Exact death/progression/PvP/party formulas | `OWNED DOWNSTREAM` | Dedicated character/combat/PvP/party/ruleset gates before implementation. |
| Exact economy rates, prices, drops, fees, sink values and scarcity thresholds | `OWNED DOWNSTREAM` | `GAME-ITEM-01`, economy/content gates and relevant DUR contracts before implementation. |
| Exact numeric product/KPI thresholds | `MILESTONE-OWNED` | Before a named external alpha/release milestone may be declared successful. |
| Exact first Evolved feature inventory | `DEFERRED` | Before the first Evolved implementation milestone; strategy is already reliability/UX-first. |
| Branding/public marketing wording | `DEFERRED` | Before public launch/marketing claims. |
| Monetization/Premium/VIP economics | `DEFERRED` | Dedicated `PROD-ENTITLEMENTS-01` / business decision before activation. |
| LiveOps cadence and automatic economy control | `DEFERRED` | Dedicated LiveOps/economy decision; automatic gameplay/economy mutation is not implied. |

### 8.1 Design-filter boundary

The decision does **not** defer having a product/design filter. The accepted player-promise baseline and the PvP/solo-party/Evolved baselines already provide binding filters.

Only exhaustive numbered pillars/anti-pillars formalization is deferred unless later proposals demonstrate that the existing filters are insufficient.

### 8.2 Exact Reference baseline is fail-closed

Deferring the exact first Global baseline is **not permission to guess Reference semantics**.

`GAME-CHAR-01` may proceed only where its architecture can remain baseline-neutral. If a character, progression, death, party, PvP, economy, item, content or other downstream decision requires concrete Reference behavior, that scope becomes blocked until the exact named Reference baseline is accepted.

Broad Reference gameplay/content implementation and final parity fixtures that depend on concrete rules remain blocked until that baseline exists.

## 9. GAME-VISION status after this decision

After this baseline is canonical on `main`:

```text
GAME-VISION-01
DecisionStatus      = ACCEPTED (minimum product-vision gate scope)
DeliveryStatus      = tracked by current task lifecycle
ImplementationStatus = NOT_STARTED
Runtime authority   = NONE
```

The minimum gate is therefore closed semantically. Explicitly deferred and downstream subjects remain open under their named gates and do not become implicitly accepted.

## 10. Downstream programme consequence

The accepted programme interpretation becomes:

```text
GAME-VISION-01 minimum
-> ACCEPTED

next product-sensitive architecture
-> GAME-CHAR-01

parallel architecture/discovery where already permitted
-> GAME-CHANNEL-01
-> bounded DUR-02 discovery from accepted DUR-01 + ANL-01

before broad concrete Reference semantics
-> select exact named first Reference baseline
```

`GAME-ITEM-01`, final character-bearing `DUR-02`, `DUR-03`, `DUR-04`, `SIM-DETERMINISM-01`, gameplay implementation and external alpha remain separately gated.

## 11. Acceptance boundary

This document is binding for the **minimum `GAME-VISION-01` product-vision gate**:

- the accepted core interaction/session/long-term loop;
- Reference-rule-first economy/scarcity philosophy;
- category-level product success evidence;
- accepted deferral/hard-gate policy;
- incorporation of the seven earlier partial owner baselines as the current minimum product direction.

It does not authorize runtime, choose the exact first Global baseline, freeze detailed formulas or close downstream gameplay/product/operations gates.
