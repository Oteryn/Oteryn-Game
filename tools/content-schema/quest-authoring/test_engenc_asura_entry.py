"""The native entry producer rejects a changed physical source."""

import unittest
from unittest.mock import patch

import engenc_asura_entry as entry


class AsuraEntryTests(unittest.TestCase):
    def test_entry_uses_canonical_area_and_live_premium(self):
        binding = entry.entry_binding()
        stage = binding["quests"][0]["stages"][0]
        callback = stage["trigger"]["callback"]
        self.assertEqual(callback["admission"], {"premium": True})
        self.assertEqual(
            callback["selector"]["native"],
            {
                "position": {"x": 32953, "y": 32685, "floor": -7},
                "ordinal": 0,
                "appearance": 923,
            },
        )
        self.assertFalse(stage["completes"])

    def test_explicit_entry_is_the_only_owner_across_the_complete_callback_registry(self):
        import qengine_access
        binding = entry.entry_binding()
        callback = binding["quests"][0]["stages"][0]["trigger"]["callback"]
        callbacks = qengine_access.document()["callbacks"] + [callback]
        peers = [row for row in callbacks
                 if (row["trigger"], row["selector"]) == (callback["trigger"], callback["selector"])]
        self.assertEqual(peers, [callback])

    def test_missing_or_changed_native_tile_refuses_authoring(self):
        for tiles in [[], [(32953, 32685, 0, 0, (), [])]]:
            with (
                self.subTest(tiles=tiles),
                patch.object(
                    entry.codec,
                    "decode_region",
                    return_value=(7, 128, 127, [(0, tiles)]),
                ),
                self.assertRaisesRegex(ValueError, "plain native ground tile"),
            ):
                entry.entry_binding()

    def test_replaced_ground_is_not_silently_rebound(self):
        palette = entry.read(entry.ROOT, "content/world/placements/index.json")[
            "palette"
        ]
        wrong = next(
            i for i, item in enumerate(palette) if item["source_item_id"] != 923
        )
        tiles = [(32953, 32685, 0, 0, (), [(wrong, 0, None)])]
        with (
            patch.object(
                entry.codec, "decode_region", return_value=(7, 128, 127, [(0, tiles)])
            ),
            self.assertRaisesRegex(ValueError, "appearance/ordinal changed"),
        ):
            entry.entry_binding()


if __name__ == "__main__":
    unittest.main()
