#!/usr/bin/env python3
"""Deterministic exact-page-ID union of the G1 live lane and protected Item lane.

This utility only deduplicates source identities. Candidate classifications and
crosswalk dispositions are deliberately not promoted or copied as gameplay truth.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

G1_SCHEMA = "OTERYN_FULL_CONTENT_SOURCE_UNIVERSE/v1"
CROSSWALK_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_IDENTITY_CROSSWALK/v1"
G1_MANIFEST_SCHEMA = "OTERYN_FULL_CONTENT_SOURCE_CENSUS_MANIFEST/v1"
CROSSWALK_MANIFEST_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_IDENTITY_CROSSWALK_MANIFEST/v1"
ITEM_STABLE_DIGEST = "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def _unique_index(rows: list[dict[str, Any]], lane: str) -> dict[int, dict[str, Any]]:
    index: dict[int, dict[str, Any]] = {}
    for row in rows:
        page_id = row.get("page_id")
        if isinstance(page_id, bool) or not isinstance(page_id, int) or page_id <= 0:
            raise ValueError(f"{lane}: invalid page_id {page_id!r}")
        if page_id in index:
            raise ValueError(f"{lane}: duplicate page_id {page_id}")
        if not isinstance(row.get("title"), str) or not row["title"]:
            raise ValueError(f"{lane}: page_id {page_id} has no title")
        index[page_id] = row
    return index


def merge(g1: dict[str, Any], g1_manifest: dict[str, Any],
          crosswalk: dict[str, Any], crosswalk_manifest: dict[str, Any],
          g1_artifact_id: str, crosswalk_artifact_id: str, expected_item_pages: int = 6918,
          g1_archive_sha256: str | None = None,
          crosswalk_archive_sha256: str | None = None,
          g1_run_id: str | None = None,
          g1_head_sha: str | None = None,
          crosswalk_run_id: str | None = None,
          crosswalk_head_sha: str | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
    if g1.get("schema") != G1_SCHEMA:
        raise ValueError("unexpected G1 source-universe schema")
    if g1_manifest.get("schema") != G1_MANIFEST_SCHEMA:
        raise ValueError("unexpected G1 manifest schema")
    if crosswalk.get("schema") != CROSSWALK_SCHEMA:
        raise ValueError("unexpected Item crosswalk schema")
    if crosswalk_manifest.get("schema") != CROSSWALK_MANIFEST_SCHEMA:
        raise ValueError("unexpected Item crosswalk manifest schema")

    g1_output = g1_manifest.get("full_output", {})
    g1_stable = g1_output.get("stable_without_retrieval_timestamp_sha256")
    if not isinstance(g1_stable, str) or not g1_stable:
        raise ValueError("G1 stable source-universe digest missing")
    stable_g1 = dict(g1)
    stable_g1.pop("retrieval_timestamp", None)
    if sha256(g1) != g1_output.get("sha256") or sha256(stable_g1) != g1_stable:
        raise ValueError("G1 source-universe content digest mismatch")
    item_digest = crosswalk_manifest.get("input_digests", {}).get("wiki_first_census_stable_sha256")
    crosswalk_digest = crosswalk_manifest.get("full_output", {}).get("sha256")
    if not crosswalk_digest or sha256(crosswalk) != crosswalk_digest:
        raise ValueError("Item crosswalk content digest mismatch")
    sealed = g1.get("sealed_lanes", [])
    sealed_digests = {lane.get("stable_digest") for lane in sealed if lane.get("root_id") == "items-protected"}
    if item_digest != ITEM_STABLE_DIGEST or ITEM_STABLE_DIGEST not in sealed_digests:
        raise ValueError("Item lane digest does not match protected sealed lane")

    live = _unique_index(g1.get("pages", []), "G1")
    items = _unique_index(crosswalk.get("records", []), "Item crosswalk")
    expected_live = g1_manifest.get("counts", {}).get("live_unique_pages")
    expected_items = crosswalk_manifest.get("counts", {}).get("source_pages")
    if expected_live != len(live) or expected_items != len(items):
        raise ValueError("row count does not match input manifest")
    if len(items) != expected_item_pages:
        raise ValueError(f"protected Item row count is not {expected_item_pages:,}")

    merged: list[dict[str, Any]] = []
    overlaps: list[dict[str, Any]] = []
    for page_id in sorted(set(live) | set(items)):
        live_row = live.get(page_id)
        item_row = items.get(page_id)
        title_observations: list[dict[str, str]] = []
        provenance: list[dict[str, Any]] = []
        if live_row is not None:
            title_observations.append({"lane": "G1_LIVE_NON_ITEM", "title": live_row["title"]})
            provenance.append({
                "lane": "G1_LIVE_NON_ITEM",
                "artifact_id": g1_artifact_id,
                "observed_title": live_row["title"],
                "source": live_row.get("source"),
                "source_role": live_row.get("source_role"),
                "source_surfaces": live_row.get("source_surfaces", []),
                "discovery_roots": live_row.get("discovery_roots", []),
                "discovery_kinds": live_row.get("discovery_kinds", []),
                "categories": live_row.get("categories", []),
                "templates": live_row.get("templates", []),
                "redirect": live_row.get("redirect"),
                "surface_dispositions": live_row.get("surface_dispositions", []),
                "source_shape": live_row.get("source_shape"),
                "revision_id": live_row.get("revision_id"),
                "revision_timestamp": live_row.get("revision_timestamp"),
                "candidate_families_evidence_only": live_row.get("candidate_families", []),
                "family_classification_state_evidence_only": live_row.get("family_classification_state"),
            })
        if item_row is not None:
            title_observations.append({"lane": "PROTECTED_ITEM", "title": item_row["title"]})
            provenance.append({
                "lane": "PROTECTED_ITEM",
                "crosswalk_artifact_id": crosswalk_artifact_id,
                "observed_title": item_row["title"],
                "sealed_lane_stable_digest": ITEM_STABLE_DIGEST,
                "crosswalk_row_title": item_row["title"],
                "source_shape": item_row.get("source_shape"),
                "source_revision": {
                    "state": "UNKNOWN",
                    "reason": "The crosswalk row does not carry an Item page revision ID or timestamp.",
                },
                "row_role": "PAGE_ID_AND_PROVENANCE_EVIDENCE_ONLY",
                "identity_crosswalk_conclusions_consumed": False,
            })
        distinct_titles = sorted({observation["title"] for observation in title_observations})
        title_divergence = len(distinct_titles) > 1
        record = {
            "page_id": page_id,
            "title_observations": title_observations,
            "cross_lane_title_divergence": title_divergence,
            "provenance": provenance,
        }
        merged.append(record)
        if live_row is not None and item_row is not None:
            overlaps.append({
                "page_id": page_id,
                "title_observations": title_observations,
                "cross_lane_title_divergence": title_divergence,
                "g1_provenance": provenance[0],
                "item_provenance": provenance[1],
                "overlap_basis": "EXACT_MEDIAWIKI_PAGE_ID",
            })

    result = {
        "schema": "OTERYN_GLOBAL_SOURCE_ID_UNIVERSE/v1",
        "identity_basis": "EXACT_MEDIAWIKI_PAGE_ID",
        "authority": {
            "identity_resolution": "NOT_PERFORMED",
            "canonical_identity_selection": "NOT_PERFORMED",
            "semantic_promotion": "NOT_PERFORMED",
            "gameplay_truth": "NONE",
        },
        "inputs": {
            "g1_artifact_id": g1_artifact_id,
            "g1_run_id": g1_run_id,
            "g1_head_sha": g1_head_sha,
            "g1_archive_sha256": g1_archive_sha256,
            "g1_stable_without_retrieval_timestamp_sha256": g1_stable,
            "g1_full_output_sha256": g1_output.get("sha256"),
            "g1_source_snapshot_sha256": g1_manifest.get("source_snapshot_sha256"),
            "g1_manifest_schema": G1_MANIFEST_SCHEMA,
            "protected_item_crosswalk_artifact_id": crosswalk_artifact_id,
            "protected_item_crosswalk_run_id": crosswalk_run_id,
            "protected_item_crosswalk_head_sha": crosswalk_head_sha,
            "protected_item_crosswalk_archive_sha256": crosswalk_archive_sha256,
            "protected_item_crosswalk_manifest_output_sha256": crosswalk_manifest.get("full_output", {}).get("sha256"),
            "protected_item_crosswalk_embedded_inputs": crosswalk_manifest.get("input_digests", {}),
            "protected_item_stable_digest": ITEM_STABLE_DIGEST,
        },
        "counts": {
            "g1_live_unique_pages": len(live),
            "protected_item_pages": len(items),
            "exact_id_overlap_pages": len(overlaps),
            "cross_lane_title_divergence_pages": sum(o["cross_lane_title_divergence"] for o in overlaps),
            "global_unique_pages": len(merged),
        },
        "overlaps": overlaps,
        "pages": merged,
    }
    result_digest = sha256(result)
    manifest = {
        "schema": "OTERYN_GLOBAL_SOURCE_OVERLAP_MANIFEST/v1",
        "status": "EXACT_ID_DEDUPLICATION_ONLY_NO_IDENTITY_OR_SEMANTIC_PROMOTION",
        "input_digests": result["inputs"],
        "counts": result["counts"],
        "global_universe_sha256": result_digest,
        "invariants": {
            "exact_page_id_only": True,
            "cross_lane_title_divergence_preserved_as_observation": True,
            "provenance_retained": True,
            "semantic_promotion_performed": False,
            "canonical_identity_selection_performed": False,
        },
    }
    return result, manifest


def read_json(path: str) -> dict[str, Any]:
    with open(path, encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--g1", required=True, help="G1 source-universe.json")
    parser.add_argument("--g1-manifest", required=True)
    parser.add_argument("--item-crosswalk", required=True, help="crosswalk-first.json from protected Item lane")
    parser.add_argument("--item-manifest", required=True, help="manifest-first.json")
    parser.add_argument("--g1-artifact-id", required=True)
    parser.add_argument("--item-artifact-id", required=True)
    parser.add_argument("--g1-archive-sha256")
    parser.add_argument("--item-archive-sha256")
    parser.add_argument("--g1-run-id")
    parser.add_argument("--g1-head-sha")
    parser.add_argument("--item-run-id")
    parser.add_argument("--item-head-sha")
    parser.add_argument("--out", required=True, help="output directory")
    args = parser.parse_args()
    result, manifest = merge(read_json(args.g1), read_json(args.g1_manifest),
                             read_json(args.item_crosswalk), read_json(args.item_manifest),
                             args.g1_artifact_id, args.item_artifact_id,
                             g1_archive_sha256=args.g1_archive_sha256,
                             crosswalk_archive_sha256=args.item_archive_sha256,
                             g1_run_id=args.g1_run_id,
                             g1_head_sha=args.g1_head_sha,
                             crosswalk_run_id=args.item_run_id,
                             crosswalk_head_sha=args.item_head_sha)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "global-source-universe.json").write_bytes(canonical_bytes(result))
    (out / "manifest.json").write_bytes(canonical_bytes(manifest))
    print(json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
