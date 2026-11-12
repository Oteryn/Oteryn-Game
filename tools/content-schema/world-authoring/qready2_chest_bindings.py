"""Restore QREADY2's source chest UIDs using the existing startup overlay."""

import hashlib
import json
import sys
from pathlib import Path

import quest_access_bindings as access

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "tools/content-schema/world-authoring/samples/qready2-chest-bindings.json"
PACKET = (
    "tools/content-schema/quest-authoring/samples/completion-matrix/qready2-cases.json"
)
PURPLE = "tools/content-schema/quest-authoring/samples/completion-matrix/qready2-purple-cases.json"


def capture(root=ROOT):
    keys = {
        c["claim_key"]
        for path in (PACKET, PURPLE)
        for c in json.loads((root / path).read_bytes())["cases"]
    }
    index = json.loads(
        (root / "content/interactions/reward_claims/index.json").read_bytes()
    )
    bindings = []
    for path in index["shards"]:
        for row in json.loads((root / path).read_bytes())["records"]:
            d = row["definition"]
            if d["identity"]["key"] not in keys:
                continue
            for p in d["placements"]:
                source = p["source_binding"]
                (uid,) = [
                    r["unique_id"]
                    for r in source["legacy_unique_ids"]
                    if r["server"] == "crystalserver"
                ]
                bindings.append(
                    {
                        "position": [source["project_position"][k] for k in "xyz"],
                        "item": p["appearance_tibia_id"],
                        "attribute": "unique",
                        "value": uid,
                        "claim": d["identity"]["key"],
                        "claim_sha256": hashlib.sha256(access.canonical(d)).hexdigest(),
                    }
                )
    return access.canonical({"schema": 1, "bindings": bindings})


def overlay_outputs(outputs, capture_bytes=None):
    raw = (ROOT / CAPTURE).read_bytes() if capture_bytes is None else capture_bytes
    if capture_bytes is None and raw != capture():
        raise ValueError("QREADY2_STARTUP_CAPTURE_DRIFT")
    access.overlay_outputs(
        outputs, raw, capture_path=CAPTURE, source_name="qready2_rewards"
    )


if __name__ == "__main__":
    (ROOT / CAPTURE).write_bytes(capture())
    if "--apply" in sys.argv:
        index = json.loads((ROOT / access.INDEX).read_bytes())
        paths = [
            access.INDEX,
            access.SUMMARY,
            *[row["path"] for row in index["regions"]],
        ]
        outputs = {path: (ROOT / path).read_bytes() for path in paths}
        overlay_outputs(outputs)
        for path, raw in outputs.items():
            if (ROOT / path).read_bytes() != raw:
                (ROOT / path).write_bytes(raw)
