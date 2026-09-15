# OTV2-20260818 Game -> Atlas coordinate-profile readiness

Task: `OTV2-20260818-world-spatial-coordinate-profile-v1`  
Coordination ID: `OTERYN-GAME-ATLAS-V1-SPATIAL`

## Verdict

**`COORDINATE_PROFILE_READY_ON_MERGE`**

This evidence closes only the canonical spatial/coordinate semantic gap identified by `OTV2-20260816-game-atlas-physical-profile-readiness.md`, contingent on merge of the exact new profile to protected `main`.

It does **not** select a production serializer, compression/container, permanent chunk size/floor packing, production resource limits, asset distribution policy or physical `Oteryn-Atlas` repository coordinate.

## Exact basis

Trusted base for this task:

- repository: `blakinio/Oteryn-v2`;
- base `main`: `5577f6fc7c1f7ddef482f0f7b08039047704e36b`;
- accepted native World authority: `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md`;
- accepted Game -> Atlas semantic contract: `docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md`;
- historical physical-profile readiness evidence: `docs/agents/evidence/OTV2-20260816-game-atlas-physical-profile-readiness.md`.

## PROVEN

- The accepted Game -> Atlas contract requires an explicit versioned coordinate profile before executable producer/consumer compatibility may be claimed.
- The historical readiness task named the missing evidence as axes/orientation, coordinate domain/bounds, floor identity/order, point/bounds validity, same-position ordering/stack-layer semantics and anchor/displacement semantics.
- Targeted preflight searches returned no open Atlas/spatial/coordinate PR or Issue, and the inspected active-task inventory exposed no task owning the new profile/evidence paths.
- The new `docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md` defines those missing semantics without promoting OTBM, legacy `z`, client `Position`, serializer order or browser behavior into canonical authority.

## Profile semantics established

The profile establishes:

- discrete `i32` horizontal tile coordinates;
- canonical north-up orientation with +X east and +Y south;
- mandatory finite per-world half-open horizontal bounds represented with a wider semantic bounds envelope to avoid positive-edge overflow;
- signed `i16` canonical floor identity with larger values meaning higher/above and an explicit finite declared floor set, without assigning universal meaning to floor zero;
- half-open rectangular inclusion;
- explicit same-position `PresentationOrderKey { plane, order }` with fail-closed duplicate rejection;
- explicit placement anchors and footprint offsets;
- resolution-independent displacement semantics with precision bound by an explicit versioned appearance/asset `units_per_tile` profile rather than frozen by the world coordinate profile;
- deterministic spatial canonicalization order independent from render order;
- mandatory explicit legacy conversion profiles rather than direct `z`/stack assumptions.

## DERIVED

Once the profile is merged to protected `main`, DYN-ATLAS-001 no longer needs to invent coordinate/floor/order semantics. A bounded Game-owned fixture/conversion/export profile can bind its exact Thais source selection to `oteryn-world-spatial-v1`, while the Atlas proof remains free to compare temporary physical encodings.

This is sufficient to remove the **semantic coordinate authority** blocker from the DYN-ATLAS-001 execution prompt. It does not by itself create an Atlas implementation repository or authorize one.

## Remaining evidence gates

### Still `UNKNOWN` / separate

- exact physical `Oteryn-Atlas` implementation repository/authority;
- exact Thais Z7 pinned source selection and conversion profile;
- asset/sprite rights and provenance for any committed visual fixture;
- production Game -> Atlas serializer/compression/container;
- permanent chunk dimensions/floor packing;
- production resource ceilings in `RESOURCE_LIMITS_REGISTRY.json`;
- executable Game exporter and Atlas consumer implementation.

### Not a DYN-ATLAS-001 semantic blocker after profile merge

- serializer winner;
- permanent chunk size;
- permanent floor packing.

The DYN-ATLAS proof is explicitly allowed to compare candidate physical encodings while those remain deferred.

## Historical evidence handling

`OTV2-20260816-game-atlas-physical-profile-readiness.md` remains unchanged and historically correct for its inspected revision. This record is the forward readiness delta; it does not rewrite prior evidence.

## Next action

After the spatial profile PR merges, refresh DYN-ATLAS-001 preflight against the live Platform programme and the actual authorized Atlas implementation repository. If no such repository exists/is authorized, stop there rather than implementing Atlas runtime in Platform or treating legacy Otheryn as the target by assumption.
