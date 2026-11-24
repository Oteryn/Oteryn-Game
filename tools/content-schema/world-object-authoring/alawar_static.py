"""Qualify the static appearance-only pieces in the pinned Alawar route."""

import argparse
import hashlib
import json
import sys
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec

from activity_static import OFFICIAL_SHA, encode
from client_appearance_reader import read_appearances

BLOCKING = {10729, 10731, 10733, 10734}
DECORATIONS = {10843, 10844, 10845}
IDS = BLOCKING | DECORATIONS
PROFILE = "OTERYN_OFFICIAL_CLIENT_ALAWAR_STATIC/v1"
PLACEMENTS = "content/world/placements/index.json"
SHARD = "content/world/terrain/terrain-08751-08757.json"


@lru_cache(maxsize=1)
def records(root=ROOT):
    artifact = f"content/assets/files/appearances-{OFFICIAL_SHA}.dat"
    appearance_bytes = (root / artifact).read_bytes()
    if hashlib.sha256(appearance_bytes).hexdigest() != OFFICIAL_SHA:
        raise ValueError("Alawar appearances digest changed")
    appearances = read_appearances(appearance_bytes)
    member_root = root / "imports/official/appearance-membership"
    admitted = json.loads((member_root / "admitted.json").read_bytes())
    membership = next(row for row in admitted["files"] if row["label"] == "client-15.30")
    member_bytes = (member_root / membership["manifest"]).read_bytes()
    members = json.loads(member_bytes)
    if (
        hashlib.sha256(member_bytes).hexdigest() != membership["manifest_sha256"]
        or membership["appearances_sha256"] != OFFICIAL_SHA
        or not IDS <= {entry[0] for entry in members["entries"]}
    ):
        raise ValueError("Alawar official appearance membership changed")
    bindings = json.loads((root / "imports/crystalserver/bindings/items.json").read_bytes())
    if IDS & {int(row["external_id"]) for row in bindings["bindings"]}:
        raise ValueError("Alawar scenery has an Item binding")
    palette = json.loads((root / PLACEMENTS).read_bytes())["palette"]
    placed = set()
    for floor in range(4, 11):
        for x in (124, 125):
            for y in (123, 124):
                path = root / f"content/world/placements/region-z{floor:02}-x{x:03}-y{y:03}.b3"
                if not path.exists():
                    continue
                _, _, _, sectors = world_region_codec.decode_region(path.read_bytes())
                for _, tiles in sectors:
                    for tx, ty, _, _, _, stack in tiles:
                        if 31925 <= tx <= 32239 and 31520 <= ty <= 31712:
                            placed.update(palette[index]["source_item_id"] for index, depth, _ in stack if depth == 0)
    if not IDS <= placed:
        raise ValueError("Alawar static scenery is missing from the source route")
    source = {
        "engine": "official_client",
        "publisher": "CipSoft",
        "product": "Tibia-client",
        "revision": OFFICIAL_SHA,
        "profile": PROFILE,
        "client_version": "15.30",
        "artifact": artifact,
        "membership_manifest_sha256": membership["manifest_sha256"],
        "membership_entries_sha256": members["entries_sha256"],
    }
    known = lambda value: {"state": "KNOWN", "value": value}
    unknown = {"state": "UNKNOWN"}
    result = []
    for item in sorted(IDS):
        appearance = appearances[item]
        flags = {"unmove", "unpass"} if item in BLOCKING else {"unmove"}
        if appearance.flags != flags:
            raise ValueError(f"Alawar scenery flags changed for appearance {item}")
        automap = {"shown": appearance.automap_color is not None}
        if appearance.automap_color is not None:
            automap["color"] = appearance.automap_color
        result.append(
            {
                "identity": {"family": "Terrain", "key": f"oteryn:terrain.tibia.i{item}", "revision": "definition-r1"},
                "kind": known("common"),
                "walkable": known("unpass" not in flags),
                "blocks_projectile": unknown,
                "blocks_sight": unknown,
                "floor_change": unknown,
                "automap": known(automap),
                "client_projection": known({"appearance_id": item}),
                "provenance": {
                    "source": source,
                    "source_item_id": item,
                    "route": {"owner": "Terrain", "reason": "alawar_static_appearance"},
                    "fields": {
                        "kind": [f"official_client:{OFFICIAL_SHA}.flags"],
                        "walkable": [f"official_client:{OFFICIAL_SHA}.flags.unpass"],
                        "automap": [f"official_client:{OFFICIAL_SHA}.flags.automap"],
                        "client_projection": [f"official_client:{OFFICIAL_SHA}.id"],
                    },
                },
            }
        )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    definitions = records()
    identities = {row["provenance"]["source_item_id"]: row["identity"]["key"] for row in definitions}
    palette = json.loads((ROOT / PLACEMENTS).read_bytes())
    selected = [row for row in palette["palette"] if row["source_item_id"] in identities]
    if len(selected) != len(identities) or {row["source_item_id"] for row in selected} != IDS:
        raise ValueError("Alawar scenery palette membership changed")
    for row in selected:
        row.update(key=identities[row["source_item_id"]], provisional=False)
    for path, expected in (
        (SHARD, encode({"family": "Terrain", "records": definitions})),
        (PLACEMENTS, encode(palette)),
    ):
        target = ROOT / path
        if args.check:
            if not target.is_file() or target.read_bytes() != expected:
                raise SystemExit(f"stale Alawar scenery: {path}")
        else:
            target.write_bytes(expected)


if __name__ == "__main__":
    main()
