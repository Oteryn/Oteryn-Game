#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest

MODULE_PATH = Path(__file__).with_name("global_source_overlap.py")
spec = importlib.util.spec_from_file_location("global_source_overlap", MODULE_PATH)
mod = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(mod)


def fixtures():
    g1 = {
        "schema": mod.G1_SCHEMA,
        "sealed_lanes": [{"root_id": "items-protected", "stable_digest": mod.ITEM_STABLE_DIGEST}],
        "pages": [
            {"page_id": 10, "title": "Shared", "source_surfaces": ["A", "B"], "discovery_roots": ["root-a"], "discovery_kinds": ["page_links"], "source_shape": "STRUCTURED_PRIMARY", "revision_id": 41, "revision_timestamp": "2026-01-01T00:00:00Z", "candidate_families": ["Item"], "family_classification_state": "EXACT_FAMILY"},
            {"page_id": 11, "title": "Live only", "source_surfaces": ["C"], "discovery_roots": ["root-c"], "discovery_kinds": ["category"], "source_shape": "REDIRECT", "revision_id": 42, "revision_timestamp": "2026-01-02T00:00:00Z", "candidate_families": [], "family_classification_state": "SOURCE_CLASSIFICATION_UNRESOLVED"},
        ],
    }
    stable_g1 = dict(g1)
    g1m = {"schema": mod.G1_MANIFEST_SCHEMA,
           "full_output": {"sha256": mod.sha256(g1), "stable_without_retrieval_timestamp_sha256": mod.sha256(stable_g1)},
           "counts": {"live_unique_pages": 2}}
    cw = {"schema": mod.CROSSWALK_SCHEMA, "records": [
        {"page_id": 10, "title": "Shared", "source_shape": "INFOBOX_ITEM", "disposition": "EXACT_MATCH", "selected_native_key": "must-not-leak"},
        {"page_id": 12, "title": "Live only", "source_shape": "NO_INFOBOX_ITEM", "disposition": "AMBIGUOUS", "selected_native_key": "must-not-leak"},
    ]}
    cwm = {"schema": mod.CROSSWALK_MANIFEST_SCHEMA,
           "input_digests": {"wiki_first_census_stable_sha256": mod.ITEM_STABLE_DIGEST},
           "full_output": {"sha256": mod.sha256(cw)},
           "counts": {"source_pages": 2}}
    return g1, g1m, cw, cwm


class GlobalOverlapTest(unittest.TestCase):
    def test_exact_id_union_preserves_provenance_without_identity_promotion(self):
        result, manifest = mod.merge(*fixtures(), "g1-artifact", "item-artifact", expected_item_pages=2)
        self.assertEqual(result["counts"], {"g1_live_unique_pages": 2, "protected_item_pages": 2, "exact_id_overlap_pages": 1, "cross_lane_title_divergence_pages": 0, "global_unique_pages": 3})
        shared = next(p for p in result["pages"] if p["page_id"] == 10)
        self.assertEqual([p["lane"] for p in shared["provenance"]], ["G1_LIVE_NON_ITEM", "PROTECTED_ITEM"])
        self.assertEqual(shared["provenance"][1]["source_revision"]["state"], "UNKNOWN")
        self.assertFalse(shared["provenance"][1]["identity_crosswalk_conclusions_consumed"])
        self.assertEqual(shared["provenance"][0]["source_surfaces"], ["A", "B"])
        self.assertEqual(shared["provenance"][1]["row_role"], "PAGE_ID_AND_PROVENANCE_EVIDENCE_ONLY")
        self.assertNotIn("selected_native_key", str(result))
        self.assertNotIn("selected_native_key", str(result["pages"]))
        self.assertNotIn("EXACT_MATCH", str(result["pages"]))
        self.assertNotIn("AMBIGUOUS", str(result["pages"]))
        self.assertFalse(manifest["invariants"]["semantic_promotion_performed"])

    def test_duplicate_page_ids_fail_closed(self):
        g1, g1m, cw, cwm = fixtures()
        g1["pages"].append(dict(g1["pages"][0]))
        g1m["full_output"]["sha256"] = mod.sha256(g1)
        stable_g1 = dict(g1)
        g1m["full_output"]["stable_without_retrieval_timestamp_sha256"] = mod.sha256(stable_g1)
        with self.assertRaisesRegex(ValueError, "duplicate page_id"):
            mod.merge(g1, g1m, cw, cwm, "g1", "item", expected_item_pages=2)

    def test_cross_lane_title_divergence_is_preserved_as_observation(self):
        g1, g1m, cw, cwm = fixtures()
        cw["records"][0]["title"] = "Renamed in Item snapshot"
        cwm["full_output"]["sha256"] = mod.sha256(cw)
        result, _ = mod.merge(g1, g1m, cw, cwm, "g1", "item", expected_item_pages=2)
        shared = next(p for p in result["pages"] if p["page_id"] == 10)
        self.assertEqual(shared["title_observations"], [
            {"lane": "G1_LIVE_NON_ITEM", "title": "Shared"},
            {"lane": "PROTECTED_ITEM", "title": "Renamed in Item snapshot"},
        ])
        self.assertTrue(shared["cross_lane_title_divergence"])
        self.assertEqual({p["page_id"] for p in result["pages"]}, {10, 11, 12})
        self.assertEqual([p["page_id"] for p in result["pages"] if p["title_observations"][0]["title"] == "Live only"], [11, 12])
        self.assertEqual(len(result["overlaps"]), 1)
        self.assertTrue(result["overlaps"][0]["cross_lane_title_divergence"])

    def test_unlinked_protected_digest_fails_closed(self):
        g1, g1m, cw, cwm = fixtures()
        cwm["input_digests"]["wiki_first_census_stable_sha256"] = "wrong"
        with self.assertRaisesRegex(ValueError, "Item lane digest"):
            mod.merge(g1, g1m, cw, cwm, "g1", "item", expected_item_pages=2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
