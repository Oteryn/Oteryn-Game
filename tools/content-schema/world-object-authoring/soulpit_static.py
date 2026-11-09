"""W07: qualify the bounded Soulpit's static appearances, without Item aliases.

Crystal 201189f src/items/items.cpp:175-176,203-208,227,233,244 loads ground,
layer, speed, unpass and unmove from the same appearance fields. Soulpit geometry
is Crystal's captured B3 region, never a fabricated walkable rectangle. The
admitted official 15.30 appearance membership supplies each exact identity.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
import world_region_codec
from client_appearance_reader import read_appearances

PROFILE = "OTERYN_OFFICIAL_CLIENT_SOULPIT_STATIC/v1"
SHARD = "content/world/terrain/terrain-08614-08668.json"
LEGACY_SHARD = "content/world/terrain/terrain-soulpit-static.json"
REGION = "content/world/placements/region-z08-x126-y121.b3"
REGION_SHA256 = "0afd733ace063eb1a72d42301098639f2a88446188618ae9cbdb9f8d2e29c38a"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
IDS = frozenset(
    {
        11075,
        11077,
        11079,
        11102,
        11103,
        11104,
        11105,
        33017,
        34943,
        34946,
        34951,
        34953,
        42880,
        42887,
        42892,
        42893,
        42894,
        42895,
        42896,
        42897,
        42898,
        42899,
        42900,
        42901,
        42902,
        42909,
        42910,
        42914,
        42916,
        42917,
        42918,
        42923,
        42929,
        42930,
        42933,
        42936,
        43516,
        46629,
        46630,
        46631,
        46632,
        46633,
        46634,
        46635,
        46636,
        46637,
        46638,
        46639,
        46640,
        46641,
        46643,
        46644,
        46645,
        46646,
        48236,
    }
)
NOTE = " Additionally 55 W07 Soulpit static appearance records; total 8669 Terrain records."


def key(item_id):
    return f"oteryn:terrain.tibia.i{item_id}"


@lru_cache(maxsize=1)
def records(root=ROOT):
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
        raise ValueError("SOULPIT_APPEARANCE_DIGEST")
    appearances = read_appearances(raw)
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
        or membership["appearances_sha256"] != APPEARANCES_SHA256
        or not IDS <= {row[0] for row in members["entries"]}
    ):
        raise ValueError("SOULPIT_APPEARANCE_MEMBERSHIP")
    region = (root / REGION).read_bytes()
    if hashlib.sha256(region).hexdigest() != REGION_SHA256:
        raise ValueError("SOULPIT_GEOMETRY_DIGEST")
    index = json.loads((root / "content/world/placements/index.json").read_bytes())
    placed = set()
    for _, tiles in world_region_codec.decode_region(region)[3]:
        for x, y, _, _, _, items in tiles:
            if 32362 <= x <= 32390 and 31132 <= y <= 31162:
                placed.update(
                    index["palette"][palette]["source_item_id"]
                    for palette, _, _ in items
                )
    if not IDS <= placed:
        raise ValueError("SOULPIT_STATIC_SCOPE")
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if IDS & {int(row["external_id"]) for row in bindings["bindings"]}:
        raise ValueError("SOULPIT_STATIC_UNEXPECTED_ITEM_ALIAS")
    known = lambda value: {"state": "KNOWN", "value": value}
    result = []
    for item_id in sorted(IDS):
        appearance = appearances[item_id]
        if "unmove" not in appearance.flags:
            raise ValueError(f"SOULPIT_STATIC_MOVABLE:{item_id}")
        # Common means the ordinary static layer, not an invented wall shape.
        # Movement is a separate exact flag and remains known for every layer.
        kind = (
            "ground"
            if "bank" in appearance.flags
            else "border"
            if "clip" in appearance.flags
            else "common"
        )
        fields = {
            "kind": ["derived:soulpit_static_layer"],
            "walkable": [f"official_client:{APPEARANCES_SHA256}.flags.unpass"],
            "automap": [f"official_client:{APPEARANCES_SHA256}.flags.automap"],
            "client_projection": [f"official_client:{APPEARANCES_SHA256}.id"],
        }
        record = {
            "identity": {
                "family": "Terrain",
                "key": key(item_id),
                "revision": "definition-r1",
            },
            "kind": known(kind),
            "walkable": known("unpass" not in appearance.flags),
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
            "client_projection": known({"appearance_id": item_id}),
            "provenance": {
                "source": {
                    "engine": "official_client",
                    "publisher": "CipSoft",
                    "product": "Tibia-client",
                    "revision": APPEARANCES_SHA256,
                    "profile": PROFILE,
                    "client_version": "15.30",
                    "artifact": artifact,
                    "membership_manifest_sha256": membership["manifest_sha256"],
                    "membership_entries_sha256": members["entries_sha256"],
                },
                "source_item_id": item_id,
                "route": {"owner": "Terrain", "reason": "soulpit_static_appearance"},
                "fields": fields,
            },
        }
        if kind == "ground":
            if appearance.speed is None:
                raise ValueError(f"SOULPIT_STATIC_GROUND_SPEED:{item_id}")
            record["ground_speed"] = known(appearance.speed)
            fields["ground_speed"] = [
                f"official_client:{APPEARANCES_SHA256}.bank.waypoints"
            ]
        result.append(record)
    # Preserve previously admitted activity-map facts for the eight shared IDs.
    # Both publishers use the same pinned official appearance. Only UNKNOWN
    # leaves may gain an independently source-qualified fact; no Known fact,
    # identity, ordinal or source geometry is replaced.
    import activity_static
    _, qualified, _ = activity_static.source_records(root)
    activity = {row['identity']['key']: row['catalogue_record']
                for row in qualified if row['identity']['family'] == 'Terrain'}
    shared = 0
    for row in result:
        source = activity.get(row['identity']['key'])
        if source is None:
            continue
        shared += 1
        for field in sorted(row.keys() | source.keys()):
            if field == 'provenance' or row.get(field) == source.get(field):
                continue
            if field not in ('blocks_projectile', 'blocks_sight') or row[field] != {'state': 'UNKNOWN'} or source[field].get('state') != 'KNOWN':
                raise ValueError('SOULPIT_ACTIVITY_KNOWN_FACT_CONFLICT:' + row['identity']['key'] + '/' + field)
            row[field] = source[field].copy()
            row['provenance']['fields'][field] = source['provenance']['fields'].get(field, source['provenance']['fields']['blocks_sight'])
    if shared != 8:
        raise ValueError('SOULPIT_ACTIVITY_SHARED_SET_CHANGED')
    return result


def outputs(root=ROOT):
    rows = records(root)
    index_path = "content/world/placements/index.json"
    index = json.loads((root / index_path).read_bytes())
    import activity_static
    _, registry, _ = activity_static.source_records(root)
    activity_static.compose_placement_palette(index, registry, rows)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    marker["notes"] = activity_static.compose_terrain_notes(marker["notes"])
    encode = lambda value: (
        json.dumps(value, indent=2, ensure_ascii=False) + "\n"
    ).encode()
    index_bytes = (
        json.dumps(index, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode()
    pin_path = "crates/client-assets/src/world.rs"
    pin = (root / pin_path).read_text(encoding="utf-8")
    pin, count = re.subn(
        r'(^pub const PLACEMENT_INDEX_SHA256: &str =\s*")[0-9a-f]{64}(";)',
        lambda match: match[1] + hashlib.sha256(index_bytes).hexdigest() + match[2],
        pin,
        flags=re.MULTILINE,
    )
    if count != 1:
        raise ValueError("SOULPIT_CLIENT_PLACEMENT_PIN_COUNT")
    return {
        SHARD: encode({"family": "Terrain", "records": rows}),
        index_path: index_bytes,
        marker_path: encode(marker),
        pin_path: pin.encode(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    legacy = ROOT / LEGACY_SHARD
    if legacy.exists():
        if args.check:
            raise SystemExit(f"stale Soulpit static content: {LEGACY_SHARD}")
        if legacy.read_bytes() != generated[SHARD]:
            raise ValueError("SOULPIT_LEGACY_SHARD_DIFFERS")
    for name, raw in generated.items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Soulpit static content: {name}")
        else:
            path.write_bytes(raw)
    if not args.check and legacy.exists():
        legacy.unlink()


if __name__ == "__main__":
    main()
