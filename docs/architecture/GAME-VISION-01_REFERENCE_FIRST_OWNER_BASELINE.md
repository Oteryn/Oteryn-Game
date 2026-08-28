# GAME-VISION-01 — Reference-first Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: first externally evaluated product-profile order only
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: gameplay/content implementation, exact parity-target selection, public branding claims, monetization policy, production rollout or acceptance of the remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's explicit acceptance that Oteryn's **first externally evaluated build is Reference-first**.

This document freezes only the launch/evaluation ordering required to stop future work from treating `Reference-first`, `Evolved-first` and simultaneous Reference+Evolved as equally open alternatives.

It does not close the whole product-vision gate.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

The owner has selected the first product-profile ordering and downstream architecture/product planning must use that selection rather than repeatedly reopening it.

### Concrete downstream work constrained by this baseline

This partial baseline constrains:

- the first externally evaluated `VSL-*` / alpha-facing product slice;
- initial world/profile setup and test fixtures for the first external build;
- parity-evidence planning for the first Reference world;
- early client world/profile disclosure UX;
- the scope assumptions used by `GAME-CHAR-01`, bounded content work and product-sensitive persistence discovery where they need to know which product family is the first evaluation target.

This decision alone does **not** authorize broad gameplay/content implementation and does not make the remaining `GAME-VISION-01` decisions optional.

### What becomes expensive if delayed or contradicted accidentally?

- QA/content/support scope can double by preparing two profile families before the first native runtime is proven;
- defects can become harder to distinguish from intentional Evolved behavior;
- parity fixtures can be designed without a stable first product target;
- persistence/client/content choices can accumulate premature profile-specific branching;
- population and test evidence can become fragmented before one product slice establishes a trustworthy baseline.

### Evidence that could justify a superseding proposal later

A future proposal may reopen the ordering only with explicit evidence such as:

- external playtest evidence that a Reference-first product cannot produce a meaningful evaluation;
- legal/provenance constraints that prevent a usable Reference evaluation target;
- measured development/QA cost showing the selected order materially blocks product proof rather than simplifying it;
- strong product research showing an Evolved-first launch is necessary for the intended audience;
- an explicit later product-owner strategy change.

No implementation convenience or local code preference may silently supersede this owner decision.

## 3. Owner-accepted decision

### USER_SOURCE — accepted 2026-08-11

For the **first externally evaluated Oteryn build**:

1. Oteryn uses **one Reference world/profile** as the evaluation target.
2. Oteryn does **not** launch Reference and Evolved profiles simultaneously for that first external evaluation.
3. The Evolved Oteryn profile remains part of the accepted product strategy, but it follows later from the same canonical engine/client/`protocol-oteryn` foundation after the Reference slice and its parity/runtime evidence are sufficiently stable to make intentional differences interpretable.
4. The first external Reference build must identify an immutable named Reference baseline/revision so test results and parity claims are reproducible.

The exact Global Tibia patch/date/behavior baseline is **not selected by this decision**.

## 4. Relationship to existing architecture

This partial baseline refines the open launch-order question in:

- `ADR-0010-reference-and-evolved-world-product-profiles.md`;
- `PRODUCT_DIRECTION_BASELINE.md`;
- `GAME-VISION-01_PREDECISION_ANALYSIS.md`;
- `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

It does not rewrite those historical/current sources. Where they still present first-launch ordering as open, this dedicated later owner baseline is authoritative for exactly the scope declared here.

The following accepted invariants remain unchanged:

- one canonical Rust engine/workspace;
- one native client;
- one `protocol-oteryn` family;
- Reference and Evolved are versioned product/ruleset/content profiles, not forks;
- distinct logical `WorldId` values across profile families;
- every channel of one logical world inherits the same profile family/revision constraints;
- gameplay value remains world-scoped by default and cross-profile item/currency/character/economy transfer remains forbidden until separately accepted.

## 5. First-build meaning of Reference-first

Reference-first means the first external evaluation optimizes for an **interpretable baseline**, not for simultaneous breadth.

The first external evaluation should therefore remain bounded around:

- one Reference profile;
- one named immutable Reference revision for that evaluated build;
- a representative but intentionally limited gameplay/content surface;
- enough native client/server/persistence/recovery/content-toolchain behavior to produce trustworthy evidence;
- explicit classification of observed parity, pending evidence and intentional/technical/legal differences.

It does not require exhaustive Global Tibia parity before any testing can occur.

## 6. Explicitly unresolved GAME-VISION decisions

This owner acceptance does **not** resolve:

- the exact Global Tibia patch/date/behavior baseline;
- long-term Reference policy: permanently pinned, continuously tracking, or hybrid immutable release revisions with continuous evidence intake;
- final internal/public player promise wording;
- final design pillars and anti-pillars beyond already accepted architectural invariants;
- the first Evolved improvement package;
- launch-level PvP importance;
- solo versus party emphasis;
- progression/death/risk philosophy beyond the decision to use Reference as the first evaluation profile;
- economy source/sink/scarcity goals;
- public names/branding;
- transfer/cosmetic/entitlement portability;
- final success KPI thresholds and LiveOps cadence.

Those remain owner/product decisions under `GAME-VISION-01` and their owning downstream gates.

## 7. Downstream guardrails

Future work must not:

- prepare simultaneous Reference+Evolved first-launch scope by default;
- create separate engines, clients, protocols or repositories for Reference and Evolved;
- treat a Reference implementation defect as an Evolved feature merely to avoid fixing it;
- use an unversioned moving parity target for the evaluated Reference build;
- infer that Evolved is cancelled or permanently deferred;
- infer that all remaining GAME-VISION recommendations were accepted with this decision.

A later Evolved build must branch through explicit versioned profile/ruleset/content differences over the shared foundation, not through a technology fork.

## 8. Acceptance boundary

This document is **binding only for first external profile ordering and the requirement that the evaluated Reference build name an immutable baseline/revision**.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until the remaining product decisions required by the gate are explicitly resolved or deliberately deferred through accepted owner policy.
