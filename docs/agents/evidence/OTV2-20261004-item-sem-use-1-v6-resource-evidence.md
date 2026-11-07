# ITEM-SEM-USE-1 v6 resource evidence

- Task: `OTV2-20261004-item-sem-use-1-consumption-group`
- Profile: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V3` (artifact `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v6`)
- Machine evidence: `OTV2-20261004-item-sem-use-1-v6-resource-evidence.json`
- Reproduce: `python3 tools/reference-item-resource-profile/item_resource_profile.py` rebuilds the
  JSON in memory and fails on drift. `--output <path>` writes it. `--profile v5` re-checks the
  v5 evidence (`OTV2-20261003-item-sem-2b3-v5-resource-evidence.json`) without drift.

## Worst shape

| Projection | Record bytes | SHA-256 |
|---|---:|---|
| server-authoritative | 3,598 | `2181e6e62d70b2a01069a696c40c4324ad06c7a5c771362f47c599072b0cfe35` |
| client-safe | 3,454 | `ad6bef56fd078582975ed1d5818cca8b8ae3cace0adff696eb960502991b899f` |

The Rust v6 encoder produces the same two records byte for byte (unit test
`typed_v6_body_matches_registered_record_maxima_and_round_trips`). The client record has the v5
length; only its body version byte differs.

## Registered ceilings

Server groups 18 and client groups 12. Potion restores 2. Body sections are 137,288,886 bytes
(server) and 131,794,278 bytes (client). Artifacts are 178,086,423 bytes (server) and
172,591,815 bytes (client); the generation pair is 350,678,238 bytes. All other ceilings are
inherited unchanged from v1 and v2.

## Boundary checks (tool)

Every check passes. The v6-specific checks are:
- Food round-trips at 1 and 1,199 and is rejected at 0 and 1,200;
- Potion round-trips with Health and a Known flask, Mana and no flask, and Health plus Mana;
- Potion is rejected with no restores, three restores, a duplicate resource, Mana before Health,
  min 0, min above max, max 10,001, and an Unknown or Conflict flask;
- a restore count of 3, an unknown variant, an unknown resource and a dangling flask ordinal are
  rejected on decode;
- a client record with group 18 is rejected;
- 18 server and 12 client groups are accepted, and 19 and 13 are rejected;
- the v5 codec rejects consumption, group 18 and the v6 body version.
