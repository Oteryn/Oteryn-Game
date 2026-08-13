# GAME-CHAR-01 — Stage B Reference Evidence Delta 02 (B4-B8)

- Status: **PRE-DECISION EVIDENCE DELTA / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-CHAR-01` Stage B
- Reference target: **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**
- Trusted task base: `blakinio/Oteryn-v2@1411994c70abbf065273c0502c88413b61ca5ca0`
- Reconciled repository base: `blakinio/Oteryn-v2@2ebce3d657f2f844883ef0a5f1a903adbf410984`
- Scope: **B4 progression, B5 promotion, B6 death/PvP, B7 offline training, B8 modern build/progression ownership only**
- B1-B3 authority: `GAME-CHAR-01_STAGE_B_B1_B3_EVIDENCE_ACQUISITION.md`
- Supersedes: nothing; additive to the prior Stage-B reconciliation and B1-B3 acquisition
- Runtime authority: **NONE**
- Does not authorize: Stage-B acceptance, overall GAME-CHAR acceptance, runtime/client/protocol/persistence/content implementation, physical schema, production rollout or intentional Reference differences

## 1. Purpose and scope discipline

This delta reduces only B4-B8 evidence blockers using additional primary official Tibia evidence.

B1 naming, B2 deletion/quota and B3 creation/starter are intentionally **not re-analysed here**. Their current evidence state is canonical in `GAME-CHAR-01_STAGE_B_B1_B3_EVIDENCE_ACQUISITION.md` delivered by PR #185 and lifecycle-closed by PR #186.

Accepted evidence discipline remains:

```text
target cut selected
!= every rule proven

current official documentation says X
!= July-28 target behavior was X

no discovered change note
!= proof that X did not change
```

## 2. Executive result

### Can full Stage B close after this delta?

**NO.**

The delta materially reduces B4-B8 uncertainty, but exact target arithmetic/effectiveness and some target-sensitive PvP/entitlement/migration edges remain unresolved.

### What improved materially?

Primary official evidence strengthens:

1. promotion level-20 + Premium core eligibility;
2. offline-training timer/pool state-machine semantics;
3. base death/blessing rule continuity and world/profile-specific PvP boundaries;
4. Wheel/Promotion Point character-build ownership;
5. Weapon Proficiency progress ownership;
6. charms/charm points, Hunting Task Points and permanent Hunting Task/Prey slots as character-specific product state;
7. Animus Mastery as strongly character-specific progression;
8. the architecture distinction between durable fact ownership and exact ruleset arithmetic.

### Most important architecture recommendation

Exact gameplay formulas should not automatically become physical-schema blockers merely because they are Reference behavior.

```text
semantic fact ownership + version/migration envelope
!=
exact ruleset arithmetic / parity fixture values
```

If a durable contract remains formula-neutral, explicitly versioned and migration-safe, exact arithmetic can stay blocked for Reference implementation/SIM fixtures without blocking every generic Character ownership/revision decision.

This is **RECOMMENDATION / NOT ACCEPTED**.

## 3. Primary evidence inventory

### E1 — promotion level 20 + Premium

Official 2001 release material introducing promoted vocations states that Premium players at or above level 20 can receive promotion:

- `https://www.tibia.com/news/?id=122&subtopic=newsarchive`

Current official character documentation still requires level >=20 and Premium and additionally documents a one-time 20,000 gp fee plus current Premium-lapse behavior:

- `https://www.tibia.com/gameguides/?section=characters&subtopic=manual`

Classification:

- level >=20 + Premium core eligibility: `DERIVED / very strong primary continuity`;
- exact 20,000 gp July-28 continuity: `UNKNOWN`;
- exact Premium-lapse suspension/reactivation July-28 semantics: `UNKNOWN`.

### E2 — offline-training counter mechanics

Official 2012 introduction material records:

- at least 10 minutes offline before training gain;
- maximum 12 consecutive hours;
- one pool second consumed per training second;
- one pool second restored per second online;
- one pool second restored per second offline without training;
- pool can refill after depletion while offline, but renewed training requires another activation;
- offline training is intentionally less effective than online training;
- vocation affects advancement;
- exact advancement formulas were not fully published there.

Primary sources:

- `https://www.tibia.com/news/?id=2105&subtopic=newsarchive`
- `https://www.tibia.com/news/?id=2125&subtopic=newsarchive`
- `https://www.tibia.com/news/?id=2197&subtopic=newsarchive`

Current official FAQ/manual materially preserves the timer/pool family.

Classification:

- timer/pool state machine: `DERIVED / strong primary continuity`;
- exact July-28 effectiveness coefficients/rounding/modifier interactions: `UNKNOWN`.

### E3 — base death/blessing model

Official 2009 death-penalty redesign records:

- level-scaled experience/skill-loss family;
- promotion reduction of 30%;
- 8% experience/skill-loss reduction per regular blessing;
- equipment-loss protection ladder 30/55/75/90/100% for 1/2/3/4/5 blessings.

Primary source:

- `https://www.tibia.com/news/?id=944&subtopic=newsarchive`

Current official character documentation materially retains the base family while newer systems/world types add edges.

Classification: `DERIVED / strong continuity` for the base family, not exhaustive proof of every July-28 death edge.

### E4 — Twist of Fate / world-type specificity / Adventurer's Blessing

Official PvP history establishes Twist of Fate and profiles where Twist/fair-fight behavior is absent or materially different:

- `https://www.tibia.com/news/?id=2405&subtopic=newsarchive`
- `https://www.tibia.com/news/?id=2946&subtopic=newsarchive`
- `https://www.tibia.com/news/?id=4128&subtopic=newsarchive`

Official 2023 blessing-price material states Twist itself was not adjusted, supporting pre-target continuity:

- `https://www.tibia.com/news/?id=7260&subtopic=newsarchive`

Official Character Bazaar entries around the July-28 boundary expose `Twist of Fate active: yes/no` with regular blessing counts. Because public maintenance evidence gives an expected rather than cryptographically exact resume time, classify the target-boundary observations as `OBSERVED / strong target alignment`.

Official 2014 material establishes Adventurer's Blessing core behavior around the level-20/21 boundary:

- `https://www.tibia.com/news/?id=2894&subtopic=newsarchive`

Classification:

- existence of world/profile-sensitive PvP death policy: `PROVEN`;
- Twist existence and target-era state surface: strong;
- exact July-28 Twist/fair-fight/skull/qualification edge matrix: `UNKNOWN`;
- Adventurer's Blessing core family: `DERIVED / strong continuity`; exact target edge matrix remains incomplete.

### E5 — Wheel / Promotion Points

Official 2022 Wheel design describes Promotion Points as progression used to specialise a character, with vocation-specific Wheel definitions:

- `https://www.tibia.com/news/?id=7013&subtopic=newsarchive`

Official 2023 presets describe saved Wheel configurations:

- `https://www.tibia.com/news/?id=7336&subtopic=newsarchive`

Official 2026 fixes refer to persistent character Promotion Point state, including excess points granted through hunting tasks:

- `https://www.tibia.com/news/?id=8747&subtopic=newsarchive`

Classification: `DERIVED / strong character-build ownership candidate` for earned points/player allocations; perk definitions/formulas remain ruleset/ability/content-owned.

### E6 — Weapon Proficiency progress ownership

Official 2025 Weapon Proficiency design states directly that **Proficiency Progress is bound to the character and cannot be transferred**. Each weapon has its own proficiency tree/definition and Character Bazaar exposes progress:

- `https://www.tibia.com/news/?id=8421&subtopic=newsarchive`

Official 2026 update preserves the character-progress/weapon-definition split and describes dust as character-bound:

- `https://www.tibia.com/news/?id=8850&subtopic=newsarchive`
- `https://www.tibia.com/news/?id=8845&subtopic=newsarchive`

Classification:

- per-weapon Proficiency Progress character-specific ownership: **`PROVEN`**;
- tree/perk definitions: definition-scoped ruleset/content state, not item-instance ownership;
- dust conservation/economy boundary: still requires GAME-ITEM/DUR-03 reconciliation.

### E7 — Character Bazaar transfer boundary

Official Character Bazaar FAQ/launch material explicitly separates account-bound and **character-specific** state. Character-specific state includes, among other things:

- XP and skill levels;
- blessings;
- charms, charm points and charm expansion;
- Hunting Task Points;
- permanent Hunting Task slots;
- permanent Prey slots;
- daily reward streak;
- completed map areas;
- quest lines, titles, achievements and bestiary progress.

Primary source:

- `https://www.tibia.com/news/?id=5692&subtopic=latestnews`

Classification:

- character-specific product scope for the listed progression/capabilities: **`PROVEN` by official transfer contract**;
- this does **not** imply one giant physical Character row or transaction aggregate.

### E8 — Animus Mastery

Official Soulpit/Animus design describes accumulating Animus Mastery for creatures and gaining bonuses from the number of mastered creatures:

- `https://www.tibia.com/news/?id=7944&subtopic=newsarchive`

Official Character Bazaar listings in 2025/2026 expose `Animus Masteries unlocked` as state of the traded character.

Classification: **strong character-specific progression candidate**. Creature definitions, mastery criteria, bonus formulas and compatibility remain ruleset/content-owned.

### E9 — current official Experience Table

CipSoft currently publishes exact total experience by level:

- `https://www.tibia.com/library/?subtopic=experiencetable`

Classification: exact current relation is primary evidence today; full July-28 continuity remains `UNKNOWN` without target continuity evidence.

## 4. B4 — progression facts versus arithmetic

### Current evidence state

Stage-B and Bazaar evidence already establish the target-era progression vocabulary: level, experience, HP, mana, capacity, speed and skill categories. Historical capacity migration evidence additionally proves that some persisted state cannot safely be assumed to be a pure function of current level/vocation.

### Mandatory decision test

**Must exact XP/skill formulas be decided now for durable schema ownership?**

**RECOMMENDATION: NO**, if the eventual durable contract preserves:

1. authoritative total experience/progression facts;
2. authoritative skill facts at sufficient precision/range;
3. explicit ruleset/profile revision context;
4. deterministic migration/versioning hooks;
5. no physical invariant that assumes an unproven formula;
6. exact formula/rounding fixtures before target-accurate Reference implementation/parity claim.

Unknown formulas still block:

- target-accurate Reference advancement implementation;
- deterministic parity fixtures;
- balance/simulation verification.

They need not alone block:

- Character ownership of persisted XP/skills;
- revision/fencing/transaction-boundary discovery;
- formula-neutral durable representation.

### Owning later gate

Exact arithmetic should be frozen under the relevant ruleset and `SIM-DETERMINISM-01`/gameplay gates before executable Reference parity is claimed.

### Supersession condition

If evidence proves a formula choice changes identity, ownership, atomicity, irreversible representation or migration semantics, that formula becomes a DUR-02/GAME-CHAR prerequisite again.

## 5. B5 — promotion

### Strong candidate core

Promotion as character profession/build state with:

- level >=20;
- Premium eligibility;
- vocation-specific promoted identity;

has very strong primary continuity.

### Still unresolved for the exact target

- exact 20,000 gp fee continuity;
- exact Premium-lapse suspension/reactivation behavior;
- final representation of durable promotion achievement versus entitlement-derived active benefits.

### Architecture recommendation

Character should own promotion achievement/state; current benefit activation should be derived from ruleset + current entitlement rather than erasing promotion history when Premium lapses.

This remains **RECOMMENDATION / NOT ACCEPTED**.

## 6. B6 — death/PvP

### Base family

The base death/blessing family has strong continuity evidence.

### Profile boundary

Official history **proves** death/PvP behavior differs materially by world/ruleset profile. Therefore one universal world-independent Character death formula is invalid architecture.

### Still unresolved

- complete July-28 Twist qualification/consumption/fair-fight rules;
- skull-specific and attacker-level/fair-fight edge behavior by profile;
- exact Death Redemption target-era recovery-count/history semantics;
- full persistent-state ownership split for all PvP-specific counters/locks where required.

### Decision-timing implication

Exact PvP edge matrices should not be silently forced into the generic Character aggregate before `GAME-CHANNEL-01`/world-profile/PvP policy selects the relevant world-type scope. However, any durable PvP fact actually required by the selected profile must be owned/fenced explicitly before final schema acceptance.

This is **RECOMMENDATION / NOT ACCEPTED**.

## 7. B7 — offline training

### Near-freeze-ready semantic state

The primary 2012 design defines a per-character activation/counter/pool state machine:

```text
activation selected
-> >=10 min offline before gain
-> consume 1 pool second / training second
-> max 12 h effective continuous training
-> regenerate 1 pool second / second online
-> regenerate 1 pool second / second offline without training
-> depletion may refill while offline
-> refilled pool requires new activation before use
```

With current continuity this is `DERIVED / strong primary continuity`.

### Still unresolved

- exact July-28 effectiveness coefficients;
- exact selectable-skill matrix where version-sensitive;
- tick/rounding behavior;
- loyalty/event/modifier interactions;
- migration edge cases.

### Architecture implication

Split B7 into:

- semantic counter/capability ownership — **near freeze-ready**;
- target arithmetic/effectiveness — Reference/ruleset/SIM evidence blocker.

## 8. B8 — modern build/progression ownership

### Wheel / Promotion Points

Earned Promotion Point facts and player allocations are strong character-build state candidates. Perk catalogue/formulas remain ruleset/ability/content definitions. Premium entitlement source remains Platform-owned input.

### Weapon Proficiency

Primary official evidence explicitly fixes Proficiency Progress as character-bound/non-transferable.

Recommended semantic boundary:

```text
Character domain
owns per-weapon Proficiency Progress / reached progress
owns player-selected proficiency choices where they are character state

weapon/content/ruleset definition
owns tree structure, thresholds, perks and compatibility

GAME-ITEM / DUR-03
owns item-instance conservation/location/value-transfer invariants
```

### Bazaar-proven character-specific progression

The official Bazaar transfer contract proves character-specific scope for:

- charms/charm points/charm expansion;
- Hunting Task Points;
- permanent Hunting Task slots;
- permanent Prey slots;
- and other explicitly listed character progression/history.

This proves **character-specific product ownership/scope**, not physical aggregate placement. Dedicated child aggregates remain possible when transaction/lifecycle/size evidence justifies them.

### Animus Mastery

Primary design + Bazaar traded-state surface make Animus Mastery a strong character-specific progression candidate. Definition/formula semantics remain content/ruleset-owned.

### Dust / purchased permanent capabilities

A resource can be character-bound while its acquisition/conservation/commercial authority remains outside core Character. Do not turn character scope into economy/Platform authority.

### Residual B8 uncertainty

- exact migration/versioning of Wheel/proficiency/Animus/charm/task definitions;
- newer systems not covered by explicit transfer/ownership evidence;
- core-Character versus child-aggregate placement where independent lifecycle/transaction evidence exists.

B8 is **SUBSTANTIALLY REDUCED**; Weapon Proficiency progress, charms/charm points, Hunting Task Points and permanent Hunting Task/Prey slots should no longer be treated as ownership-unknown.

## 9. Revised B4-B8 blocker state

### B4 progression

**Semantic ownership largely ready; exact arithmetic open.**

Recommendation: exact XP/skill formulas remain Reference fixture/ruleset/SIM blockers unless they prove to constrain durable identity/representation.

### B5 promotion

**Core strongly evidenced; target monetary/entitlement edges open.**

### B6 death/PvP

**Base family/profile scoping strongly evidenced; full target edge matrix open.**

### B7 offline training

**Counter/capability semantics near ready; effectiveness arithmetic open.**

### B8 modern build/progression

**Ownership substantially reduced.** Residual work is mostly definition migration/sub-aggregate placement plus newer systems without explicit transfer evidence.

## 10. DUR-02 implications

DUR-02 remains bounded by Stage A; this delta creates no physical-schema authority.

Nonbinding extension points worth preserving:

- authoritative XP/skill facts independent from exact arithmetic implementation;
- persisted/derived distinction for migration-sensitive stats such as capacity;
- Character-scoped offline-training pool/capability state;
- Character-scoped Wheel/promotion-point state;
- Character-scoped per-weapon proficiency progress keyed by stable definition identity;
- character-scoped charms/task/permanent-slot progression or explicit child aggregates under the same game-domain ownership scope;
- profile/ruleset-scoped death/blessing semantics rather than one universal death formula.

## 11. Decision timing

### Must full Stage B be accepted now?

**NO.**

### Must formula-vs-schema split be accepted in this evidence task?

**NO.** Carry it into a later owner Stage-B closure package.

### Downstream still blocked

- final character-bearing DUR-02 schema acceptance;
- broad target-accurate Reference character implementation;
- final GAME-CHAR Stage-B contract;
- parity fixtures for unresolved formulas/edges.

### Safe parallel work

- B1-B3 evidence follow-up only with a new source/hypothesis per #185 stopping rule;
- bounded DUR-02 discovery under Stage A;
- GAME-CHANNEL/GAME-ITEM evidence work under separate ownership;
- SIM/ruleset formula evidence gathering;
- evidence-manifest/tooling design.

## 12. Next evidence priorities after combined #183/#185/this delta

1. resolve only B1-B3 gaps for which genuinely new target-era primary evidence appears; do not repeat generic search exhausted by #185;
2. determine which B6 PvP/death edges belong to GAME-CHAR versus world/profile policy before demanding one exhaustive matrix;
3. acquire migration/versioning evidence for modern character-specific progression definitions;
4. obtain exact formulas only where required for Reference fixtures or where evidence proves they constrain durable representation;
5. then prepare an owner Stage-B closure packet that explicitly separates accepted semantic ownership from still-deferred versioned policy/arithmetic.

## 13. Deliberately not decided

- no Stage-B rule is owner accepted;
- no XP/skill formula;
- no exact promotion price/Premium-lapse target rule;
- no complete PvP/death edge matrix;
- no offline-training effectiveness formula;
- no final physical sub-aggregate/table layout;
- no DDL/runtime/protocol/content implementation.

Until explicit owner acceptance, this document remains **PRE-DECISION EVIDENCE DELTA / NOT ACCEPTED** and does not change `GAME-CHAR-01 = PROPOSED / PLANNED / NOT_STARTED`.
