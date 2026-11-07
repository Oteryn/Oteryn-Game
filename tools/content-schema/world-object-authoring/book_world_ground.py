"""Admit the captured Book World paper ground through the existing client-only route.

Ground50362 occurs at32551,32623,11 in summer-update-2025.otbm, member SHA256
 d4b4baeedbccdae35011e197a78cab557ec6de774aeb52a1e47cec0646e99a26.
Archive maps.7z SHA256 c770e2399f60203f87b68b11b7393cd625f19e069177787dc991b5ee8f53ab77,
Crystal00ce02a57ca5a12e48f32a3476e37471167e4c3f:data-global/world/maps.7z.
Only normalized client facts are emitted; no map or Item alias is copied.
"""
import argparse
from pathlib import Path
import summer_ground

ROOT = Path(__file__).resolve().parents[3]
IDENTIFIER = 50362
KEY = "oteryn:terrain.tibia.i50362"
SHARD = "content/world/terrain/terrain-08614-08614.json"


def outputs(root=ROOT):
    return summer_ground.outputs(
        root, identifier=IDENTIFIER, expected_speed=150, shard=SHARD,
        note=" Additionally one NPCFIX appearance-only Book World ground; total 8615 Terrain records.",
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.is_file() or path.read_bytes() != raw:
                raise SystemExit("stale Book World ground: " + name)
        elif not path.is_file() or path.read_bytes() != raw:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
