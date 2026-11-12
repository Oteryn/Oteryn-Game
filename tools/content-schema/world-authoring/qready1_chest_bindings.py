"""Restore QREADY1's pinned startup selectors through the existing World overlay."""

import hashlib
import json
from pathlib import Path

import quest_access_bindings as access

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = (
    "tools/content-schema/world-authoring/samples/quest-chest-bindings/qready1.json"
)


def overlay_outputs(outputs):
    raw = (ROOT / CAPTURE).read_bytes()
    capture = json.loads(raw)
    if (
        capture["revision"] != "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
        or capture["table_sha256"]
        != "0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca"
        or capture["startup_sha256"]
        != "897c4ea5572c6adef1d2b300fd75c141910d608a093ca15c3d3bf65fcf56826c"
    ):
        raise ValueError("QREADY1 startup source pin changed")
    index = json.loads(outputs[access.INDEX])
    bindings = [
        {
            "position": r["position"],
            "item": r["appearance"],
            "attribute": "unique",
            "value": r["unique"],
        }
        for r in capture["bindings"]
    ]
    imported, byte_delta = {}, 0
    for region in index["regions"]:
        path = region["path"]
        if path not in outputs:
            continue
        previous = outputs[path]
        after, found = access.overlay_region(previous, index["palette"], bindings)
        imported.update(found)
        outputs[path] = after
        byte_delta += len(after) - len(previous)
        region["sha256"] = hashlib.sha256(after).hexdigest()
    if not imported:
        return
    previous_count = index["source"].get("quest_chests_qready1", {}).get("imported", 0)
    index["source"]["quest_chests_qready1"] = {
        "captures": [{"path": CAPTURE, "sha256": hashlib.sha256(raw).hexdigest()}],
        "imported": len(imported),
    }
    outputs[access.INDEX] = access.canonical(index)
    summary = json.loads(outputs[access.SUMMARY])
    summary["source"] = index["source"]
    summary["item_attributes"]["unique"] = (
        summary["item_attributes"].get("unique", 0) + len(imported) - previous_count
    )
    summary["bytes_on_disk"] += byte_delta
    outputs[access.SUMMARY] = access.canonical(summary)


if __name__ == "__main__":
    index = json.loads((ROOT / access.INDEX).read_bytes())
    paths = [access.INDEX, access.SUMMARY] + [r["path"] for r in index["regions"]]
    outputs = {p: (ROOT / p).read_bytes() for p in paths}
    overlay_outputs(outputs)
    for path, raw in outputs.items():
        if (ROOT / path).read_bytes() != raw:
            (ROOT / path).write_bytes(raw)
