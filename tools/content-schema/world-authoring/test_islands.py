"""Tests for the island snapshot reader, the island converter and the island validator.

The map is a small synthetic grid written as real region files, so the whole path is
covered: region decode, ground classes, component search, the cap, city containment, keys,
the family files and the validator.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import convert_islands as convert
import fandom_island_snapshot as snapshot_tool
import validate_islands as validate
import world_region_codec as codec
from convert_world_metadata import ConvertError, canonical

FRAME = "global-target-2026-09-27"
LAND_ID, WATER_ID, LAVA_ID = 100, 101, 102
CAP = 50
FLOOR = 7


def paint(tiles: dict, x0: int, y0: int, x1: int, y1: int, kind: int) -> None:
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            tiles[(x, y)] = kind


def make_tiles() -> dict:
    """Sea (x 10-99) with: A island, B lava-enclosed island, C landmass (100 tiles, over the
    cap), D land at the map edge (void neighbours), E1/E2 two islets, F island, I island, J
    island and K an island without any wiki page."""
    tiles: dict = {}
    paint(tiles, 10, 10, 99, 60, WATER_ID)
    paint(tiles, 20, 20, 23, 22, LAND_ID)  # A: 12 tiles
    paint(tiles, 40, 20, 46, 26, LAVA_ID)
    paint(tiles, 42, 22, 44, 24, LAND_ID)  # B: 9 tiles inside lava
    paint(tiles, 60, 20, 69, 29, LAND_ID)  # C: 100 tiles, capped at 50
    paint(tiles, 100, 40, 101, 41, LAND_ID)  # D: void beside it
    paint(tiles, 30, 50, 31, 51, LAND_ID)  # E1: 4 tiles
    paint(tiles, 35, 50, 37, 50, LAND_ID)  # E2: 3 tiles
    paint(tiles, 80, 20, 84, 21, LAND_ID)  # F: 10 tiles
    paint(tiles, 70, 40, 71, 41, LAND_ID)  # I: 4 tiles
    paint(tiles, 90, 30, 91, 31, LAND_ID)  # J: 4 tiles
    paint(
        tiles, 15, 40, 15, 40, LAND_ID
    )  # K: 1 tile (plus the item-less tile at 16,40)
    paint(tiles, 50, 40, 50, 40, LAND_ID)  # L: 1 tile
    return tiles


def write(root: Path, path: str, value) -> None:
    (root / path).parent.mkdir(parents=True, exist_ok=True)
    (root / path).write_bytes(canonical(value))


def write_regions(root: Path, tiles: dict) -> None:
    """Every tile as a one-item ground tile, except (16, 40): a tile without items."""
    sectors: dict[int, list] = {}
    for (x, y), kind in sorted(tiles.items(), key=lambda t: (t[0][1], t[0][0])):
        items = [] if (x, y) == (16, 40) else [(kind - LAND_ID, 0, None)]
        sector = ((y // 32) % 8) * 8 + (x // 32) % 8
        sectors.setdefault(sector, []).append((x, y, 0, 0, (), items))
    payloads = {
        local: codec.encode_sector_tiles(local % 8, local // 8, rows)
        for local, rows in sectors.items()
    }
    path = (
        f"{convert.PLACEMENT_INDEX.rsplit('/', 1)[0]}/{codec.region_name(FLOOR, 0, 0)}"
    )
    (root / path).parent.mkdir(parents=True, exist_ok=True)
    (root / path).write_bytes(codec.encode_region(FLOOR, 0, 0, payloads))
    write(
        root,
        convert.PLACEMENT_INDEX,
        {
            "palette": [
                {"source_item_id": LAND_ID},
                {"source_item_id": WATER_ID},
                {"source_item_id": LAVA_ID},
            ],
            "regions": [{"path": path}],
        },
    )


def pos(x: int, y: int, floor: int = FLOOR) -> dict:
    return {"coordinate_frame": FRAME, "floor": floor, "x": x, "y": y}


def write_cities(root: Path) -> None:
    towns = [("Aville", 21, 21), ("Bville", 43, 23), ("Elsewhere", 200, 200)]
    records = [
        {
            "declaration": {
                "identity": {
                    "key": f"oteryn:area.city.{name.lower()}",
                    "revision": "definition-r1",
                },
                "name": name,
                "temple": pos(x, y),
            },
            "source_bindings": [],
        }
        for name, x, y in towns
    ]
    directory = "content/world/areas/cities"
    write(root, f"{directory}/cities-00000-00002.json", {"records": records})
    write(
        root,
        f"{directory}/index.json",
        {"shards": [f"{directory}/cities-00000-00002.json"]},
    )


def write_world(root: Path) -> None:
    declaration = {
        "bounds": {
            "max_x_exclusive": 256,
            "max_y_exclusive": 256,
            "min_x": 0,
            "min_y": 0,
        },
        "floors": [FLOOR],
    }
    write(
        root,
        validate.WORLD_SHARD,
        {"records": [{"declaration": declaration, "source_bindings": []}]},
    )


def page(pageid: int, title: str, coordinates=(), **extra) -> dict:
    row = {
        "coordinates": [
            {"floor": FLOOR, "origin": "wiki_lead", "x": x, "y": y}
            for x, y in coordinates
        ],
        "event_only": False,
        "evidence": f"{title} is an island.",
        "pageid": pageid,
        "revid": pageid + 1000,
        "sha256": "0" * 64,
        "title": title,
        "wiki_class": "island",
    }
    row.update(extra)
    return row


def standard_pages() -> list[dict]:
    return [
        page(1, "Alpha", [(21, 21)], wiki_cities=["Aville", "Nowhere"]),
        page(2, "Beta", [(43, 23)]),
        page(3, "Gamma", [(65, 25)]),  # inside the capped landmass
        page(4, "Delta", [(100, 40)]),
        page(
            5,
            "Epsilon Islets",
            [(30, 50), (36, 50), (90, 55)],
            wiki_class="archipelago",
        ),
        page(6, "Zeta", [(80, 20)], event_only=True),
        page(7, "Zeta Alias", [(84, 21)], alias_of="Zeta", event_only=True),
        page(8, "Iota", [(74, 40)]),  # water: moved to the nearest land tile
        page(
            9,
            "Kappa Wrong",
            [(65, 25)],
            map_correction={
                "floor": FLOOR,
                "note": "elsewhere",
                "source_page": "Alpha",
                "x": 90,
                "y": 30,
            },
        ),
        page(10, "Lambda", [(150, 55)], event_only=True),  # water far from land
        page(11, "Mu"),  # no coordinates
        page(12, "Nu Place", [(21, 20)], place_within="Alpha"),
        page(13, "Xi Group", [(30, 50), (21, 21)], wiki_class="archipelago"),
        page(14, "Omicron", [(15, 40)]),
        page(15, "Pi", [(50, 40)], kind_hint="continent"),
    ]


def snapshot_bytes(pages: list[dict]) -> bytes:
    for row in pages:
        row.setdefault("coordinates", [])
    return canonical(
        {
            "coordinate_pages": [],
            "fetched_at": "2026-09-29",
            "license": "test",
            "pages": sorted(pages, key=lambda r: r["pageid"]),
            "schema": convert.SNAPSHOT_SCHEMA,
            "source_url": "https://tibia.fandom.com",
        }
    )


def make_root(
    root: Path, pages: list[dict] | None = None, tiles: dict | None = None
) -> bytes:
    write_regions(root, tiles or make_tiles())
    write(
        root,
        convert.GROUND_CLASSES,
        {
            "lava": {"lava": str(LAVA_ID)},
            "schema": convert.GROUND_SCHEMA,
            "water": {"water": str(WATER_ID)},
        },
    )
    write_cities(root)
    write_world(root)
    data = snapshot_bytes(pages if pages is not None else standard_pages())
    (root / convert.SNAPSHOT).parent.mkdir(parents=True, exist_ok=True)
    (root / convert.SNAPSHOT).write_bytes(data)
    return data


def build(root: Path, data: bytes) -> dict:
    return convert.build(data, root, cap=CAP)


def write_family(root: Path, data: bytes) -> dict[str, bytes]:
    out = build(root, data)
    for path, content in out.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(content)
    return out


def records(out: dict) -> dict[str, dict]:
    shard = json.loads(out["content/world/areas/islands/islands-00000-00009.json"])
    return {r["declaration"]["name"]: r["declaration"] for r in shard["records"]}


class ConverterTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.data = make_root(self.root)
        self.out = build(self.root, self.data)
        self.summary = json.loads(self.out[convert.SUMMARY])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def excluded(self) -> dict[str, dict]:
        return {row["title"]: row for row in self.summary["excluded"]}

    def test_deterministic(self) -> None:
        self.assertEqual(self.out, build(self.root, self.data))

    def test_island_footprint_anchor_and_city(self) -> None:
        alpha = records(self.out)["Alpha"]
        self.assertEqual(alpha["area_kind"], "island")
        self.assertEqual(
            alpha["footprint"],
            {
                "coordinate_frame": FRAME,
                "floor": FLOOR,
                "max_x": 23,
                "max_y": 22,
                "min_x": 20,
                "min_y": 20,
                "tile_count": 12,
            },
        )
        self.assertEqual(alpha["anchor"], pos(21, 21))
        self.assertEqual(
            [c["key"] for c in alpha["cities"]], ["oteryn:area.city.aville"]
        )
        self.assertEqual(alpha["identity"]["key"], "oteryn:area.island.alpha")
        # A wiki city that is not a City Area is counted, not invented.
        self.assertEqual(
            [c["key"] for c in alpha["source_facts"]["wiki_cities"]],
            ["oteryn:area.city.aville"],
        )
        self.assertEqual(self.summary["wiki_cities_unmatched"], 1)
        self.assertNotIn("event_only", alpha)

    def test_lava_encloses_an_island(self) -> None:
        beta = records(self.out)["Beta"]
        self.assertEqual(beta["footprint"]["tile_count"], 9)
        self.assertEqual(
            [c["key"] for c in beta["cities"]], ["oteryn:area.city.bville"]
        )
        row = next(r for r in self.summary["included"] if r["title"] == "Beta")
        self.assertEqual(row["boundary_tiles"], {"lava": 12, "water": 0, "void": 0})

    def test_landmass_over_the_cap_is_excluded(self) -> None:
        self.assertEqual(self.excluded()["Gamma"]["reason"], "part_of_landmass")
        self.assertNotIn("Gamma", records(self.out))

    def test_void_neighbours_still_enclose(self) -> None:
        delta = next(r for r in self.summary["included"] if r["title"] == "Delta")
        self.assertEqual(delta["tile_count"], 4)
        self.assertEqual(delta["boundary_tiles"], {"lava": 0, "water": 2, "void": 6})

    def test_alias_page_merges_into_one_record(self) -> None:
        zeta = records(self.out)["Zeta"]
        self.assertNotIn("Zeta Alias", records(self.out))
        self.assertEqual(zeta["also_known_as"], ["Zeta Alias"])
        self.assertTrue(zeta["event_only"])
        shard = json.loads(
            self.out["content/world/areas/islands/islands-00000-00009.json"]
        )
        bindings = next(
            r["source_bindings"]
            for r in shard["records"]
            if r["declaration"]["name"] == "Zeta"
        )
        self.assertEqual([b["external_id"] for b in bindings], ["6", "7"])
        self.assertEqual(
            {b["target"]["key"] for b in bindings}, {"oteryn:area.island.zeta"}
        )

    def test_archipelago_keeps_each_component(self) -> None:
        epsilon = records(self.out)["Epsilon Islets"]
        self.assertEqual(epsilon["area_kind"], "archipelago")
        self.assertNotIn("footprint", epsilon)
        self.assertEqual(
            [
                (c["footprint"]["min_x"], c["footprint"]["tile_count"])
                for c in epsilon["components"]
            ],
            [(30, 4), (35, 3)],
        )
        self.assertEqual(self.summary["records_with"]["components"], 12)

    def test_archipelago_with_one_component_is_an_island(self) -> None:
        pages = [
            page(1, "Solo Group", [(30, 50), (90, 55)], wiki_class="archipelago"),
            page(2, "Pair Group", [(30, 50), (21, 21)], wiki_class="archipelago"),
        ]
        out = build(self.root, make_root(self.root, pages))
        got = json.loads(out["content/world/areas/islands/islands-00000-00001.json"])
        kinds = {
            r["declaration"]["name"]: r["declaration"]["area_kind"]
            for r in got["records"]
        }
        self.assertEqual(kinds, {"Pair Group": "archipelago", "Solo Group": "island"})

    def test_component_links_to_the_island_record_with_the_same_footprint(self) -> None:
        pages = [
            page(1, "Alpha", [(21, 21)]),
            page(2, "Duo", [(21, 21), (43, 23)], wiki_class="archipelago"),
            page(3, "Beta", [(43, 23)]),
        ]
        out = build(self.root, make_root(self.root, pages))
        got = json.loads(out["content/world/areas/islands/islands-00000-00002.json"])
        duo = next(
            r["declaration"]
            for r in got["records"]
            if r["declaration"]["name"] == "Duo"
        )
        self.assertEqual(
            [c["island"]["key"] for c in duo["components"]],
            ["oteryn:area.island.alpha", "oteryn:area.island.beta"],
        )

    def test_anchor_moves_to_the_nearest_land_tile(self) -> None:
        iota = records(self.out)["Iota"]
        self.assertEqual(iota["anchor"], pos(71, 40))
        self.assertEqual(iota["source_facts"]["source_coordinate"], pos(74, 40))
        self.assertEqual(self.summary["records_with"]["anchor_moved"], 1)

    def test_map_correction_replaces_a_landmass_coordinate(self) -> None:
        kappa = records(self.out)["Kappa Wrong"]
        self.assertTrue(kappa["anchor_corrected_from_wiki"])
        self.assertEqual(kappa["anchor"], pos(90, 30))
        self.assertEqual(kappa["footprint"]["tile_count"], 4)
        self.assertEqual(kappa["source_facts"]["source_coordinate"], pos(65, 25))
        self.assertEqual(kappa["source_facts"]["anchor_origin"], "map_correction")

    def test_correction_is_refused_when_the_wiki_coordinate_is_an_island(self) -> None:
        pages = [
            page(
                1,
                "Alpha",
                [(21, 21)],
                map_correction={
                    "floor": FLOOR,
                    "note": "x",
                    "source_page": "Alpha",
                    "x": 90,
                    "y": 30,
                },
            )
        ]
        with self.assertRaises(ConvertError):
            build(self.root, make_root(self.root, pages))

    def test_exclusions_carry_their_reason(self) -> None:
        excluded = self.excluded()
        self.assertEqual(excluded["Lambda"]["reason"], "event_only_not_on_map")
        self.assertEqual(excluded["Mu"]["reason"], "no_coordinates")
        self.assertEqual(excluded["Nu Place"]["reason"], "part_of_landmass")
        self.assertEqual(excluded["Nu Place"]["detail"], "place within Alpha")
        self.assertEqual(
            self.summary["excluded_reasons"],
            {"event_only_not_on_map": 1, "no_coordinates": 1, "part_of_landmass": 2},
        )

    def test_a_tile_without_items_is_land(self) -> None:
        omicron = records(self.out)["Omicron"]
        self.assertEqual(omicron["footprint"]["tile_count"], 2)

    def test_continent_kind_hint(self) -> None:
        self.assertEqual(records(self.out)["Pi"]["area_kind"], "continent")

    def test_two_pages_on_one_component_need_an_alias(self) -> None:
        pages = [page(1, "Alpha", [(21, 21)]), page(2, "Twin", [(20, 20)])]
        with self.assertRaises(ConvertError):
            build(self.root, make_root(self.root, pages))

    def test_alias_must_share_the_component(self) -> None:
        pages = [
            page(1, "Alpha", [(21, 21)]),
            page(2, "Twin", [(43, 23)], alias_of="Alpha"),
        ]
        with self.assertRaises(ConvertError):
            build(self.root, make_root(self.root, pages))

    def test_place_within_must_lie_inside(self) -> None:
        pages = [
            page(1, "Alpha", [(21, 21)]),
            page(2, "Inside", [(43, 23)], place_within="Alpha"),
        ]
        with self.assertRaises(ConvertError):
            build(self.root, make_root(self.root, pages))

    def test_city_temple_coordinate_must_match_the_city(self) -> None:
        row = page(1, "Aville", [])
        row["coordinates"] = [
            {
                "city": "Aville",
                "floor": FLOOR,
                "origin": "city_temple",
                "x": 22,
                "y": 21,
            }
        ]
        with self.assertRaises(ConvertError):
            build(self.root, make_root(self.root, [row]))

    def test_keys_are_reused_by_page_id(self) -> None:
        write_family(self.root, self.data)
        renamed = standard_pages()
        renamed[1]["title"] = "Beta Renamed"
        out = build(self.root, make_root(self.root, renamed))
        self.assertIn("Beta Renamed", records(out))
        self.assertEqual(
            records(out)["Beta Renamed"]["identity"]["key"], "oteryn:area.island.beta"
        )

    def test_snapshot_must_be_well_formed(self) -> None:
        for mutate in (
            lambda d: d.update(schema="other"),
            lambda d: d["pages"].append(dict(d["pages"][0])),
            lambda d: d["pages"][0].update(place_within="Missing"),
        ):
            data = json.loads(self.data)
            mutate(data)
            with self.assertRaises(ConvertError):
                convert.build(canonical(data), self.root, cap=CAP)

    def test_bounded_search(self) -> None:
        tiles, _, capped = convert.land_component(
            lambda x, y, z: (
                convert.LAND if 0 <= x < 100 and 0 <= y < 100 else convert.VOID
            ),
            0,
            0,
            FLOOR,
            20,
        )
        self.assertTrue(capped)
        self.assertEqual(len(tiles), 20)


class GroundClassTest(unittest.TestCase):
    XML = (
        b'<items><item id="100" name="Grass"/><item fromid="101" toid="103" name="Shallow Water"/>'
        b'<item id="200" name="lava"/><item id="300" name="water"/></items>'
    )

    def test_ids_round_trip(self) -> None:
        ids = {5, 6, 7, 9, 12, 13}
        self.assertEqual(convert.format_ids(ids), "5-7,9,12-13")
        self.assertEqual(convert.parse_ids("5-7,9,12-13"), ids)

    def test_derived_from_names_and_palette_only(self) -> None:
        document = convert.derive_ground_classes(self.XML, {100, 101, 102, 200})
        self.assertEqual(document["water"], {"shallow water": "101-102"})
        self.assertEqual(document["lava"], {"lava": "200"})
        self.assertEqual(
            document["source"]["sha256"], convert.hashlib.sha256(self.XML).hexdigest()
        )

    def test_disjoint_and_non_empty(self) -> None:
        data = canonical(
            {
                "lava": {"lava": "5"},
                "schema": convert.GROUND_SCHEMA,
                "water": {"water": "5"},
            }
        )
        with self.assertRaises(ConvertError):
            convert.load_ground_classes(data)


class SnapshotReaderTest(unittest.TestCase):
    def test_mapper_coordinates(self) -> None:
        text = (
            "{{Infobox\n| status = deprecated\n}}'''X''' is an island {{Mapper Coords|126.5|126.129|7|3|text=here}}."
            "\n== Travel ==\n{{Mapper Coords|125.153|126.200|7|4|text=Ferry}} "
            "{{Mapper Coords|125.153|126.200|7|4}} {{Mapper Coords|1.300|2.1|7}}"
        )
        rows = snapshot_tool.wiki_coordinates(text)
        self.assertEqual(
            [(r["x"], r["y"], r["floor"], r["origin"]) for r in rows],
            [(32261, 32385, 7, "wiki_lead"), (32153, 32456, 7, "wiki_body")],
        )
        self.assertTrue(snapshot_tool.has_coordinate(text, 32153, 32456, 7))
        self.assertEqual(snapshot_tool.infobox_status(text), ["deprecated"])

    def test_island_sentence_skips_templates(self) -> None:
        text = "{{Infobox Geography|List={{{1|}}}}}\n__TOC__ '''Foo''' is a [[small]] island, see [[File:x.png]]."
        self.assertEqual(
            snapshot_tool.island_sentence(text), "Foo is a small island, see ."
        )


class ValidatorTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.data = make_root(self.root)
        write_family(self.root, self.data)
        patch = mock.patch.object(validate, "CAP", CAP)
        patch.start()
        self.addCleanup(patch.stop)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def shard_path(self) -> Path:
        return self.root / "content/world/areas/islands/islands-00000-00009.json"

    def edit_shard(self, mutate) -> None:
        shard = json.loads(self.shard_path().read_text(encoding="utf-8"))
        mutate(shard)
        self.shard_path().write_bytes(canonical(shard))

    def declaration(self, shard: dict, name: str) -> dict:
        return next(
            r["declaration"]
            for r in shard["records"]
            if r["declaration"]["name"] == name
        )

    def errors(self) -> str:
        return "\n".join(validate.validate(self.root))

    def test_valid(self) -> None:
        self.assertEqual(validate.validate(self.root), [])

    def test_stray_file(self) -> None:
        (self.root / "content/world/areas/islands/notes.txt").write_text("x")
        self.assertIn("files other than", self.errors())

    def test_footprint_outside_the_world(self) -> None:
        def mutate(shard):
            box = self.declaration(shard, "Alpha")["footprint"]
            box["max_x"] = 300

        self.edit_shard(mutate)
        self.assertIn("outside the World", self.errors())

    def test_anchor_outside_the_footprint(self) -> None:
        def mutate(shard):
            self.declaration(shard, "Alpha")["anchor"]["x"] = 50

        self.edit_shard(mutate)
        self.assertIn("anchor is outside its footprint", self.errors())

    def test_anchor_far_from_the_snapshot_coordinate(self) -> None:
        snapshot = json.loads(
            (self.root / convert.SNAPSHOT).read_text(encoding="utf-8")
        )
        snapshot["pages"][0]["coordinates"][0]["x"] = 60
        (self.root / convert.SNAPSHOT).write_bytes(canonical(snapshot))
        self.assertIn("not within 5 tiles", self.errors())

    def test_city_temple_must_lie_inside(self) -> None:
        def mutate(shard):
            self.declaration(shard, "Alpha")["cities"] = [
                {
                    "family": "Area",
                    "key": "oteryn:area.city.elsewhere",
                    "revision": "definition-r1",
                }
            ]

        self.edit_shard(mutate)
        self.assertIn("temple of oteryn:area.city.elsewhere is outside", self.errors())

    def test_unknown_city(self) -> None:
        def mutate(shard):
            self.declaration(shard, "Alpha")["cities"][0]["key"] = (
                "oteryn:area.city.nowhere"
            )

        self.edit_shard(mutate)
        self.assertIn("is not a City Area", self.errors())

    def test_binding_must_be_in_the_snapshot(self) -> None:
        def mutate(shard):
            shard["records"][0]["source_bindings"][0]["source_revision"] = "999999"

        self.edit_shard(mutate)
        self.assertIn("not in the pinned snapshot", self.errors())

    def test_pins_must_match_the_files(self) -> None:
        (self.root / convert.SNAPSHOT).write_bytes(
            (self.root / convert.SNAPSHOT)
            .read_bytes()
            .replace(b"is an island", b"is an isle")
        )
        self.assertIn("differs from the sha256 pinned", self.errors())

    def test_alias_metadata_must_match(self) -> None:
        def mutate(shard):
            del self.declaration(shard, "Zeta")["also_known_as"]

        self.edit_shard(mutate)
        self.assertIn("also_known_as differs", self.errors())

    def test_event_only_must_match_the_snapshot(self) -> None:
        def mutate(shard):
            del self.declaration(shard, "Zeta")["event_only"]

        self.edit_shard(mutate)
        self.assertIn("event_only differs", self.errors())

    def test_keys_sorted_and_summary_consistent(self) -> None:
        summary = json.loads((self.root / convert.SUMMARY).read_text(encoding="utf-8"))
        summary["excluded"].pop()
        (self.root / convert.SUMMARY).write_bytes(canonical(summary))
        self.assertIn("differ from the snapshot", self.errors())

    def test_archipelago_needs_components_not_a_footprint(self) -> None:
        def mutate(shard):
            self.declaration(shard, "Epsilon Islets")["footprint"] = self.declaration(
                shard, "Alpha"
            )["footprint"]

        self.edit_shard(mutate)
        self.assertIn("schema", self.errors())

    def test_component_link_must_match_the_island(self) -> None:
        with tempfile.TemporaryDirectory() as other:
            root = Path(other)
            pages = [
                page(1, "Alpha", [(21, 21)]),
                page(2, "Duo", [(21, 21), (43, 23)], wiki_class="archipelago"),
                page(3, "Beta", [(43, 23)]),
            ]
            write_family(root, make_root(root, pages))
            self.assertEqual(validate.validate(root), [])
            path = root / "content/world/areas/islands/islands-00000-00002.json"
            shard = json.loads(path.read_text(encoding="utf-8"))
            duo = self.declaration(shard, "Duo")
            duo["components"][0]["island"]["key"] = "oteryn:area.island.beta"
            path.write_bytes(canonical(shard))
            self.assertIn("component island link", "\n".join(validate.validate(root)))

    def test_search_parameters_are_pinned(self) -> None:
        with mock.patch.object(validate, "CAP", 400_000):
            self.assertIn("search parameters differ", self.errors())


if __name__ == "__main__":
    unittest.main()
