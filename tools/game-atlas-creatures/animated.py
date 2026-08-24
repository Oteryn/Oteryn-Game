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


def _moving_in_place_reason(exc: Exception) -> str:
    text = str(exc)
    if "moving outfit frame group is unavailable" in text:
        return "NO_MOVING_FRAME_GROUP"
    if "ambiguous moving outfit frame group" in text:
        return "AMBIGUOUS_MOVING_FRAME_GROUP"
    if "moving south direction is unavailable" in text:
        return "UNSUPPORTED_MOVING_DIRECTION"
    if "moving animation timing" in text or "moving phase count" in text:
        return "UNSUPPORTED_MOVING_TIMING"
    if "reverse-addon south" in text:
        return "UNSUPPORTED_REVERSE_ADDONS_SOUTH"
    if "invalid outfit spatial displacement" in text:
        return "INVALID_OUTFIT_SPATIAL"
    return "INVALID_MOVING_IN_PLACE_PROJECTION"


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
    # Exact 15.32 encodes a reverse_addons_south flag for a bounded subset of
    # outfits, while the pinned migration/reference renderer does not define its
    # ordering behavior. It is irrelevant when no addon rows are enabled; with
    # addons present we must fail closed rather than invent the composition order.
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


def _moving_in_place_projection(presentation: dict[str, Any], spatial: dict[str, Any], addons: int) -> dict[str, Any]:
    groups = presentation.get("groups")
    if not isinstance(groups, list) or not groups:
        raise RuntimeError("resolved outfit has no frame groups")
    moving = [group for group in groups if group.get("frame_group", {}).get("semantic") == "outfit-moving"]
    if not moving:
        raise RuntimeError("moving outfit frame group is unavailable")
    if len(moving) > 1:
        raise RuntimeError("ambiguous moving outfit frame group")
    selected = moving[0]
    directions = selected.get("directions", {})
    if not isinstance(directions, dict) or STATIC_DIRECTION not in directions:
        raise RuntimeError("moving south direction is unavailable")
    enabled_rows = selected.get("enabled_addon_pattern_y")
    if not isinstance(enabled_rows, list) or not enabled_rows:
        raise RuntimeError("moving addon rows are unavailable")
    enabled_addons = [value for value in enabled_rows if int(value) > 0]
    if addons and enabled_addons and bool(spatial.get("reverse_addons", {}).get("south")):
        raise RuntimeError("reverse-addon south ordering is not proven")
    phase_count = int(selected.get("phase_count", 0))
    if phase_count <= 0:
        raise RuntimeError("moving phase count is invalid")
    animation = selected.get("animation") if phase_count > 1 else None
    if phase_count > 1:
        if not isinstance(animation, dict):
            raise RuntimeError("moving animation timing is unavailable")
        durations = animation.get("presentation_durations_ms")
        if (
            not isinstance(durations, list)
            or len(durations) != phase_count
            or any(not isinstance(value, int) or isinstance(value, bool) or value <= 0 for value in durations)
        ):
            raise RuntimeError("moving animation timing is invalid")
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
        "enabled_addon_pattern_y": enabled_rows,
        "frame_group": selected["frame_group"],
        "pattern_x": int(directions[STATIC_DIRECTION]),
        "pattern_z": int(selected["pattern_z"]),
        "phase_count": phase_count,
        "selection_policy": "unique-outfit-moving-fixed-south-in-place-v1",
        "spatial_record_id": spatial["spatial_record_id"],
        "uses_moving_group_in_place": True,
    }


def _validate_projection_against_group(
    projection: dict[str, Any],
    group: dict[str, Any],
    *,
    projection_kind: str,
) -> None:
    directions = group.get("directions")
    if not isinstance(directions, dict) or STATIC_DIRECTION not in directions:
        raise RuntimeError("validated creature projection has no authoritative south direction")
    expected_policy = (
        "unique-outfit-moving-fixed-south-in-place-v1"
        if projection_kind == "moving"
        else "prefer-outfit-idle-else-moving-in-place-v1"
    )
    if projection.get("selection_policy") != expected_policy:
        raise RuntimeError("creature projection selection policy mismatch")
    if projection.get("direction") != STATIC_DIRECTION or int(projection.get("pattern_x", -1)) != int(directions[STATIC_DIRECTION]):
        raise RuntimeError("creature projection direction mismatch")
    if projection.get("animation_program_id") != group.get("animation_program_id"):
        raise RuntimeError("creature projection animation program mismatch")
    if projection.get("frame_group") != group.get("frame_group"):
        raise RuntimeError("creature projection frame group mismatch")
    if projection.get("enabled_addon_pattern_y") != group.get("enabled_addon_pattern_y"):
        raise RuntimeError("creature projection addon rows mismatch")
    if int(projection.get("pattern_z", -1)) != int(group.get("pattern_z", -2)):
        raise RuntimeError("creature projection pattern_z mismatch")
    phase_count = int(group.get("phase_count", 0))
    if int(projection.get("phase_count", 0)) != phase_count or phase_count <= 0:
        raise RuntimeError("creature projection phase count mismatch")
    expected_animation = group.get("animation") if phase_count > 1 else None
    if projection.get("animation") != expected_animation:
        raise RuntimeError("creature projection timing mismatch")
    expected_moving_flag = group.get("frame_group", {}).get("semantic") == "outfit-moving"
    if bool(projection.get("uses_moving_group_in_place")) is not expected_moving_flag:
        raise RuntimeError("creature projection moving-in-place flag mismatch")


def validate_animated_creatures(result: dict[str, Any]) -> None:
    if result.get("capability") != CAPABILITY:
        raise RuntimeError("unsupported animated creature capability")
    for key in ("npcs", "monster_spawns"):
        records = result.get(key)
        if not isinstance(records, list):
            raise RuntimeError(f"animated creature {key} missing")
        for record in records:
            if record.get("presentation_resolution_state") != "RESOLVED":
                continue
            presentation = record.get("outfit_presentation")
            if not isinstance(presentation, dict):
                raise RuntimeError("resolved animated creature presentation missing")
            groups = presentation.get("groups")
            if not isinstance(groups, list):
                raise RuntimeError("resolved animated creature groups missing")
            static = presentation.get("static_projection")
            if not isinstance(static, dict):
                raise RuntimeError("resolved animated creature static projection missing")
            static_matches = [group for group in groups if group.get("animation_program_id") == static.get("animation_program_id")]
            if len(static_matches) != 1:
                raise RuntimeError("static projection does not identify exactly one Game frame group")
            _validate_projection_against_group(static, static_matches[0], projection_kind="static")
            moving_state = presentation.get("moving_in_place_resolution_state")
            moving_groups = [group for group in groups if group.get("frame_group", {}).get("semantic") == "outfit-moving"]
            if moving_state == "RESOLVED":
                projection = presentation.get("moving_in_place_projection")
                if not isinstance(projection, dict) or len(moving_groups) != 1:
                    raise RuntimeError("resolved moving-in-place projection is ambiguous or missing")
                _validate_projection_against_group(projection, moving_groups[0], projection_kind="moving")
                if projection.get("displacement") != static.get("displacement") or projection.get("anchor_policy") != static.get("anchor_policy"):
                    raise RuntimeError("moving-in-place projection anchor semantics drifted from static projection")
                if projection.get("spatial_record_id") != static.get("spatial_record_id"):
                    raise RuntimeError("moving-in-place projection spatial authority mismatch")
            elif moving_state == "FALLBACK_STATIC":
                if "moving_in_place_projection" in presentation:
                    raise RuntimeError("fallback static presentation must not publish a moving-in-place projection")
                reason = presentation.get("moving_in_place_reason")
                if not isinstance(reason, str) or not reason:
                    raise RuntimeError("fallback static presentation reason missing")
            else:
                raise RuntimeError("moving-in-place resolution state missing")


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

    # Index files are immutable for one content-addressed product. Read them once
    # per process; repeated outfit tuples must not turn that lookup into per-record I/O.
    loader = getattr(appearance_module, "load_program_indexes", None)
    if loader is not None and not hasattr(loader, "cache_info"):
        appearance_module.load_program_indexes = functools.lru_cache(maxsize=2)(loader)

    result = copy.deepcopy(static_result)
    previous_digest = result.pop("semantic_digest", None)
    result["static_semantic_digest"] = previous_digest
    result["capability"] = CAPABILITY
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
        moving_outfits: set[str] = set()
        moving_dynamic_outfits: set[str] = set()
        moving_fallback_outfits: set[str] = set()
        resolved_records = 0
        unresolved_records = 0
        moving_records = 0
        moving_dynamic_records = 0
        moving_fallback_records = 0
        reason_counts: dict[str, int] = {}
        moving_reason_counts: dict[str, int] = {}
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
                        resolved = {**presentation, "static_projection": _static_projection(presentation, spatial, cache_key[5])}
                        try:
                            moving_projection = _moving_in_place_projection(presentation, spatial, cache_key[5])
                        except (appearance_module.ProductError, spatial_module.SpatialError, KeyError, TypeError, ValueError, RuntimeError) as moving_exc:
                            resolved["moving_in_place_resolution_state"] = "FALLBACK_STATIC"
                            resolved["moving_in_place_reason"] = _moving_in_place_reason(moving_exc)
                        else:
                            resolved["moving_in_place_resolution_state"] = "RESOLVED"
                            resolved["moving_in_place_projection"] = moving_projection
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
            moving_state = resolved["moving_in_place_resolution_state"]
            if moving_state == "RESOLVED":
                moving_records += 1
                moving_outfits.add(outfit_key)
                moving_projection = resolved["moving_in_place_projection"]
                if moving_projection.get("animation") is not None and int(moving_projection.get("phase_count", 1)) > 1:
                    moving_dynamic_records += 1
                    moving_dynamic_outfits.add(outfit_key)
            else:
                moving_fallback_records += 1
                moving_fallback_outfits.add(outfit_key)
                moving_reason = str(resolved["moving_in_place_reason"])
                moving_reason_counts[moving_reason] = moving_reason_counts.get(moving_reason, 0) + 1

        per_kind[kind] = {
            "resolved_presentation_records": resolved_records,
            "unresolved_presentation_records": unresolved_records,
            "resolved_unique_outfits": len(resolved_outfits),
            "resolved_animated_unique_outfits": len(animated_outfits),
            "presentation_reason_counts": dict(sorted(reason_counts.items())),
            "resolved_moving_in_place_records": moving_records,
            "resolved_dynamic_moving_in_place_records": moving_dynamic_records,
            "fallback_static_moving_in_place_records": moving_fallback_records,
            "resolved_moving_in_place_unique_outfits": len(moving_outfits),
            "resolved_dynamic_moving_in_place_unique_outfits": len(moving_dynamic_outfits),
            "fallback_static_moving_in_place_unique_outfits": len(moving_fallback_outfits),
            "moving_in_place_reason_counts": dict(sorted(moving_reason_counts.items())),
        }

    stats = dict(result.get("statistics", {}))
    stats["presentation_unresolved"] = total_presentation_unresolved
    stats["outfit_resolution_cache_entries"] = len(resolution_cache)
    stats["npc_presentation"] = per_kind["npc"]
    stats["monster_presentation"] = per_kind["monster"]
    result["statistics"] = stats
    validate_animated_creatures(result)
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
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"semantic_digest": result["semantic_digest"], **result["statistics"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
