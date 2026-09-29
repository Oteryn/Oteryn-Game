"""Tests for the client appearance reader, the Terrain converter and the Terrain validator.

The client file and the map are small synthetic fixtures written as real files (a protobuf
appearances file, region files, the client manifest and the base map palette), so the whole
path is covered: decode, class rule, id selection, occurrence counts, the family files and the
validator.
"""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import client_appearance_reader as reader
import client_map_reader
import convert_terrain as convert
import validate_terrain as validate
import world_region_codec as codec
from convert_world_metadata import canonical

# Appearance ids: 100 grass-like ground, 101 border, 102 wall, 103 decoration, 104 an id the
# items.xml declares, 105 an item-bound id, 106 an id the client does not know.
GRASS, EDGE, WALL, PLANT, DECLARED, BOUND, UNKNOWN = 100, 101, 102, 103, 104, 105, 106
PLACEMENTS = "content/world/placements"
ITEM_KEY = "oteryn:item.registry.i00000105"


def varint(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        out.append(byte | (0x80 if value else 0))
        if not value:
            return bytes(out)


def field(number: int, value) -> bytes:
    if isinstance(value, int):
        return varint(number << 3) + varint(value)
    if isinstance(value, str):
        value = value.encode()
    return varint(number << 3 | 2) + varint(len(value)) + value


def flags(
    bank=None, clip=False, unpass=False, unmove=True, automap=None, extra=b""
) -> bytes:
    out = b""
    if bank is not None:
        out += field(1, field(1, bank))
    if clip:
        out += field(2, 1)
    if unpass:
        out += field(13, 1)
    if unmove:
        out += field(14, 1)
    if automap is not None:
        out += field(30, field(1, automap))
    return out + extra


def appearance(object_id: int, name=None, flag_bytes=None, sprites=True) -> bytes:
    out = field(1, object_id)
    if sprites:
        out += field(2, field(1, 0) + field(3, field(5, 4242)))  # skipped frame group
    if flag_bytes is not None:
        out += field(3, flag_bytes)
    if name:
        out += field(4, name)
    return out


def appearances_file(objects: list[bytes]) -> bytes:
    return b"".join(field(1, o) for o in objects) + field(2, appearance(999))


def standard_objects() -> list[bytes]:
    return [
        appearance(GRASS, flag_bytes=flags(bank=150, automap=24)),
        appearance(EDGE, flag_bytes=flags(clip=True)),
        appearance(WALL, name="stone wall", flag_bytes=flags(unpass=True)),
        appearance(PLANT, flag_bytes=flags()),
        appearance(DECLARED, flag_bytes=flags(bank=100)),
        appearance(BOUND, flag_bytes=flags(bank=100)),
    ]


def write(root: Path, path: str, data: bytes) -> None:
    (root / path).parent.mkdir(parents=True, exist_ok=True)
    (root / path).write_bytes(data)


def make_root(root: Path, objects: list[bytes] | None = None) -> str:
    """Client files and a base map whose palette lists every fixture id; returns the digest."""
    data = appearances_file(objects if objects is not None else standard_objects())
    digest = hashlib.sha256(data).hexdigest()
    name = f"appearances-{digest}.dat"
    write(root, f"{convert.regions.FILES}/{name}", data)
    write(
        root,
        convert.regions.MANIFEST,
        canonical(
            {"archive_sha256": "a" * 64, "files": [{"name": name, "sha256": digest}]}
        ),
    )
    ids = [GRASS, EDGE, WALL, PLANT, DECLARED, BOUND, UNKNOWN]
    palette = [
        {
            "key": ITEM_KEY if i == BOUND else f"donor:crystalserver@00ce02a5:item/{i}",
            "provisional": i != BOUND,
            "source_item_id": i,
        }
        for i in ids
    ]
    # Region tiles: ground under a wall, twice; a plant and a border on their own.
    rows = [
        (10, 10, 0, 0, (), [(0, 0, None), (2, 0, None)]),
        (11, 10, 0, 0, (), [(0, 0, None), (2, 0, None), (5, 0, None)]),
        (12, 10, 0, 0, (), [(3, 0, None), (1, 1, None)]),  # depth 1: container content
        (13, 10, 0, 0, (), [(4, 0, None)]),
    ]
    payload = codec.encode_sector_tiles(0, 0, rows)
    region = f"{PLACEMENTS}/{codec.region_name(7, 0, 0)}"
    write(root, region, codec.encode_region(7, 0, 0, {0: payload}))
    write(
        root,
        f"{PLACEMENTS}/index.json",
        canonical({"palette": palette, "regions": [{"path": region}]}),
    )
    return digest


def set_keys(root: Path, keys: dict[int, str]) -> None:
    """Switch palette entries to Terrain keys, as `convert_world_base.py` does."""
    path = root / f"{PLACEMENTS}/index.json"
    index = json.loads(path.read_text(encoding="utf-8"))
    for row in index["palette"]:
        if row["source_item_id"] in keys:
            row.update(key=keys[row["source_item_id"]], provisional=False)
    path.write_bytes(canonical(index))


def install(root: Path, out: dict[str, bytes]) -> None:
    for path, data in out.items():
        write(root, path, data)


DECLARED_IDS = {DECLARED}
TERRAIN_IDS = {GRASS: 0, EDGE: 1, WALL: 2, PLANT: 3}  # appearance id -> palette index


def keys() -> dict[int, str]:
    return {i: convert.record_key(i) for i in TERRAIN_IDS}


class ReaderTest(unittest.TestCase):
    def test_flags_and_facts(self) -> None:
        found = reader.read_appearances(appearances_file(standard_objects()))
        self.assertEqual(sorted(found), [100, 101, 102, 103, 104, 105])
        grass = found[GRASS]
        self.assertEqual(
            (grass.flags, grass.speed, grass.automap_color, grass.name),
            (frozenset({"bank", "unmove"}), 150, 24, None),
        )
        self.assertEqual(found[EDGE].flags, frozenset({"clip", "unmove"}))
        self.assertEqual(found[WALL].flags, frozenset({"unpass", "unmove"}))
        self.assertEqual(found[WALL].name, "stone wall")
        self.assertEqual(found[PLANT].flags, frozenset({"unmove"}))

    def test_unread_flags_are_skipped(self) -> None:
        extra = field(18, 1) + field(23, field(1, 5) + field(2, 200))
        found = reader.read_appearances(
            appearances_file([appearance(7, flag_bytes=flags(extra=extra))])
        )
        self.assertEqual(found[7].flags, frozenset({"unmove"}))

    def test_false_boolean_and_absent_flags(self) -> None:
        raw = field(13, 0) + field(14, 1)
        found = reader.read_appearances(
            appearances_file([appearance(7, flag_bytes=raw), appearance(8)])
        )
        self.assertEqual(found[7].flags, frozenset({"unmove"}))
        self.assertEqual(found[8].flags, frozenset())

    def test_malformed_files_fail_closed(self) -> None:
        good = appearances_file(standard_objects())
        cases = {
            "truncated": good[:-3],
            "duplicate id": appearances_file([appearance(7), appearance(7)]),
            "non boolean flag": appearances_file(
                [appearance(7, flag_bytes=field(13, 2))]
            ),
            "repeated flag": appearances_file(
                [appearance(7, flag_bytes=field(13, 1) + field(13, 1))]
            ),
            "bank not a message": appearances_file(
                [appearance(7, flag_bytes=field(1, 5))]
            ),
            "object without id": appearances_file([field(4, "x")]),
            "fixed32 field": field(1, appearance(7)) + bytes([0x1D, 0, 0, 0, 0]),
        }
        for name, data in cases.items():
            with (
                self.subTest(name),
                self.assertRaises(client_map_reader.ClientMapError),
            ):
                reader.read_appearances(data)


class ClassRuleTest(unittest.TestCase):
    def test_first_matching_rule_decides(self) -> None:
        cases = [
            ({"bank", "unpass", "unmove", "clip"}, "ground"),
            ({"clip", "unpass"}, "border"),
            ({"unpass", "unmove"}, "blocking"),
            ({"unmove"}, "decoration"),
            (set(), "decoration"),
        ]
        for names, expected in cases:
            with self.subTest(sorted(names)):
                self.assertEqual(convert.terrain_class(frozenset(names)), expected)


class ConverterTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.digest = make_root(self.root)

    def records(self, out: dict[str, bytes]) -> dict[int, dict]:
        (path,) = [k for k in out if k.startswith("content/world/terrain/terrain-")]
        rows = json.loads(out[path])["records"]
        return {r["declaration"]["appearance_id"]: r for r in rows}

    def build(self, declared=DECLARED_IDS) -> dict[str, bytes]:
        return convert.build(self.root, declared)

    def test_derived_set_is_provisional_appearance_only_ids(self) -> None:
        found = self.records(self.build())
        # 104 is declared by items.xml, 105 is item-bound, 106 has no appearance
        self.assertEqual(sorted(found), [GRASS, EDGE, WALL, PLANT])

    def test_facts_come_from_the_appearance(self) -> None:
        found = self.records(self.build())
        grass = found[GRASS]["declaration"]
        self.assertEqual(
            grass,
            {
                "appearance_id": 100,
                "automap_color": 24,
                "class": "ground",
                "flags": ["bank", "unmove"],
                "identity": {
                    "key": "oteryn:terrain.a000100",
                    "revision": "definition-r1",
                },
                "kind": "Terrain",
                "occurrences_on_base_map": 2,
                "speed": 150,
            },
        )
        self.assertEqual(found[EDGE]["declaration"]["class"], "border")
        wall = found[WALL]["declaration"]
        self.assertEqual((wall["class"], wall["name"]), ("blocking", "stone wall"))
        self.assertNotIn("speed", wall)
        self.assertEqual(found[PLANT]["declaration"]["class"], "decoration")
        (binding,) = found[GRASS]["source_bindings"]
        self.assertEqual(
            binding,
            {
                "disposition": "EXACT",
                "external_id": "100",
                "identity_namespace": "tibia-client/appearance-id",
                "source_key": "oteryn:source.tibia_client",
                "source_revision": self.digest,
                "target": {
                    "family": "Terrain",
                    "key": "oteryn:terrain.a000100",
                    "revision": "definition-r1",
                },
            },
        )

    def test_occurrences_count_top_level_tile_items_only(self) -> None:
        found = self.records(self.build())
        counts = {
            i: r["declaration"]["occurrences_on_base_map"] for i, r in found.items()
        }
        # the wall (2) twice, the plant once at depth 0, the border only as container content
        self.assertEqual(counts, {GRASS: 2, EDGE: 0, WALL: 2, PLANT: 1})

    def test_summary_counts_by_class(self) -> None:
        out = self.build()
        summary = json.loads(out[convert.SUMMARY])
        self.assertEqual(summary["families"], {"Terrain": 4})
        self.assertEqual(
            {k: v["records"] for k, v in summary["classes"].items()},
            {"blocking": 1, "border": 1, "decoration": 1, "ground": 1},
        )
        self.assertEqual(summary["occurrences_on_base_map"], 5)
        self.assertEqual(summary["flags"]["unmove"], 4)
        self.assertEqual(
            summary["records_with"], {"automap_color": 1, "name": 1, "speed": 1}
        )

    def test_offline_set_is_the_terrain_keyed_palette_entries(self) -> None:
        derived = self.build()
        set_keys(self.root, keys())
        self.assertEqual(convert.build(self.root), derived)

    def test_offline_build_without_terrain_keys_needs_the_checkout(self) -> None:
        with self.assertRaises(convert.ConvertError):
            convert.build(self.root)

    def test_malformed_terrain_key_or_missing_appearance_fails(self) -> None:
        set_keys(self.root, {GRASS: "oteryn:terrain.a000101"})
        with self.assertRaises(convert.ConvertError):
            convert.build(self.root)
        set_keys(
            self.root,
            {GRASS: convert.record_key(GRASS), UNKNOWN: convert.record_key(UNKNOWN)},
        )
        with self.assertRaises(convert.ConvertError):
            convert.build(self.root)

    def test_client_file_must_match_the_manifest(self) -> None:
        path = self.root / f"{convert.regions.FILES}/appearances-{self.digest}.dat"
        path.write_bytes(path.read_bytes() + b"\x00")
        with self.assertRaises(convert.ConvertError):
            self.build()

    def test_bank_without_speed_fails_closed(self) -> None:
        broken = appearance(GRASS, flag_bytes=field(1, b"") + field(14, 1))
        self.digest = make_root(self.root, [broken, *standard_objects()[1:]])
        with self.assertRaises(convert.ConvertError):
            self.build()


class ValidatorTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        make_root(self.root)
        self.out = convert.build(self.root, DECLARED_IDS)
        set_keys(self.root, keys())
        install(self.root, self.out)
        self.shard = next(
            self.root / p
            for p in self.out
            if p.startswith("content/world/terrain/terrain-")
        )

    def errors(self) -> str:
        return "\n".join(validate.validate(self.root))

    def edit_shard(self, mutate) -> None:
        shard = json.loads(self.shard.read_text(encoding="utf-8"))
        mutate(shard)
        self.shard.write_bytes(canonical(shard))

    def test_valid(self) -> None:
        self.assertEqual(validate.validate(self.root), [])

    def test_class_must_follow_the_appearance(self) -> None:
        self.edit_shard(
            lambda s: s["records"][0]["declaration"].update({"class": "border"})
        )
        self.assertIn("facts differ from the client appearance", self.errors())

    def test_speed_must_follow_the_appearance(self) -> None:
        self.edit_shard(lambda s: s["records"][0]["declaration"].update({"speed": 1}))
        self.assertIn("facts differ from the client appearance", self.errors())

    def test_occurrences_must_match_the_map(self) -> None:
        self.edit_shard(
            lambda s: s["records"][0]["declaration"].update(
                {"occurrences_on_base_map": 9}
            )
        )
        self.assertIn("differ from the base map", self.errors())

    def test_key_must_spell_the_appearance_id(self) -> None:
        def mutate(shard):
            shard["records"][0]["declaration"]["identity"]["key"] = (
                "oteryn:terrain.a000999"
            )

        self.edit_shard(mutate)
        self.assertIn("differs from appearance id", self.errors())

    def test_palette_must_map_the_id_to_the_key(self) -> None:
        path = self.root / f"{PLACEMENTS}/index.json"
        index = json.loads(path.read_text(encoding="utf-8"))
        index["palette"][0].update(
            key=f"donor:crystalserver@00ce02a5:item/{GRASS}", provisional=True
        )
        path.write_bytes(canonical(index))
        self.assertIn("does not map id", self.errors())

    def test_terrain_key_in_the_palette_needs_a_record(self) -> None:
        set_keys(self.root, {DECLARED: convert.record_key(DECLARED)})
        self.assertIn("palette terrain keys differ from the records", self.errors())

    def test_stray_file_and_wrong_pin(self) -> None:
        (self.root / convert.DIRECTORY / "notes.txt").write_text("x")
        self.assertIn("files other than", self.errors())
        (self.root / convert.DIRECTORY / "notes.txt").unlink()
        index_path = self.root / validate.INDEX
        index = json.loads(index_path.read_text(encoding="utf-8"))
        index["source"]["files"][0]["sha256"] = "0" * 64
        index_path.write_bytes(canonical(index))
        self.assertIn("pinned client source differs", self.errors())

    def test_summary_must_match_the_records(self) -> None:
        path = self.root / convert.SUMMARY
        summary = json.loads(path.read_text(encoding="utf-8"))
        summary["classes"]["ground"]["records"] += 1
        path.write_bytes(canonical(summary))
        self.assertIn("differs from the records", self.errors())


if __name__ == "__main__":
    unittest.main()
