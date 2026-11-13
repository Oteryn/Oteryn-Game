"""Authored overlays must not change the original map donor count pin."""

import copy
import json
import unittest

import convert_world_base as base


class DonorOverlayTotals(unittest.TestCase):
    def summary(self):
        path = (
            base.ROOT
            / "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
        )
        return json.loads(path.read_bytes())

    def test_merged_regions_reconstruct_the_original_donor_totals(self):
        summary = self.summary()
        self.assertEqual(
            base.world_otbm_totals(summary["totals"], summary), base.PINNED_TOTALS
        )

    def test_overlay_deductions_require_exact_source_proofs(self):
        summary = self.summary()
        for name in ("engclaim", "npcfix2_shared_sources"):
            with self.subTest(name=name):
                changed = copy.deepcopy(summary)
                changed["source"][name]["sha256"] = "0" * 64
                with self.assertRaises(ValueError):
                    base.world_otbm_totals(changed["totals"], changed)
        changed = copy.deepcopy(summary)
        changed["source"]["npcfix2_shared_sources"]["items"] += 1
        with self.assertRaises(ValueError):
            base.world_otbm_totals(changed["totals"], changed)


if __name__ == "__main__":
    unittest.main()
