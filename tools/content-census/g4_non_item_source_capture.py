#!/usr/bin/env python3
"""Capture pinned raw-content digests for G1 non-Item source pages only.

Article bytes are hashed in memory and discarded. The output retains only
source identity, exact-revision digest, and G1 candidate provenance.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import re
from typing import Any

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]

SCHEMA = "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE/v1"
MANIFEST_SCHEMA = "OTERYN_G4_NON_ITEM_SOURCE_CAPTURE_MANIFEST/v1"
G2_SCHEMA = "OTERYN_GLOBAL_SOURCE_ID_UNIVERSE/v1"
G2_MANIFEST_SCHEMA = "OTERYN_GLOBAL_SOURCE_OVERLAP_MANIFEST/v1"
ITEM_STABLE_DIGEST = "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"
EXPECTED_ITEM_PAGES = 6918
MAX_INPUT_BYTES = 128 * 1024 * 1024
MAX_PAGES = 100_000
MAX_BATCH = 20
MAX_CONTENT_BYTES = 16 * 1024 * 1024
SOURCE_ID = "TIBIAWIKI_STRUCTURED"
SOURCE_ROLE = "STRUCTURED_REFERENCE_DATA"
SOURCE_NAMESPACE = "mediawiki/tibiawiki.com.br"
PAGE_KEY_PREFIX = SOURCE_NAMESPACE + "/page_id/"
PAGE_FIELDS = {
    "page_id", "title", "revision_id", "revision_timestamp",
    "candidate_families", "discovery_roots", "source_surfaces",
    "source", "source_role", "source_namespace", "external_id", "page_key",
    "raw_utf8_sha256",
}


class CaptureError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def load_g1_module() -> Any:
    g1_path = HERE / "source_universe.py"
    spec = importlib.util.spec_from_file_location("g4_g1_source_universe", g1_path)
    if spec is None or spec.loader is None:
        raise CaptureError("G1_SOURCE_UNIVERSE_IMPORT_FAILED")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_json(path: Path, label: str) -> dict[str, Any]:
    payload = path.read_bytes()
    if len(payload) > MAX_INPUT_BYTES:
        raise CaptureError(f"{label}_MAX_PLUS_ONE")
    try:
        value = json.loads(payload)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise CaptureError(f"{label}_JSON_INVALID") from exc
    if not isinstance(value, dict):
        raise CaptureError(f"{label}_ROOT_INVALID")
    return value


def _positive_int(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise CaptureError(f"{label}_INVALID")
    return value


def _nonempty(value: Any, label: str, limit: int = 512) -> str:
    if not isinstance(value, str) or not value or value != value.strip() or len(value.encode("utf-8")) > limit:
        raise CaptureError(f"{label}_INVALID")
    return value


def _string_list(value: Any, label: str, *, allow_empty: bool = False) -> list[str]:
    if not isinstance(value, list) or (not allow_empty and not value) or len(value) > 256:
        raise CaptureError(f"{label}_INVALID")
    result = []
    for item in value:
        result.append(_nonempty(item, label, 256))
    if len(result) != len(set(result)):
        raise CaptureError(f"{label}_DUPLICATE")
    return sorted(result, key=str.casefold)


def verify_g1(full: dict[str, Any], manifest: dict[str, Any]) -> list[dict[str, Any]]:
    if full.get("schema") != "OTERYN_FULL_CONTENT_SOURCE_UNIVERSE/v1":
        raise CaptureError("G1_SCHEMA_INVALID")
    if manifest.get("schema") != "OTERYN_FULL_CONTENT_SOURCE_CENSUS_MANIFEST/v1":
        raise CaptureError("G1_MANIFEST_SCHEMA_INVALID")
    if manifest.get("status") != "G1_SOURCE_DISCOVERY_ONLY_NO_IDENTITY_PROMOTION":
        raise CaptureError("G1_STATUS_INVALID")
    invariants = manifest.get("invariants", {})
    if invariants.get("protected_item_full_census_refetched") is not False:
        raise CaptureError("G1_ITEM_LANE_NOT_SEALED")
    if invariants.get("live_page_ids_deduplicated_before_counts") is not True:
        raise CaptureError("G1_PAGE_ID_DEDUP_INVARIANT_MISSING")
    for key in ("exact_revisions_reverified", "hard_exclusions_absent"):
        if invariants.get(key) is not True:
            raise CaptureError(f"G1_INVARIANT_MISSING:{key}")
    if any(invariants.get(key) is not False for key in (
        "identity_resolution_performed", "identity_minting_performed",
        "semantic_promotion_performed", "worldproject_population_performed",
        "raw_long_form_prose_collected", "assets_collected",
    )):
        raise CaptureError("G1_AUTHORITY_BOUNDARY_INVALID")
    expected = manifest.get("full_output", {}).get("sha256")
    if not isinstance(expected, str) or sha256_bytes(canonical_bytes(full)) != expected:
        raise CaptureError("G1_FULL_OUTPUT_DIGEST_MISMATCH")
    pages = full.get("pages")
    if not isinstance(pages, list) or len(pages) > MAX_PAGES:
        raise CaptureError("G1_PAGES_INVALID")
    if manifest.get("counts", {}).get("live_unique_pages") != len(pages):
        raise CaptureError("G1_PAGE_COUNT_MISMATCH")
    source = full.get("source")
    if not isinstance(source, dict) or source.get("id") != "TIBIAWIKI_STRUCTURED" or source.get("role") != "STRUCTURED_REFERENCE_DATA":
        raise CaptureError("G1_SOURCE_INVALID")
    if full.get("authority", {}).get("gameplay_truth") != "NONE":
        raise CaptureError("G1_AUTHORITY_INVALID")
    return pages


def deduplicate_pages(pages: list[dict[str, Any]]) -> list[dict[str, Any]]:
    merged: dict[int, dict[str, Any]] = {}
    for raw in pages:
        if not isinstance(raw, dict):
            raise CaptureError("G1_PAGE_ROW_INVALID")
        page_id = _positive_int(raw.get("page_id"), "PAGE_ID")
        title = _nonempty(raw.get("title"), "TITLE", 256)
        revision_id = _positive_int(raw.get("revision_id"), "REVISION_ID")
        timestamp = _nonempty(raw.get("revision_timestamp"), "REVISION_TIMESTAMP", 64)
        if re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", timestamp) is None:
            raise CaptureError(f"REVISION_TIMESTAMP_INVALID:{page_id}")
        candidate_families = _string_list(raw.get("candidate_families"), "CANDIDATE_FAMILIES", allow_empty=True)
        roots = _string_list(raw.get("discovery_roots"), "DISCOVERY_ROOTS")
        surfaces = _string_list(raw.get("source_surfaces"), "SOURCE_SURFACES")
        row = {
            "page_id": page_id,
            "title": title,
            "revision_id": revision_id,
            "revision_timestamp": timestamp,
            "candidate_families": set(candidate_families),
            "discovery_roots": set(roots),
            "source_surfaces": set(surfaces),
        }
        existing = merged.get(page_id)
        if existing is None:
            if len(merged) >= MAX_PAGES:
                raise CaptureError("PAGE_COUNT_MAX_PLUS_ONE")
            merged[page_id] = row
            continue
        for key in ("title", "revision_id", "revision_timestamp"):
            if existing[key] != row[key]:
                raise CaptureError(f"DUPLICATE_PAGE_SNAPSHOT_CONFLICT:{page_id}:{key}")
        for key in ("candidate_families", "discovery_roots", "source_surfaces"):
            existing[key].update(row[key])
    result = []
    for _, row in sorted(merged.items()):
        normalized = {key: sorted(row[key], key=str.casefold) for key in ("candidate_families", "discovery_roots", "source_surfaces")}
        result.append({**row, **normalized})
    return result


def protected_item_page_ids(g2: dict[str, Any], g2_manifest: dict[str, Any]) -> set[int]:
    if g2.get("schema") != G2_SCHEMA or g2_manifest.get("schema") != G2_MANIFEST_SCHEMA:
        raise CaptureError("G2_SCHEMA_INVALID")
    if g2.get("identity_basis") != "EXACT_MEDIAWIKI_PAGE_ID":
        raise CaptureError("G2_IDENTITY_BASIS_INVALID")
    if g2_manifest.get("invariants", {}).get("exact_page_id_only") is not True:
        raise CaptureError("G2_PAGE_ID_INVARIANT_MISSING")
    if g2_manifest.get("invariants", {}).get("semantic_promotion_performed") is not False or g2_manifest.get("invariants", {}).get("canonical_identity_selection_performed") is not False:
        raise CaptureError("G2_AUTHORITY_BOUNDARY_INVALID")
    inputs = g2_manifest.get("input_digests", {})
    if inputs.get("protected_item_stable_digest") != ITEM_STABLE_DIGEST:
        raise CaptureError("G2_PROTECTED_ITEM_DIGEST_INVALID")
    digest = g2_manifest.get("global_universe_sha256")
    if not isinstance(digest, str) or sha256_bytes(canonical_bytes(g2)) != digest:
        raise CaptureError("G2_FULL_OUTPUT_DIGEST_MISMATCH")
    counts = g2_manifest.get("counts", {})
    records = g2.get("pages")
    if not isinstance(records, list) or counts.get("global_unique_pages") != len(records):
        raise CaptureError("G2_PAGE_COUNT_MISMATCH")
    item_ids: set[int] = set()
    seen: set[int] = set()
    for row in records:
        if not isinstance(row, dict):
            raise CaptureError("G2_PAGE_ROW_INVALID")
        page_id = _positive_int(row.get("page_id"), "G2_PAGE_ID")
        if page_id in seen:
            raise CaptureError(f"G2_DUPLICATE_PAGE_ID:{page_id}")
        seen.add(page_id)
        provenance = row.get("provenance")
        if not isinstance(provenance, list):
            raise CaptureError(f"G2_PROVENANCE_INVALID:{page_id}")
        item_rows = [item for item in provenance if isinstance(item, dict) and item.get("lane") == "PROTECTED_ITEM"]
        if item_rows:
            if len(item_rows) != 1:
                raise CaptureError(f"G2_ITEM_PROVENANCE_CARDINALITY_INVALID:{page_id}")
            item_ids.add(page_id)
    if len(item_ids) != EXPECTED_ITEM_PAGES or counts.get("protected_item_pages") != EXPECTED_ITEM_PAGES:
        raise CaptureError("G2_PROTECTED_ITEM_PARTITION_MISMATCH")
    return item_ids


def validate_capture_rows(rows: list[dict[str, Any]]) -> None:
    seen_page_ids: set[int] = set()
    seen_external_ids: set[str] = set()
    seen_page_keys: set[str] = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != PAGE_FIELDS:
            raise CaptureError("CAPTURE_ROW_SCHEMA_INVALID")
        page_id = _positive_int(row.get("page_id"), "CAPTURE_PAGE_ID_INVALID")
        external_id = str(page_id)
        if row.get("source") != SOURCE_ID or row.get("source_role") != SOURCE_ROLE:
            raise CaptureError(f"CAPTURE_SOURCE_INVALID:{page_id}")
        if row.get("source_namespace") != SOURCE_NAMESPACE:
            raise CaptureError(f"CAPTURE_SOURCE_NAMESPACE_INVALID:{page_id}")
        if row.get("external_id") != external_id:
            raise CaptureError(f"CAPTURE_EXTERNAL_ID_MAPPING_INVALID:{page_id}")
        if row.get("page_key") != PAGE_KEY_PREFIX + external_id:
            raise CaptureError(f"CAPTURE_PAGE_KEY_MAPPING_INVALID:{page_id}")
        if page_id in seen_page_ids or external_id in seen_external_ids or row["page_key"] in seen_page_keys:
            raise CaptureError(f"CAPTURE_IDENTITY_DUPLICATE:{page_id}")
        seen_page_ids.add(page_id)
        seen_external_ids.add(external_id)
        seen_page_keys.add(row["page_key"])
        digest = row.get("raw_utf8_sha256")
        if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
            raise CaptureError(f"CAPTURE_DIGEST_INVALID:{page_id}")


def fetch_exact_revisions(client: Any, pages: list[dict[str, Any]]) -> dict[int, str]:
    """Fetch immutable G1 revisions in a bounded batch and validate the partition."""
    if not pages or len(pages) > MAX_BATCH:
        raise CaptureError("CONTENT_BATCH_SIZE_INVALID")
    expected = {page["revision_id"]: page for page in pages}
    if len(expected) != len(pages):
        raise CaptureError("CONTENT_BATCH_DUPLICATE_REVISION_ID")
    value = client.get_json({
        "action": "query", "revids": "|".join(str(value) for value in sorted(expected)), "prop": "revisions",
        "rvprop": "ids|timestamp|content", "rvslots": "main",
        "format": "json", "formatversion": "2",
    })
    if not isinstance(value, dict) or "error" in value:
        raise CaptureError("API_RESPONSE_INVALID")
    if "continue" in value:
        raise CaptureError("CONTENT_CONTINUATION_UNEXPECTED")
    query = value.get("query")
    api_pages = query.get("pages") if isinstance(query, dict) else None
    if not isinstance(api_pages, list) or len(api_pages) != len(pages):
        raise CaptureError("CONTENT_PAGE_CARDINALITY_INVALID")
    output: dict[int, str] = {}
    seen_revision_ids: set[int] = set()
    for api_page in api_pages:
        if not isinstance(api_page, dict) or api_page.get("missing") is True:
            raise CaptureError("CONTENT_PAGE_MISSING")
        page_id = _positive_int(api_page.get("pageid"), "API_PAGE_ID")
        title = _nonempty(api_page.get("title"), "API_TITLE", 256)
        revisions = api_page.get("revisions")
        if not isinstance(revisions, list) or len(revisions) != 1 or not isinstance(revisions[0], dict):
            raise CaptureError(f"CONTENT_REVISION_CARDINALITY_INVALID:{page_id}")
        revision = revisions[0]
        revision_id = _positive_int(revision.get("revid"), "API_REVISION_ID")
        page = expected.get(revision_id)
        if page is None or revision_id in seen_revision_ids:
            raise CaptureError(f"CONTENT_REVISION_PARTITION_INVALID:{revision_id}")
        if page_id != page["page_id"] or title != page["title"] or revision.get("timestamp") != page["revision_timestamp"]:
            raise CaptureError(f"SOURCE_SNAPSHOT_DRIFT:{page_id}")
        slots = revision.get("slots")
        main = slots.get("main") if isinstance(slots, dict) else None
        content = main.get("content") if isinstance(main, dict) else None
        if not isinstance(content, str):
            raise CaptureError(f"CONTENT_MISSING:{revision_id}")
        try:
            size = len(content.encode("utf-8", errors="strict"))
        except UnicodeEncodeError as exc:
            raise CaptureError(f"CONTENT_UTF8_INVALID:{revision_id}") from exc
        if size > MAX_CONTENT_BYTES:
            raise CaptureError(f"CONTENT_MAX_PLUS_ONE:{revision_id}")
        output[page_id] = content
        seen_revision_ids.add(revision_id)
    if seen_revision_ids != set(expected):
        raise CaptureError("CONTENT_REVISION_PARTITION_MISMATCH")
    return output


def capture(full: dict[str, Any], g1_manifest: dict[str, Any], g2: dict[str, Any], g2_manifest: dict[str, Any], client: Any, *, retrieval_timestamp: str) -> tuple[dict[str, Any], dict[str, Any]]:
    g1_pages = deduplicate_pages(verify_g1(full, g1_manifest))
    item_ids = protected_item_page_ids(g2, g2_manifest)
    overlap_ids = sorted({page["page_id"] for page in g1_pages} & item_ids)
    pages = [page for page in g1_pages if page["page_id"] not in item_ids]
    retained = []
    for offset in range(0, len(pages), MAX_BATCH):
        batch = pages[offset:offset + MAX_BATCH]
        content_by_page = fetch_exact_revisions(client, batch)
        for page in batch:
            raw_content = content_by_page.pop(page["page_id"])
            digest = sha256_bytes(raw_content.encode("utf-8", errors="strict"))
            page_id = page["page_id"]
            external_id = str(page_id)
            retained.append({
                **page,
                "source": SOURCE_ID,
                "source_role": SOURCE_ROLE,
                "source_namespace": SOURCE_NAMESPACE,
                "external_id": external_id,
                "page_key": PAGE_KEY_PREFIX + external_id,
                "raw_utf8_sha256": digest,
            })
            del raw_content
    validate_capture_rows(retained)
    artifact = {
        "schema": SCHEMA,
        "source": {"id": SOURCE_ID, "role": SOURCE_ROLE, "api": "https://www.tibiawiki.com.br/api.php"},
        "retrieval_timestamp": retrieval_timestamp,
        "authority": {
            "gameplay_truth": "NONE",
            "identity_resolution": "NOT_PERFORMED",
            "identity_minting": "FORBIDDEN",
            "semantic_promotion": "FORBIDDEN",
            "item_lane": "EXCLUDED_SEALED_G1_LANE",
            "second_wiki": "UNKNOWN",
            "ots_hypothesis": "UNVERIFIED_HYPOTHESIS_ONLY",
        },
        "pages": retained,
    }
    artifact_bytes = canonical_bytes(artifact)
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "status": "G4_NON_ITEM_SOURCE_PROVENANCE_ONLY",
        "collector": {"path": "tools/content-census/g4_non_item_source_capture.py", "sha256": sha256_bytes(Path(__file__).read_bytes().replace(b"\r\n", b"\n"))},
        "g1": {"schema": full["schema"], "source_snapshot_sha256": full.get("source_snapshot_sha256"), "manifest_sha256": sha256_bytes(canonical_bytes(g1_manifest))},
        "g2": {"schema": g2["schema"], "global_universe_sha256": g2_manifest["global_universe_sha256"], "protected_item_stable_digest": ITEM_STABLE_DIGEST},
        "retrieval_timestamp": retrieval_timestamp,
        "counts": {"g1_live_unique_pages": len(g1_pages), "protected_item_pages": len(item_ids), "g1_item_id_overlap_pages": len(overlap_ids), "unique_non_item_pages": len(retained)},
        "g1_item_id_overlap_page_ids": overlap_ids,
        "artifact": {"schema": SCHEMA, "sha256": sha256_bytes(artifact_bytes), "committed_bulk_corpus": False},
        "invariants": {
            "deduplicated_by_page_id": True,
            "protected_item_page_ids_subtracted": True,
            "item_overlap_page_ids_retained_as_manifest_observations": True,
            "exact_g1_revision_fetched": True,
            "raw_utf8_content_hashed_then_discarded": True,
            "raw_content_retained": False,
            "normalized_fields_emitted": False,
            "source_row_schema_validated": True,
            "external_id_is_verbatim_decimal_page_id": True,
            "namespaced_mediawiki_page_key": True,
            "source_identifiers_unique": True,
            "candidate_families_are_not_assignments": True,
            "placements_are_observations_only": True,
            "item_lane_refetched": False,
            "second_wiki_resolved": False,
            "ots_treated_as_verified": False,
            "identity_promotion_performed": False,
            "semantic_promotion_performed": False,
        },
    }
    return artifact, manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--g1-output", type=Path, required=True)
    parser.add_argument("--g1-manifest", type=Path, required=True)
    parser.add_argument("--g2-output", type=Path, required=True)
    parser.add_argument("--g2-manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    parser.add_argument("--retrieval-timestamp", default=None)
    args = parser.parse_args()
    timestamp = args.retrieval_timestamp or utc_now_iso()
    if re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", timestamp) is None:
        raise CaptureError("RETRIEVAL_TIMESTAMP_INVALID")
    g1_module = load_g1_module()
    artifact, manifest = capture(read_json(args.g1_output, "G1_OUTPUT"), read_json(args.g1_manifest, "G1_MANIFEST"), read_json(args.g2_output, "G2_OUTPUT"), read_json(args.g2_manifest, "G2_MANIFEST"), g1_module.predecessor.ApiClient(), retrieval_timestamp=timestamp)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(artifact))
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
