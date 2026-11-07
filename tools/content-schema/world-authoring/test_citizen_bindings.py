"""Qualify shipped civic items, including the Svargrond roof entrance."""

import json
import unittest

import citizen_bindings as subject
import world_region_codec as codec


class CitizenBindingsTest(unittest.TestCase):
    def test_shipped_civic_items_carry_all_donor_startup_stamps(self):
        index = json.loads((subject.ROOT / subject.INDEX).read_bytes())
        document = json.loads((subject.ROOT / subject.CAPTURE).read_bytes())
        self.assertEqual(18, len(document["bindings"]))
        self.assertEqual(
            set(range(9056, 9069)) | {9240, 9500, 9510, 9515},
            {row["unique"] for row in document["bindings"] if "unique" in row},
        )
        for row in document["bindings"]:
            x, y, z = row["position"]
            path = (
                subject.ROOT
                / "content/world/placements"
                / codec.region_name(z, x // 256, y // 256)
            )
            _, _, _, sectors = codec.decode_region(path.read_bytes())
            matches = [
                (depth, attrs)
                for _, tiles in sectors
                for tx, ty, _, house, _, items in tiles
                if (tx, ty) == (x, y) and not house
                for palette, depth, attrs in items
                if index["palette"][palette]["source_item_id"] == row["item"]
            ]
            with self.subTest(position=row["position"]):
                self.assertEqual(1, len(matches))
                depth, attrs = matches[0]
                self.assertEqual(0, depth)
                attribute = "unique" if "unique" in row else "action"
                self.assertEqual(row[attribute], (attrs or {}).get(attribute))

    def test_shipped_overlay_is_current_and_idempotent(self):
        subject.apply(check=True)


if __name__ == "__main__":
    unittest.main()
