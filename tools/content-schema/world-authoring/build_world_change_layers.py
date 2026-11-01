#!/usr/bin/env python3
"""Extract daily world-change tile facts from the exact Crystal 201189f donor.

No binary donor map enters the production tree. Blob hashes and the supported attribute
subset are checked before the source positions/stacks are emitted as content data.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from otbm_reader import read_tiles

REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
PREFIX = "data-global/world/world_changes/"
BLOBS = {
    "fury_gates/abdendriel": "c41c56415e9b25caa04bf53c4cbb98fd775a2710",
    "fury_gates/ankrahmun": "d426bf442236303c41f258d6abccc85914252aed",
    "fury_gates/carlin": "15f3fc688b62823a374c033d36d339c892fd9d95",
    "fury_gates/darashia": "4e25939682431fd70573c8bb712067300098ec16",
    "fury_gates/edron": "3ad2c77ca4a26f8a09a777beb730c07f68fd840b",
    "fury_gates/kazordoon": "4a6e4386f29bedd3d6ce9982a5de01ea67d34560",
    "fury_gates/libertybay": "c9a57e0f45fe225d8a51c9a8d7f98710726786ce",
    "fury_gates/porthope": "2232ecbc988b575df858bd4b4a47c0bbe8f2b697",
    "fury_gates/thais": "a131dd8d0962a448e4e6a357d8eeb0d5fc951f78",
    "fury_gates/venore": "1922f21ab60638f031a404dcba69b5ac3bbeece1",
    "nightmare_isle/ankrahmun-north": "face78fff455469cb80f7abbf8c076ebd5a010cc",
    "nightmare_isle/darashia-north": "a2c6504edbfb8c916370dbf0601b9f9c0228e271",
    "nightmare_isle/darashia-west": "a052564e3bd875a980725061cf3701e8afb3097c",
    "oriental_trader/ankrahmun": "67e21ffe29a55a6e26c77cd89af8775c2db153ce",
    "oriental_trader/carlin": "fe85d7157bff87723cc6ec0bfaab2e3bb1e1d93d",
    "oriental_trader/libertybay": "07715b1ff95d035901d14a330a40d698ed20a62c",
}


def extract(donor: Path) -> dict:
    layers = []
    for key, expected in sorted(BLOBS.items()):
        path = PREFIX + key + ".otbm"
        raw = (donor / path).read_bytes()
        blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        if blob != expected:
            raise ValueError(f"{path}: donor blob differs from {REVISION}")
        tiles = []

        def tile(x, y, z, flags, house, zones, items):
            if house is not None or zones or len(items) > 64:
                raise ValueError(f"{path}: unsupported tile owner at {(x, y, z)}")
            entries = []
            for appearance, depth, attrs in items:
                if depth != 0 or set(attrs) - {4, 5, 15} or attrs.get(15, 0) != 0:
                    raise ValueError(f"{path}: unsupported item attributes at {(x, y, z)}")
                entries.append([appearance, attrs.get(4), attrs.get(5)])
            tiles.append({"position": [x, y, z], "flags": flags, "entries": entries})

        facts = read_tiles(raw, tile)
        if facts.unknown_item_attrs or facts.unknown_tile_attrs:
            raise ValueError(f"{path}: unknown source attributes")
        tiles.sort(key=lambda row: (row["position"][2], row["position"][1], row["position"][0]))
        layers.append({"key": key, "source": path, "blob": blob, "tiles": tiles})
    return {"schema": "OTERYN_WORLD_CHANGE_LAYERS/v1", "source_revision": REVISION, "layers": layers}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--donor", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = extract(args.donor)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    # One tile per line keeps the source geometry reviewable without a huge indentation tax.
    lines = ["{", '  "schema": ' + json.dumps(result["schema"]) + ",",
             '  "source_revision": ' + json.dumps(REVISION) + ",", '  "layers": [']
    for index, layer in enumerate(result["layers"]):
        lines.append("    {" + ", ".join(json.dumps(k) + ": " + json.dumps(layer[k])
                     for k in ("key", "source", "blob")) + ', "tiles": [')
        lines.extend("      " + json.dumps(tile, separators=(",", ":"))
                     + ("," if i + 1 < len(layer["tiles"]) else "")
                     for i, tile in enumerate(layer["tiles"]))
        lines.append("    ]}" + ("," if index + 1 < len(result["layers"]) else ""))
    lines.extend(["  ]", "}", ""])
    args.out.write_text("\n".join(lines))


if __name__ == "__main__":
    main()
