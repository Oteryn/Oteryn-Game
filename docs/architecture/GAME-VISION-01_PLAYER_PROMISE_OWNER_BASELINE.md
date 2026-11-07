# GAME-VISION-01 — Player Promise Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: internal product/player promise and product-identity filter only
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: gameplay/content implementation, public marketing copy, exact gameplay formulas, branding, monetization, production rollout or acceptance of the remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's acceptance of the following internal product foundation for Oteryn:

> Preserve the depth, readability and persistent-world character that make Tibia recognizable; rebuild the experience on a modern, reliable native stack; and make every Oteryn difference from Reference explicit, versioned and measurable.

This is an internal product promise and architecture/design filter. It is not final public marketing wording and does not require copying proprietary implementation, data or assets.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

`Reference-first` and hybrid Reference tracking are already owner-accepted. The project now needs a stable product-identity filter so gameplay, client, content, persistence and Evolved proposals do not optimize independently toward incompatible definitions of what Oteryn is.

### Concrete downstream work constrained by this baseline

This partial baseline constrains:

- `GAME-CHAR-01` and later progression/risk decisions;
- `GAME-ITEM-01` and economy-facing item semantics where product meaning matters;
- `DUR-04` content/ruleset tooling and how explicit Reference/Evolved differences are represented;
- client UX priorities and disclosure of Reference versus Evolved behavior;
- parity acceptance and divergence classification;
- selection and review of future Evolved gameplay proposals;
- alpha scope and product-quality acceptance criteria.

It does not authorize those implementations by itself.

### What becomes expensive if delayed or contradicted accidentally?

- the project can drift into a generic feature-soup MMO rather than a recognizable Tibia-derived product;
- technical modernization can accidentally become undocumented gameplay redesign;
- client UX, content and gameplay teams can optimize toward different product identities;
- Evolved proposals can accumulate without a coherent acceptance filter;
- parity defects can be mislabeled as intentional changes;
- later rework can spread across rulesets, persistence, content schemas, tests and player-facing UX.

### Evidence that could justify a superseding proposal later

A later proposal may reopen this promise only with explicit evidence such as:

- strong external playtest/player research showing the intended product identity is wrong or unclear;
- evidence that a listed principle materially prevents a better coherent Oteryn product;
- legal/provenance constraints affecting the Reference promise;
- measurable product evidence from Reference/Evolved releases;
- an explicit later product-owner strategy change.

Implementation convenience alone cannot silently replace this owner decision.

## 3. Owner-accepted product promise

### USER_SOURCE — accepted 2026-08-11

Oteryn is not intended to become a generic MMO that merely happens to share some Tibia-like mechanics.

The accepted internal promise has three parts.

### 3.1 Recognizable Tibia depth and persistent-world identity

Reference and the shared product foundation should preserve the depth, readability and persistent-world character that make Tibia recognizable to an experienced player.

This includes preserving the importance of coherent long-term character/world progression, risk/reward, world interaction, loot/economy, exploration/content goals and social persistence where their detailed rules are later accepted by the appropriate gates.

This statement does **not** freeze exact death rules, progression rates, PvP rules, vocation balance, economy formulas, quest structure, boss mechanics or other detailed gameplay semantics.

### 3.2 Modern native quality is part of the product

Oteryn deliberately rebuilds the experience on the project-owned modern native stack rather than treating architecture as an invisible implementation detail.

Product quality therefore includes, where their owning contracts permit and prove them:

- reliable authoritative server behavior;
- durable progress/value integrity;
- robust reconnect/recovery;
- modern native-client usability;
- clear diagnostics and failure handling;
- scalable multichannel operation without silently fragmenting world identity;
- observability and operational quality that help protect the player experience.

Technical modernization is not permission to silently change Reference gameplay semantics.

### 3.3 Oteryn differences are explicit, versioned and measurable

A behavior that intentionally differs from Reference must not arise from accidental implementation drift.

An intentional Oteryn/Evolved difference must be attributable to an explicit product/ruleset/content revision and must be reviewable against named acceptance evidence appropriate to the change.

`Measurable` means that the project must be able to determine whether the intended change occurred and evaluate its relevant effects using deterministic tests, parity evidence, playtest evidence and/or privacy-safe Game Intelligence telemetry as appropriate. Analytics informs human decisions; it does not autonomously govern gameplay.

Not every difference requires the same metric, and this baseline does not select numeric KPIs.

## 4. Relationship to accepted Reference decisions

This baseline complements:

- `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` — first external evaluation is Reference-first;
- `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md` — upstream evidence may be observed continuously while released Reference revisions remain immutable and changes are explicitly promoted into later revisions;
- `ADR-0010-reference-and-evolved-world-product-profiles.md` — Reference and Evolved remain versioned profiles over one canonical engine/client/`protocol-oteryn` foundation.

Together, these decisions mean:

1. establish an interpretable Reference product first;
2. keep each released Reference revision reproducible;
3. modernize reliability/client/operations without hiding gameplay divergence;
4. evolve Oteryn through explicit, reviewable revisions rather than accidental drift or technology forks.

No accepted Reference decision is superseded by this document.

## 5. Product decision filter

Future product proposals should be challenged against this promise before acceptance.

A proposal is suspicious when it:

- makes Oteryn less recognizable without a deliberate product reason;
- adds complexity or features merely because the engine can support them;
- improves convenience by accidentally removing meaningful risk, progression, economy or social consequences;
- creates invisible divergence between Reference and its named target;
- requires a separate engine/client/protocol fork;
- cannot state how its intended behavior and relevant effects will be validated.

This is a decision filter, not an automatic rejection rule. The owner may explicitly accept a justified Evolved change that departs substantially from Reference.

## 6. Explicitly unresolved decisions

This acceptance does **not** resolve:

- final public-facing marketing wording or branding;
- the complete design-pillars/anti-pillars set beyond this accepted product promise;
- the first Evolved improvement package;
- launch-level PvP importance;
- solo versus party emphasis;
- exact progression/death/risk philosophy;
- economy source/sink/scarcity goals;
- exact Global Tibia baseline for the first Reference revision;
- exact gameplay/content formulas or feature lists;
- monetization/business model;
- final KPI numeric targets and LiveOps cadence.

These remain owner/product or downstream architecture decisions.

## 7. Acceptance boundary

This document is **binding only for the internal product/player promise**:

- preserve recognizable Tibia depth, readability and persistent-world character;
- rebuild on a modern reliable native foundation;
- make intentional Oteryn differences explicit, versioned and measurable.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until its remaining owner decisions are explicitly resolved or deliberately deferred through accepted policy.
