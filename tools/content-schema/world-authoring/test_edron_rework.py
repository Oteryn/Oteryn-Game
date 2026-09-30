"""Tests for the Edron underground rework (rules 1-3, the pins and their validation)."""

from __future__ import annotations

import io
import json
import shutil
import struct
import sys
import tempfile
import unittest
from pathlib import Path

import convert_world_base as convert
import edron_rework as edron
import test_world_authoring as fixtures
import validate_world_base as validate
from test_world_base import ITEMS_BY_SERVER_ID, ITEMS_XML, write_definitions

ROCK, ROCK2, GROUND, GROUND2, WALL_ITEM = 101, 1128, 4000, 4001, 5000
STAIRS, YELLOW = 1010, 1020
BLOCKING = {ROCK, ROCK2, WALL_ITEM}
KINDS = {STAIRS: "down", 386: "rope"}
X, Y = 33300, 31800


def tile(*ids):
    return (0, None, (), [(i, 0, None) for i in ids])


def walk_of(**floors):
    """`walk_of(z10=[(dx, dy)])` -> `{10: {(x, y)}}` relative to (X, Y)."""
    return {int(k[1:]): {(X + dx, Y + dy) for dx, dy in v} for k, v in floors.items()}


def maps(**floors):
    return edron.TibiaMaps(walk=walk_of(**floors), markers={})


def at(dx, dy, z=10):
    return (X + dx, Y + dy, z)


def filler():
    """Walkable and blocking ground on floors 9 and 10, so rule 2 can pick its grounds."""
    found = {}
    for z in (9, 10):
        found.update({at(i, 50, z): tile(GROUND2) for i in range(3)})
        found.update({at(i, 51, z): tile(ROCK2) for i in range(2)})
    return found


class RuleOneTest(unittest.TestCase):
    def plan(self, base, summer, walk):
        return edron.plan_rules(
            {**filler(), **base}, summer, maps(z10=walk, z9=[]), BLOCKING
        )

    def test_fill_replace_and_keep_follow_the_walkability_agreement(self):
        summer = {
            at(0, 0): tile(GROUND),  # walkable core, base absent: filled
            at(1, 0): tile(GROUND),  # walkable core, base blocked: replaced
            at(2, 0): tile(GROUND, 6000),  # walkable core, base walkable: kept
            at(0, 1): tile(
                ROCK
            ),  # ring, blocked where tibiamaps is blocked, absent: filled
            at(3, 0): tile(
                ROCK
            ),  # ring, base walkable, tibiamaps blocked: replaced by rock
            at(1, -1): tile(
                GROUND
            ),  # ring but tibiamaps blocked: disagrees, not included
            at(1, 1): tile(
                ROCK
            ),  # ring but tibiamaps walkable: disagrees, not included
            at(9, 9): tile(ROCK),  # far from the core: not included
            at(0, 0, 9): tile(GROUND),  # another floor: ignored
            (X + 500, Y, 10): tile(GROUND),  # outside the box: ignored
        }
        base = {
            at(1, 0): tile(ROCK2),
            at(2, 0): tile(GROUND2),
            at(3, 0): tile(GROUND),
            at(1, 1): tile(
                GROUND2
            ),  # base walkable, tibiamaps walkable, summer blocked
        }
        # the walkable rock case: (1, 1) is walkable for tibiamaps
        out = self.plan(base, summer, [(0, 0), (1, 0), (2, 0), (1, 1)])
        self.assertEqual(set(out.add), {at(0, 0), at(0, 1)})
        self.assertEqual(set(out.replace), {at(1, 0), at(3, 0)})
        self.assertEqual(out.add[at(0, 0)], tile(GROUND))
        self.assertEqual(out.replace[at(3, 0)], tile(ROCK))
        rule1 = out.stats["rule1"]
        self.assertEqual(
            (rule1["core"], rule1["included"], rule1["kept_base"]), (3, 5, 1)
        )
        self.assertEqual(
            (rule1["filled"], rule1["filled_walkable"], rule1["filled_blocked"]),
            (2, 1, 1),
        )
        self.assertEqual(
            (
                rule1["replaced"],
                rule1["replaced_to_walkable"],
                rule1["replaced_to_blocked"],
            ),
            (2, 1, 1),
        )

    def test_a_walkable_base_tile_that_tibiamaps_shows_walkable_is_never_removed(self):
        summer = {at(0, 0): tile(GROUND), at(1, 0): tile(ROCK), at(2, 0): tile(ROCK)}
        base = {at(1, 0): tile(GROUND2), at(2, 0): tile(GROUND2)}
        out = self.plan(base, summer, [(0, 0), (1, 0), (2, 0)])
        # summer blocks (1, 0) and (2, 0) where tibiamaps is walkable: summer disagrees
        self.assertEqual(out.replace, {})
        self.assertEqual(set(out.add), {at(0, 0)})
        self.assertEqual(out.final[at(1, 0)], tile(GROUND2))
        self.assertEqual(out.final[at(2, 0)], tile(GROUND2))

    def test_a_base_tile_that_agrees_with_tibiamaps_is_kept_even_if_summer_differs(
        self,
    ):
        summer = {at(0, 0): tile(GROUND), at(1, 0): tile(ROCK2)}
        base = {at(0, 0): tile(GROUND2), at(1, 0): tile(ROCK)}
        out = self.plan(base, summer, [(0, 0)])
        self.assertEqual((out.add, out.replace), ({}, {}))
        self.assertEqual(out.stats["rule1"]["kept_base"], 2)

    def test_special_tiles_are_refused(self):
        summer = {at(0, 0): (0, 5, (), [(GROUND, 0, None)])}
        with self.assertRaises(convert.ConvertError):
            self.plan({}, summer, [(0, 0)])
        summer = {at(0, 0): tile(GROUND)}
        for special in (
            (0, 5, (), [(ROCK, 0, None)]),
            (0, None, (), [(ROCK, 0, {"dest": (5, 5, 5)})]),
        ):
            with self.assertRaises(convert.ConvertError):
                self.plan({at(0, 0): special}, summer, [(0, 0)])


class RuleTwoTest(unittest.TestCase):
    def setUp(self):
        # floor 9: GROUND2 is the most common walkable, ROCK2 the most common blocking ground
        self.base = {
            **filler(),
            **{at(dx, 20, 9): tile(GROUND2) for dx in range(5)},
            at(5, 20, 9): tile(GROUND),
            **{at(dx, 21, 9): tile(ROCK2) for dx in range(3)},
            at(3, 21, 9): tile(ROCK),
        }

    def plan(self, base, summer, walk9):
        return edron.plan_rules(base, summer, maps(z9=walk9, z10=[]), BLOCKING)

    def test_tibiamaps_walkable_positions_get_ground_and_a_closing_rock(self):
        out = self.plan(self.base, {}, [(2, 0)])
        row = out.stats["rule2"]["9"]
        self.assertEqual(
            (row["walkable_ground"], row["blocking_ground"]), (GROUND2, ROCK2)
        )
        self.assertEqual(
            (row["targets"], row["walkable_added"], row["walkable_replaced"]),
            (1, 1, 0),
        )
        self.assertEqual(out.add[at(2, 0, 9)], tile(GROUND2))
        # the eight neighbours: none has a tile, so all get rock (none is walkable)
        self.assertEqual(row["rock_added"], 8)
        self.assertEqual(
            {p for p, t in out.add.items() if t == tile(ROCK2)},
            {at(2 + dx, dy, 9) for dx, dy in edron.NEIGHBOURS},
        )
        self.assertEqual(out.replace, {})

    def test_a_blocked_tile_is_replaced_and_existing_neighbours_are_untouched(self):
        base = {**self.base, at(0, 0, 9): tile(ROCK), at(1, 0, 9): tile(GROUND)}
        out = self.plan(base, {}, [(0, 0), (2, 0)])
        self.assertEqual(out.replace, {at(0, 0, 9): tile(GROUND2)})
        self.assertNotIn(at(1, 0, 9), out.add)
        self.assertNotIn(at(1, 0, 9), out.replace)
        self.assertNotIn(at(2, 0, 9), out.replace)
        row = out.stats["rule2"]["9"]
        self.assertEqual((row["walkable_added"], row["walkable_replaced"]), (1, 1))

    def test_a_walkable_tile_is_never_overwritten(self):
        base = {**self.base, at(0, 0, 9): tile(GROUND, 6000)}
        out = self.plan(base, {}, [(0, 0)])
        self.assertEqual(out.stats["rule2"]["9"]["targets"], 0)
        self.assertNotIn(at(0, 0, 9), out.replace)
        self.assertNotIn(at(0, 0, 9), out.add)
        # nor is a position the summer file has walkable, even where the base has nothing
        out = self.plan(self.base, {at(7, 0, 9): tile(GROUND)}, [(7, 0)])
        self.assertEqual((out.add, out.replace), ({}, {}))

    def test_a_walkable_neighbour_becomes_no_rock_and_a_blocked_one_is_kept(self):
        base = {**self.base, at(1, 0, 9): tile(ROCK)}
        out = self.plan(base, {}, [(0, 0), (1, 1)])
        # (1, 1) is tibiamaps walkable, so it gets ground and no rock; (1, 0) keeps its rock
        self.assertEqual(out.add[at(1, 1, 9)], tile(GROUND2))
        self.assertNotIn(at(1, 0, 9), out.add)
        self.assertNotIn(at(1, 0, 9), out.replace)

    def test_replacing_a_blocked_tile_with_a_zone_is_refused(self):
        base = {**self.base, at(0, 0, 9): (0, None, (7,), [(ROCK, 0, None)])}
        with self.assertRaises(convert.ConvertError):
            self.plan(base, {}, [(0, 0)])

    def test_a_floor_without_walkable_ground_fails_closed(self):
        with self.assertRaises(convert.ConvertError):
            self.plan({at(0, 0, 9): tile(ROCK)}, {}, [(1, 1)])


class RuleThreeTest(unittest.TestCase):
    def maps(self, markers):
        return edron.TibiaMaps(walk={}, markers=markers)

    def test_connected_markers_and_unresolved_ones_are_reported_never_invented(self):
        final = {
            at(0, 0, 10): tile(GROUND, STAIRS),  # same position
            at(5, 5, 11): tile(GROUND, 386),  # one floor apart for a marker on 10
            at(9, 9, 10): tile(GROUND),  # no floor change anywhere
            at(3, 3, 9): tile(GROUND, YELLOW),  # yellow item that is no floor change
        }
        markers = {
            9: {(X + 3, Y + 3)},
            10: {(X, Y), (X + 5, Y + 5), (X + 9, Y + 9), (X + 20, Y + 20)},
            11: set(),
        }
        result = edron.entrances(final, self.maps(markers), KINDS, {YELLOW})
        self.assertEqual(result["markers"], {"9": 0, "10": 4, "11": 0})
        self.assertEqual(result["explained_by_yellow_items"]["9"], 1)
        self.assertEqual(result["connected"], {"9": 0, "10": 2, "11": 0})
        self.assertEqual(
            result["unresolved_entrances"],
            [[X + 9, Y + 9, 10], [X + 20, Y + 20, 10]],
        )
        # the position without a tile stays without one: nothing is added by the report
        self.assertNotIn(at(20, 20, 10), final)

    def test_yellow_areas_are_ground_not_markers(self):
        pixels = {(X + i, Y) for i in range(3)} | {(X + 10, Y), (X + 11, Y)}
        result = edron.entrances(
            {}, self.maps({9: pixels, 10: set(), 11: set()}), KINDS, set()
        )
        self.assertEqual(result["markers"]["9"], 2)
        self.assertEqual(result["yellow_area_pixels"]["9"], 3)

    def test_the_unresolved_entrance_is_listed_by_the_plan_not_repaired(self):
        base = {**filler(), at(0, 0, 10): tile(GROUND)}
        summer = {at(1, 0, 10): tile(GROUND)}
        tm = edron.TibiaMaps(
            walk=walk_of(z10=[(0, 0), (1, 0)], z9=[]),
            markers={9: {(X + 1, Y)}, 10: set(), 11: set()},
        )
        plan = edron.plan_rules(base, summer, tm, BLOCKING)
        result = edron.entrances(plan.final, tm, KINDS, set())
        self.assertEqual(result["unresolved_entrances"], [[X + 1, Y, 9]])
        self.assertFalse(any(i[0] in KINDS for t in plan.add.values() for i in t[3]))

    def test_reachability_follows_walkable_tiles_and_floor_change_links(self):
        final = {
            at(0, 0, 7): tile(GROUND),
            at(1, 0, 7): tile(GROUND, STAIRS),  # down to (1, 0, 8)
            at(1, 0, 8): tile(GROUND, STAIRS),  # down to (1, 0, 9)
            at(1, 0, 9): tile(GROUND, STAIRS),  # down to (1, 0, 10)
            at(1, 0, 10): tile(GROUND),
            at(2, 1, 10): tile(GROUND),  # diagonal neighbour
            at(9, 9, 10): tile(GROUND),  # island
            at(3, 0, 10): tile(ROCK),
        }
        result = edron.reachability(final, {at(2, 1), at(9, 9)}, BLOCKING, KINDS)
        self.assertEqual(result["floor10_walkable"], 3)
        self.assertEqual(result["floor10_walkable_reached_from_surface"], 2)
        self.assertEqual(result["floor10_new_walkable"], 2)
        self.assertEqual(result["floor10_new_walkable_reached_from_surface"], 1)
        self.assertEqual(result["floor10_components"], 2)
        self.assertEqual(result["floor10_components_reached_from_surface"], 1)
        self.assertEqual(result["floor10_largest_component"], 2)


class BuildAndValidateTest(unittest.TestCase):
    SUMMER_KEY = convert.fill_key(convert.FILL[1])

    @staticmethod
    def area(z, tiles):
        return fixtures.node(
            4,
            struct.pack("<HHB", X, Y, z),
            *(
                fixtures.node(
                    5,
                    bytes([dx, dy]),
                    *(fixtures.node(6, struct.pack("<H", i)) for i in ids),
                )
                for dx, dy, ids in tiles
            ),
        )

    def otbm(self, *areas):
        return b"\x00\x00\x00\x00" + fixtures.node(
            0,
            struct.pack("<IHHII", 4, 40000, 40000, 3, 57),
            fixtures.node(2, bytes([1]) + fixtures.string("edron"), *areas),
        )

    def build(self, tibiamaps=None, blobs=None):
        base = self.otbm(
            self.area(
                10,
                [
                    (0, 0, [ROCK2]),  # replaced by the summer walkable ground
                    (2, 0, [GROUND2, 7]),  # kept
                    (
                        3,
                        0,
                        [GROUND],
                    ),  # walkable, tibiamaps blocked, summer rock: replaced
                ],
            ),
            self.area(9, [(0, 0, [GROUND2]), (1, 0, [ROCK2]), (1, 1, [ROCK2])]),
            self.area(7, [(0, 0, [GROUND])]),
        )
        summer = self.otbm(
            self.area(
                10,
                [
                    (0, 0, [GROUND]),
                    (1, 0, [GROUND, 6000]),  # filled
                    (2, 0, [GROUND]),
                    (3, 0, [ROCK]),
                ],
            )
        )
        tm = tibiamaps or edron.TibiaMaps(
            walk=walk_of(z10=[(0, 0), (1, 0), (2, 0)], z9=[(4, 0)]),
            markers={9: {(X + 4, Y)}, 10: set(), 11: set()},
        )
        return convert.build(
            {
                convert.OTBM: base,
                convert.ITEMS_XML: ITEMS_XML,
                self.SUMMER_KEY: summer,
                **(blobs or {}),
            },
            ITEMS_BY_SERVER_ID,
            land=lambda *_: False,
            tibiamaps=tm,
            blocking=BLOCKING,
            kinds=KINDS,
            yellow={YELLOW},
        )

    def install(self, out):
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root)
        for path, data in out.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_bytes(data)
        (root / validate.ITEM_BINDINGS).parent.mkdir(parents=True, exist_ok=True)
        (root / validate.ITEM_BINDINGS).write_bytes(ITEMS_BY_SERVER_ID)
        write_definitions(root, ITEMS_BY_SERVER_ID)
        return root

    def summary(self, out):
        return json.loads(out[str(convert.SUMMARY.relative_to(convert.ROOT))])

    def test_the_rework_changes_only_what_the_rules_select(self):
        out = self.build()
        summary = self.summary(out)
        rework = summary["edron"]
        self.assertTrue(rework["applied"])
        # rule 1: fill (1, 0); replace (0, 0) and (5, 0); keep (2, 0)
        self.assertEqual(rework["rule1"]["filled"], 1)
        self.assertEqual(rework["rule1"]["replaced"], 2)
        self.assertEqual(rework["rule1"]["kept_base"], 1)
        # rule 2: floor 9 (4, 0) is added with rock around it, floor 10 has no target
        self.assertEqual(rework["rule2"]["9"]["walkable_added"], 1)
        self.assertEqual(rework["rule2"]["9"]["rock_added"], 8)
        self.assertEqual(rework["rule2"]["10"]["targets"], 0)
        self.assertEqual(rework["tiles_added_by_floor"]["10"], 1)
        self.assertEqual(
            rework["tiles_added"],
            1
            + 1
            + rework["rule2"]["9"]["rock_added"]
            + rework["rule2"]["10"]["rock_added"],
        )
        self.assertEqual(rework["tiles_replaced"], 2)
        self.assertEqual(rework["entrances"]["unresolved_entrances"], [[X + 4, Y, 9]])
        self.assertEqual(rework["replaced_items_removed"], 2)
        self.assertEqual(rework["replaced_items_added"], 2)
        self.assertEqual(rework["items_added"], rework["tiles_added"] + 1)
        index = json.loads(out[validate.INDEX])
        self.assertEqual(index["source"]["edron"], edron.PIN)
        self.assertEqual(summary["source"], index["source"])
        # the base file has 7 tiles with 8 items; the rework adds 10 tiles with 11 items
        self.assertEqual(rework["tiles_added_by_floor"], {"9": 9, "10": 1})
        self.assertEqual(index["totals"], {**index["totals"], "tiles": 17, "items": 19})
        world = {"items": 8, "tiles": 7}
        self.assertEqual(convert.world_otbm_totals(index["totals"], summary), world)
        root = self.install(out)
        self.assertEqual(validate.validate(root, workers=1), [])
        self.assertEqual(validate.validate(root, world, workers=1), [])

    def test_without_tibiamaps_no_tile_changes(self):
        out = convert.build(
            {
                convert.OTBM: self.otbm(self.area(10, [(0, 0, [GROUND])])),
                convert.ITEMS_XML: ITEMS_XML,
            },
            ITEMS_BY_SERVER_ID,
        )
        summary = self.summary(out)
        self.assertEqual(summary["edron"]["applied"], False)
        self.assertNotIn("edron", json.loads(out[validate.INDEX])["source"])
        self.assertEqual(validate.validate(self.install(out), workers=1), [])

    def test_the_rework_needs_the_summer_member(self):
        with self.assertRaises(convert.ConvertError):
            convert.build(
                {
                    convert.OTBM: self.otbm(self.area(10, [(0, 0, [GROUND])])),
                    convert.ITEMS_XML: ITEMS_XML,
                },
                ITEMS_BY_SERVER_ID,
                tibiamaps=maps(z9=[], z10=[]),
                blocking=BLOCKING,
                kinds=KINDS,
                yellow=set(),
            )

    def test_tampered_records_and_pins_are_rejected(self):
        out = self.build()
        name = str(convert.SUMMARY.relative_to(convert.ROOT))

        def errors_after(edit, pin=None):
            summary = self.summary(out)
            edit(summary["edron"])
            index = json.loads(out[validate.INDEX])
            if pin:
                pin(summary["source"]["edron"])
                pin(index["source"]["edron"])
            root = self.install(out)
            (root / name).write_bytes(validate.canonical(summary))
            (root / validate.INDEX).write_bytes(validate.canonical(index))
            return validate.validate(root, workers=1)

        def more_tiles(rework):
            rework["tiles_added"] += 1

        self.assertTrue(any("edron" in e for e in errors_after(more_tiles)))

        def lost_marker(rework):
            rework["entrances"]["connected"]["9"] += 1

        self.assertTrue(any("entrances" in e for e in errors_after(lost_marker)))

        def outside(rework):
            rework["entrances"]["unresolved_entrances"].append([1, 2, 9])

        self.assertTrue(any("edron" in e for e in errors_after(outside)))

        def bad_pin(pin):
            pin["tibiamaps"]["files"][1]["sha256"] = "0" * 64

        self.assertTrue(
            any("source.edron" in e for e in errors_after(lambda r: None, bad_pin))
        )

        def unapplied(rework):
            rework["tiles_added"] = 0
            rework["applied"] = False

        self.assertTrue(any("edron" in e for e in errors_after(unapplied)))


class TibiamapsFilesTest(unittest.TestCase):
    def test_root_rejects_missing_and_tampered_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(convert.ConvertError):
                edron.read_tibiamaps_root(Path(tmp))
            for name, _sha in edron.TIBIAMAPS_FILES:
                (Path(tmp) / name).write_bytes(b"not the pinned file")
            with self.assertRaises(convert.ConvertError) as caught:
                edron.read_tibiamaps_root(Path(tmp))
            self.assertIn("sha256 differs", str(caught.exception))

    def test_pins_cover_the_maps_and_paths_of_the_marker_floors(self):
        names = {n for n, _s in edron.TIBIAMAPS_FILES}
        for z in edron.MARKER_FLOORS:
            self.assertIn(f"floor-{z:02d}-map.png", names)
            self.assertIn(f"floor-{z:02d}-path.png", names)
        self.assertIn("bounds.json", names)

    def test_decode_reads_walkable_grey_and_yellow_marker_pixels(self):
        try:
            from PIL import Image
        except ImportError:
            self.skipTest("Pillow is not installed")
        bounds = {"xMin": 31744, "yMin": 30976, "width": 2560, "height": 2048}
        blobs = {edron.tibiamaps_blob_key("bounds.json"): json.dumps(bounds).encode()}
        left, top = edron.BOX[0] - 31744, edron.BOX[1] - 30976
        for z in edron.MARKER_FLOORS:
            path = Image.new("RGBA", (2560, 2048), (255, 0, 255, 255))
            path.putpixel((left, top), (100, 100, 100, 255))  # walkable
            path.putpixel((left + 1, top), (255, 255, 0, 255))  # explored, not walkable
            path.putpixel((left - 1, top), (100, 100, 100, 255))  # outside the box
            overview = Image.new("RGBA", (2560, 2048), (0, 0, 0, 255))
            overview.putpixel((left + 2, top), (255, 255, 0, 255))
            for kind, image in (("path", path), ("map", overview)):
                buffer = io.BytesIO()
                image.save(buffer, "PNG")
                blobs[edron.tibiamaps_blob_key(f"floor-{z:02d}-{kind}.png")] = (
                    buffer.getvalue()
                )
        found = edron.decode_tibiamaps(blobs)
        for z in edron.MARKER_FLOORS:
            self.assertEqual(found.walk[z], {(edron.BOX[0], edron.BOX[1])})
            self.assertEqual(found.markers[z], {(edron.BOX[0] + 2, edron.BOX[1])})
        blobs[edron.tibiamaps_blob_key("bounds.json")] = json.dumps(
            {**bounds, "xMin": 31745}
        ).encode()
        with self.assertRaises(convert.ConvertError):
            edron.decode_tibiamaps(blobs)


if __name__ == "__main__":
    unittest.main()
    sys.exit(0)
