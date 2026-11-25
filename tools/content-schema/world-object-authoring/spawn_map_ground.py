"""Admit Crystal-backed walkable ground for the third spawn-map slice."""

import argparse
import hashlib
import json
import sys
from functools import lru_cache
from pathlib import Path

import activity_static

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from client_appearance_reader import read_appearances

CAPTURE = ROOT / "tools/world-bundle-compiler/spawn-map-slice-3.json"
GROUND_IDS = tuple(json.loads(CAPTURE.read_bytes())["ground_ids"])
SHARD = "content/world/terrain/terrain-08846-08885.json"
PLACEMENTS = "content/world/placements/index.json"
SOURCE_DIGEST = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
PROFILE = "OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_GROUND/v1"
NOTE = " Additionally 40 donor-verified ground definitions for spawn-map slice 3."


def canonical(value):
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def known(value):
    return {"state": "KNOWN", "value": value}


@lru_cache(maxsize=1)
def records():
    if len(GROUND_IDS) != 40 or list(GROUND_IDS) != sorted(set(GROUND_IDS)):
        raise ValueError("spawn ground IDs must be unique and ascending")
    artifact = f"content/assets/files/appearances-{SOURCE_DIGEST}.dat"
    raw = (ROOT / artifact).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_DIGEST:
        raise ValueError("spawn ground official appearances digest")
    appearances = read_appearances(raw)
    membership_dir = ROOT / "imports/official/appearance-membership"
    membership = next(
        row
        for row in json.loads((membership_dir / "admitted.json").read_bytes())["files"]
        if row["label"] == "client-15.30"
    )
    raw = (membership_dir / membership["manifest"]).read_bytes()
    if (
        membership["appearances_sha256"] != SOURCE_DIGEST
        or hashlib.sha256(raw).hexdigest() != membership["manifest_sha256"]
    ):
        raise ValueError("spawn ground official appearance membership digest")
    members = json.loads(raw)
    if set(GROUND_IDS) - {row[0] for row in members["entries"]}:
        raise ValueError("spawn ground lacks official appearance membership")
    source = {
        "engine": "official_client",
        "publisher": "CipSoft",
        "product": "Tibia-client",
        "revision": SOURCE_DIGEST,
        "client_version": "15.30",
        "artifact": artifact,
        "membership_manifest_sha256": membership["manifest_sha256"],
        "membership_entries_sha256": members["entries_sha256"],
    }
    result = {}
    for identifier in GROUND_IDS:
        appearance = appearances[identifier]
        if (
            appearance.flags != {"bank", "unmove"}
            or appearance.speed is None
            or not 0 < appearance.speed <= 1000
        ):
            raise ValueError(f"source appearance {identifier} is not walkable ground")
        provenance = {
            "source": {
                **source,
                "profile": PROFILE,
            },
            "source_item_id": identifier,
            "route": {
                "owner": "Terrain",
                "reason": "appearance_only_ground",
            },
            "fields": {
                field: [f"official_client:{SOURCE_DIGEST}.{flag}"]
                for field, flag in [
                    ("kind", "flags.bank"),
                    ("walkable", "flags.unpass"),
                    ("ground_speed", "bank.waypoints"),
                    ("automap", "flags.automap"),
                    ("client_projection", "id"),
                ]
            },
        }
        automap = {"shown": appearance.automap_color is not None}
        if appearance.automap_color is not None:
            automap["color"] = appearance.automap_color
        result[identifier] = {
            "identity": {
                "family": "Terrain",
                "key": f"oteryn:terrain.tibia.i{identifier}",
                "revision": "definition-r1",
            },
            "kind": known("ground"),
            "walkable": known(True),
            "ground_speed": known(appearance.speed),
            "blocks_projectile": {"state": "UNKNOWN"},
            "blocks_sight": {"state": "UNKNOWN"},
            "floor_change": {"state": "UNKNOWN"},
            "automap": known(automap),
            "client_projection": known({"appearance_id": identifier}),
            "provenance": provenance,
        }
    return result


def terrain_output():
    return canonical({"family": "Terrain", "records": list(records().values())})


def outputs():
    grounds = records()
    index = json.loads((ROOT / PLACEMENTS).read_bytes())
    for row in index["palette"]:
        record = grounds.get(row["source_item_id"])
        if record is None:
            continue
        key = record["identity"]["key"]
        if not row["provisional"] and row["key"] != key:
            raise ValueError(
                f"spawn ground overlaps an admitted palette key: {row['key']}"
            )
        row.update(key=key, provisional=False)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((ROOT / marker_path).read_bytes())
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    marker["notes"] = activity_static.compose_terrain_notes(marker["notes"])
    return {
        SHARD: terrain_output(),
        PLACEMENTS: canonical(index),
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f"stale spawn ground: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
