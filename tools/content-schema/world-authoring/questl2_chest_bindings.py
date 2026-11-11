"""Restore L2 startup metadata through the existing world overlay.

Reuse: QREADY1 c4266e5a's overlay pattern and quest_access_bindings.overlay_region.
Crystal9f5a72c data-global/startup/tables/chest.lua:992, SHA256
0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca.
"""

import argparse
import hashlib
import json

import quest_access_bindings as access

REGION = "content/world/placements/region-z08-x125-y124.b3"
BINDINGS = [
    {"position": [32248, 31866, 8], "item": 2476, "attribute": "unique", "value": 6055}
]
SOURCE = "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/startup/tables/chest.lua:992:sha256:0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca"


def overlay_outputs(outputs):
    if REGION not in outputs:
        return
    index = json.loads(outputs[access.INDEX])
    previous = outputs[REGION]
    after, imported = access.overlay_region(previous, index["palette"], BINDINGS)
    if len(imported) != len(BINDINGS):
        raise ValueError("L2 startup chest binding is incomplete")
    outputs[REGION] = after
    for region in index["regions"]:
        if region["path"] == REGION:
            region["sha256"] = hashlib.sha256(after).hexdigest()
    old = index["source"].get("questl2_chests", {}).get("imported", 0)
    index["source"]["questl2_chests"] = {"source": SOURCE, "imported": len(imported)}
    outputs[access.INDEX] = access.canonical(index)
    summary = json.loads(outputs[access.SUMMARY])
    summary["source"] = index["source"]
    summary["item_attributes"]["unique"] = (
        summary["item_attributes"].get("unique", 0) + len(imported) - old
    )
    summary["bytes_on_disk"] += len(after) - len(previous)
    outputs[access.SUMMARY] = access.canonical(summary)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("content", nargs="?", default="content")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = [access.INDEX, access.SUMMARY, REGION]
    outputs = {path: (access.ROOT / path).read_bytes() for path in paths}
    overlay_outputs(outputs)
    for path, raw in outputs.items():
        target = access.ROOT / path
        if target.read_bytes() != raw:
            if args.check:
                raise SystemExit("L2 startup chest binding is stale: " + path)
            target.write_bytes(raw)
