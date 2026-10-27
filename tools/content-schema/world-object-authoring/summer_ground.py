"""Decision 21: admit appearance-only Summer ground, without an Item alias."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from client_appearance_reader import read_appearances

IDENTIFIER = 44576
KEY = f"oteryn:terrain.tibia.i{IDENTIFIER}"
SHARD = "content/world/terrain/terrain-08612-08612.json"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
MARKER_NOTE = " Additionally one decision-21 appearance-only Summer ground; total 8613 Terrain records."


def outputs(root=ROOT):
    manifest = json.loads(
        (root / "imports/official/client-assets/15.30/manifest.json").read_bytes()
    )
    file = next(
        row
        for row in manifest["files"]
        if row["name"].startswith("appearances-") and row["name"].endswith(".dat")
    )
    artifact = "content/assets/files/" + file["name"]
    raw = (root / artifact).read_bytes()
    if (
        file["sha256"] != APPEARANCES_SHA256
        or hashlib.sha256(raw).hexdigest() != APPEARANCES_SHA256
    ):
        raise ValueError("official appearances digest")
    appearances = read_appearances(raw)
    appearance = appearances[IDENTIFIER]
    if "bank" not in appearance.flags or appearance.speed != 150:
        raise ValueError("Summer ground differs from the accepted appearance")
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if any(row["external_id"] == str(IDENTIFIER) for row in bindings["bindings"]):
        raise ValueError("appearance-only ground unexpectedly has an Item binding")
    admitted = json.loads(
        (root / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    membership = next(
        row for row in admitted["files"] if row["label"] == "client-15.30"
    )
    raw = (
        root / "imports/official/appearance-membership" / membership["manifest"]
    ).read_bytes()
    members = json.loads(raw)
    digest = file["sha256"]
    if (
        hashlib.sha256(raw).hexdigest() != membership["manifest_sha256"]
        or membership["appearances_sha256"] != digest
    ):
        raise ValueError("official appearance membership digest")
    if not any(row[0] == IDENTIFIER for row in members["entries"]):
        raise ValueError("Summer ground has no official appearance membership")
    known = lambda value: {"state": "KNOWN", "value": value}
    record = {
        "identity": {"family": "Terrain", "key": KEY, "revision": "definition-r1"},
        "kind": known("ground"),
        "walkable": known("unpass" not in appearance.flags),
        "ground_speed": known(appearance.speed),
        "blocks_projectile": {"state": "UNKNOWN"},
        "blocks_sight": {"state": "UNKNOWN"},
        "floor_change": {"state": "UNKNOWN"},
        "automap": known(
            {
                "shown": appearance.automap_color is not None,
                **(
                    {"color": appearance.automap_color}
                    if appearance.automap_color is not None
                    else {}
                ),
            }
        ),
        "client_projection": known({"appearance_id": IDENTIFIER}),
        "provenance": {
            "source": {
                "engine": "official_client",
                "publisher": "CipSoft",
                "product": "Tibia-client",
                "revision": digest,
                "profile": "OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_GROUND/v1",
                "client_version": "15.30",
                "artifact": artifact,
                "membership_manifest_sha256": membership["manifest_sha256"],
                "membership_entries_sha256": members["entries_sha256"],
            },
            "source_item_id": IDENTIFIER,
            "route": {"owner": "Terrain", "reason": "appearance_only_ground"},
            "fields": {
                field: [f"official_client:{digest}.{flag}"]
                for field, flag in [
                    ("kind", "flags.bank"),
                    ("walkable", "flags.unpass"),
                    ("ground_speed", "bank.waypoints"),
                    ("automap", "flags.automap"),
                    ("client_projection", "id"),
                ]
            },
        },
    }
    index_path = "content/world/placements/index.json"
    index = json.loads((root / index_path).read_bytes())
    selected = [row for row in index["palette"] if row["source_item_id"] == IDENTIFIER]
    if len(selected) != 1:
        raise ValueError("Summer ground must have one stable palette position")
    selected[0].update(key=KEY, provisional=False)
    encode = lambda value: (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    marker["notes"] = marker["notes"].removesuffix(MARKER_NOTE) + MARKER_NOTE
    marker_bytes = (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()
    return {
        SHARD: encode({"family": "Terrain", "records": [record]}),
        index_path: encode(index),
        marker_path: marker_bytes,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Summer ground: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
