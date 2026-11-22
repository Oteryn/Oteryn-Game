"""Admit the official ground appearances confirmed around Crystal slice-one spawn anchors."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from convert_appearance_only_ids import load_client

EVIDENCE = "tools/world-bundle-compiler/spawn-reconciliation-1.json"
SHARD = "content/world/terrain/terrain-08751-08793.json"
PLACEMENTS = "content/world/placements/index.json"
MEMBERSHIP = "imports/official/appearance-membership"


def canonical(value):
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()


def known(value):
    return {"state": "KNOWN", "value": value}


def record(appearance, source):
    item_id = appearance.id
    if (
        "bank" not in appearance.flags
        or "unpass" in appearance.flags
        or not appearance.speed
    ):
        raise ValueError(f"expected walkable ground for donor appearance {item_id}")
    color = appearance.automap_color
    automap = {"shown": color is not None}
    if color is not None:
        automap["color"] = color
    return {
        "identity": {
            "family": "Terrain",
            "key": f"oteryn:terrain.tibia.i{item_id}",
            "revision": "definition-r1",
        },
        "kind": known("ground"),
        "walkable": known(True),
        "ground_speed": known(appearance.speed),
        "blocks_projectile": {"state": "UNKNOWN"},
        "blocks_sight": {"state": "UNKNOWN"},
        "floor_change": {"state": "UNKNOWN"},
        "automap": known(automap),
        "client_projection": known({"appearance_id": item_id}),
        "provenance": {
            "source": source,
            "source_item_id": item_id,
            "route": {"owner": "Terrain", "reason": "appearance_only_ground"},
            "fields": {
                field: [f"official_client:{source['revision']}.{path}"]
                for field, path in [
                    ("kind", "flags.bank"),
                    ("walkable", "flags.unpass"),
                    ("ground_speed", "bank.waypoints"),
                    ("automap", "flags.automap"),
                    ("client_projection", "id"),
                ]
            },
        },
    }


def terrain_output(root=ROOT):
    evidence = json.loads((root / EVIDENCE).read_bytes())
    ids = evidence["ground_appearances"]
    if ids != sorted(set(ids)) or len(ids) != 43:
        raise ValueError("invalid slice-one ground appearance inventory")
    appearances, client = load_client(root)
    artifact = client["files"][0]
    admitted = json.loads((root / MEMBERSHIP / "admitted.json").read_bytes())
    membership = next(
        row for row in admitted["files"] if row["label"] == "client-15.30"
    )
    raw = (root / MEMBERSHIP / membership["manifest"]).read_bytes()
    members = json.loads(raw)
    if (
        hashlib.sha256(raw).hexdigest() != membership["manifest_sha256"]
        or membership["appearances_sha256"] != artifact["sha256"]
        or not set(ids) <= {row[0] for row in members["entries"]}
    ):
        raise ValueError(
            "ground appearances differ from the admitted client membership"
        )
    source = {
        "engine": "official_client",
        "publisher": "CipSoft",
        "product": "Tibia-client",
        "revision": artifact["sha256"],
        "profile": "OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_GROUND/v1",
        "client_version": client["client_version"],
        "artifact": artifact["path"],
        "membership_manifest_sha256": membership["manifest_sha256"],
        "membership_entries_sha256": members["entries_sha256"],
    }
    records = [record(appearances[item_id], source) for item_id in ids]
    return canonical({"family": "Terrain", "records": records})


def outputs(root=ROOT):
    terrain = terrain_output(root)
    index = json.loads((root / PLACEMENTS).read_bytes())
    entries = {row["source_item_id"]: row for row in index["palette"]}
    for row in json.loads(terrain)["records"]:
        item_id = row["provenance"]["source_item_id"]
        key = row["identity"]["key"]
        entry = entries[item_id]
        if (entry["key"], entry["provisional"]) not in {
            (f"donor:crystalserver@00ce02a5:item/{item_id}", True),
            (key, False),
        }:
            raise ValueError(
                f"slice-one ground already has a different route: {item_id}"
            )
        entry.update(key=key, provisional=False)
    return {SHARD: terrain, PLACEMENTS: canonical(index)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, data in outputs().items():
        target = ROOT / path
        if not args.check:
            target.write_bytes(data)
        elif target.read_bytes() != data:
            raise SystemExit(f"stale spawn ground: {path}")


if __name__ == "__main__":
    main()
