"""Lower the donor's startup UID assignments against the admitted B3 map."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))
import world_region_codec as codec

DONOR = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua"
SOURCE = "content/quests/world-callbacks-held/batch3-a-in_service_of_yalahar.json"
WORLD = "content/quests/world-callbacks/n4-yalahar-mechanisms.json"
# uid, startup row line, source x, source y, z, item id
MECHANISMS = (
    (1018, 3795, 32775, 31118, 7, 8618),
    (1019, 3800, 32778, 31118, 7, 8618),
    (3091, 4294, 32743, 31164, 5, 8618),
    (3092, 4298, 32743, 31161, 5, 8618),
    (3093, 4303, 32832, 31266, 5, 8618),
    (3094, 4308, 32832, 31269, 5, 8618),
    (3095, 4312, 32734, 31199, 5, 8615),
    (3096, 4316, 32729, 31199, 5, 8615),
    (3097, 4320, 32775, 31145, 5, 8615),
    (3098, 4324, 32775, 31141, 5, 8615),
    (3099, 4328, 32869, 31203, 5, 8615),
    (3100, 4332, 32874, 31203, 5, 8615),
    (3101, 4336, 32857, 31248, 5, 8618),
    (3102, 4340, 32853, 31251, 5, 8618),
)


def generate(root):
    source = json.loads((root / SOURCE).read_bytes())
    by_uid = {row["selector"]["unique"]: row for row in source["callbacks"]}
    if len(by_uid) != len(source["callbacks"]) or set(by_uid) != {
        row[0] for row in MECHANISMS
    }:
        raise ValueError("Yalahar mechanism source selector drift")
    index = json.loads((root / "content/world/placements/index.json").read_bytes())
    digests = {row["path"]: row["sha256"] for row in index["regions"]}
    palette = index["palette"]
    regions = {}

    def tile(x, y, z):
        path = (
            f"content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
        )
        if path not in regions:
            raw = (root / path).read_bytes()
            if hashlib.sha256(raw).hexdigest() != digests[path]:
                raise ValueError(f"Yalahar region digest drift: {path}")
            _, _, _, sectors = codec.decode_region(raw)
            regions[path] = {
                (tx, ty): (house, items)
                for _, tiles in sectors
                for tx, ty, _, house, _, items in tiles
            }
        return regions[path].get((x, y)), f"{path}:sha256:{digests[path]}"

    callbacks = []
    for uid, line, x, y, z, appearance in MECHANISMS:
        source_tile, placement_source = tile(x, y, z)
        if source_tile is None or source_tile[0]:
            raise ValueError(f"Yalahar mechanism {uid} is absent or inside a house")
        entries = [
            (palette[palette_index]["source_item_id"], attrs)
            for palette_index, depth, attrs in source_tile[1]
            if depth == 0
        ]
        ordinals = [
            ordinal for ordinal, (item, _) in enumerate(entries) if item == appearance
        ]
        if (
            len(ordinals) != 1
            or entries[ordinals[0]][1]
            or any((attrs or {}).keys() & {"unique", "action"} for _, attrs in entries)
        ):
            raise ValueError(
                f"Yalahar mechanism {uid} lacks an unambiguous untagged native placement"
            )
        row = by_uid[uid]
        destination = row["teleport"]
        if tile(destination["x"], destination["y"], -destination["floor"])[0] is None:
            raise ValueError(f"Yalahar mechanism {uid} landing is missing")
        callbacks.append(
            {
                **row,
                "selector": {
                    "position": {"x": x, "y": y, "floor": -z},
                    "ordinal": ordinals[0],
                    "unique": None,
                    "action": None,
                    "appearance": appearance,
                },
                "effects": {"teleport_effect": 11, "teleport_departure_effect": True},
                "sources": [
                    *row["sources"],
                    f"{DONOR}:{line}-{line + 2}",
                    placement_source,
                ],
            }
        )
    return (
        json.dumps({"schema": 1, "callbacks": callbacks}, ensure_ascii=False, indent=2)
        + "\n"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    path = root / WORLD
    text = generate(root)
    if args.check:
        if not path.is_file() or path.read_text(encoding="utf-8") != text:
            raise ValueError(f"stale Yalahar mechanism registry: {WORLD}")
    else:
        path.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
