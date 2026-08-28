# GAME-VISION-01 — Hybrid Reference Tracking Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Repository recording date: 2026-08-11
- Gate: `GAME-VISION-01`
- Scope: long-term Reference upstream-tracking policy only
- Source type: `USER_SOURCE`
- Full `GAME-VISION-01` status: **NOT ACCEPTED**
- Does not authorize: gameplay/content implementation, selection of a concrete Global Tibia baseline, release cadence, migration/rollback implementation, public branding claims, monetization, production rollout or acceptance of remaining GAME-VISION decisions

## 1. Purpose

Persist the owner's explicit acceptance that Oteryn Reference follows a **hybrid upstream-tracking model**.

Hybrid means:

1. Global Tibia changes and behavioral evidence may be observed continuously.
2. A released Oteryn Reference revision is immutable as its declared parity target.
3. New upstream behavior does **not** silently mutate that released Reference revision.
4. Upstream changes become Reference semantics only when they are explicitly promoted into a later named Reference revision through a controlled release decision.

This resolves the long-term tracking-policy choice between a permanently static pin, continuously live-mutating tracking and hybrid release-train tracking. It does not select the exact Global target used by any particular Reference revision and does not close the whole product-vision gate.

## 2. Decision timing record

### Must this decision be recorded now?

**YES.**

Reference-first is already owner-accepted. The project therefore needs one stable rule for how parity evidence and later Global changes relate to released Oteryn Reference revisions before parity fixtures, content intake, compatibility and release workflows are designed around incompatible assumptions.

### Concrete downstream work constrained by this baseline

This partial baseline constrains:

- Reference parity-evidence intake and fixture versioning;
- Reference ruleset/content revision identity;
- bug-report and regression reproduction against a named Reference revision;
- later Reference upgrade/release workflow design;
- compatibility and migration discovery for Reference revision changes;
- client/server disclosure of the active Reference revision where a downstream gate requires it;
- alpha/release evidence that must identify the exact Reference revision under test.

This decision does not authorize those implementations by itself.

### What becomes expensive if delayed or contradicted accidentally?

- tests and bug reports can become non-reproducible if their behavioral oracle moves underneath them;
- a live world can drift silently from the baseline it claims to represent;
- content and ruleset changes can be mixed with unrelated hotfixes without a release boundary;
- historical parity evidence can become ambiguous about which behavior was authoritative at the time;
- migrations may be designed as continuous ad-hoc mutation instead of explicit revision transitions;
- developers may repeatedly debate whether every new Global patch must immediately alter Reference.

### Evidence that could justify a superseding proposal later

A later proposal may reopen this policy only with explicit evidence such as:

- measured operational evidence that immutable Reference revisions make parity maintenance impractical;
- player/product research showing that release-train lag materially defeats the intended Reference promise;
- legal/provenance constraints affecting continued upstream observation or parity evidence;
- migration/release evidence showing another model is materially safer and more reproducible;
- an explicit later product-owner strategy change.

Implementation convenience alone cannot silently replace the hybrid model.

## 3. Owner-accepted policy

### USER_SOURCE — accepted 2026-08-11

Reference uses the following model:

```text
Global Tibia changes
        |
        v
continuous observation / evidence intake
        |
        v
analysis + parity classification + validation
        |
        v
explicit promotion decision
        |
        v
new immutable Oteryn Reference revision
```

For example, if `Reference R1` is the active released revision and Global Tibia changes afterward, `R1` remains `R1`. The new behavior is collected and evaluated, but it does not become part of `R1`. When accepted for promotion, it contributes to a later revision such as `R2`.

The exact revision naming format (`R1`, date-based names, semantic versions or another scheme) is not selected here.

## 4. Normative consequences

### 4.1 Continuous observation is not live synchronization

The project may continuously collect lawful behavioral evidence about newer Global Tibia revisions. Continuous observation does not grant those observations product authority over any currently released Reference revision.

### 4.2 Released Reference revisions are immutable parity targets

A named Reference revision fixes the parity target used for its tests, bug reproduction and release evidence. Its declared behavioral target must not change merely because upstream changes later.

A correction that shows the existing Oteryn implementation failed to match the already-declared target is a bug fix against that same revision where compatible; it is not an excuse to redefine the target retroactively.

### 4.3 Upstream promotion is explicit

A newer Global behavior becomes part of Reference only through an explicit later Reference revision decision/release boundary. The promotion must identify what baseline/evidence it consumes and what Reference revision it produces.

This baseline does not require every observed upstream change to be promoted. Inclusion/exclusion and intentional-difference policy remain subject to parity evidence, legal constraints and later product/release decisions.

### 4.4 Historical evidence stays interpretable

Tests, parity matrices, issue reports and release evidence must be able to state which Reference revision they concern. Later revisions do not retroactively change the meaning of evidence collected for earlier revisions.

## 5. Authority and scoped supersession of ADR-0010

`ADR-0010-reference-and-evolved-world-product-profiles.md` remains accepted and authoritative except for the exact deferred policy scope explicitly resolved here.

ADR-0010 §4 (`Reference parity is measured and explicit`) states that the choice between a pinned reference release, a dated behavior baseline or continuous tracking remained deferred to `GAME-VISION-01` or a dedicated parity contract. Its `Deferred decisions` section likewise lists continuous tracking versus pinned Reference worlds as unresolved.

This later owner decision **supersedes only that long-term upstream-tracking-policy deferral**:

- Reference is not permanently frozen to one historical upstream state for all future releases;
- Reference does not live-mutate released semantics whenever Global Tibia changes;
- Reference continuously observes upstream evidence and promotes accepted changes only into later immutable named Reference revisions.

The following ADR-0010 requirements remain fully binding:

- every Reference parity claim names a versioned target;
- even continuously collected evidence must materialize through immutable Reference revisions;
- parity classification remains explicit;
- one canonical engine/client/`protocol-oteryn` and profile isolation remain unchanged;
- exact target definition for a particular Reference revision remains a separate decision.

A Reference revision may still use a date, patch, behavior snapshot or another accepted identifier to name its immutable target. This baseline selects the **tracking lifecycle**, not the exact target-naming scheme or first Global baseline.

No other ADR-0010 decision is superseded by this document.

## 6. Relationship to the Reference-first decision

`GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` establishes that the first externally evaluated Oteryn build is Reference-first and must name an immutable Reference baseline/revision.

This document adds the long-term policy for what happens after that first revision:

- upstream may continue moving;
- evidence intake may continue;
- the released Reference revision stays fixed;
- a later accepted promotion creates a new fixed Reference revision.

Together these two partial owner baselines remove both the first-profile-order ambiguity and the pinned-vs-live-vs-hybrid tracking ambiguity.

## 7. Explicitly unresolved decisions

This owner acceptance does **not** resolve:

- the concrete Global Tibia patch/date/behavior baseline for the first Reference revision;
- how often candidate Reference upgrades are considered or released;
- whether a specific upstream patch is promoted wholly, partially or not at all;
- exact revision naming/version-number scheme;
- migration compatibility classes between Reference revisions;
- maintenance-window, rollback and deployment mechanics;
- support lifetime for older Reference revisions;
- public wording/branding of parity claims;
- final internal player promise and design pillars;
- first Evolved improvement package;
- launch-level PvP importance;
- solo/party emphasis;
- progression/death/risk philosophy;
- economy source/sink/scarcity goals;
- final KPI and LiveOps policy.

These remain owner/product or downstream architecture decisions.

## 8. Guardrails

Future work must not:

- silently mutate a released Reference target in place because Global Tibia changed;
- make production/reference semantics depend directly on an unversioned live upstream state;
- reinterpret older parity evidence using a newer Reference revision;
- claim that continuous observation means automatic production promotion;
- create a separate engine/client/protocol fork for each Reference revision;
- infer that accepting hybrid tracking closes the remaining `GAME-VISION-01` decisions.

Reference revisions remain versioned product/ruleset/content semantics over the same canonical engine, native client and `protocol-oteryn` foundation.

## 9. Acceptance boundary

This document is **binding only for the hybrid Reference upstream-tracking model**: continuous observation plus explicit promotion into immutable named Reference revisions, and it explicitly supersedes ADR-0010's prior deferment of that long-term tracking-policy choice.

`GAME-VISION-01` as a whole remains **NOT ACCEPTED** until its remaining owner decisions are explicitly resolved or deliberately deferred through accepted policy.
