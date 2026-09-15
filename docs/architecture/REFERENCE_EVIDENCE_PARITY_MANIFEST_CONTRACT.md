# Reference Evidence and Parity Manifest Contract

- Status: **CANDIDATE / NOT ACCEPTED**
- Date: 2026-08-13
- Parent owner: `GAME-VISION-01`
- Delivery issue: #219
- Stable gate: **NONE — this contract implements the evidence-manifest obligation of the accepted first Reference baseline**
- Reference target: **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**
- Runtime authority: **NONE**
- ImplementationStatus: **NOT_STARTED**

## 1. Purpose

This contract defines the canonical, versioned evidence/parity registry required by
`GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`. It records what is
known about the immutable first Reference target separately from what Oteryn has
implemented and tested.

The registry is `docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`.
Its normative machine-readable case schema is\n`docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json`. Every\nmanifest revision MUST validate against that exact schema version before acceptance.\n
## 2. Normative distinctions

The following dimensions are independent:

1. **target evidence class**: `PROVEN | OBSERVED | DERIVED | UNKNOWN | CONFLICT | DECLARED_DIFFERENCE`;
2. **Oteryn implementation state**: `NOT_STARTED | PARTIAL | IMPLEMENTED | NOT_APPLICABLE`;
3. **parity status**: `PARITY_CONFIRMED | PARITY_PENDING_EVIDENCE | PARITY_CONFLICT | DECLARED_DIFFERENCE | OUT_OF_SCOPE`.

`PARITY_CONFIRMED` requires both sufficient target evidence and a linked,
passing Oteryn fixture/test on an exact implementation revision. Source-code or
data similarity alone is insufficient.

## 3. Manifest identity and versioning

The manifest has:

- `schema_version`: schema compatibility, initially `1`;
- `manifest_revision`: monotonically increasing positive integer within this target;
- `reference_target.id`: stable project identifier for the immutable target cut;
- `reference_target.effective_boundary_date`: accepted external behavior boundary date;
- `cases`: uniquely keyed parity cases conforming to the v1 case schema.\n
A change to the accepted target requires a separately owner-approved Reference
revision. It must not edit the meaning of the v1 target identifier in place.

Because the exact server-save time is not frozen, revision 1 records only the accepted date and must not synthesize a UTC time.

A future deterministic digest MUST be computed from UTF-8 RFC 8785 JSON
Canonicalization Scheme bytes of the complete manifest after removing only the
top-level `canonical_digest` member. The digest form is
`sha256:<64 lowercase hex>`. Until repository tooling computes and verifies
that value, `canonical_digest` is `null`; no hand-computed digest may be
claimed.

## 4. Case contract

Each case MUST validate against\n`docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json`. The schema\nis normative for field names, types, required fields, nullability, allowed enums\nand fail-closed conditional rules. In particular, it requires explicit scope,\npreconditions, target evidence, Oteryn state, provenance and classification\nhistory; `PARITY_CONFIRMED` requires cleared provenance, evidence, exact revision\nand a passing fixture/test.\n\nEach case contains:\n
- stable `case_id`, `domain`, `title`;
- `scope` and explicit `preconditions`;
- `target.expected_observable_behavior`;
- target `evidence_class`, sources, effective/observed/retrieved dates and uncertainty;
- conflicts with both sides and resolution state when applicable;
- Oteryn implementation state, revision and fixture/test links;
- parity status and any accepted difference reference;
- provenance/legal review state;
- history entries for material classification changes.

Unknown optional facts use JSON `null` or empty arrays as defined by the schema;
they are never omitted to imply success.

## 5. Source hierarchy and provenance

Permitted source types are:

1. `OFFICIAL_PUBLIC` or owner-provided primary capture with provenance;
2. `CONTROLLED_BLACK_BOX_OBSERVATION`;
3. `COMMUNITY_CORROBORATION`;
4. `OTS_HYPOTHESIS_ONLY`;
5. `OTERYN_ACCEPTED_CONTRACT`;
6. `OTERYN_FIXTURE_OR_TEST`.

OTS code is hypothesis/inventory input only and cannot independently raise a
target evidence class to `PROVEN` or parity to `PARITY_CONFIRMED`.

Restricted/leaked material, proprietary source copying and assets without
confirmed rights are prohibited. A provenance state other than `CLEARED`
blocks evidence promotion.

## 6. Time and historical continuity

Post-target observation may support the 2026-07-28 target only when continuity
from the target boundary is evidenced. Patch-note/search absence is not
continuity proof. Later Global behavior is candidate evidence for a later
Reference revision, not an in-place mutation of this target.

All timestamps are RFC 3339 UTC. Date-only historical statements use
`YYYY-MM-DD` and a separate uncertainty note.

## 7. Conflict and fail-closed rules

- Missing proof is `UNKNOWN`, never an inferred default.
- Credible incompatible evidence is `CONFLICT`.
- `UNKNOWN` maps to `PARITY_PENDING_EVIDENCE` for an in-scope exercised case.
- `CONFLICT` maps to `PARITY_CONFLICT`.
- `DECLARED_DIFFERENCE` requires an accepted decision reference.
- A security, integrity, legal or provenance conflict blocks parity promotion and
  may require an explicit safe `DECLARED_DIFFERENCE`.
- Classification promotion must append history; do not rewrite earlier evidence.

## 8. Initial manifest scope

Revision 1 registers the immutable target, enums, provenance policy and a
fail-closed domain inventory for Character, Item, Channel, Content, Simulation,
Ability/Combat, AI/Spawn, World Interaction and Economy. Initial inventory cases make no
mechanic-level parity claim. Mechanic cases are added only from reviewed evidence
packages.

Historical pre-decision deltas may be referenced as discovery evidence, but they
do not become accepted truth merely by entry. In particular, unresolved work on
PR #191 must not be imported as accepted provenance.

## 9. Validation and consumers

Before acceptance:

- parse JSON successfully and validate it against the exact v1 JSON Schema;\n- verify unique case IDs, domain inventory identifiers and enum values;\n- verify every `PARITY_CONFIRMED` case has adequate target sources plus exact
  Oteryn revision and passing fixture/test evidence;
- verify every declared difference links an accepted decision;
- verify conflicts and uncleared provenance cannot be confirmed;
- run repository governance/document checks on the exact unchanged head.

Future runtime, release, content and QA consumers must pin an immutable manifest
revision/digest. This paper contract does not authorize those consumers.

## 10. Explicit exclusions

No runtime/client/server/protocol/persistence/content implementation, database
schema, evidence-capture automation, official-client automation, Platform write,
external-repository write, proprietary asset/code/protocol acquisition,
production rollout or new stable architecture gate is authorized.
