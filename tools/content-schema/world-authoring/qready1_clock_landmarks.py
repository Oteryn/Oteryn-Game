"""Apply the explicitly Oteryn-authored cauldron landmark; no donor placement is claimed."""

import hashlib
import json
from pathlib import Path

import world_region_codec as codec
from convert_world_metadata import canonical

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "tools/content-schema/world-authoring/samples/qready1-clock-landmarks.json"
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"


def overlay_outputs(outputs):
    capture_raw = (ROOT / CAPTURE).read_bytes()
    capture = json.loads(capture_raw)
    if (
        capture["schema"] != "OTERYN_AUTHORED_QUEST_LANDMARK/v1"
        or capture["basis"] != "CHOSEN_OTERYN_APPROXIMATION"
        or capture["quest"] != "oteryn:quest.authored.bewitched"
        or capture["source_item_id"] != 12023
        or capture["native_ordinal"] != 1
    ):
        raise ValueError("M19_LANDMARK_SCOPE")
    x, y, floor = capture["position"]
    z = -floor
    path = "content/world/placements/" + codec.region_name(z, x // 256, y // 256)
    if path not in outputs:
        return  # Synthetic conversion fixtures do not contain this native room.
    index = json.loads(outputs[INDEX])
    palette = index["palette"]
    matches = [
        i
        for i, row in enumerate(palette)
        if row["source_item_id"] == capture["source_item_id"]
    ]
    if len(matches) != 1:
        raise ValueError("M19_LANDMARK_ITEM_PALETTE")
    item_palette = matches[0]
    before = outputs[path]
    zz, rx, ry, sectors = codec.decode_region(before)
    found = False
    added = 0
    encoded = {}
    for local, tiles in sectors:
        rewritten = []
        for tx, ty, flags, house, zones, items in tiles:
            if (tx, ty) == (x, y):
                found = True
                if (
                    flags
                    or house
                    or zones
                    or palette[items[0][0]]["source_item_id"] != capture["base_ground"]
                ):
                    raise ValueError("M19_LANDMARK_ROOM_CHANGED")
                if items == [(items[0][0], 0, None)]:
                    items = [*items, (item_palette, 0, None)]
                    added = 1
                elif items != [(items[0][0], 0, None), (item_palette, 0, None)]:
                    raise ValueError("M19_LANDMARK_OCCUPIED")
            rewritten.append((tx, ty, flags, house, zones, items))
        encoded[local] = codec.encode_sector_tiles(
            rx * 8 + local % 8, ry * 8 + local // 8, rewritten
        )
    if not found:
        raise ValueError("M19_LANDMARK_TILE_ABSENT")
    after = codec.encode_region(zz, rx, ry, encoded)
    outputs[path] = after
    region = next(row for row in index["regions"] if row["path"] == path)
    region["sha256"] = hashlib.sha256(after).hexdigest()
    region["items"] += added
    index["totals"]["items"] += added
    index["source"]["qready1_clock_landmark"] = {
        "path": CAPTURE,
        "sha256": hashlib.sha256(capture_raw).hexdigest(),
        "items_added": 1,
    }
    outputs[INDEX] = canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary["source"] = index["source"]
    summary["totals"] = index["totals"]
    summary["bytes_on_disk"] += len(after) - len(before)
    summary["palette"]["families"]["item"]["occurrences"] += added
    outputs[SUMMARY] = canonical(summary)


def donor_delta(summary):
    """Exclude exactly the checked authored addition from the donor's unchanged pin."""
    declared = summary.get("source", {}).get("qready1_clock_landmark")
    if declared is None:
        return 0
    raw = (ROOT / CAPTURE).read_bytes()
    expected = {
        "path": CAPTURE,
        "sha256": hashlib.sha256(raw).hexdigest(),
        "items_added": 1,
    }
    if declared != expected:
        raise ValueError("M19_LANDMARK_SOURCE_CHANGED")
    return 1


def validate_installed(root, index, summary):
    if not donor_delta(summary):
        return
    capture = json.loads((ROOT / CAPTURE).read_bytes())
    x, y, floor = capture["position"]
    path = "content/world/placements/" + codec.region_name(-floor, x // 256, y // 256)
    outputs = {p: (root / p).read_bytes() for p in [INDEX, SUMMARY, path]}
    before = outputs.copy()
    overlay_outputs(outputs)
    if outputs != before:
        raise ValueError("M19_LANDMARK_NOT_INSTALLED")


if __name__ == "__main__":
    capture = json.loads((ROOT / CAPTURE).read_bytes())
    x, y, floor = capture["position"]
    path = "content/world/placements/" + codec.region_name(-floor, x // 256, y // 256)
    outputs = {p: (ROOT / p).read_bytes() for p in [INDEX, SUMMARY, path]}
    overlay_outputs(outputs)
    for p, raw in outputs.items():
        (ROOT / p).write_bytes(raw)
