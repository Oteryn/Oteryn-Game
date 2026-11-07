# GAME-VISION-01 — Reference Parity Precedence Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: precedence between Reference parity and long-term Oteryn/Evolved gameplay vision; progression/death/risk direction
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: runtime/client/server/content implementation, exact Reference patch/date/revision selection, gameplay formula changes, production rollout or acceptance of remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's explicit clarification that **Reference is the fidelity lane** and `GAME-VISION-01` is the product-direction lane.

For gameplay semantics, a released Reference revision follows its selected named Global Tibia parity baseline unless a separately accepted and explicitly documented Reference difference applies.

`GAME-VISION-01` defines how Oteryn should evolve as a product, especially the long-term Evolved gameplay direction and prioritization, while also defining shared quality and authority expectations that apply to both Reference and Evolved.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

The owner has now clarified the intended relationship between Reference parity and product-vision decisions. Without a canonical precedence rule, downstream work could incorrectly treat product-level choices such as `PvP = secondary pillar` or `solo viable, party rewarded` as authorization to alter Reference mechanics.

### Concrete work blocked until this decision is recorded

This decision is required to close or safely interpret:

- `GAME-VISION-01` owner-decision packet item 8 (`Progression/risk philosophy`);
- the application of the already accepted PvP and solo/party product baselines to Reference worlds;
- `GAME-CHAR-01` progression/death/risk assumptions without accidental Reference divergence;
- later Reference parity fixtures and Evolved difference declarations where a gameplay rule is product-sensitive.

### What becomes expensive if delayed or contradicted accidentally?

- Reference could drift from its selected parity target because a future-facing design preference was mistaken for a Reference rule;
- parity tests could encode contradictory or moving expectations;
- gameplay/persistence contracts could freeze Evolved assumptions into Reference state transitions;
- bug reports could become ambiguous between parity defects and intentional Oteryn design;
- later separation of Reference and Evolved behavior could require migrations or duplicated ruleset logic that should have been explicit from the start.

### Evidence that could justify superseding this policy later

A later proposal may change this precedence only with explicit owner approval and evidence such as:

- a deliberate product strategy that retires or fundamentally redefines the Reference profile family;
- legal/provenance/safety requirements that make a specific parity behavior unacceptable;
- evidence that the current Reference/Evolved product-family model is no longer viable;
- an accepted ADR that explicitly supersedes the Reference parity contract.

Implementation convenience alone cannot supersede this precedence rule.

### Deliberately not decided

This baseline does not choose:

- the exact Global Tibia patch/date/revision used by any Reference release;
- exact PvP, party, shared-XP, progression, death, blessing or loss formulas;
- a concrete Evolved progression/death redesign;
- launch world-type taxonomy;
- channel/PvP switching policy;
- final public wording or branding.

## 3. Owner-accepted precedence

### USER_SOURCE — accepted 2026-08-11

For **Reference**:

1. Gameplay mechanics are reproduced from the selected named Global Tibia baseline.
2. This precedence explicitly includes **solo/party behavior, PvP behavior, progression, death and risk mechanics**.
3. A GAME-VISION product preference cannot silently override a Reference parity rule.
4. Any intentional Reference difference must be separately accepted, explicit, versioned/disclosed where appropriate, and distinguishable from a parity defect.

For **Evolved / long-term Oteryn gameplay direction**:

1. GAME-VISION decisions define intended product emphasis and the direction in which Oteryn may evolve beyond Reference.
2. Intentional Evolved differences must remain explicit, versioned and measurable.
3. A product-direction decision does not automatically activate an implementation or a concrete mechanic.

## 4. What GAME-VISION applies to

`GAME-VISION-01` has two kinds of product rules and they must not be conflated.

### Cross-profile quality and authority rules

These apply to both Reference and Evolved unless a narrower accepted contract says otherwise:

- native reliable client/server quality;
- server authority and anti-cheat boundaries;
- durable progress/value integrity;
- safe recovery and observability;
- explicit/versioned/measurable intentional differences;
- one canonical engine/client/`protocol-oteryn` foundation.

### Gameplay-direction rules

These primarily describe **future Oteryn/Evolved product intent** and product prioritization. They do not rewrite Reference parity.

Examples already accepted at product level:

- `PvP = secondary pillar`;
- `solo viable, party rewarded`;
- first Evolved differentiation = reliability/UX-first;
- future Oteryn progression/death changes, if any, are isolated Evolved decisions rather than silent Reference drift.

## 5. Progression, death and risk direction

### USER_SOURCE — accepted 2026-08-11

The owner accepted the following direction:

1. **Reference progression/death/risk follows the selected Reference parity baseline.**
2. Reference does not receive an Oteryn-specific death/progression redesign merely because GAME-VISION discusses a future target.
3. The first Evolved differentiation package remains reliability/UX-first and does **not automatically include** a progression/death/risk redesign.
4. If Oteryn later changes death penalty, skill/experience loss, blessings/protection loss, progression pacing or related risk semantics in Evolved, that change requires a **separate explicit gate** with versioned rules and measurable acceptance evidence.
5. Exact formulas and numeric values remain intentionally unresolved until their owning gameplay/character/content gates require them.

This closes the product-level progression/risk direction only. It does not accept any concrete redesign.

## 6. Relationship to PvP secondary-pillar baseline

`GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md` remains binding as a long-term product-priority decision:

- PvP is important and supported seriously where enabled;
- PvP must not dominate the entire product direction.

For Reference, however, the selected parity baseline remains authoritative for actual PvP semantics and observable rules. The secondary-pillar decision does not authorize reducing, expanding or redesigning Reference PvP behavior away from that target.

## 7. Relationship to solo viable, party rewarded baseline

`GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md` remains binding as a long-term product-direction decision:

- ordinary meaningful progression should remain viable without permanent party dependence;
- coordinated party play should have real gameplay value.

For Reference, actual party/shared-XP/content mechanics remain whatever the selected parity baseline specifies. The product-direction baseline does not authorize a Reference XP bonus, party-size rule, encounter redesign or other parity deviation.

## 8. Reference release interpretation

Each released Reference revision therefore has a conceptually ordered rule source:

```text
accepted safety/legal/authority constraints
-> selected named Reference parity baseline
-> explicitly accepted documented Reference differences, if any
-> implementation
```

GAME-VISION future gameplay preferences are not inserted into this chain as implicit Reference overrides.

For Evolved, the rule source is instead:

```text
shared Oteryn architecture/quality invariants
-> accepted GAME-VISION product direction
-> explicit Evolved gameplay gates and revisions
-> implementation
```

## 9. Acceptance boundary

This document is binding for:

- Reference parity precedence over future-facing gameplay preferences;
- interpretation of existing PvP and solo/party product baselines;
- Reference-first progression/death/risk direction;
- requirement that later Evolved progression/death/risk redesign be explicit, isolated, versioned and measurable;
- distinction between cross-profile quality rules and Evolved-oriented gameplay direction.

It does **not** select exact gameplay mechanics, numeric formulas, Reference version/date, or authorize implementation.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until its remaining required owner decisions are explicitly resolved or deliberately deferred through accepted policy.
