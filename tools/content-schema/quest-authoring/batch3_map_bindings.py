"""Reproduce the batch-three UID/AID audit against the pinned native map.

This is a read-only audit. It neither invents selectors nor changes placements.
The separate request file also retains source selectors currently held outside
the runtime callback registry.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec as codec

REQUEST = Path(__file__).with_name("batch3_map_selectors.json")
OUTPUT = ROOT / "docs/agents/evidence/quests-3-20261102/map-bindings.json"


def audit():
    request = json.loads(REQUEST.read_text())
    evidence = request["input_evidence"]
    raw_index = (ROOT / evidence["index_path"]).read_bytes()
    if hashlib.sha256(raw_index).hexdigest() != evidence["index_sha256"]:
        raise ValueError("the pinned map index changed; review a new source snapshot")
    index = json.loads(raw_index)
    palette = index["palette"]
    matches = {kind: {int(value): [] for value in values}
               for kind, values in request["requested"].items()}
    regions = index["regions"]
    if len(regions) != request["region_count"]:
        raise ValueError("map region denominator changed")
    for number, region in enumerate(regions, 1):
        path = ROOT / region["path"]
        raw = path.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        if digest != region["sha256"]:
            raise ValueError(f"region digest mismatch: {region['path']}")
        z, _, _, sectors = codec.decode_region(raw)
        for _, tiles in sectors:
            for x, y, _, _, _, items in tiles:
                ordinal = 0
                for palette_index, depth, attrs in items:
                    for kind, values in matches.items():
                        value = (attrs or {}).get(kind)
                        if value in values:
                            values[value].append({
                                "position": [x, y, z], "depth": depth,
                                "ordinal": ordinal if depth == 0 else None,
                                "source_item_id": palette[palette_index]["source_item_id"],
                                "attrs": attrs, "region": region["path"],
                                "region_sha256": digest,
                            })
                    if depth == 0:
                        ordinal += 1
        if number % 200 == 0:
            print(f"verified {number}/{len(regions)} map regions", flush=True)
    bindings = evidence["item_bindings"]
    if hashlib.sha256((ROOT / bindings["path"]).read_bytes()).hexdigest() != bindings["sha256"]:
        raise ValueError("item binding input changed")
    return {**request, "matches": matches,
            "input_evidence": {**evidence, "all_current_region_hashes_match_index": True,
                               "mismatches": []}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(audit(), indent=2) + "\n"
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != text:
            raise ValueError("stale batch-three map binding audit")
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(text)
    print("batch-three map bindings verified" if args.check else "batch-three map bindings generated")


if __name__ == "__main__":
    main()
