# Oteryn Reference Item Artifact Resource Profile v4 (artifact v7)

- Profile ID: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V4`
- Artifact profile: `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v7` (compiler and canonicalization `/v7`)
- Decision: ITEM-SEM-BED-1, bed item group packet §1.1-§1.4 and §2.1
  (`reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md`)
- Status: **CANDIDATE**. Independent contract review on the final frozen head is required.
- Predecessor: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3.md` (artifact v6). It stays in force for decoding v6.

This profile registers the ceilings of the typed Item grammar that ITEM-SEM-BED-1 adds. It changes
no identity, wire protocol or runtime authority. Bed semantics are recorded, not executed: BED-1
owns sleep, wake and the occupied look, and BED-CONTENT-1 owns the bed facts and their lowering.

## 1. What changed from v6

| Change | v6 | v7 |
|---|---|---|
| Typed groups | 1-18 | 1-19: group 19 `Bed` |
| Client allowlist | 12 groups | 12 groups, unchanged: group 19 is server-only |
| Typed body record version | 4 | 5 |
| Cross-Item target slots | 13 | 15: the bed's two occupied looks |

Group 19 is one `FieldState` followed, when Known, by 10 bytes:

- `u8 part`: `HEAD` (1) or `FOOT` (2);
- `u8 partner_direction`: `NORTH` (1), `EAST` (2), `SOUTH` (3) or `WEST` (4), the direction from
  this part to its partner part;
- `u32 occupied_male` and `u32 occupied_female`: Item ordinals of the look while occupied. Both
  must resolve to an Item of the same family, or the encode and the load fail. An ordinal equal to
  the record's own Item means "no change".

An unknown part, an unknown direction and a dangling ordinal fail closed on encode and decode.

**Set rule.** At compile and at load, an occupied target other than the record's own Item must
carry a Known bed group with the same part and the same partner direction. A target without a bed
group, with the other part or with another partner direction is refused.

The group is a new grammar. v7 therefore uses its own magic (`OTRPA07\0`), profile version,
manifest profile id, compiler and canonicalization profiles. v6 bytes are never reinterpreted as
v7: a v6 reader rejects group 19 and the v7 body version, and either reader refuses the other
profile's bytes by manifest profile id. A client record with group 19 is rejected. The compiler
writes only v7; v4 to v6 are written only by the compatibility tests.

**V3 correction.** The V1 cross-Item target row registers 12 slots and V3 inherited it. v6 had
already added the potion empty flask as a thirteenth target slot, so the V3 limit is corrected to
13 (`DUR04-REFERENCE-ITEM-PROFILE-V3-CROSS-ITEM-TARGETS`). V4 registers 15.

## 2. Reproducible evidence

- Tool: `tools/reference-item-resource-profile/item_resource_profile.py`. `--profile v7` is the
  default. `--profile v6` and `--profile v5` re-check their evidence without drift, and
  `--profile v4 --output <path>` reproduces the v4 measurement byte for byte.
- Evidence: `docs/agents/evidence/OTV2-20261005-item-sem-bed-1-v7-resource-evidence.json`. A run
  without `--output` rebuilds the evidence and fails on any difference.
- Human summary: `docs/agents/evidence/OTV2-20261005-item-sem-bed-1-v7-resource-evidence.md`.

The worst shape is the v6 worst shape plus a Known bed with both looks on the last definition
ordinal. The Rust codec's v7 worst-shape records match the tool's digests exactly (unit test
`typed_v7_body_matches_registered_record_maxima_and_round_trips`).

## 3. Recomputed ceilings

| Resource | v6 | v7 | Derivation |
|---|---:|---:|---|
| Server groups per record | 18 | 19 | closed server group vocabulary |
| Client groups per record | 12 | 12 | explicit client allowlist, unchanged |
| Cross-Item target slots | 13 | 15 | +2 bed occupied looks |
| Server Item body | 3,598 | 3,612 | +14 group 19 (3 header + 1 state + 1 part + 1 direction + 2 × 4 ordinal) |
| Client Item body | 3,454 | 3,454 | group 19 is not projected |
| Server body section | 137,288,886 | 137,823,084 | `38,157 * 3,612` |
| Client body section | 131,794,278 | 131,794,278 | `38,157 * 3,454` |
| Server artifact | 178,086,423 | 178,620,621 | `200 envelope + 7,500 manifest + 40,789,837 index + body` |
| Client artifact | 172,591,815 | 172,591,815 | the same with the client body |
| Generation pair | 350,678,238 | 351,212,436 | checked sum of both artifacts |

The other v1-v3 ceilings are unchanged and apply to v7 as registered. These are: records,
presentation atoms, aliases, tags, capability states, Equipment patterns, reserved slots,
exclusive groups, group key bytes, vocation sets, weapon elements, resistances, modifiers,
imbuement limits, potion restores, and manifest and index bytes.

## 4. Required enforcement

v1 §6, v2 §4 and v3 §4 apply unchanged. In addition, every group count is checked against the
profile of the artifact being read, and the set rule runs at compile and at load. The registry
rows prefixed `DUR04-REFERENCE-ITEM-PROFILE-V4-` carry the max and max+1 boundary tests. The Rust
unit tests `typed_v7_*`, `cross_item_target_slots_are_13_in_v6_and_15_in_v7`, `bed_*` and
`compiler_writes_only_v7_and_v4_to_v6_artifacts_still_decode_under_their_profile` exercise those
tests.
