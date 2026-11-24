"""Admit the client-confirmed ground connecting Crystal's vestibule entry and Bloodshade."""

import argparse

import summer_ground

# Crystal movements_blood_entrance.lua:4 and world-npc.xml:3182-3183.
# The spawn ground catalogues already supply the other 13 appearances.
IDS = (40198, 40232)
SHARD = "content/world/terrain/terrain-08886-08887.json"
NOTE = " Additionally two appearance-only Blood Vestibule ground records."


def outputs(root=summer_ground.ROOT):
    return summer_ground.outputs(root, identifiers=IDS, shard=SHARD, note=NOTE)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = summer_ground.ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Blood Vestibule ground: {name}")
        else:
            path.write_bytes(raw)
