"""SRV7 / Decision21: canonical Terrain admission for the Cake bridge ground48227.

This resolves the prior map-owner hold without inventing a canonical Item alias.
Official client ground flags are the physical authority; Crystal's pinned Cake
producer identifies the bridge's source use and coordinates.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from functools import lru_cache
from pathlib import Path

from summer_entry_ground import APPEARANCES_SHA256, read_appearances

ROOT = Path(__file__).resolve().parents[3]
SHARD = "content/world/terrain/terrain-08729-08729.json"
KEY = "oteryn:terrain.tibia.i48227"
NOTE = " SRV7 Decision21 admits Cake bridge ground48227 through Terrain, without an Item alias; total8730 Terrain records."

PRODUCER = 'zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/a_piece_of_cake_config.lua:249-302'
PRIOR_HOLD_RESOLUTION = 'PR2781 answer6549960114 required canonical Item/map-owner qualification; SRV7 qualifies the map-owned Terrain route under Decision21 and retains no Item alias'


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()


@lru_cache(maxsize=1)
def qualified_record(root=ROOT):
    manifest = json.loads((root / "imports/official/client-assets/15.30/manifest.json").read_bytes())
    source = next(row for row in manifest["files"] if row["name"].startswith("appearances-") and row["name"].endswith(".dat"))
    artifact = "content/assets/files/" + source["name"]
    raw = (root / artifact).read_bytes()
    if source["sha256"] != APPEARANCES_SHA256 or hashlib.sha256(raw).hexdigest() != APPEARANCES_SHA256:
        raise ValueError("CAKE_BRIDGE_OFFICIAL_APPEARANCE_DIGEST")
    appearance = read_appearances(raw).get(48227)
    if appearance is None or not {"bank", "unmove"}.issubset(appearance.flags) or "unpass" in appearance.flags or appearance.speed != 150 or appearance.automap_color != 121:
        raise ValueError("CAKE_BRIDGE_OFFICIAL_GROUND_FACTS")
    admission = json.loads((root / "imports/official/appearance-membership/admitted.json").read_bytes())
    membership = next(row for row in admission["files"] if row["label"] == "client-15.30")
    member_raw = (root / "imports/official/appearance-membership" / membership["manifest"]).read_bytes()
    members = json.loads(member_raw)
    if hashlib.sha256(member_raw).hexdigest() != membership["manifest_sha256"] or membership["appearances_sha256"] != APPEARANCES_SHA256 or 48227 not in {row[0] for row in members["entries"]}:
        raise ValueError("CAKE_BRIDGE_OFFICIAL_MEMBERSHIP")
    bindings = json.loads((root / "imports/crystalserver/bindings/items.json").read_bytes())
    if any(int(row["external_id"]) == 48227 for row in bindings["bindings"]):
        raise ValueError("CAKE_BRIDGE_UNEXPECTED_ITEM_ALIAS")
    known = lambda value: {"state": "KNOWN", "value": value}
    return {"identity": {"family": "Terrain", "key": KEY, "revision": "definition-r1"},
            "kind": known("ground"), "walkable": known(True), "ground_speed": known(150),
            "blocks_projectile": {"state": "UNKNOWN"}, "blocks_sight": {"state": "UNKNOWN"}, "floor_change": {"state": "UNKNOWN"},
            "automap": known({"shown": True, "color": 121}), "client_projection": known({"appearance_id": 48227}),
            "provenance": {"source": {"engine": "official_client", "publisher": "CipSoft", "product": "Tibia-client",
                "revision": APPEARANCES_SHA256, "profile": "OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_GROUND/v1", "client_version": "15.30",
                "artifact": artifact, "membership_manifest_sha256": membership["manifest_sha256"], "membership_entries_sha256": members["entries_sha256"]},
                "source_item_id": 48227, "route": {"owner": "Terrain", "reason": "appearance_only_ground"},
                "fields": {field: [f"official_client:{APPEARANCES_SHA256}.{flag}"] for field, flag in [
                    ("kind", "flags.bank"), ("walkable", "flags.unpass"), ("ground_speed", "bank.waypoints"), ("automap", "flags.automap"), ("client_projection", "id")]}}}


def outputs(root=ROOT):
    record = qualified_record(root)
    path = "content/world/placements/index.json"
    index = json.loads((root / path).read_bytes())
    entries = [row for row in index["palette"] if row.get("source_item_id") == 48227]
    if len(entries) != 1:
        raise ValueError("CAKE_BRIDGE_STABLE_PALETTE_IDENTITY")
    entries[0].update(key=KEY, provisional=False)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    return {SHARD: encoded({"family": "Terrain", "records": [record]}), path: (json.dumps(index, sort_keys=True, separators=(",", ":"), ensure_ascii=False)+"\n").encode(), marker_path: encoded(marker)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    stale = []
    for path, raw in outputs().items():
        target = ROOT / path
        if args.check:
            if not target.exists() or target.read_bytes() != raw: stale.append(path)
        else: target.write_bytes(raw)
    if stale: raise SystemExit("stale Cake bridge Terrain: " + ", ".join(stale))
    print("Cake bridge Terrain qualification: ok")


if __name__ == "__main__":
    main()
