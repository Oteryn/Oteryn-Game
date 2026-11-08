import json
import unittest
from pathlib import Path

import quest_access_bindings as access
import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]


class QuestAccessBindings(unittest.TestCase):
    def fixture(self, attrs=None):
        tiles = [
            (10, 20, 0, 0, (), [(0, 0, attrs), (1, 1, {"count": 3})]),
            (11, 20, 0, 0, (), [(1, 0, {"text": "preserved"})]),
        ]
        raw = codec.encode_region(7, 0, 0, {0: codec.encode_sector_tiles(0, 0, tiles)})
        palette = [{"source_item_id": 2772}, {"source_item_id": 100}]
        row = {
            "position": [10, 20, 7],
            "item": 2772,
            "attribute": "unique",
            "value": 30034,
        }
        return raw, palette, row

    def test_overlay_preserves_unowned_facts_and_is_idempotent(self):
        raw, palette, row = self.fixture({"text": "lever", "action": 200})
        changed, imported = access.overlay_region(raw, palette, [row])
        self.assertEqual(imported, {(10, 20, 7): "unique"})
        before = codec.decode_region(raw)[3][0][1]
        after = codec.decode_region(changed)[3][0][1]
        before[0][5][0][2]["unique"] = 30034
        self.assertEqual(before, after)
        self.assertEqual(access.overlay_region(changed, palette, [row])[0], changed)

    def test_conflicting_selector_fails_closed(self):
        raw, palette, row = self.fixture({"unique": 123})
        with self.assertRaisesRegex(ValueError, "selector conflict"):
            access.overlay_region(raw, palette, [row])

    def test_missing_source_tile_fails_closed(self):
        raw, palette, row = self.fixture()
        row["position"][0] = 12
        with self.assertRaisesRegex(ValueError, "missing quest access tiles"):
            access.overlay_region(raw, palette, [row])

    def outputs(self):
        raw, palette, row = self.fixture()
        path = "content/world/placements/region-z07-x000-y000.b3"
        return {
            access.INDEX: access.canonical(
                {"regions": [{"path": path}], "palette": palette, "source": {}}
            ),
            path: raw,
            access.SUMMARY: access.canonical(
                {"item_attributes": {"count": 1, "text": 1}, "bytes_on_disk": len(raw)}
            ),
        }, row

    def test_missing_declared_region_still_fails_completeness(self):
        outputs, row = self.outputs()
        missing = {**row, "position": [266, 20, 7]}
        with self.assertRaisesRegex(
            ValueError, "quest access capture did not bind completely.*266"
        ):
            access.overlay_outputs(
                outputs, access.canonical({"bindings": [row, missing]})
            )

    def test_explicit_capture_binds_every_declared_row(self):
        outputs, row = self.outputs()
        access.overlay_outputs(outputs, access.canonical({"bindings": [row]}))
        source = json.loads(outputs[access.INDEX])["source"]["quest_access"]
        self.assertEqual(source["attributes"], {"unique": 1, "action": 0})
        self.assertEqual(
            json.loads(outputs[access.SUMMARY])["item_attributes"],
            {"count": 1, "text": 1, "unique": 1},
        )
        before = outputs.copy()
        access.overlay_outputs(outputs, access.canonical({"bindings": [row]}))
        self.assertEqual(outputs, before)

    def test_committed_source_quest_access_selectors(self):
        index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
        capture = json.loads(
            (
                ROOT
                / "tools/content-schema/world-authoring/samples/quest-access-bindings.json"
            ).read_bytes()
        )
        for row in capture["bindings"]:
            x, y, z = row["position"]
            path = (
                ROOT
                / f"content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
            )
            found = []
            # Runtime startup projection preserves the binary source inputs.
            projected, imported = access.overlay_region(
                path.read_bytes(), index["palette"], [row]
            )
            self.assertEqual(len(imported), 1, row)
            for _, tiles in codec.decode_region(projected)[3]:
                for tx, ty, _, _, _, items in tiles:
                    if (tx, ty) == (x, y):
                        found.extend(
                            attrs
                            for palette, depth, attrs in items
                            if depth == 0
                            and index["palette"][palette]["source_item_id"]
                            == row["item"]
                        )
            self.assertEqual(len(found), 1, row)
            self.assertEqual((found[0] or {}).get(row["attribute"]), row["value"], row)


if __name__ == "__main__":
    unittest.main()
