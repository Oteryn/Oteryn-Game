"""Fixture tests for the OTBM reader, the metadata converter and the validator."""

from __future__ import annotations

import gzip
import json
import re
import shutil
import struct
import tempfile
import unittest
from pathlib import Path

import convert_hunting_places as hunting
import convert_world_metadata as convert
import fandom_hunting_snapshot as snapshot_tool
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


def fixture_map(extra_houses=(), extra_areas=()) -> bytes:
    """One town, one house (id 0xFE forces escaping) with a door, two teleports.

    `extra_houses` adds one plain tile per additional house id; `extra_areas` appends raw
    tile-area nodes.
    """
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
        *(
            node(14, bytes([10 + i, 1]) + struct.pack("<I", house_id))
            for i, house_id in enumerate(extra_houses)
        ),
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
    map_data = node(
        2, bytes([1]) + string("fixture"), area, *extra_areas, towns, waypoints
    )
    return b"\x00\x00\x00\x00" + node(
        0, struct.pack("<IHHII", 4, 2048, 2048, 3, 57), map_data
    )


HOUSE_XML = (
    b'<?xml version="1.0"?><houses><house name="Fixture Hall" houseid="254" entryx="1001" '
    b'entryy="1002" entryz="7" rent="1000" guildhall="true" townid="7" size="2" clientid="1" '
    b'beds="1" /></houses>'
)


def house_row(house_id: int, name: str) -> bytes:
    return (
        f'<house name="{name}" houseid="{house_id}" entryx="1001" entryy="1002" '
        'entryz="7" rent="1000" guildhall="false" townid="7" size="1" clientid="1" '
        'beds="1" />'
    ).encode()


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
        self.root = Path(tempfile.mkdtemp())
        self.out = convert.build(blobs, self.root)
        for path, data in self.out.items():
            (self.root / path).parent.mkdir(parents=True, exist_ok=True)
            (self.root / path).write_bytes(data)
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(convert.ITEM_BINDINGS, bindings)
        data = snapshot_tool.canonical(
            snapshot_tool.build_snapshot(PAGES, "2026-09-29")
        )
        (self.root / hunting.SNAPSHOT).parent.mkdir(parents=True, exist_ok=True)
        (self.root / hunting.SNAPSHOT).write_bytes(data)
        for path, blob in hunting.build(data, self.root).items():
            (self.root / path).parent.mkdir(parents=True, exist_ok=True)
            (self.root / path).write_bytes(blob)

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
        self.assertEqual(convert.build(blobs, self.root), self.out)

    def test_house_xml_must_match_map_house_tiles(self):
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
            "data-global/world/world-house.xml": HOUSE_XML.replace(b'"254"', b'"9"'),
        }
        with self.assertRaises(convert.ConvertError):
            convert.build(blobs, self.root)

    def rebuild(self, house_xml: bytes, extra_houses=()) -> dict[str, bytes]:
        """Second build against the families already committed in the temp root."""
        blobs = {
            "data-global/world/world.otbm": fixture_map(extra_houses),
            "data-global/world/world-house.xml": house_xml,
        }
        return convert.build(blobs, self.root)

    @staticmethod
    def house_records(out):
        (path,) = [p for p in out if re.fullmatch(r"content/houses/houses-.*\.json", p)]
        return {
            r["declaration"]["identity"]["key"]: r
            for r in json.loads(out[path])["records"]
        }

    def test_renamed_house_keeps_its_key(self):
        renamed = HOUSE_XML.replace(b"Fixture Hall", b"Renamed Hall")
        records = self.house_records(self.rebuild(renamed))
        self.assertEqual(list(records), ["oteryn:house.fixture_hall"])
        record = records["oteryn:house.fixture_hall"]
        self.assertEqual(record["declaration"]["name"], "Renamed Hall")
        self.assertEqual(
            record["source_bindings"][0]["target"]["key"], "oteryn:house.fixture_hall"
        )

    def test_new_house_gets_a_new_slug_key(self):
        renamed = HOUSE_XML.replace(b"Fixture Hall", b"Renamed Hall")
        xml = renamed.replace(
            b"</houses>", house_row(300, "Second Hall") + b"</houses>"
        )
        records = self.house_records(self.rebuild(xml, (300,)))
        self.assertEqual(
            sorted(records),
            ["oteryn:house.fixture_hall", "oteryn:house.second_hall"],
        )

    def test_new_house_colliding_with_a_committed_key_fails_closed(self):
        xml = HOUSE_XML.replace(
            b"</houses>", house_row(300, "Fixture Hall") + b"</houses>"
        )
        with self.assertRaises(convert.ConvertError):
            self.rebuild(xml, (300,))

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


def wiki_page(pageid: int, title: str, wikitext: str, revid: int | None = None) -> dict:
    return {
        "pageid": pageid,
        "revisions": [
            {
                "revid": revid or pageid * 10,
                "slots": {"main": {"content": wikitext}},
                "timestamp": "2026-01-01T00:00:00Z",
            }
        ],
        "title": title,
    }


CAVE_TEXT = """{{Infobox Hunt
| name            = Test Cave <!-- ignored -->
| city            = [[Test Town]]
| location        = West of Test Town, {{Mapper Coords|3.232|3.233|7|3|text=here}}.
| lvlknights      = 50
| lvlpaladins     = ?
| lvlmages        = 600?
| skknights       = 70
}}
Prose that must never be stored.
{{Infobox Hunt Skills
| areaname = Deep
}}
== Creatures ==
{{CreatureList|type=List/Sorted|caption=Creatures
 |Rat
 |Cave_Rat <!-- appears while digging -->
 |Rat
}}
{{CreatureList|caption=Deep
 |Bat
}}
"""
PAGES = [
    wiki_page(11, "Test Cave", CAVE_TEXT),
    wiki_page(
        12,
        "Nowhere Hole",
        "{{Infobox Hunt\n| city = [[Unknown Land]]\n| location = Somewhere.\n"
        "| lvlknights = 8\n}}\n",
    ),
    wiki_page(
        13,
        "Double Trouble",
        "{{Infobox Hunt\n| city = test town\n"
        "| location = {{Mapper Coords|3.232|3.233|7|3|text=here}} or "
        "{{Mapper Coords|text=there|3.240|3.233|7|3}}\n| lvlmages = 20\n}}\n",
    ),
    wiki_page(
        14,
        "Far Away",
        "{{Infobox Hunt\n| city = Test Town\n"
        "| location = {{Mapper Coords|30.0|3.233|7|3}}\n}}\n",
    ),
    wiki_page(15, "Overview", "Just a list page."),
    wiki_page(16, "Skills Only", "{{Infobox Hunt Skills\n| areaname = X\n}}"),
]


class SnapshotParseTest(unittest.TestCase):
    def test_raw_facts_of_a_synthetic_infobox(self):
        facts = snapshot_tool.raw_facts(CAVE_TEXT)
        self.assertEqual(
            facts,
            {
                "city": "[[Test Town]]",
                "creatures": ["Rat", "Cave_Rat", "Rat", "Bat"],
                "location_coordinates": ["3.232|3.233|7|3|text=here"],
                "lvlknights": "50",
                "lvlmages": "600?",
                "lvlpaladins": "?",
            },
        )

    def test_snapshot_stores_hashes_and_facts_never_prose(self):
        snapshot = snapshot_tool.build_snapshot(PAGES, "2026-09-29")
        self.assertEqual(
            [row["title"] for row in snapshot["pages"]],
            ["Test Cave", "Nowhere Hole", "Double Trouble", "Far Away"],
        )
        self.assertEqual(
            [row["title"] for row in snapshot["pages_without_hunt_infobox"]],
            ["Overview", "Skills Only"],
        )
        self.assertNotIn("Prose that must never", json.dumps(snapshot))
        self.assertEqual(
            snapshot["pages"][0]["sha256"],
            snapshot_tool.hashlib.sha256(CAVE_TEXT.encode()).hexdigest(),
        )
        self.assertEqual(
            (snapshot["pages"][0]["pageid"], snapshot["pages"][0]["revid"]), (11, 110)
        )

    def test_no_infobox_and_unbalanced_templates_yield_no_facts(self):
        self.assertIsNone(snapshot_tool.raw_facts("plain page"))
        self.assertIsNone(snapshot_tool.raw_facts("{{Infobox Hunt\n| city = X"))

    def test_second_coordinate_form_keeps_named_parameter_first(self):
        text = (
            "{{Infobox Hunt\n| location = {{Mapper Coords|text=here|3.1|3.2|7|3}}\n}}"
        )
        self.assertEqual(
            snapshot_tool.raw_facts(text)["location_coordinates"],
            ["text=here|3.1|3.2|7|3"],
        )


class HuntingParseTest(unittest.TestCase):
    def test_mapper_coords_convert_to_absolute_tiles(self):
        # Thais temple 32369,32241,7 in the same frame as the City Areas.
        self.assertEqual(
            hunting.parse_mapper_coords("126.113|125.241|7|3|text=here"),
            (32369, 32241, 7),
        )
        self.assertEqual(
            hunting.parse_mapper_coords("text=here|126.113|125.241|7|3|3"),
            (32369, 32241, 7),
        )

    def test_mapper_coords_reject_anything_not_exactly_that_form(self):
        for raw in (
            "126.113|125.241",
            "126.256|125.241|7",
            "126|125.241|7",
            "126.113|125.241|16",
            "126.113|125.241|x",
            "text=here",
            "",
        ):
            self.assertIsNone(hunting.parse_mapper_coords(raw), raw)

    def test_levels_are_plain_integers_only(self):
        self.assertEqual(hunting.parse_level(" 350 "), 350)
        for raw in (None, "", "?", "600?", "20-30", "0", "99999"):
            self.assertIsNone(hunting.parse_level(raw), raw)


class HuntingPlaceEndToEndTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
            "data-global/world/world-house.xml": HOUSE_XML,
        }
        self.write(convert.build(blobs, self.root))
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(convert.ITEM_BINDINGS, bindings)
        self.load(snapshot_tool.build_snapshot(PAGES, "2026-09-29"))

    def tearDown(self):
        shutil.rmtree(self.root)

    def write(self, files: dict[str, bytes]):
        for path, data in files.items():
            (self.root / path).parent.mkdir(parents=True, exist_ok=True)
            (self.root / path).write_bytes(data)

    def load(self, snapshot: dict):
        data = snapshot_tool.canonical(snapshot)
        self.write({hunting.SNAPSHOT: data})
        self.out = hunting.build(data, self.root)
        self.write(self.out)

    def shard_path(self):
        return (
            self.root
            / "content/world/areas/hunting-places/hunting-places-00000-00003.json"
        )

    def records(self):
        return {
            r["declaration"]["name"]: r
            for r in json.loads(self.shard_path().read_text())["records"]
        }

    def test_fixture_converts_to_valid_records(self):
        self.assertEqual(validate.validate(self.root), [])
        records = self.records()
        cave = records["Test Cave"]["declaration"]
        self.assertEqual(cave["identity"]["key"], "oteryn:area.hunting_place.test_cave")
        self.assertEqual(cave["city"]["key"], "oteryn:area.city.test_town")
        self.assertEqual(
            cave["position"],
            {
                "coordinate_frame": convert.COORDINATE_FRAME,
                "floor": 7,
                "x": 1000,
                "y": 1001,
            },
        )
        self.assertEqual(cave["recommended_levels"], {"knight": 50})
        self.assertEqual(
            cave["source_facts"],
            {"city_name": "Test Town", "creature_names": ["Bat", "Cave Rat", "Rat"]},
        )
        binding = records["Test Cave"]["source_bindings"][0]
        self.assertEqual(
            (
                binding["identity_namespace"],
                binding["external_id"],
                binding["source_revision"],
            ),
            ("tibiawiki-fandom/page-id", "11", "110"),
        )

    def test_unparsable_facts_are_omitted_and_counted(self):
        records = self.records()
        nowhere = records["Nowhere Hole"]["declaration"]
        self.assertNotIn("city", nowhere)
        self.assertEqual(nowhere["source_facts"], {"city_name": "Unknown Land"})
        self.assertNotIn("position", nowhere)
        self.assertNotIn("position", records["Double Trouble"]["declaration"])
        self.assertEqual(
            records["Double Trouble"]["declaration"]["city"]["key"],
            "oteryn:area.city.test_town",
        )
        self.assertNotIn("position", records["Far Away"]["declaration"])
        summary = json.loads(self.out[hunting.SUMMARY])
        self.assertEqual(summary["families"], {"Area.HuntingPlace": 4})
        self.assertEqual(
            summary["records_with"],
            {"city": 3, "creatures": 1, "levels": 3, "position": 1},
        )
        self.assertEqual(
            summary["not_imported"],
            {
                "city_absent": 0,
                "city_unmatched": 1,
                "creature_names_unparsed": 0,
                "levels_absent": 1,
                "pages_without_hunt_infobox": 2,
                "position_absent": 1,
                "position_ambiguous": 1,
                "position_unparsed": 1,
            },
        )

    def test_conversion_is_deterministic(self):
        self.assertEqual(
            hunting.build(
                snapshot_tool.canonical(
                    json.loads((self.root / hunting.SNAPSHOT).read_text())
                ),
                self.root,
            ),
            self.out,
        )

    def test_renamed_page_keeps_its_key_and_new_page_gets_a_slug(self):
        pages = [
            wiki_page(11, "Renamed Cave", CAVE_TEXT, 111),
            wiki_page(
                21, "Fresh Hole", PAGES[1]["revisions"][0]["slots"]["main"]["content"]
            ),
        ]
        self.load(snapshot_tool.build_snapshot(pages, "2026-10-01"))
        keys = {
            r["declaration"]["name"]: r["declaration"]["identity"]["key"]
            for r in json.loads(
                (
                    self.root
                    / "content/world/areas/hunting-places/hunting-places-00000-00001.json"
                ).read_text()
            )["records"]
        }
        self.assertEqual(
            keys,
            {
                "Renamed Cave": "oteryn:area.hunting_place.test_cave",
                "Fresh Hole": "oteryn:area.hunting_place.fresh_hole",
            },
        )

    def test_new_page_colliding_with_a_committed_key_fails_closed(self):
        pages = [*PAGES[:1], wiki_page(31, "Test Cave", CAVE_TEXT)]
        with self.assertRaises(convert.ConvertError):
            snapshot = snapshot_tool.canonical(
                snapshot_tool.build_snapshot(pages, "2026-10-01")
            )
            hunting.build(snapshot, self.root)

    def test_rejects_snapshot_with_repeated_or_foreign_shape(self):
        pages = [PAGES[0], wiki_page(11, "Twin", CAVE_TEXT)]
        with self.assertRaises(convert.ConvertError):
            hunting.build(
                snapshot_tool.canonical(
                    snapshot_tool.build_snapshot(pages, "2026-10-01")
                ),
                self.root,
            )
        with self.assertRaises(convert.ConvertError):
            hunting.build(b'{"schema":"other","pages":[]}\n', self.root)

    def rewrite(self, mutate):
        shard = json.loads(self.shard_path().read_text())
        mutate(shard)
        self.shard_path().write_text(validate.canonical(shard), encoding="utf-8")

    def errors(self):
        return validate.validate(self.root)

    def test_rejects_unknown_city_reference(self):
        def mutate(shard):
            for record in shard["records"]:
                if "city" in record["declaration"]:
                    record["declaration"]["city"]["key"] = "oteryn:area.city.nowhere"

        self.rewrite(mutate)
        self.assertTrue(any("is not a City Area" in e for e in self.errors()))

    def test_rejects_position_outside_the_map_extent(self):
        def mutate(shard):
            shard["records"][0]["declaration"]["position"] = {
                "coordinate_frame": convert.COORDINATE_FRAME,
                "floor": 7,
                "x": 5000,
                "y": 5,
            }

        self.rewrite(mutate)
        self.assertTrue(
            any("outside the source map extent" in e for e in self.errors())
        )

    def test_rejects_unsorted_keys(self):
        self.rewrite(lambda shard: shard["records"].reverse())
        self.assertTrue(any("unique and sorted" in e for e in self.errors()))

    def test_rejects_page_revision_outside_the_pinned_snapshot(self):
        def mutate(shard):
            shard["records"][0]["source_bindings"][0]["source_revision"] = "999"

        self.rewrite(mutate)
        self.assertTrue(any("not in the pinned snapshot" in e for e in self.errors()))

    def test_rejects_counts_that_differ_from_the_records(self):
        def mutate(shard):
            for record in shard["records"]:
                record["declaration"].pop("position", None)

        self.rewrite(mutate)
        self.assertTrue(any("capture counts differ" in e for e in self.errors()))

    def test_rejects_edited_snapshot_stray_file_and_non_canonical_bytes(self):
        path = self.root / hunting.SNAPSHOT
        path.write_text(path.read_text().replace("Test Cave", "Test Cove"))
        (self.root / "content/world/areas/hunting-places/extra.json").write_text("{}\n")
        index = self.root / "content/world/areas/hunting-places/index.json"
        index.write_text(index.read_text() + " ")
        errors = self.errors()
        self.assertTrue(any("differs from the sha256 pinned" in e for e in errors))
        self.assertTrue(any("files other than" in e for e in errors))
        self.assertTrue(any("not canonical" in e for e in errors))

    def test_rejects_schema_violation_in_a_record(self):
        def mutate(shard):
            shard["records"][0]["declaration"]["recommended_levels"] = {"knight": 0}

        self.rewrite(mutate)
        self.assertTrue(any("schema" in e for e in self.errors()))


class CommittedContentTest(unittest.TestCase):
    def test_committed_families_validate(self):
        self.assertEqual(validate.validate(convert.ROOT), [])


if __name__ == "__main__":
    unittest.main()
