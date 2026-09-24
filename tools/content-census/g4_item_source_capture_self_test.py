#!/usr/bin/env python3
from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("g4_item_source_capture", HERE / "g4_item_source_capture.py")
assert SPEC is not None and SPEC.loader is not None
capture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(capture)


def fixture():
    pages = [
        {"source": capture.SOURCE_ID, "source_role": capture.SOURCE_ROLE, "page_id": 9, "title": "Axe", "revision_id": 109, "revision_timestamp": "2026-09-23T00:00:00Z", "retrieval_timestamp": "2026-09-23T20:00:00Z", "source_digest": "a" * 64, "source_shape": "NO_INFOBOX_ITEM", "normalized_fields": {}, "unmapped_infobox_fields": {}, "infobox_present": False},
        {"source": capture.SOURCE_ID, "source_role": capture.SOURCE_ROLE, "page_id": 7, "title": "Book of Tests", "revision_id": 107, "revision_timestamp": "2026-09-22T00:00:00Z", "retrieval_timestamp": "2026-09-23T20:00:00Z", "source_digest": "b" * 64, "source_shape": "INFOBOX_ITEM_PARSE_ERROR", "normalized_fields": {}, "unmapped_infobox_fields": {}, "infobox_present": True, "source_parse_error": "DUPLICATE_FIELD_CONFLICT"},
    ]
    census = {
        "schema": capture.CENSUS_SCHEMA,
        "collector_profile": "synthetic",
        "source": {"id": capture.SOURCE_ID, "role": capture.SOURCE_ROLE, "api": capture.SOURCE_API,
                   "discovery": {"kind": "MEDIAWIKI_CATEGORYMEMBERS", "category": "Categoria:Itens", "namespace": 0}},
        "retrieval_timestamp": "2026-09-23T20:00:00Z",
        "authority": {"gameplay_truth": "NONE", "identity_minting": "FORBIDDEN"},
        "pages": pages,
        "counts": {"discovered_pages": 2, "fetched_pages": 2, "pages_with_infobox": 1, "pages_without_infobox": 1,
                   "infobox_parse_errors": 1, "source_shapes": {"INFOBOX_ITEM": 0, "INFOBOX_ITEM_PARSE_ERROR": 1, "NO_INFOBOX_ITEM": 1},
                   "distinct_infobox_fields": 0, "mapped_field_occurrences": 0, "unmapped_field_occurrences": 0, "top_fields": []},
    }
    source_manifest = {
        "schema": capture.MANIFEST_SCHEMA, "status": "WIKI_FIRST_SOURCE_EVIDENCE_ONLY_NO_IDENTITY_PROMOTION",
        "counts": copy.deepcopy(census["counts"]),
        "full_output": {"schema": capture.CENSUS_SCHEMA},
        "invariants": {"wiki_first_discovery": True, "starts_from_crystal_38157": False,
                       "identity_resolution_performed": False, "semantic_promotion_performed": False,
                       "raw_long_form_prose_collected": False, "raw_wikitext_committed": False},
    }
    stable = dict(census)
    stable.pop("retrieval_timestamp")
    source_manifest["full_output"].update({
        "sha256": hashlib.sha256(capture.canonical_bytes(census)).hexdigest(),
        "stable_without_retrieval_timestamp_sha256": hashlib.sha256(capture.canonical_bytes(stable)).hexdigest(),
    })
    return census, source_manifest


def rejects(fn, code):
    try:
        fn()
    except capture.CaptureError as exc:
        assert code in str(exc), (code, str(exc))
    else:
        raise AssertionError(f"expected {code}")


def refresh_manifest(census):
    manifest = fixture()[1]
    stable = dict(census)
    stable.pop("retrieval_timestamp", None)
    manifest["counts"] = copy.deepcopy(census["counts"])
    manifest["full_output"]["sha256"] = hashlib.sha256(capture.canonical_bytes(census)).hexdigest()
    manifest["full_output"]["stable_without_retrieval_timestamp_sha256"] = hashlib.sha256(capture.canonical_bytes(stable)).hexdigest()
    return manifest


def run():
    census, manifest = fixture()
    artifact, out_manifest = capture.transform(census, manifest)
    assert artifact["source_page_count"] == 2
    assert [r["external_id"] for r in artifact["rows"]] == ["7", "9"]
    assert artifact["rows"][0]["page_key"] == "mediawiki/tibiawiki.com.br/page_id/7"
    assert artifact["rows"][0]["source_digest"] == "b" * 64
    assert set(artifact["rows"][0]) == set(out_manifest["row_schema_fields"])
    serialized = json.dumps(artifact, ensure_ascii=False)
    assert "DUPLICATE_FIELD_CONFLICT" not in serialized
    assert "normalized_fields" not in serialized and "unmapped_infobox_fields" not in serialized
    assert out_manifest["invariants"]["second_wiki_corroboration"] == "UNKNOWN"
    assert out_manifest["invariants"]["ots_identity_or_semantics"] == "HYPOTHESIS_ONLY"

    bad = copy.deepcopy(census)
    bad["pages"].append(copy.deepcopy(bad["pages"][0]))
    rejects(lambda: capture.transform(bad, refresh_manifest(bad)), "DUPLICATE_PAGE_ID")
    bad = copy.deepcopy(census)
    bad["pages"][1]["page_id"] = bad["pages"][0]["page_id"]
    rejects(lambda: capture.transform(bad, refresh_manifest(bad)), "DUPLICATE_PAGE_ID_CONFLICT")
    for field, value, code in (("revision_id", None, "REVISION_ID_INVALID"),
                               ("revision_timestamp", "unknown", "REVISION_TIMESTAMP_INVALID"),
                               ("source_digest", "x", "SOURCE_DIGEST_INVALID")):
        bad = copy.deepcopy(census)
        bad["pages"][0][field] = value
        rejects(lambda b=bad: capture.transform(b, refresh_manifest(b)), code)
    bad = copy.deepcopy(census)
    bad["counts"]["discovered_pages"] = 3
    rejects(lambda: capture.transform(bad, manifest), "CENSUS_MANIFEST_COUNTS_MISMATCH")
    bad = copy.deepcopy(manifest)
    bad["full_output"]["sha256"] = "0" * 64
    rejects(lambda: capture.transform(census, bad), "CENSUS_FULL_DIGEST_MISMATCH")
    print("g4 item source capture self-test: PASS")


if __name__ == "__main__":
    run()
