#!/usr/bin/env python3
"""Offline exact-cohort and fail-closed regressions for G3 UNKNOWN closure."""
from __future__ import annotations

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import unknown_closure as closure

INPUT = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("/tmp/g3-unknown-ledger.json")


def require_failure(fn, fragment: str) -> None:
    try:
        fn()
    except (ValueError, KeyError, TypeError) as exc:
        assert fragment in str(exc), (fragment, str(exc))
    else:
        raise AssertionError(f"expected failure containing {fragment!r}")


def synthetic_refresh(pages: list[dict]) -> dict[int, dict]:
    output = {}
    for row in pages:
        p = row["lane_observations"][0]
        revisions = row.get("pinned_revision_ids", [])
        output[row["page_id"]] = {
            "page_id": row["page_id"],
            "title": row.get("title_observations", [{}])[0].get("title"),
            "revision_id": revisions[0] if revisions else p.get("revision_id", 1),
            "revision_timestamp": p.get("revision_timestamp"),
            "redirect": p.get("redirect") is True,
            "categories": p.get("categories", []),
            "templates": p.get("templates", []),
        }
    return output


def main() -> None:
    ledger = closure.load_pinned_ledger(INPUT)
    pages = closure.validate_ledger(ledger)
    assert len(pages) == 5512
    assert closure.pinned_template_partition(pages) == closure.EXPECTED_TEMPLATE_PARTITION

    fresh = synthetic_refresh(pages)
    output, manifest = closure.build_output(
        ledger,
        fresh,
        {"attempted_pages": 5512, "verified_same_revision_pages": 5512, "revision_drift_pages": 0, "source_unavailable_pages": 0, "refresh_failures": {}},
    )
    assert output["counts"]["output_pages"] == 5512
    assert output["counts"]["exact_signature_definition_family_routes"] == 12
    assert output["counts"]["source_revision_status"]["NO_PINNED_BASELINE"] == 1406
    assert output["counts"]["family_disposition"] == {
        "ROUTED_EXACT_SOURCE_SIGNATURE": 12,
        "UNRESOLVED_AMBIGUOUS_OR_NONDEFINITION": 4094,
        "UNRESOLVED_NO_PINNED_BASELINE": 1406,
    }
    assert output["counts"]["content_disposition"] == {
        "ALTERNATE_SOURCE_ONLY": 187,
        "AMBIGUOUS_FAMILY": 131,
        "DYNAMIC_STATE_RELATIONSHIP_RETAINED": 12,
        "EDITORIAL_RELATIONSHIP_ONLY": 521,
        "MULTI_FAMILY_DEFINITION_CANDIDATE": 3121,
        "MULTI_FAMILY_RELATIONSHIP_ONLY": 18,
        "NO_PRIMARY_DEFINITION_SIGNATURE": 1399,
        "PARSER_EVIDENCE_REQUIRED": 7,
        "REDIRECT_TARGET_UNPROVEN": 104,
        "SOURCE_CLASSIFIER_EVIDENCE_REQUIRED": 12,
    }
    assert manifest["invariants"]["exact_5512_page_id_partition"] is True
    assert all(
        row["placement_disposition"] == "NO_EXACT_PLACEMENT_RECORD_IN_PINNED_CENSUS"
        and row["exclusion_disposition"] == "NOT_EXCLUDED"
        and "relationship_disposition" in row
        and "source_only_disposition" in row
        for row in output["rows"]
    )

    unavailable_output, _ = closure.build_output(
        ledger,
        {},
        {"attempted_pages": 20, "verified_same_revision_pages": 0, "revision_drift_pages": 0, "source_unavailable_pages": 5512, "refresh_failures": {"HTTP_403": 20, "REMAINING_UNQUERIED_AFTER_ACCESS_LIMIT": 5492}},
    )
    assert unavailable_output["counts"]["family_disposition"] == {"UNRESOLVED_NO_CURRENT_SOURCE": 5512}
    assert unavailable_output["counts"]["content_disposition"] == {
        "ALTERNATE_SOURCE_ONLY": 187,
        "AMBIGUOUS_FAMILY": 131,
        "DYNAMIC_STATE_RELATIONSHIP_CANDIDATE": 12,
        "EDITORIAL_RELATIONSHIP_ONLY": 521,
        "MULTI_FAMILY_DEFINITION_CANDIDATE": 3121,
        "MULTI_FAMILY_RELATIONSHIP_ONLY": 18,
        "NO_PRIMARY_DEFINITION_SIGNATURE": 1399,
        "PARSER_EVIDENCE_REQUIRED": 7,
        "REDIRECT_TARGET_UNPROVEN": 104,
        "SOURCE_CLASSIFIER_EVIDENCE_REQUIRED": 12,
    }

    world = [r for r in pages if r["blocker_class"] == "WORLD_CHANGE_DYNAMIC_SCOPE_UNRESOLVED"]
    assert len(world) == 12
    assert all(fresh[r["page_id"]]["revision_id"] == r["pinned_revision_ids"][0] for r in world)
    assert all(
        closure.disposition(r, fresh[r["page_id"]])["definition_family"] == "Encounter"
        for r in world
    )

    overlap = next(r for r in pages if r["page_id"] == 19087)
    overlap_result = closure.disposition(overlap, fresh[19087])
    assert overlap_result["definition_family"] is None
    assert overlap_result["content_disposition"] == "MULTI_FAMILY_RELATIONSHIP_ONLY"
    assert overlap_result["candidate_relationships"] == overlap["candidate_relationships_evidence_only"]

    root_counterexample = next(r for r in pages if r["page_id"] == 3297)
    assert closure.disposition(root_counterexample, fresh[3297])["definition_family"] is None
    redirect = next(r for r in pages if r["primary_source_shape"] == "REDIRECT")
    assert closure.disposition(redirect, fresh[redirect["page_id"]])["content_disposition"] == "REDIRECT_TARGET_UNPROVEN"

    first_world = world[0]
    drift = dict(fresh[first_world["page_id"]], revision_id=first_world["pinned_revision_ids"][0] + 1)
    drift_result = closure.disposition(first_world, drift)
    assert drift_result["source_revision_status"] == "DRIFT"
    assert drift_result["definition_family"] is None

    title_drift = dict(fresh[first_world["page_id"]], title="Unobserved Moved Title")
    title_result = closure.disposition(first_world, title_drift)
    assert title_result["source_revision_status"] == "TITLE_DRIFT"
    assert title_result["definition_family"] is None

    unavailable = closure.disposition(world[0], {"refresh_error": "HTTP_403"})
    assert unavailable["source_revision_status"] == "UNVERIFIED"
    assert unavailable["definition_family"] is None

    duplicated = dict(ledger)
    duplicated["pages"] = list(ledger["pages"])
    duplicated["pages"][1] = dict(duplicated["pages"][1], page_id=duplicated["pages"][0]["page_id"])
    require_failure(lambda: closure.validate_ledger(duplicated), "duplicated")

    excluded = dict(ledger)
    excluded["pages"] = list(ledger["pages"])
    excluded["pages"][0] = dict(excluded["pages"][0], title_observations=[{"title": "Kalkulatory"}])
    require_failure(lambda: closure.validate_ledger(excluded), "hard-excluded")

    bad_count = dict(ledger)
    bad_count["pages"] = ledger["pages"][:-1]
    require_failure(lambda: closure.validate_ledger(bad_count), "row count mismatch")

    print("unknown closure exact-cohort and fail-closed self-test: PASS")


if __name__ == "__main__":
    main()
