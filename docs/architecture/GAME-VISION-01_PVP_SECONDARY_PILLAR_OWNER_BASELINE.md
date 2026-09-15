# GAME-VISION-01 — PvP Secondary-Pillar Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: launch/product-level importance of PvP only
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: runtime/client/server/content implementation, exact PvP rules, death formulas, world-type selection, channel/PvP switching policy, production rollout or acceptance of remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's explicit acceptance that **PvP is a secondary pillar of Oteryn**.

PvP must be a real, supported and high-quality part of the product where a world/ruleset enables it, but Oteryn is not designed globally around PvP as the dominant purpose of progression, economy, content or social play.

`Secondary pillar` means **important but not product-dominating**. It does not mean low priority for correctness, safety, fairness or exploit resistance.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

The minimum `GAME-VISION-01` contract needs a stable answer to how much product authority PvP has before character lifecycle, death/risk, channel-sensitive design, disconnect handling, content scope and alpha acceptance can be prioritized coherently.

### Concrete work blocked until this decision is recorded

This owner decision is required to close:

- `GAME-VISION-01` owner-decision packet item 6 (`launch-level PvP importance`);
- the product-scope decision for how much PvP breadth the first Reference/Evolved product proofs and later alpha milestones must demonstrate;
- prioritization of PvP-specific backlog work relative to PvE, progression, economy, client/recovery and content slices.

The decision does **not** by itself unblock implementation of PvP mechanics; those remain subject to their owning gameplay, character, persistence, channel, security and recovery contracts.

### Downstream work constrained, but not wholly blocked, by this baseline

This baseline constrains:

- `GAME-CHAR-01` death/risk/lifecycle design where PvP consequences interact with character state;
- later PvP ruleset/world-policy work;
- disconnect/grace/post-grace policy where combat and PvP abuse prevention matter;
- `GAME-CHANNEL-01` product policy where PvP state interacts with channel transitions; this document does not choose switching eligibility, anti-escape semantics or cooldowns;
- combat authority, anti-cheat and exploit analysis;
- content and milestone breadth decisions for PvP-sensitive encounters, areas and world types.

### What becomes expensive if delayed or contradicted accidentally?

- core systems may overfit PvP and impose PvP-driven complexity on every world and player;
- the opposite failure is also possible: PvP may be treated as an afterthought and later require invasive changes to death, combat, persistence, reconnect and channel policy;
- alpha scope can become ambiguous because teams cannot tell whether PvP parity/breadth is mandatory for the next proof;
- PvP-specific exploit prevention may be bolted on after state-transition semantics are already frozen;
- different subsystems may assume incompatible priorities between PvE and PvP.

### Evidence that could justify a superseding proposal later

A later proposal may reopen this product-level priority only with explicit evidence such as:

- player research or playtests showing PvP must be a dominant acquisition/retention pillar;
- population/world evidence showing PvP is materially less important than assumed;
- a deliberate owner strategy change toward a PvP-centric or PvE-only product family;
- evidence that a specific Reference target requires a different product commitment for credible parity;
- legal, platform or safety constraints that materially change feasible PvP support.

Implementation convenience alone cannot silently change this priority.

## 3. Owner-accepted product position

### USER_SOURCE — accepted 2026-08-11

PvP is a **secondary pillar** of Oteryn.

This establishes the following product rules:

1. PvP is a legitimate, durable part of the game rather than a temporary compatibility feature.
2. Where a world/ruleset exposes PvP, it must meet normal Oteryn standards for server authority, consistency, fairness, observability, recovery safety and exploit resistance.
3. PvP must not become the sole organizing principle for the entire product; PvE, progression, economy, exploration/content and social persistence must remain coherent without requiring every player to participate in PvP.
4. The first externally evaluated Reference proof does not need exhaustive PvP-system breadth merely because PvP is a pillar; milestone breadth remains deliberately bounded and must be stated explicitly.
5. Evolved may later alter PvP behavior only through explicit, versioned and measurable product/ruleset decisions.

## 4. Relationship to Reference and Evolved

### Reference

Reference PvP behavior, where included in a selected Reference revision/world ruleset, remains governed by the named parity target and declared intentional/technical/legal differences.

This secondary-pillar decision does not authorize silent modernization of PvP semantics in Reference.

### Evolved

Evolved may later introduce explicitly accepted PvP changes, but this baseline does not choose them.

Any intentional Evolved PvP difference remains subject to the existing product rule that differences are explicit, versioned and measurable and must preserve one canonical engine/client/`protocol-oteryn` foundation.

## 5. Safety and authority guardrails

`Secondary pillar` must not be interpreted as permission to weaken PvP correctness.

PvP-sensitive work must preserve, where applicable under already accepted architecture and future owning gates:

- server-authoritative legality and outcomes;
- durable value/progression integrity and anti-duplication guarantees;
- stale-writer and lease fencing where the owning persistence/session contracts require them;
- anti-cheat and abuse evidence without client authority leakage;
- safe reconnect/disconnect behavior under its owning recovery policy;
- privacy-safe telemetry and human product authority.

**No channel-switch/PvP eligibility rule is accepted by this section.** Combat-lock interaction, anti-escape semantics, switching eligibility and cooldown policy remain owned by `GAME-CHANNEL-01` and related PvP/recovery gates.

This document does not choose timers, formulas or enforcement thresholds.

## 6. Explicitly unresolved PvP decisions

This acceptance does **not** decide:

- exact PvP world types or which world types launch first;
- Open/Optional/Retro-like rules or any equivalent Oteryn taxonomy;
- skull, frag, unjustified-kill or reputation rules;
- war/guild-war mechanics;
- death penalty, blessing/protection-loss or experience/skill-loss formulas;
- PvP damage scaling, vocation balance or combat formulas;
- secure-mode, targeting or client-control details;
- safe zones, protection zones or PvP-area taxonomy;
- disconnect grace duration, post-grace outcomes or logout-abuse policy;
- whether and under what conditions PvP/combat state blocks or restricts channel changes;
- anti-escape semantics for channel changes;
- channel-switch cooldowns, PvP lock durations or anti-hopping thresholds;
- transfer restrictions between PvP/non-PvP world types;
- anti-cheat detector/enforcement policy;
- PvP rewards, ranking, seasons, matchmaking or battleground systems;
- whether the first external Reference proof uses a PvP-enabled world type.

Those require separate owner/product decisions and/or downstream architecture gates.

## 7. Milestone interpretation

Because PvP is secondary rather than core-dominant:

- the first Reference vertical slice may prove combat, death/recovery and authority with bounded PvP coverage rather than full PvP feature parity;
- later milestones must state explicitly what PvP scenarios are in scope and what remains deferred;
- PvP-sensitive invariants that protect safety/integrity cannot be deferred merely because feature breadth is deferred;
- channel-transition PvP scenarios must follow the separately accepted `GAME-CHANNEL-01` policy rather than being inferred from this product-priority baseline.

This is milestone guidance, not implementation authorization.

## 8. Relationship to accepted product decisions

This baseline complements:

- `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md`;
- `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md`;
- `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md`;
- `GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md`;
- ADR-0010 Reference/Evolved profile boundaries.

Together they mean Oteryn first proves a trustworthy Reference product, treats modern reliability as first-class quality, evolves explicitly, and supports PvP seriously without allowing PvP to dominate every product decision.

## 9. Acceptance boundary

This document is **binding only for launch/product-level PvP importance**:

- PvP = **secondary pillar**;
- supported seriously where enabled;
- not the dominant organizing principle of the entire game;
- correctness, fairness, safety and exploit resistance remain first-class;
- exact PvP mechanics, world policies and PvP/channel-transition policy remain unresolved.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until its remaining owner decisions are explicitly resolved or deliberately deferred through accepted policy.
