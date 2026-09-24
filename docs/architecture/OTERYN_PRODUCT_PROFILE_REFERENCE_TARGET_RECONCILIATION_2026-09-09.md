# Oteryn Product Profile Scope — Reference Target Reconciliation

- Status: **AUTHORITY RECONCILIATION**
- DecisionStatus: `ACCEPTED_EXISTING_AUTHORITY_RESTATED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Protected source base: `main@0c69d04a49778e539515fb6848b0ab89268c1fa9`
- Scope: reconcile the product-profile scope baseline with the already accepted first Reference target
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Purpose

Protected `main` already contains two accepted facts that must be consumed together:

1. `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` accepts the first Oteryn Reference target as **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**.
2. `OTERYN_PRODUCT_PROFILE_SCOPE_OWNER_BASELINE_2026-09-09.md` correctly separates `SHARED`, `REFERENCE` and `EVOLVED`, but contains several stale sentences that incorrectly describe selection of the first Reference baseline as still pending.

This reconciliation fixes only that authority mismatch. It creates no new product decision and does not change the accepted target cut.

## 2. Existing authority that wins

`GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` is already `OWNER_ACCEPTED` and explicitly states:

```text
first Oteryn Reference target
=
Global Tibia production-observable behavior
AFTER the 2026-07-28 server-save/maintenance change boundary
```

The target is a dated external behavior cut. It is immutable for the first Reference target and is not whatever Global Tibia happens to do later.

Later upstream behavior may be observed continuously, but it enters Reference only through a later explicit Reference revision/promotion decision under the accepted hybrid-tracking policy.

## 3. What remains unresolved

Target selection and evidence completeness are separate.

The following remain legitimately open:

- the final human/public/internal **revision naming syntax** for the accepted cut;
- evidence completeness for individual mechanics at that cut;
- `UNKNOWN` or `CONFLICT` parity cases;
- exhaustive parity-matrix contents;
- runtime/client/server/content implementation;
- later Reference revisions and their promotion cadence.

Therefore the correct fail-closed rule is:

```text
accepted first target cut = 2026-07-28 behavior boundary

for each concrete Reference mechanic:
    PROVEN / OBSERVED / DERIVED -> consume only within its evidence strength
    UNKNOWN / CONFLICT          -> do not guess; affected parity claim remains blocked
```

The project is **not** blocked on selecting the first target again.

## 4. Scoped supersession of the 2026-09-09 product-profile baseline

This document supersedes only the stale target-selection wording in `OTERYN_PRODUCT_PROFILE_SCOPE_OWNER_BASELINE_2026-09-09.md`.

### 4.1 Section 3.2 — Reference gameplay authority

Where the product-profile baseline says the selected immutable Reference revision is the gameplay oracle, interpret the first Reference profile concretely as:

- accepted external target cut: **post-2026-07-28 server-save/maintenance production-observable Global Tibia behavior**;
- final revision label/naming syntax: still deferred;
- behavior-by-behavior evidence: governed by the accepted Reference evidence hierarchy and manifest.

### 4.2 Section 4 — `REFERENCE`

The sentence saying that concrete mechanics remain deferred **until the exact first Reference revision is selected** is superseded.

The correct rule is:

> The first Reference target cut is already selected. Concrete mechanics remain fail-closed only where evidence for the accepted 2026-07-28 cut is `UNKNOWN`, `CONFLICT`, insufficient for the claimed rule, or requires an explicit declared Reference difference.

### 4.3 Section 5 audit matrix

The audit matrix must be read with this additional binding row:

| Existing authority / decision | Scope | Binding interpretation |
|---|---|---|
| `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` | `REFERENCE` | First Reference target is the immutable post-2026-07-28 Global Tibia production-observable behavior cut; evidence completeness is tracked separately. |

### 4.4 Section 13 — implementation discipline

For the first Reference profile, step 3 is satisfied at the target-selection level by the accepted 2026-07-28 behavior cut.

Before implementing a concrete mechanic, work must additionally identify:

- the relevant parity/evidence case for that cut;
- its evidence class and source provenance;
- any explicit accepted Reference difference or safety/legal/technical substitution;
- the profile/ruleset/content revision representation owned by downstream contracts.

A concrete mechanic with insufficient evidence remains blocked even though the target itself is selected.

The sentence saying the exact first Global Tibia Reference baseline remains a separate required decision is superseded and must not be used as a blocker.

### 4.5 Section 14 — acceptance boundary

The statement that the product-profile baseline "does not select the exact first Reference revision" must not be interpreted as reopening target selection.

Correct interpretation:

- #454 did not itself originate the first target decision;
- the target had already been selected by the earlier owner-accepted first-Reference baseline;
- #454 and this reconciliation consume that existing decision;
- only final revision naming syntax and per-behavior evidence completeness remain open.

## 5. Profile scope remains unchanged

No other #454 scope rule changes.

### `SHARED`

Common engine/client/protocol, authority, durability, security, fencing, conservation, recovery, provenance, profile isolation and semantically neutral quality remain shared.

### `REFERENCE`

Player-observable gameplay follows the accepted first Reference target cut and its evidence manifest. An undocumented divergence remains a defect; `UNKNOWN` remains unknown rather than being filled from Evolved design or OTS convenience.

### `EVOLVED`

Explicit Oteryn-designed differences remain Evolved-only unless separately accepted for Reference.

In particular, the #295 Rested/no-delevel/`DeathDebt`/`DeathExhaustion`/corpse-XP-recovery design and the `15 / 45 / 7.5+7.5 / 30 min` candidate remain **EVOLVED-only** and do not affect Reference.

The #447 player-facing hybrid Residence/physical-house product rules remain **EVOLVED** except for independently shared safety/topology/conservation clauses or behavior independently established by Reference parity authority.

## 6. Consequence for current architecture continuation

The prior apparent "select exact first Reference baseline" next step is closed as already satisfied.

The next genuinely open owner-sensitive architecture decision in the current #220 continuation is therefore the **Evolved death/recovery numeric package**, unless a newer live owner decision supersedes it.

Current candidate remains:

```text
NominalDeathPenalty = 15% of achieved LevelXPSpan
DeathDebtCap        = 45% of achieved LevelXPSpan
full-applied split  = 7.5% permanent + 7.5% corpse-recoverable
full recovery window = 30 minutes
```

This reconciliation does **not** approve those numbers. It only removes the false Reference-target blocker so the continuation can return to the actual Evolved owner decision.

## 7. Acceptance boundary

This document performs authority reconciliation only.

It establishes that:

- the first Reference target cut was already owner-accepted;
- the accepted cut is post-2026-07-28 Global Tibia production-observable behavior;
- per-mechanic evidence gaps, not target selection, are the remaining Reference parity blockers;
- #454's `SHARED / REFERENCE / EVOLVED` classification remains binding;
- no runtime or implementation authority is created.

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
