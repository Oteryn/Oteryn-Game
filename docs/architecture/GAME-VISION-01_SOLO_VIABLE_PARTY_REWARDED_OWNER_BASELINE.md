# GAME-VISION-01 — Solo Viable, Party Rewarded Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: product-level solo/party emphasis only
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: runtime/client/server/content implementation, shared-XP formulas, party-size limits, party bonus values, vocation-balance changes, mandatory-party content rules, matchmaking, channel/co-location policy, boss/reward eligibility, production rollout or acceptance of remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's explicit acceptance that Oteryn uses a **solo viable, party rewarded** product model.

A player should be able to maintain meaningful ordinary character progression and complete normal play sessions without permanent dependence on a party. At the same time, coordinated party play should provide real reasons to cooperate through advantages such as efficiency, safety, profession synergy and access to more demanding coordinated goals where separately designed and accepted.

This is a product-emphasis decision. It does not define the exact mechanics that make solo viable or party play rewarding.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

`GAME-VISION-01_PREDECISION_ANALYSIS.md` explicitly identifies relative solo/party importance as a minimum owner decision so that `GAME-CHAR-01`, shared-XP design and content sizing do not guess incompatible assumptions.

### Concrete work blocked until this decision is recorded

This owner decision is required to close:

- `GAME-VISION-01` owner-decision packet item 7 (`Solo/party emphasis`);
- the product-level progression assumption consumed by `GAME-CHAR-01` when it defines character progression/lifecycle expectations;
- the product intent that later party/shared-XP and encounter/content-sizing gates must satisfy before they choose exact formulas or requirements;
- milestone acceptance wording for whether ordinary progression must remain demonstrably viable without permanent party dependency.

This decision does **not** itself unblock implementation of party/shared-XP, encounter, boss, channel or vocation mechanics. Those remain subject to their owning gates.

### Downstream work constrained, but not wholly blocked, by this baseline

This baseline constrains:

- `GAME-CHAR-01` progression and character-lifecycle design;
- later party/shared-XP policy and reward design;
- vocation/profession synergy design;
- encounter, hunt, quest and boss content sizing;
- client party/social UX;
- analytics used to compare solo and group progression quality;
- `GAME-CHANNEL-01` only insofar as future co-location UX must not accidentally make ordinary solo progression impossible; exact channel/co-location policy remains owned by `GAME-CHANNEL-01`.

### What becomes expensive if delayed or contradicted accidentally?

- progression systems may assume mandatory parties while content/client planning assumes broad solo access, or the reverse;
- shared-XP and party rewards may be tuned before the product goal they are meant to serve is known;
- vocation roles may be over-specialized until solo play is unintentionally nonviable;
- content may become either trivialized by group scaling or inaccessible without constant group formation;
- telemetry and playtests may measure solo-versus-party performance without a product interpretation for what constitutes a healthy result;
- later corrections can require rebalance of progression, encounters, rewards, professions and economy simultaneously.

### Evidence that could justify a superseding proposal later

A future proposal may reopen this product-level emphasis only with explicit evidence such as:

- playtests showing solo viability removes too much cooperative meaning or party dependence creates unacceptable friction;
- retention/session data showing the selected balance systematically fails the intended audience;
- profession/content evidence showing the model cannot produce meaningful group synergy without damaging ordinary solo progression;
- economy or progression telemetry showing party rewards create unacceptable inflation, power gaps or exploit incentives;
- a deliberate owner strategy change toward a strongly group-centric or strongly solo-centric product family.

Implementation convenience alone cannot silently supersede this owner decision.

## 3. Owner-accepted product position

### USER_SOURCE — accepted 2026-08-11

Oteryn uses the baseline:

**Solo viable, party rewarded.**

This establishes the following product rules:

1. Ordinary meaningful progression must not require a permanent party relationship.
2. A player should be able to choose a normal solo session without the game treating that choice as an invalid way to progress.
3. Coordinated party play must have meaningful reasons to exist rather than being merely cosmetic or socially optional with no gameplay value.
4. Party advantages may be expressed through efficiency, survivability, complementary profession capabilities and access to harder coordinated objectives, but the exact mechanism is not selected here.
5. Oteryn must avoid both extremes: a product in which party play is functionally pointless and a product in which routine play constantly forces group formation.

## 4. Important interpretation boundaries

### `Solo viable` does not mean `everything soloable`

This baseline does **not** require every boss, quest, event, raid, hunt or endgame objective to be completable by one character.

Hard coordinated content may exist and may intentionally reward or require multiple players when separately accepted by its owning content/gameplay gate.

The product requirement is narrower: a player must be able to sustain meaningful ordinary play and progression without permanent dependence on party availability.

### `Party rewarded` does not mean `automatic XP multiplier`

This baseline does **not** select:

- a shared-XP formula;
- a flat party bonus;
- minimum or maximum party size;
- level-range rules;
- vocation-composition requirements;
- loot multipliers;
- boss eligibility multipliers;
- guaranteed higher raw XP/hour for every possible party composition.

Those are later mechanics decisions. They must serve this product intent without breaking progression, economy, fairness or abuse-resistance constraints.

## 5. Reference and Evolved interpretation

### Reference

For a Reference world/revision, observable solo/party mechanics remain governed by the named parity baseline and declared intentional/technical/legal differences.

This product-emphasis decision does not authorize silently modifying Reference shared XP, party bonuses, content requirements or vocation behavior just to force a preferred solo/party ratio.

### Evolved

Evolved may later change party, progression or content mechanics to better serve the accepted `solo viable, party rewarded` direction, but each intentional difference remains explicit, versioned and measurable.

The first Evolved package remains reliability/UX-first; this decision does not pull systemic party/progression redesign into that first package automatically.

## 6. Safety, integrity and anti-abuse guardrails

Neither solo viability nor party rewards may weaken already accepted authority/integrity requirements.

Later mechanics must preserve, where applicable under their owning gates:

- server-authoritative participation, eligibility and reward outcomes;
- durable item/currency/progression conservation;
- anti-duplication and stale-writer protection;
- abuse-resistant reward eligibility;
- privacy-safe analytics;
- explicit rules for any cross-channel/group behavior under `GAME-CHANNEL-01`;
- human product authority over balance changes.

This document does not choose anti-multibox rules, account limits, party-abuse detectors, reward formulas or enforcement thresholds.

## 7. Explicitly unresolved decisions

This owner acceptance does **not** decide:

- exact shared-XP eligibility or formula;
- any numeric party bonus or penalty;
- maximum/minimum party size;
- level-range or vocation-composition rules;
- exact profession synergy mechanics;
- whether specific bosses/quests/hunts require a party;
- solo versus party loot distribution rules;
- personal versus shared loot policy;
- party finder/matchmaking design;
- party leadership/kick/invite permissions;
- cross-channel party co-location, preferred-channel assignment or switching semantics;
- instance ownership/party admission rules;
- multi-account/multibox policy;
- exact solo/party balance KPIs;
- progression duration, death/risk philosophy or economy source/sink policy;
- public branding or marketing wording.

Those remain separate owner/product or downstream architecture decisions.

## 8. Milestone interpretation

Future product proofs should be able to show both sides of the accepted model without requiring full content breadth.

A representative proof may demonstrate that:

- an ordinary character can complete a meaningful progression/session loop solo;
- forming a coordinated party creates a visible, legitimate advantage or enables a more demanding goal;
- the party advantage does not depend on duplicating rewards or weakening server authority;
- ordinary progression does not become blocked merely because no party is available;
- any group-sensitive content explicitly declares its requirements rather than relying on accidental difficulty.

Exact scenarios and numeric acceptance thresholds remain milestone decisions.

## 9. Relationship to accepted product decisions

This baseline complements:

- `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md`;
- `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md`;
- `GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md`;
- ADR-0010 Reference/Evolved profile boundaries.

Together they mean Oteryn first proves a trustworthy Reference experience, preserves meaningful ordinary solo progression, makes coordinated party play worthwhile, supports PvP seriously without making it dominant, and evolves rules only through explicit profile revisions.

## 10. Acceptance boundary

This document is **binding only for product-level solo/party emphasis**:

- solo = viable for meaningful ordinary progression and normal sessions;
- party = materially rewarded as coordinated play;
- not every content item must be soloable;
- routine play must not require constant party formation;
- exact party/shared-XP/content/vocation/channel mechanics remain unresolved.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until its remaining owner decisions are explicitly resolved or deliberately deferred through accepted policy.
