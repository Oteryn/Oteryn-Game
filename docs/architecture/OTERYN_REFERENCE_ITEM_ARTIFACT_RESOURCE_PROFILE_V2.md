# Oteryn Reference Item Artifact Resource Profile v2 (artifact v5)

- Profile ID: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2`
- Artifact profile: `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v5` (compiler and canonicalization `/v5`)
- Decision: ITEM-SEM-2b-3, architect batch `ARCH-BATCH-ITEM-EQUIP-PACKETS-V1` §1.12 and §2.2a, D448
- Status: **CANDIDATE**. Independent contract review on the final frozen head is required.
- Predecessor: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V1.md` (artifact v4). It stays in force for decoding v4.

This profile registers the ceilings of the typed Item grammar that ITEM-SEM-2b-3 adds. It changes
no identity, wire protocol or runtime authority. Use requirements are recorded, not enforced:
RUNE-USE-0 and RANGED-0 own enforcement.

## 1. What changed from v4

| Change | v4 | v5 |
|---|---|---|
| Base-vocation domain | 5 values (Druid 1 … Sorcerer 5) | 6: adds `None` = 6 (A13 key `none`, a character without a vocation) |
| Vocation set per Equipment pattern, TradeRestrictions record and use-requirements group | 5 | 6 |
| Typed groups | 1-16 | 1-17: group 17 `UseRequirements` |
| Client allowlist | 11 groups | 12 groups: adds `UseRequirements` |
| Typed body record version | 2 | 3 |

Group 17 has this layout: `FieldState<u16 min_level>`, then `FieldState<u16 min_magic_level>`,
then `FieldState<base-vocation set>`, then a required one-byte `enforcement_mode`. The only
admitted mode is `ON_USE` (1). Any other value fails closed on encode and decode.

The two additions are a new grammar. v5 therefore uses its own magic, profile version, manifest
profile id, compiler and canonicalization profiles. v4 bytes are never reinterpreted as v5: a
v4 reader rejects vocation 6, group 17 and the v5 body version. A reader also refuses an
artifact whose manifest profile id does not match its header profile.

## 2. Reproducible evidence

- Tool: `tools/reference-item-resource-profile/item_resource_profile.py`. `--profile v5` is the
  default; `--profile v4 --output <path>` reproduces the v4 measurement byte for byte.
- Evidence: `docs/agents/evidence/OTV2-20261003-item-sem-2b3-v5-resource-evidence.json`. A run
  without `--output` rebuilds the evidence and fails on any difference.
- Human summary: `docs/agents/evidence/OTV2-20261003-item-sem-2b3-v5-resource-evidence.md`.

The worst shape is the v4 worst shape plus three things: six vocations in both Equipment
patterns, six trade vocations, and a fully known group 17 with six vocations. The Rust codec's
v5 worst-shape records match the tool's digests exactly (unit test
`typed_v5_body_matches_registered_record_maxima_and_round_trips`).

## 3. Recomputed ceilings

| Resource | v4 | v5 | Derivation |
|---|---:|---:|---|
| Server groups per record | 16 | 17 | closed server group vocabulary |
| Client groups per record | 11 | 12 | explicit client allowlist |
| Equipment vocations per pattern | 5 | 6 | closed six-value domain |
| Trade vocations | 5 | 6 | same domain |
| Use-requirement vocations | — | 6 | same domain |
| Server Item body | 3,555 | 3,577 | +2 pattern vocations, +1 trade vocation, +19 group 17 (3 header + 1 state + 3 + 3 + 8 + 1 mode) |
| Client Item body | 3,433 | 3,454 | +2 pattern vocations, +19 group 17 |
| Server body section | 135,648,135 | 136,487,589 | `38,157 * 3,577` |
| Client body section | 130,992,981 | 131,794,278 | `38,157 * 3,454` |
| Server artifact | 176,445,672 | 177,285,126 | `200 envelope + 7,500 manifest + 40,789,837 index + body` |
| Client artifact | 171,790,518 | 172,591,815 | the same with the client body |
| Generation pair | 348,236,190 | 349,876,941 | checked sum of both artifacts |

The other v1 ceilings are unchanged and apply to v5 as registered under v1. These are: records,
presentation atoms, aliases, tags, capability states, Equipment patterns, reserved slots,
exclusive groups, group key bytes, weapon elements, resistances, modifiers, imbuement limits,
cross-Item targets, and manifest and index bytes.

## 4. Required enforcement

v1 §6 applies unchanged. In addition, every vocation vector and group count is checked against
the profile of the artifact being read, before any allocation. The registry rows prefixed
`DUR04-REFERENCE-ITEM-PROFILE-V2-` carry the max and max+1 boundary tests. The Rust unit tests
`typed_v5_*`, `use_requirements_*`, `vocation_none_and_use_requirements_exist_only_in_v5` and
`compiler_writes_only_v5_and_v4_artifacts_still_decode_under_their_profile` exercise those tests.
