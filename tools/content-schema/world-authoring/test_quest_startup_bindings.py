"""Native selectors must exist on exact source Items, not just PG fixture tiles."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import quest_startup_bindings as startup
import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
EXPECTED = (
    ((33152, 32866, 8), 2772, "action", 5633),
    ((32035, 31642, 8), 7042, "action", 4503),
    ((32107, 31568, 9), 7033, "action", 4502),
    ((32180, 31633, 8), 2772, "unique", 30008),
    ((32966, 32626, 5), 4634, "action", 50022),
    ((32968, 32630, 7), 1720, "action", 50022),
    ((32968, 32629, 7), 1949, "action", 50022),
    ((32967, 32630, 8), 1949, "action", 50022),
    ((33190, 31684, 14), 5102, "action", 1070),
    ((33195, 31684, 14), 5102, "action", 1070),
    ((32573, 32121, 7), 2772, "unique", 30034),
    ((33290, 31785, 13), 776, "action", 9715),
    ((33498, 32613, 8), 10840, "unique", 35020),
    ((32209, 32292, 6), 1949, "action", 4253),
    ((32210, 32292, 6), 1949, "action", 4253),
)

NIGHTMARE = (
    ((33472, 32645, 9), 20142, "action", 64103),
    ((33473, 32645, 9), 20142, "action", 64103),
    ((33474, 32641, 10), 20143, "action", 64104),
    ((33456, 32579, 8), 20142, "action", 64105),
    ((33462, 32585, 8), 20143, "action", 64106),
    ((33419, 32588, 10), 20142, "action", 64107),
    ((33419, 32602, 10), 20142, "action", 64108),
    ((33420, 32602, 10), 20142, "action", 64108),
    ((33430, 32599, 10), 20143, "action", 64109),
    ((33439, 32619, 10), 10842, "action", 64120),
    ((33452, 32619, 11), 10840, "action", 64121),
    ((33465, 32636, 10), 20143, "action", 64121),
    ((33416, 32625, 10), 20142, "action", 64122),
    ((33434, 32631, 8), 20143, "action", 64123),
    ((33429, 32633, 8), 20142, "action", 64124),
    ((33483, 32628, 8), 20142, "action", 64125),
    ((33478, 32622, 11), 20142, "action", 64126),
    ((33453, 32628, 8), 20142, "action", 64127),
    ((33420, 32582, 9), 20142, "action", 64128),
)


class QuestStartupBindingsTests(unittest.TestCase):
    def outputs(self):
        index = json.loads((ROOT / startup.doors.INDEX).read_bytes())
        document = json.loads((ROOT / startup.CAPTURE).read_bytes())
        paths = [
            startup.doors.INDEX.as_posix(),
            startup.SUMMARY,
            *(r["path"] for r in startup.affected(index, document)),
        ]
        return {p: (ROOT / p).read_bytes() for p in paths}

    def test_startup_overlay_is_idempotent_and_rejects_ambiguous_items(self):
        outputs = self.outputs()
        before = outputs.copy()
        startup.overlay_outputs(outputs, require_all=True)
        self.assertEqual(outputs, before)
        name = "content/world/placements/region-z07-x127-y125.b3"
        z, rx, ry, sectors = codec.decode_region(outputs[name])
        for _, tiles in sectors:
            for x, y, _, _, _, items in tiles:
                if (x, y) == (32573, 32121):
                    items.extend(
                        [
                            entry
                            for entry in items
                            if (entry[2] or {}).get("unique") == 30034
                        ]
                    )
        outputs[name] = codec.encode_region(
            z,
            rx,
            ry,
            {
                local: codec.encode_sector_tiles(
                    rx * 8 + local % 8, ry * 8 + local // 8, tiles
                )
                for local, tiles in sectors
            },
        )
        with self.assertRaisesRegex(ValueError, "missing or ambiguous"):
            startup.overlay_outputs(outputs, require_all=True)

    def test_capture_parses_pinned_source_shapes_and_hashes_inputs(self):
        # Literal source shapes; CI must not depend on a worker's local donor cache.
        sources = {
            "data-global/startup/tables/tile.lua": "TileAction = {[50022] = {itemId=4634, itemPos={{x=32966,y=32626,z=5}}}}",
            "data-global/startup/tables/teleport.lua": """
                TeleportAction = {
                    [4253] = {itemId=1949,itemPos={{x=32209,y=32292,z=6},{x=32210,y=32292,z=6}}},
                    [50022] = {
                        {itemId=1720,itemPos={{x=32968,y=32630,z=7}}},
                        {itemId=1949,itemPos={{x=32968,y=32629,z=7},{x=32967,y=32630,z=8}}},
                    },
                }
                TeleportUnique = {[35020] = {itemId=10840,itemPos={x=33498,y=32613,z=8}}}
            """,
            "data-global/startup/tables/door_level.lua": "LevelDoorAction = {[1070] = {itemId=false,itemPos={{x=33190,y=31684,z=14},{x=33195,y=31684,z=14}}}}",
            "data-global/startup/tables/lever.lua": "LeverAction = {[5633] = {itemId=2772,itemPos={{x=33152,y=32866,z=8}}}} LeverUnique = {[30008] = {itemId=2772,itemPos={x=32180,y=31633,z=8}}, [30034] = {itemId=2772,itemPos={x=32573,y=32121,z=7}}}",
            "data-global/startup/tables/item.lua": "ItemAction = {[4503] = {{itemId=2970,itemPos={{x=32030,y=31688,z=8}}},{itemId=7042,itemPos={{x=32035,y=31642,z=8}}}}, [9715] = {itemId=776,itemPos={{x=33290,y=31785,z=13}}}}",
            "data-global/startup/tables/door_key.lua": "KeyDoorAction = {[Storage.Quest.Key.ID3142] = {itemId=false,itemPos={{x=32450,y=32044,z=8}}}, [Storage.Quest.Key.ID4502] = {itemId=false,itemPos={{x=32107,y=31568,z=9}}}}",
            "data-global/lib/core/storages.lua": "Storage = {Quest={Key={ID4502=4502,ID3142=3142}}}",
            "data/libs/tables/doors.lua": "LevelDoorTable = {{closedDoor=5102,openDoor=5103}}",
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, text in sources.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
            capture = startup.capture(root, startup.SPECS)
        actual = [
            (tuple(r["position"]), r["item"], r["attribute"], r["action"])
            for r in capture["bindings"]
        ]
        self.assertEqual(sorted(actual), sorted(EXPECTED))
        self.assertEqual(
            capture["sources"],
            [
                {"path": name, "sha256": hashlib.sha256(text.encode()).hexdigest()}
                for name, text in sources.items()
            ],
        )

    def test_serpentine_lever_has_its_source_action(self):
        self.assert_native_startup_selectors(
            (((33152, 32866, 8), 2772, "action", 5633),)
        )

    def test_native_startup_selectors_match_exact_source_items(self):
        self.assert_native_startup_selectors((*EXPECTED, *NIGHTMARE))

    def assert_native_startup_selectors(self, selectors):
        index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
        for (x, y, z), appearance, attribute, expected in selectors:
            with self.subTest(position=(x, y, z)):
                raw = (
                    ROOT
                    / f"content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
                ).read_bytes()
                _, _, _, sectors = codec.decode_region(raw)
                tiles = [t for _, tiles in sectors for t in tiles if t[:2] == (x, y)]
                self.assertEqual(len(tiles), 1)
                self.assertEqual(tiles[0][3], 0)
                items = [
                    attrs
                    for palette, depth, attrs in tiles[0][5]
                    if depth == 0
                    and index["palette"][palette]["source_item_id"] == appearance
                ]
                self.assertEqual(len(items), 1)
                self.assertEqual((items[0] or {}).get(attribute), expected)


if __name__ == "__main__":
    unittest.main()
