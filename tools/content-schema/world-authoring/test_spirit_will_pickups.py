import hashlib
import json
import unittest

import spirit_will_pickups as spirit


class SpiritPickupOverlay(unittest.TestCase):
    def setUp(self):
        appearances = sorted(
            {
                appearance
                for _, _, _, base, pickup in spirit.PICKUPS
                for appearance in (*base, pickup)
            }
        )
        indices = {appearance: index for index, appearance in enumerate(appearances)}
        self.before = {}
        self.after = {}
        self.rows = []
        for x, y, z, base, item in spirit.PICKUPS:
            rx, ry = x // 256, y // 256
            path = "content/world/placements/" + spirit.codec.region_name(z, rx, ry)

            def encode(stack, x=x, y=y, z=z, rx=rx, ry=ry):
                tiles = [
                    (
                        x,
                        y,
                        0,
                        0,
                        (),
                        [(indices[appearance], 0, None) for appearance in stack],
                    )
                ]
                local = (y // 32 % 8) * 8 + x // 32 % 8
                sector = spirit.codec.encode_sector_tiles(x // 32, y // 32, tiles)
                return spirit.codec.encode_region(z, rx, ry, {local: sector})

            self.before[path] = encode(base)
            self.after[path] = encode((*base, item))
            self.rows.append(
                {
                    "path": path,
                    "items": len(base),
                    "sha256": hashlib.sha256(self.before[path]).hexdigest(),
                }
            )
        self.items = sum(row["items"] for row in self.rows)
        self.inputs = {
            spirit.world.INDEX: spirit.world.canonical(
                {
                    "source": {},
                    "palette": [
                        {"source_item_id": appearance} for appearance in appearances
                    ],
                    "regions": self.rows,
                    "totals": {"items": self.items},
                }
            ),
            spirit.world.SUMMARY: spirit.world.canonical(
                {
                    "bytes_on_disk": sum(map(len, self.before.values())),
                    "palette": {"families": {"item": {"occurrences": self.items}}},
                }
            ),
        }

    def test_adopts_all_three_placed_regions_and_reconciles_metadata(self):
        direct = {**self.inputs, **self.before}
        adopted = {**self.inputs, **self.after}
        spirit.overlay_outputs(direct)
        spirit.overlay_outputs(adopted)
        self.assertEqual(adopted, direct)
        index = json.loads(adopted[spirit.world.INDEX])
        for row in index["regions"]:
            self.assertEqual(
                row["sha256"], hashlib.sha256(self.after[row["path"]]).hexdigest()
            )
        summary = json.loads(adopted[spirit.world.SUMMARY])
        self.assertEqual(summary["bytes_on_disk"], sum(map(len, self.after.values())))
        self.assertEqual(index["totals"]["items"], self.items + len(self.after))
        spirit.overlay_outputs(adopted)
        self.assertEqual(adopted, direct)

    def test_refuses_a_region_with_an_unrelated_predecessor_hash(self):
        index = json.loads(self.inputs[spirit.world.INDEX])
        index["regions"][0]["sha256"] = "0" * 64
        outputs = {
            **self.inputs,
            **self.after,
            spirit.world.INDEX: spirit.world.canonical(index),
        }
        with self.assertRaisesRegex(ValueError, "Spirit pickup region"):
            spirit.overlay_outputs(outputs)


if __name__ == "__main__":
    unittest.main()
