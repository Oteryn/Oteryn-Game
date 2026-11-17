"""W07 source geometry must retain exact admitted appearance identities and flags."""

import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import soulpit_static
import world_objects


class SoulpitStaticTests(unittest.TestCase):
    def test_repository_soulpit_entries_and_wave_anchor_resolve_real_static_tiles(self):
        root = soulpit_static.ROOT
        index = json.loads((root / "content/world/placements/index.json").read_bytes())
        palette = index["palette"]
        manifest = json.loads(
            (root / "imports/official/client-assets/15.30/manifest.json").read_bytes()
        )
        file = next(
            row
            for row in manifest["files"]
            if row["name"].startswith("appearances-") and row["name"].endswith(".dat")
        )
        appearances = soulpit_static.read_appearances(
            (root / "content/assets/files" / file["name"]).read_bytes()
        )
        required = {(32375, 31158 + slot) for slot in range(5)}
        required |= {(32373 + slot, 31151) for slot in range(5)}
        required |= {(32373, 31158), (32376, 31144)}
        found = set()
        for _, tiles in soulpit_static.world_region_codec.decode_region(
            (root / soulpit_static.REGION).read_bytes()
        )[3]:
            for x, y, _, _, _, items in tiles:
                if not (32362 <= x <= 32390 and 31132 <= y <= 31162):
                    continue
                for slot, _, _ in items:
                    self.assertFalse(
                        palette[slot]["provisional"], (x, y, palette[slot])
                    )
                if (x, y) in required:
                    flags = [
                        appearances[palette[slot]["source_item_id"]].flags
                        for slot, depth, _ in items
                        if depth == 0
                    ]
                    self.assertTrue(any("bank" in flag for flag in flags), (x, y))
                    self.assertFalse(any("unpass" in flag for flag in flags), (x, y))
                    found.add((x, y))
        self.assertEqual(found, required)

    def test_ground_border_and_static_blocker_have_independent_physical_flags(self):
        rows = {
            row["provenance"]["source_item_id"]: row for row in soulpit_static.records()
        }
        self.assertEqual(set(rows), soulpit_static.IDS)
        self.assertEqual(rows[46631]["kind"]["value"], "ground")
        self.assertEqual(rows[46631]["ground_speed"]["value"], 150)
        self.assertTrue(rows[46631]["walkable"]["value"])
        self.assertEqual(rows[34951]["kind"]["value"], "border")
        self.assertTrue(rows[34951]["walkable"]["value"])
        self.assertEqual(rows[42893]["kind"]["value"], "common")
        self.assertFalse(rows[42893]["walkable"]["value"])
        self.assertFalse(rows[43516]["walkable"]["value"])
        for row in rows.values():
            self.assertNotIn("item_pointer", row["provenance"])
            self.assertEqual(world_objects.validate_record(row), [])
        altered = copy.deepcopy(rows[42893])
        altered["walkable"]["value"] = True
        self.assertTrue(world_objects.validate_record(altered))

    def test_generated_static_content_preserves_palette_positions(self):
        generated = soulpit_static.outputs()
        old = json.loads(
            (soulpit_static.ROOT / "content/world/placements/index.json").read_bytes()
        )
        new = json.loads(generated["content/world/placements/index.json"])
        self.assertEqual(
            [row["source_item_id"] for row in old["palette"]],
            [row["source_item_id"] for row in new["palette"]],
        )
        digest = hashlib.sha256(
            generated["content/world/placements/index.json"]
        ).hexdigest()
        self.assertIn(digest.encode(), generated["crates/client-assets/src/world.rs"])
        pin_source = generated["crates/client-assets/src/world.rs"].decode()
        for invalid_source in ["", pin_source + "\n" + pin_source]:
            with (
                patch.object(
                    soulpit_static.Path, "read_text", return_value=invalid_source
                ),
                self.assertRaisesRegex(
                    ValueError, "SOULPIT_CLIENT_PLACEMENT_PIN_COUNT"
                ),
            ):
                soulpit_static.outputs()
        for path, raw in generated.items():
            self.assertEqual((soulpit_static.ROOT / path).read_bytes(), raw, path)


if __name__ == "__main__":
    unittest.main()
