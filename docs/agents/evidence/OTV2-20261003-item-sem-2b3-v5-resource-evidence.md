# ITEM-SEM-2b-3 v5 resource evidence

- Task: `OTV2-20261003-item-sem-2b3-vocation-none-and-use-requirements`
- Profile: `OTERYN_REFERENCE_ITEM_ARTIFACT_RESOURCE_PROFILE_V2` (artifact `OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v5`)
- Machine evidence: `OTV2-20261003-item-sem-2b3-v5-resource-evidence.json`
- Reproduce: `python3 tools/reference-item-resource-profile/item_resource_profile.py` rebuilds the
  JSON in memory and fails on drift. `--output <path>` writes it. `--profile v4 --output <path>`
  reproduces the v4 measurement unchanged; without `--source-xml`, that run records only the
  digest of the pinned XML.

## Worst shape

| Projection | Record bytes | SHA-256 |
|---|---:|---|
| server-authoritative | 3,577 | `779f3a027f8ccaa9da8bd0e0236f9b0f76da8322985904122daa18e53c67b47e` |
| client-safe | 3,454 | `c99b352169d7d5c88dd6215d971640f9aedd2cea2b0b19d3048bcbc56718383e` |

The Rust v5 encoder produces the same two records byte for byte (unit test
`typed_v5_body_matches_registered_record_maxima_and_round_trips`).

## Registered ceilings

Server groups 17 and client groups 12. Vocation sets 6 for Equipment patterns, trade
restrictions and use requirements. Body sections are 136,487,589 bytes (server) and 131,794,278
bytes (client). Artifacts are 177,285,126 bytes (server) and 172,591,815 bytes (client); the
generation pair is 349,876,941 bytes. All other ceilings are inherited unchanged from v1.

## Boundary checks (tool)

Every check passes. The v5-specific checks are:
- use requirements round-trip with fields present and absent;
- an enforcement mode other than `ON_USE` is rejected on encode and decode;
- use-requirement vocations at max+1 are rejected on encode and decode;
- the vocation `None` (wire 6) round-trips, and wire 7 is rejected;
- group id 18 is rejected;
- the client admits group 17;
- 17 server and 12 client groups are accepted, and 18 and 13 are rejected;
- the v4 codec rejects `None`, group 17 and the v5 body version.
