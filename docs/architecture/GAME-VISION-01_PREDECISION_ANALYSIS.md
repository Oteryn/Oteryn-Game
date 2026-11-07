# GAME-VISION-01 — Pre-Decision Product Vision Analysis

- Status: **PRE-DECISION ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-VISION-01`
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Decision owner: product owner
- Does not authorize: gameplay implementation, content production, public branding claims, production rollout, monetization policy or acceptance of any recommendation below

## 1. Problem

Oteryn already has a strong technical and authority foundation, plus an accepted high-level product direction: reproduce important observable Global Tibia behavior, permit both Reference and Evolved world-profile families, and keep both on one engine/client/`protocol-oteryn`.

What is still missing is the **minimum measurable product contract** that tells developers, game designers, content authors and testers what the first Oteryn build is trying to prove as a game rather than only as a platform.

Without this gate, technically valid but mutually inconsistent choices can be made for progression, death, combat breadth, quests, economy, content tooling, client UX and world population strategy.

This document prepares owner decisions. It does not make them.

## 2. Accepted constraints — do not silently redesign

### PROVEN

The following are already accepted through ADR-0010, `PRODUCT_DIRECTION_BASELINE.md`, foundation ADRs and the current programme status:

- Oteryn uses one canonical Rust engine/workspace, one native client and one `protocol-oteryn` family.
- A **Reference** world-profile family may target documented player-observable Global Tibia parity.
- An **Evolved Oteryn** world-profile family may apply explicit, versioned Oteryn changes and original expansion.
- The two families use distinct logical `WorldId` values and one logical world never changes profile family by switching channel.
- Characters, progression, inventories, currencies, houses, market and other gameplay value are world-scoped by default; cross-profile value transfer is forbidden until separately accepted.
- Reference and Evolved profiles are permitted to coexist, but simultaneous launch is **not** required.
- A reference parity claim must name an immutable target revision/baseline and classify unsupported or intentional differences explicitly.
- Security, data integrity, privacy and legal safety are not weakened merely for parity.
- Broad gameplay/content production is gated on a minimum `GAME-VISION-01` product contract.
- The first externally evaluated build should normally stay narrow: one profile, one representative area, bounded professions/abilities/items/economy and one channel before a later multichannel proof.

## 3. Decision timing test

### Must decide now?

**YES — minimum scope only.**

The gate must now freeze enough product intent to unblock:

- `GAME-CHAR-01` progression/death/lifecycle design;
- broad gameplay/content implementation choices;
- later `ALPHA-RULESET-01` and `ALPHA-MILESTONE-01` scope;
- parity fixtures and reference evidence strategy;
- initial client UX priorities;
- product-sensitive parts of `DUR-02` discovery.

It does **not** need to freeze endgame formulas, every Oteryn improvement, full monetization, final branding, every profession/ability or complete expansion roadmap.

### What becomes expensive if delayed?

- persistence may encode the wrong progression/lifecycle assumptions;
- content/runtime APIs may be optimized for incompatible gameplay loops;
- parity and evolved tests may lack a stable oracle;
- client UX may optimize for systems that are not part of the launch promise;
- two-profile support may double content/testing cost before one profile proves the product.

### Evidence that could justify superseding later

- parity fixture results;
- playtest retention/session data;
- economy/progression telemetry;
- performance/recovery evidence;
- player research showing the chosen audience/promise is wrong;
- legal/provenance constraints;
- owner-approved product strategy change.

## 4. Key decision A — first launch/evaluation profile

### Option A — Reference-first

Start the first externally evaluated Oteryn build on a single Reference profile. Use that profile to prove engine correctness, native client/server behavior, lawful parity evidence, persistence, recovery and the content toolchain. Introduce an Evolved profile only after the baseline is stable enough that intentional differences can be measured against it.

**Benefits**

- smallest combinatorial test/content/support surface;
- strong oracle for engine correctness;
- easier to distinguish bugs from deliberate design changes;
- preserves development focus while the first real runtime is still missing;
- creates a trustworthy baseline for future Evolved experiments.

**Costs/risks**

- weaker immediate product differentiation;
- players may ask why they should move from Global Tibia if the visible promise is only parity;
- parity work can become an endless chase if the target is not bounded.

### Option B — Evolved-first

Launch only the Evolved profile and use Global Tibia primarily as a behavioral reference/test oracle.

**Benefits**

- stronger differentiation from day one;
- avoids publicly promising exhaustive parity;
- owner-selected improvements can become the product identity immediately.

**Costs/risks**

- engine defects and intentional changes are harder to distinguish;
- gameplay-design scope expands before the runtime foundation produces evidence;
- significantly more owner decisions are required now;
- weaker controlled baseline for regression/parity analysis.

### Option C — simultaneous Reference + Evolved

Launch both profile families together.

**Benefits**

- offers player choice immediately;
- demonstrates the shared-engine profile architecture visibly.

**Costs/risks**

- highest content, QA, support and release complexity;
- population fragmentation from day one;
- doubles product questions before the first complete native slice exists;
- every profile-sensitive persistence/content/client bug becomes harder to diagnose.

### RECOMMENDATION — owner decision required

**Reference-first for the first externally evaluated build; Evolved follows after the reference vertical slice and parity evidence are stable.**

This is recommended because the current dominant project risk is not lack of ideas; it is the gap between mature architecture and missing native runtime evidence. Reference-first minimizes common-mode ambiguity while preserving the already accepted future Evolved product family.

This recommendation must not be interpreted as `GAME-VISION-01 = ACCEPTED` until the owner explicitly selects or replaces it.

## 5. Key decision B — reference parity target policy

### Option A — pinned immutable baseline

Each Reference release targets one named Global Tibia version/date/behavior evidence revision until a later explicit Reference profile revision upgrades it.

**Benefits:** deterministic tests, reproducible bug reports, controlled content migration, clear player disclosure.

**Risk:** may lag live Global Tibia.

### Option B — continuously tracking live target

Reference behavior is continuously updated as Global Tibia changes.

**Benefits:** strongest current-live parity story.

**Risks:** moving oracle, test instability, continual content workload, migrations become routine, hard to reproduce historical parity claims.

### Option C — hybrid release-train tracking

Continuously observe current Global Tibia, but only materialize changes into immutable Oteryn Reference revisions on an explicit release train.

**Benefits:** keeps current evidence flowing without making production semantics move continuously; each Oteryn revision remains reproducible.

**Risk:** requires a disciplined parity intake/release process.

### RECOMMENDATION — owner decision required

**Hybrid release-train tracking.**

Operationally, every shipped Reference revision should remain pinned and immutable, while evidence collection may continuously observe newer Global behavior. A later accepted upgrade decides what changes are promoted into the next Reference revision.

Do not publicly use an unqualified `1:1` claim. Prefer a scoped claim equivalent to **“Reference profile targeting documented parity with baseline X, with listed intentional/technical/legal differences.”**

## 6. Key decision C — first Evolved differences

The complete Canary/OTS future-gameplay inventory is intentionally **not** a launch checklist. It contains many useful later proposals, but activating many at once would destroy the ability to determine whether the core game and architecture are correct.

### Candidate strategies

#### Strategy 1 — reliability and UX differences first

Prioritize changes that improve trust, usability and recovery while minimally disturbing power/economy balance, for example:

- modern native-client scaling/input/layout/diagnostics;
- explicit reconnect/disconnect safety already accepted by Oteryn architecture;
- better map/discovery/social UX where privacy and PvP rules remain server-authoritative;
- safer transaction/provenance UX without changing conservation rules;
- clearer error/recovery/session/channel state.

#### Strategy 2 — systemic gameplay redesign immediately

Start with death, progression, PvP, boss loot, dynamic spawn capacity, skill progression or economy redesign.

This produces stronger differentiation but requires many `GAME-*`, `DUR-*`, balance and analytics decisions before the basic product proof exists.

#### Strategy 3 — broad feature pack

Launch many custom systems from the historical proposal inventory together.

This is the highest-risk path and conflicts with the architecture programme's vertical-slice bias.

### RECOMMENDATION — owner decision required

Use **Strategy 1 first**, then introduce systemic Evolved changes as isolated, versioned decisions with telemetry/test evidence. Do not make the initial Evolved profile a feature dump.

## 7. Candidate player promise

### RECOMMENDATION — wording concept, not accepted branding

A suitable internal product promise is:

> Preserve the depth, readability and persistent-world identity that make Tibia recognizable, rebuild the experience on a modern reliable native stack, and make every Oteryn difference explicit, measurable and reversible instead of silently drifting from the reference.

This implies three promises rather than one vague `better Tibia` claim:

1. **Recognizable behavior** — Reference semantics have named evidence.
2. **Trustworthy persistence and recovery** — progress/value is protected by authoritative transactions, fencing, audit and tested recovery.
3. **Intentional evolution** — Evolved changes are explicit product revisions, not hidden implementation divergence.

Final public wording and branding remain owner/legal decisions.

## 8. Candidate design pillars and anti-pillars

These are proposed decision aids, not accepted game design.

### Recommended pillars

1. **Server-authoritative skill and fairness** — client sends intent; legality/results remain authoritative.
2. **Persistent-world trust** — character/item/economy state must survive failure without duplication or stale-writer corruption.
3. **Recognizable Tibia interaction grammar** — movement/combat/resource/loot/world interaction should remain understandable to an experienced Tibia player when the Reference profile says it is parity.
4. **Explicit difference over silent divergence** — every Evolved change has a rationale, version and acceptance evidence.
5. **Social/economic continuity** — channels and instances must not casually fragment world identity or multiply valuable eligibility.
6. **Modern operational quality** — low-latency feedback, clear recovery, modern client UX, observability and admin safety are first-class product quality.
7. **Evidence-driven evolution** — balance/economy/world changes consume testing and Game Intelligence evidence, while humans retain product authority.

### Recommended anti-pillars

1. **Generic feature-soup MMO** — reject systems that do not strengthen the chosen Oteryn player promise.
2. **Fork-per-ruleset architecture** — no separate Reference/Evolved engines, clients or protocols.
3. **Convenience that destroys risk/meaning** — quality-of-life changes must be checked for combat, economy and social consequences.
4. **Invisible rule drift** — implementation convenience is not a valid reason for undocumented parity divergence.
5. **Architecture as the product** — a technically elegant subsystem does not justify delaying the next real gameplay proof unless it protects a required invariant.

Monetization/pay-to-win policy is deliberately not inferred here because no owner-accepted monetization contract was found in the current sources.

## 9. Core-loop decision frame

The owner does not need to freeze every activity now, but `GAME-VISION-01` should confirm or replace a minimal loop model so downstream domains aim at the same game.

### Candidate Reference-first loop

```text
moment-to-moment
movement + target selection + combat + resource management + loot

session
choose hunt / quest / boss / trade / social goal
-> prepare
-> play the risk/reward activity
-> secure loot/progress
-> repair/restock/trade/plan next activity

weekly / medium horizon
advance character capabilities
+ unlock/access content
+ pursue economy/equipment goals
+ coordinate party/guild/boss/event objectives

long horizon
master character and content
+ accumulate durable economic/social status
+ tackle harder or rarer goals
+ in Evolved worlds, engage with explicitly versioned expanded systems
```

### UNKNOWN / owner decision

- relative importance of solo versus party progression;
- intended death/loss severity beyond already accepted safety mechanics;
- PvP importance at launch;
- desired progression duration/endgame philosophy;
- whether economy/trade is a central player pillar or supporting system;
- which Oteryn-specific long-term loop differentiates Evolved worlds first.

These must be resolved by `GAME-VISION-01`, `GAME-CHAR-01`, later PvP/economy gates or an explicit owner deferral.

## 10. Success-measure framework

`GAME-VISION-01` should freeze categories and evidence owners now; exact target numbers may remain milestone-specific.

### Product correctness

- percentage/coverage of the selected Reference parity matrix with `PARITY_CONFIRMED`, pending and intentional-difference classifications;
- number/severity of undocumented divergences;
- Evolved changes with deterministic acceptance and rollback evidence.

### Player experience

- time to enter gameplay after launch/login under healthy conditions;
- reconnect/recovery success and player-visible clarity;
- combat/movement responsiveness on supported latency ranges;
- session completion/abandonment reasons;
- party co-location/channel friction once multichannel is enabled.

### Progress and economy trust

- no known item/currency duplication path under accepted failure scenarios;
- recovery tests preserve committed progress and reject stale writers;
- economy inflation/source/sink and suspicious-value metrics once the relevant systems exist.

### Product health

- retention/session-frequency/content-consumption metrics only after privacy-safe instrumentation exists;
- population distribution by world/profile/channel;
- support burden and high-severity incident rate;
- content production throughput once the headless toolchain/Studio becomes measurable.

These measures inform later decisions; analytics never autonomously changes the game.

## 11. Owner decision packet

The minimum owner decisions required to turn this analysis into an accepted `GAME-VISION-01` contract are:

1. **First externally evaluated profile:** Reference-first, Evolved-first or simultaneous.
2. **Reference target policy:** pinned baseline, continuous live tracking or hybrid immutable release revisions.
3. **Internal player promise:** confirm/replace the three-part promise in section 7.
4. **Design pillars/anti-pillars:** confirm/replace the proposed set sufficiently to reject out-of-scope features.
5. **First Evolved strategy:** reliability/UX-first versus early systemic gameplay redesign.
6. **Launch-level PvP importance:** core launch pillar, supported secondary mode, or deferred breadth.
7. **Solo/party emphasis:** choose the intended baseline so `GAME-CHAR-01`, shared XP and content sizing do not guess.
8. **Progression/risk philosophy:** confirm whether Reference is the launch rule oracle and whether Evolved death/progression redesign is a later isolated gate.

Exact formulas, content lists and numeric targets do not need to be selected in this gate unless a downstream contract cannot proceed without them.

## 12. Recommended decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

For the smallest coherent first product proof:

```text
first external build
-> one Reference world profile
-> immutable named parity baseline per released Reference revision
-> continuous evidence intake, explicit upgrade to a later revision
-> one representative area
-> bounded professions/abilities/items/economy
-> one channel
-> modern/reliable client + recovery/persistence quality

then
-> Evolved profile introduced from the same engine/content foundation
-> first differences reliability/UX-focused
-> systemic gameplay redesigns added one accepted gate at a time
-> second channel introduced specifically to prove GAME-CHANNEL-01 semantics
```

This recommendation preserves every accepted foundation decision while maximizing the chance that the next major effort produces interpretable player and engineering evidence.

## 13. Deliberately not decided here

- exact Global Tibia patch/date used as baseline;
- final public names/branding;
- monetization/business model;
- exact profession/vocation roster;
- exact death/progression/economy formulas;
- endgame duration/content breadth;
- final PvP rules;
- first custom boss/progression/economy system;
- transfer/cosmetic/entitlement portability;
- full LiveOps cadence;
- final launch KPI numeric thresholds.
