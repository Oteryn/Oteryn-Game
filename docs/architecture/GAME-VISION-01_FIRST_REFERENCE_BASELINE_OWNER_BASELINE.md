# GAME-VISION-01 — First Reference Baseline Owner Baseline

- Status: **OWNER_ACCEPTED**
- Owner decision date: 2026-08-11
- Owner decision time: 22:09 +02:00
- Repository recording date: 2026-08-11
- Parent gate: `GAME-VISION-01`
- Primary downstream consumer: `GAME-CHAR-01` Stage B
- Source type: `USER_SOURCE`
- ImplementationStatus: **NOT_STARTED**
- Runtime authority: **NONE**
- Does not authorize: GAME-CHAR Stage-B acceptance, runtime/client/protocol/persistence/content implementation, production rollout, proprietary-code/protocol/asset copying or a final Reference revision naming syntax

## 1. Owner source and accepted package

The owner was presented with the complete ten-item recommendation in section 21 of `GAME-VISION-01_FIRST_REFERENCE_BASELINE_DECISION_DOSSIER.md` and explicitly answered:

> Tak

This document accepts that package as one coherent product/evidence baseline.

## 2. Accepted first Reference target cut

The first Oteryn Reference target is:

> **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**, for mechanics/content inside Oteryn's declared Reference parity scope.

This is a dated external **behavior cut**. It is not:

- a proprietary Global server source snapshot;
- a proprietary client binary hash contract;
- a copy of historical market prices, total wealth or player population;
- a Canary/crystalserver revision;
- whatever Global happens to do today;
- a continuously moving upstream target.

## 3. Immutable target semantics

This first Reference target is immutable as an accepted behavior cut.

Later Global Tibia changes:

```text
may be observed
-> may become candidate evidence for a later Reference revision
-> do not silently mutate this accepted target
-> require an explicit later owner-approved Reference revision/promotion decision
```

The accepted hybrid-tracking policy therefore remains binding: observe upstream continuously, but keep an accepted Reference revision reproducible.

## 4. Target date and evidence completeness are separate

Selecting the 2026-07-28 behavior cut does **not** mean every Global rule at that cut is already known.

Every material parity assertion uses explicit evidence classification:

- `PROVEN` — strong primary/public evidence and/or reproducible bounded observation establishes the behavior for the target cut without material conflict;
- `OBSERVED` — target-relevant black-box behavior was directly observed with recorded context, but the full rule/formula is not yet proven;
- `DERIVED` — a rule is explicitly reasoned from evidenced facts and the derivation is recorded;
- `UNKNOWN` — evidence is missing or insufficient; the project stops rather than guessing;
- `CONFLICT` — credible evidence disagrees or describes incompatible states/conditions;
- `DECLARED_DIFFERENCE` — Oteryn intentionally differs through an explicit accepted decision.

An accepted target cut can therefore coexist with `UNKNOWN` mechanics. `UNKNOWN` blocks the affected parity claim/fixture; it is never replaced by implementation convenience.

## 5. Accepted evidence-source hierarchy

### Tier 1 — primary public authority

Prefer official public Tibia/CipSoft news/change announcements, manuals/FAQ/rules/product surfaces and owner-provided primary captures with provenance.

Tier 1 is authoritative evidence where applicable but is **not assumed exhaustive**.

### Tier 2 — controlled black-box observation

Lawful bounded observations of official production gameplay may establish or constrain behavior not specified by public documentation. Evidence should record applicable character/world/ruleset context, inputs, outputs, sample count, timestamps, captures and uncertainty.

A post-target observation proves historical July-28 behavior only when evidence also establishes that the relevant mechanic did not change between the target cut and the observation.

### Tier 3 — reputable public community documentation

Community wikis, guides, calculators, forum research and similar public sources may discover candidate rules, provide historical clues and corroborate stronger evidence. They do not silently become the sole canonical oracle when material uncertainty remains.

### Tier 4 — OTS/reference implementations

Canary, crystalserver and other OTS repositories are hypothesis generators, coverage inventories, migration-discovery inputs and test-case inspiration only.

They are **not proof of Global Tibia behavior** merely because they implement a similar rule, and they never become Oteryn production authority through this baseline.

## 6. Patch-note/search absence is not evidence of absence

The project accepts that official publication may lag or omit production behavior details. The July-16 Echo Raid production change, announced later, is retained as concrete evidence of that risk.

Therefore:

```text
no discovered patch note
!= proof that no production behavior changed
```

The dossier's incomplete proof of post-July-28 chronology remains an evidence limitation, not a reason to move the target silently.

## 7. Evidence manifest requirement

Reference-sensitive work must converge on a versioned evidence manifest or equivalent canonical evidence registry with, where applicable:

```text
reference_target_cut
behavior_id / parity_case_id
domain
scope/profile/world assumptions
evidence_class
source_type
source_locator / evidence artifact id
observed/effective date
retrieval/capture date
input/preconditions
expected observable behavior
confidence / uncertainty notes
conflicts
Oteryn fixture/test linkage when implemented
accepted difference reference when applicable
```

The manifest/revision should eventually receive a deterministic identity/hash under the owning evidence/release tooling contract.

Adding evidence that moves a behavior from `UNKNOWN` to `PROVEN` does not by itself change the accepted external target cut. Intentionally changing target behavior because Global changed later requires a later Reference revision.

## 8. Parity classification rule

Material Reference implementation/release claims should converge on explicit outcomes such as:

- `PARITY_CONFIRMED` — target evidence plus Oteryn fixture/test prove the intended observable behavior;
- `PARITY_PENDING_EVIDENCE` — target is selected but proof is insufficient;
- `PARITY_CONFLICT` — evidence conflict remains unresolved;
- `DECLARED_DIFFERENCE` — explicit accepted Oteryn Reference difference;
- `OUT_OF_SCOPE` — excluded from the bounded Reference release scope.

No `PARITY_CONFIRMED` claim can be inferred merely from code/data similarity with Canary, crystalserver or another OTS.

## 9. Native implementation and provenance boundary

Reference parity specifies evidenced public/observable behavior and implements it natively inside accepted Oteryn architecture.

This baseline does not authorize:

- proprietary server/client source copying;
- distribution of proprietary assets without confirmed rights;
- private/restricted leaked material as canonical evidence;
- importing a proprietary network protocol into `protocol-oteryn` merely for parity;
- laundering uncertain provenance through another OTS repository.

Where evidence provenance is uncertain, the affected rule remains blocked/unknown pending review.

## 10. Security, integrity and legal overrides remain binding

Reference fidelity never requires intentional reproduction of:

- item/currency duplication;
- stale-authority writes;
- corrupt durable state;
- security vulnerabilities;
- unsafe replay/downgrade behavior;
- legally unusable code/assets/data.

Such conflicts require a safe native invariant and, when player-visible parity differs, an explicit `DECLARED_DIFFERENCE` or superseding decision.

## 11. Cross-domain consistency

The accepted 2026-07-28 target is the default first Reference behavior cut for later Reference-sensitive work across:

- `GAME-CHAR-01`;
- `GAME-ITEM-01`;
- combat/abilities and simulation fixtures;
- content/world rules;
- Reference economy source/sink rules;
- parity matrices and release evidence.

A subsystem may not silently use `current Global`, July 13, a convenient wiki revision or an OTS implementation instead.

A different target requires an explicit scoped owner decision or a later Reference revision.

## 12. Consequence for GAME-CHAR Stage B

The exact-target prerequisite is now satisfied.

`GAME-CHAR-01` Stage B is therefore **unblocked for paper-only evidence reconciliation**, but it remains **NOT ACCEPTED**.

Stage B must reconcile, against the accepted July-28 cut and evidence classes above:

- character creation choices and starter state;
- naming namespace/normalization/recycling behavior;
- slot/quota behavior where Reference-visible;
- persistent progression catalogue and exact semantics;
- vocation/class/promotion state;
- death/respawn/progression-loss/blessing/protection behavior;
- offline training/progression where present;
- deterministic formulas/fixtures under their owning simulation/gameplay gates.

The accepted date never fills an `UNKNOWN` rule automatically.

## 13. Consequence for DUR-02

Bounded `DUR-02` discovery may continue using accepted GAME-CHAR Stage-A invariants and the selected target as **question/compatibility context**.

Until Stage B is accepted, DUR-02 must not freeze final character progression/name/death/starter physical structures from unsupported assumptions.

Final character-bearing DUR-02 schema remains blocked on accepted Stage B / full `GAME-CHAR-01` closure.

## 14. Supersession and later revision evidence

This target may be superseded only through an explicit later owner decision. Evidence justifying reconsideration may include:

- a deliberate product decision to create a later Reference revision;
- primary/historical evidence proving the selected production boundary was incorrectly identified;
- legal/provenance constraints;
- security/integrity findings requiring a declared difference;
- release/compatibility evidence showing a different revision strategy is required.

Implementation convenience or availability of an OTS codebase is insufficient.

## 15. Decision-timing discipline

### Must decide now?

**YES.** The exact target was the hard prerequisite for evidence-backed GAME-CHAR Stage B and later cross-domain Reference parity work.

### Downstream work unblocked

- GAME-CHAR Stage-B paper-only evidence reconciliation;
- consistent Reference questions/fixtures across GAME-ITEM, combat/content/economy work;
- evidence-manifest construction against one stable target.

### What becomes harder if changed later

Changing the first target after Stage-B/schema/content decisions would force parity-fixture rewrites, migration/compatibility analysis and potentially destructive semantic corrections. Therefore later movement is an explicit new Reference revision rather than silent mutation.

### Evidence that may justify supersession

Named new Reference-release intent, historical target-identification error, security/legal constraints or other explicit owner-reviewed evidence described above.

### Deliberately not decided

- final Reference revision identifier/naming syntax;
- exhaustive contents of the parity matrix;
- exact Stage-B character rules/formulas until evidenced;
- physical PostgreSQL schema;
- runtime/client/protocol/content implementation;
- automated evidence-capture tooling details;
- future Reference revision cadence;
- Evolved gameplay changes.

## 16. No runtime authority

Acceptance of this owner baseline means:

```text
first Reference target = ACCEPTED
GAME-CHAR Stage B      = UNBLOCKED, NOT ACCEPTED
GAME-CHAR overall      = PROPOSED / PLANNED / NOT_STARTED
runtime implementation = NOT AUTHORIZED
production rollout     = NOT AUTHORIZED
```
