#!/usr/bin/env python3
"""Enrich Game-owned static creature projections with verified outfit programs."""
from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
from typing import Any

HERE = Path(__file__).resolve().parent
APPEARANCE_EXPORT = HERE.parent / "game-atlas-appearances" / "export.py"
STATIC_EXPORT = HERE / "export.py"
CAPABILITY = "animated-creatures-v1"


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
    return "INVALID_OUTFIT_PRESENTATION"


def enrich_creatures(static_result: dict[str, Any], appearance_product: Path, *, appearance_module=None) -> dict[str, Any]:
    appearance_module = appearance_module or _load(APPEARANCE_EXPORT, "game_atlas_appearance_product")
    manifest = json.loads((appearance_product / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("capability") != appearance_module.CAPABILITY or manifest.get("contract_id") != appearance_module.CONTRACT_ID:
        raise RuntimeError("unsupported appearance product capability")
    if manifest.get("source") != appearance_module._source_identity():
        raise RuntimeError("appearance product source identity mismatch")

    result = copy.deepcopy(static_result)
    previous_digest = result.pop("semantic_digest", None)
    result["static_semantic_digest"] = previous_digest
    result["capability"] = CAPABILITY
    result["appearance_capability"] = appearance_module.CAPABILITY
    result["appearance_product_root"] = manifest["product_root"]
    result["appearance_source"] = manifest["source"]

    total_presentation_unresolved = 0
    per_kind: dict[str, dict[str, Any]] = {}
    for kind, key in (("npc", "npcs"), ("monster", "monster_spawns")):
        resolved_outfits: set[str] = set()
        animated_outfits: set[str] = set()
        resolved_records = 0
        unresolved_records = 0
        reason_counts: dict[str, int] = {}
        for record in result.get(key, []):
            if record.get("resolution_state") != "RESOLVED" or not isinstance(record.get("appearance"), dict):
                record["presentation_resolution_state"] = "FALLBACK_MARKER"
                record["presentation_fallback"] = "factual-marker"
                continue
            raw = record["appearance"]
            try:
                presentation = appearance_module.resolve_outfit_presentation(
                    appearance_product,
                    look_type=int(raw["look_type"]), head=int(raw["head"]), body=int(raw["body"]),
                    legs=int(raw["legs"]), feet=int(raw["feet"]), addons=int(raw["addons"]),
                )
            except (appearance_module.ProductError, KeyError, TypeError, ValueError) as exc:
                reason = _presentation_reason(exc)
                record["presentation_resolution_state"] = "UNRESOLVED_APPEARANCE"
                record["presentation_reason"] = reason
                record["presentation_fallback"] = "factual-marker"
                unresolved_records += 1
                total_presentation_unresolved += 1
                reason_counts[reason] = reason_counts.get(reason, 0) + 1
                continue
            record["presentation_resolution_state"] = "RESOLVED"
            record["outfit_presentation"] = presentation
            outfit_key = str(raw["outfit_key"])
            resolved_outfits.add(outfit_key)
            resolved_records += 1
            if any(group.get("animation") is not None and int(group.get("phase_count", 1)) > 1 for group in presentation["groups"]):
                animated_outfits.add(outfit_key)
        per_kind[kind] = {
            "resolved_presentation_records": resolved_records,
            "unresolved_presentation_records": unresolved_records,
            "resolved_unique_outfits": len(resolved_outfits),
            "resolved_animated_unique_outfits": len(animated_outfits),
            "presentation_reason_counts": dict(sorted(reason_counts.items())),
        }

    stats = dict(result.get("statistics", {}))
    stats["presentation_unresolved"] = total_presentation_unresolved
    stats["npc_presentation"] = per_kind["npc"]
    stats["monster_presentation"] = per_kind["monster"]
    result["statistics"] = stats
    result["semantic_digest"] = "sha256:" + hashlib.sha256(_canonical(result)).hexdigest()
    return result


def export_animated_creatures(world_root: Path, npc_root: Path, monster_root: Path, appearance_product: Path) -> dict[str, Any]:
    static_module = _load(STATIC_EXPORT, "game_atlas_static_creatures")
    return enrich_creatures(static_module.export_creatures(world_root, npc_root, monster_root), appearance_product)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("world_root", type=Path)
    parser.add_argument("npc_root", type=Path)
    parser.add_argument("monster_root", type=Path)
    parser.add_argument("appearance_product", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = export_animated_creatures(args.world_root, args.npc_root, args.monster_root, args.appearance_product)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"semantic_digest": result["semantic_digest"], **result["statistics"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
