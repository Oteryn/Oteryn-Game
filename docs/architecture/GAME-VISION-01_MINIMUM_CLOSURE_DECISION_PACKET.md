# GAME-VISION-01 — Minimum Closure Decision Packet

- Status: **PRE-DECISION SYNTHESIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-VISION-01`
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Trusted-base reconciliation point: `blakinio/Oteryn-v2@c88f778a3d4a8d26efeb3a2ad2f328b4efca3768`
- Decision owner: product owner
- Purpose: reconcile current owner-accepted partial baselines and reduce the remaining minimum `GAME-VISION-01` scope to one explicit owner closure decision
- Does not authorize: runtime/client/server/protocol/persistence/content implementation, production rollout, exact gameplay/economy formulas, monetization, branding or acceptance of any recommendation below

## 1. Why this packet exists

`GAME-VISION-01_PREDECISION_ANALYSIS.md` and its economy addendum were produced before the owner subsequently accepted seven narrower GAME-VISION baselines. Those historical pre-decision documents remain valid evidence, but their original owner-decision lists now mix already accepted decisions with genuinely unresolved ones.

Rewriting those historical analyses would blur the decision trail. This packet instead provides a current synthesis.

The accepted 2026-08-10 programme refinement requires the minimum product vision to define, before broad gameplay/content production:

- launch profile and player promise;
- core session loop and long-term loop;
- progression/risk/death baseline;
- party/cooperation baseline;
- PvP baseline if applicable;
- economic sources/sinks and scarcity goals;
- first intentional improvements beyond Reference behavior;
- measurable success criteria.

This packet shows which are already owner-accepted, what still needs an owner choice, and what can be deliberately deferred without blocking the next safe architecture gates.

## 2. Current owner-accepted GAME-VISION baseline

The following are **PROVEN owner-accepted partial baselines** on the trusted base. This packet does not reopen them.

| Product question | Current status | Canonical owner baseline | Binding meaning |
|---|---|---|---|
| First external profile | `ACCEPTED` | `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` | First externally evaluated build is one Reference profile; Evolved follows later. |
| Reference upstream tracking | `ACCEPTED` | `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md` | Observe upstream continuously, but promote changes only into later immutable named Reference revisions. |
| Internal player promise | `ACCEPTED` | `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md` | Preserve recognizable Tibia depth/persistent-world character, modern native quality, and explicit/versioned/measurable intentional differences. |
| First Evolved strategy | `ACCEPTED` | `GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md` | First Evolved differentiation is reliability/UX-first, not broad systemic gameplay redesign. |
| PvP product importance | `ACCEPTED` | `GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md` | PvP is important and high-quality where enabled, but not the dominant organizing principle of the whole product. |
| Solo/party emphasis | `ACCEPTED` | `GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md` | Ordinary meaningful progression remains solo viable; coordinated party play has real value. |
| Reference-vs-Vision precedence and progression/risk direction | `ACCEPTED` | `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md` | Actual Reference gameplay follows its selected named parity baseline; future-facing GAME-VISION preferences primarily guide Evolved. Later Evolved progression/death redesign requires an explicit isolated gate. |

### Consequence

The project must not ask the owner again whether Reference-first, hybrid tracking, reliability/UX-first, PvP-secondary or solo-viable/party-rewarded are still open alternatives. A later superseding proposal requires new evidence and explicit owner action.

## 3. Remaining minimum closure gaps

Comparing the accepted baselines with the 2026-08-10 minimum requirements leaves three material product decisions.

| Minimum requirement | State now | Why |
|---|---|---|
| Core session loop and long-term loop | `REMAINS_OWNER_DECISION` | Candidate loop exists only in `GAME-VISION-01_PREDECISION_ANALYSIS.md`; no owner baseline accepts it. |
| Economic sources/sinks and scarcity goals | `REMAINS_OWNER_DECISION` | `GAME-VISION-01_PREDECISION_ECONOMY_SCARCITY_ADDENDUM.md` contains a recommendation, not owner acceptance. |
| Measurable success criteria | `REMAINS_OWNER_DECISION` at category level | Candidate measurement categories exist, but no owner baseline makes them the minimum product-success contract. Exact numeric thresholds can remain later milestone decisions. |

### Decision timing

**Must decide now? YES — only at product-semantics/category level.**

These three questions shape `GAME-CHAR-01`, later item/economy work, representative content selection, alpha proof design and interpretation of product evidence. Deferring them entirely would allow downstream gates to optimize toward incompatible ideas of a healthy Oteryn product.

The decision does **not** need to choose formulas, exact rates, final content breadth or numeric KPI thresholds.

## 4. Recommended remaining closure package

### RECOMMENDATION — owner decision required

Accept the following four-part package as the **minimum GAME-VISION closure contract**:

1. the core-loop contract in section 5;
2. the Reference-rule-first economy/scarcity contract in section 6;
3. the success-measure categories in section 7;
4. the explicit deferrals and hard downstream gates in section 8.

The package is intentionally coherent with all seven existing owner baselines and does not redefine Reference parity.

## 5. Recommended core session and long-term loop

### 5.1 Moment-to-moment interaction loop

Oteryn's recognizable low-level interaction grammar should remain centered on:

```text
move / position
-> select target or world interaction
-> execute server-authoritative combat / ability / interaction intent
-> manage health, mana/resources, positioning and risk
-> obtain or lose progress/value according to the active ruleset
-> decide whether to continue, disengage, recover or change objective
```

This is a product loop, not a combat formula or client-authority contract. Exact mechanics in Reference remain dictated by the selected named Reference revision.

### 5.2 Normal session loop

A normal meaningful session should support this shape:

```text
choose a goal
-> prepare character / equipment / supplies / party as needed
-> travel or gain access to the activity
-> execute hunt / quest / boss / trade / exploration / social objective
-> secure committed progress and value
-> recover / restock / trade / reorganize
-> choose the next goal
```

Not every session must contain every step or activity. The contract means the game should repeatedly convert **player-chosen goals + risk + execution** into durable progress, value, knowledge or social achievement without requiring one mandatory activity type.

### 5.3 Solo and party interpretation

The accepted `solo viable, party rewarded` direction constrains the loop:

- a normal ordinary session must be capable of producing meaningful progress solo;
- coordinated party play must produce legitimate value through efficiency, safety, synergy and/or access to harder objectives;
- not every boss, quest, event or endgame objective must be soloable;
- party rewards must not depend on duplicated eligibility or integrity failures.

Reference uses its selected parity mechanics. Evolved may later change mechanics explicitly to serve this direction.

### 5.4 PvP interpretation

PvP is a secondary pillar:

- PvP may materially affect risk, preparation, conflict and social goals where a world/ruleset enables it;
- PvP is not required in every normal session and must not dominate progression/economy design globally;
- exact Reference PvP behavior remains parity-owned;
- Evolved PvP changes require explicit later decisions.

### 5.5 Medium and long-term loop

Oteryn's medium/long horizon should preserve multiple durable reasons to return and progress:

```text
character capability progression
+ equipment / wealth / resource goals
+ exploration, access, quest and encounter mastery
+ social, party, guild and world relationships
+ increasingly difficult, rare or prestigious objectives
+ explicit Evolved systems only when separately accepted
```

The product should not collapse long-term progression into one scalar such as level, wealth or a seasonal reset. Persistent character/world identity is part of the accepted player promise.

This does not freeze an endgame duration, vocation roster, quest topology, achievement system, crafting system, seasons or content cadence.

## 6. Recommended economy and scarcity contract

### 6.1 Reference-rule economy first

For the first Reference product proof, use the selected named Reference revision as the **mechanical source/sink rule oracle** for in-scope gameplay semantics where lawful observable evidence exists.

Reference parity concerns the rules that create, transform and remove gameplay value. It does **not** require copying or fabricating Global Tibia's historical market prices, total currency supply, item quantities, liquidity or population history.

### 6.2 Product-level source/sink/scarcity principles

Accept these minimum principles:

1. **Mechanical source/sink parity, not price parity.** Reference reproduces the selected in-scope value-creation/removal mechanics; market outcomes are measured outcomes of Oteryn's own world history.
2. **Conservation before balance tuning.** Duplication, stale writers, failed transaction races, channel multiplication or technical defects must never become accepted sources of value.
3. **No hidden macro tuning.** An intentional change to a source, sink, reward cadence or scarcity rule is an explicit profile/ruleset revision difference, not an implementation convenience.
4. **Measurable provenance.** Important item/currency sources and sinks should be classifiable by stable domain cause once their owning event/transaction contracts exist, allowing Game Intelligence to measure economy health without mutation authority.
5. **Scarcity is semantic before numeric.** Rare/valuable rewards should remain meaningfully scarce relative to the selected ruleset/activity cadence; exact probabilities and quantities remain downstream game-design decisions.
6. **World age/population matter.** Price, liquidity, wealth concentration and total supply are interpreted in world-age/population context rather than treated as Reference constants.
7. **Security/legal/integrity override defect compatibility.** Reference parity never requires reproducing a duplication exploit, stale-authority defect or unsafe mutation path.

### 6.3 Evolved economy direction

Evolved retains the same conservation/provenance/audit foundations but may later change source/sink/scarcity policy through explicit, versioned and measurable economy decisions after the Reference baseline and real evidence exist.

The first Evolved package remains reliability/UX-first and does not automatically include macroeconomic redesign.

## 7. Recommended measurable success categories

The minimum vision should freeze **what kinds of outcomes matter**, while leaving target numbers to a named alpha/release milestone based on real evidence.

### 7.1 Reference correctness

Measure whether the bounded Reference surface behaves as declared:

- selected parity-matrix coverage;
- `PARITY_CONFIRMED`, pending-evidence and declared-difference classifications;
- number/severity of undocumented divergences;
- reproducibility against the exact Reference revision under test.

### 7.2 Player interaction quality

Measure whether a player can understand and complete the intended loop:

- healthy launch/login/world-entry path;
- movement/combat/input responsiveness on supported conditions;
- reconnect/recovery success and clarity;
- session completion and abandonment reasons;
- actionable client-visible error/session/channel state.

### 7.3 Progress and value trust

Measure whether durable progress remains trustworthy:

- no known item/currency duplication path under accepted failure scenarios;
- committed progress/value survives required recovery cases;
- stale writers/obsolete session generations cannot overwrite newer state;
- important value sources/sinks are attributable once the owning event/transaction infrastructure exists.

### 7.4 Core-loop health

Measure both sides of the accepted product direction:

- an ordinary meaningful solo session can produce legitimate progress;
- coordinated party play can produce a real advantage or unlock a harder coordinated objective;
- PvP, where enabled/in scope, remains fair, authoritative and exploit-resistant without becoming mandatory for all progression;
- representative content supports preparation -> activity -> secure progress -> recovery/planning rather than accidental dead ends.

### 7.5 Economy health

Once privacy-safe and transaction-safe instrumentation exists, measure at least:

- item/currency source and sink composition;
- money-supply/value growth in world-age/population context;
- concentration and liquidity where markets exist;
- suspicious value creation and duplication indicators;
- progression time-to-upgrade or equivalent scarcity outcomes when the relevant systems exist.

These observations inform human product decisions. Game Intelligence does not autonomously tune the economy.

### 7.6 Product and operational health

When external evaluation begins and privacy policy permits, track categories such as:

- session frequency/return behavior and bounded retention cohorts;
- severe player-visible incidents;
- support burden and common abandonment/failure causes;
- population distribution and later channel-friction metrics;
- content-production throughput once the headless toolchain is real.

This section selects categories, not numeric success thresholds.

## 8. Deliberate deferrals and hard downstream gates

### RECOMMENDATION — accept these deferrals with the package

The following remain important but do **not** all need to be frozen to let the minimum `GAME-VISION-01` gate unblock its next architecture work.

| Topic | Proposed state after closure | Hard requirement before |
|---|---|---|
| Exact Global Tibia patch/date/behavior baseline for first Reference revision | `DEFERRED WITH HARD GATE` | Any broad Reference mechanics/content implementation or final parity fixtures that require concrete target semantics, and the first external Reference evaluation/release contract. |
| Exact Reference revision naming scheme | `DEFERRED` | Release/version tooling depends on it. |
| Complete numbered design-pillars/anti-pillars catalogue | `DEFERRED FORMALIZATION` | Revisit only if the already accepted player-promise/product filters cease to reject scope coherently. |
| Exact death/progression/PvP/party formulas | `OWNED DOWNSTREAM` | Their dedicated character/combat/PvP/party/ruleset gates before implementation. |
| Exact economy rates, prices, drops, fees, sink values and scarcity thresholds | `OWNED DOWNSTREAM` | `GAME-ITEM-01`, economy/content gates and relevant DUR contracts before implementation. |
| Exact numeric product/KPI thresholds | `MILESTONE-OWNED` | The named external alpha/release milestone may be declared successful. |
| Exact first Evolved feature inventory | `DEFERRED` | First Evolved implementation milestone. Strategy is already reliability/UX-first. |
| Branding/public marketing wording | `DEFERRED` | Public launch/marketing claim. |
| Monetization/Premium/VIP economics | `DEFERRED` | Dedicated `PROD-ENTITLEMENTS-01` / business decision before activation. |
| LiveOps cadence and automatic economy control | `DEFERRED` | Dedicated LiveOps/economy decision; automatic gameplay/economy mutation is not implied. |

### 8.1 Design-filter clarification

This does **not** defer having a product/design filter. `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md` already provides a binding internal product promise and an explicit proposal-rejection filter, and the accepted PvP/solo-party/Evolved baselines add further product constraints.

Only the creation of a more exhaustive numbered pillars/anti-pillars catalogue is deferred. If future proposals cannot be decided coherently from the accepted promise and baselines, that formalization becomes necessary before those proposals proceed.

### 8.2 Exact Reference baseline clarification

Reference-first already requires every evaluated release to name an immutable Reference baseline, and hybrid tracking defines how future revisions evolve. The exact first patch/date does not need to be selected merely to let `GAME-CHAR-01` define lifecycle/ownership architecture that can faithfully represent Reference rules.

However, the deferral is **not permission to guess Reference semantics**. If a `GAME-CHAR-01` or other downstream decision cannot be made baseline-neutral, it becomes blocked on the concrete Reference baseline rather than inventing a default.

Likewise, broad Reference mechanics/content implementation and final parity fixtures that depend on concrete rules remain blocked until the exact named baseline is selected. This preserves the older `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md` requirement without forcing an early patch/date choice merely for completeness.

## 9. What acceptance of this packet would mean

If the owner accepts the recommended package, a separate owner-baseline delivery should record that:

1. the seven existing partial owner baselines remain binding and unchanged;
2. sections 5-7 of this packet become the accepted minimum loop/economy/success contract;
3. section 8 becomes the accepted deliberate-deferral and hard-gate policy;
4. `GAME-VISION-01` may move from `PROPOSED / PLANNED` to **accepted for its minimum product-vision gate scope**, while every explicitly deferred/downstream topic remains open under its named gate;
5. acceptance still authorizes **no runtime implementation by itself**;
6. `GAME-CHAR-01` becomes the next product-sensitive architecture gate in the programme order, alongside already permitted bounded `DUR-02` discovery;
7. broad Reference gameplay/content implementation remains blocked whenever concrete parity semantics are required but the exact first Reference baseline has not yet been selected.

The accepted closure baseline should update current mutable programme/status documents rather than rewriting the historical pre-decision analyses or seven narrower owner baselines.

## 10. What rejection or partial acceptance would mean

If the owner rejects or modifies any part, preserve all seven existing accepted baselines and change only the unresolved component.

Real alternative choices are:

- **Core loop:** replace the recommended multi-goal persistent-world loop with a different product loop and state what downstream work it should optimize for.
- **Economy:** choose `Reference with early Oteryn macro corrections` instead of Reference-rule-first, with every initial difference explicitly named; or deliberately pull an economy-led Evolved strategy forward, which would supersede the accepted first-Evolved ordering and therefore requires a stronger explicit superseding decision.
- **Success criteria:** replace/add product-success categories, while numeric targets may still remain milestone-owned.
- **Deferrals:** pull a deferred subject into the minimum gate only if it genuinely blocks the next safe proof.

## 11. Evidence that could justify later supersession

After acceptance, reopen the minimum vision only for named evidence such as:

- parity fixtures showing the Reference model is not representable or interpretable as defined;
- external playtests showing the core session/long-term loop does not produce the intended player experience;
- economy telemetry showing inherited source/sink mechanics create unacceptable outcomes that justify an Evolved revision;
- progression/retention/session evidence showing solo/party or PvP product emphasis is materially wrong;
- legal/provenance constraints;
- owner-approved product strategy change.

Implementation convenience, a fashionable MMO mechanic or an isolated technical preference is not sufficient evidence to silently redefine the product vision.

## 12. Owner decision requested

### RECOMMENDATION

Accept the **minimum closure package** in sections 5-8 as one coherent product decision:

```text
core loop
= player-chosen goals -> preparation -> risk/activity -> secure progress/value -> recovery/planning
  across persistent character/economy/exploration/social horizons

Reference economy
= mechanical source/sink parity, not historical price/supply parity
  + conservation
  + measurable provenance
  + no hidden tuning

success
= Reference correctness
  + player interaction quality
  + progress/value trust
  + core-loop health
  + economy health
  + product/operational health
  with numeric thresholds owned by later milestones

explicit deferrals
= exact first Global baseline is a hard downstream gate before concrete Reference semantics;
  formulas/rates, KPI numbers, branding, monetization, exact Evolved feature list
  and LiveOps cadence remain downstream
```

Until the product owner explicitly accepts or replaces this recommendation, this file remains **PRE-DECISION SYNTHESIS / NOT ACCEPTED** and `GAME-VISION-01` remains not fully accepted.
