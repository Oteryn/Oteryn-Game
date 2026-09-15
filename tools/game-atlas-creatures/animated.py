#!/usr/bin/env python3
"""Enrich Game-owned static creature projections with verified outfit programs."""
from __future__ import annotations

import argparse
import copy
import functools
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
from typing import Any

HERE = Path(__file__).resolve().parent
APPEARANCE_EXPORT = HERE.parent / "game-atlas-appearances" / "export.py"
OUTFIT_SPATIAL_EXPORT = HERE.parent / "game-atlas-outfit-spatial" / "export.py"
STATIC_EXPORT = HERE / "export.py"
CAPABILITY = "animated-creatures-v1"
PLAYBACK_PROJECTION_CAPABILITY = "creature-moving-in-place-v1"
PLAYBACK_SELECTION_POLICY = "prefer-outfit-moving-in-place-else-static-v1"
STATIC_DIRECTION = "south"


def _load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _presentation_reason(exc: Exception) -> str:
    text = str(exc)
    if text.startswith("unknown lookType"):
        return "UNKNOWN_LOOK_TYPE"
    if "outside supported 0..132" in text:
        return "UNSUPPORTED_OUTFIT_COLOR"
    if "unsupported direction pattern width" in text:
        return "UNSUPPORTED_DIRECTION_SEMANTICS"
    if "ambiguous" in text:
        return "AMBIGUOUS_OUTFIT_FRAME_GROUP"
    if "no supported outfit frame groups" in text:
        return "UNSUPPORTED_OUTFIT_FRAME_GROUP"
    if "missing outfit spatial record" in text:
        return "UNKNOWN_OUTFIT_SPATIAL"
    if "reverse-addon south" in text:
        return "UNSUPPORTED_REVERSE_ADDONS_SOUTH"
    return "INVALID_OUTFIT_PRESENTATION"


def _static_projection(presentation: dict[str, Any], spatial: dict[str, Any], addons: int) -> dict[str, Any]:
    groups = presentation.get("groups")
    if not isinstance(groups, list) or not groups:
        raise RuntimeError("resolved outfit has no frame groups")
    idle = [group for group in groups if group.get("frame_group", {}).get("semantic") == "outfit-idle"]
    moving = [group for group in groups if group.get("frame_group", {}).get("semantic") == "outfit-moving"]
    if len(idle) > 1 or len(moving) > 1:
        raise RuntimeError("ambiguous static outfit frame group")
    selected = idle[0] if idle else moving[0] if moving else None
    if selected is None:
        raise RuntimeError("no supported static outfit frame group")
    directions = selected.get("directions", {})
    if STATIC_DIRECTION not in directions:
        raise RuntimeError("static south direction is unavailable")
    enabled_addons = [value for value in selected.get("enabled_addon_pattern_y", []) if int(value) > 0]
    if addons and enabled_addons and bool(spatial.get("reverse_addons", {}).get("south")):
        raise RuntimeError("reverse-addon south ordering is not proven")
    phase_count = int(selected.get("phase_count", 1))
    animation = selected.get("animation") if phase_count > 1 else None
    displacement = spatial.get("displacement")
    if not isinstance(displacement, dict) or not all(isinstance(displacement.get(axis), int) for axis in ("x", "y")):
        raise RuntimeError("invalid outfit spatial displacement")
    return {
        "anchor_policy": spatial["anchor_policy"],
        "animate_always": bool(spatial.get("animate_always")),
        "animation": animation,
        "animation_program_id": selected["animation_program_id"],
        "direction": STATIC_DIRECTION,
        "displacement": {"x": int(displacement["x"]), "y": int(displacement["y"])},
        "enabled_addon_pattern_y": selected["enabled_addon_pattern_y"],
        "frame_group": selected["frame_group"],
        "pattern_x": int(directions[STATIC_DIRECTION]),
        "pattern_z": int(selected["pattern_z"]),
        "phase_count": phase_count,
        "selection_policy": "prefer-outfit-idle-else-moving-in-place-v1",
        "spatial_record_id": spatial["spatial_record_id"],
        "uses_moving_group_in_place": selected["frame_group"]["semantic"] == "outfit-moving",
    }


def _playback_fallback(
    presentation: dict[str, Any],
    static_projection: dict[str, Any],
    reason: str,
) -> dict[str, Any]:
    projection = copy.deepcopy(static_projection)
    projection.update({
        "outfit_presentation_id": presentation["outfit_presentation_id"],
        "selection_policy": PLAYBACK_SELECTION_POLICY,
        "playback_resolution_state": "FALLBACK_STATIC_PROJECTION",
        "playback_reason": reason,
        "presentation_mode": "static-fallback",
        "world_position_policy": "UNCHANGED",
    })
    return projection


def _playback_projection(
    presentation: dict[str, Any],
    spatial: dict[str, Any],
    addons: int,
    static_projection: dict[str, Any],
) -> dict[str, Any]:
    groups = presentation.get("groups")
    if not isinstance(groups, list):
        return _playback_fallback(presentation, static_projection, "MOVING_GROUP_MALFORMED")
    moving = [group for group in groups if group.get("frame_group", {}).get("semantic") == "outfit-moving"]
    if not moving:
        return _playback_fallback(presentation, static_projection, "MOVING_GROUP_UNAVAILABLE")
    if len(moving) != 1:
        return _playback_fallback(presentation, static_projection, "AMBIGUOUS_MOVING_GROUP")
    selected = moving[0]
    try:
        directions = selected.get("directions", {})
        if not isinstance(directions, dict) or STATIC_DIRECTION not in directions:
            return _playback_fallback(presentation, static_projection, "MOVING_DIRECTION_UNAVAILABLE")
        enabled_rows = selected.get("enabled_addon_pattern_y", [])
        if not isinstance(enabled_rows, list):
            return _playback_fallback(presentation, static_projection, "MOVING_GROUP_MALFORMED")
        enabled_addons = [value for value in enabled_rows if int(value) > 0]
        if addons and enabled_addons and bool(spatial.get("reverse_addons", {}).get("south")):
            return _playback_fallback(presentation, static_projection, "MOVING_REVERSE_ADDONS_UNPROVEN")
        phase_count = int(selected.get("phase_count", 1))
        if phase_count <= 0:
            return _playback_fallback(presentation, static_projection, "MOVING_GROUP_MALFORMED")
        animation = selected.get("animation") if phase_count > 1 else None
        if phase_count > 1 and not isinstance(animation, dict):
            return _playback_fallback(presentation, static_projection, "MOVING_TIMING_UNAVAILABLE")
        displacement = spatial.get("displacement")
        if not isinstance(displacement, dict) or not all(isinstance(displacement.get(axis), int) for axis in ("x", "y")):
            return _playback_fallback(presentation, static_projection, "MOVING_SPATIAL_UNAVAILABLE")
        return {
            "anchor_policy": spatial["anchor_policy"],
            "animate_always": bool(spatial.get("animate_always")),
            "animation": animation,
            "animation_program_id": selected["animation_program_id"],
            "direction": STATIC_DIRECTION,
            "displacement": {"x": int(displacement["x"]), "y": int(displacement["y"])},
            "enabled_addon_pattern_y": enabled_rows,
            "frame_group": selected["frame_group"],
            "outfit_presentation_id": presentation["outfit_presentation_id"],
            "pattern_x": int(directions[STATIC_DIRECTION]),
            "pattern_z": int(selected["pattern_z"]),
            "phase_count": phase_count,
            "playback_resolution_state": "RESOLVED_MOVING_IN_PLACE",
            "presentation_mode": "moving-in-place",
            "selection_policy": PLAYBACK_SELECTION_POLICY,
            "spatial_record_id": spatial["spatial_record_id"],
            "uses_moving_group_in_place": True,
            "world_position_policy": "UNCHANGED",
        }
    except (KeyError, TypeError, ValueError):
        return _playback_fallback(presentation, static_projection, "MOVING_GROUP_MALFORMED")


def verify_enriched_creatures(
    result: dict[str, Any],
    outfit_spatial_product: Path,
    *,
    spatial_module=None,
) -> dict[str, int]:
    if result.get("capability") != CAPABILITY:
        raise RuntimeError("unsupported animated creature capability")
    if result.get("playback_projection_capability") != PLAYBACK_PROJECTION_CAPABILITY:
        raise RuntimeError("unsupported creature playback projection capability")
    body = copy.deepcopy(result)
    actual_digest = body.pop("semantic_digest", None)
    expected_digest = "sha256:" + hashlib.sha256(_canonical(body)).hexdigest()
    if actual_digest != expected_digest:
        raise RuntimeError("animated creature semantic digest mismatch")

    spatial_module = spatial_module or _load(OUTFIT_SPATIAL_EXPORT, "game_atlas_outfit_spatial_verify")
    spatial_manifest, spatial_index = spatial_module.load_index(outfit_spatial_product)
    if spatial_manifest.get("product_root") != result.get("outfit_spatial_product_root"):
        raise RuntimeError("animated creature outfit spatial root mismatch")
    resolved = 0
    moving = 0
    fallback = 0
    for key in ("npcs", "monster_spawns"):
        for record in result.get(key, []):
            if record.get("presentation_resolution_state") != "RESOLVED":
                continue
            raw = record.get("appearance")
            presentation = record.get("outfit_presentation")
            if not isinstance(raw, dict) or not isinstance(presentation, dict):
                raise RuntimeError("resolved creature is missing outfit presentation")
            look_type = int(raw["look_type"])
            addons = int(raw["addons"])
            spatial = spatial_index.get(look_type)
            if spatial is None:
                raise RuntimeError(f"missing outfit spatial record for lookType {look_type}")
            expected_static = _static_projection(presentation, spatial, addons)
            if presentation.get("static_projection") != expected_static:
                raise RuntimeError("corrupt static creature projection")
            expected_playback = _playback_projection(presentation, spatial, addons, expected_static)
            if presentation.get("playback_projection") != expected_playback:
                raise RuntimeError("corrupt creature playback projection")
            resolved += 1
            if expected_playback["playback_resolution_state"] == "RESOLVED_MOVING_IN_PLACE":
                moving += 1
            else:
                fallback += 1
    return {"resolved": resolved, "moving": moving, "fallback": fallback}


def enrich_creatures(
    static_result: dict[str, Any],
    appearance_product: Path,
    outfit_spatial_product: Path,
    *,
    appearance_module=None,
    spatial_module=None,
) -> dict[str, Any]:
    appearance_module = appearance_module or _load(APPEARANCE_EXPORT, "game_atlas_appearance_product")
    spatial_module = spatial_module or _load(OUTFIT_SPATIAL_EXPORT, "game_atlas_outfit_spatial_product")
    manifest = json.loads((appearance_product / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("capability") != appearance_module.CAPABILITY or manifest.get("contract_id") != appearance_module.CONTRACT_ID:
        raise RuntimeError("unsupported appearance product capability")
    if manifest.get("source") != appearance_module._source_identity():
        raise RuntimeError("appearance product source identity mismatch")
    spatial_manifest, spatial_index = spatial_module.load_index(outfit_spatial_product)
    if spatial_manifest.get("source") != manifest.get("source"):
        raise RuntimeError("outfit spatial product source identity mismatch")

    loader = getattr(appearance_module, "load_program_indexes", None)
    if loader is not None and not hasattr(loader, "cache_info"):
        appearance_module.load_program_indexes = functools.lru_cache(maxsize=2)(loader)

    result = copy.deepcopy(static_result)
    previous_digest = result.pop("semantic_digest", None)
    result["static_semantic_digest"] = previous_digest
    result["capability"] = CAPABILITY
    result["playback_projection_capability"] = PLAYBACK_PROJECTION_CAPABILITY
    result["appearance_capability"] = appearance_module.CAPABILITY
    result["appearance_product_root"] = manifest["product_root"]
    result["appearance_source"] = manifest["source"]
    result["outfit_spatial_capability"] = spatial_manifest["capability"]
    result["outfit_spatial_product_root"] = spatial_manifest["product_root"]

    resolution_cache: dict[tuple[int, int, int, int, int, int], tuple[dict[str, Any] | None, str | None]] = {}
    total_presentation_unresolved = 0
    per_kind: dict[str, dict[str, Any]] = {}
    for kind, key in (("npc", "npcs"), ("monster", "monster_spawns")):
        resolved_outfits: set[str] = set()
        animated_outfits: set[str] = set()
        moving_playback_outfits: set[str] = set()
        animated_moving_playback_outfits: set[str] = set()
        resolved_records = 0
        unresolved_records = 0
        moving_playback_records = 0
        fallback_playback_records = 0
        reason_counts: dict[str, int] = {}
        playback_reason_counts: dict[str, int] = {}
        for record in result.get(key, []):
            if record.get("resolution_state") != "RESOLVED" or not isinstance(record.get("appearance"), dict):
                record["presentation_resolution_state"] = "FALLBACK_MARKER"
                record["presentation_fallback"] = "factual-marker"
                continue
            raw = record["appearance"]
            try:
                cache_key = (
                    int(raw["look_type"]), int(raw["head"]), int(raw["body"]),
                    int(raw["legs"]), int(raw["feet"]), int(raw["addons"]),
                )
            except (KeyError, TypeError, ValueError) as exc:
                cache_key = (-1, -1, -1, -1, -1, -1)
                reason = _presentation_reason(exc)
                resolved = None
            else:
                cached = resolution_cache.get(cache_key)
                if cached is None:
                    try:
                        presentation = appearance_module.resolve_outfit_presentation(
                            appearance_product,
                            look_type=cache_key[0], head=cache_key[1], body=cache_key[2],
                            legs=cache_key[3], feet=cache_key[4], addons=cache_key[5],
                        )
                        spatial = spatial_index.get(cache_key[0])
                        if spatial is None:
                            raise RuntimeError(f"missing outfit spatial record for lookType {cache_key[0]}")
                        static_projection = _static_projection(presentation, spatial, cache_key[5])
                        playback_projection = _playback_projection(presentation, spatial, cache_key[5], static_projection)
                        resolved = {
                            **presentation,
                            "static_projection": static_projection,
                            "playback_projection": playback_projection,
                        }
                        cached = (resolved, None)
                    except (appearance_module.ProductError, spatial_module.SpatialError, KeyError, TypeError, ValueError, RuntimeError) as exc:
                        cached = (None, _presentation_reason(exc))
                    resolution_cache[cache_key] = cached
                resolved, reason = cached

            if resolved is None:
                record["presentation_resolution_state"] = "UNRESOLVED_APPEARANCE"
                record["presentation_reason"] = reason or "INVALID_OUTFIT_PRESENTATION"
                record["presentation_fallback"] = "factual-marker"
                unresolved_records += 1
                total_presentation_unresolved += 1
                reason_counts[record["presentation_reason"]] = reason_counts.get(record["presentation_reason"], 0) + 1
                continue

            record["presentation_resolution_state"] = "RESOLVED"
            record["outfit_presentation"] = resolved
            outfit_key = str(raw["outfit_key"])
            resolved_outfits.add(outfit_key)
            resolved_records += 1
            projection = resolved["static_projection"]
            if projection.get("animation") is not None and int(projection.get("phase_count", 1)) > 1:
                animated_outfits.add(outfit_key)
            playback = resolved["playback_projection"]
            if playback["playback_resolution_state"] == "RESOLVED_MOVING_IN_PLACE":
                moving_playback_records += 1
                moving_playback_outfits.add(outfit_key)
                if playback.get("animation") is not None and int(playback.get("phase_count", 1)) > 1:
                    animated_moving_playback_outfits.add(outfit_key)
            else:
                fallback_playback_records += 1
                playback_reason = str(playback.get("playback_reason", "UNKNOWN_PLAYBACK_FALLBACK"))
                playback_reason_counts[playback_reason] = playback_reason_counts.get(playback_reason, 0) + 1

        per_kind[kind] = {
            "resolved_presentation_records": resolved_records,
            "unresolved_presentation_records": unresolved_records,
            "resolved_unique_outfits": len(resolved_outfits),
            "resolved_animated_unique_outfits": len(animated_outfits),
            "resolved_moving_playback_records": moving_playback_records,
            "fallback_static_playback_records": fallback_playback_records,
            "resolved_moving_playback_unique_outfits": len(moving_playback_outfits),
            "resolved_animated_moving_playback_unique_outfits": len(animated_moving_playback_outfits),
            "presentation_reason_counts": dict(sorted(reason_counts.items())),
            "playback_reason_counts": dict(sorted(playback_reason_counts.items())),
        }

    stats = dict(result.get("statistics", {}))
    stats["presentation_unresolved"] = total_presentation_unresolved
    stats["outfit_resolution_cache_entries"] = len(resolution_cache)
    stats["npc_presentation"] = per_kind["npc"]
    stats["monster_presentation"] = per_kind["monster"]
    result["statistics"] = stats
    result["semantic_digest"] = "sha256:" + hashlib.sha256(_canonical(result)).hexdigest()
    return result


def export_animated_creatures(
    world_root: Path,
    npc_root: Path,
    monster_root: Path,
    appearance_product: Path,
    outfit_spatial_product: Path,
) -> dict[str, Any]:
    static_module = _load(STATIC_EXPORT, "game_atlas_static_creatures")
    return enrich_creatures(
        static_module.export_creatures(world_root, npc_root, monster_root),
        appearance_product,
        outfit_spatial_product,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("world_root", type=Path)
    parser.add_argument("npc_root", type=Path)
    parser.add_argument("monster_root", type=Path)
    parser.add_argument("appearance_product", type=Path)
    parser.add_argument("outfit_spatial_product", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = export_animated_creatures(
        args.world_root,
        args.npc_root,
        args.monster_root,
        args.appearance_product,
        args.outfit_spatial_product,
    )
    verify_enriched_creatures(result, args.outfit_spatial_product)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"semantic_digest": result["semantic_digest"], **result["statistics"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
