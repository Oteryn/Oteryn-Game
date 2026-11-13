#!/usr/bin/env python3
"""Check stable seal IDs against every indexed native B3 region (read-only)."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from world_region_codec import decode_region  # noqa: E402


def main() -> int:
    index_path = ROOT / "content/world/placements/index.json"
    payload = index_path.read_bytes()
    index = json.loads(payload)
    seal_palette = {
        number
        for number, entry in enumerate(index["palette"])
        if entry.get("source_item_id") == 7178
    }
    if not seal_palette or any(
        index["palette"][number].get("key") != "oteryn:item.tibia.i7178"
        or index["palette"][number].get("provisional") is not False
        for number in seal_palette
    ):
        raise SystemExit("seal palette is absent, provisional or incorrectly bound")
    actual: list[tuple[int, int, int, int]] = []
    for region in index["regions"]:
        data = (ROOT / region["path"]).read_bytes()
        if hashlib.sha256(data).hexdigest() != region["sha256"]:
            raise SystemExit(f"region digest mismatch: {region['path']}")
        floor, _, _, sectors = decode_region(data)
        for _, tiles in sectors:
            for x, y, _, _, _, items in tiles:
                ordinal = 0
                for palette, depth, _ in items:
                    if depth == 0:
                        if palette in seal_palette:
                            actual.append((x, y, -floor, ordinal))
                        ordinal += 1
    source = (ROOT / "crates/gameplay/quests/src/quest/ice_islands_seals/targets.rs").read_text()
    table = source.split("const SEALS:", 1)[1].split("];", 1)[0]
    declared = [
        tuple(map(int, row))
        for row in re.findall(r"\(\((\d+),\s*(\d+),\s*(-?\d+)\),\s*(\d+)\)", table)
    ]
    if len(set(declared)) != len(declared) or len(set(actual)) != len(actual):
        raise SystemExit("duplicate seal map identity")
    if set(declared) != set(actual):
        raise SystemExit(
            f"seal identity mismatch: missing={len(set(actual) - set(declared))} "
            f"extra={len(set(declared) - set(actual))}"
        )
    print(
        f"seal targets: PASS {len(actual)} objects / {len(index['regions'])} regions; "
        f"placement-index sha256={hashlib.sha256(payload).hexdigest()}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
