# Oteryn Reference Item Artifact Resource Profile v3 (artifact v6)

- Profile ID: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3`
- Artifact profile: `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v6` (compiler and canonicalization `/v6`)
- Decision: ITEM-SEM-USE-1, `ITEM-SEM-USE-PACKET-1` §1.2-§1.4 and §2.1
  (`reviews/OTERYN_GAME_ITEM_SEM_USE_FOOD_AND_POTION_SEMANTICS_PACKET_2026-10-04.md`)
- Status: **CANDIDATE**. Independent contract review on the final frozen head is required.
- Predecessor: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2.md` (artifact v5). It stays in force for decoding v5.

This profile registers the ceilings of the typed Item grammar that ITEM-SEM-USE-1 adds. It changes
no identity, wire protocol or runtime authority. Consumption semantics are recorded, not executed:
ITEM-USE-1 executes both potion restores and FOOD-REGEN-1 owns the food condition.

## 1. What changed from v5

| Change | v5 | v6 |
|---|---|---|
| Typed groups | 1-17 | 1-18: group 18 `Consumption` |
| Client allowlist | 12 groups | 12 groups, unchanged: group 18 is server-only |
| Typed body record version | 3 | 4 |

Group 18 is one `FieldState` followed, when Known, by a one-byte variant:

- `FOOD` (1): `u16 regeneration_seconds`, 1-1,199.
- `POTION` (2): a `u8` restore count of 1-2. Each restore is a `u8` resource (`HEALTH` 1, `MANA` 2),
  `u16 min` and `u16 max` with `1 <= min <= max <= 10,000`. Restores are strictly ordered by
  resource, so a resource appears at most once and Health precedes Mana. Then
  `FieldState<Item ordinal> empty_flask`, which is `KNOWN` or `NOT_APPLICABLE` only. A Known flask
  must resolve to an Item of the same family, or the encode and the load fail.

An unknown variant, an unknown resource and any value outside these ranges fail closed on encode
and decode. Decoded records are validated before they are returned.

The group is a new grammar. v6 therefore uses its own magic (`OTRPA06\0`), profile version,
manifest profile id, compiler and canonicalization profiles. v5 bytes are never reinterpreted as
v6: a v5 reader rejects group 18 and the v6 body version, and either reader refuses the other
profile's bytes by manifest profile id. The compiler writes only v6; v4 and v5 are written only by
the compatibility tests.

## 2. Reproducible evidence

- Tool: `tools/reference-item-resource-profile/item_resource_profile.py`. `--profile v6` is the
  default. `--profile v5` re-checks the v5 evidence without drift, and `--profile v4 --output
  <path>` reproduces the v4 measurement byte for byte.
- Evidence: `docs/agents/evidence/OTV2-20261004-item-sem-use-1-v6-resource-evidence.json`. A run
  without `--output` rebuilds the evidence and fails on any difference.
- Human summary: `docs/agents/evidence/OTV2-20261004-item-sem-use-1-v6-resource-evidence.md`.

The worst shape is the v5 worst shape plus a fully known Potion: Health 10,000/10,000,
Mana 10,000/10,000 and a Known flask on the last definition ordinal. The Rust codec's v6
worst-shape records match the tool's digests exactly (unit test
`typed_v6_body_matches_registered_record_maxima_and_round_trips`).

## 3. Recomputed ceilings

| Resource | v5 | v6 | Derivation |
|---|---:|---:|---|
| Server groups per record | 17 | 18 | closed server group vocabulary |
| Client groups per record | 12 | 12 | explicit client allowlist, unchanged |
| Potion restores | — | 2 | one per resource, closed two-value domain |
| Server Item body | 3,577 | 3,598 | +21 group 18 (3 header + 1 state + 1 variant + 1 count + 2 × 5 restore + 1 flask state + 4 ordinal) |
| Client Item body | 3,454 | 3,454 | group 18 is not projected |
| Server body section | 136,487,589 | 137,288,886 | `38,157 * 3,598` |
| Client body section | 131,794,278 | 131,794,278 | `38,157 * 3,454` |
| Server artifact | 177,285,126 | 178,086,423 | `200 envelope + 7,500 manifest + 40,789,837 index + body` |
| Client artifact | 172,591,815 | 172,591,815 | the same with the client body |
| Generation pair | 349,876,941 | 350,678,238 | checked sum of both artifacts |

The other v1 and v2 ceilings are unchanged and apply to v6 as registered. These are: records,
presentation atoms, aliases, tags, capability states, Equipment patterns, reserved slots,
exclusive groups, group key bytes, vocation sets, weapon elements, resistances, modifiers,
imbuement limits, cross-Item targets, and manifest and index bytes.

## 4. Required enforcement

v1 §6 and v2 §4 apply unchanged. In addition, the restore count is checked before allocation and
every group count against the profile of the artifact being read. The registry rows prefixed
`DUR04-REFERENCE-ITEM-PROFILE-V3-` carry the max and max+1 boundary tests. The Rust unit tests
`typed_v6_*`, `food_*`, `potion_*`, `consumption_group_exists_only_in_v6_server_records` and
`compiler_writes_only_v6_and_v4_v5_artifacts_still_decode_under_their_profile` exercise those
tests.
