"""Tests for the minimap draft (colour mapping, the draft rule, pins and their validation)."""

from __future__ import annotations

import io
import json
import shutil
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import convert_world_base as convert
import minimap_draft as draft
import test_world_authoring as fixtures
import validate_world_base as validate
from test_world_base import ITEMS_BY_SERVER_ID, ITEMS_XML

GRASS, ROCK_COLOUR, WATER_COLOUR = b"\x00\xcc\x00", b"\x66\x66\x66", b"\x33\x66\x99"
RED, YELLOW, BLACK = b"\xff\x33\x00", draft.MARKER, draft.BLACK
GREY, MAGENTA, PATH_YELLOW = b"\x96\x96\x96", b"\xff\x00\xff", b"\xff\xff\x00"
GRASS_GROUND, GRASS_GROUND2, ROCK_GROUND, WALL, TREE = 4000, 4001, 1128, 5000, 5001
SAND, STAIRS = 4002, 1010
WATER = min(convert.ground_ids()[0])
X0, Y0 = draft.AREAS[0]["bbox"][:2]
X0, Y0 = X0 + 5, Y0 + 5  # inside the area
LX, LY = X0 - 30, Y0  # learning tiles: outside the area, on the frame
FRAME = (31744, 30976, 400, 1200)


def tile(*ids):
    return (0, None, (), [(i, 0, None) for i in ids])


def images(pixels):
    """`pixels` maps `(z, x, y)` to `(map rgb, path rgb)`; the rest is black and magenta."""
    x_min, y_min, width, height = FRAME
    found = draft.Images(x_min, y_min, width, height)
    for z in draft.FLOORS:
        found.maps[z] = bytearray(BLACK * (width * height))
        found.paths[z] = bytearray(MAGENTA * (width * height))
    for (z, x, y), (colour, path) in pixels.items():
        i = ((y - y_min) * width + x - x_min) * 3
        found.maps[z][i : i + 3] = colour
        found.paths[z][i : i + 3] = path
    for z in draft.FLOORS:
        found.maps[z], found.paths[z] = bytes(found.maps[z]), bytes(found.paths[z])
    return found


def scan_of(pixels, tiles, area=None):
    area = area or {"bbox": [X0, Y0, X0 + 9, Y0 + 9], "floors": [6, 7], "name": "test"}
    scan = draft.Scanner(images(pixels), [area])
    for (x, y, z), value in tiles.items():
        scan(x, y, z, *value)
    return scan


MX = X0 - 70  # left of the test area


class MappingTest(unittest.TestCase):
    def table(self):
        pixels, tiles = {}, {}
        # walkable grass: 60 samples, the most frequent ground wins
        for i in range(60):
            pixels[(7, MX + i, LY)] = (GRASS, GREY)
            tiles[(MX + i, LY, 7)] = tile(GRASS_GROUND if i < 40 else GRASS_GROUND2)
        # blocked rock: most frequent ground, most frequent single top item
        for i in range(55):
            pixels[(7, MX + i, LY + 1)] = (ROCK_COLOUR, PATH_YELLOW)
            top = [WALL] * 30 + [TREE] * 10 + [None] * 15
            tiles[(MX + i, LY + 1, 7)] = tile(
                ROCK_GROUND, *([top[i]] if top[i] else [])
            )
        # only 49 samples: unmapped
        for i in range(49):
            pixels[(6, MX + i, LY + 2)] = (RED, GREY)
            tiles[(MX + i, LY + 2, 6)] = tile(SAND)
        # marker colour and unexplored pixels are never sampled
        pixels[(7, MX, LY + 3)] = (YELLOW, GREY)
        tiles[(MX, LY + 3, 7)] = tile(STAIRS)
        tiles[(MX, LY + 4, 7)] = tile(SAND)
        return draft.learn(scan_of(pixels, tiles))

    def test_the_table_is_learned_from_the_base_tiles(self):
        table = self.table()
        grass = table[(7, GRASS, "walkable")]
        self.assertEqual(
            (grass["ground"], grass["item"], grass["samples"], grass["mapped"]),
            (GRASS_GROUND, None, 60, True),
        )
        rock = table[(7, ROCK_COLOUR, "blocked")]
        self.assertEqual(
            (rock["ground"], rock["item"], rock["samples"]), (ROCK_GROUND, WALL, 55)
        )

    def test_an_item_that_is_absent_most_often_is_no_item(self):
        pixels, tiles = {}, {}
        for i in range(60):
            pixels[(7, MX + i, LY)] = (ROCK_COLOUR, PATH_YELLOW)
            tiles[(MX + i, LY, 7)] = tile(ROCK_GROUND, *([WALL] if i < 20 else []))
        rock = draft.learn(scan_of(pixels, tiles))[(7, ROCK_COLOUR, "blocked")]
        self.assertIsNone(rock["item"])

    def test_a_colour_below_the_minimum_is_unmapped_and_markers_are_not_learned(self):
        table = self.table()
        red = table[(6, RED, "walkable")]
        self.assertEqual((red["samples"], red["mapped"]), (49, False))
        self.assertFalse(any(key[1] in (YELLOW, BLACK) for key in table))

    def test_the_walkable_class_never_takes_an_item(self):
        pixels, tiles = {}, {}
        for i in range(60):
            pixels[(7, MX + i, LY)] = (GRASS, GREY)
            tiles[(MX + i, LY, 7)] = tile(GRASS_GROUND, TREE)
        self.assertIsNone(
            draft.learn(scan_of(pixels, tiles))[(7, GRASS, "walkable")]["item"]
        )

    def test_tiles_inside_the_drafted_area_are_not_samples(self):
        pixels, tiles = {}, {}
        for i in range(60):
            pixels[(7, X0 + i % 10, Y0 + i // 10)] = (GRASS, GREY)
            tiles[(X0 + i % 10, Y0 + i // 10, 7)] = tile(WATER)
        scan = scan_of(pixels, tiles)
        self.assertEqual(draft.learn(scan), {})
        self.assertEqual(len(scan.tiles), 60)


class PlanTest(unittest.TestCase):
    """The draft rule over an area of 10 x 10 at (X0, Y0), with the mapping learned in place."""

    def setUp(self):
        patcher = mock.patch.object(draft, "MIN_SAMPLES", 2)
        patcher.start()
        self.addCleanup(patcher.stop)

    def base(self):
        pixels, tiles = {}, {}
        for i, (colour, path, ids) in enumerate(
            [
                (GRASS, GREY, (GRASS_GROUND,)),
                (ROCK_COLOUR, PATH_YELLOW, (ROCK_GROUND, WALL)),
                (WATER_COLOUR, PATH_YELLOW, (WATER,)),
            ]
        ):
            for j in range(3):
                pixels[(7, LX + j, LY + i)] = (colour, path)
                tiles[(LX + j, LY + i, 7)] = tile(*ids)
        for j in range(3):  # floor 6 has its own table
            pixels[(6, LX + j, LY)] = (GRASS, GREY)
            tiles[(LX + j, LY, 6)] = tile(GRASS_GROUND)
        return pixels, tiles

    def plan(self, pixels, tiles, land=lambda *_: True):
        base_pixels, base_tiles = self.base()
        scan = scan_of({**base_pixels, **pixels}, {**base_tiles, **tiles})
        return draft.plan_draft(scan, scan.images, land, {WATER})

    def row(self, plan, z=7):
        return plan.stats["test"][str(z)]

    def test_empty_and_plain_water_positions_are_drafted_with_the_mapped_tile(self):
        pixels = {
            (7, X0, Y0): (GRASS, GREY),
            (7, X0 + 1, Y0): (ROCK_COLOUR, PATH_YELLOW),
            (7, X0 + 2, Y0): (GRASS, GREY),
        }
        tiles = {(X0 + 2, Y0, 7): tile(WATER)}
        plan = self.plan(pixels, tiles)
        self.assertEqual(plan.add[(X0, Y0, 7)], tile(GRASS_GROUND))
        self.assertEqual(plan.add[(X0 + 1, Y0, 7)], tile(ROCK_GROUND, WALL))
        self.assertEqual(plan.replace[(X0 + 2, Y0, 7)], tile(GRASS_GROUND))
        self.assertEqual(
            (plan.replaced_items_removed, plan.replaced_items_added), (1, 1)
        )
        row = self.row(plan)
        self.assertEqual((row["added"], row["replaced"]), (2, 1))
        self.assertEqual((row["walkable"], row["blocked"]), (2, 1))

    def test_a_base_tile_that_is_not_plain_water_is_never_replaced(self):
        pixels = {(7, X0 + i, Y0): (GRASS, GREY) for i in range(4)}
        tiles = {
            (X0, Y0, 7): tile(SAND),
            (X0 + 1, Y0, 7): tile(WATER, TREE),  # water with an item
            (X0 + 2, Y0, 7): (0, 3, (), [(WATER, 0, None)]),  # water inside a house
            (X0 + 3, Y0, 7): tile(GRASS_GROUND),
        }
        plan = self.plan(pixels, tiles)
        self.assertEqual((plan.add, plan.replace), ({}, {}))
        self.assertEqual(self.row(plan)["kept_base"], 4)

    def test_a_replaced_water_tile_keeps_its_flags(self):
        plan = self.plan(
            {(7, X0, Y0): (GRASS, GREY)},
            {(X0, Y0, 7): (1, None, (), [(WATER, 0, None)])},
        )
        self.assertEqual(plan.replace[(X0, Y0, 7)][0], 1)

    def test_unexplored_pixels_and_positions_outside_the_bbox_are_not_drafted(self):
        pixels = {(7, X0 + 20, Y0): (GRASS, GREY), (7, X0 - 6, Y0): (GRASS, GREY)}
        plan = self.plan(pixels, {})
        self.assertEqual((plan.add, plan.replace), ({}, {}))
        self.assertEqual(self.row(plan)["explored"], 0)

    def test_an_unmapped_colour_is_skipped_and_counted(self):
        pixels = {(7, X0, Y0): (RED, GREY), (7, X0 + 1, Y0): (GRASS, GREY)}
        plan = self.plan(pixels, {})
        self.assertEqual(list(plan.add), [(X0 + 1, Y0, 7)])
        self.assertEqual(self.row(plan)["unmapped"], 1)
        row = plan.table[(7, RED, "walkable")]
        self.assertEqual(
            (row["mapped"], row["samples"], row["ground"]), (False, 0, None)
        )

    def test_every_floor_needs_official_land_except_for_water(self):
        pixels = {
            (7, X0, Y0): (GRASS, GREY),
            (7, X0 + 1, Y0): (GRASS, GREY),
            (7, X0 + 2, Y0): (WATER_COLOUR, PATH_YELLOW),
            (6, X0, Y0): (GRASS, GREY),
            (6, X0 + 1, Y0): (GRASS, GREY),
        }
        plan = self.plan(
            pixels, {}, land=lambda x, y, z: (z, x) in ((7, X0 + 1), (6, X0))
        )
        self.assertEqual(
            sorted(plan.add),
            [(X0, Y0, 6), (X0 + 1, Y0, 7), (X0 + 2, Y0, 7)],
        )
        self.assertEqual(plan.add[(X0 + 2, Y0, 7)], tile(WATER))
        self.assertEqual(self.row(plan)["no_official_land"], 1)
        self.assertEqual(self.row(plan, 6)["no_official_land"], 1)
        self.assertEqual(dict(plan.on_official_land), {7: 1, 6: 1})

    def test_water_over_water_is_no_change(self):
        pixels = {(7, X0, Y0): (WATER_COLOUR, PATH_YELLOW)}
        plan = self.plan(pixels, {(X0, Y0, 7): tile(WATER)})
        self.assertEqual((plan.add, plan.replace), ({}, {}))
        self.assertEqual(self.row(plan)["water_over_water"], 1)

    def test_yellow_markers_are_listed_and_never_drafted(self):
        pixels = {
            (7, X0 + 1, Y0 + 1): (YELLOW, GREY),
            (6, X0, Y0): (YELLOW, PATH_YELLOW),
            (6, X0 + 3, Y0): (YELLOW, GREY),
        }
        plan = self.plan(
            pixels, {(X0 + 3, Y0, 6): tile(SAND)}
        )  # a base tile hides the marker
        self.assertEqual(plan.entrances, [[X0, Y0, 6], [X0 + 1, Y0 + 1, 7]])
        self.assertEqual((plan.add, plan.replace), ({}, {}))
        self.assertEqual(
            (self.row(plan)["markers"], self.row(plan, 6)["markers"]), (1, 1)
        )


class BuildAndValidateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        try:
            from PIL import Image
        except ImportError:
            raise unittest.SkipTest("Pillow is not installed") from None
        x_min, y_min, width, height = FRAME
        cls.blobs = {
            draft.blob_key("bounds.json"): json.dumps(
                {"xMin": x_min, "yMin": y_min, "width": width, "height": height}
            ).encode()
        }
        cls.pixels = cls.pixels_of()
        for z in draft.FLOORS:
            for kind, index, default in (("map", 0, BLACK), ("path", 1, MAGENTA)):
                image = Image.new("RGB", (width, height), tuple(default))
                for (pz, x, y), pair in cls.pixels.items():
                    if pz == z:
                        image.putpixel((x - x_min, y - y_min), tuple(pair[index]))
                buffer = io.BytesIO()
                image.save(buffer, "PNG")
                name = f"floor-{z:02d}-{kind}.png"
                cls.blobs[draft.blob_key(name)] = buffer.getvalue()

    @staticmethod
    def pixels_of():
        pixels = {}
        for i, (colour, path) in enumerate(
            [(GRASS, GREY), (ROCK_COLOUR, PATH_YELLOW), (WATER_COLOUR, PATH_YELLOW)]
        ):
            for j in range(3):
                pixels[(7, LX + j, LY + i)] = (colour, path)
        for j in range(3):  # floor 6 has its own table
            pixels[(6, LX + j, LY)] = (GRASS, GREY)
        pixels[(7, X0, Y0)] = (GRASS, GREY)  # added
        pixels[(7, X0 + 1, Y0)] = (GRASS, GREY)  # no official land
        pixels[(7, X0 + 2, Y0)] = (GRASS, GREY)  # replaces plain water
        pixels[(7, X0 + 3, Y0)] = (GRASS, GREY)  # base land kept
        pixels[(7, X0 + 4, Y0)] = (ROCK_COLOUR, PATH_YELLOW)  # added with its item
        pixels[(7, X0 + 5, Y0)] = (YELLOW, GREY)  # marker
        pixels[(6, X0, Y0)] = (GRASS, GREY)
        pixels[(6, X0 + 1, Y0)] = (RED, GREY)  # unmapped
        return pixels

    def otbm(self, tiles):
        """`tiles`: `{(x, y, z): ids}`; one area node per tile keeps the offsets small."""
        areas = [
            fixtures.node(
                4,
                struct.pack("<HHB", x, y, z),
                fixtures.node(
                    5,
                    bytes([0, 0]),
                    *(fixtures.node(6, struct.pack("<H", i)) for i in ids),
                ),
            )
            for (x, y, z), ids in sorted(tiles.items())
        ]
        return b"\x00\x00\x00\x00" + fixtures.node(
            0,
            struct.pack("<IHHII", 4, 40000, 40000, 3, 57),
            fixtures.node(2, bytes([1]) + fixtures.string("draft"), *areas),
        )

    def base_tiles(self):
        tiles = {}
        for i, ids in enumerate([(GRASS_GROUND,), (ROCK_GROUND, WALL), (WATER,)]):
            for j in range(3):
                tiles[(LX + j, LY + i, 7)] = ids
        for j in range(3):
            tiles[(LX + j, LY, 6)] = (GRASS_GROUND,)
        tiles[(X0 + 2, Y0, 7)] = (WATER,)
        tiles[(X0 + 3, Y0, 7)] = (SAND,)
        return tiles

    def build(self, blobs=None):
        patcher = mock.patch.object(draft, "MIN_SAMPLES", 2)
        patcher.start()
        self.addCleanup(patcher.stop)
        return convert.build(
            {
                convert.OTBM: self.otbm(self.base_tiles()),
                convert.ITEMS_XML: ITEMS_XML,
                **(self.blobs if blobs is None else blobs),
            },
            ITEMS_BY_SERVER_ID,
            land=lambda x, y, z: (x, y) != (X0 + 1, Y0),
        )

    def install(self, out):
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root)
        for path, data in out.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_bytes(data)
        (root / validate.ITEM_BINDINGS).parent.mkdir(parents=True, exist_ok=True)
        (root / validate.ITEM_BINDINGS).write_bytes(ITEMS_BY_SERVER_ID)
        return root

    def summary(self, out):
        return json.loads(out[str(convert.SUMMARY.relative_to(convert.ROOT))])

    def test_the_draft_adds_and_replaces_only_what_the_rule_selects(self):
        out = self.build()
        summary = self.summary(out)
        record = summary["draft"]
        self.assertTrue(record["applied"])
        # floor 7: added (X0, Y0) and the rock; floor 6: added (X0, Y0)
        self.assertEqual(record["tiles_added_by_floor"], {"6": 1, "7": 2})
        self.assertEqual(record["tiles_replaced_by_floor"], {"7": 1})
        self.assertEqual((record["tiles_added"], record["tiles_replaced"]), (3, 1))
        self.assertEqual(record["items_added"], 4)  # three ground tiles and one wall
        self.assertEqual(
            (record["replaced_items_added"], record["replaced_items_removed"]), (1, 1)
        )
        floors = record["areas"][0]["floors"]
        self.assertEqual(floors["7"]["no_official_land"], 1)
        self.assertEqual(floors["7"]["kept_base"], 1)
        self.assertEqual(floors["6"]["unmapped"], 1)
        self.assertEqual(record["unresolved_entrances"], [[X0 + 5, Y0, 7]])
        # a listed marker is no tile and no item
        self.assertEqual(floors["7"]["markers"], 1)
        index = json.loads(out[validate.INDEX])
        self.assertEqual(index["source"]["minimap_draft"], draft.PIN)
        self.assertEqual(summary["source"], index["source"])
        # the base file has 14 tiles with 17 items
        world = {"items": 17, "tiles": 14}
        self.assertEqual(index["totals"]["tiles"], 17)
        self.assertEqual(convert.world_otbm_totals(index["totals"], summary), world)
        root = self.install(out)
        self.assertEqual(validate.validate(root, workers=1), [])
        self.assertEqual(validate.validate(root, world, workers=1), [])

    def test_without_the_files_no_tile_changes(self):
        out = self.build({})
        self.assertEqual(self.summary(out)["draft"], draft.unapplied())
        self.assertNotIn("minimap_draft", json.loads(out[validate.INDEX])["source"])
        self.assertEqual(validate.validate(self.install(out), workers=1), [])

    def test_tampered_records_and_pins_are_rejected(self):
        out = self.build()
        name = str(convert.SUMMARY.relative_to(convert.ROOT))

        def errors_after(edit, pin=None):
            summary = self.summary(out)
            edit(summary["draft"])
            index = json.loads(out[validate.INDEX])
            if pin:
                pin(summary["source"]["minimap_draft"])
                pin(index["source"]["minimap_draft"])
            root = self.install(out)
            (root / name).write_bytes(validate.canonical(summary))
            (root / validate.INDEX).write_bytes(validate.canonical(index))
            return validate.validate(root, workers=1)

        def more_tiles(record):
            record["tiles_added"] += 1

        def lost_marker(record):
            record["unresolved_entrances"].clear()

        def marker_outside(record):
            record["unresolved_entrances"].append([1, 2, 7])

        def small_sample(record):
            record["mapping"]["rows"][0]["samples"] = 1

        def marker_mapped(record):
            record["mapping"]["rows"].append(
                {
                    "class": "walkable",
                    "colour": "#ffff00",
                    "floor": 7,
                    "ground": 1,
                    "item": None,
                    "mapped": True,
                    "samples": 99,
                }
            )

        def sum_off(record):
            record["areas"][0]["floors"]["7"]["walkable"] += 1

        def unapplied(record):
            record["tiles_added"] = 0
            record["applied"] = False

        for edit in (
            more_tiles,
            lost_marker,
            marker_outside,
            small_sample,
            marker_mapped,
            sum_off,
            unapplied,
        ):
            with self.subTest(edit=edit.__name__):
                self.assertTrue(any("draft" in e for e in errors_after(edit)))

        def bad_pin(pin):
            pin["tibiamaps"]["files"][1]["sha256"] = "0" * 64

        self.assertTrue(
            any(
                "source.minimap_draft" in e
                for e in errors_after(lambda r: None, bad_pin)
            )
        )


class FilesTest(unittest.TestCase):
    def test_root_rejects_missing_and_tampered_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(convert.ConvertError):
                draft.read_files(Path(tmp))
            for name, _sha in draft.FILES:
                (Path(tmp) / name).write_bytes(b"not the pinned file")
            with self.assertRaises(convert.ConvertError) as caught:
                draft.read_files(Path(tmp))
            self.assertIn("sha256 differs", str(caught.exception))

    def test_pins_cover_the_maps_and_paths_of_the_draft_floors(self):
        names = {n for n, _s in draft.FILES}
        for z in draft.FLOORS:
            self.assertIn(f"floor-{z:02d}-map.png", names)
            self.assertIn(f"floor-{z:02d}-path.png", names)
        self.assertIn("bounds.json", names)

    def test_a_differing_frame_is_rejected(self):
        bounds = {"xMin": 31745, "yMin": 30976, "width": 4, "height": 4}
        blobs = {draft.blob_key("bounds.json"): json.dumps(bounds).encode()}
        try:
            with self.assertRaises(convert.ConvertError):
                draft.decode_images(blobs)
        except ImportError:
            self.skipTest("Pillow is not installed")


if __name__ == "__main__":
    unittest.main()
    sys.exit(0)
