"""W02 closed Sugar room physical corpus, without Item/use admission.

Crystal00ce data/items/appearances.dat, items.cpp:163-174: exact appearance.id
selects the source type; the complete official15.30 artifact independently has
identical flags for all79 selected identities. The immutable captured room is
boss_lever.lua Sugar geometry; no region bytes or palette indices are rewritten.
"""

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

IDS = frozenset(
    (
        44805,
        44814,
        44815,
        44816,
        44817,
        44818,
        44819,
        44820,
        44821,
        44822,
        44823,
        44824,
        44825,
        44826,
        44835,
        44836,
        44837,
        44838,
        44839,
        44840,
        44841,
        44842,
        44843,
        44844,
        44845,
        44846,
        44963,
        44964,
        44986,
        44991,
        44994,
        44997,
        44998,
        44999,
        45010,
        45011,
        45012,
        45015,
        45031,
        45135,
        45141,
        45145,
        45146,
        45148,
        45149,
        45150,
        45164,
        45166,
        45167,
        45168,
        45170,
        45171,
        45173,
        45174,
        45175,
        45176,
        45177,
        45178,
        45179,
        45180,
        45181,
        45182,
        45183,
        45196,
        45250,
        45566,
        45567,
        45568,
        45569,
        45573,
        45863,
        45870,
        48121,
        48228,
        48229,
        48230,
        48231,
        48232,
        48233,
    )
)
PROFILE = "OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_BOSS_ROOM/v1"
SHARD = "content/world/terrain/terrain-08614-08692.json"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CAPTURE = (
    "tools/content-schema/world-object-authoring/samples/sugar-room-source-flags.json"
)
CAPTURE_SHA256 = "adfcb0b4415e2850ab4b2a67422d1b455f3006a10d65b9305d92321826377639"
MARKER_NOTE = " Additionally 79 exact source Sugar Daddy room appearance-only Terrain records; total 8693 Terrain records."


@lru_cache(maxsize=4)
def outputs(root=ROOT):
    return qualified_outputs(root, IDS, PROFILE, SHARD, CAPTURE, CAPTURE_SHA256,
                             MARKER_NOTE, "appearance_only_boss_room")


@lru_cache(maxsize=8)
def qualified_outputs(root, ids, profile, shard, capture_path, capture_sha256,
                      marker_note, reason):
    """One bounded static corpus, verified against complete artifacts and exact source flags.

    The caller supplies a closed, independently captured id set. This produces Terrain
    only; Item identities, use hooks and generic donor-key fallbacks remain inadmissible.
    """
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
    # Crystal00ce appearance flags are byte-for-byte equal for this exact corpus
    # to the admitted official15.30 artifact; capture() verifies the donor too.
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if any(int(row["external_id"]) in ids for row in bindings["bindings"]):
        raise ValueError("Static appearance-only identity unexpectedly has Item binding")
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
    if not ids.issubset({row[0] for row in members["entries"]}):
        raise ValueError("Static corpus lacks official appearance membership")
    # The complete artifact supplies every witnessed geometry fact. There is no
    # generic donor-key fallback, Item admission or authorization of use hooks.
    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles as source

    objects = source.engine_items.load_appearance_objects(
        (root / artifact).read_bytes()
    )
    capture_raw = (root / capture_path).read_bytes()
    if hashlib.sha256(capture_raw).hexdigest() != capture_sha256:
        raise ValueError("Static exact source capture digest")
    witness = json.loads(capture_raw)
    records = []
    known = lambda value: {"state": "KNOWN", "value": value}
    for item in sorted(ids):
        flags = objects[item]["flags"]
        if witness["flags"][str(item)] != flags:
            raise ValueError("Static source geometry flags differ")
        kind = (
            "ground"
            if flags.get("flags.bank")
            else "border"
            if flags.get("flags.clip")
            else "wall"
            if flags.get("flags.unpass")
            else "common"
        )
        appearance = appearances[item]
        record = {
            "identity": {
                "family": "Terrain",
                "key": f"oteryn:terrain.tibia.i{item}",
                "revision": "definition-r1",
            },
            "kind": known(kind),
            "walkable": known(not flags.get("flags.unpass", False)),
            # Crystal src/items/items.cpp:234: unsight -> blockProjectile.
            "blocks_projectile": known(flags.get("flags.unsight", False)),
            "blocks_sight": known(flags.get("flags.unsight", False)),
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
            "client_projection": known({"appearance_id": item}),
            "provenance": {
                "source": {
                    "engine": "official_client",
                    "publisher": "CipSoft",
                    "product": "Tibia-client",
                    "revision": digest,
                    "profile": profile,
                    "client_version": "15.30",
                    "artifact": artifact,
                    "membership_manifest_sha256": membership["manifest_sha256"],
                    "membership_entries_sha256": members["entries_sha256"],
                },
                "source_item_id": item,
                "route": {"owner": "Terrain", "reason": reason},
                "fields": {
                    name: [f"official_client:{digest}.{fact}"]
                    for name, fact in [
                        ("kind", "flags.bank.clip.unpass"),
                        ("walkable", "flags.unpass"),
                        ("blocks_sight", "flags.unsight"),
                        ("blocks_projectile", "flags.unsight"),
                        ("client_projection", "id"),
                        ("automap", "flags.automap"),
                    ]
                },
            },
        }
        if kind == "ground":
            if appearance.speed is None or (
                appearance.speed == 0 and not flags.get("flags.unpass")
            ):
                raise ValueError("Static unknown ground speed")
            record["ground_speed"] = known(appearance.speed)
            record["provenance"]["fields"]["ground_speed"] = [
                f"official_client:{digest}.bank.waypoints"
            ]
        records.append(record)
    index_path = "content/world/placements/index.json"
    index = json.loads((root / index_path).read_bytes())
    for item in sorted(ids):
        selected = [row for row in index["palette"] if row["source_item_id"] == item]
        if len(selected) != 1:
            raise ValueError(
                "Static corpus requires one stable palette index per identity"
            )
        selected[0].update(key=f"oteryn:terrain.tibia.i{item}", provisional=False)
    encode = lambda value: (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    if marker_note not in marker["notes"]:
        marker["notes"] += marker_note
    marker_bytes = (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()
    return {
        shard: encode({"family": "Terrain", "records": records}),
        index_path: encode(index),
        marker_path: marker_bytes,
    }


def capture(donor_dat, donor_xml):
    """Regenerate normalized witnesses from complete pinned donor artifacts."""
    dat = donor_dat.read_bytes()
    xml = donor_xml.read_bytes()
    if (
        hashlib.sha256(dat).hexdigest()
        != "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
    ):
        raise ValueError("Sugar donor appearance digest")
    if (
        hashlib.sha256(xml).hexdigest()
        != "13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece"
    ):
        raise ValueError("Sugar donor XML digest")
    sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles as source

    if IDS & source.xml_index(xml).keys():
        raise ValueError("Sugar corpus has unexpected XML behavior")
    objects = source.engine_items.load_appearance_objects(dat)
    document = {
        "source_revision": "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
        "appearances_sha256": hashlib.sha256(dat).hexdigest(),
        "items_xml_sha256": hashlib.sha256(xml).hexdigest(),
        "flags": {str(item): objects[item]["flags"] for item in sorted(IDS)},
    }
    raw = (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode()
    (ROOT / CAPTURE).write_bytes(raw)
    import re

    path = Path(__file__)
    path.write_text(
        re.sub(
            r'^CAPTURE_SHA256 = "[^"]+"$',
            'CAPTURE_SHA256 = "' + hashlib.sha256(raw).hexdigest() + '"',
            path.read_text(),
            flags=re.MULTILINE,
        )
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--donor-dat", type=Path)
    parser.add_argument("--donor-xml", type=Path)
    args = parser.parse_args()
    if args.donor_dat or args.donor_xml:
        if not (args.donor_dat and args.donor_xml):
            raise ValueError("both donor artifacts required")
        capture(args.donor_dat, args.donor_xml)
        return
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Sugar room: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
