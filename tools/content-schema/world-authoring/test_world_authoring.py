"""Fixture tests for the OTBM reader, the metadata converter and the validator."""

from __future__ import annotations

import gzip
import json
import shutil
import struct
import tempfile
import unittest
from pathlib import Path

import convert_world_metadata as convert
import otbm_reader
import validate_world_metadata as validate

TELEPORT_ITEM = 1949  # bound in imports/crystalserver/bindings/items.json


def escape(payload: bytes) -> bytes:
    out = bytearray()
    for byte in payload:
        if byte in (0xFD, 0xFE, 0xFF):
            out.append(0xFD)
        out.append(byte)
    return bytes(out)


def node(kind: int, props: bytes = b"", *children: bytes) -> bytes:
    return b"\xfe" + bytes([kind]) + escape(props) + b"".join(children) + b"\xff"


def string(text: str) -> bytes:
    return struct.pack("<H", len(text)) + text.encode("latin-1")


def fixture_map() -> bytes:
    """One town, one house (id 0xFE forces escaping) with a door, two teleports."""
    door = node(6, struct.pack("<H", 1234) + bytes([14, 3]))
    teleport = node(
        6,
        struct.pack("<H", TELEPORT_ITEM)
        + bytes([8])
        + struct.pack("<HHB", 1002, 1002, 7),
    )
    unset = node(
        6, struct.pack("<H", TELEPORT_ITEM) + bytes([8]) + struct.pack("<HHB", 0, 0, 0)
    )
    area = node(
        4,
        struct.pack("<HHB", 1000, 1000, 7),
        node(14, bytes([1, 1]) + struct.pack("<I", 0xFE), door),
        node(
            14,
            bytes([2, 1])
            + struct.pack("<I", 0xFE)
            + bytes([9])
            + struct.pack("<H", 100),
        ),
        node(5, bytes([1, 3]), teleport, unset),
        node(5, bytes([2, 2]) + bytes([3]) + struct.pack("<I", 0)),
    )
    towns = node(
        12,
        b"",
        node(
            13,
            struct.pack("<I", 7)
            + string("Test Town")
            + struct.pack("<HHB", 1001, 1001, 7),
        ),
    )
    waypoints = node(
        15, b"", node(16, string("wp") + struct.pack("<HHB", 1000, 1000, 7))
    )
    map_data = node(2, bytes([1]) + string("fixture"), area, towns, waypoints)
    return b"\x00\x00\x00\x00" + node(
        0, struct.pack("<IHHII", 4, 2048, 2048, 3, 57), map_data
    )


HOUSE_XML = (
    b'<?xml version="1.0"?><houses><house name="Fixture Hall" houseid="254" entryx="1001" '
    b'entryy="1002" entryz="7" rent="1000" guildhall="true" townid="7" size="2" clientid="1" '
    b'beds="1" /></houses>'
)


class ReaderTest(unittest.TestCase):
    def test_reads_towns_houses_teleports_and_escaped_bytes(self):
        for raw in (fixture_map(), gzip.compress(fixture_map())):
            facts = otbm_reader.read(raw)
            self.assertEqual(
                (facts.version, facts.width, facts.height, facts.tiles),
                (4, 2048, 2048, 4),
            )
            self.assertEqual(
                facts.towns,
                [{"town_id": 7, "name": "Test Town", "temple": (1001, 1001, 7)}],
            )
            self.assertEqual(
                facts.waypoints, [{"name": "wp", "position": (1000, 1000, 7)}]
            )
            house = facts.houses[0xFE]
            self.assertEqual(
                (house.tiles, house.bbox, house.doors),
                (2, [1001, 1001, 1002, 1001], [(1001, 1001, 7, 3)]),
            )
            self.assertEqual(
                [tp["to"] for tp in facts.teleports], [(1002, 1002, 7), (0, 0, 0)]
            )
            self.assertFalse(facts.unknown_item_attrs)

    def test_probe_reports_only_present_positions(self):
        facts = otbm_reader.read(fixture_map(), probe={(1002, 1002, 7), (5, 5, 7)})
        self.assertEqual(facts.present, {(1002, 1002, 7)})

    def test_rejects_truncated_and_foreign_files(self):
        with self.assertRaises(otbm_reader.OtbmError):
            otbm_reader.read(fixture_map()[:-3])
        with self.assertRaises(otbm_reader.OtbmError):
            otbm_reader.read(b"not a map at all")


class ConvertAndValidateTest(unittest.TestCase):
    def setUp(self):
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
            "data-global/world/world-house.xml": HOUSE_XML,
        }
        self.out = convert.build(blobs)
        self.root = Path(tempfile.mkdtemp())
        for path, data in self.out.items():
            (self.root / path).parent.mkdir(parents=True, exist_ok=True)
            (self.root / path).write_bytes(data)
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(convert.ITEM_BINDINGS, bindings)

    def tearDown(self):
        shutil.rmtree(self.root)

    def shard(self, family):
        return json.loads(
            (
                self.root
                / f"{validate.FAMILIES[family][0]}/{validate.FAMILIES[family][1]}-00000-00000.json"
            ).read_text()
        )

    def rewrite(self, family, shard):
        path = (
            self.root
            / f"{validate.FAMILIES[family][0]}/{validate.FAMILIES[family][1]}-00000-00000.json"
        )
        path.write_text(validate.canonical(shard), encoding="utf-8")

    def test_fixture_converts_to_valid_families(self):
        self.assertEqual(validate.validate(self.root), [])
        summary = json.loads(self.out[str(convert.SUMMARY.relative_to(convert.ROOT))])
        self.assertEqual(
            summary["families"], {"Area.City": 1, "House": 1, "Transition.Teleport": 1}
        )
        self.assertEqual(summary["not_imported"]["teleports"]["unset_destination"], 1)
        house = self.shard("House")["records"][0]["declaration"]
        self.assertEqual(house["identity"]["key"], "oteryn:house.fixture_hall")
        self.assertEqual(house["city"]["key"], "oteryn:area.city.test_town")
        self.assertEqual(
            (house["declared_size"], house["footprint"]["tile_count"]), (2, 2)
        )
        teleport = self.shard("Transition.Teleport")["records"][0]["declaration"]
        self.assertEqual(
            teleport["identity"]["key"], "oteryn:transition.teleport.x1001_y1003_z7"
        )
        self.assertEqual(teleport["object"]["key"], convert.item_keys()[TELEPORT_ITEM])

    def test_conversion_is_deterministic(self):
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
            "data-global/world/world-house.xml": HOUSE_XML,
        }
        self.assertEqual(convert.build(blobs), self.out)

    def test_house_xml_must_match_map_house_tiles(self):
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
            "data-global/world/world-house.xml": HOUSE_XML.replace(b'"254"', b'"9"'),
        }
        with self.assertRaises(convert.ConvertError):
            convert.build(blobs)

    def test_rejects_unknown_city_reference(self):
        shard = self.shard("House")
        shard["records"][0]["declaration"]["city"]["key"] = "oteryn:area.city.nowhere"
        self.rewrite("House", shard)
        self.assertTrue(
            any("is not a City Area" in e for e in validate.validate(self.root))
        )

    def test_rejects_stray_file_in_family_directory(self):
        (self.root / "content/world/transitions/extra.json").write_text("{}\n")
        self.assertTrue(
            any("files other than" in e for e in validate.validate(self.root))
        )

    def test_rejects_schema_violation_and_non_canonical_bytes(self):
        shard = self.shard("Area.City")
        shard["records"][0]["declaration"]["temple"]["floor"] = 16
        self.rewrite("Area.City", shard)
        path = self.root / "content/world/areas/cities/index.json"
        path.write_text(path.read_text() + " ")
        errors = validate.validate(self.root)
        self.assertTrue(any("schema" in e for e in errors))
        self.assertTrue(any("not canonical" in e for e in errors))

    def test_rejects_binding_that_targets_another_record(self):
        shard = self.shard("Transition.Teleport")
        shard["records"][0]["source_bindings"][0]["target"]["key"] = (
            "oteryn:transition.teleport.x1_y1_z7"
        )
        self.rewrite("Transition.Teleport", shard)
        self.assertTrue(
            any("binding target" in e for e in validate.validate(self.root))
        )


class CommittedContentTest(unittest.TestCase):
    def test_committed_families_validate(self):
        self.assertEqual(validate.validate(convert.ROOT), [])


if __name__ == "__main__":
    unittest.main()
