# FIRST_PRODUCTION_CONTENT_PROFILE/v1 — Amendment 03: exact provenance byte grammar and corrected manifest maxima

- Date: 2026-09-09
- Tracking: Issue #433 / PR #462 / CONTENT #54.
- Applies to: `OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md` plus Amendments 01 and 02.
- Normative status: **part of the same `FIRST_PRODUCTION_CONTENT_PROFILE/v1` architecture decision**. This amendment closes the two P2 findings from exact-head review of `2ea46ea118999b2a7da3095a89d2c0ab0aab9317`.
- Authority: unchanged. Architecture implementation authority remains bounded; repository integration remains normal reviewed PR + required checks + FULL Merge Queue; **live deployment / production activation authority remains NONE**.

## 1. Exact package-provenance byte grammar

Amendment 02's phrase `canonical length-prefixed byte encoding` is superseded by the exact grammar below.

All five `PackageManifestBinding` source fields are validated as ASCII before provenance hashing. No Unicode normalization, locale transform, trimming, case folding, NUL termination, delimiter insertion, or host-native integer representation is permitted.

For each field `F`, define:

```text
field(F) = u32_be(byte_length(F)) || exact_ASCII_bytes(F)
```

where:

- `u32_be` is exactly four bytes, unsigned, network/big-endian order;
- `byte_length(F)` is the number of ASCII bytes following that prefix;
- the length itself is not included in `byte_length(F)`;
- the source-manifest digest field is hashed as its already-validated **64 lowercase hexadecimal ASCII bytes**, not decoded to 32 binary bytes;
- `package_key`, `package_revision`, `semantic_schema_version`, and `licensing_metadata` are hashed as the exact validated ASCII byte sequences carried by the typed binding.

The complete provenance preimage is exactly:

```text
field(package_key)
|| field(package_revision)
|| field(semantic_schema_version)
|| field(licensing_metadata)
|| field(source_manifest_digest)
```

There is no prefix, suffix, separator, optional field, map ordering, padding, or extra metadata outside those five length-prefixed fields in `FIRST_PRODUCTION_CONTENT_PROFILE/v1`.

`package_provenance_digest` is:

```text
lowercase_hex(SHA-256(provenance_preimage))
```

and therefore is exactly 64 lowercase hexadecimal ASCII bytes.

Compiler, Content Lock construction/verification, artifact construction, pair staging, authorization matching, restart verification and rollback verification MUST use this byte grammar verbatim. A different prefix width, endianness, decoded-binary digest representation, field order, normalization, or additional byte is an incompatible profile and fails closed.

Mandatory deterministic conformance vectors for the implementation allocation MUST cover at least:

1. the same five field values produce the same provenance preimage and SHA-256 digest across compiler and loader implementations;
2. changing each field individually changes the preimage and recomputed digest;
3. a one-byte versus multi-byte length distinction cannot alias another field split;
4. replacing source-manifest lowercase-hex ASCII with decoded binary bytes produces a different value and is rejected as the wrong profile implementation;
5. little-endian/native-endian length prefixes are rejected by the canonical-vector tests.

This grammar is only the internal provenance-hash grammar for the bounded first-production bootstrap profile. It does **not** select a permanent World Project/World Bundle serializer, source-manifest physical format, signing format, signing topology, or CDN protocol.

## 2. Corrected manifest maximum

Amendment 02 correctly establishes exactly 20 manifest fields, but its worst-case byte arithmetic incorrectly treated the two exact-width digest fields as if they could each consume the generic 512-byte semantic-atom maximum.

For the same conservative first-profile model:

- 18 manifest fields are bounded by the generic `512` ASCII-byte semantic-atom ceiling and therefore consume at most `2 + 512 = 514` encoded bytes each;
- `source_manifest_digest` is fixed at exactly `64` lowercase-hex ASCII bytes and consumes exactly `2 + 64 = 66` encoded bytes;
- `package_provenance_digest` is fixed at exactly `64` lowercase-hex ASCII bytes and consumes exactly `2 + 64 = 66` encoded bytes.

Therefore the exact manifest-section hard maximum is:

```text
max_manifest_section_bytes
  = 18 * 514 + 2 * 66
  = 9,384 bytes
```

This supersedes Amendment 02's `10,280`-byte manifest result. Manifest field count remains exactly `20`.

The server/client body limits and fixed framing are unchanged:

```text
max_server_body_bytes = 4,295,078
max_client_body_bytes = 24,712
fixed_framing_bytes    = 152
```

The corrected projection and pair hard maxima are therefore:

```text
max_server_artifact_bytes
  = 152 + 9,384 + 4,295,078
  = 4,304,614

max_client_artifact_bytes
  = 152 + 9,384 + 24,712
  = 34,248

max_generation_pair_bytes
  = 4,304,614 + 34,248
  = 4,338,862
```

The decoded-field-instance count remains unchanged because there are still exactly 20 manifest fields:

```text
max_decoded_field_instances
  = 2 * 20 + 1,043 * 8 + 6 * 8
  = 8,432
```

The one-section maximum remains `4,295,078` because the server body is still larger than the corrected manifest section.

## 3. Exact `RESOURCE_LIMITS_REGISTRY.json` serialization correction

The post-protected-readback serialized registry update remains mechanically determined by the base decision plus Amendments 01 and 02, with this additional supersession:

- retain Amendment 02's `DUR04-FIRST-PROD-MANIFEST-FIELDS = 20`;
- retain Amendment 02's `DUR04-FIRST-PROD-DECODED-FIELDS = 8432`;
- replace Amendment 02's three artifact/pair objects below by identical ID with the corrected hard maxima here;
- do not retain the superseded `4305510`, `35144`, or `4340654` values anywhere in the resulting registry;
- do not add duplicate IDs.

```json
[
  {
    "id": "DUR04-FIRST-PROD-SERVER-ARTIFACT-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_03",
    "resource": "One server-authoritative bootstrap artifact",
    "unit": "bytes",
    "hard_maximum": 4304614,
    "configurable_range": {"minimum": 1, "maximum": 4304614},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Check server slice length before header parse or input-sized allocation.",
    "client_visible": false,
    "boundary_tests": [
      "4,304,614 bytes accepted only when nested limits pass",
      "4,304,615 rejected before parse/allocation"
    ]
  },
  {
    "id": "DUR04-FIRST-PROD-CLIENT-ARTIFACT-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_03",
    "resource": "One client-safe bootstrap artifact",
    "unit": "bytes",
    "hard_maximum": 34248,
    "configurable_range": {"minimum": 1, "maximum": 34248},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Check client slice length before header parse or input-sized allocation.",
    "client_visible": true,
    "boundary_tests": [
      "34,248 bytes accepted only when nested/client-allowlist limits pass",
      "34,249 rejected before parse/allocation"
    ]
  },
  {
    "id": "DUR04-FIRST-PROD-GENERATION-PAIR-BYTES",
    "owner_contract": "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md+AMENDMENT_03",
    "resource": "Combined server and client-safe artifact bytes in one staging request",
    "unit": "bytes",
    "hard_maximum": 4338862,
    "configurable_range": {"minimum": 2, "maximum": 4338862},
    "failure_category": "CAPACITY_EXCEEDED",
    "allocation_impact": "Use checked addition of the two projection-bounded inputs before staging either as a generation pair.",
    "client_visible": false,
    "boundary_tests": [
      "4,338,862 cumulative bytes accepted only when both projection maxima pass",
      "4,338,863 rejected",
      "pair-byte checked-add overflow rejected"
    ]
  }
]
```

The exact registry serialization algorithm after this amendment is therefore:

1. start with the base-decision §6 objects;
2. insert Amendment 01's two population objects immediately after `DUR04-FIRST-PROD-SPAWNS`;
3. insert Amendment 02's five package/provenance objects immediately after `DUR04-FIRST-PROD-PACKAGES`;
4. apply Amendment 02's replacements for `MANIFEST-FIELDS = 20` and `DECODED-FIELDS = 8432`;
5. apply Amendment 03's replacements for server artifact `4304614`, client artifact `34248`, and generation pair `4338862`;
6. preserve every other non-superseded base/Amendment 01/Amendment 02 row exactly once;
7. reject duplicate IDs and any superseded numeric value during registry conformance testing.

No owner choice remains in that serialization.

## 4. CONTENT #54 allocation/test correction

The eventual fresh CONTENT #54 allocation additionally requires:

- one shared canonical provenance-byte encoder or equivalently proven byte-identical compiler/loader implementations using the §1 grammar;
- golden conformance vectors for all five fields and exact SHA-256 lowercase-hex output;
- compile/staging/restart/rollback tests proving a noncanonical prefix width, byte order, field representation, or field order cannot validate;
- arithmetic conformance asserting:
  - manifest fields `20`;
  - manifest max `9384` bytes;
  - server artifact max `4304614` bytes;
  - client artifact max `34248` bytes;
  - pair max `4338862` bytes;
  - decoded field instances `8432`;
- max+1 denial at `4304615`, `34249`, and `4338863` before peer/input-sized allocation or staging.

The earlier Amendment 02 implementation-test item that quoted `4305510`, `35144`, or `4340654` is superseded by these exact values.

## 5. Activation, recovery, coexistence and authority impact

No activation, rollback, restart, coexistence or authority semantic changes.

The corrected provenance digest grammar is validated before a pair can become `staged`. A grammar/digest mismatch therefore cannot reach the atomic activation commit point and leaves the current active generation unchanged.

The smaller corrected artifact/pair maxima strengthen the existing fail-closed pre-allocation boundary; they do not increase any accepted resource budget.

Live deployment / production activation authority remains **NONE**.

## 6. Criterion-13 state

This amendment moves PR #462 to a new exact head. Review and CI evidence for `a1b3fea3...`, `694ac89d...`, and `2ea46ea1...` is historical after this commit.

Before #433 can become `completed`, the new exact PR head must again pass:

1. independent architecture/security review with P0=0, P1=0, P2=0;
2. applicable canonical CI/governance;
3. normal FULL Merge Queue;
4. protected-main readback.

Until then #433 remains open.