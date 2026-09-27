#!/usr/bin/env python3
"""Synthetic fail-closed tests for the direct non-Item family crosswalk."""
from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("g4_direct_nonitem_family_crosswalk", HERE / "g4_direct_nonitem_family_crosswalk.py")
if spec is None or spec.loader is None:
    raise RuntimeError("G4 direct non-Item crosswalk import failed")
crosswalk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(crosswalk)


def direct_row(page_id: int, family: str, revision_id: int, timestamp: str) -> dict:
    signature = crosswalk.FAMILIES[family]["signature"]
    return {
        "page_id": page_id,
        "source_family_classification": {
            "primary_definition_family": family,
            "state": "SOURCE_DEFINITION_PRIMARY",
            "basis_signature_ids": [signature],
            "assignment_rule": signature,
            "target_identity_selected": False,
            "candidate_relations_resolved": False,
        },
        "provenance": [{
            "lane": "G1_LIVE_NON_ITEM",
            "source": crosswalk.SOURCE_KEY,
            "revision_id": revision_id,
            "revision_timestamp": timestamp,
        }],
    }


def g4_page(page_id: int, revision_id: int, timestamp: str, title: str) -> dict:
    external_id = str(page_id)
    return {
        "page_id": page_id,
        "title": title,
        "revision_id": revision_id,
        "revision_timestamp": timestamp,
        "source": crosswalk.SOURCE_KEY,
        "source_role": "STRUCTURED_REFERENCE_DATA",
        "source_namespace": crosswalk.SOURCE_NAMESPACE,
        "external_id": external_id,
        "page_key": f"{crosswalk.SOURCE_NAMESPACE}/page_id/{external_id}",
        "raw_utf8_sha256": hashlib.sha256(f"raw-{page_id}".encode()).hexdigest(),
    }


def fake_inputs(revision_drift: set[str] | None = None):
    revision_drift = revision_drift or set()
    families = list(crosswalk.FAMILIES)
    pages_g3 = []
    pages_g4 = []
    for index, family in enumerate(families, start=1):
        revision = 1000 + index
        timestamp = f"2026-09-20T00:00:{index:02d}Z"
        current_revision = revision + (1 if family in revision_drift else 0)
        current_timestamp = f"2026-09-21T00:00:{index:02d}Z" if family in revision_drift else timestamp
        pages_g3.append(direct_row(index, family, revision, timestamp))
        # Intentionally use the same title for every different ID. Titles must
        # not create a join or collapse source identities.
        pages_g4.append(g4_page(index, current_revision, current_timestamp, "Repeated title"))
    g3 = {
        "schema": "OTERYN_SOURCE_FAMILY_CLASSIFIED_UNIVERSE/v1",
        "authority": {
            "gameplay_truth": "NONE",
            "identity_resolution": "NOT_PERFORMED",
            "target_identity_selection": "NOT_PERFORMED",
            "semantic_promotion": "NOT_PERFORMED",
        },
        "pages": pages_g3,
    }
    g3_manifest = {
        "schema": "OTERYN_SOURCE_FAMILY_CLASSIFICATION_MANIFEST/v1",
        "classified_universe_sha256": crosswalk.sha256_bytes(crosswalk.canonical_bytes(g3)),
        "classified_page_count": len(pages_g3),
        "counts": {family: 1 for family in families},
        "invariants": {
            "candidate_relations_resolved": False,
            "target_identity_selection_performed": False,
            "semantic_promotion_performed": False,
        },
    }
    g4 = {
        "schema": "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE/v1",
        "authority": {"identity_resolution": "NOT_PERFORMED", "gameplay_truth": "NONE"},
        "pages": pages_g4,
    }
    g4_manifest = {
        "schema": "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE_MANIFEST/v1",
        "status": "G4_NON_ITEM_SOURCE_PROVENANCE_ONLY",
        "artifact": {"sha256": crosswalk.sha256_bytes(crosswalk.canonical_bytes(g4))},
        "counts": {"unique_non_item_pages": len(pages_g4)},
        "invariants": {
            "raw_content_retained": False,
            "normalized_fields_emitted": False,
            "candidate_families_are_not_assignments": True,
            "identity_promotion_performed": False,
            "semantic_promotion_performed": False,
        },
    }
    return g3, g3_manifest, g4, g4_manifest


def expect_error(code: str, fn) -> None:
    try:
        fn()
    except crosswalk.CrosswalkError as exc:
        assert code in str(exc), (code, str(exc))
    else:
        raise AssertionError(f"expected {code}")


def run() -> None:
    families = list(crosswalk.FAMILIES)
    g3, g3m, g4, g4m = fake_inputs({"Creature", "Quest"})
    artifact, manifest = crosswalk.build_crosswalk(
        g3, g3m, g4, g4m, [],
        expected_family_counts={family: 1 for family in families},
        strict_artifacts=False,
    )
    assert len(artifact["records"]) == 7
    assert manifest["totals"]["records"] == 7
    assert manifest["totals"]["source_revision_revalidation_required"] == 2
    assert manifest["totals"]["missing_canonical_identity"] == 5
    by_family = {row["family"]: row for row in artifact["records"]}
    assert by_family["Creature"]["external_id"] == "1"
    assert by_family["Creature"]["page_key"].endswith("/page_id/1")
    assert by_family["Creature"]["disposition"] == "SOURCE_REVISION_REVALIDATION_REQUIRED"
    assert by_family["Quest"]["disposition"] == "SOURCE_REVISION_REVALIDATION_REQUIRED"
    assert by_family["NPC"]["disposition"] == "MISSING_CANONICAL_IDENTITY"
    assert all(row["canonical_target"] is None for row in artifact["records"])
    assert all("title" not in row for row in artifact["records"])
    assert artifact["authority"]["title_matching_performed"] is False
    assert artifact["authority"]["source_identity_binding_emitted"] is False

    _, blocked_manifest = crosswalk.build_crosswalk(
        g3, g3m, g4, g4m, ["content/world/definitions/declarations.json"],
        expected_family_counts={family: 1 for family in families},
        strict_artifacts=False,
    )
    assert blocked_manifest["totals"]["missing_canonical_identity"] == 0
    assert blocked_manifest["totals"]["canonical_target_inventory_review_required"] == 5

    broken = {**g4, "pages": [page for page in g4["pages"] if page["page_id"] != 3]}
    broken_manifest = {
        **g4m,
        "artifact": {"sha256": crosswalk.sha256_bytes(crosswalk.canonical_bytes(broken))},
        "counts": {"unique_non_item_pages": len(broken["pages"])},
    }
    expect_error("G3_PAGE_MISSING_FROM_G4:Achievement:3", lambda: crosswalk.build_crosswalk(
        g3, g3m, broken, broken_manifest, [],
        expected_family_counts={family: 1 for family in families},
        strict_artifacts=False,
    ))

    wrong_signature = {**g3, "pages": [dict(page) for page in g3["pages"]]}
    wrong_signature["pages"][0] = direct_row(1, "Creature", 1001, "2026-09-20T00:00:01Z")
    wrong_signature["pages"][0]["source_family_classification"]["basis_signature_ids"] = ["G1_WRONG_SIGNATURE"]
    wrong_manifest = {**g3m, "classified_universe_sha256": crosswalk.sha256_bytes(crosswalk.canonical_bytes(wrong_signature))}
    expect_error("G3_DIRECT_SIGNATURE_MISMATCH", lambda: crosswalk.build_crosswalk(
        wrong_signature, wrong_manifest, g4, g4m, [],
        expected_family_counts={family: 1 for family in families},
        strict_artifacts=False,
    ))

    bad_identity = {**g4, "pages": [dict(page) for page in g4["pages"]]}
    bad_identity["pages"][0]["external_id"] = "001"
    bad_identity_manifest = {**g4m, "artifact": {"sha256": crosswalk.sha256_bytes(crosswalk.canonical_bytes(bad_identity))}}
    expect_error("G4_SOURCE_IDENTITY_INVALID:1", lambda: crosswalk.build_crosswalk(
        g3, g3m, bad_identity, bad_identity_manifest, [],
        expected_family_counts={family: 1 for family in families},
        strict_artifacts=False,
    ))

    print("g4-direct-nonitem-family-crosswalk-self-test: PASS")


if __name__ == "__main__":
    run()
