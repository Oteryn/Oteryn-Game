"""Tests for the World record converter, its validator and the worlds/ tree exception."""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

import convert_world_record as convert
import validate_world_record as validate
import world_region_codec as codec
from convert_world_metadata import ConvertError, canonical

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import validate_materialized_game_tree as tree

# Floors 7 and 9 only (a sparse set). Sector x 24 holds the minimum x (770) beside 799, sector
# x 31 the maximum x (1023) beside 1000, and likewise for y, so exactness needs the decode.
TILES = [
    (7, 770, 775),
    (7, 799, 799),
    (7, 1000, 1480),
    (7, 1023, 1500),
    (9, 900, 1000),
]
FRAME = "global-target-2026-09-27"


def pos(x: int, y: int, floor: int) -> dict:
    return {"coordinate_frame": FRAME, "floor": floor, "x": x, "y": y}


def write(root: Path, path: str, value) -> None:
    (root / path).parent.mkdir(parents=True, exist_ok=True)
    (root / path).write_bytes(canonical(value))


def write_regions(root: Path) -> None:
    regions: dict[tuple[int, int, int], dict[int, list]] = {}
    for z, x, y in sorted(TILES, key=lambda t: (t[0], t[2], t[1])):
        sector = ((y // 32) % 8) * 8 + (x // 32) % 8
        key = (z, x // 256, y // 256)
        regions.setdefault(key, {}).setdefault(sector, []).append((x, y, 0, 0, (), []))
    rows = []
    for (z, rx, ry), sectors in sorted(regions.items()):
        payloads = {
            local: codec.encode_sector_tiles(
                rx * 8 + local % 8, ry * 8 + local // 8, tiles
            )
            for local, tiles in sectors.items()
        }
        path = f"{convert.PLACEMENTS}/{codec.region_name(z, rx, ry)}"
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(codec.encode_region(z, rx, ry, payloads))
        rows.append({"path": path})
    write(root, f"{convert.PLACEMENTS}/index.json", {"regions": rows})


def make_root(root: Path) -> None:
    write_regions(root)
    write(
        root,
        convert.BASE_SUMMARY,
        {
            "map": {
                "floors": [0, 15],
                "height": 2000,
                "otbm_version": 4,
                "width": 2000,
            },
            "tiles_by_floor": {"7": 4, "9": 1},
        },
    )
    write(
        root,
        convert.REFERENCE,
        {"coordinate_frame": FRAME, "world_id": "0123456789ab70cd8ef0123456789abc"},
    )
    families = {
        "Area.City": {"temple": pos(800, 800, 7)},
        "House": {
            "entry": pos(900, 1000, 9),
            "doors": [{"door_id": 1, "position": pos(901, 1000, 9)}],
            "footprint": {
                "floors": [{"floor": 9, "tiles": 4}],
                "max_x": 902,
                "max_y": 1002,
                "min_x": 900,
                "min_y": 1000,
                "tile_count": 4,
            },
        },
        "Transition.Teleport": {"from": pos(800, 800, 7), "to": pos(900, 1000, 9)},
        "Area.HuntingPlace": {"position": pos(1000, 1480, 7)},
        "Area.Region": {
            "anchor": pos(800, 800, 7),
            "footprint": {
                "coordinate_frame": FRAME,
                "floor": 7,
                "max_x": 802,
                "max_y": 802,
                "min_x": 800,
                "min_y": 800,
                "tile_count": 4,
            },
        },
    }
    for family, declaration in families.items():
        directory = validate.FAMILIES[family][0]
        declaration = {
            **declaration,
            "identity": {"key": f"oteryn:test.{family}", "revision": "definition-r1"},
        }
        shard = f"{directory}/x-00000-00000.json"
        write(root, shard, {"records": [{"declaration": declaration}]})
        write(root, f"{directory}/index.json", {"shards": [shard]})
    write_family(root)


def write_family(root: Path) -> None:
    for path, data in convert.build(root).items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)
    (root / convert.DIRECTORY / validate.LEGACY_LOCATOR).write_text(
        '{"schema":"OTERYN_WORLD_PROJECT_WORLDS/v2","worlds":[],"placements":[]}'
    )


class WorldRecordTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        make_root(self.root)

    def shard(self) -> dict:
        return json.loads((self.root / convert.SHARD_PATH).read_text())

    def write_shard(self, shard: dict) -> None:
        (self.root / convert.SHARD_PATH).write_bytes(canonical(shard))

    def errors(self) -> list[str]:
        return validate.validate(self.root)

    def has(self, text: str) -> bool:
        return any(text in error for error in self.errors())

    def test_fixture_converts_to_a_valid_record_with_exact_bounds(self):
        self.assertEqual(self.errors(), [])
        declaration = self.shard()["records"][0]["declaration"]
        self.assertEqual(
            declaration["bounds"],
            {
                "max_x_exclusive": 1024,
                "max_y_exclusive": 1501,
                "min_x": 770,
                "min_y": 775,
            },
        )
        self.assertEqual(declaration["floors"], [7, 9])
        self.assertEqual(declaration["identity"]["key"], "oteryn:world.oteryn")
        self.assertEqual(declaration["source_map"]["width"], 2000)
        self.assertEqual(convert.build(self.root), convert.build(self.root))

    def test_extent_matches_a_full_decode(self):
        xs, ys = [], []
        for path in (self.root / convert.PLACEMENTS).glob("*.b3"):
            _z, _rx, _ry, sectors = codec.decode_region(path.read_bytes())
            for _local, tiles in sectors:
                xs += [t[0] for t in tiles]
                ys += [t[1] for t in tiles]
        extent = convert.placement_extent(self.root)
        self.assertEqual(
            (extent["min_x"], extent["max_x"], extent["min_y"], extent["max_y"]),
            (min(xs), max(xs), min(ys), max(ys)),
        )

    def test_missing_base_map_fails_closed(self):
        shutil.rmtree(self.root / convert.PLACEMENTS)
        with self.assertRaises(ConvertError):
            convert.build(self.root)

    def test_bounds_and_floors_must_equal_the_base_map(self):
        for edit, text in (
            (lambda d: d["bounds"].update(min_x=769), "differ from the base map"),
            (
                lambda d: d["bounds"].update(max_y_exclusive=1600),
                "differ from the base",
            ),
            (lambda d: d.update(floors=[7, 8, 9]), "floors differ from the base map"),
            (lambda d: d.update(floors=[9, 7]), "strictly increasing"),
            (lambda d: d.update(legacy_world_id="0" * 32), "legacy_world_id"),
            (lambda d: d["bounds"].update(min_x=2000, max_x_exclusive=2001), "exceed"),
        ):
            with self.subTest(text=text):
                shard = self.shard()
                edit(shard["records"][0]["declaration"])
                self.write_shard(shard)
                self.assertTrue(self.has(text), self.errors())
                write_family(self.root)
        self.assertEqual(self.errors(), [])

    def test_source_map_and_binding_are_checked(self):
        shard = self.shard()
        shard["records"][0]["declaration"]["source_map"]["width"] = 1999
        self.write_shard(shard)
        self.assertTrue(self.has("source_map differs"))
        write_family(self.root)
        shard = self.shard()
        shard["records"][0]["source_bindings"][0]["target"]["key"] = "oteryn:world.x"
        self.write_shard(shard)
        self.assertTrue(self.has("target differs from identity"))

    def test_positions_outside_bounds_or_floors_are_rejected(self):
        cases = {
            "Area.City": ("temple", pos(1024, 800, 7), "outside the world bounds"),
            "Transition.Teleport": ("to", pos(900, 1000, 8), "undeclared floor"),
            "Area.HuntingPlace": ("position", pos(1000, 774, 7), "outside"),
        }
        for family, (name, bad, text) in cases.items():
            with self.subTest(family=family):
                path = f"{validate.FAMILIES[family][0]}/x-00000-00000.json"
                shard = json.loads((self.root / path).read_text())
                original = shard["records"][0]["declaration"][name]
                shard["records"][0]["declaration"][name] = bad
                write(self.root, path, shard)
                self.assertTrue(self.has(text), self.errors())
                shard["records"][0]["declaration"][name] = original
                write(self.root, path, shard)
        self.assertEqual(self.errors(), [])

    def test_house_door_and_footprint_positions_are_rejected(self):
        path = f"{validate.FAMILIES['House'][0]}/x-00000-00000.json"
        shard = json.loads((self.root / path).read_text())
        declaration = shard["records"][0]["declaration"]
        declaration["doors"][0]["position"] = pos(901, 1000, 8)
        write(self.root, path, shard)
        self.assertTrue(self.has("undeclared floor"))
        declaration["doors"][0]["position"] = pos(901, 1000, 9)
        declaration["footprint"]["max_x"] = 5000
        write(self.root, path, shard)
        self.assertTrue(self.has("outside the world bounds"))

    def test_stray_files_and_non_canonical_bytes_are_rejected(self):
        (self.root / convert.DIRECTORY / "extra.json").write_text("{}")
        self.assertTrue(self.has("expected exactly"))
        (self.root / convert.DIRECTORY / "extra.json").unlink()
        (self.root / convert.DIRECTORY / validate.LEGACY_LOCATOR).unlink()
        self.assertTrue(self.has("expected exactly"))
        write_family(self.root)
        path = self.root / convert.SHARD_PATH
        path.write_text(path.read_text() + " ")
        self.assertTrue(self.has("not canonical"))

    def test_schema_rejects_a_second_record_and_extra_fields(self):
        shard = self.shard()
        shard["records"].append(shard["records"][0])
        self.write_shard(shard)
        self.assertTrue(self.has("schema"))
        shard["records"].pop()
        shard["records"][0]["declaration"]["extra"] = 1
        self.write_shard(shard)
        self.assertTrue(self.has("schema"))

    def test_committed_family_is_current(self):
        repo = convert.ROOT
        for path, data in convert.build(repo).items():
            self.assertEqual((repo / path).read_bytes(), data, path)
        self.assertEqual(validate.validate(repo), [])


class SharedDirectoryRuleTest(unittest.TestCase):
    """`worlds/` is the one successor directory beside a legacy locator (worlds/world.json)."""

    PATH = tree.SHARED_WITH_LOCATOR
    SHARD = "worlds-00000-00000.json"

    def index(self, shards) -> dict:
        return {
            "population_state": "POPULATED",
            "shards": [self.PATH + name for name in shards],
        }

    def test_worlds_may_hold_one_shard_beside_the_locator(self):
        payload = self.index([self.SHARD])
        local = {"index.json", "world.json", self.SHARD}
        tree.check_family_index(self.PATH, payload, local, {"world.json"})

    def test_worlds_rejects_two_shards_a_stray_file_or_a_missing_locator(self):
        two = self.index([self.SHARD, "worlds-00001-00001.json"])
        local = {"index.json", "world.json", self.SHARD, "worlds-00001-00001.json"}
        with self.assertRaises(tree.ValidationError):
            tree.check_family_index(self.PATH, two, local, {"world.json"})
        one = self.index([self.SHARD])
        for bad in (
            {"index.json", "world.json", self.SHARD, "extra.json"},
            {"index.json", self.SHARD},
        ):
            with self.assertRaises(tree.ValidationError):
                tree.check_family_index(self.PATH, one, bad, {"world.json"})

    def test_every_other_directory_still_refuses_a_family_beside_a_locator(self):
        path = "content/world/areas/cities/"
        payload = {
            "population_state": "POPULATED",
            "shards": [path + "cities-00000-00000.json"],
        }
        local = {"index.json", "cities-00000-00000.json", "world.json"}
        with self.assertRaises(tree.ValidationError):
            tree.check_family_index(path, payload, local, {"world.json"})
        tree.check_family_index(path, payload, local - {"world.json"}, set())

    def test_the_shard_is_a_successor_file_never_a_legacy_locator(self):
        dirs = tree.directory_nodes()
        self.assertIn("worlds/" + self.SHARD, tree.world_successor_files(dirs))
        self.assertNotIn("worlds/" + self.SHARD, tree.legacy_locators())


if __name__ == "__main__":
    unittest.main()
