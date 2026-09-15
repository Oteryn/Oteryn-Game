# GAME-CHAR-01 — Stage B Reference Evidence Reconciliation

- Status: **PRE-DECISION EVIDENCE ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-CHAR-01` Stage B
- Reference target: **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Trusted repository base: `blakinio/Oteryn-v2@ef906b3c2d9cbb9cb7a455a94f84068fb6175795`
- Purpose: determine which Reference-sensitive character semantics are sufficiently evidenced for the selected target and which remain fail-closed
- Does not authorize: Stage-B acceptance, overall GAME-CHAR acceptance, runtime/client/protocol/persistence/content implementation, physical schema, production rollout or intentional Reference differences

## 1. Executive result

### Can full GAME-CHAR Stage B close now?

**NO.**

The selected July-28 target removes version ambiguity, but it does not remove evidence gaps. Current evidence establishes useful character-state vocabulary, several long-lived rule families and important migration constraints. It is **not** sufficient to freeze the complete target-accurate naming/deletion/creation contract, exact progression formulas, all promotion/death edge cases, offline-training effectiveness or the ownership/shape of every modern character-build subsystem.

The correct next step is evidence acquisition and reconciliation, not owner acceptance of guessed mechanics.

### What is already materially useful

The evidence is strong enough to constrain later architecture without closing Stage B:

- five vocation families and promoted forms are clearly present around the selected cut;
- official target-day Character Bazaar surfaces expose level, experience, hit points, mana, capacity, speed, blessings and the named skill vocabulary;
- the 25-active-character account quota predates the target and is still documented after it;
- seven regular blessings existed around the selected cut;
- the long-lived death-loss model and offline-training timing have strong before/after continuity evidence;
- capacity has explicit historical migration semantics proving it cannot always be treated as a pure level/vocation-derived value;
- Targuna was introduced months before the target, while a currently indexed official Quick Start still describes the superseded Thais Peninsula / Blue Valley route, proving that even official documentation must be evaluated claim-by-claim and chronologically.

## 2. Evidence discipline

The accepted first-Reference owner baseline defines:

- `PROVEN`
- `OBSERVED`
- `DERIVED`
- `UNKNOWN`
- `CONFLICT`
- `DECLARED_DIFFERENCE`

These classifications apply to a **July-28 target claim**, not merely to the existence of a current web page.

```text
PROVEN that current official documentation says X
!= PROVEN that July-28 target behavior was X
```

Likewise:

```text
no discovered change note
!= proof that a rule did not change
```

Where primary evidence exists both before and after the target with no material conflict, this dossier may classify continuity as `DERIVED`. It does not relabel that as direct target proof.

### Target-boundary timestamp rule

Official July 27 news stated that the July 28 server save would take about 45 minutes and that worlds/website were **expected** back around 10:45 CEST. That is an expected recovery time, not a cryptographically exact cut timestamp.

Character Bazaar entries starting around 10:42–10:45 on July 28 are therefore strong target-day evidence and strongly aligned with post-maintenance availability, but the dossier does not overclaim minute-level certainty where the public evidence does not establish it.

Auctions created before the server save and merely **ending** later on July 28 are not direct post-cut snapshots: Character Bazaar state is associated with the character put up for auction, so an end timestamp after the cut does not prove the listed state was captured after the cut.

This distinction supersedes the earlier draft's overly strong use of an auction ending at 10:00 CEST as direct target evidence.

## 3. Primary evidence inventory

### 3.1 Target-day official Character Bazaar evidence

Official Character Bazaar/search surfaces preserved around July 28 show auctions **starting on July 28 at approximately 10:42–10:45 CEST** after the maintenance/server-save window, including examples across:

- Druid / Elder Druid;
- Knight / Elite Knight;
- Monk / Exalted Monk;
- Paladin / Royal Paladin;
- Sorcerer / Master Sorcerer.

Examples include official listings such as:

- `Mirassol Paido Palmeiras` — Elder Druid, start Jul 28 10:43 CEST;
- `Cremaax` / `Daoro` / `Szajbaszajba` — Elite Knight, start Jul 28 10:42–10:44 CEST;
- `Erothir Dannius` — Exalted Monk, start Jul 28 10:45 CEST;
- `Ubf Metax` / `Uthr Capac` — Royal Paladin, start Jul 28 10:42–10:43 CEST;
- `Lord Passo` — Master Sorcerer, start Jul 28 10:42 CEST;
- `Storn do Machado` — Elite Knight, start Jul 28 10:44 CEST, blessings 6/7.

These are classified as **`OBSERVED / strong target alignment`** rather than automatically `PROVEN post-cut to the exact minute`.

### 3.2 Boundary-era detailed character evidence

Detailed official auction pages around the cut expose the shape of Character Bazaar state, including:

- HP;
- mana;
- capacity;
- speed;
- regular blessings count;
- experience;
- fist, club, sword, axe, distance, shielding, fishing and magic-level skills;
- world-transfer availability;
- charm/task/proficiency/build-related surfaces;
- Bonus Promotion Points and Animus-related fields.

`Fiebe` (Elder Druid; auction ended Jul 28 20:00 CEST) is a useful detailed **boundary-era** example with HP 650, mana 2,880, capacity 1,600, speed 210, blessings 7/7, all eight skill categories and experience. Because its auction started Jul 27, it is not treated as direct post-cut state; it is corroborative target-era vocabulary evidence.

Likewise, any auction ending exactly at 10:00 or created before the July-28 save is boundary/pre-cut-adjacent evidence only.

### 3.3 Dated pre-target primary sources

Official historical material establishes:

- 2009 death-penalty redesign: level-based experience/skill loss, promotion reduction and blessing/item-loss percentages;
- 2012 offline-training introduction and timing rules;
- 2017 two additional regular blessings, taking the total to seven, plus Death Redemption;
- 2025 Monk rollout and explicit active-character-limit increase from 20 to 25;
- 2025 base-capacity increase by 200, with existing-character effect applied only after level-up;
- 2026 Targuna introduction and follow-up live fixes months before the target;
- 2026 five-vocation balancing/release-state material and July live adjustments.

### 3.4 Current primary sources

Current official sources include:

- `Creating a Character` manual;
- account-management character/deletion manual;
- character manual for progression, skills, promotion, vocation and death;
- Offline Training FAQ;
- promotion FAQ;
- Quick Start guide.

They are post-target evidence, not automatic July-28 snapshots.

## 4. Evidence summary matrix

| Stage-B subject | July-28 evidence result | Freeze readiness | Main reason |
|---|---|---|---|
| Five-vocation roster + promoted forms | `OBSERVED` target-day + strong pre-target continuity | **STRONG vocabulary candidate** | July-28-start Bazaar surfaces plus 2025/2026 official vocation material |
| Level / experience / HP / mana / capacity / speed vocabulary | `OBSERVED` target-era / strongly target-aligned | **STRONG vocabulary candidate** | official Bazaar detailed state around target; exact minute continuity not overclaimed |
| Eight skill categories | `OBSERVED` target-era / strongly target-aligned | **STRONG vocabulary candidate** | official Bazaar detail exposes fist/club/sword/axe/distance/shielding/fishing/magic level |
| Active-character quota = 25 | `DERIVED` strong continuity | **STRONG candidate** | official 2025 increase 20→25 + current official manuals 25 |
| Total active+deletion quota = 30 | `UNKNOWN` target continuity | **NOT READY** | current account manual only in gathered evidence |
| 60-day reversible deletion | `UNKNOWN` target continuity | **NOT READY** | current account manual only in gathered evidence |
| Deleted-name reuse delay ≥6 months | `UNKNOWN` target continuity | **NOT READY** | current manual only; exact recycling semantics unresolved |
| Name max 29 letters / restrictions | `UNKNOWN` target continuity | **NOT READY** | current creation manual only; implementation-grade normalization unspecified |
| Creation inputs name/sex/world | `UNKNOWN` exact target contract | **NOT READY** | current manual says so; target-era surfaces prove facts exist but not creation transaction semantics |
| Newhaven→Targuna flow | `PROVEN` live before target | **READY as content chronology** | March/April 2026 official release/fix chronology |
| Exact starter template/items/stats/home-city/citizenship | `UNKNOWN` | **NOT READY** | no complete target-period primary capture |
| Vocation selection at/around level 8 | `DERIVED` | **PARTIALLY READY** | long-lived official level-8 model + Targuna chronology/current guide, exact target transaction not captured |
| Per-level HP/mana/capacity gains | `OBSERVED`/`DERIVED` | **NOT READY as universal formula** | current table + target-era samples; capacity migration proves state caveat |
| Capacity as pure derived value | rejected by `PROVEN` migration evidence | **READY negative constraint** | 2025 +200 migration activates for existing character only after level-up |
| Exact experience→level formula | `UNKNOWN` | **NOT READY** | no complete primary target formula evidence |
| Exact skill advancement formulas | `UNKNOWN` | **NOT READY** | skill vocabulary known, advancement curves not proven |
| Promotion existence + titles | `OBSERVED` target-day + pre-target continuity | **STRONG vocabulary candidate** | promoted forms visible around target; exact eligibility remains separate |
| Promotion level 20 / 20,000 gp / Premium eligibility | `UNKNOWN` target continuity | **NOT READY** | current official rules gathered, target continuity not yet proved |
| Seven regular blessings | `OBSERVED` target-day + `PROVEN` pre-target introduction | **STRONG candidate** | 2017 introduction plus July-28-start listings showing /7 counts |
| Base death XP/skill model | `DERIVED` strong continuity | **STRONG candidate rule** | 2009 official rule + current official manual materially agree |
| Promotion death-loss reduction = 30% | `DERIVED` strong continuity | **STRONG candidate rule** | historical/current primary agreement |
| Regular blessing XP/skill reduction = 8% each | `DERIVED` strong continuity | **STRONG candidate rule** | historical/current primary agreement + seven-blessing continuity |
| Equipment-loss protection ladder | `DERIVED` strong continuity | **STRONG candidate rule** | 2009 official model + current manual materially agree |
| PvP blessing / skull / Adventurer's Blessing target edge rules | `UNKNOWN` target continuity | **NOT READY** | current guide/manual detail is not target continuity proof |
| Death Redemption exists | `DERIVED` strong continuity | **STRONG existence candidate** | official 2017 introduction + current manual |
| Offline training >10 min + max 12 h | `DERIVED` strong continuity | **STRONG candidate rule** | 2012 official introduction + current FAQ materially agree |
| Exact offline-training effectiveness/formulas | `UNKNOWN` | **NOT READY** | primary material does not establish full target formula |
| Wheel / Bonus Promotion Points visible | `OBSERVED` target-era | **VOCABULARY ONLY** | Bazaar exposes fields; aggregate ownership remains unresolved |
| Proficiencies / Animus / charm/task ownership | `UNKNOWN / boundary unresolved` | **NOT READY** | public character surface does not establish semantic owner |

## 5. Creation and starter-state reconciliation

### 5.1 Current official creation inputs

The current official creation/Quick Start material says creation includes at least:

- character name;
- gender/sex;
- game world.

The current account/creation surfaces also state a 25-active-character limit.

### Target classification

- existence of name/sex/world as public character facts is well supported;
- the exact July-28 creation transaction, validation policy and initial template remain `UNKNOWN`.

The architecture must preserve room for these inputs without treating today's website flow as a complete July-28 creation contract.

### 5.2 Targuna versus stale Quick Start — verified official conflict

Official March 17, 2026 news states that from level 8 onward players no longer travel directly from Newhaven to Thais Peninsula; instead all characters, including monks, continue to **Targuna**. Follow-up March/April fixes describe Targuna as introduced/live well before July 28.

A currently indexed official Quick Start page still says the only way out of Newhaven leads to Thais Peninsula, with monks going to Blue Valley.

Classification:

- Targuna live before July 28: `PROVEN` by dated primary release chronology;
- current Quick Start's Thais/Blue Valley departure wording: `CONFLICT / stale official documentation` for target flow;
- exact complete starter transaction/template at July 28: `UNKNOWN`.

### Architecture consequence

Do **not** hard-code starter routing, initial home-city/citizenship or starter state from whichever official manual paragraph is easiest to find. Stage A's explicit versioned ruleset/content starter template remains the correct boundary.

## 6. Account quota, deletion and restore

### 6.1 Active-character quota

Official February/March 2025 Monk rollout material explicitly increased the active-character limit from 20 to 25. Current official creation/account manuals also state 25 active characters.

Target classification: **`DERIVED` strong continuity**.

This is a strong July-28 candidate rule, but not direct target snapshot evidence.

### 6.2 Current deletion model

The current official account manual states:

- deletion reversible for 60 days;
- deletion-scheduled characters do not count against the 25 active limit;
- active + deletion-scheduled total may not exceed 30;
- undelete during the grace period is constrained by the 25-active limit;
- final deletion cannot be restored;
- deleted names cannot be chosen for new characters for at least six months.

The gathered evidence does not yet establish a dated July-28 snapshot or sufficient before/after continuity for those exact numeric values.

Therefore:

- `60 days` = `UNKNOWN` target continuity;
- `30 total` = `UNKNOWN` target continuity;
- `≥6 months name hold` = `UNKNOWN` target continuity.

Stage A already safely freezes the semantic lifecycle distinction:

```text
ACTIVE -> DELETION_SCHEDULED -> RETIRED
```

without guessing these target-specific durations/quotas.

## 7. Naming evidence

Current official creation documentation states, among other things:

- maximum 29 letters;
- no numbers;
- no special characters;
- some words/letter combinations are disallowed;
- Tibia Rules apply;
- rename exists as a later paid operation.

This is insufficient for an implementation-grade July-28 Character Authority contract. Missing target-quality evidence includes:

- Unicode/canonical normalization;
- case folding;
- whitespace normalization;
- exact permitted repertoire;
- machine-level uniqueness scope;
- reserved-word revision/source;
- deleted-name reuse timing/exceptions;
- rename history/redirect semantics.

Exact naming policy remains **NOT READY**. DUR-02 must not freeze final unique-index scope or normalization based on current-only documentation.

## 8. Durable progression vocabulary

Official Character Bazaar surfaces around the target establish a stable public vocabulary including:

- level;
- experience;
- hit points;
- mana;
- capacity;
- speed;
- vocation / promoted vocation;
- blessings;
- Fist Fighting;
- Club Fighting;
- Sword Fighting;
- Axe Fighting;
- Distance Fighting;
- Shielding;
- Fishing;
- Magic Level.

July-28-start listings around 10:42–10:45 and detailed boundary-era auction pages support this vocabulary. Because the public maintenance notice provides an expected return time rather than an exact cut attestation, this dossier classifies the target-day evidence conservatively as `OBSERVED / strong target alignment`, combined with pre/post continuity where available.

### Result

The vocabulary is strong enough to constrain Stage-B evidence work and later schema questions, but this analysis does not ask the owner to accept it yet as a complete Stage-B contract.

### Exact formulas remain separate

Current official manuals do not provide every implementation-grade formula for:

- total experience required per level;
- low-level modifiers;
- exact walking-speed formula;
- skill advancement curves;
- all derived combat stats;
- rounding/overflow.

Those remain `UNKNOWN` or belong to `SIM-DETERMINISM-01` / gameplay/ability gates.

## 9. Capacity migration — authoritative fact versus derived value

Official 2025 Newhaven release material states that base capacity was increased by 200, raising it to 600, but for an **existing character** the new value took effect in game only after that character levelled up.

This is a decisive semantic constraint.

At a later target such as July 28, imported/reference state cannot safely assume:

```text
capacity = pure_function(level, vocation)
```

without source-revision/migration-state evidence.

The eventual persistence model therefore needs either:

- authoritative persisted capacity state; or
- sufficient deterministic source revision + migration state to derive the target value exactly.

The physical choice belongs to DUR-02. The semantic requirement follows directly from primary migration evidence and reinforces Stage A's progression-facts-vs-derived-values rule.

## 10. Vocation and promotion reconciliation

### 10.1 Five vocation families

Official 2025 Monk launch material and 2026 vocation-adjustment releases establish the five-vocation model before July 28. July-28 Character Bazaar surfaces around the maintenance boundary expose Druid/Elder Druid, Knight/Elite Knight, Monk/Exalted Monk, Paladin/Royal Paladin and Sorcerer/Master Sorcerer listings.

Classification: **strong `OBSERVED` target-day + pre-target continuity**.

### 10.2 Ownership boundary

- vocation/promotion is durable Character-owned build/profession state;
- exact combat effects belong to ruleset/ability/simulation layers;
- the five target vocations do not become a universal engine enum required by every future profile.

### 10.3 Promotion eligibility

Current official material states level 20, 20,000 gp and Premium-account eligibility, with suspension/restoration behavior tied to Premium state.

Target continuity for these exact values/effects has not yet been established in this task.

Classification: `UNKNOWN` target continuity for the exact promotion eligibility/commercial contract.

## 11. Modern character-build state boundary

Character Bazaar surfaces around the target expose or reference additional long-lived state such as:

- Bonus Promotion Points;
- Animus Masteries;
- charm points / minor charm echoes;
- Hunting Task points;
- proficiency-related surfaces;
- permanent/additional slots.

Public display on one character page does **not** prove all such state belongs to the core Character aggregate.

Open ownership questions:

- Wheel allocation / promotion-point spend may be Character-owned build state but must reconcile with ability/simulation rules;
- weapon proficiency crosses Character progression and item/weapon-definition semantics;
- charms/bestiary/Animus/task systems may be separate progression/content aggregates or character-scoped subdomains;
- purchased/permanent slots may involve Platform/product entitlement authority.

Full modern build-state ownership is not Stage-B freeze-ready.

## 12. Death and respawn reconciliation

### 12.1 Long-lived base loss model

Official 2009 production-change documentation introduced the level-scaled death-loss model that current official material still describes materially similarly:

- an unprotected low-level character loses 10% experience/skills under the documented threshold model;
- higher levels use a level-dependent reduction;
- promotion reduces death loss by 30%;
- each regular blessing reduces experience/skill loss by 8%.

Classification: `DERIVED` strong continuity, not direct July-28 proof.

### 12.2 Seven regular blessings

Official 2017 material introduced two extra regular blessings, taking the total to seven. July-28 target-day listings show blessing denominators/counts such as 6/7 and 7/7.

Classification: strong target-day `OBSERVED` + pre-target `PROVEN` introduction.

### 12.3 Equipment-loss protection

The 2009 official model and current manual materially agree on:

- 10% base chance per equipped item;
- blessing protection ladder 30/55/75/90/100% for 1/2/3/4/5+ blessings.

Classification: `DERIVED` strong continuity.

Item/equipment/corpse conservation remains combat + `GAME-ITEM-01` / `DUR-03` authority, not duplicated into Character state.

### 12.4 Still unresolved target edge cases

Target continuity remains insufficient for the complete persistent effects of:

- Twist of Fate / PvP blessing;
- PvP damage qualification windows;
- red/black skull exceptions;
- Adventurer's Blessing;
- Retro Hardcore PvP loss rules;
- Death Redemption eligibility details;
- other special PvP loss adjustments.

These remain `UNKNOWN` until evidenced.

## 13. Offline training reconciliation

Official 2012 material introduced offline training, and current official FAQ material still documents the same key timing semantics:

- training begins only after more than 10 minutes offline;
- counter maximum is 12 hours;
- training consumes/refills the counter under documented timing rules;
- statue/bed choice selects the supported training path.

Classification for the **10-minute threshold and 12-hour maximum**: `DERIVED` strong continuity.

Still unresolved:

- exact target-era selectable skills per vocation;
- advancement effectiveness/formula;
- tick/rounding behavior;
- loyalty/temporary modifier interactions;
- interruption/migration edge cases.

These remain `UNKNOWN`; no background timer implementation may invent them.

## 14. Evidence-safe downstream constraints before Stage-B acceptance

This dossier remains nonbinding, but downstream discovery may use the following as **questions/constraints**, not final Reference rules:

1. the target character domain must be able to represent level, experience, HP, mana, capacity, speed, the eight skill categories, vocation/promotion and blessing state;
2. vocation remains profile/ruleset-owned rather than an engine fork;
3. capacity must support migration-sensitive authoritative state and cannot universally be recomputed from level/vocation;
4. deletion needs a reversible/nonterminal phase distinct from terminal retirement, already accepted by Stage A;
5. death consequences must split Character-owned progression/blessing effects from item-conservation transactions;
6. offline training needs explicit capability/counter/revision semantics rather than implicit progress;
7. modern build/progression systems require explicit ownership rather than one giant Character row;
8. final naming unique-index/normalization cannot be frozen yet.

DUR-02 may use these only inside its already accepted bounded-discovery permission. Final schema still waits for accepted Stage B.

## 15. Full Stage-B blockers

### B1 — naming contract

Need target-quality evidence for canonicalization, case/space semantics, permitted repertoire, uniqueness scope, reserved/restricted patterns, rename history/conflict and deleted-name recycling.

### B2 — deletion/quota target continuity

Need target-quality evidence for 60-day deletion, total 30 limit, undelete/quota interaction and deleted-name hold. The 25-active quota has strong continuity evidence but remains derived.

### B3 — exact creation/starter template

Need target-period evidence for initial level/stats/skills/capacity, starter equipment/items, home/citizenship state, Targuna/vocation transition ownership and retry-visible creation semantics.

### B4 — exact progression formulas

Need evidence/ownership assignment for experience→level, skill curves, speed, low-level modifiers, authoritative-vs-derived representation, rounding and overflow.

### B5 — promotion target contract

Need target continuity for level/cost/Premium eligibility and promotion suspension semantics, plus durable state versus entitlement projection boundary.

### B6 — death edge cases

Need target evidence for PvP/skull/blessing/Death-Redemption rules that materially affect persistent character/value state.

### B7 — offline-training effectiveness

Need target evidence for selectable skills, effectiveness and exact state transitions/rounding.

### B8 — modern build-state ownership

Need explicit ownership for Wheel allocations, Bonus Promotion Points, proficiency and other character-visible progression systems without absorbing GAME-ITEM/content/entitlement authorities.

## 16. Evidence-acquisition plan

### Priority 1 — historical primary snapshots

Seek preserved July-28-era official pages for:

- account/character management;
- character creation;
- character manual;
- promotion/offline-training/naming/deletion FAQ/manual content.

### Priority 2 — target-day Bazaar samples

Capture a bounded set whose **auction start** timestamps align with resumed July-28 service, across all five vocation families and relevant progression/migration histories. Treat auction end time as insufficient for snapshot timing.

### Priority 3 — rule-specific official chronology

Build dated chronology for:

- quotas/deletion/name policy;
- capacity/HP/mana progression;
- promotion;
- death/blessing/PvP death;
- offline training;
- Targuna/creation flow;
- Wheel and modern progression systems.

### Priority 4 — controlled black-box observation

Current Global observation answers a July-28 question only if continuity from target to observation is evidenced. Otherwise record it as current-state `OBSERVED`, not target proof.

### Priority 5 — community/OTS research

Use community and OTS sources only to discover candidate formulas/change dates, design tests and locate missing primary evidence. They do not independently promote a claim to `PROVEN`.

## 17. Owner-decision result

### Recommendation

**Do not ask the owner to accept full Stage B yet.**

The unresolved items are primarily evidence questions, not product preferences. Owner intervention becomes appropriate only if:

- target evidence proves unavailable and an explicit Reference fallback/difference policy is required;
- credible sources remain in `CONFLICT` after investigation;
- a genuine product choice is not determined by Reference evidence;
- an intentional simplification would materially diverge from the selected target.

Until then, autonomous evidence acquisition should continue.

## 18. Decision-timing discipline

### Must Stage B be accepted now?

**NO — evidence acquisition should continue first.**

### Blocked downstream work

- final character-bearing DUR-02 schema;
- broad Reference character-progression implementation;
- target-accurate naming/deletion/creation fixtures;
- final death/offline-training persistent behavior.

### Still allowed

- GAME-CHANNEL architecture under separate ownership;
- bounded DUR-02 discovery using Stage-A invariants and target context only;
- Reference evidence-manifest/tooling design;
- GAME-ITEM evidence work against the same target without ownership/path collision;
- Stage-B evidence acquisition.

### Why guessing is expensive

Freezing current-only or OTS-inferred semantics now would bake uncertainty into unique constraints, migration logic, progression state and deterministic fixtures. Repairing that later is more expensive than preserving `UNKNOWN` today.

## 19. Deliberately not decided here

- final Stage-B owner contract;
- final naming normalization/index model;
- deletion timers/quotas without target continuity;
- exact starter template;
- exact XP/skill/speed formulas;
- exact promotion eligibility/entitlement interaction;
- full PvP death edge matrix;
- offline-training effectiveness;
- Wheel/proficiency/charms/Animus/task aggregate ownership;
- PostgreSQL physical schema;
- runtime/client/protocol/content implementation;
- production activation.

Until evidence blockers are reduced enough for an honest owner package, `GAME-CHAR-01` remains **PROPOSED / PLANNED / NOT_STARTED** with Stage A accepted, Stage B unaccepted, and runtime authority **NONE**.
