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
    ((32966, 32626, 5), 4634, "action", 50022),
    ((32968, 32630, 7), 1720, "action", 50022),
    ((32968, 32629, 7), 1949, "action", 50022),
    ((32967, 32630, 8), 1949, "action", 50022),
    ((33190, 31684, 14), 5102, "action", 1070),
    ((33195, 31684, 14), 5102, "action", 1070),
    ((32573, 32121, 7), 2772, "unique", 30034),
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
            "data-global/startup/tables/teleport.lua": "TeleportAction = {[50022] = {{itemId=1720,itemPos={{x=32968,y=32630,z=7}}},{itemId=1949,itemPos={{x=32968,y=32629,z=7},{x=32967,y=32630,z=8}}}}}",
            "data-global/startup/tables/door_level.lua": "LevelDoorAction = {[1070] = {itemId=false,itemPos={{x=33190,y=31684,z=14},{x=33195,y=31684,z=14}}}}",
            "data-global/startup/tables/lever.lua": "LeverUnique = {[30034] = {itemId=2772,itemPos={x=32573,y=32121,z=7}}}",
            "data/libs/tables/doors.lua": "LevelDoorTable = {{closedDoor=5102,openDoor=5103}}",
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, text in sources.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
            capture = startup.capture(root)
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

    def test_native_startup_selectors_match_exact_source_items(self):
        index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
        for (x, y, z), appearance, attribute, expected in EXPECTED:
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
