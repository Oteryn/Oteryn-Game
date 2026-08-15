from __future__ import annotations

from copy import deepcopy
from typing import Any

SEMANTIC_CLASSES = {
    "ground",
    "ground_border",
    "static_item",
    "dynamic_item",
    "creature",
    "npc",
    "effect_or_ephemeral",
    "unknown",
}
MAPPING_STATUSES = {"MAPPED", "UNMAPPED", "NOT_APPLICABLE"}
DIFF_STATUSES = {
    "MATCH",
    "MISSING_IN_REFERENCE",
    "MISSING_IN_RECONSTRUCTION",
    "GROUND_MISMATCH",
    "ITEM_MISMATCH",
    "STACK_ORDER_MISMATCH",
    "UNMAPPED_ID",
    "REFERENCE_CONFLICT",
    "NOT_OBSERVED",
}
STATIC_CLASSES = {"ground", "ground_border", "static_item"}


class ValidationError(ValueError):
    pass


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def tile_key(tile: dict[str, Any]) -> tuple[int, int, int]:
    coordinate = tile["coordinate"]
    return coordinate["x"], coordinate["y"], coordinate["z"]


def validate_document(document: dict[str, Any]) -> None:
    _require(isinstance(document, dict), "document must be an object")
    _require(document.get("format") == "oteryn-worldmap-normalized-v1", "unsupported format")
    _require(isinstance(document.get("source"), dict), "source must be an object")
    _require(isinstance(document.get("tiles"), list), "tiles must be a list")
    _require(isinstance(document.get("entities", []), list), "entities must be a list")

    seen: set[tuple[int, int, int]] = set()
    for tile_index, tile in enumerate(document["tiles"]):
        _require(isinstance(tile, dict), f"tiles[{tile_index}] must be an object")
        coordinate = tile.get("coordinate")
        _require(isinstance(coordinate, dict), f"tiles[{tile_index}].coordinate must be an object")
        for axis in ("x", "y", "z"):
            _require(_is_int(coordinate.get(axis)), f"tiles[{tile_index}].coordinate.{axis} must be int")
        key = tile_key(tile)
        _require(key not in seen, f"duplicate tile coordinate {key}")
        seen.add(key)
        _require(isinstance(tile.get("observed"), bool), f"tiles[{tile_index}].observed must be bool")
        _require(_is_int(tile.get("sequence")) and tile["sequence"] >= 0, f"tiles[{tile_index}].sequence must be non-negative int")
        contents = tile.get("contents")
        _require(isinstance(contents, list), f"tiles[{tile_index}].contents must be a list")
        stack_indices: list[int] = []
        for content_index, content in enumerate(contents):
            prefix = f"tiles[{tile_index}].contents[{content_index}]"
            _require(isinstance(content, dict), f"{prefix} must be an object")
            stack_index = content.get("stack_index")
            _require(_is_int(stack_index) and stack_index >= 0, f"{prefix}.stack_index must be non-negative int")
            stack_indices.append(stack_index)
            appearance_id = content.get("client_appearance_id")
            _require(_is_int(appearance_id) and appearance_id >= 0, f"{prefix}.client_appearance_id must be non-negative int")
            semantic_class = content.get("semantic_class")
            _require(semantic_class in SEMANTIC_CLASSES, f"{prefix}.semantic_class is invalid")
            mapping_status = content.get("mapping_status")
            _require(mapping_status in MAPPING_STATUSES, f"{prefix}.mapping_status is invalid")
            server_id = content.get("server_otb_id")
            if mapping_status == "MAPPED":
                _require(_is_int(server_id) and server_id > 0, f"{prefix}.server_otb_id required for MAPPED")
            else:
                _require(server_id is None, f"{prefix}.server_otb_id must be null unless MAPPED")
        _require(stack_indices == sorted(stack_indices) and len(stack_indices) == len(set(stack_indices)), f"tiles[{tile_index}] stack indices must be unique and ascending")


def index_tiles(document: dict[str, Any]) -> dict[tuple[int, int, int], dict[str, Any]]:
    validate_document(document)
    return {tile_key(tile): tile for tile in document["tiles"]}


def merge_documents(base: dict[str, Any], update: dict[str, Any]) -> dict[str, Any]:
    validate_document(base)
    validate_document(update)
    if update.get("entities"):
        raise ValidationError("entity merge semantics are not defined; incoming entities must be empty")

    merged = deepcopy(base)
    indexed = {tile_key(tile): deepcopy(tile) for tile in merged["tiles"]}
    history = list(merged.get("merge_history", []))

    for incoming in update["tiles"]:
        if not incoming["observed"]:
            continue
        key = tile_key(incoming)
        previous = indexed.get(key)
        if previous is None or incoming["sequence"] > previous["sequence"]:
            if previous is not None:
                history.append({"coordinate": dict(previous["coordinate"]), "replaced_sequence": previous["sequence"], "by_sequence": incoming["sequence"]})
            indexed[key] = deepcopy(incoming)
        elif incoming["sequence"] == previous["sequence"] and incoming != previous:
            raise ValidationError(f"conflicting tile update at {key} for sequence {incoming['sequence']}")

    merged["tiles"] = [indexed[key] for key in sorted(indexed)]
    merged["merge_history"] = history
    merged.setdefault("sources", [])
    merged["sources"].append(deepcopy(update["source"]))
    validate_document(merged)
    return merged


def _static_signature(tile: dict[str, Any]) -> tuple[int | None, list[int], bool]:
    ground: int | None = None
    items: list[int] = []
    unmapped = False
    for content in tile["contents"]:
        if content["semantic_class"] not in STATIC_CLASSES:
            continue
        if content["mapping_status"] != "MAPPED":
            unmapped = True
            continue
        server_id = content["server_otb_id"]
        if content["semantic_class"] == "ground":
            if ground is None:
                ground = server_id
            elif ground != server_id:
                unmapped = True
        else:
            items.append(server_id)
    return ground, items, unmapped


def compare_documents(reconstruction: dict[str, Any], reference: dict[str, Any]) -> dict[str, Any]:
    left = index_tiles(reconstruction)
    right = index_tiles(reference)
    results: list[dict[str, Any]] = []

    for key in sorted(set(left) | set(right)):
        candidate = left.get(key)
        baseline = right.get(key)
        coordinate = {"x": key[0], "y": key[1], "z": key[2]}
        if candidate is not None and not candidate["observed"]:
            status = "NOT_OBSERVED"
        elif candidate is None:
            status = "MISSING_IN_RECONSTRUCTION"
        elif baseline is None:
            status = "MISSING_IN_REFERENCE"
        elif not baseline["observed"]:
            status = "REFERENCE_CONFLICT"
        else:
            left_ground, left_items, left_unmapped = _static_signature(candidate)
            right_ground, right_items, right_unmapped = _static_signature(baseline)
            if left_unmapped or right_unmapped:
                status = "UNMAPPED_ID"
            elif left_ground != right_ground:
                status = "GROUND_MISMATCH"
            elif left_items == right_items:
                status = "MATCH"
            elif sorted(left_items) == sorted(right_items):
                status = "STACK_ORDER_MISMATCH"
            else:
                status = "ITEM_MISMATCH"
        results.append({"coordinate": coordinate, "status": status})

    counts = {status: 0 for status in sorted(DIFF_STATUSES)}
    for result in results:
        counts[result["status"]] += 1
    return {
        "format": "oteryn-worldmap-diff-v1",
        "reconstruction_source": deepcopy(reconstruction["source"]),
        "reference_source": deepcopy(reference["source"]),
        "summary": counts,
        "tiles": results,
    }


def build_otbm_export_plan(document: dict[str, Any]) -> dict[str, Any]:
    validate_document(document)
    output_tiles: list[dict[str, Any]] = []
    blockers: list[dict[str, Any]] = []
    for tile in document["tiles"]:
        if not tile["observed"]:
            continue
        ground, items, unmapped = _static_signature(tile)
        if unmapped:
            blockers.append({"coordinate": dict(tile["coordinate"]), "reason": "UNMAPPED_ID"})
            continue
        if ground is None:
            blockers.append({"coordinate": dict(tile["coordinate"]), "reason": "GROUND_NOT_PROVEN"})
            continue
        output_tiles.append({"coordinate": dict(tile["coordinate"]), "ground_server_otb_id": ground, "ordered_static_server_otb_ids": items})
    return {
        "format": "oteryn-otbm-export-plan-v1",
        "source": deepcopy(document["source"]),
        "ready": not blockers,
        "tiles": output_tiles,
        "blockers": blockers,
    }
