"""Tests for the region codec, the base map converter and the base map validator."""

from __future__ import annotations

import json
import shutil
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import convert_world_base as convert
import otbm_reader
import test_world_authoring as fixtures
import validate_world_base as validate
import world_region_codec as codec

ALL_ATTRS = {
    "count": 255,
    "action": 65535,
    "unique": 65535,
    "door": 255,
    "text": "",
    "charges": 65535,
    "description": "dsc ÿ é 中",
    "teleport": (65535, 65535, 15),
    "depot": 65535,
}
GOLD = "oteryn:item.currency.gold_coin"
REGISTRY = "oteryn:item.registry.i90000000"
DONOR = convert.DONOR_PREFIX
assert DONOR == "donor:crystalserver@00ce02a5:item/"


def bindings_for(pairs) -> bytes:
    return json.dumps(
        {
            "bindings": [
                {
                    "external_id": str(server_id),
                    "identity_namespace": "ots/item_server_id",
                    "target": {"key": key},
                }
                for server_id, key in pairs
            ]
        }
    ).encode()


def write_definitions(
    root: Path, bindings: bytes, skip: set[str] = frozenset()
) -> None:
    """One Item definition record per binding target (the rule of A12 section 5)."""
    keys = sorted({r["target"]["key"] for r in json.loads(bindings)["bindings"]} - skip)
    path = root / "content/items/definitions/items-00000-00499.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    records = [{"definition": {"identity": {"key": key}}} for key in keys]
    path.write_text(json.dumps({"records": records}), encoding="utf-8")


# 100 is bound to a named key, 1234 to a registry key and 1949 (the teleport) is unbound.
ITEMS_BY_SERVER_ID = bindings_for([(100, GOLD), (1234, REGISTRY), (7, "oteryn:x")])
ITEMS_XML = (
    b'<items><item id="7" name="x"/><item fromid="100" toid="101" name="y"/></items>'
)
ITEMS_XML_WITH_TELEPORT = ITEMS_XML.replace(b"</items>", b'<item id="1949"/></items>')


def normalized(tiles):
    """Encoder input in decoder shape: an item without attributes has ``None``."""
    return [
        (x, y, flags, house, zones, [(r, d, a or None) for r, d, a in items])
        for x, y, flags, house, zones, items in tiles
    ]


class CodecTest(unittest.TestCase):
    def round_trip(self, sx, sy, tiles):
        payload = codec.encode_sector_tiles(sx, sy, tiles)
        self.assertEqual(codec.decode_sector(payload, sx, sy), normalized(tiles))
        return payload

    def test_round_trips_every_attribute_container_house_and_edge_value(self):
        tiles = [
            (64, 96, 0, 0, (), []),
            (
                65,
                96,
                0xFFFFFFFF,
                0xFFFFFFFF,
                (1, 65535, 1),
                [(0x7FFFFFFF, 0, dict(ALL_ATTRS))],
            ),
            (
                95,
                96,
                1,
                1,
                (),
                [
                    (1, 0, {}),
                    (2, 0, {"count": 0}),
                    (3, 1, {"text": "x" * 70000}),
                    (4, 2, None),
                    (5, 1, {"action": 0, "unique": 0, "teleport": (0, 0, 0)}),
                    (6, 0, {"description": ""}),
                ],
            ),
            (64, 127, 128, 16384, (), [(16383, 0, None), (16384, 0, None)]),
            (95, 127, 0, 0, (), [(9, 0, {"door": 0})]),
        ]
        self.round_trip(2, 3, tiles)

    def test_empty_sector_and_max_tiles(self):
        self.round_trip(0, 0, [])
        tiles = [(x, y, 0, 0, (), [(1, 0, None)]) for y in range(32) for x in range(32)]
        self.round_trip(0, 0, tiles)

    def test_region_file_layout(self):
        first = codec.encode_sector_tiles(8, 0, [(256, 0, 0, 0, (), [(1, 0, None)])])
        second = codec.encode_sector_tiles(
            15, 7, [(480, 224, 5, 0, (), [(2, 0, None)])]
        )
        data = codec.encode_region(3, 1, 0, {63: second, 0: first})
        self.assertEqual(data[:4], b"OTRB")
        self.assertEqual(codec.parse_region(data)[:3], (3, 1, 0))
        z, rx, ry, sectors = codec.decode_region(data)
        self.assertEqual(
            (z, rx, ry, [local for local, _ in sectors]), (3, 1, 0, [0, 63])
        )
        self.assertEqual(sectors[1][1], [(480, 224, 5, 0, (), [(2, 0, None)])])
        self.assertEqual(codec.region_name(3, 1, 0), "region-z03-x001-y000.b3")

    def test_encoding_is_deterministic(self):
        tiles = [(1, 2, 3, 4, (), [(5, 0, {"count": 2})])]
        self.assertEqual(
            codec.encode_sector_tiles(0, 0, tiles),
            codec.encode_sector_tiles(0, 0, tiles),
        )
        payload = codec.encode_sector_tiles(0, 0, tiles)
        self.assertEqual(
            codec.encode_region(0, 0, 0, {0: payload}),
            codec.encode_region(0, 0, 0, {0: payload}),
        )

    def test_rejects_unrepresentable_input(self):
        bad = [
            (0, 0, 0, 0, (), [(1, 0, {"count": 256})]),
            (0, 0, 0, 0, (), [(1, 0, {"action": 65536})]),
            (0, 0, 0, 0, (), [(1, 0, {"nope": 1})]),
            (0, 0, 0, 0, (), [(1, 0, {"teleport": (0, 0, 256)})]),
            (0, 0, 0, 0, (), [(1, 0, {"text": b"bytes"})]),
            (0, 0, 0, 0, (), [(1, 1, None)]),
            (0, 0, 0, 0, (), [(1, 0, None), (2, 2, None)]),
            (0, 0, 2**32, 0, (), []),
            (0, 0, 0, -1, (), []),
            (0, 0, 0, 0, (), [(-1, 0, None)]),
            (32, 0, 0, 0, (), []),
        ]
        for tile in bad:
            with self.assertRaises(codec.CodecError, msg=tile):
                codec.encode_sector_tiles(0, 0, [tile])
        with self.assertRaises(codec.CodecError):
            codec.encode_sector_tiles(
                0, 0, [(1, 0, 0, 0, (), []), (0, 0, 0, 0, (), [])]
            )
        with self.assertRaises(codec.CodecError):
            codec.encode_sector_tiles(
                0, 0, [(1, 0, 0, 0, (), []), (1, 0, 0, 0, (), [])]
            )

    def test_rejects_damaged_payloads_and_files(self):
        payload = codec.encode_sector_tiles(
            0, 0, [(1, 1, 5, 0, (), [(9, 0, {"text": "ab"})])]
        )
        for damaged in (payload[:-1], payload + b"\x00", b"", payload[:3]):
            with self.assertRaises(codec.CodecError):
                codec.decode_sector(damaged, 0, 0)
        data = codec.encode_region(0, 0, 0, {0: payload})
        with self.assertRaises(codec.CodecError):
            codec.parse_region(b"XXXX" + data[4:])
        with self.assertRaises(codec.CodecError):
            codec.parse_region(data + b"\x00")
        with self.assertRaises(codec.CodecError):
            codec.parse_region(data[:-1])
        with self.assertRaises(codec.CodecError):
            codec.decode_region(data[:-1] + bytes([data[-1] ^ 1]))
        two = codec.encode_region(0, 0, 0, {0: payload, 1: payload})
        entry = codec.HEADER.size
        swapped = bytearray(two)
        swapped[entry], swapped[entry + 9] = swapped[entry + 9], swapped[entry]
        with self.assertRaises(codec.CodecError):
            codec.parse_region(bytes(swapped))


class ConvertAndValidateTest(unittest.TestCase):
    def setUp(self):
        self.blobs = {
            convert.OTBM: fixtures.fixture_map(),
            convert.ITEMS_XML: ITEMS_XML,
        }
        self.out = convert.build(self.blobs, ITEMS_BY_SERVER_ID)
        self.root = Path(tempfile.mkdtemp())
        for path, data in self.out.items():
            (self.root / path).parent.mkdir(parents=True, exist_ok=True)
            (self.root / path).write_bytes(data)
        self.write_bindings(ITEMS_BY_SERVER_ID)
        self.region = self.root / validate.DIRECTORY / "region-z07-x003-y003.b3"

    def tearDown(self):
        shutil.rmtree(self.root)

    def write_bindings(self, data):
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        bindings.write_bytes(data)
        write_definitions(self.root, data)

    def index(self):
        return json.loads((self.root / validate.INDEX).read_text())

    def write_index(self, index):
        (self.root / validate.INDEX).write_bytes(validate.canonical(index))

    def errors(self):
        return validate.validate(self.root)

    def summary(self, out=None):
        return json.loads(
            (out or self.out)[str(convert.SUMMARY.relative_to(convert.ROOT))]
        )

    def test_fixture_converts_to_a_valid_family(self):
        self.assertEqual(self.errors(), [])
        index = self.index()
        self.assertEqual(
            index["totals"], {"items": 4, "regions": 1, "sectors": 1, "tiles": 4}
        )
        self.assertEqual(index["shards"], [row["path"] for row in index["regions"]])
        self.assertEqual(
            [row["path"] for row in index["source"]["files"]],
            [convert.OTBM, convert.ITEMS_XML],
        )
        self.assertEqual(
            index["palette"],
            [
                {"key": GOLD, "provisional": False, "source_item_id": 100},
                {"key": REGISTRY, "provisional": False, "source_item_id": 1234},
                {
                    "key": DONOR + "1949",
                    "provisional": True,
                    "source_item_id": 1949,
                },
            ],
        )
        summary = self.summary()
        self.assertEqual(summary["tiles_by_floor"], {"7": 4})
        self.assertEqual(summary["tiles_with_house"], 2)
        self.assertEqual(summary["item_attributes"], {"door": 1, "teleport": 2})
        self.assertEqual(summary["rejected_items"], {"unsupported_attributes": 0})
        self.assertEqual(
            summary["floor_changes"], {"by_kind": {}, "item_types": 0, "occurrences": 0}
        )
        self.assertEqual(
            summary["palette"],
            {
                "entries": 3,
                "provisional": {
                    "appearance_only": {"entries": 1, "occurrences": 2},
                    "entries": 1,
                    "in_items_xml": {"entries": 0, "occurrences": 0},
                    "occurrences": 2,
                },
                "families": {
                    "item": {"entries": 2, "occurrences": 2},
                    "terrain": {"entries": 0, "occurrences": 0},
                    "world_object": {"entries": 0, "occurrences": 0},
                },
            },
        )
        z, rx, ry, sectors = codec.decode_region(self.region.read_bytes())
        self.assertEqual((z, rx, ry), (7, 3, 3))
        tiles = {(x, y): (f, h, zn, i) for x, y, f, h, zn, i in sectors[0][1]}
        self.assertEqual(
            tiles,
            {
                (1001, 1001): (0, 0xFE, (), [(1, 0, {"door": 3})]),
                (1002, 1001): (0, 0xFE, (), [(0, 0, None)]),
                (1001, 1003): (
                    0,
                    0,
                    (),
                    [
                        (2, 0, {"teleport": (1002, 1002, 7)}),
                        (2, 0, {"teleport": (0, 0, 0)}),
                    ],
                ),
                (1002, 1002): (0, 0, (), []),
            },
        )

    FILL_KEY = convert.fill_key(convert.FILL[0])

    @staticmethod
    def fill_map(*areas):
        """A fragment map: `areas` are (z, [(dx, dy, [item nodes])]) around (1000, 1000)."""
        nodes = [
            fixtures.node(
                4,
                struct.pack("<HHB", 1000, 1000, z),
                *(fixtures.node(5, bytes([dx, dy]), *items) for dx, dy, items in tiles),
            )
            for z, tiles in areas
        ]
        map_data = fixtures.node(2, bytes([1]) + fixtures.string("fill"), *nodes)
        return b"\x00\x00\x00\x00" + fixtures.node(
            0, struct.pack("<IHHII", 4, 2048, 2048, 3, 57), map_data
        )

    @staticmethod
    def item(server_id, *tail):
        return fixtures.node(6, struct.pack("<H", server_id) + bytes(tail))

    def build_with_fill(self, raw):
        blobs = {**self.blobs, self.FILL_KEY: raw}
        return convert.build(blobs, ITEMS_BY_SERVER_ID)

    def standard_fill(self):
        return self.fill_map(
            (
                7,
                [
                    (1, 1, [self.item(555)]),  # base has this tile: skipped
                    (3, 1, [self.item(100), self.item(555)]),  # missing: added
                    (3, 1, [self.item(1234)]),  # repeated inside the fill: skipped
                    (1, 2, []),  # missing, empty tile: added
                ],
            ),
            (6, [(1, 1, [self.item(100)])]),  # a floor the base lacks: added
        )

    def test_fill_adds_missing_tiles_and_skips_existing_ones(self):
        out = self.build_with_fill(self.standard_fill())
        summary = self.summary(out)
        self.assertEqual(
            summary["fill"],
            {
                "items_added": 3,
                "rule": convert.FILL_RULE,
                "sources": [
                    {
                        "archive": convert.FILL[0]["archive"],
                        "items_added": 3,
                        "member": {
                            **convert.FILL[0]["member"],
                            "bytes": len(self.standard_fill()),
                        },
                        "tiles_added": 3,
                        "tiles_added_by_floor": {"6": 1, "7": 2},
                        "tiles_in_source": 5,
                        "tiles_skipped_existing": 2,
                        "tiles_skipped_existing_by_floor": {"7": 2},
                    }
                ],
                "tiles_added": 3,
            },
        )
        self.assertEqual(summary["totals"]["tiles"], 4 + 3)
        self.assertEqual(summary["tiles_by_floor"], {"6": 1, "7": 6})
        index = json.loads(out[validate.INDEX])
        self.assertEqual(index["source"]["fill"], convert.FILL[:1])
        self.assertEqual(index["totals"]["items"], 4 + 3)
        # the tile the base has keeps its own items; the added tiles are in the region files
        palette = [row["source_item_id"] for row in index["palette"]]
        _z, _rx, _ry, sectors = codec.decode_region(self.decode_path(out, 7))
        tiles = {
            (x, y): [(palette[i], d, a) for i, d, a in items]
            for x, y, _f, _h, _z, items in sectors[0][1]
        }
        self.assertEqual(tiles[(1001, 1001)], [(1234, 0, {"door": 3})])
        self.assertEqual(tiles[(1003, 1001)], [(100, 0, None), (555, 0, None)])
        self.assertEqual(tiles[(1001, 1002)], [])
        # the new floor gets a region file, existing regions elsewhere keep their bytes
        self.assertIn(f"{validate.DIRECTORY}/region-z06-x003-y003.b3", out)

    def decode_path(self, out, floor):
        return out[f"{validate.DIRECTORY}/region-z{floor:02d}-x003-y003.b3"]

    def test_fill_palette_is_append_only(self):
        previous = json.loads(self.out[validate.INDEX])["palette"]
        blobs = {**self.blobs, self.FILL_KEY: self.standard_fill()}
        out = convert.build(blobs, ITEMS_BY_SERVER_ID, previous=previous)
        palette = json.loads(out[validate.INDEX])["palette"]
        self.assertEqual(palette[: len(previous)], previous)
        self.assertEqual(
            [row["source_item_id"] for row in palette[len(previous) :]], [555]
        )
        self.assertEqual(palette[-1]["key"], DONOR + "555")

    def test_fill_output_validates_and_pins_are_checked(self):
        out = self.build_with_fill(self.standard_fill())
        root = self.install(out)
        self.assertEqual(validate.validate(root, workers=1), [])
        # the world.otbm totals stay checkable after the fill
        self.assertEqual(validate.validate(root, {"items": 4, "tiles": 4}), [])
        self.assertTrue(
            any(
                "pinned source" in e
                for e in validate.validate(root, {"items": 4, "tiles": 5})
            )
        )

    def test_fill_summary_tampering_is_rejected(self):
        out = self.build_with_fill(self.standard_fill())
        root = self.install(out)
        path = root / convert.SUMMARY.relative_to(convert.ROOT)
        summary = json.loads(path.read_text())
        for edit, expected in (
            (lambda f: f["sources"][0].update(tiles_added=4), "do not add up"),
            (lambda f: f.update(tiles_added=9), "fill totals differ"),
            (lambda f: f.update(rule="other"), "fill rule differs"),
            (
                lambda f: f["sources"][0]["archive"].update(sha256="0" * 64),
                "fill sources differ",
            ),
        ):
            with self.subTest(expected):
                tampered = json.loads(json.dumps(summary))
                edit(tampered["fill"])
                path.write_bytes(validate.canonical(tampered))
                errors = validate.validate(root)
                self.assertTrue(any(expected in e for e in errors), errors)
        path.write_bytes(validate.canonical(summary))
        index = json.loads((root / validate.INDEX).read_text())
        index["source"]["fill"][0]["member"]["sha256"] = "0" * 64
        (root / validate.INDEX).write_bytes(validate.canonical(index))
        self.assertTrue(any("source" in e for e in validate.validate(root)))

    def test_fill_tiles_with_houses_zones_or_teleports_are_refused(self):
        house = fixtures.node(14, bytes([5, 5]) + struct.pack("<I", 7))
        teleport = self.item(1949, 8, *struct.pack("<HHB", 1, 1, 7))
        for name, tile in (
            ("house", house),
            ("teleport", fixtures.node(5, bytes([6, 6]), teleport)),
        ):
            area = fixtures.node(4, struct.pack("<HHB", 1000, 1000, 7), tile)
            raw = b"\x00\x00\x00\x00" + fixtures.node(
                0,
                struct.pack("<IHHII", 4, 2048, 2048, 3, 57),
                fixtures.node(2, bytes([1]) + fixtures.string("f"), area),
            )
            with self.subTest(name), self.assertRaises(convert.ConvertError):
                self.build_with_fill(raw)

    def test_existing_tiles_with_houses_or_teleports_are_skipped_not_refused(self):
        # the base has a house tile at (1001, 1001) and a teleport tile at (1001, 1003)
        house = fixtures.node(14, bytes([1, 1]) + struct.pack("<I", 9))
        teleport = self.item(1949, 8, *struct.pack("<HHB", 1, 1, 7))
        area = fixtures.node(
            4,
            struct.pack("<HHB", 1000, 1000, 7),
            house,
            fixtures.node(5, bytes([1, 3]), teleport),
        )
        raw = b"\x00\x00\x00\x00" + fixtures.node(
            0,
            struct.pack("<IHHII", 4, 2048, 2048, 3, 57),
            fixtures.node(2, bytes([1]) + fixtures.string("f"), area),
        )
        out = self.build_with_fill(raw)
        self.assertEqual(self.summary(out)["fill"]["tiles_added"], 0)
        self.assertEqual(
            self.summary(out)["fill"]["sources"][0]["tiles_skipped_existing"], 2
        )
        self.assertEqual(
            out,
            {
                **self.out,
                **{
                    k: out[k]
                    for k in (
                        validate.INDEX,
                        str(convert.SUMMARY.relative_to(convert.ROOT)),
                    )
                },
            },
        )

    def test_fill_tile_outside_the_extent_is_refused(self):
        tile = fixtures.node(5, bytes([1, 1]), self.item(100))
        area = fixtures.node(4, struct.pack("<HHB", 2100, 1000, 7), tile)
        raw = b"\x00\x00\x00\x00" + fixtures.node(
            0,
            struct.pack("<IHHII", 4, 4096, 2048, 3, 57),
            fixtures.node(2, bytes([1]) + fixtures.string("f"), area),
        )
        with self.assertRaises(convert.ConvertError):
            self.build_with_fill(raw)

    def test_replacement_swaps_water_for_official_land_and_nothing_else(self):
        water, water2, land_id = 4597, 4598, 555
        base = self.fill_map(
            (
                7,
                [
                    (1, 1, [self.item(water), self.item(1234)]),  # replaced (2 items)
                    (2, 1, [self.item(water)]),  # minimap shows water: kept
                    (3, 1, [self.item(land_id)]),  # base land: never replaced
                    (5, 1, [self.item(water)]),  # fragment water: kept
                    (6, 1, [self.item(water)]),  # replaced on another tile
                ],
            ),
            (6, [(1, 1, [self.item(water)])]),  # outside the floors of the rule
        )
        fragment = self.fill_map(
            (
                7,
                [
                    (1, 1, [self.item(100)]),
                    (2, 1, [self.item(100)]),
                    (3, 1, [self.item(100)]),
                    (4, 1, [self.item(100)]),  # base has no tile: ordinary fill
                    (5, 1, [self.item(water2)]),
                    (6, 1, [self.item(100), self.item(555)]),
                ],
            ),
            (6, [(1, 1, [self.item(100)])]),
        )
        shown = {(1001, 1001, 7), (1003, 1001, 7), (1005, 1001, 7), (1006, 1001, 7)}
        out = convert.build(
            {
                convert.OTBM: base,
                convert.ITEMS_XML: ITEMS_XML,
                self.FILL_KEY: fragment,
            },
            ITEMS_BY_SERVER_ID,
            land=lambda x, y, z: (x, y, z) in shown or (x, y, z) == (1001, 1001, 6),
        )
        summary = self.summary(out)
        self.assertEqual(
            summary["replace"],
            {
                "items_added": 3,
                "items_removed": 3,
                "member": "blue_valley.otbm",
                "rule": convert.REPLACE_RULE,
                "tiles_replaced": 2,
                "tiles_replaced_by_floor": {"7": 2},
            },
        )
        self.assertEqual(summary["fill"]["tiles_added"], 1)
        self.assertEqual(summary["totals"]["tiles"], 6 + 1)
        palette = [
            r["source_item_id"] for r in json.loads(out[validate.INDEX])["palette"]
        ]
        _z, _rx, _ry, sectors = codec.decode_region(self.decode_path(out, 7))
        tiles = {
            (x, y): [palette[i] for i, _d, _a in items]
            for _local, rows in sectors
            for x, y, _f, _h, _z, items in rows
        }
        self.assertEqual(
            tiles,
            {
                (1001, 1001): [100],
                (1002, 1001): [water],
                (1003, 1001): [land_id],
                (1004, 1001): [100],
                (1005, 1001): [water],
                (1006, 1001): [100, 555],
            },
        )
        floor6 = codec.decode_region(self.decode_path(out, 6))[3]
        self.assertEqual(
            [
                palette[i]
                for _l, rows in floor6
                for *_r, items in rows
                for i, *_ in items
            ],
            [water],
        )
        root = self.install(out)
        self.assertEqual(validate.validate(root, workers=1), [])
        # the world.otbm totals stay checkable: 6 base tiles with 7 items in all
        self.assertEqual(validate.validate(root, {"items": 7, "tiles": 6}), [])
        tampered = json.loads(out[str(convert.SUMMARY.relative_to(convert.ROOT))])
        tampered["replace"]["tiles_replaced"] = 9
        tampered["replace"]["tiles_replaced_by_floor"] = {"7": 9}
        (root / convert.SUMMARY.relative_to(convert.ROOT)).write_bytes(
            validate.canonical(tampered)
        )
        self.assertTrue(
            any(
                "outside the pinned rule" in e
                for e in validate.validate(root, workers=1)
            )
        )

    def test_a_replaced_base_tile_with_a_house_is_refused(self):
        house = fixtures.node(14, bytes([1, 1]) + struct.pack("<I", 9), self.item(4597))
        area = fixtures.node(4, struct.pack("<HHB", 1000, 1000, 7), house)
        base = b"\x00\x00\x00\x00" + fixtures.node(
            0,
            struct.pack("<IHHII", 4, 2048, 2048, 3, 57),
            fixtures.node(2, bytes([1]) + fixtures.string("b"), area),
        )
        fragment = self.fill_map((7, [(1, 1, [self.item(100)])]))
        blobs = {convert.OTBM: base, convert.ITEMS_XML: ITEMS_XML}
        with self.assertRaises(convert.ConvertError):
            convert.build(
                {**blobs, self.FILL_KEY: fragment},
                ITEMS_BY_SERVER_ID,
                land=lambda *_: True,
            )

    def selected_fill(self, exclude_box):
        """The summer-style partial fill with its exclusion box moved onto the fixture."""
        row = json.loads(json.dumps(convert.FILL[1]))
        row["select"]["exclude"][0].update(bbox=exclude_box, floors=[12, 12])
        return row

    def test_selective_fill_takes_components_land_tiles_and_never_overwrites(self):
        row = self.selected_fill([1010, 1010, 1012, 1012])
        one = [self.item(100)]
        raw = self.fill_map(
            (
                12,
                [
                    (10, 10, one),  # inside the box: the whole component is excluded
                    (11, 10, one),
                    (11, 11, one),
                    (16, 16, one),  # elsewhere: included
                    (17, 16, one),
                    (20, 20, one),  # diagonal to (21, 21): 4-connectivity splits them
                    (21, 21, one),
                ],
            ),
            (
                9,
                [(10, 10, one)],  # same box, but floor 9 is outside its floors 12-12
            ),
            (
                7,
                [
                    (1, 1, [self.item(555)]),  # the base has it: never overwritten
                    (22, 22, one),  # official land: kept
                    (23, 22, one),  # no official land: dropped
                ],
            ),
        )
        key = convert.fill_key(row)
        rows = [convert.FILL[0], row]

        def land(x, y, z):
            return (x, y, z) == (1022, 1022, 7)

        with (
            mock.patch.object(convert, "FILL", rows),
            mock.patch.object(validate, "FILL", rows),
        ):
            out = convert.build({**self.blobs, key: raw}, ITEMS_BY_SERVER_ID, land=land)
            self.assertEqual(validate.validate(self.install(out), workers=1), [])
        source = self.summary(out)["fill"]["sources"][0]
        self.assertEqual(source["tiles_added"], 6)
        self.assertEqual(source["tiles_added_by_floor"], {"12": 4, "7": 1, "9": 1})
        self.assertEqual(source["tiles_skipped_existing"], 1)
        self.assertEqual(
            source["selection"],
            {
                "components_excluded": 1,
                "components_included": 4,
                "tiles_excluded_by_floor": {"12": 3},
                "tiles_not_selected": 4,
                "tiles_without_land_by_floor": {"7": 1},
            },
        )
        palette = [
            r["source_item_id"] for r in json.loads(out[validate.INDEX])["palette"]
        ]
        _z, _rx, _ry, sectors = codec.decode_region(self.decode_path(out, 7))
        tiles = {
            (x, y): [(palette[i], d, a) for i, d, a in items]
            for _local, rows_ in sectors
            for x, y, _f, _h, _z, items in rows_
        }
        self.assertEqual(tiles[(1001, 1001)], [(1234, 0, {"door": 3})])
        self.assertEqual(tiles[(1022, 1022)], [(100, 0, None)])
        self.assertNotIn((1023, 1022), tiles)
        floor12 = codec.decode_region(self.decode_path(out, 12))[3]
        self.assertEqual(
            sorted((x, y) for _local, tiles in floor12 for x, y, *_ in tiles),
            [(1016, 1016), (1017, 1016), (1020, 1020), (1021, 1021)],
        )

    def test_reading_the_archive_needs_py7zr(self):
        with (
            mock.patch.dict(sys.modules, {"py7zr": None}),
            self.assertRaises(convert.ConvertError),
        ):
            convert.extract_member(b"", "blue_valley.otbm")

    def test_without_a_fill_the_source_lists_none(self):
        self.assertEqual(json.loads(self.out[validate.INDEX])["source"]["fill"], [])
        self.assertEqual(self.summary()["fill"]["sources"], [])

    def test_conversion_is_deterministic(self):
        self.assertEqual(convert.build(self.blobs, ITEMS_BY_SERVER_ID), self.out)

    def test_split_of_provisional_ids_follows_items_xml(self):
        blobs = {**self.blobs, convert.ITEMS_XML: ITEMS_XML_WITH_TELEPORT}
        provisional = self.summary(convert.build(blobs, ITEMS_BY_SERVER_ID))["palette"]
        self.assertEqual(
            provisional["provisional"],
            {
                "appearance_only": {"entries": 0, "occurrences": 0},
                "entries": 1,
                "in_items_xml": {"entries": 1, "occurrences": 2},
                "occurrences": 2,
            },
        )

    def test_binding_to_an_undefined_item_key_is_provisional(self):
        defined = {GOLD, "oteryn:x"}
        out = convert.build(self.blobs, ITEMS_BY_SERVER_ID, defined=defined)
        palette = json.loads(out[validate.INDEX])["palette"]
        row = next(r for r in palette if r["source_item_id"] == 1234)
        self.assertEqual(
            row, {"key": f"{DONOR}1234", "provisional": True, "source_item_id": 1234}
        )
        self.assertEqual(
            convert.bound_keys(ITEMS_BY_SERVER_ID, defined), {100: GOLD, 7: "oteryn:x"}
        )
        self.assertEqual(len(convert.bound_keys(ITEMS_BY_SERVER_ID)), 3)

    def test_validator_rejects_an_item_key_without_a_definition(self):
        write_definitions(self.root, ITEMS_BY_SERVER_ID, skip={REGISTRY})
        self.assertTrue(
            any("nor an Item key with a record" in e for e in self.errors())
        )
        out = convert.build(self.blobs, ITEMS_BY_SERVER_ID, defined={GOLD, "oteryn:x"})
        for path, data in out.items():
            (self.root / path).write_bytes(data)
        self.assertEqual(self.errors(), [])

    def test_without_bindings_every_id_is_provisional_and_regions_do_not_change(self):
        out = convert.build(self.blobs, bindings_for([]))
        palette = json.loads(out[validate.INDEX])["palette"]
        self.assertEqual(
            [
                (row["source_item_id"], row["key"], row["provisional"])
                for row in palette
            ],
            [(i, f"{DONOR}{i}", True) for i in (100, 1234, 1949)],
        )
        region = f"{validate.DIRECTORY}/region-z07-x003-y003.b3"
        self.assertEqual(out[region], self.out[region])

    def test_ambiguous_or_shared_bindings_fail_closed(self):
        for pairs in (
            [(100, GOLD), (100, REGISTRY)],
            [(100, GOLD), (1234, GOLD)],
        ):
            with self.assertRaises(convert.ConvertError, msg=pairs):
                convert.build(self.blobs, bindings_for(pairs))

    def test_unpopulated_marker_is_accepted_until_the_family_exists(self):
        marker = {
            "schema": "OTERYN_GAME_TREE_DIRECTORY/v1",
            "population_state": "READY_UNPOPULATED",
        }
        for path in (self.root / validate.DIRECTORY).iterdir():
            path.unlink()
        (self.root / validate.INDEX).write_text(json.dumps(marker, indent=2))
        self.assertEqual(self.errors(), [])
        (self.root / validate.DIRECTORY / "region-z07-x003-y003.b3").write_bytes(b"x")
        with self.assertRaises(validate.ValidationError):
            self.errors()

    def test_unsupported_attributes_fail_closed(self):
        for attr, size in ((12, 1), (99, 1)):
            item = fixtures.node(
                6, fixtures.struct.pack("<H", 100) + bytes([attr] + [1] * size)
            )
            area = fixtures.node(
                4,
                fixtures.struct.pack("<HHB", 0, 0, 7),
                fixtures.node(5, b"\x00\x00", item),
            )
            map_data = fixtures.node(2, b"\x01" + fixtures.string("x"), area)
            raw = b"\x00\x00\x00\x00" + fixtures.node(
                0, fixtures.struct.pack("<IHHII", 4, 64, 64, 3, 57), map_data
            )
            with self.assertRaises(convert.ConvertError, msg=attr):
                convert.build(
                    {convert.OTBM: raw, convert.ITEMS_XML: ITEMS_XML},
                    ITEMS_BY_SERVER_ID,
                )

    def test_reader_streams_tiles_with_containers_and_inline_items(self):
        inner = fixtures.node(6, fixtures.struct.pack("<H", 1949))
        bag = fixtures.node(6, fixtures.struct.pack("<H", 100) + bytes([15, 4]), inner)
        tile = fixtures.node(
            5,
            bytes([0, 0, 3])
            + fixtures.struct.pack("<I", 9)
            + bytes([9])
            + fixtures.struct.pack("<H", 1234),
            bag,
            fixtures.node(19, fixtures.struct.pack("<HHH", 2, 1, 65535)),
        )
        area = fixtures.node(4, fixtures.struct.pack("<HHB", 256, 0, 0), tile)
        map_data = fixtures.node(2, b"\x01" + fixtures.string("x"), area)
        raw = b"\x00\x00\x00\x00" + fixtures.node(
            0, fixtures.struct.pack("<IHHII", 4, 512, 64, 3, 57), map_data
        )
        seen = []
        otbm_reader.read_tiles(raw, lambda *args: seen.append(args))
        self.assertEqual(
            seen,
            [
                (
                    256,
                    0,
                    0,
                    9,
                    None,
                    (1, 65535),
                    [(1234, 0, {}), (100, 0, {15: 4}), (1949, 1, {})],
                )
            ],
        )

    def test_tampered_region_is_a_sha_mismatch(self):
        data = bytearray(self.region.read_bytes())
        data[-1] ^= 1
        self.region.write_bytes(bytes(data))
        self.assertTrue(any("sha256" in e for e in self.errors()))

    def test_stray_and_missing_files_are_rejected(self):
        (self.root / validate.DIRECTORY / "extra.b3").write_bytes(b"x")
        self.assertTrue(any("files other than" in e for e in self.errors()))
        (self.root / validate.DIRECTORY / "extra.b3").unlink()
        self.region.unlink()
        self.assertTrue(any("files other than" in e for e in self.errors()))

    def edit_palette(self, edit):
        index = self.index()
        edit(index["palette"])
        self.write_index(index)
        return self.errors()

    def test_palette_rules_are_enforced(self):
        def unused(palette):
            palette.append(
                {"key": DONOR + "2000", "provisional": True, "source_item_id": 2000}
            )

        def out_of_range(palette):
            palette.pop()

        def provisional_but_bound(palette):
            palette[1] = {**palette[1], "key": DONOR + "1234", "provisional": True}

        def wrong_binding(palette):
            palette[0] = {**palette[0], "key": "oteryn:item.currency.other"}

        def wrong_revision(palette):
            palette[2] = {**palette[2], "key": "donor:crystalserver@ff7ede59:item/1949"}

        def retired_but_used(palette):
            palette[0] = {**palette[0], "retired": True}

        def retired_not_true(palette):
            palette[0] = {**palette[0], "retired": False}

        def duplicate_key(palette):
            palette[1] = {**palette[1], "key": GOLD}

        def duplicate_id(palette):
            palette[1] = {**palette[1], "source_item_id": 100}

        def extra_field(palette):
            palette[0] = {**palette[0], "name": "x"}

        for edit, expected in (
            (unused, "no item uses"),
            (out_of_range, "palette indexes outside"),
            (provisional_but_bound, "has a family key"),
            (wrong_binding, "nor an Item key with a record"),
            (wrong_revision, "provisional key must be"),
            (retired_but_used, "retired palette entries an item still uses"),
            (retired_not_true, "malformed entry"),
            (duplicate_key, "listed twice"),
            (duplicate_id, "listed twice"),
            (extra_field, "malformed entry"),
        ):
            with self.subTest(edit.__name__):
                errors = self.edit_palette(edit)
                self.assertTrue(any(expected in e for e in errors), errors)
                self.write_index(json.loads(self.out[validate.INDEX]))
        self.assertEqual(self.errors(), [])

    def test_appearance_class_follows_one_ordered_flag_rule(self):
        import convert_appearance_only_ids as appearance_only

        rule = appearance_only.appearance_class
        self.assertEqual(rule(frozenset({"bank", "clip", "unpass"})), "ground")
        self.assertEqual(rule(frozenset({"clip", "unpass"})), "border")
        self.assertEqual(rule(frozenset({"unpass", "unmove"})), "blocking")
        self.assertEqual(rule(frozenset({"unmove"})), "decoration")
        self.assertEqual(rule(frozenset()), "decoration")

    def test_floor_change_summary_counts_kinds_present_on_the_map(self):
        kinds = {100: "down", 101: "down", 102: "up_north", 103: "rope"}
        counts = convert.Counter({100: 3, 102: 2, 999: 5})
        self.assertEqual(
            convert.floor_change_summary(kinds, counts),
            {
                "by_kind": {
                    "down": {"item_types": 1, "occurrences": 3},
                    "up_north": {"item_types": 1, "occurrences": 2},
                },
                "item_types": 2,
                "occurrences": 5,
            },
        )

    TERRAIN_KEY = "oteryn:terrain.tibia.i1949"
    OBJECT_KEY = "oteryn:world-object.tibia.i1949"

    def write_catalogue(self, root, family, keys):
        """A minimal committed A12 section 4.6 catalogue: one record per Tibia id."""
        directory, stem, _prefix = convert.CATALOGUES[family]
        records = [{"identity": {"key": key}} for _id, key in sorted(keys.items())]
        (root / directory).mkdir(parents=True, exist_ok=True)
        (root / f"{directory}/{stem}00000-00000.json").write_text(
            json.dumps({"records": records})
        )

    def test_palette_resolution_order_is_terrain_object_item_provisional(self):
        terrain, objects = {1949: self.TERRAIN_KEY}, {1949: self.OBJECT_KEY}
        both = self.palette_of(
            convert.build(
                self.blobs, ITEMS_BY_SERVER_ID, terrain=terrain, world_object=objects
            )
        )
        self.assertEqual(both[2]["key"], self.TERRAIN_KEY)
        only_object = self.palette_of(
            convert.build(self.blobs, ITEMS_BY_SERVER_ID, world_object=objects)
        )
        self.assertEqual(only_object[2]["key"], self.OBJECT_KEY)
        # an Item-bound id that a catalogue also routes takes the catalogue key
        routed = self.palette_of(
            convert.build(
                self.blobs,
                ITEMS_BY_SERVER_ID,
                terrain={100: "oteryn:terrain.tibia.i100"},
            )
        )
        self.assertEqual(routed[0]["key"], "oteryn:terrain.tibia.i100")
        self.assertFalse(routed[0]["provisional"])
        # an id no family covers stays provisional
        self.assertTrue(self.palette_of(self.out)[2]["provisional"])

    def test_catalogue_ids_take_their_family_key(self):
        terrain = {1949: self.TERRAIN_KEY}
        out = convert.build(self.blobs, ITEMS_BY_SERVER_ID, terrain=terrain)
        self.assertEqual(
            self.palette_of(out)[2],
            {"key": self.TERRAIN_KEY, "provisional": False, "source_item_id": 1949},
        )
        summary = self.summary(out)["palette"]
        self.assertEqual(
            summary["families"]["terrain"], {"entries": 1, "occurrences": 2}
        )
        self.assertEqual(summary["provisional"]["entries"], 0)
        # Only `index.json` and the summary change: region files keep their bytes.
        region = f"{validate.DIRECTORY}/region-z07-x003-y003.b3"
        self.assertEqual(out[region], self.out[region])
        root = self.install(out)
        self.write_catalogue(root, "terrain", terrain)
        self.assertEqual(validate.validate(root), [])
        self.assertEqual(convert.terrain_keys(root), terrain)
        self.assertEqual(convert.world_object_keys(root), {})

    def test_palette_key_must_belong_to_a_catalogue_record_of_that_id(self):
        out = convert.build(
            self.blobs, ITEMS_BY_SERVER_ID, terrain={1949: self.TERRAIN_KEY}
        )
        root = self.install(out)
        # no catalogue: the key is neither a catalogue key nor an Item binding
        errors = validate.validate(root)
        self.assertTrue(
            any("nor an Item key with a record" in e for e in errors), errors
        )
        # the catalogue routes the id to a different family
        self.write_catalogue(root, "world_object", {1949: self.OBJECT_KEY})
        errors = validate.validate(root)
        self.assertTrue(any("must use" in e for e in errors), errors)
        self.write_catalogue(root, "terrain", {1949: self.TERRAIN_KEY})
        self.assertEqual(validate.validate(root), [])

    def test_a_routed_item_bound_id_must_use_the_catalogue_key(self):
        root = self.install(self.out)
        self.write_catalogue(root, "terrain", {100: "oteryn:terrain.tibia.i100"})
        errors = validate.validate(root)
        self.assertTrue(any("id 100 must use" in e for e in errors), errors)

    def test_provisional_id_with_a_catalogue_record_is_rejected(self):
        root = self.install(self.out)
        self.write_catalogue(root, "terrain", {1949: self.TERRAIN_KEY})
        errors = validate.validate(root)
        self.assertTrue(any("has a family key" in e for e in errors), errors)

    def test_a_catalogue_key_must_spell_its_family_and_id(self):
        root = self.install(self.out)
        self.write_catalogue(root, "terrain", {1949: "oteryn:terrain.a001949"})
        with self.assertRaises(convert.ConvertError):
            convert.terrain_keys(root)

    def install(self, out):
        """Write a build into a fresh temp root and return it."""
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root)
        for path, data in out.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_bytes(data)
        (root / validate.ITEM_BINDINGS).parent.mkdir(parents=True, exist_ok=True)
        (root / validate.ITEM_BINDINGS).write_bytes(ITEMS_BY_SERVER_ID)
        write_definitions(root, ITEMS_BY_SERVER_ID)
        return root

    @staticmethod
    def palette_of(out):
        return json.loads(out[validate.INDEX])["palette"]

    @staticmethod
    def regions_of(out):
        return {p: d for p, d in out.items() if p.endswith(".b3")}

    def map_with_lower_id(self):
        """The fixture plus one tile of server id 50 in another region (x5, y5)."""
        tile = fixtures.node(5, bytes([1, 1]), fixtures.node(6, struct.pack("<H", 50)))
        area = fixtures.node(4, struct.pack("<HHB", 1280, 1280, 7), tile)
        return {**self.blobs, convert.OTBM: fixtures.fixture_map(extra_areas=(area,))}

    def test_new_lower_id_is_appended_and_untouched_regions_stay_identical(self):
        blobs = self.map_with_lower_id()
        out = convert.build(blobs, ITEMS_BY_SERVER_ID, self.palette_of(self.out))
        palette = self.palette_of(out)
        self.assertEqual(palette[:3], self.palette_of(self.out))
        self.assertEqual(
            palette[3], {"key": DONOR + "50", "provisional": True, "source_item_id": 50}
        )
        old_regions = self.regions_of(self.out)
        new_regions = self.regions_of(out)
        self.assertEqual(len(new_regions), len(old_regions) + 1)
        for path, data in old_regions.items():
            self.assertEqual(new_regions[path], data)
        # A fresh build sorts 50 first, renumbers every index and rewrites the old region.
        fresh = convert.build(blobs, ITEMS_BY_SERVER_ID)
        self.assertEqual(self.palette_of(fresh)[0]["source_item_id"], 50)
        region = f"{validate.DIRECTORY}/region-z07-x003-y003.b3"
        self.assertNotEqual(fresh[region], self.out[region])
        self.assertEqual(validate.validate(self.install(out)), [])

    def test_binding_change_rewrites_only_the_index(self):
        provisional = convert.build(self.blobs, bindings_for([]))
        out = convert.build(
            self.blobs, ITEMS_BY_SERVER_ID, self.palette_of(provisional)
        )
        self.assertEqual(self.regions_of(out), self.regions_of(provisional))
        self.assertEqual(out, self.out)
        self.assertNotEqual(out[validate.INDEX], provisional[validate.INDEX])
        self.assertEqual(
            [row["source_item_id"] for row in self.palette_of(out)], [100, 1234, 1949]
        )
        self.assertEqual(self.palette_of(out)[0]["key"], GOLD)

    def test_unused_entry_is_kept_and_flagged_retired(self):
        previous = convert.build(self.map_with_lower_id(), ITEMS_BY_SERVER_ID)
        out = convert.build(self.blobs, ITEMS_BY_SERVER_ID, self.palette_of(previous))
        palette = self.palette_of(out)
        self.assertEqual(
            [row["source_item_id"] for row in palette], [50, 100, 1234, 1949]
        )
        self.assertEqual(
            [row.get("retired") for row in palette], [True, None, None, None]
        )
        region = f"{validate.DIRECTORY}/region-z07-x003-y003.b3"
        self.assertEqual(out[region], previous[region])
        self.assertEqual(self.summary(out)["palette"]["entries"], 4)
        root = self.install(out)
        self.assertEqual(validate.validate(root), [])
        # A retired entry that the map uses again is revived and loses the flag.
        again = convert.build(self.map_with_lower_id(), ITEMS_BY_SERVER_ID, palette)
        self.assertEqual(again, previous)

    def test_committed_palette_with_a_repeated_id_fails_closed(self):
        palette = self.palette_of(self.out)
        with self.assertRaises(convert.ConvertError):
            convert.build(self.blobs, ITEMS_BY_SERVER_ID, palette + palette[:1])

    def test_bindings_change_makes_a_registry_entry_stale(self):
        self.write_bindings(bindings_for([(100, GOLD), (1234, GOLD.upper())]))
        self.assertTrue(
            any("nor an Item key with a record" in e for e in self.errors())
        )
        self.write_bindings(
            bindings_for([(100, GOLD), (1234, REGISTRY), (1949, "a:b")])
        )
        self.assertTrue(any("has a family key" in e for e in self.errors()))

    def test_summary_palette_counts_are_checked(self):
        path = self.root / validate.SUMMARY
        summary = json.loads(path.read_text())
        summary["palette"]["provisional"]["occurrences"] += 1
        path.write_bytes(validate.canonical(summary))
        self.assertTrue(
            any("provisional entries/occurrences" in e for e in self.errors())
        )
        summary["palette"]["provisional"]["occurrences"] -= 1
        summary["palette"]["provisional"]["in_items_xml"]["entries"] += 1
        path.write_bytes(validate.canonical(summary))
        self.assertTrue(any("does not add up" in e for e in self.errors()))

    def replace_region(self, data):
        self.region.write_bytes(data)
        index = self.index()
        index["regions"][0]["sha256"] = validate.hashlib.sha256(data).hexdigest()
        self.write_index(index)

    def test_misordered_sector_table_and_wrong_header_are_rejected(self):
        payload = codec.encode_sector_tiles(24, 24, [(768, 768, 0, 0, (), [])])
        good = codec.encode_region(7, 3, 3, {0: payload, 1: payload})
        entry = codec.HEADER.size
        swapped = bytearray(good)
        swapped[entry], swapped[entry + 9] = swapped[entry + 9], swapped[entry]
        self.replace_region(bytes(swapped))
        self.assertTrue(any("strictly ascending" in e for e in self.errors()))
        self.replace_region(codec.encode_region(7, 3, 4, {0: payload}))
        self.assertTrue(any("file name" in e for e in self.errors()))

    def test_tile_outside_map_extent_or_region_and_wrong_counts_are_rejected(self):
        summary_path = self.root / validate.SUMMARY
        summary = json.loads(summary_path.read_text())
        summary["map"]["width"] = 1001
        summary_path.write_bytes(validate.canonical(summary))
        self.assertTrue(any("outside the map extent" in e for e in self.errors()))
        summary["map"]["width"] = 2048
        summary["totals"]["tiles"] += 1
        summary["tiles_by_floor"]["7"] += 1
        summary_path.write_bytes(validate.canonical(summary))
        self.assertTrue(any("totals" in e for e in self.errors()))
        index = self.index()
        index["regions"][0]["tiles"] += 1
        self.write_index(index)
        self.assertTrue(any("decoded 4 tiles" in e for e in self.errors()))

    def test_pinned_totals_and_non_canonical_index(self):
        self.assertTrue(validate.validate(self.root, {"tiles": 1, "items": 1}))
        path = self.root / validate.INDEX
        path.write_text(path.read_text() + " ")
        with self.assertRaises(validate.ValidationError):
            validate.validate(self.root)


if __name__ == "__main__":
    unittest.main()
