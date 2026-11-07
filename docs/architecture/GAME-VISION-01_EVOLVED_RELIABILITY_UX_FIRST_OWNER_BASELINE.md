# GAME-VISION-01 — Evolved Reliability/UX-First Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: first Evolved differentiation strategy only
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: runtime/client/server/content implementation, an exact Evolved feature list, systemic gameplay redesign, production rollout, branding, monetization or acceptance of remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's explicit acceptance that Oteryn's **first Evolved differentiation strategy is reliability/UX-first**.

The first Evolved package should prioritize player-visible trust, usability, recovery and clarity while deliberately avoiding a first package dominated by systemic changes to combat power, progression, death, PvP, boss rewards or economy.

This decision selects the **ordering and character of the first Evolved package**. It does not select its exact feature inventory or close the whole product-vision gate.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

Reference-first, hybrid Reference tracking and the internal player promise are already owner-accepted. `GAME-VISION-01_PREDECISION_ANALYSIS.md` also identifies the first Evolved strategy as one of the minimum owner decisions required to turn the pre-decision packet into an accepted product contract.

### Concrete downstream work blocked until this decision is recorded

The following work cannot be closed safely while the first Evolved strategy remains ambiguous:

1. **`GAME-VISION-01` minimum owner-decision packet item 5** — the product-vision gate cannot claim that the first Evolved strategy is resolved while reliability/UX-first, immediate systemic redesign and a broad feature pack remain equally valid interpretations.
2. **Scope and acceptance plan for the first Evolved product proof / Evolved portion of a later alpha milestone** — the milestone cannot define a bounded, testable Evolved slice without knowing whether it is proving reliability/UX evolution or simultaneously redesigning progression, death, PvP, rewards and economy.
3. **Prioritization of the first Evolved candidate backlog** — client/recovery/clarity work cannot be separated from systemic gameplay gates unless the project knows which category has owner priority first.

This decision also constrains, but does not by itself block, later Evolved profile/ruleset planning, `DUR-04` representation of explicit profile differences, and evidence design for Evolved changes.

This decision does not authorize implementation by itself.

### What becomes expensive if delayed or contradicted accidentally?

- the first Evolved release can become a broad feature dump rather than an interpretable product experiment;
- engine defects, UX improvements and systemic gameplay redesign can become impossible to separate in player/test evidence;
- several high-cost gameplay gates may be pulled forward before the Reference/runtime baseline is stable;
- balance and economy changes can create migration/test obligations before the shared native stack has proven reliability;
- the project can undermine its accepted player promise by changing too many meanings at once.

### Evidence that could justify a superseding proposal later

A future proposal may reopen this ordering only with explicit evidence such as:

- external playtests showing a reliability/UX-first Evolved package is too weak to produce useful differentiation evidence;
- product research showing an early systemic gameplay change is necessary to validate the intended audience or product identity;
- measured technical evidence that the proposed reliability/UX improvements are already fully shared by Reference and cannot form a meaningful first Evolved package;
- legal/provenance or platform constraints affecting the candidate first package;
- an explicit later product-owner strategy change.

Implementation convenience or feature enthusiasm alone cannot silently supersede this owner decision.

## 3. Owner-accepted strategy

### USER_SOURCE — accepted 2026-08-11

For the **first Evolved Oteryn differentiation package**:

1. Use a **reliability/UX-first** strategy.
2. Prefer improvements that increase player trust, clarity, usability and recovery while minimally disturbing power, progression and economy semantics.
3. Do **not** make the first package a broad systemic gameplay redesign or historical feature dump.
4. Larger gameplay changes should follow later as isolated, explicit, versioned owner decisions with appropriate test/playtest/analytics evidence.

Examples of the intended strategy class include, where separately designed and accepted by their owning gates:

- modern native-client scaling, input, layout and diagnostics;
- clearer reconnect/disconnect/recovery behavior and presentation;
- clearer session/channel/error/failure state;
- better map/discovery/social UX that preserves server authority, privacy and PvP constraints;
- safer and clearer transaction/provenance UX without weakening conservation rules;
- accessibility/usability improvements that do not silently alter Reference gameplay semantics.

These examples are **not an accepted feature checklist**.

## 4. Shared-foundation rule

Reliability/UX-first must not be misread as permission to make Reference intentionally worse.

Where an improvement is semantically neutral with respect to the selected Reference target—for example crash resistance, rendering/input quality, diagnostics, accessibility, clearer errors or recovery UX—it may belong to the shared native product foundation and may benefit Reference as well.

An improvement becomes an **Evolved-specific difference** only when it intentionally changes player-facing product/ruleset/content behavior beyond the declared Reference contract. Such a difference remains subject to the accepted rule that Oteryn differences are explicit, versioned and measurable.

Therefore:

- shared technical quality is not withheld merely to manufacture profile differentiation;
- Reference parity remains authoritative for gameplay meaning;
- Evolved-specific policy or behavior changes must be named and versioned;
- one engine/client/`protocol-oteryn` foundation remains mandatory.

## 5. What is deliberately not in the first strategy

Absent a separate explicit owner decision, the first Evolved differentiation package does not use the following as its primary scope:

- death-penalty redesign;
- progression-rate or long-term progression redesign;
- major vocation/class power rebalance;
- new PvP rule model;
- systemic boss-reward or loot redesign;
- broad spawn-capacity/economic redesign;
- economy source/sink/scarcity redesign;
- a broad bundle of historical Canary/OTS proposals.

This does not reject those topics permanently. It places them **after** the first reliability/UX-oriented Evolved proof and keeps each one subject to its own gate and evidence.

## 6. Relationship to accepted product decisions

This baseline complements:

- `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` — the first external evaluation is Reference-first;
- `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md` — released Reference revisions remain immutable while newer upstream evidence is explicitly promoted;
- `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md` — modern reliable native quality is part of the product and intentional differences must be explicit, versioned and measurable;
- ADR-0010 — Reference and Evolved remain product profiles over one canonical engine/client/`protocol-oteryn` foundation.

Together they establish this sequence:

```text
Reference proof
-> stable named Reference revision + trustworthy native runtime evidence
-> first Evolved package: reliability/UX-first
-> evaluate the explicit differences
-> later systemic gameplay changes one accepted gate at a time
```

No prior accepted Reference or player-promise decision is superseded by this document.

## 7. Explicitly unresolved decisions

This owner acceptance does **not** resolve:

- the exact first Evolved feature list;
- which reliability/UX improvements are shared across both profiles versus Evolved-specific;
- exact UI/UX design or accessibility requirements;
- reconnect/disconnect timers, grace windows or recovery algorithms;
- first systemic Evolved gameplay difference after the reliability/UX package;
- launch-level PvP importance;
- solo versus party emphasis;
- progression/death/risk philosophy;
- economy source/sink/scarcity goals;
- exact first Global Tibia Reference baseline;
- public branding/marketing wording;
- monetization/business model;
- numeric KPI targets, release cadence and LiveOps policy.

Those remain owner/product or downstream architecture decisions.

## 8. Guardrails

Future work must not:

- interpret reliability/UX-first as automatic approval for any convenience feature;
- weaken server authority, persistence/value integrity, anti-duplication guarantees, privacy or PvP safety for UX convenience;
- silently modify Reference gameplay under the label of modernization;
- intentionally degrade Reference UX to create an artificial Evolved selling point;
- combine the first Evolved package with unrelated systemic balance/economy redesign by default;
- create separate engine/client/protocol forks for Evolved;
- infer that accepting this strategy accepts the exact examples listed above;
- infer that this decision closes all remaining `GAME-VISION-01` questions.

## 9. Acceptance boundary

This document is **binding only for the first Evolved differentiation strategy**:

- reliability/UX-first;
- minimal initial disturbance to gameplay power/progression/economy meaning;
- systemic redesigns later as separate explicit decisions;
- shared semantic-neutral quality improvements may remain shared rather than being artificially Evolved-only.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until its remaining owner decisions are explicitly resolved or deliberately deferred through accepted policy.
