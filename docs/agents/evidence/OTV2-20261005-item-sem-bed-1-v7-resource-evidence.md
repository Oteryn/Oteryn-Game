# ITEM-SEM-BED-1 v7 resource evidence

- Task: `OTV2-20261005-item-sem-bed-1-bed-group`
- Profile: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V4` (artifact `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v7`)
- Machine evidence: `OTV2-20261005-item-sem-bed-1-v7-resource-evidence.json`
- Reproduce: `python3 tools/reference-item-resource-profile/item_resource_profile.py` rebuilds the
  JSON in memory and fails on drift. `--output <path>` writes it. `--profile v6` re-checks the
  v6 evidence (`OTV2-20261004-item-sem-use-1-v6-resource-evidence.json`) without drift.

## Worst shape

| Projection | Record bytes | SHA-256 |
|---|---:|---|
| server-authoritative | 3,612 | `60719d884c6008a4472b70c30a3b925fd0aaeea28ab86e6ea9c6c6869d3e8521` |
| client-safe | 3,454 | `ec9391cb9289ee626185363cfe2b6dd2d55de35b4f4a87997591053d2b4af599` |

The Rust v7 encoder produces the same two records byte for byte (unit test
`typed_v7_body_matches_registered_record_maxima_and_round_trips`). The client record has the v6
length; only its body version byte differs.

## Registered ceilings

Server groups 19 and client groups 12. Cross-Item target slots 15 (v6: 13, a correction of the
inherited V1 value 12). Body sections are 137,823,084 bytes (server) and 131,794,278 bytes
(client). Artifacts are 178,620,621 bytes (server) and 172,591,815 bytes (client); the generation
pair is 351,212,436 bytes. All other ceilings are inherited unchanged from v1 to v3.

## Boundary checks (tool)

Every check passes. The v7-specific checks are:
- Bed round-trips for Head and Foot in all four partner directions, each with distinct male and
  female looks, the same look for both, and "no change" (its own ordinal);
- Bed is rejected on encode with part 0 or 3, direction 0 or 5, and a dangling look ordinal;
- an unknown part, an unknown direction and a dangling ordinal are rejected on decode;
- a client record with group 19 is rejected;
- 19 server and 12 client groups are accepted, and 20 and 13 are rejected;
- the v6 codec rejects the bed, group 19 and the v7 body version;
- the cross-Item target slots count 13 in v6 and 15 in v7.

The set rule depends on the linked family and is checked by the Rust tests
`bed_set_refuses_*` and `bed_set_admits_a_matching_occupied_target_at_compile_and_load`.
