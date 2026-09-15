# FIRST_PRODUCTION_CONTENT_PROFILE/v1 — Amendment 02: package provenance and source-manifest binding

- Date: 2026-09-09
- Tracking: Issue #433 / PR #462 / CONTENT #54.
- Applies to: `OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md` plus Amendment 01.
- Normative status: **part of the same `FIRST_PRODUCTION_CONTENT_PROFILE/v1` architecture decision**. This amendment closes the second exact-head P1 finding from the original PR review: the base profile carried a generic provenance token but did not explicitly bind the package semantic schema version, licensing metadata and source-manifest digest required by DUR-04 §3.
- Authority: unchanged. Architecture implementation is bounded; repository integration remains normal reviewed PR + required checks + FULL Merge Queue; **live deployment / production activation authority remains NONE**.

## 1. Required production package identity

For `FIRST_PRODUCTION_CONTENT_PROFILE/v1`, the typed compiler input MUST contain exactly one `PackageManifestBinding` with these five source identity fields:

1. `package_key`;
2. `package_revision`;
3. `semantic_schema_version`;
4. `licensing_metadata`;
5. `source_manifest_digest`.

The profile additionally derives `package_provenance_digest` from those five fields.

### 1.1 Field semantics and bounds

- `package_key` and `package_revision` retain the base profile's existing key/revision bounds.
- `semantic_schema_version` is one non-empty ASCII semantic metadata atom, hard maximum **512 bytes**.
- `licensing_metadata` is one non-empty ASCII metadata atom, hard maximum **512 bytes**. For this first slice it is an exact policy/expression/metadata token carried as package provenance; this decision does **not** freeze a permanent licensing document format or legal-policy vocabulary.
- `source_manifest_digest` is exactly **64 lowercase hexadecimal ASCII bytes**, representing a 32-byte SHA-256 digest of the immutable upstream source manifest associated with the typed graph.
- `package_provenance_digest` is exactly **64 lowercase hexadecimal ASCII bytes**, representing SHA-256 over the canonical length-prefixed byte encoding of, in this exact order: `package_key`, `package_revision`, `semantic_schema_version`, `licensing_metadata`, `source_manifest_digest`.

SHA-256 is selected only for this bounded bootstrap profile because the existing content seam already uses a 32-byte SHA-256 digest primitive for artifact integrity. This does not select a signing topology, CDN policy or permanent World Project/World Bundle encoding; a later accepted provenance/signing contract may supersede the profile.

The production compiler has no authority to infer, default or omit any of these fields. Empty, malformed, oversize or inconsistent values fail closed.

### 1.2 Content Lock binding

The first-production Content Lock contains exactly **one** resolved package entry because the profile admits exactly one package and excludes package dependencies.

That entry binds:

`package_key -> package_revision + package_provenance_digest`

The Content Lock therefore cannot claim the same package revision while silently substituting another semantic schema, licensing metadata value or source-manifest digest. A second lock entry, floating revision, mutable branch, network-time latest reference or dependency entry is rejected.

The existing Content Lock revision/digest token in the artifact manifest remains distinct from `package_provenance_digest`.

## 2. Compile, projection, staging and authorization validation

### 2.1 Compiler

Before canonical graph lowering, the production compiler MUST:

1. require the five-field `PackageManifestBinding`;
2. enforce all key/string/digest length and syntax bounds before retaining/copying peer-controlled data;
3. verify the binding's `package_key` and `package_revision` exactly match the graph identity;
4. require a non-empty semantic schema version and licensing metadata token;
5. require the source-manifest digest to be exactly 64 lowercase hexadecimal bytes;
6. recompute `package_provenance_digest` from the canonical five-field binding;
7. require the one-entry Content Lock to bind the same package key, package revision and recomputed provenance digest;
8. reject evidence/test/synthetic provenance identities independently of syntactic validity.

The compiler consumes a typed in-memory graph plus this typed package-manifest binding. It still does **not** parse or select a permanent serialized World Project/source-manifest format. The upstream physical source manifest remains outside the first-profile parser surface; its immutable digest is carried and bound into production identity.

### 2.2 Artifact manifest correction

The base decision's production-bootstrap artifact manifest is expanded from 17 to exactly **20 bounded fields**.

Fields 1-13 and 17-20 below preserve the base concepts, while fields 4-6 below add the explicit package binding and the old generic provenance slot becomes the derived package-provenance digest:

1. artifact profile ID;
2. package key;
3. package revision;
4. semantic schema version;
5. licensing metadata;
6. source-manifest digest;
7. world ID;
8. content revision;
9. map revision;
10. ruleset revision;
11. world-policy revision;
12. compiler revision;
13. canonicalization revision;
14. Content Lock revision/digest token;
15. package-provenance digest;
16. SIM profile revision;
17. first-production content profile revision;
18. projection class;
19. durable migration class;
20. required capability profile.

No separate generic provenance field exists in v1 after this amendment: `package_provenance_digest` is the exact package provenance identity used by the first profile.

### 2.3 Pair staging

Before a pair can become `staged`, the loader MUST:

- enforce the revised 20-field manifest and corrected artifact/pair byte ceilings in §4;
- parse the three explicit package-binding fields and the package-provenance digest under their hard bounds;
- recompute `package_provenance_digest` from package key/revision + semantic schema + licensing metadata + source-manifest digest;
- require the recomputed digest to match the manifest;
- require server and client artifacts to carry identical package binding, provenance digest and Content Lock identity;
- require the expected generation/package identity supplied to staging to match those exact values;
- reject any mismatch before derived runtime state or authoritative publication.

Integrity digest success alone is insufficient: an internally self-consistent artifact with the wrong package binding is incompatible and fails closed.

### 2.4 Authorization matching

`AuthorizedContentGeneration` from the base decision MUST bind, in addition to its existing exact artifact/revision/fencing identity:

- `package_key`;
- `package_revision`;
- `semantic_schema_version`;
- `licensing_metadata`;
- `source_manifest_digest`;
- `package_provenance_digest`;
- Content Lock revision/digest token.

Activation rechecks that the staged generation exactly matches all of those values before the atomic pointer publication. The content subsystem still cannot mint this authorization, select a target revision or infer authority from a passing compiler/test/CI result.

Restart and rollback revalidate the same package binding from immutable bytes and require current external authorization. A previously valid package revision cannot bypass changed/missing provenance fields.

## 3. Resource-dimension classification additions

The following are `REQUIRED_NOW` and are part of the complete §5.1 inventory across the decision packet:

| Resource dimension | Unit | Hard maximum | Evidence / derivation | Mandatory boundary behavior |
|---|---:|---:|---|---|
| semantic schema version | ASCII bytes | 512 | base production semantic-atom ceiling; package schema identity is required by DUR-04 §3 | 512-byte valid atom accepted; 513 rejected before retention; empty rejected |
| licensing metadata | ASCII bytes | 512 | base production semantic-atom ceiling; package licensing metadata is required by DUR-04 §3 | 512-byte valid atom accepted; 513 rejected before retention; empty rejected |
| source-manifest digest | lowercase hex ASCII bytes | 64 | exact 32-byte SHA-256 digest representation for this bootstrap profile | exactly 64 valid lowercase hex bytes accepted; 63/65/non-hex/uppercase rejected before retention |
| package-provenance digest | lowercase hex ASCII bytes | 64 | SHA-256 over canonical five-field package binding | exact recomputed 64-byte value accepted; wrong/malformed digest rejected before staging |
| resolved Content Lock entries | entries | 1 | exactly one package and dependencies excluded in v1 | exactly one matching package/provenance entry accepted; 0/2/floating/dependency entries rejected |

The permanent source-manifest physical byte size, source-file count/nesting and licensing-document physical representation remain `DEFERRED_REQUIRES_FUTURE_DECISION`, because the first production compiler consumes a typed graph plus bounded manifest binding and does not parse those physical inputs. Trigger: any production source parser/importer/Studio authoring pipeline or permanent source representation allocation.

Raw legal/license documents, source manifest payloads and network provenance fetches are `EXCLUDED_FAIL_CLOSED` from the runtime artifact/profile; only the bounded metadata identity described above is accepted.

## 4. Corrected manifest/artifact arithmetic

The base body-record and semantic graph arithmetic is unchanged. Only manifest-bearing bounds change.

A semantic text atom remains at most 512 bytes, encoded with a two-byte length prefix. The corrected 20-field manifest therefore has:

`max_manifest_section_bytes = 20 * (2 + 512) = 10,280`.

Server body remains `4,295,078` bytes. Client body remains `24,712` bytes. Fixed framing remains 152 bytes. Therefore the following base maxima are **superseded**:

- `max_server_artifact_bytes = 152 + 10,280 + 4,295,078 = 4,305,510`;
- `max_client_artifact_bytes = 152 + 10,280 + 24,712 = 35,144`;
- `max_generation_pair_bytes = 4,305,510 + 35,144 = 4,340,654`;
- `max_decoded_field_instances = 2 * 20 + 1,043 * 8 + 6 * 8 = 8,432`.

The largest section remains the server body at `4,295,078`, so the base one-section hard maximum does not change.

## 5. Exact `RESOURCE_LIMITS_REGISTRY.json` serialization corrections

The separate post-readback registry PR remains mechanically defined as follows:

1. start with every base-decision §6 registry object;
2. insert the two Amendment 01 spawn-population objects immediately after `DUR04-FIRST-PROD-SPAWNS`;
3. insert the five new package/provenance objects below immediately after `DUR04-FIRST-PROD-PACKAGES`;
4. **replace**, by identical `id`, the five base objects listed in §5.2 with the exact corrected objects below;
5. do not add duplicate IDs and do not change any other existing registry entry.

### 5.1 New package/provenance registry objects

```json
[
  {
    "id": "DUR04-FIRST-PROD-SEMANTIC-SCHEMA-VERSION-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Semantic schema version in one first-production package binding",
    "unit": "ASCII bytes",
    "hard_maximum": 512,
    "configurable_range": {"minimum": 1, "maximum": 512},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Reject length/emptiness before retaining or copying package schema identity.",
    "client_visible": false,
    "boundary_tests": ["512-byte valid schema atom accepted", "513-byte schema atom rejected before retention", "empty schema version rejected"]
  },
  {
    "id": "DUR04-FIRST-PROD-LICENSING-METADATA-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Licensing metadata atom in one first-production package binding",
    "unit": "ASCII bytes",
    "hard_maximum": 512,
    "configurable_range": {"minimum": 1, "maximum": 512},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Reject length/emptiness before retaining or copying licensing metadata identity; no raw legal document is loaded by the runtime profile.",
    "client_visible": false,
    "boundary_tests": ["512-byte valid licensing metadata atom accepted", "513-byte atom rejected before retention", "empty licensing metadata rejected"]
  },
  {
    "id": "DUR04-FIRST-PROD-SOURCE-MANIFEST-DIGEST-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Source-manifest digest token in one first-production package binding",
    "unit": "lowercase hex ASCII bytes",
    "hard_maximum": 64,
    "configurable_range": {"minimum": 64, "maximum": 64},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Require exact 64-byte lowercase hex syntax before retaining/copying and before provenance hashing.",
    "client_visible": false,
    "boundary_tests": ["64-byte lowercase hex SHA-256 token accepted", "63/65-byte token rejected", "non-hex or uppercase token rejected"]
  },
  {
    "id": "DUR04-FIRST-PROD-PACKAGE-PROVENANCE-DIGEST-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Derived package-provenance digest token",
    "unit": "lowercase hex ASCII bytes",
    "hard_maximum": 64,
    "configurable_range": {"minimum": 64, "maximum": 64},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Recompute from the canonical five-field package binding and reject malformed or mismatching digest before staging.",
    "client_visible": false,
    "boundary_tests": ["exact recomputed 64-byte digest accepted", "wrong digest rejected before staging", "63/65/non-hex digest rejected"]
  },
  {
    "id": "DUR04-FIRST-PROD-CONTENT-LOCK-ENTRIES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Resolved package entries in the first-production Content Lock",
    "unit": "entries",
    "hard_maximum": 1,
    "configurable_range": {"minimum": 1, "maximum": 1},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Accept exactly the single package key/revision/provenance binding; reject extra/floating/dependency entries before lowering/staging.",
    "client_visible": false,
    "boundary_tests": ["one exact package/provenance lock entry accepted", "zero or two entries rejected", "floating/dependency lock entry rejected"]
  }
]
```

### 5.2 Exact replacements for base registry objects

```json
[
  {
    "id": "DUR04-FIRST-PROD-MANIFEST-FIELDS",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Manifest fields per first-production artifact",
    "unit": "fields",
    "hard_maximum": 20,
    "configurable_range": {"minimum": 20, "maximum": 20},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Require exactly the versioned 20-field manifest before retaining metadata.",
    "client_visible": false,
    "boundary_tests": ["20 profile-defined fields accepted", "19 or 21 rejected as incompatible profile"]
  },
  {
    "id": "DUR04-FIRST-PROD-SERVER-ARTIFACT-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "One server-authoritative bootstrap artifact",
    "unit": "bytes",
    "hard_maximum": 4305510,
    "configurable_range": {"minimum": 1, "maximum": 4305510},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Check server slice length before header parse or input-sized allocation.",
    "client_visible": false,
    "boundary_tests": ["4,305,510 bytes accepted only when nested limits pass", "4,305,511 rejected before parse/allocation"]
  },
  {
    "id": "DUR04-FIRST-PROD-CLIENT-ARTIFACT-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "One client-safe bootstrap artifact",
    "unit": "bytes",
    "hard_maximum": 35144,
    "configurable_range": {"minimum": 1, "maximum": 35144},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Check client slice length before header parse or input-sized allocation.",
    "client_visible": true,
    "boundary_tests": ["35,144 bytes accepted only when nested/client-allowlist limits pass", "35,145 rejected before parse/allocation"]
  },
  {
    "id": "DUR04-FIRST-PROD-GENERATION-PAIR-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Combined server and client-safe artifact bytes in one staging request",
    "unit": "bytes",
    "hard_maximum": 4340654,
    "configurable_range": {"minimum": 2, "maximum": 4340654},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Use checked addition of the two projection-bounded inputs before staging either as a generation pair.",
    "client_visible": false,
    "boundary_tests": ["4,340,654 cumulative bytes accepted only when both projection maxima pass", "4,340,655 rejected", "pair-byte checked-add overflow rejected"]
  },
  {
    "id": "DUR04-FIRST-PROD-DECODED-FIELDS",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_02",
    "resource": "Decoded manifest/body field instances processed for one staged server+client pair",
    "unit": "fields",
    "hard_maximum": 8432,
    "configurable_range": {"minimum": 1, "maximum": 8432},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Track cumulative decoded fields with checked arithmetic; nested record/profile checks occur before each retained field.",
    "client_visible": false,
    "boundary_tests": ["8,432 fields accepted only when all nested limits pass", "8,433rd field cannot be materialized", "cumulative counter overflow rejected"]
  }
]
```

## 6. CONTENT #54 allocation/test corrections

The post-registry CONTENT #54 allocation additionally requires production-only symbols equivalent to:

- `PackageManifestBinding`;
- `SemanticSchemaVersion`;
- `LicensingMetadata`;
- `SourceManifestDigest`;
- `PackageProvenanceDigest`;
- a one-entry `ContentLockBinding` validator;
- canonical package-provenance digest computation;
- exact package-binding fields in artifact expectation/staging/authorization matching.

The base mandatory test list is extended with:

23. package manifest binding missing semantic schema, licensing metadata or source-manifest digest fails before lowering;
24. semantic-schema/licensing atoms accept 512 bytes and reject 513/empty before retention;
25. source-manifest and package-provenance digests accept exactly valid 64-byte lowercase hex and reject 63/65/non-hex/uppercase values;
26. changing any of the five package-binding fields changes the recomputed package-provenance digest and deterministic artifact bytes;
27. Content Lock with zero/two/floating/dependency entries or wrong provenance digest fails closed;
28. server/client artifact pair with mismatched semantic schema, licensing metadata, source-manifest digest, provenance digest or Content Lock identity fails before staging;
29. authorization mismatch on any package-binding/provenance field leaves the current active generation unchanged;
30. restart/LKG/rollback revalidates the full package binding and cannot use an older authorized package revision to bypass current provenance checks;
31. conformance arithmetic proves `manifest_fields=20`, `server_artifact=4305510`, `client_artifact=35144`, `pair=4340654`, `decoded_fields=8432` exactly match the protected registry.

No source parser, licensing-document parser, signing authority, network provenance fetch, DDL, protocol or live deployment authority is added.

## 7. Security/trust consequences

The package provenance chain for the first slice is now:

```text
immutable upstream source manifest
    -> source_manifest_digest
    -> PackageManifestBinding(package key/revision + semantic schema + licensing metadata + source digest)
    -> package_provenance_digest
    -> one-entry Content Lock
    -> deterministic server/client artifact manifests + artifact integrity digests
    -> validated pair
    -> staged exact generation
    -> external exact AuthorizedContentGeneration match
    -> atomic active pointer publication
```

No arrow grants live authority by itself. Digest integrity does not prove publisher authenticity, and this amendment deliberately does not select signing topology.

Malformed/missing/oversize package provenance fails before staging. Unknown critical provenance fields or capabilities remain fail-closed; v1 has no optional provenance extension map that could silently add unbounded metadata.

## 8. Criterion-13 state

This amendment moves PR #462 to a new exact head. Review/CI evidence from `a1b3fea3...` and `694ac89d...` is historical after this commit.

Before #433 can become `completed`, the new exact PR head must pass independent architecture/security review with P0=0/P1=0/P2=0, applicable canonical CI/governance, normal FULL Merge Queue and protected-main readback. Until then #433 remains open.