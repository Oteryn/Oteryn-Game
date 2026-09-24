#!/usr/bin/env python3
"""Build a compact, source-only provenance capture from the protected #803 census.

The output deliberately contains identifiers and provenance metadata only. It
does not retain census field values, article text, or raw wikitext.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
from typing import Any

CENSUS_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS_MANIFEST/v1"
CAPTURE_SCHEMA = "OTERYN_G4_ITEM_SOURCE_CAPTURE/v1"
CAPTURE_MANIFEST_SCHEMA = "OTERYN_G4_ITEM_SOURCE_CAPTURE_MANIFEST/v1"
SOURCE_ID = "TIBIAWIKI_STRUCTURED"
SOURCE_ROLE = "STRUCTURED_REFERENCE_DATA"
SOURCE_API = "https://www.tibiawiki.com.br/api.php"
PAGE_KEY_PREFIX = "mediawiki/tibiawiki.com.br/page_id/"
HEX_256 = re.compile(r"^[0-9a-f]{64}$")
ISO_UTC = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
MAX_PAGES = 20_000


class CaptureError(ValueError):
    """Malformed or conflicting source evidence; never silently repaired."""


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def read_object(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = path.read_bytes()
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CaptureError(f"JSON_INVALID:{path.name}") from exc
    if not isinstance(value, dict):
        raise CaptureError(f"ROOT_NOT_OBJECT:{path.name}")
    return value, raw


def _required_string(value: Any, code: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CaptureError(code)
    return value


def _positive_int(value: Any, code: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise CaptureError(code)
    return value


def _validate_timestamp(value: Any) -> str:
    timestamp = _required_string(value, "REVISION_TIMESTAMP_INVALID")
    if not ISO_UTC.fullmatch(timestamp):
        raise CaptureError("REVISION_TIMESTAMP_INVALID")
    try:
        datetime.strptime(timestamp, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as exc:
        raise CaptureError("REVISION_TIMESTAMP_INVALID") from exc
    return timestamp


def _check_manifest(census: dict[str, Any], manifest: dict[str, Any]) -> str:
    if manifest.get("schema") != MANIFEST_SCHEMA:
        raise CaptureError("CENSUS_MANIFEST_SCHEMA_INVALID")
    if manifest.get("status") != "WIKI_FIRST_SOURCE_EVIDENCE_ONLY_NO_IDENTITY_PROMOTION":
        raise CaptureError("CENSUS_MANIFEST_STATUS_INVALID")
    counts = manifest.get("counts")
    census_counts = census.get("counts")
    if not isinstance(counts, dict) or not isinstance(census_counts, dict) or counts != census_counts:
        raise CaptureError("CENSUS_MANIFEST_COUNTS_MISMATCH")
    full = manifest.get("full_output")
    if not isinstance(full, dict) or full.get("schema") != CENSUS_SCHEMA:
        raise CaptureError("CENSUS_MANIFEST_FULL_OUTPUT_INVALID")
    raw_digest = full.get("sha256")
    stable_digest = full.get("stable_without_retrieval_timestamp_sha256")
    if not isinstance(raw_digest, str) or not HEX_256.fullmatch(raw_digest):
        raise CaptureError("CENSUS_FULL_DIGEST_INVALID")
    if not isinstance(stable_digest, str) or not HEX_256.fullmatch(stable_digest):
        raise CaptureError("CENSUS_STABLE_DIGEST_INVALID")
    stable = dict(census)
    stable.pop("retrieval_timestamp", None)
    if sha256_bytes(canonical_bytes(census)) != raw_digest:
        raise CaptureError("CENSUS_FULL_DIGEST_MISMATCH")
    if sha256_bytes(canonical_bytes(stable)) != stable_digest:
        raise CaptureError("CENSUS_STABLE_DIGEST_MISMATCH")
    invariants = manifest.get("invariants")
    required = {
        "wiki_first_discovery": True,
        "starts_from_crystal_38157": False,
        "identity_resolution_performed": False,
        "semantic_promotion_performed": False,
        "raw_long_form_prose_collected": False,
        "raw_wikitext_committed": False,
    }
    if not isinstance(invariants, dict) or any(invariants.get(k) is not v for k, v in required.items()):
        raise CaptureError("CENSUS_INVARIANTS_INVALID")
    return stable_digest


def transform(census: dict[str, Any], source_manifest: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    if census.get("schema") != CENSUS_SCHEMA:
        raise CaptureError("CENSUS_SCHEMA_INVALID")
    source = census.get("source")
    if not isinstance(source, dict) or source.get("id") != SOURCE_ID or source.get("role") != SOURCE_ROLE or source.get("api") != SOURCE_API:
        raise CaptureError("CENSUS_SOURCE_INVALID")
    discovery = source.get("discovery")
    if not isinstance(discovery, dict) or discovery.get("kind") != "MEDIAWIKI_CATEGORYMEMBERS" or discovery.get("namespace") != 0:
        raise CaptureError("CENSUS_DISCOVERY_INVALID")
    stable_batch_digest = _check_manifest(census, source_manifest)
    pages = census.get("pages")
    if not isinstance(pages, list) or not 0 < len(pages) <= MAX_PAGES:
        raise CaptureError("CENSUS_PAGES_INVALID")
    rows: list[dict[str, Any]] = []
    by_id: dict[int, tuple[Any, ...]] = {}
    by_title: dict[str, tuple[Any, ...]] = {}
    for page in pages:
        if not isinstance(page, dict):
            raise CaptureError("PAGE_ROW_INVALID")
        page_id = _positive_int(page.get("page_id"), "PAGE_ID_INVALID")
        external_id = str(page_id)
        title = _required_string(page.get("title"), "PAGE_TITLE_INVALID")
        revision_id = _positive_int(page.get("revision_id"), "REVISION_ID_INVALID")
        revision_timestamp = _validate_timestamp(page.get("revision_timestamp"))
        digest = page.get("source_digest")
        if not isinstance(digest, str) or not HEX_256.fullmatch(digest):
            raise CaptureError("SOURCE_DIGEST_INVALID")
        if page.get("source") != SOURCE_ID or page.get("source_role") != SOURCE_ROLE:
            raise CaptureError("PAGE_SOURCE_INVALID")
        record = (external_id, title, revision_id, revision_timestamp, digest)
        if page_id in by_id and by_id[page_id] != record:
            raise CaptureError("DUPLICATE_PAGE_ID_CONFLICT")
        if page_id in by_id:
            raise CaptureError("DUPLICATE_PAGE_ID")
        title_key = title.casefold()
        if title_key in by_title and by_title[title_key] != record:
            raise CaptureError("DUPLICATE_PAGE_TITLE_CONFLICT")
        by_id[page_id] = record
        by_title[title_key] = record
        rows.append({
            "source": SOURCE_ID,
            "source_role": SOURCE_ROLE,
            "source_namespace": "mediawiki/tibiawiki.com.br",
            "page_key": PAGE_KEY_PREFIX + external_id,
            "external_id": external_id,
            "title": title,
            "revision_id": revision_id,
            "revision_timestamp": revision_timestamp,
            "source_digest": digest,
        })
    rows.sort(key=lambda row: (int(row["external_id"]), row["title"]))
    source_counts = source_manifest["counts"]["source_shapes"]
    expected = source_manifest["counts"]["discovered_pages"]
    if len(rows) != expected or sum(source_counts.values()) != expected:
        raise CaptureError("CENSUS_PARTITION_INVALID")
    rows_payload = canonical_bytes(rows)
    artifact = {
        "schema": CAPTURE_SCHEMA,
        "batch_id": f"tibiawiki-item-census:{stable_batch_digest}",
        "source_batch_digest": stable_batch_digest,
        "source_page_count": len(rows),
        "rows": rows,
    }
    artifact_payload = canonical_bytes(artifact)
    artifact_digest = sha256_bytes(artifact_payload)
    capture_manifest = {
        "schema": CAPTURE_MANIFEST_SCHEMA,
        "status": "SOURCE_PROVENANCE_ONLY_NO_CANONICAL_BINDING_OR_PROMOTION",
        "source": {"id": SOURCE_ID, "role": SOURCE_ROLE, "api": SOURCE_API},
        "source_capture_batch_id": artifact["batch_id"],
        "source_census_schema": CENSUS_SCHEMA,
        "source_census_digest": stable_batch_digest,
        "source_census_page_count": expected,
        "captured_row_count": len(rows),
        "source_shape_counts": dict(sorted(source_counts.items())),
        "row_schema_fields": ["source", "source_role", "source_namespace", "page_key", "external_id", "title", "revision_id", "revision_timestamp", "source_digest"],
        "artifact": {"path": "g4-item-source-capture.json", "sha256": artifact_digest, "rows_sha256": sha256_bytes(rows_payload), "bytes": len(artifact_payload), "committed_bulk_corpus": False},
        "invariants": {
            "all_census_pages_retained": True,
            "duplicate_or_conflicting_ids_rejected": True,
            "external_id_is_verbatim_decimal_page_id": True,
            "namespaced_mediawiki_page_key": True,
            "raw_source_digest_preserved": True,
            "raw_wikitext_or_prose_retained": False,
            "canonical_identity_binding_performed": False,
            "field_or_semantic_promotion_performed": False,
            "second_wiki_corroboration": "UNKNOWN",
            "ots_identity_or_semantics": "HYPOTHESIS_ONLY",
        },
        "next_gate": "INDEPENDENT_SOURCE_CORROBORATION_AND_REVIEW",
    }
    return artifact, capture_manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--census", type=Path, required=True)
    parser.add_argument("--source-manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    census, _ = read_object(args.census)
    source_manifest, _ = read_object(args.source_manifest)
    artifact, manifest = transform(census, source_manifest)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(artifact))
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    print(f"g4 item source capture: PASS rows={manifest['captured_row_count']} digest={manifest['artifact']['sha256']}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CaptureError as exc:
        raise SystemExit(f"g4 item source capture: FAIL {exc}") from exc
