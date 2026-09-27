# Oteryn Reference Item Artifact Resource Profile v1

- Profile ID: `D6_M1_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE/v1`
- Decision status: **MEASURED AND INDEPENDENTLY REVIEWED; PENDING LIVE EXACT-COMMIT REGISTRATION CHECKPOINT**
- Protection status: **RESOLVE FROM LIVE GITHUB AT PUBLICATION; THIS DRAFT MAKES NO PROTECTION CLAIM**
- Delivery lineage: draft PR #749, candidate head `0e518a4238148d2d50397fee923e893c74d170d9`
- Protected base observed for the final draft refresh: `main@02f52378b32791025d1c1d30356acb7b14185315`
- Runtime authority: **THIS DRAFT GRANTS NONE; RESOLVE LIVE AT PUBLICATION**
- Production acceptance: **PENDING LIVE EXACT-COMMIT REGISTRATION CHECKPOINT**

This draft defines the smallest evidence-backed finite resource profile for the
typed Reference Item successor artifact. It does not accept a wire format,
promote source gameplay values, qualify a Rust implementation or change current
protected behavior.

> **Registration hold:** final independent review accepted the frozen
> compatibility-repaired measurement packet with 81/81 checks passing. It
> includes both the retained `ReferenceItemDefinition` core and the typed
> extension with `G=2` bounded `ProductionKey` group keys. These values are not
> accepted or protected until the live exact-commit registration checkpoint
> completes.

## 1. Timing and acceptance gate

The profile and its registry rows must be accepted in a live coordinator
checkpoint, bound to exact profile, registry, tool and evidence blobs, before
schema/codec mutation is released on PR #749. The later Rust candidate must
mirror the accepted typed wire shape and pass actual encode/decode boundary
tests on its exact head. The Python candidate measurements below are sizing
evidence only.

Acceptance of this profile requires all of:

1. independent review of the frozen reproducibility tool and combined evidence;
2. registry JSON and `python tools/agents/validate_governance.py` passing on the
   exact profile-only candidate;
3. a live `D6_M1_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE/v1 = ACCEPTED`
   checkpoint tied to exact blob hashes;
4. unchanged accepted profile blobs when the later schema/codec head is
   qualified.

## 2. Reproducible evidence

The canonical publication paths are:

- tool: `tools/reference-item-resource-profile/item_resource_profile.py`;
- combined output: `docs/agents/evidence/OTV2-20260922-content-world-item-schema-readiness.json`;
- task: `docs/agents/tasks/active/OTV2-20260922-content-world-item-schema-readiness-504.md`.

Frozen scratch inputs reviewed for this draft:

| Evidence | SHA-256 | Disposition |
|---|---|---|
| portable measurement tool | `73e0c3e009966b3db75a60b39ca53f63648405b8ce794eca73231ff59fdbb6d0` | candidate tool; eventual repository blob must match |
| combined portable output | `c69b7626ca052b5494d14c034848088175f79a662aa7d9e41876d3452917eb78` | candidate measurement, not production acceptance |
| typed candidate tool | `f61dccfc95f9954b4fcba490668d3127e01dc6922b5808b13c5392afeec1fc25` | nested executable codec evidence |
| typed candidate output | `911f2d1799de0cf3941cc061c263d2363ef643118bba34cac5def0743a7ba0fa` | nested sizing evidence |
| equipment-pattern census | `12f3eda9df712e91df44492b9aa71f231cad4e12bcc43d3f2d964027c9336713` | complete 38,157-row candidate census |
| equipment source preflight | `e3055a714e3469c4a52bc5c1828a0a62e862d5c6daca826af728cadfb7166a98` | bounded current-source domain evidence |

Canonical command shape:

```text
python3 tools/reference-item-resource-profile/item_resource_profile.py \
  --repo-root . \
  --source-xml <exact-pinned-read-only-XML> \
  --output docs/agents/evidence/OTV2-20260922-content-world-item-schema-readiness.json
```

The XML input is optional. Without it, the tool retains the frozen source packet
and makes no fresh presentation-atom measurement claim. With it, the output
records the exact input digest.

## 3. Authority and compatibility boundary

- All 38,157 protected native Item identities remain unchanged. This profile
  performs no Crystal/B1 import and no identity generation.
- Existing Reference Item artifact versions v1, v2 and v3 retain their current
  decoding and limits. A typed successor uses a new version discriminator; old
  bytes are never reinterpreted under the successor schema.
- The successor body layout is one shared version discriminator followed by the
  retained `ReferenceItemDefinition` core and then the typed-group extension.
  Server core fields remain `physical_class`, `materializable`, `stack_class`
  and `legal_destinations`; the client retains `physical_class` and
  `stack_class`. The identity-only and materialization guards are unchanged.
- A pure identity-only record has the retained physical/stack core unresolved,
  `materializable=false`, no legal destinations and every typed group
  `UNKNOWN`. A record may retain truthful partial typed facts while that core is
  unresolved; it is then partially described, remains nonmaterializable and is
  not relabelled identity-only. Core `Unknown` never means known false.
- Known stack semantics must agree across the retained core and typed Stack
  group. `StackCapable` with typed `stackable=false`, and `NonStackable` with
  typed `stackable=true`, fail on both encode and decode.
- B1/OTS observations size candidate vocabularies and retain provenance. They do
  not become Reference gameplay truth.
- `UNKNOWN`, `NOT_APPLICABLE`, `CONFLICT`, `KNOWN_ZERO`, `KNOWN_FALSE` and
  `KNOWN_VALUE` remain distinct. Absence never means zero or false.
- Client-safe output is a projection of the same canonical semantic record. It
  has one allowlist and no second parser or Item authority.

## 4. Selected finite bounds

### 4.1 Semantic atoms and collections

| Resource | Maximum | Derivation |
|---|---:|---|
| Item identities / records per projection | 38,157 | protected identity closure; exact structural coverage |
| presentation name | 46 UTF-8 bytes | maximum measured in the exact pinned XML input |
| presentation description | 200 UTF-8 bytes | maximum measured in the exact pinned XML input |
| presentation aliases | 0 entries | explicit unsupported v1; no source vocabulary |
| presentation tags | 0 entries | explicit unsupported v1; no source vocabulary |
| capability truth states | exactly 24 | closed capability vocabulary; fixed shape, not a variable list |
| server capability groups present | 16 | complete closed server group vocabulary |
| client capability groups present | 11 | explicit client allowlist |
| Equipment patterns per Item | 2 | `max(normalized candidate-corpus maximum 1, minimum non-singleton schema witness 2)` |
| additional reserved slots per pattern | 9 | duplicate-free subset after one primary slot from the 10-slot domain |
| mutually exclusive group memberships per pattern | 2 | `max(complete explicit-membership census maximum 0, minimum non-singleton schema witness 2)`; keys reuse the existing namespaced `ProductionKey` grammar and 512-byte bound |
| one Equipment group key | 512 ASCII bytes | inherited canonical `ProductionKey` representation; not the unrelated first-production record ceiling |
| base-vocation entries per Equipment pattern | 5 | exact five-value current base-vocation domain |
| base-vocation entries per TradeRestrictions record | 5 | the same duplicate-free closed domain |
| Weapon elemental entries | 5 | complete admitted pinned elemental candidate vocabulary |
| Protection resistance entries | 12 | complete admitted protection/resistance candidate vocabulary |
| SkillModifier entries | 37 | `55 candidates - 1 unsupported augment - 12 protection - 5 weapon elemental` |
| imbuement gameplay slots | 3 | current-source maximum; gameplay power, not presentation |
| allowed family/tier entries | 20 | complete admitted candidate family vocabulary |
| excluded imbuement families | 20 | duplicate-free subset of the same family vocabulary |
| typed cross-Item target slots | 12 | ten fixed UseTransform kinds plus Temporal decay and write-once targets |

Equipment patterns are canonicalized and ordered by semantic content. Duplicate
semantic patterns are rejected even if their local IDs differ. A third pattern
fails closed; it is not truncated, heuristically merged or converted to
`UNKNOWN`. Evidence is retained and a versioned successor profile is required.

The 37 SkillModifier definitions use closed kind IDs and typed parameter shapes.
Their target-domain and evaluation-phase capacity keys do not assert gameplay
bindings; unresolved bindings remain `UNKNOWN_PENDING_RULESET`. Percent-bearing
values use exact signed numerator / nonzero `u64` denominator rationals.

### 4.2 Record and artifact bytes

| Resource | Maximum bytes | Derivation |
|---|---:|---|
| one server-authoritative Item body | 3,555 | retained server core plus typed extension at every admitted semantic maximum, `P=2`, `G=2` and two 512-byte group keys per pattern |
| one client-safe Item body | 3,433 | retained client core plus the same extension projected through the explicit client allowlist |
| server Item body section | 135,648,135 | `38,157 * 3,555` |
| client Item body section | 130,992,981 | `38,157 * 3,433` |
| manifest | 7,500 | preserved existing Reference carrier maximum |
| index | 40,789,837 | `4 + 38,157 * (1 + 2 + 512 + 2 + 512 + 4 + 4 + 32)` |
| server-authoritative artifact | 176,445,672 | `200 envelope + 7,500 manifest + 40,789,837 index + 135,648,135 body` |
| client-safe artifact | 171,790,518 | `200 envelope + 7,500 manifest + 40,789,837 index + 130,992,981 body` |
| combined generation pair | 348,236,190 | checked sum of the two artifact maxima |

The observed protected index (3,358,044 bytes) and observed manifests (498 and
489 bytes) are scenario measurements only. They are not maxima. The artifact
ceilings use the complete existing carrier grammar with 512-byte key/atom bounds.
The pair bound covers encoded bytes staged together, not process RSS or a generic
allocator budget.

## 5. Projection and unsupported dispositions

Client allowlist:

```text
Presentation, Classification, Physical, Stack, Equipment, Weapon,
Protection, SkillModifiers, Charges, Container, Imbuement
```

Server-only groups rejected by the client decoder:

```text
Temporal, UseTransform, TradeRestrictions, Fluid, ReadableWriteable
```

Explicit unsupported v1 dispositions:

```text
presentation.appearance_binding
presentation.aliases
presentation.tags
equipment.compatibility_rule
modifier.augment_binding
```

Unsupported fields must retain a provenance/loss disposition. They may not be
silently dropped or granted gameplay authority through presentation metadata.

## 6. Required enforcement

The production Rust successor must:

- check artifact, section, record, atom and vector lengths before allocation,
  copy or decompression;
- use checked addition and multiplication for index, body, artifact and pair
  arithmetic;
- reject duplicate keys, duplicate semantic patterns, duplicate vector kinds,
  non-canonical ordering, invalid enums, zero rational denominators, dangling
  accepted Item references and unknown critical groups;
- enforce the client allowlist during both encode and decode;
- publish neither projection when either projection or the checked pair exceeds
  its bound;
- produce deterministic bytes and digests for identical canonical input;
- leave identity-only Items structurally valid without making them automatically
  materializable or gameplay-usable.

Boundary tests must cover every registered maximum and maximum plus one on both
encode and decode where the dimension is serialized. They must also cover
duplicates, non-canonical order, arithmetic overflow, client rejection of every
server-only group and deterministic repeated generation.

## 7. Acceptance meaning

Profile acceptance authorizes implementation against these ceilings. It does
not prove a Rust artifact, source verification, semantic promotion, runtime
consumption, native-client consumption or Item E2E readiness. Those states
advance only from their own exact-head evidence.
