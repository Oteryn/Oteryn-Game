"""Tests for the region codec, the base map converter and the base map validator."""

from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

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
ITEMS_BY_SERVER_ID = json.dumps(
    {
        "bindings": [
            {
                "external_id": str(server_id),
                "identity_namespace": "ots/item_server_id",
                "target": {"key": f"oteryn:item.registry.i{number:08}"},
            }
            for server_id, number in ((100, 7), (1234, 90000000), (1949, 3))
        ]
    }
).encode()


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
        self.blobs = {convert.OTBM: fixtures.fixture_map()}
        self.out = convert.build(self.blobs, ITEMS_BY_SERVER_ID)
        self.root = Path(tempfile.mkdtemp())
        for path, data in self.out.items():
            (self.root / path).parent.mkdir(parents=True, exist_ok=True)
            (self.root / path).write_bytes(data)
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        bindings.write_bytes(ITEMS_BY_SERVER_ID)
        self.region = self.root / validate.DIRECTORY / "region-z07-x003-y003.b3"

    def tearDown(self):
        shutil.rmtree(self.root)

    def index(self):
        return json.loads((self.root / validate.INDEX).read_text())

    def write_index(self, index):
        (self.root / validate.INDEX).write_bytes(validate.canonical(index))

    def errors(self):
        return validate.validate(self.root)

    def test_fixture_converts_to_a_valid_family(self):
        self.assertEqual(self.errors(), [])
        index = self.index()
        self.assertEqual(
            index["totals"], {"items": 4, "regions": 1, "sectors": 1, "tiles": 4}
        )
        self.assertEqual(index["shards"], [row["path"] for row in index["regions"]])
        self.assertEqual(index["source"]["files"][0]["path"], convert.OTBM)
        summary = json.loads(self.out[str(convert.SUMMARY.relative_to(convert.ROOT))])
        self.assertEqual(summary["tiles_by_floor"], {"7": 4})
        self.assertEqual(summary["tiles_with_house"], 2)
        self.assertEqual(summary["item_attributes"], {"door": 1, "teleport": 2})
        z, rx, ry, sectors = codec.decode_region(self.region.read_bytes())
        self.assertEqual((z, rx, ry), (7, 3, 3))
        tiles = {(x, y): (f, h, zn, i) for x, y, f, h, zn, i in sectors[0][1]}
        self.assertEqual(
            tiles,
            {
                (1000 + 1, 1000 + 1): (0, 0xFE, (), [(90000000, 0, {"door": 3})]),
                (1002, 1001): (0, 0xFE, (), [(7, 0, None)]),
                (1001, 1003): (
                    0,
                    0,
                    (),
                    [
                        (3, 0, {"teleport": (1002, 1002, 7)}),
                        (3, 0, {"teleport": (0, 0, 0)}),
                    ],
                ),
                (1002, 1002): (0, 0, (), []),
            },
        )

    def test_conversion_is_deterministic(self):
        self.assertEqual(convert.build(self.blobs, ITEMS_BY_SERVER_ID), self.out)

    def test_unbound_item_ids_fail_closed_and_are_all_reported(self):
        bindings = json.loads(ITEMS_BY_SERVER_ID)
        bindings["bindings"] = [
            b for b in bindings["bindings"] if b["external_id"] == "1949"
        ]
        with self.assertRaises(convert.ConvertError) as raised:
            convert.build(self.blobs, json.dumps(bindings).encode())
        self.assertIn("1234", str(raised.exception))
        self.assertIn("100", str(raised.exception))

    def test_named_item_targets_carry_no_registry_number(self):
        bindings = json.loads(ITEMS_BY_SERVER_ID)
        bindings["bindings"][0]["target"]["key"] = "oteryn:item.currency.gold_coin"
        with self.assertRaises(convert.UnboundItemsError) as raised:
            convert.build(self.blobs, json.dumps(bindings).encode())
        self.assertEqual(
            raised.exception.report()["unbound"],
            [
                {
                    "named_target": "oteryn:item.currency.gold_coin",
                    "occurrences": 1,
                    "server_id": 100,
                }
            ],
        )

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
                convert.build({convert.OTBM: raw}, ITEMS_BY_SERVER_ID)

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

    def test_unbound_registry_number_in_a_region_is_rejected(self):
        bindings = json.loads(ITEMS_BY_SERVER_ID)
        bindings["bindings"] = [
            b for b in bindings["bindings"] if b["external_id"] != "1949"
        ]
        (self.root / validate.ITEM_BINDINGS).write_text(json.dumps(bindings))
        self.assertTrue(any("without a binding" in e for e in self.errors()))

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
