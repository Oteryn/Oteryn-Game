#!/usr/bin/env python3
"""Synthetic fail-closed tests for the G4 non-Item provenance capture."""
from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("g4_non_item_source_capture", HERE / "g4_non_item_source_capture.py")
if spec is None or spec.loader is None:
    raise RuntimeError("G4 collector import failed")
g4 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(g4)


class FakeClient:
    def __init__(self, responses):
        self.responses = list(responses)
        self.calls = []

    def get_json(self, params):
        self.calls.append(params)
        if not self.responses:
            raise AssertionError("unexpected API call")
        return self.responses.pop(0)


def api_page(page_id=7, title="A page", revid=70, timestamp="2026-09-23T01:02:03Z", content="á raw wiki"):
    return {"query": {"pages": [{
        "pageid": page_id,
        "title": title,
        "revisions": [{"revid": revid, "timestamp": timestamp, "slots": {"main": {"content": content}}}],
    }]}}


def fixture(rows=None):
    full = {
        "schema": "OTERYN_FULL_CONTENT_SOURCE_UNIVERSE/v1",
        "source": {"id": "TIBIAWIKI_STRUCTURED", "role": "STRUCTURED_REFERENCE_DATA", "api": "https://example.invalid/api.php"},
        "source_snapshot_sha256": "a" * 64,
        "authority": {"gameplay_truth": "NONE"},
        "pages": rows if rows is not None else [{
            "page_id": 7, "title": "A page", "revision_id": 70, "revision_timestamp": "2026-09-23T01:02:03Z",
            "candidate_families": ["Area", "WorldPlacement"], "discovery_roots": ["geography"], "source_surfaces": ["Map"],
        }],
    }
    manifest = {
        "schema": "OTERYN_FULL_CONTENT_SOURCE_CENSUS_MANIFEST/v1",
        "status": "G1_SOURCE_DISCOVERY_ONLY_NO_IDENTITY_PROMOTION",
        "counts": {"live_unique_pages": len(full["pages"])},
        "invariants": {
            "protected_item_full_census_refetched": False,
            "live_page_ids_deduplicated_before_counts": True,
            "exact_revisions_reverified": True,
            "hard_exclusions_absent": True,
            "identity_resolution_performed": False,
            "identity_minting_performed": False,
            "semantic_promotion_performed": False,
            "worldproject_population_performed": False,
            "raw_long_form_prose_collected": False,
            "assets_collected": False,
        },
        "full_output": {"sha256": g4.sha256_bytes(g4.canonical_bytes(full))},
    }
    return full, manifest


def g2_fixture(first_id=10000):
    pages = [{"page_id": page_id, "provenance": [{"lane": "PROTECTED_ITEM"}]}
             for page_id in range(first_id, first_id + g4.EXPECTED_ITEM_PAGES)]
    full = {
        "schema": g4.G2_SCHEMA,
        "identity_basis": "EXACT_MEDIAWIKI_PAGE_ID",
        "pages": pages,
    }
    manifest = {
        "schema": g4.G2_MANIFEST_SCHEMA,
        "global_universe_sha256": g4.sha256_bytes(g4.canonical_bytes(full)),
        "input_digests": {"protected_item_stable_digest": g4.ITEM_STABLE_DIGEST},
        "counts": {"protected_item_pages": g4.EXPECTED_ITEM_PAGES, "global_unique_pages": len(pages)},
        "invariants": {"exact_page_id_only": True, "semantic_promotion_performed": False, "canonical_identity_selection_performed": False},
    }
    return full, manifest


def expect_error(code, fn):
    try:
        fn()
    except g4.CaptureError as exc:
        assert code in str(exc), (code, str(exc))
    else:
        raise AssertionError(f"expected {code}")


def run():
    full, manifest = fixture()
    g2, g2_manifest = g2_fixture()
    api = FakeClient([api_page()])
    artifact, result_manifest = g4.capture(full, manifest, g2, g2_manifest, api, retrieval_timestamp="2026-09-24T00:00:00Z")
    page = artifact["pages"][0]
    assert page["raw_utf8_sha256"] == hashlib.sha256("á raw wiki".encode("utf-8")).hexdigest()
    assert set(page) == g4.PAGE_FIELDS | {"raw_utf8_sha256"}
    assert result_manifest["invariants"]["raw_content_retained"] is False
    assert result_manifest["invariants"]["candidate_families_are_not_assignments"] is True
    assert artifact["authority"]["second_wiki"] == "UNKNOWN"
    assert artifact["authority"]["ots_hypothesis"] == "UNVERIFIED_HYPOTHESIS_ONLY"
    assert result_manifest["counts"]["protected_item_pages"] == 6918
    assert result_manifest["counts"]["g1_item_id_overlap_pages"] == 0
    assert api.calls[0]["revids"] == "70"
    assert "pageids" not in api.calls[0]

    duplicate = full["pages"][0].copy()
    duplicate["candidate_families"] = ["Quest"]
    duplicate["discovery_roots"] = ["quests"]
    duplicate["source_surfaces"] = ["Quest list"]
    merged = g4.deduplicate_pages(full["pages"] + [duplicate])
    assert len(merged) == 1
    assert merged[0]["candidate_families"] == ["Area", "Quest", "WorldPlacement"]
    assert merged[0]["discovery_roots"] == ["geography", "quests"]
    conflict = duplicate | {"revision_id": 71}
    expect_error("DUPLICATE_PAGE_SNAPSHOT_CONFLICT", lambda: g4.deduplicate_pages(full["pages"] + [conflict]))

    drift_client = FakeClient([api_page(timestamp="2026-09-24T01:02:03Z")])
    expect_error("SOURCE_SNAPSHOT_DRIFT", lambda: g4.fetch_exact_revisions(drift_client, [full["pages"][0]]))
    malformed = FakeClient([{"continue": {"rvcontinue": "x"}, "query": {"pages": []}}])
    expect_error("CONTENT_CONTINUATION_UNEXPECTED", lambda: g4.fetch_exact_revisions(malformed, [full["pages"][0]]))
    expect_error("G1_FULL_OUTPUT_DIGEST_MISMATCH", lambda: g4.verify_g1(full, manifest | {"full_output": {"sha256": "0" * 64}}))
    expect_error("G1_ITEM_LANE_NOT_SEALED", lambda: g4.verify_g1(full, manifest | {"invariants": {"protected_item_full_census_refetched": True, "live_page_ids_deduplicated_before_counts": True}}))

    overlap_g2, overlap_manifest = g2_fixture(first_id=7)
    overlap_full, overlap_g1_manifest = fixture()
    overlap_api = FakeClient([])
    overlap_artifact, overlap_evidence = g4.capture(overlap_full, overlap_g1_manifest, overlap_g2, overlap_manifest, overlap_api, retrieval_timestamp="2026-09-24T00:00:00Z")
    assert overlap_artifact["pages"] == []
    assert overlap_evidence["counts"]["g1_item_id_overlap_pages"] == 1
    assert overlap_evidence["g1_item_id_overlap_page_ids"] == [7]
    expect_error("G2_FULL_OUTPUT_DIGEST_MISMATCH", lambda: g4.protected_item_page_ids(overlap_g2, overlap_manifest | {"global_universe_sha256": "0" * 64}))

    print("g4-non-item-source-capture-self-test: PASS")


if __name__ == "__main__":
    run()
