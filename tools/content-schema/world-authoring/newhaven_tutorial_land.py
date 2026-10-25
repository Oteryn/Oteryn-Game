#!/usr/bin/env python3
"""TUTORIAL-1 exact land mask, reusing the committed island converter's B3 rules."""

import argparse
from pathlib import Path

import convert_islands as islands

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "crates/gameplay/content/src/content/newhaven_tutorial_land.bin"


def build():
    water, lava = islands.load_ground_classes(
        (ROOT / islands.GROUND_CLASSES).read_bytes()
    )
    ground = islands.GroundMap(ROOT, water, lava)
    bits = bytearray((129 * 118 + 7) // 8)
    for x, y, expected in [(32534, 32513, 50), (32544, 32515, 9189)]:
        tiles, _, capped = islands.land_component(ground.cls, x, y, 7)
        if capped or len(tiles) != expected:
            raise ValueError("Newhaven committed land component changed")
        for tile in tiles:
            tx, ty = tile >> 16, tile & 65535
            if not (32506 <= tx <= 32634 and 32405 <= ty <= 32522):
                raise ValueError("Newhaven land outside accepted bounds")
            index = (ty - 32405) * 129 + tx - 32506
            bits[index // 8] |= 1 << (index % 8)
    return bytes(bits)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = build()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("Newhaven tutorial land mask is stale")
    else:
        OUTPUT.write_bytes(data)
