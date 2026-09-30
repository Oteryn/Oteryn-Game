"""Fixture tests for the OTBM reader, the metadata converter and the validator."""

from __future__ import annotations

import gzip
import hashlib
import json
import lzma
import shutil
import struct
import tempfile
import unittest
from pathlib import Path

import client_map_reader as client_map
import convert_city_facts as cityfacts
import convert_hunting_places as hunting
import convert_map_regions as regions
import convert_world_metadata as convert
import fandom_city_snapshot as city_snapshot
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


def fixture_map(extra_houses=()) -> bytes:
    """One town, one house (id 0xFE forces escaping) with a door, two teleports.

    `extra_houses` adds one plain tile per additional house id.
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
    map_data = node(2, bytes([1]) + string("fixture"), area, towns, waypoints)
    return b"\x00\x00\x00\x00" + node(
        0, struct.pack("<IHHII", 4, 2048, 2048, 3, 57), map_data
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
        add_regions(self.root)
        add_cities(self.root)

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
            summary["families"], {"Area.City": 1, "Transition.Teleport": 1}
        )
        self.assertEqual(summary["not_imported"]["teleports"]["unset_destination"], 1)
        teleport = self.shard("Transition.Teleport")["records"][0]["declaration"]
        self.assertEqual(
            teleport["identity"]["key"], "oteryn:transition.teleport.x1001_y1003_z7"
        )
        self.assertEqual(teleport["object"]["key"], convert.item_keys()[TELEPORT_ITEM])

    def test_conversion_is_deterministic(self):
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
        }
        with tempfile.TemporaryDirectory() as empty:
            self.assertEqual(convert.build(blobs, Path(empty)), self.out)

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
        }
        self.write(convert.build(blobs, self.root))
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(convert.ITEM_BINDINGS, bindings)
        self.load(snapshot_tool.build_snapshot(PAGES, "2026-09-29"))
        add_regions(self.root)
        add_cities(self.root)

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


def pb_varint(value: int) -> bytes:
    out = bytearray()
    while value >= 0x80:
        out.append(value & 0x7F | 0x80)
        value >>= 7
    return bytes(out + bytes([value]))


def pb_field(number: int, value) -> bytes:
    if isinstance(value, int):
        return pb_varint(number << 3) + pb_varint(value)
    if isinstance(value, str):
        value = value.encode()
    return pb_varint(number << 3 | 2) + pb_varint(len(value)) + value


def pb_position(x: int, y: int, z: int) -> bytes:
    return pb_field(1, x) + pb_field(2, y) + pb_field(3, z)


def pb_area(area_id, name, kind, children=(), anchor=None, flag=None, secondary=()):
    body = pb_field(1, area_id) + pb_field(2, name) + pb_field(3, kind)
    body += b"".join(pb_field(4, child) for child in children)
    if anchor:
        body += pb_field(5, pb_position(*anchor))
    if flag is not None:
        body += pb_field(6, flag)
    body += b"".join(pb_field(7, name) for name in secondary)
    return pb_field(1, body)


def bmp32(rows) -> bytes:
    """A bottom-up 32-bit BMP of `rows` (top row first; 1 = opaque white, 0 = transparent)."""
    height, width = len(rows), len(rows[0])
    pixels = b"".join(
        b"".join(b"\xff\xff\xff\xff" if cell else b"\x00\x00\x00\x00" for cell in row)
        for row in reversed(rows)
    )
    header = b"BM" + struct.pack("<IHHI", 54 + len(pixels), 0, 0, 54)
    header += struct.pack(
        "<IiiHHIIiiII", 40, width, height, 1, 32, 0, len(pixels), 0, 0, 0, 0
    )
    return header + pixels


def frame(bmp: bytes) -> bytes:
    """Tibia framing (zeros, 70 0A, two varints) around an LZMA-alone stream."""
    return (
        b"\x00" * 25
        + b"\x70\x0a\xfa\xad\x02\x98\x06"
        + lzma.compress(bmp, format=lzma.FORMAT_ALONE)
    )


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


TOWN_ROWS = [[0] * 8] + [[0, *([1] * 6), 0] for _ in range(4)] + [[0] * 8]  # 6 rows
TOWN_BMP = bmp32(TOWN_ROWS)
TOWN_FILE = f"subarea-0002-{sha(TOWN_BMP)}.bmp.lzma"
REGION_AREAS = [
    pb_area(1, "Test Land", 1, [2, 3], (1000, 1000, 7), 0),
    pb_area(2, "Test Town", 2, [], (1001, 1000, 7)),
    pb_area(3, "Empty Reach", 2, [], None, None, ["Old Name"]),
    pb_area(4, "Test Town", 1, [2]),
]


def pb_image(kind, position, file, width, height, tail=b""):
    body = pb_field(1, kind) + pb_field(2, pb_position(*position))
    return pb_field(
        3, body + pb_field(3, file) + pb_field(4, width) + pb_field(5, height) + tail
    )


def client_files(areas=REGION_AREAS, images=None, town=TOWN_BMP) -> dict[str, bytes]:
    """A synthetic client map, its subarea mask, the catalog and the checksum manifest."""
    if images is None:
        images = [
            pb_image(0, (998, 998, 7), TOWN_FILE, 8, 6, pb_field(6, 2)),
            pb_image(
                1,
                (768, 768, 7),
                "satellite-16-0003-0003-07-x.bmp.lzma",
                256,
                256,
                pb_varint(7 << 3 | 1) + bytes(8),
            ),
        ]
    data = b"".join(areas) + b"".join(images)
    data += pb_field(2, pb_field(1, "A Marker") + pb_field(2, pb_position(1, 1, 7)))
    data += pb_field(4, pb_position(900, 900, 0)) + pb_field(
        5, pb_position(1100, 1100, 7)
    )
    map_name = f"map-{sha(data)}.dat"
    town = frame(town)
    files = {
        f"{regions.FILES}/{map_name}": data,
        f"{regions.FILES}/{TOWN_FILE}": town,
    }
    manifest = {
        "archive_sha256": "0" * 64,
        "files": [
            {"bytes": len(blob), "name": path.rsplit("/", 1)[-1], "sha256": sha(blob)}
            for path, blob in files.items()
        ],
        "schema": "OTERYN_CLIENT_ASSET_MANIFEST/v1",
    }
    files[regions.MANIFEST] = json.dumps(manifest).encode()
    files[regions.CATALOG] = json.dumps([{"type": "map", "file": map_name}]).encode()
    return files


def write_files(root: Path, files: dict[str, bytes]) -> None:
    for path, data in files.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)


def add_regions(root: Path) -> None:
    write_files(root, client_files())
    write_files(root, regions.build(root))


CITY_TEXT = """{{Infobox Geography
| ruler        = [[King Test]] <!-- ignored -->
| implemented  = 6.2
| population   = {{PAGESINCATEGORY:Test Town NPCs|pages}}
| near         = [[Hill]], [[Lake|the Lake]]
| map          = [[File:Map_test.jpg]]
}}
Prose that must never be stored.
"""
CITY_NPCS = ["Ann Bee", "Carl D'Oh", "Zed", "Zed"]


def city_snapshot_data(text: str = CITY_TEXT, npcs=CITY_NPCS) -> bytes:
    pages = {"Test Town": wiki_page(21, "Test Town", text)}
    return snapshot_tool.canonical(
        city_snapshot.build_snapshot(
            ["Test Town"], pages, {"Test Town": list(npcs)}, "2026-09-29"
        )
    )


def add_cities(root: Path, data: bytes | None = None) -> None:
    data = data or city_snapshot_data()
    keys = ("ann_bee", "zed", "other")
    shard = "content/npcs/definitions/npcs-00000-00002.json"
    write_files(
        root,
        {
            cityfacts.NPC_INDEX: validate.canonical({"shards": [shard]}).encode(),
            shard: validate.canonical(
                {
                    "records": [
                        {"declaration": {"identity": {"key": f"oteryn:npc.{k}"}}}
                        for k in keys
                    ]
                }
            ).encode(),
            cityfacts.SNAPSHOT: data,
        },
    )
    write_files(root, cityfacts.build(data, root))


class ClientMapReaderTest(unittest.TestCase):
    def test_reads_areas_images_and_counts_the_rest(self):
        files = client_files()
        data = next(v for k, v in files.items() if k.endswith(".dat"))
        facts = client_map.read_map(data)
        self.assertEqual(
            [(a.id, a.name, a.kind, a.children) for a in facts.areas],
            [
                (1, "Test Land", 1, [2, 3]),
                (2, "Test Town", 2, []),
                (3, "Empty Reach", 2, []),
                (4, "Test Town", 1, [2]),
            ],
        )
        self.assertEqual(facts.areas[0].anchor, (1000, 1000, 7))
        self.assertEqual(facts.areas[2].secondary_names, ["Old Name"])
        self.assertEqual((facts.markers, facts.other_images), (1, {"satellite": 1}))
        self.assertEqual(
            (facts.minimum, facts.maximum), ((900, 900, 0), (1100, 1100, 7))
        )
        image = facts.subareas[0]
        mask = client_map.read_mask(files[f"{regions.FILES}/{TOWN_FILE}"], image)
        self.assertEqual(mask.bounds(), (999, 999, 1004, 1002))
        self.assertEqual(mask.tile_count, 24)
        self.assertTrue(mask.contains(1001, 1001, 7))
        self.assertFalse(mask.contains(1001, 1001, 6))
        self.assertFalse(mask.contains(998, 998, 7))

    def test_rejects_unknown_fields_wire_types_and_truncation(self):
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_map(pb_field(9, 1))
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_map(pb_field(1, pb_field(1, 1) + pb_field(8, 1)))
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_map(b"\x0a\x05abc")
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_map(b"\x0f")
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_map(pb_field(3, pb_field(1, 5)))

    def test_rejects_foreign_framing_and_mismatched_masks(self):
        image = client_map.SubareaImage(2, 998, 998, 7, 8, 6, TOWN_FILE)
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_mask(b"\x00" * 30, image)
        wrong_size = client_map.SubareaImage(2, 998, 998, 7, 9, 6, TOWN_FILE)
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_mask(frame(TOWN_BMP), wrong_size)
        renamed = client_map.SubareaImage(
            2, 998, 998, 7, 8, 6, "subarea-0002-0.bmp.lzma"
        )
        with self.assertRaises(client_map.ClientMapError):
            client_map.read_mask(frame(TOWN_BMP), renamed)


class MapRegionEndToEndTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
        }
        write_files(self.root, convert.build(blobs, self.root))
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(convert.ITEM_BINDINGS, bindings)
        data = snapshot_tool.canonical(
            snapshot_tool.build_snapshot(PAGES, "2026-09-29")
        )
        write_files(self.root, {hunting.SNAPSHOT: data})
        write_files(self.root, hunting.build(data, self.root))
        add_cities(self.root)
        self.load(client_files())

    def tearDown(self):
        shutil.rmtree(self.root)

    def load(self, files):
        write_files(self.root, files)
        self.out = regions.build(self.root)
        shutil.rmtree(self.root / "content/world/areas/regions", ignore_errors=True)
        write_files(self.root, self.out)

    def shard_path(self):
        index = json.loads(
            (self.root / "content/world/areas/regions/index.json").read_text()
        )
        return self.root / index["shards"][0]

    def records(self):
        return {
            r["declaration"]["identity"]["key"]: r["declaration"]
            for r in json.loads(self.shard_path().read_text())["records"]
        }

    def rewrite(self, mutate):
        shard = json.loads(self.shard_path().read_text())
        mutate(shard)
        self.shard_path().write_text(validate.canonical(shard), encoding="utf-8")

    def errors(self):
        return validate.validate(self.root)

    def test_fixture_converts_to_valid_records(self):
        self.assertEqual(self.errors(), [])
        records = self.records()
        self.assertEqual(
            sorted(records),
            [
                "oteryn:area.region.empty_reach",
                "oteryn:area.region.test_land",
                "oteryn:area.region.test_town",
                "oteryn:area.region.test_town_subregion",
            ],
        )
        town = records["oteryn:area.region.test_town_subregion"]
        self.assertEqual(town["area_kind"], "subregion")
        self.assertEqual(
            [p["key"] for p in town["parent_regions"]],
            ["oteryn:area.region.test_land", "oteryn:area.region.test_town"],
        )
        footprint = town["footprint"]
        self.assertEqual(
            (
                footprint["min_x"],
                footprint["min_y"],
                footprint["max_x"],
                footprint["max_y"],
            ),
            (999, 999, 1004, 1002),
        )
        self.assertEqual((footprint["floor"], footprint["tile_count"]), (7, 24))
        self.assertEqual(footprint["image"]["path"], f"{regions.FILES}/{TOWN_FILE}")
        self.assertEqual(
            town["anchor"],
            {
                "coordinate_frame": convert.COORDINATE_FRAME,
                "floor": 7,
                "x": 1001,
                "y": 1000,
            },
        )
        city = [
            {
                "family": "Area",
                "key": "oteryn:area.city.test_town",
                "revision": "definition-r1",
            }
        ]
        self.assertEqual(town["cities"], city)
        for key in ("oteryn:area.region.test_land", "oteryn:area.region.test_town"):
            self.assertEqual(records[key]["area_kind"], "region")
            self.assertEqual(records[key]["cities"], city)
            self.assertNotIn("footprint", records[key])
        empty = records["oteryn:area.region.empty_reach"]
        self.assertEqual({"anchor", "cities", "footprint"} & set(empty), set())
        binding = json.loads(self.shard_path().read_text())["records"][0][
            "source_bindings"
        ][0]
        self.assertEqual(
            (binding["identity_namespace"], binding["external_id"]),
            ("tibia-client/map-area-id", "3"),
        )
        summary = json.loads(self.out[regions.SUMMARY])
        self.assertEqual(summary["records_by_kind"], {"region": 2, "subregion": 2})
        self.assertEqual(
            summary["records_with"],
            {"anchor": 2, "cities": 3, "footprint": 1, "parent_regions": 2},
        )
        self.assertEqual(
            summary["not_imported"],
            {
                "area_flag_field": 1,
                "areas_with_secondary_names": 1,
                "cities_outside_every_footprint": 0,
                "cities_with_temple_floor_absent_from_footprints": 0,
                "map_markers": 1,
                "minimap_images": 0,
                "satellite_images": 1,
                "subregions_without_footprint": 1,
            },
        )
        self.assertEqual(
            summary["cities"],
            {"linked": 1, "projected": [], "total": 1, "unlinked": []},
        )

    def test_conversion_is_deterministic(self):
        self.assertEqual(regions.build(self.root), self.out)

    def test_renamed_area_keeps_its_key_and_new_area_gets_a_slug(self):
        areas = list(REGION_AREAS)
        areas[2] = pb_area(3, "Renamed Reach", 2, [], None, None, ["Old Name"])
        areas.append(pb_area(5, "Fresh Place", 2, []))
        areas[0] = pb_area(1, "Test Land", 1, [2, 3, 5], (1000, 1000, 7), 0)
        self.load(client_files(areas))
        records = self.records()
        self.assertEqual(
            records["oteryn:area.region.empty_reach"]["name"], "Renamed Reach"
        )
        self.assertIn("oteryn:area.region.fresh_place", records)
        self.assertEqual(self.errors(), [])

    def test_new_area_colliding_with_a_committed_key_fails_closed(self):
        areas = list(REGION_AREAS)
        areas[0] = pb_area(1, "Test Land", 1, [2, 3, 5], (1000, 1000, 7), 0)
        areas.append(pb_area(5, "Test Town Subregion", 2, []))
        with self.assertRaises(convert.ConvertError):
            self.load(client_files(areas))

    def test_two_regions_with_one_name_fail_closed(self):
        areas = [*REGION_AREAS, pb_area(5, "Test Land", 1, [3])]
        with self.assertRaises(convert.ConvertError):
            self.load(client_files(areas))

    def test_rejects_broken_hierarchies(self):
        cases = {
            "orphan subarea": [*REGION_AREAS, pb_area(5, "Lost", 2, [])],
            "kind without children": [*REGION_AREAS, pb_area(5, "Empty", 1, [])],
            "child that is a region": [
                pb_area(1, "Test Land", 1, [2, 3, 4], None),
                *REGION_AREAS[1:],
            ],
            "unknown child": [pb_area(1, "Test Land", 1, [2, 3, 9]), *REGION_AREAS[1:]],
            "repeated id": [*REGION_AREAS, pb_area(2, "Again", 2, [])],
            "unknown kind": [*REGION_AREAS, pb_area(5, "Odd", 3, [])],
            "repeated child": [
                pb_area(1, "Test Land", 1, [2, 2, 3]),
                *REGION_AREAS[1:],
            ],
        }
        for label, areas in cases.items():
            with self.subTest(label), self.assertRaises(convert.ConvertError):
                self.load(client_files(areas))

    def test_rejects_images_that_do_not_match_the_map_entry_or_manifest(self):
        with self.assertRaises(client_map.ClientMapError):
            self.load(client_files(town=bmp32([[1] * 8] * 5)))
        files = client_files()
        manifest = json.loads(files[regions.MANIFEST])
        manifest["files"][1]["sha256"] = "1" * 64
        files[regions.MANIFEST] = json.dumps(manifest).encode()
        with self.assertRaises(convert.ConvertError):
            self.load(files)

    def test_rejects_mask_outside_the_map_extent(self):
        image = pb_image(0, (2047, 998, 7), TOWN_FILE, 8, 6, pb_field(6, 2))
        with self.assertRaises(convert.ConvertError):
            self.load(client_files(images=[image]))

    def test_city_on_another_floor_or_outside_the_mask_is_not_linked(self):
        image = pb_image(0, (998, 998, 6), TOWN_FILE, 8, 6, pb_field(6, 2))
        self.load(client_files(images=[image]))
        summary = json.loads(self.out[regions.SUMMARY])
        self.assertEqual(summary["cities"]["linked"], 0)
        self.assertEqual(
            summary["not_imported"]["cities_with_temple_floor_absent_from_footprints"],
            1,
        )
        image = pb_image(0, (1003, 998, 7), TOWN_FILE, 8, 6, pb_field(6, 2))
        self.load(client_files(images=[image]))
        summary = json.loads(self.out[regions.SUMMARY])
        self.assertEqual(summary["not_imported"]["cities_outside_every_footprint"], 1)
        self.assertEqual(self.errors(), [])

    def move_temple(self, **fields):
        path = self.root / "content/world/areas/cities/cities-00000-00000.json"
        shard = json.loads(path.read_text())
        shard["records"][0]["declaration"]["temple"].update(fields)
        path.write_text(validate.canonical(shard), encoding="utf-8")
        self.load(client_files())
        return json.loads(self.out[regions.SUMMARY])["cities"]

    def test_temple_off_floor_7_links_when_its_projection_hits_one_mask(self):
        cities = self.move_temple(floor=5)
        self.assertEqual(cities["linked"], 1)
        self.assertEqual(cities["unlinked"], [])
        self.assertEqual(
            cities["projected"],
            [
                {
                    "city": "oteryn:area.city.test_town",
                    "method": "temple_projected_to_floor_7",
                    "subregion": "oteryn:area.region.test_town_subregion",
                }
            ],
        )
        self.assertEqual(self.errors(), [])
        summary = json.loads(self.out[regions.SUMMARY])
        summary["cities"]["projected"] = []
        write_files(self.root, {regions.SUMMARY: json.dumps(summary).encode()})
        self.assertTrue(any("projection" in e for e in self.errors()))

    def test_temple_off_floor_7_outside_every_mask_stays_unlinked(self):
        cities = self.move_temple(floor=5, x=1010)
        self.assertEqual(cities["linked"], 0)
        self.assertEqual(cities["projected"], [])
        self.assertEqual(
            cities["unlinked"],
            [{"city": "oteryn:area.city.test_town", "reason": "projection_outside"}],
        )
        self.assertEqual(self.errors(), [])

    def test_floor_7_temple_outside_the_mask_records_nearest_mask_and_distance(self):
        cities = self.move_temple(x=1010)
        self.assertEqual(cities["linked"], 0)
        self.assertEqual(
            cities["unlinked"],
            [
                {
                    "city": "oteryn:area.city.test_town",
                    "distance": 6,
                    "nearest_mask": "oteryn:area.region.test_town_subregion",
                    "reason": "outside_every_mask",
                }
            ],
        )
        self.assertEqual(self.errors(), [])

    def test_rejects_unknown_parent_and_parent_that_is_not_a_region(self):
        def mutate(shard):
            for record in shard["records"]:
                if "parent_regions" in record["declaration"]:
                    record["declaration"]["parent_regions"][0]["key"] = (
                        "oteryn:area.region.empty_reach"
                    )

        self.rewrite(mutate)
        self.assertTrue(any("is not a region" in e for e in self.errors()))

    def test_rejects_wrong_city_links(self):
        def mutate(shard):
            for record in shard["records"]:
                declaration = record["declaration"]
                if declaration["area_kind"] == "region":
                    declaration.pop("cities", None)
                elif "cities" in declaration:
                    declaration["cities"][0]["key"] = "oteryn:area.city.nowhere"

        self.rewrite(mutate)
        errors = self.errors()
        self.assertTrue(any("is not a City Area" in e for e in errors))
        self.assertTrue(
            any("differ from the cities of its subregions" in e for e in errors)
        )

    def test_rejects_footprint_that_excludes_the_city_temple_or_leaves_the_map(self):
        def mutate(shard):
            for record in shard["records"]:
                footprint = record["declaration"].get("footprint")
                if footprint:
                    footprint["min_x"], footprint["max_x"] = 1002, 5000
                    footprint["tile_count"] = 10**9

        self.rewrite(mutate)
        errors = self.errors()
        self.assertTrue(any("temple is outside the footprint" in e for e in errors))
        self.assertTrue(any("outside the source map extent" in e for e in errors))
        self.assertTrue(any("does not fit its bounding box" in e for e in errors))

    def test_rejects_edited_image_wrong_counts_and_stray_files(self):
        image = self.root / f"{regions.FILES}/{TOWN_FILE}"
        image.write_bytes(image.read_bytes() + b"x")
        (self.root / "content/world/areas/regions/extra.json").write_text("{}\n")
        self.rewrite(
            lambda shard: [
                r["declaration"].pop("anchor", None) for r in shard["records"]
            ]
        )
        errors = self.errors()
        self.assertTrue(any("footprint image differs" in e for e in errors))
        self.assertTrue(any("files other than" in e for e in errors))
        self.assertTrue(any("capture counts differ" in e for e in errors))

    def test_rejects_binding_revision_and_area_id_problems(self):
        def mutate(shard):
            shard["records"][0]["source_bindings"][0]["source_revision"] = "2" * 64
            shard["records"][1]["source_bindings"][0]["external_id"] = "3"

        self.rewrite(mutate)
        errors = self.errors()
        self.assertTrue(any("differs from the pinned map file" in e for e in errors))
        self.assertTrue(any("is bound twice" in e for e in errors))

    def test_rejects_unknown_area_kind(self):
        def mutate(shard):
            shard["records"][0]["declaration"]["area_kind"] = "district"

        self.rewrite(mutate)
        self.assertTrue(any("schema" in e for e in self.errors()))

    def test_rejects_subregion_without_parents_and_region_with_footprint(self):
        def mutate(shard):
            for record in shard["records"]:
                declaration = record["declaration"]
                if declaration["area_kind"] == "subregion":
                    declaration.pop("parent_regions", None)
                else:
                    declaration["footprint"] = {"x": 1}

        self.rewrite(mutate)
        self.assertTrue(any("schema" in e for e in self.errors()))


class CityFactsTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
        }
        write_files(self.root, convert.build(blobs, self.root))
        self.plain = (self.root / "content/world/areas/cities/index.json").read_bytes()
        bindings = self.root / validate.ITEM_BINDINGS
        bindings.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(convert.ITEM_BINDINGS, bindings)
        data = snapshot_tool.canonical(
            snapshot_tool.build_snapshot(PAGES, "2026-09-29")
        )
        write_files(self.root, {hunting.SNAPSHOT: data})
        write_files(self.root, hunting.build(data, self.root))
        add_regions(self.root)
        add_cities(self.root)

    def tearDown(self):
        shutil.rmtree(self.root)

    def shard_path(self):
        return self.root / "content/world/areas/cities/cities-00000-00000.json"

    def record(self):
        return json.loads(self.shard_path().read_text())["records"][0]

    def rewrite(self, mutate):
        shard = json.loads(self.shard_path().read_text())
        mutate(shard)
        self.shard_path().write_text(validate.canonical(shard), encoding="utf-8")

    def errors(self):
        return validate.validate(self.root)

    def test_snapshot_keeps_raw_facts_and_npc_names_never_prose(self):
        data = city_snapshot_data()
        self.assertNotIn(b"Prose", data)
        self.assertNotIn(b"population", data)
        row = json.loads(data)["pages"][0]
        self.assertEqual(
            row["facts"],
            {
                "implemented": "6.2",
                "near": "[[Hill]], [[Lake|the Lake]]",
                "npc_names": ["Ann Bee", "Carl D'Oh", "Zed"],
                "ruler": "[[King Test]]",
            },
        )
        self.assertEqual(row["sha256"], hashlib.sha256(CITY_TEXT.encode()).hexdigest())

    def test_snapshot_lists_missing_ambiguous_and_non_city_pages_apart(self):
        pages = {
            "Test Town": wiki_page(21, "Test Town", CITY_TEXT),
            "Cave": wiki_page(22, "Cave", "{{Infobox Hunt\n| city = X\n}}"),
            "Targuna": wiki_page(23, "Targuna", CITY_TEXT),
        }
        names = ["Test Town", "Cave", "Targuna", "Home"]
        snapshot = city_snapshot.build_snapshot(names, pages, {}, "2026-09-29")
        self.assertEqual([row["title"] for row in snapshot["pages"]], ["Test Town"])
        self.assertEqual(
            {row["name"]: row["reason"] for row in snapshot["unmatched"]},
            {
                "Cave": city_snapshot.NO_INFOBOX,
                "Home": city_snapshot.NO_PAGE,
                "Targuna": city_snapshot.AMBIGUOUS["Targuna"],
            },
        )

    def test_fixture_enriches_the_city_and_keeps_its_original_fields(self):
        self.assertEqual(self.errors(), [])
        record = self.record()
        declaration = record["declaration"]
        self.assertEqual(declaration["implemented"], "6.2")
        self.assertEqual(
            [ref["key"] for ref in declaration["npcs"]],
            ["oteryn:npc.ann_bee", "oteryn:npc.zed"],
        )
        self.assertEqual(
            declaration["source_facts"],
            {
                "implemented": "6.2",
                "near": "Hill, the Lake",
                "npc_names_unmatched": ["Carl D'Oh"],
                "ruler": "King Test",
            },
        )
        self.assertEqual(declaration["identity"]["key"], "oteryn:area.city.test_town")
        crystal, wiki = record["source_bindings"]
        self.assertEqual(crystal["identity_namespace"], "crystalserver/town-id")
        self.assertEqual(
            (wiki["identity_namespace"], wiki["external_id"], wiki["source_revision"]),
            ("tibiawiki-fandom/page-id", "21", "210"),
        )
        summary = json.loads((self.root / cityfacts.SUMMARY).read_text())
        self.assertEqual(
            summary["records_with"],
            {"implemented": 1, "npcs": 1, "source_facts": 1, "wiki_binding": 1},
        )
        self.assertEqual(summary["npc_names_linked"], 2)

    def test_world_metadata_regeneration_keeps_the_city_enrichment(self):
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
        }
        out = convert.build(blobs, self.root)
        self.assertIn(cityfacts.SUMMARY, out)
        for path, data in out.items():
            if path.startswith("content/world/areas/cities/") or (
                path == cityfacts.SUMMARY
            ):
                self.assertEqual((self.root / path).read_bytes(), data, path)

    def test_unparsable_version_stays_raw_only(self):
        add_cities(
            self.root,
            city_snapshot_data(CITY_TEXT.replace("6.2", "13.40.54ea79")),
        )
        declaration = self.record()["declaration"]
        self.assertNotIn("implemented", declaration)
        self.assertEqual(declaration["source_facts"]["implemented"], "13.40.54ea79")
        self.assertEqual(self.errors(), [])

    def test_conversion_is_idempotent_and_survives_plain_regeneration(self):
        data = (self.root / cityfacts.SNAPSHOT).read_bytes()
        first = cityfacts.build(data, self.root)
        write_files(self.root, first)
        self.assertEqual(cityfacts.build(data, self.root), first)
        blobs = {
            "data-global/world/world.otbm": fixture_map(),
        }
        write_files(self.root, convert.build(blobs, self.root))
        self.assertEqual(cityfacts.build(data, self.root), first)

    def test_rejects_snapshots_that_do_not_fit_the_cities(self):
        pages = {"Other City": wiki_page(24, "Other City", CITY_TEXT)}
        foreign = city_snapshot.build_snapshot(["Other City"], pages, {}, "2026-09-29")
        with self.assertRaises(convert.ConvertError):
            cityfacts.build(snapshot_tool.canonical(foreign), self.root)
        with self.assertRaises(convert.ConvertError):
            cityfacts.build(
                b'{"schema":"other","pages":[],"unmatched":[]}\n', self.root
            )
        twice = json.loads(city_snapshot_data())
        twice["unmatched"] = [{"name": "Test Town", "reason": "x"}]
        with self.assertRaises(convert.ConvertError):
            cityfacts.build(snapshot_tool.canonical(twice), self.root)

    def test_rejects_unknown_npc_key(self):
        def mutate(shard):
            shard["records"][0]["declaration"]["npcs"][0]["key"] = "oteryn:npc.nobody"

        self.rewrite(mutate)
        self.assertTrue(any("is not an NPC definition" in e for e in self.errors()))

    def test_rejects_stale_revision(self):
        def mutate(shard):
            shard["records"][0]["source_bindings"][1]["source_revision"] = "999"

        self.rewrite(mutate)
        self.assertTrue(any("not in the pinned snapshot" in e for e in self.errors()))

    def test_rejects_wiki_facts_without_a_binding(self):
        def mutate(shard):
            del shard["records"][0]["source_bindings"][1]

        self.rewrite(mutate)
        errors = self.errors()
        self.assertTrue(any("wiki facts without a wiki binding" in e for e in errors))
        self.assertTrue(any("exactly once" in e for e in errors))

    def test_rejects_counts_and_npc_coverage_that_differ(self):
        def mutate(shard):
            declaration = shard["records"][0]["declaration"]
            declaration["npcs"].pop()
            declaration["implemented"] = "6.3"

        self.rewrite(mutate)
        errors = self.errors()
        self.assertTrue(any("do not cover the snapshot" in e for e in errors))
        self.assertTrue(any("implemented differs" in e for e in errors))

    def test_rejects_non_canonical_bytes_and_edited_snapshot(self):
        path = self.root / cityfacts.SNAPSHOT
        path.write_text(path.read_text() + " ")
        index = self.root / "content/world/areas/cities/index.json"
        index.write_text(index.read_text() + " ")
        errors = self.errors()
        self.assertTrue(any("differs from the sha256 pinned" in e for e in errors))
        self.assertTrue(any("not canonical" in e for e in errors))

    def test_rejects_enrichment_pin_that_differs_from_the_capture(self):
        index_path = self.root / "content/world/areas/cities/index.json"
        index = json.loads(index_path.read_text())
        index["enrichment"]["fetched_at"] = "2026-01-01"
        index_path.write_text(validate.canonical(index), encoding="utf-8")
        self.assertTrue(any("enrichment differs" in e for e in self.errors()))

    def test_rejects_schema_violation_in_a_city(self):
        def mutate(shard):
            shard["records"][0]["declaration"]["source_facts"]["extra"] = "x"

        self.rewrite(mutate)
        self.assertTrue(any("schema" in e for e in self.errors()))

    def test_plain_cities_fail_validation_until_enriched(self):
        (self.root / "content/world/areas/cities/index.json").write_bytes(self.plain)
        self.assertTrue(any("enrichment differs" in e for e in self.errors()))


class CommittedCitiesTest(unittest.TestCase):
    """The committed cities equal the conversion of the committed wiki snapshot."""

    def test_committed_files_are_current(self):
        data = (convert.ROOT / cityfacts.SNAPSHOT).read_bytes()
        for path, blob in cityfacts.build(data).items():
            self.assertEqual((convert.ROOT / path).read_bytes(), blob, path)


class CommittedRegionsTest(unittest.TestCase):
    """The committed regions equal the conversion of the committed official client files."""

    @classmethod
    def setUpClass(cls):
        cls.out = regions.build(convert.ROOT)

    def test_committed_files_are_current(self):
        for path, data in self.out.items():
            self.assertEqual((convert.ROOT / path).read_bytes(), data, path)

    def test_known_cities_lie_in_their_named_footprints(self):
        shard = json.loads(
            next(data for path, data in self.out.items() if "regions-00000" in path)
        )
        records = {r["declaration"]["name"]: r["declaration"] for r in shard["records"]}
        for name, city in (
            ("Thais City", "oteryn:area.city.thais"),
            ("Ab'Dendriel City", "oteryn:area.city.ab_dendriel"),
        ):
            self.assertIn(city, [c["key"] for c in records[name]["cities"]])


class CommittedContentTest(unittest.TestCase):
    def test_committed_families_validate(self):
        self.assertEqual(validate.validate(convert.ROOT), [])


if __name__ == "__main__":
    unittest.main()
