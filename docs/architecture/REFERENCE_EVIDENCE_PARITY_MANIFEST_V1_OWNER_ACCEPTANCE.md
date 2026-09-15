# Reference Evidence/Parity Manifest v1 — Owner Acceptance and Pin

- Status: **OWNER-ACCEPTED**
- Date: 2026-08-14
- Parent owner: `GAME-VISION-01`
- Acceptance issue: #251
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

The already-delivered Reference evidence/parity manifest package is accepted for the immutable first Reference target without rewriting its historical candidate contract.

The accepted semantic pin is:

```text
reference_target.id = global-tibia-observable-2026-07-28-post-server-save
schema_version      = 1
manifest_revision   = 2
manifest_status     = ACCEPTED
schema_file         = docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json
schema_blob         = 208506f461231eb3ed8966ae16dade0764eb39b8
manifest_file       = docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
```

Schema v1 is accepted unchanged from the package delivered by PR #220. Manifest revision 2 changes only acceptance/pinning metadata and normative linkage; it does not add a mechanic case or promote Reference evidence/parity.

## Historical supersession boundary

This acceptance preserves `REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` as the historical design contract and supersedes **only** its candidate/not-accepted status and pre-acceptance wording for v1.

It also supersedes **only** the factual status sentence in `GAME-ABILITY-01_REFERENCE_MECHANIC_CATALOGUE_ENTRY_PARITY_FIXTURE_CONTRACT.md` that says the current Reference evidence manifest is still candidate/not accepted. All catalogue identity, coverage, revision, fixture and fail-closed requirements in that contract remain binding.

No other historical architecture text is rewritten by this decision. When an older current-status/handoff document still says to build the manifest, this later accepted contract is authoritative for this exact scope: the manifest exists and is accepted; do not build a duplicate registry.

## What acceptance means

Acceptance freezes the following paper contract:

- one immutable first target: Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary;
- schema version 1 as the normative machine shape for the accepted manifest family;
- independent target-evidence, Oteryn-implementation and parity axes;
- `PROVEN | OBSERVED | DERIVED | UNKNOWN | CONFLICT | DECLARED_DIFFERENCE` evidence classes;
- fail-closed `UNKNOWN` and `CONFLICT` behavior;
- explicit source hierarchy/provenance/legal-review requirements;
- OTS code as hypothesis/inventory input only, never independent proof of Global behavior;
- no parity confirmation from implementation similarity, catalogue presence or source-code/data similarity;
- `PARITY_CONFIRMED` requiring sufficient target evidence, cleared provenance, exact Oteryn implementation revision and passing fixture/test evidence;
- all nine domain inventory entries including Economy;
- RFC 8785 JCS + SHA-256 as the future canonical digest algorithm.

## Empty manifest is intentionally accepted

At this acceptance revision:

```text
cases = []
all domain coverage = NO_MECHANIC_CASES_REGISTERED
```

This is not a deficiency disguised as parity. It is the explicit fail-closed starting state for mechanic-level evidence population.

Acceptance of the registry structure does **not** accept any concrete Reference ability, combat formula, timing, cooldown, targeting rule, condition rule, item behavior, AI behavior, economy behavior or other mechanic.

## Canonical digest boundary

`canonical_digest` remains `null` at accepted manifest revision 2 because digest tooling has not been accepted or executed. This is explicitly permitted by the existing v1 contract/schema and must not be replaced by a hand-computed value.

Until digest tooling is separately accepted:

- paper architecture/evidence references may bind the accepted target ID + schema version + manifest revision and exact repository delivery revision;
- no runtime/release/content consumer is authorized by this decision;
- a future consumer that requires a cryptographic immutable manifest pin must wait for accepted canonicalization/digest tooling or use a separately accepted immutable artifact contract.

The lack of a digest therefore does not weaken evidence semantics and does not silently authorize mutable `latest` lookup.

## Relationship to GAME-ABILITY catalogue parity

The accepted manifest satisfies the structural acceptance/pinning prerequisite identified by the Reference Mechanic Catalogue entry contract. It does **not** satisfy mechanic evidence or implementation parity by itself.

A future mechanic package must still provide explicit manifest cases, provenance, evidence classification, declared behavior-aspect coverage, exact behavior-affecting revisions and bounded passing parity fixtures. Aggregate mechanic parity remains forbidden unless complete declared in-scope case/aspect coverage is confirmed.

## Change control

Material changes after accepted manifest revision 2 must increment `manifest_revision` and append change history. A schema-incompatible change requires a new schema version/file; schema v1 may not be silently reinterpreted in place.

Changes to the immutable first Reference target require a separately owner-approved Reference revision and must not mutate the meaning of the accepted target ID.

Classification promotions require evidence/history updates and may not erase earlier uncertainty/conflict history.

## Required analysis

**Problem:** downstream mechanic-level Reference catalogue/fixture work is blocked by a registry that exists but still self-identifies as candidate, creating ambiguity over whether evidence/parity claims may bind it.

**Constraints:** preserve historical candidate delivery; no mechanic promotion; immutable first target; independent evidence/implementation/parity axes; no invented digest; no runtime authority; no new stable gate ID.

**Options:**

- **A — later owner-acceptance baseline + manifest revision promotion — SELECTED.** Preserves history, creates an explicit machine pin and keeps schema v1 immutable.
- **B — edit the historical candidate contract in place — REJECTED.** Makes historical design/delivery read as if acceptance existed earlier and weakens supersession traceability.
- **C — create a second manifest/schema family — REJECTED.** Duplicates evidence authority and creates immediate drift risk.
- **D — treat delivery merge as implicit acceptance — REJECTED.** Contradicts explicit candidate status and downstream fail-closed gating.

**Trade-off / risk:** a separate acceptance baseline means older status overlays can remain textually stale until their next bounded reconciliation. The mitigation is explicit supersession scope here plus source-hierarchy precedence; no duplicate manifest may be created from those older instructions.

## Decision timing

**Must decide now: YES.** The first reviewed `ABILITY_COMBAT` mechanic evidence cases and parity fixtures need one accepted versioned evidence authority before they can make trustworthy classifications.

**Blocked without it:** mechanic-level evidence population, catalogue-to-manifest binding and any future parity-confirmed claim.

**Harder later:** adding cases against a candidate registry would force later reclassification/migration and make it ambiguous which schema/revision a parity claim actually used.

**Supersession evidence:** schema limitations found by real reviewed mechanic packages; tooling evidence showing the pin is operationally inadequate; provenance/security/legal findings; or an owner-approved later Reference target revision.

## Deliberately not decided

No mechanic cases, formulas, values, evidence sources, captured observations, physical catalogue schema, fixture runner, digest implementation, runtime loader, client UI, protocol, persistence/DDL, Platform behavior or production rollout are accepted here.

## Next paper-only action

After this acceptance is merged and lifecycle-closed, the next bounded architecture/evidence action is to add the first reviewed representative `ABILITY_COMBAT` mechanic evidence case(s) and parity-fixture binding package under the already-defined GAME-ABILITY catalogue contract, with unresolved behavior remaining fail closed and no runtime authority implied.
