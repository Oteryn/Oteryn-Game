# Oteryn Atlas animated appearances v1

Status: Game-owned export contract  
Capability: `animated-appearances-v1`  
Source profile: `oteryn-atlas-15-32-appearance-spatial-v1`

## Authority and scope

`Oteryn/Oteryn-Game` owns the semantic conversion of the exact 15.32 appearance source into this contract. `Oteryn/Oteryn-Atlas` may consume the normalized product and publish/cache pixels, but it MUST NOT reopen Tibia DAT/SPR/OTBM/legacy files in browser runtime and MUST NOT infer animation phases, frame groups, pattern selection, outfit directions, addon layers or tint semantics.

This capability supports object/item appearance animation and creature outfit presentation. Effects and missiles are census-only in v1 and are not promoted to Atlas runtime support.

Exact accepted source identity:

- Drive file ID: `1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv`
- ZIP SHA-256: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`
- `catalog-content.json` SHA-256: `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`
- `appearances-*.dat` SHA-256: `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`

The pinned `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` decoder/protobuf/renderers are migration/conversion evidence only. The executable contract in `tools/game-atlas-appearances/` is the Game-owned authority exported to Atlas.

## Product

A generated product contains `product.json`, `manifest.json`, `census.json`, `object-programs.jsonl` and `outfit-programs.jsonl`. The product root is SHA-256 over a domain-separated canonical manifest core followed by every canonical object/outfit program in stable order. Each program has its own `animation-program:sha256:<digest>` identity. The product contains metadata and source sprite identities only; proprietary raw archives are not committed.

## Frame and sprite semantics

Supported fixed frame groups are `0=outfit idle`, `1=outfit moving`, and `2=object initial`. Object records using another group fail closed. Outfit records using a group other than idle/moving fail closed.

For a concrete program the source sprite array is indexed in this exact order:

`phase -> pattern_z -> pattern_y -> pattern_x -> layer`

Equivalent index:

`((((phase * patternDepth + z) * patternHeight + y) * patternWidth + x) * layers + layer)`

Atlas does not choose object patterns. The Game placement producer resolves item coordinate/stack/hook/fluid/subtype semantics and exports the concrete `{x,y,z}` plus `animation_ref`. An animated placement therefore references a deduplicated program and a resolved pattern, rather than carrying phase arrays on every world primitive.

## Animation timing and looping

Source phase ordering is the protobuf `sprite_phase` order. The source `default_start_phase`, `synchronized`, `random_start_phase`, `loop_type`, `loop_count`, and raw `[duration_min,duration_max]` values are preserved.

Loop types are `pingpong=-1`, `infinite=0`, and `counted=1`. `duration_min > duration_max`, out-of-range default phase, invalid counted-loop metadata, missing phase sprite references, unsupported enums and malformed cardinalities fail closed.

The exact 15.32 source contains some `0/0` phase ranges. The accepted migration/reference renderer uses the first non-zero duration range in that frame group as the effective range; when an entire animated group is `0/0`, effective timing is `1/1 ms`. This contract preserves both raw and effective ranges. For deterministic Atlas presentation, v1 chooses the midpoint of each effective range. That midpoint is **derived presentation behavior**, not canonical game/server state. Atlas MUST NOT sample outside the authoritative range or reinterpret this policy.

Animation never implies movement, AI, pathfinding, combat, respawn progression or server-observed state.

## Outfit composition

`lookType` resolves only against the exact source outfit category. Unknown look types fail closed for pixel presentation while the factual creature record remains present for marker fallback.

For current static creature placements:

- pattern `z = 0`; mounted-state pattern semantics are not claimed;
- `pattern_width == 1` exposes the sole presentation as `south -> 0`;
- `pattern_width >= 4` exposes `north/east/south/west -> 0/1/2/3`;
- pattern widths `2` or `3` are unsupported rather than assigned invented cardinal meanings;
- static default direction is south;
- addon pattern `y=0` is always composed, and `y>=1` is composed iff `addons & (1 << (y-1))`;
- layer 0 is base RGBA;
- for two-layer outfits, layer 1 is the exact Tibia color mask;
- mask role colors are yellow=`head`, red=`body`, green=`legs`, blue=`feet` for exact opaque RGBA mask pixels;
- tint RGB values use the Game-owned Tibia HSI 19x7 resolver exported by `tools/game-atlas-appearances/export.py`;
- enabled addon rows are alpha-composed in ascending pattern-y order.

Outfit layer counts outside `1..2`, ambiguous duplicate idle/moving groups, unsupported direction dimensions, invalid color inputs, unknown look types or missing source sprites produce explicit unresolved presentation; they never erase a factual NPC/monster placement.

A moving outfit frame group may be presented **in place** as appearance animation. This does not claim that the creature moved or faced a server-observed direction.

## Full-world placement handoff

`tools/game-atlas-fullworld-source/animated.py` decorates the existing Game tile projection. For an animated object presentation it emits `animation_resolution_state=RESOLVED`, `animation_ref.animation_program_id`, exact concrete `pattern {x,y,z}`, and a content-derived `variant_id`. Static object appearances emit `STATIC`. Existing unresolved semantic presentations remain unresolved. All layers of a resolved presentation must agree on the same concrete pattern or the producer fails closed.

## Creature handoff

`tools/game-atlas-creatures/animated.py` layers verified pixel-presentation metadata onto the existing factual static creature export. It preserves the original placement/outfit-definition resolution and adds a separate `presentation_resolution_state`: `RESOLVED` includes Game-owned `outfit_presentation`; `UNRESOLVED_APPEARANCE` retains `presentation_fallback=factual-marker`; already unresolved/ambiguous creature definitions remain factual marker fallbacks. The enriched capability remains `animated-creatures-v1` and records the exact `appearance_product_root`.

Every resolved creature presentation contains the existing Game-owned `static_projection` unchanged. Its selection policy is `prefer-outfit-idle-else-moving-in-place-v1`: choose the unique `outfit-idle` frame group when present; only if idle is absent choose the unique `outfit-moving` group and explicitly mark `uses_moving_group_in_place=true`. The projection fixes the verified static south direction/pattern index, pattern-z, enabled addon rows, program identity, phase count and animation descriptor. Atlas therefore does not choose a creature frame group or direction.

The same presentation now carries a separate moving-in-place decision:

- `moving_in_place_resolution_state=RESOLVED` means `moving_in_place_projection` is present and is derived only from the unique authoritative `outfit-moving` frame group;
- that projection fixes Game-owned south, the authoritative south pattern index, pattern-z, enabled addon rows, animation program, exact phase count/timing, displacement, anchor policy and spatial record identity;
- its selection policy is `unique-outfit-moving-fixed-south-in-place-v1` and `uses_moving_group_in_place=true`;
- Atlas may advance only the published phase program while keeping the factual creature `record_id`, X, Y and floor unchanged;
- `moving_in_place_resolution_state=FALLBACK_STATIC` means no moving projection is published. `moving_in_place_reason` records why moving playback is unsupported and Atlas MUST preserve the already verified static/reference presentation instead of inventing motion.

Current fail-closed moving reasons include `NO_MOVING_FRAME_GROUP`, `AMBIGUOUS_MOVING_FRAME_GROUP`, `UNSUPPORTED_MOVING_DIRECTION`, `UNSUPPORTED_MOVING_TIMING`, `UNSUPPORTED_REVERSE_ADDONS_SOUTH`, `INVALID_OUTFIT_SPATIAL`, and `INVALID_MOVING_IN_PLACE_PROJECTION`. These reasons are presentation metadata only; they do not alter factual creature placement.

A resolved moving-in-place projection MUST match the exact source group program/direction/addon/timing fields and MUST use the same displacement, anchor policy and spatial record identity as the verified static projection. `validate_animated_creatures` enforces these invariants and rejects corrupted projections. Repeated identical outfit tuples remain resolution-cached, and immutable program indexes are read once per enrichment process; this is a performance optimization only and does not alter content identities.

The producer reports NPC and monster counts for resolved moving-in-place records, dynamically multi-phase moving records, static-fallback moving records, unique outfits in each class, and fallback reason counts. Those census fields are evidence; they do not create support by inference.

## Validation and fail-closed rules

The producer verifies exact ZIP/catalog/appearance identities before accepting the real source. It bounds appearance/group/pattern/layer/phase/reference counts, validates sprite catalogue coverage and frame cardinality, and rejects malformed timing/loop metadata. Product verification recomputes every file digest, every per-program content identity and the complete product root.

Required CI builds the exact 15.32 product twice and requires byte-identical trees, verifies the exhaustive census against committed evidence, proves deliberate product corruption fails closed, enriches real pinned NPC/monster source records, validates the moving/static handoff, and records the exact moving-in-place support census including ordinary-creature fixtures.

## Rights boundary

This contract does not widen pixel redistribution rights. The Atlas repository's current exact-15.32 attestation authorizes the bounded public DYN-ATLAS-001/Thais Z7 proof only. Metadata/product generation and private validation may proceed, but unrestricted full-world public redistribution of exact 15.32 derived pixels must not be claimed authorized unless separately evidenced.
