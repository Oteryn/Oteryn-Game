"""M52: qualify the retained full-moon map's fixed appearance-only decoration.

The existing World palette position is retained. This grants no portable Item,
USE action, quest completion or source map redistribution.
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-migration"))
import feroxa_maps

sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from client_appearance_reader import read_appearances

IDENTIFIER = 13841
KEY = f"oteryn:terrain.tibia.i{IDENTIFIER}"
PROFILE = "OTERYN_FEROXA_FIXED_SOURCE_SCENERY/v1"
SHARD = "content/world/terrain/terrain-08748-08748.json"
NOTE = " Additionally one M52 source-qualified full-moon static decoration."


def records(root=ROOT):
    capture = feroxa_maps.load(root)
    physical = next(
        row for row in capture["palette"] if row["server_item_id"] == IDENTIFIER
    )
    placed = [
        (phase["name"], tile[:3])
        for phase in capture["phases"]
        for tile in phase["tiles"]
        if IDENTIFIER in tile[4]
    ]
    if placed != [
        ("middle", [33427, 31532, 11]),
        ("middle", [33424, 31539, 11]),
        ("middle", [33426, 31541, 11]),
    ]:
        raise ValueError("FEROXA_STATIC_SOURCE_SCOPE")
    if any(
        physical[key]
        for key in (
            "ground",
            "movable",
            "pickupable",
            "container",
            "floor_change",
            "block_solid",
            "block_projectile",
        )
    ):
        raise ValueError("FEROXA_STATIC_PHYSICAL_SOURCE")
    digest = capture["physical_sources"]["appearances_sha256"]
    artifact = f"content/assets/files/appearances-{digest}.dat"
    raw = (root / artifact).read_bytes()
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError("FEROXA_STATIC_APPEARANCE_DIGEST")
    appearance = read_appearances(raw)[IDENTIFIER]
    if appearance.flags != frozenset({"unmove", "clip"}):
        raise ValueError("FEROXA_STATIC_APPEARANCE_FLAGS")
    admitted = json.loads(
        (root / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    membership = next(
        row for row in admitted["files"] if row["label"] == "client-15.30"
    )
    member_raw = (
        root / "imports/official/appearance-membership" / membership["manifest"]
    ).read_bytes()
    members = json.loads(member_raw)
    if (
        hashlib.sha256(member_raw).hexdigest() != membership["manifest_sha256"]
        or membership["appearances_sha256"] != digest
        or not any(row[0] == IDENTIFIER for row in members["entries"])
    ):
        raise ValueError("FEROXA_STATIC_MEMBERSHIP")
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if any(row["external_id"] == str(IDENTIFIER) for row in bindings["bindings"]):
        raise ValueError("FEROXA_STATIC_UNEXPECTED_ITEM_ALIAS")
    known = lambda value: {"state": "KNOWN", "value": value}
    return [
        {
            "identity": {"family": "Terrain", "key": KEY, "revision": "definition-r1"},
            "kind": known("border"),
            "walkable": known(True),
            "blocks_projectile": known(False),
            "blocks_sight": known(False),
            "floor_change": {"state": "UNKNOWN"},
            "automap": known(
                dict(
                    shown=appearance.automap_color is not None,
                    **(
                        {"color": appearance.automap_color}
                        if appearance.automap_color is not None
                        else {}
                    ),
                )
            ),
            "client_projection": known({"appearance_id": IDENTIFIER}),
            "provenance": {
                "source": {
                    "engine": "official_client",
                    "publisher": "CipSoft",
                    "product": "Tibia-client",
                    "revision": digest,
                    "profile": PROFILE,
                    "client_version": "15.30",
                    "artifact": artifact,
                    "membership_manifest_sha256": membership["manifest_sha256"],
                    "membership_entries_sha256": members["entries_sha256"],
                },
                "source_item_id": IDENTIFIER,
                "route": {"owner": "Terrain", "reason": "feroxa_fixed_source_scenery"},
                "fields": {
                    field: [evidence]
                    for field, evidence in (
                        ("kind", f"official_client:{digest}.flags.clip"),
                        ("walkable", f"official_client:{digest}.flags.unpass"),
                        (
                            "blocks_projectile",
                            f"official_client:{digest}.flags.unsight",
                        ),
                        ("blocks_sight", f"official_client:{digest}.flags.unsight"),
                        ("automap", f"official_client:{digest}.flags.automap"),
                        ("client_projection", f"official_client:{digest}.id"),
                    )
                },
            },
        }
    ]


def outputs(root=ROOT):
    rows = records(root)
    index_path = "content/world/placements/index.json"
    index = json.loads((root / index_path).read_bytes())
    selected = [row for row in index["palette"] if row["source_item_id"] == IDENTIFIER]
    if len(selected) != 1:
        raise ValueError("FEROXA_STATIC_PALETTE_IDENTITY")
    selected[0].update(key=KEY, provisional=False)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    encode = lambda value: (
        json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode()
    index_bytes = encode(index)
    pin_path = "crates/client-assets/src/world.rs"
    pin, count = re.subn(
        r'(^pub const PLACEMENT_INDEX_SHA256: &str =\s*")[0-9a-f]{64}(";)',
        lambda match: match[1] + hashlib.sha256(index_bytes).hexdigest() + match[2],
        (root / pin_path).read_text(),
        flags=re.MULTILINE,
    )
    if count != 1:
        raise ValueError("FEROXA_STATIC_CLIENT_PIN_COUNT")
    return {
        SHARD: encode({"family": "Terrain", "records": rows}),
        index_path: index_bytes,
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode(),
        pin_path: pin.encode(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Feroxa scenery: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
