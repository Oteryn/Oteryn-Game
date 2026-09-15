# GAME-ABILITY-01 — Reference Mechanic Catalogue Entry and Parity-Fixture Binding Contract

- Status: **CANDIDATE PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

A Reference Mechanic Catalogue entry is a catalogue-local provenance record binding one concrete mechanic to accepted GAME-ABILITY semantics, the applicable versioned Reference evidence manifest, exact behavior-affecting revisions and bounded parity fixtures. It is not executable content, a runtime identity, protocol discriminator, effect-family definition or independent parity authority. Catalogue presence never proves Reference behavior or parity.

The current `REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` remains `CANDIDATE / NOT ACCEPTED`; this contract does not promote it. A future physical catalogue must pin an accepted manifest/schema revision before parity confirmation is possible.

## Required binding

Each entry must be capable of carrying a catalogue-local mechanic key, immutable Reference target context, manifest case references, a declared parity-coverage matrix, exact behavior-affecting revision references, effect-family composition, owning-domain integration references, parity-fixture references and exact implementation linkage.

The mechanic key is local to catalogue/provenance. It must not become a foundation ID, runtime occurrence ID, protocol opcode, persistence ownership token or cross-domain fence.

## Evidence and parity

The catalogue references/projects classification from the applicable manifest revision; it does not duplicate or override it. No mechanic case is an explicit evidence gap. `UNKNOWN` and `CONFLICT` fail closed. Uncleared provenance blocks promotion. OTS implementations remain hypothesis input only. Later upstream behavior cannot silently rewrite the immutable Reference target.

Parity is **case/scenario scoped**. Every referenced manifest case and every declared behavior aspect participating in a mechanic parity claim must have explicit coverage linked to bounded fixtures/tests.

An entry-level aggregate `PARITY_CONFIRMED`, if a later schema exposes one, is derived only and is permitted only when:

1. an accepted pinned manifest/schema revision exists;
2. the entry declares the complete in-scope coverage matrix for the aggregate claim;
3. every in-scope manifest case is individually eligible for confirmation;
4. every declared behavior aspect required by the aggregate claim is covered by a passing fixture/test on the exact Oteryn revision;
5. every covered scenario pins all required behavior-affecting revisions; and
6. no in-scope case/aspect is unknown, conflicting, uncovered, pending, failed or blocked by provenance/domain-integration gaps.

If any required case/aspect is not confirmed, whole-mechanic aggregate confirmation is forbidden. A happy-path fixture cannot certify unrelated targeting, interruption, cost timing, cooldown, condition, RNG, cross-domain or edge behavior.

## Revision graph

As applicable, entries bind exact revisions for Ability/Mechanic Definition, targeting, legality, cast/channel/interruption, cost/reservation/commit anchors, cooldown/charges, conditions, effect composition, damage/heal formula composition, SIM numeric/RNG semantics and named RNG purposes, DUR-04 script profile/artifact, ruleset/profile and owning-domain integrations. A fixture with a missing required revision binding is invalid; retry/replay may not silently substitute a newer incompatible revision.

## Ownership

Catalogue metadata creates no mutation authority. Effect Families remain governed by GAME-ABILITY. Cross-domain consequences remain with their owning domains; conserved item/currency/loot value remains under `GAME-ITEM` / `DUR-03`. Generic state patches, direct script mutation, hidden event-bus mutation and invented global transaction ownership are forbidden. Missing required cross-domain atomicity keeps dependent parity fail closed.

## Parity fixture

One fixture binds one bounded observable scenario and names its fixture key, mechanic key, manifest cases, declared behavior aspects, Reference target, preconditions, normalized invocation/origin context, exact semantic revisions, expected observable outcomes, exact Oteryn implementation revision, test locator and result.

A fixture result contributes only to explicitly declared case/aspect coverage. Coverage is never inferred from a mechanic name or shared implementation path.

## Non-factual example

```yaml
catalogue_mechanic_key: example.non_reference.training_pulse
manifest_case_refs: []
parity_coverage: []
classification_projection:
  target_evidence: UNKNOWN
  implementation_state: NOT_STARTED
  aggregate_parity_status: PARITY_PENDING_EVIDENCE
revision_bindings: {}
effect_families: []
owning_domain_integrations: []
parity_fixtures: []
```

This is illustrative only, not a real Reference mechanic, evidence case or parity claim.

## Future schema validation

A later physical schema must reject duplicate local keys, unknown manifest references, projections inconsistent with the pinned accepted manifest, aggregate confirmation with incomplete declared coverage, uncovered in-scope cases/aspects, parity claims without exact implementation revision/passing fixtures, missing semantic revision bindings, generic state patches, unowned cross-domain mutation, catalogue keys reused as runtime/protocol/global identity and silent defaults for `UNKNOWN`/`CONFLICT`.

Physical JSON/RON/YAML choice, serializer, indexes, digest tooling and fixture runner remain deferred.

## Required analysis

**Problem:** mechanic-level evidence and fixtures need one binding contract so content/tests cannot be mistaken for Reference truth or over-promote a partially exercised mechanic.

**Constraints:** immutable Reference target; separate evidence/implementation/parity axes; accepted GAME-ABILITY boundaries; domain ownership/conservation; fail-closed unknowns; candidate manifest status; no runtime authority.

**Selected:** thin catalogue index + exact revision/fixture bindings + explicit parity-coverage matrix. Duplicate evidence, executable-content-as-parity-record and unstructured fixture-only registries are rejected because they create duplicate truth, implementation/evidence conflation or weak auditability.

**Trade-off / risk:** explicit coverage and revision bookkeeping adds authoring/tooling cost and can become stale. The mitigation is one canonical manifest authority, exact revision pinning and future schema validation that rejects incomplete aggregate claims rather than allowing optimistic defaults.

**Must decide now: YES.** First reviewed `ABILITY_COMBAT` cases, representative fixtures, Studio linking and later AI/interaction integration need this boundary.

**Blocked without it:** mechanic-level `ABILITY_COMBAT` evidence population, bounded parity fixtures and evidence-driven GAME-ABILITY closure cannot make trustworthy parity claims without duplicating or conflating evidence.

**Harder later:** allowing catalogue/content/test growth first would create duplicated status fields and broad mechanic-level claims whose later correction requires data migration plus a manual audit of which scenarios were actually evidenced and tested.

**Supersession evidence:** representative packages show the graph/coverage model is insufficient or excessive; tooling evidence shows unacceptable authoring cost; replay/provenance failures; repeated cross-domain cases prove another boundary is needed.

## Deliberately not decided

No actual Reference mechanics, exact values/formulas/RNG probabilities, manifest case population, physical schema/IDs, fixture runner, runtime loader, protocol/client UI, persistence/DDL, cross-domain transaction protocol or production rollout are decided here. Unresolved Reference-sensitive behavior remains fail closed.

## Current status

```text
GAME-ABILITY-01 -> REQUIRED_FOR_ALPHA / OPEN
candidate -> local key + manifest refs + declared parity coverage + exact revision graph + bounded fixtures
candidate -> parity is case/scenario scoped; aggregate confirmation requires complete declared coverage
candidate -> catalogue presence never upgrades evidence, manifest acceptance or parity
next -> accept/pin the evidence-manifest revision, then add reviewed ABILITY_COMBAT cases/fixtures without runtime authority
```
