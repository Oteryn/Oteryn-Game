"""Tests for the floor-change converter and validator (WorldObject.FloorChange)."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import convert_floor_changes as convert
import convert_world_base as base
import convert_world_metadata as metadata
import validate_floor_changes as validate
import world_region_codec as codec
from convert_world_metadata import ConvertError, canonical

REGISTRY = "oteryn:item.registry.i{:08d}"
RAMP = "oteryn:item.decor.ramp"
XML = b"""<items>
<item id="100" name="stairs"><attribute key="floorchange" value="north"/></item>
<item fromid="200" toid="201" name="trapdoor">
  <attribute key="FloorChange" value="Down"/></item>
<item id="300" name="ramp"><attribute key="floorchange" value="eastalt"/></item>
<item id="301" name="ladder"><attribute key="description" value="scripted"/></item>
<item id="400" name="hole"><attribute key="floorchange" value="southalt"/></item>
<item id="401" name="steps"><attribute key="floorchange" value="west"/></item>
<item id="402" name="steps"><attribute key="floorchange" value="south"/></item>
<item id="403" name="steps"><attribute key="floorchange" value="east"/></item>
</items>"""
BINDINGS = json.dumps(
    {
        "bindings": [
            {
                "external_id": str(server_id),
                "identity_namespace": "ots/item_server_id",
                "target": {"key": key},
            }
            for server_id, key in [
                (100, REGISTRY.format(1)),
                (200, REGISTRY.format(2)),
                (300, RAMP),
                (5, REGISTRY.format(5)),
            ]
        ]
    }
).encode()
# Palette: 100, 200, an unrelated 5 and the provisional 400. 300 is bound but not on the map.
PALETTE = [
    {"key": REGISTRY.format(1), "provisional": False, "source_item_id": 100},
    {"key": REGISTRY.format(2), "provisional": False, "source_item_id": 200},
    {"key": REGISTRY.format(5), "provisional": False, "source_item_id": 5},
    {"key": base.donor_key(400), "provisional": True, "source_item_id": 400},
]


def write_definitions(
    root: Path, bindings: bytes, skip: set[str] = frozenset()
) -> None:
    """One Item definition record per binding target (the rule of A12 section 5)."""
    keys = sorted({r["target"]["key"] for r in json.loads(bindings)["bindings"]} - skip)
    path = root / "content/items/definitions/items-00000-00499.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    records = [{"definition": {"identity": {"key": key}}} for key in keys]
    path.write_text(json.dumps({"records": records}), encoding="utf-8")


def make_root(root: Path) -> None:
    """Twelve regions: enough for the parallel path. Palette 0 is seen twice at depth 0
    and once inside a container, palette 1 once, palette 3 twice."""
    tiles = {
        (7, 10, 10): [(0, 0, None), (2, 0, None)],
        (7, 300, 10): [(0, 0, None), (0, 1, None)],
        (7, 600, 10): [(1, 0, None)],
        (7, 900, 10): [(3, 0, None), (3, 0, None)],
    }
    for n in range(8):
        tiles[(8, 10 + 256 * n, 10)] = [(2, 0, None)]
    rows = []
    for (z, x, y), items in sorted(tiles.items()):
        rx, ry = x // 256, y // 256
        payload = codec.encode_sector_tiles(x // 32, y // 32, [(x, y, 0, 0, (), items)])
        local = ((y // 32) % 8) * 8 + (x // 32) % 8
        path = f"{base.DIRECTORY}/{codec.region_name(z, rx, ry)}"
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(codec.encode_region(z, rx, ry, {local: payload}))
        rows.append({"path": path})
    index = {"palette": PALETTE, "regions": rows}
    (root / convert.PLACEMENTS_INDEX).write_bytes(canonical(index))
    bindings = root / base.ITEM_BINDINGS.relative_to(convert.ROOT)
    bindings.parent.mkdir(parents=True, exist_ok=True)
    bindings.write_bytes(BINDINGS)
    write_definitions(root, BINDINGS)


def write_family(root: Path, xml: bytes = XML) -> dict[str, bytes]:
    out = convert.build(xml, root, BINDINGS)
    for path, data in out.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)
    return out


class FloorChangeTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        make_root(self.root)
        self.out = write_family(self.root)
        self.shard_path = next(p for p in self.out if "floor-changes-0" in p)

    def shard(self) -> dict:
        return json.loads((self.root / self.shard_path).read_text())

    def write_shard(self, shard: dict) -> None:
        (self.root / self.shard_path).write_bytes(canonical(shard))

    def errors(self) -> list[str]:
        return validate.validate(self.root)

    def has(self, text: str) -> bool:
        return any(text in error for error in self.errors())

    def declarations(self) -> dict[int, dict]:
        return {
            r["declaration"]["source_item_id"]: r["declaration"]
            for r in self.shard()["records"]
        }

    def test_fixture_converts_to_a_valid_family(self):
        self.assertEqual(self.errors(), [])
        by_id = self.declarations()
        self.assertEqual(sorted(by_id), [100, 200, 201, 300, 400, 401, 402, 403])
        self.assertNotIn(301, by_id)
        self.assertEqual(
            {i: d["floor_change"] for i, d in by_id.items()},
            {
                100: "up_north",
                200: "down",
                201: "down",
                300: "up_east_alt",
                400: "up_south_alt",
                401: "up_west",
                402: "up_south",
                403: "up_east",
            },
        )
        self.assertEqual(by_id[100]["name"], "stairs")
        self.assertEqual(
            by_id[100]["identity"]["key"],
            "oteryn:world_object.floor_change.registry_i00000001",
        )
        self.assertEqual(
            by_id[300]["identity"]["key"], "oteryn:world_object.floor_change.decor_ramp"
        )

    def test_unbound_ids_take_the_provisional_donor_key(self):
        by_id = self.declarations()
        for server_id in (201, 400, 401):
            self.assertTrue(by_id[server_id]["provisional_item"])
            self.assertEqual(by_id[server_id]["item"]["key"], base.donor_key(server_id))
        self.assertEqual(
            by_id[201]["identity"]["key"], "oteryn:world_object.floor_change.donor_201"
        )
        self.assertFalse(by_id[100]["provisional_item"])

    def test_a_binding_to_an_undefined_item_key_is_provisional(self):
        defined = {REGISTRY.format(1), REGISTRY.format(5), RAMP}
        out = convert.build(XML, self.root, BINDINGS, defined=defined)
        shard = json.loads(out[self.shard_path])
        row = next(
            r for r in shard["records"] if r["declaration"]["source_item_id"] == 200
        )
        self.assertTrue(row["declaration"]["provisional_item"])
        self.assertEqual(row["declaration"]["item"]["key"], base.donor_key(200))

    def test_validator_rejects_an_item_key_without_a_definition(self):
        write_definitions(self.root, BINDINGS, skip={REGISTRY.format(1)})
        self.assertTrue(self.has("unbound id must use the provisional donor key"))

    def test_occurrences_count_top_level_tile_items_only(self):
        by_id = self.declarations()
        self.assertEqual(by_id[100]["occurrences_on_base_map"], 2)
        self.assertEqual(by_id[200]["occurrences_on_base_map"], 1)
        self.assertEqual(by_id[400]["occurrences_on_base_map"], 2)
        for server_id in (201, 300, 401, 402, 403):
            self.assertEqual(by_id[server_id]["occurrences_on_base_map"], 0)
        summary = json.loads((self.root / validate.SUMMARY).read_text())
        self.assertEqual(summary["item_types"], 8)
        self.assertEqual(summary["item_types_on_base_map"], 3)
        self.assertEqual(summary["occurrences_on_base_map"], 5)
        self.assertEqual(summary["provisional_items"], 5)
        self.assertEqual(summary["kinds"]["down"], {"item_types": 2, "occurrences": 1})
        self.assertEqual(summary["kinds"]["up_west"]["item_types"], 1)
        self.assertEqual(
            [row["kind"] for row in summary["excluded"]],
            ["ladder_up", "rope_spot", "sewer_grate", "shovel_or_pick_hole"],
        )

    def test_parallel_count_equals_serial_count(self):
        wanted = {0, 1, 3}
        self.assertEqual(
            convert.count_tile_items(self.root, wanted, 1),
            convert.count_tile_items(self.root, wanted, 2),
        )

    def test_conversion_is_deterministic(self):
        self.assertEqual(self.out, convert.build(XML, self.root, BINDINGS))

    def test_source_fails_closed(self):
        cases = {
            b'<items><item id="1" name="a"><attribute key="floorchange" value="up"/></item></items>': "unknown",
            b'<items><item id="1" name="a"><attribute key="floorchange" value="down"/>'
            b'<attribute key="floorchange" value="north"/></item></items>': "one floorchange",
            b'<items><item id="1"><attribute key="floorchange" value="down"/></item></items>': "a name",
            b'<items><item id="1" name="a"><attribute key="floorchange" value="down"/></item>'
            b'<item id="1" name="b"/></items>': "twice",
            b'<items><item id="1" name="a"/></items>': "no floorchange",
        }
        for xml, text in cases.items():
            with self.subTest(text=text), self.assertRaises(ConvertError) as caught:
                convert.build(xml, self.root, BINDINGS)
            self.assertIn(text, str(caught.exception))

    def test_record_key_rules(self):
        self.assertEqual(
            convert.record_key("oteryn:item.currency.gold_coin"),
            "oteryn:world_object.floor_change.currency_gold_coin",
        )
        with self.assertRaises(ConvertError):
            convert.record_key("other:item.x")

    def test_missing_base_map_fails_closed(self):
        (self.root / convert.PLACEMENTS_INDEX).unlink()
        with self.assertRaises(ConvertError):
            convert.build(XML, self.root, BINDINGS)

    def test_tampered_records_are_rejected(self):
        def edit(mutate):
            shard = self.shard()
            mutate(shard["records"])
            self.write_shard(shard)

        cases = [
            (
                lambda r: next(
                    x for x in r if x["declaration"]["source_item_id"] == 100
                )["declaration"].update(occurrences_on_base_map=99),
                "differ from the base map",
            ),
            (lambda r: r[0]["declaration"].update(floor_change="up"), "schema"),
            (
                lambda r: r[0]["declaration"]["identity"].update(
                    key="oteryn:world_object.floor_change.x"
                ),
                "does not derive",
            ),
            (
                lambda r: r[0]["declaration"].update(provisional_item=True),
                "differs from its binding",
            ),
            (
                lambda r: r[0]["declaration"]["item"].update(key=REGISTRY.format(9)),
                "differs from its binding",
            ),
            (
                lambda r: r[0]["declaration"].update(source_item_id=7),
                "binding is not the items.xml id",
            ),
            (lambda r: r.reverse(), "sorted"),
            (
                lambda r: r[1]["declaration"].update(
                    source_item_id=r[0]["declaration"]["source_item_id"]
                ),
                "unique",
            ),
        ]
        for mutate, text in cases:
            with self.subTest(text=text):
                self.write_shard(
                    json.loads(canonical(json.loads(self.out[self.shard_path])))
                )
                edit(mutate)
                self.assertTrue(self.has(text), self.errors())
        self.write_shard(json.loads(self.out[self.shard_path]))
        self.assertEqual(self.errors(), [])

    def test_unbound_id_must_use_the_donor_key_and_palette_keys_must_agree(self):
        shard = self.shard()
        row = next(
            r for r in shard["records"] if r["declaration"]["source_item_id"] == 400
        )
        row["declaration"]["item"]["key"] = REGISTRY.format(400)
        self.write_shard(shard)
        self.assertTrue(self.has("provisional donor key"))
        write_family(self.root)
        index = json.loads((self.root / convert.PLACEMENTS_INDEX).read_text())
        index["palette"][0]["key"] = REGISTRY.format(77)
        (self.root / convert.PLACEMENTS_INDEX).write_bytes(canonical(index))
        self.assertTrue(self.has("base map palette key"))

    def test_stale_bindings_stray_files_and_summary_are_rejected(self):
        bindings = self.root / base.ITEM_BINDINGS.relative_to(convert.ROOT)
        bindings.write_bytes(BINDINGS + b" ")
        self.assertTrue(self.has("digest is stale"))
        bindings.write_bytes(BINDINGS)
        (self.root / convert.DIRECTORY / "extra.json").write_text("{}")
        self.assertTrue(self.has("files other than"))
        (self.root / convert.DIRECTORY / "extra.json").unlink()
        summary = self.root / validate.SUMMARY
        data = json.loads(summary.read_text())
        data["item_types"] += 1
        summary.write_bytes(canonical(data))
        self.assertTrue(self.has("differs from the records"))
        write_family(self.root)
        path = self.root / self.shard_path
        path.write_text(path.read_text() + " ")
        self.assertTrue(self.has("not canonical"))

    def test_the_marker_shape_is_not_a_populated_family(self):
        (self.root / convert.DIRECTORY / "index.json").write_text(
            json.dumps({"schema": "OTERYN_GAME_TREE_DIRECTORY/v1"})
        )
        self.assertTrue(self.has("schema"))

    def test_every_engine_value_is_mapped_once(self):
        self.assertEqual(
            sorted(convert.FLOOR_CHANGE),
            ["down", "east", "eastalt", "north", "south", "southalt", "west"],
        )
        self.assertEqual(len(set(convert.FLOOR_CHANGE.values())), 7)
        self.assertEqual(metadata.SOURCE["revision"], convert.SOURCE["revision"])


if __name__ == "__main__":
    unittest.main()
