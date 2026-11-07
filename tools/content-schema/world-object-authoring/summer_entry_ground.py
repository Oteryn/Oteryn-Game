"""Decision 21: admit source-pinned Summer entry ground without Item aliases."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from client_appearance_reader import read_appearances

IDENTIFIER = 52077
KEY = f"oteryn:terrain.tibia.i{IDENTIFIER}"
SHARD = "content/world/terrain/terrain-08614-08728.json"
SOURCES = "tools/content-schema/world-object-authoring/summer-entry-ground-sources.json"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
MARKER_NOTE = (
    " Additionally 115 decision-21 appearance-only Summer entry grounds;"
    " total 8729 Terrain records."
)


@lru_cache(maxsize=1)
def qualified_records(root=ROOT):
    spec = json.loads((root / SOURCES).read_bytes())
    sources = {row["id"]: row for row in spec["grounds"]}
    if len(sources) != 115 or len(spec["grounds"]) != 115 or spec["appearances_sha256"] != APPEARANCES_SHA256:
        raise ValueError("Summer entry source roster")
    manifest = json.loads(
        (root / "imports/official/client-assets/15.30/manifest.json").read_bytes()
    )
    file = next(
        row for row in manifest["files"]
        if row["name"].startswith("appearances-") and row["name"].endswith(".dat")
    )
    artifact = "content/assets/files/" + file["name"]
    raw = (root / artifact).read_bytes()
    if file["sha256"] != APPEARANCES_SHA256 or hashlib.sha256(raw).hexdigest() != APPEARANCES_SHA256:
        raise ValueError("official appearances digest")
    appearances = read_appearances(raw)
    for refused in spec["refused_spawn_points"]:
        appearance = appearances[refused["appearance_id"]]
        if not {"bank", "unpass"}.issubset(appearance.flags) or appearance.speed != refused["speed"]:
            raise ValueError("refused Summer spawn ground differs from its official collision facts")
    bindings = json.loads((root / "imports/crystalserver/bindings/items.json").read_bytes())
    if any(int(row["external_id"]) in sources for row in bindings["bindings"]):
        raise ValueError("appearance-only ground unexpectedly has an Item binding")
    admitted = json.loads(
        (root / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    membership = next(row for row in admitted["files"] if row["label"] == "client-15.30")
    raw = (root / "imports/official/appearance-membership" / membership["manifest"]).read_bytes()
    members = json.loads(raw)
    if hashlib.sha256(raw).hexdigest() != membership["manifest_sha256"] or membership["appearances_sha256"] != APPEARANCES_SHA256:
        raise ValueError("official appearance membership digest")
    member_ids = {row[0] for row in members["entries"]}
    known = lambda value: {"state": "KNOWN", "value": value}
    records = []
    for identifier, source in sorted(sources.items()):
        appearance = appearances[identifier]
        if (
            identifier not in member_ids
            or "bank" not in appearance.flags
            or "unpass" in appearance.flags
            or appearance.speed != source["speed"]
            or appearance.automap_color != source["automap_color"]
        ):
            raise ValueError(f"Summer entry ground {identifier} differs from its official facts")
        records.append({
            "identity": {"family": "Terrain", "key": f"oteryn:terrain.tibia.i{identifier}", "revision": "definition-r1"},
            "kind": known("ground"),
            "walkable": known(True),
            "ground_speed": known(appearance.speed),
            "blocks_projectile": {"state": "UNKNOWN"},
            "blocks_sight": {"state": "UNKNOWN"},
            "floor_change": {"state": "UNKNOWN"},
            "automap": known({"shown": appearance.automap_color is not None, **({"color": appearance.automap_color} if appearance.automap_color is not None else {})}),
            "client_projection": known({"appearance_id": identifier}),
            "provenance": {
                "source": {
                    "engine": "official_client", "publisher": "CipSoft", "product": "Tibia-client",
                    "revision": APPEARANCES_SHA256,
                    "profile": "OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_GROUND/v1",
                    "client_version": "15.30", "artifact": artifact,
                    "membership_manifest_sha256": membership["manifest_sha256"],
                    "membership_entries_sha256": members["entries_sha256"],
                },
                "source_item_id": identifier,
                "route": {"owner": "Terrain", "reason": "appearance_only_ground"},
                "fields": {field: [f"official_client:{APPEARANCES_SHA256}.{flag}"] for field, flag in [
                    ("kind", "flags.bank"), ("walkable", "flags.unpass"),
                    ("ground_speed", "bank.waypoints"), ("automap", "flags.automap"),
                    ("client_projection", "id"),
                ]},
            },
        })
    return tuple(records)


def outputs(root=ROOT):
    records = qualified_records(root)
    sources = {record["provenance"]["source_item_id"] for record in records}
    index_path = "content/world/placements/index.json"
    index = json.loads((root / index_path).read_bytes())
    for identifier in sources:
        selected = [row for row in index["palette"] if row["source_item_id"] == identifier]
        if len(selected) != 1:
            raise ValueError("Summer entry ground must have one stable palette position")
        selected[0].update(key=f"oteryn:terrain.tibia.i{identifier}", provisional=False)
    encode = lambda value: (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    if MARKER_NOTE not in marker["notes"]:
        marker["notes"] += MARKER_NOTE
    return {
        SHARD: (json.dumps({"family": "Terrain", "records": records}, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode(),
        index_path: encode(index),
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Summer entry ground: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
