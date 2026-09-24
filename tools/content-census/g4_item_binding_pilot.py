#!/usr/bin/env python3
"""Compile a bounded G4 Item source-identity binding pilot.

Inputs are exact, separately verified evidence products.  This compiler does
not mutate ProjectV2, Item definitions, source imports, gameplay fields, or
Presentation/Asset/runtime identifiers.  It emits only compact crosswalk
evidence and non-persisted typed binding candidates for EXACT rows.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
from typing import Any

PROFILE = "OTERYN_G4_ITEM_BINDING_PILOT/v1"
OUTPUT_SCHEMA = "OTERYN_G4_ITEM_BINDING_PILOT/v1"
MANIFEST_SCHEMA = "OTERYN_G4_ITEM_BINDING_PILOT_MANIFEST/v1"
CAPTURE_SCHEMA = "OTERYN_G4_ITEM_SOURCE_CAPTURE/v1"
CAPTURE_MANIFEST_SCHEMA = "OTERYN_G4_ITEM_SOURCE_CAPTURE_MANIFEST/v1"
CURRENT_SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI/v1"
CURRENT_MANIFEST_SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_MANIFEST/v1"
CROSSWALK_SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK/v1"
SOURCE_ID = "TIBIAWIKI_STRUCTURED"
SOURCE_ROLE = "STRUCTURED_REFERENCE_DATA"
PROJECT_SOURCE_KEY = "oteryn:source.tibiawiki"
IDENTITY_NAMESPACE = "mediawiki/page_id"
PAGE_KEY_PREFIX = "mediawiki/tibiawiki.com.br/page_id/"
EXPECTED_CAPTURE_SHA256 = "0fa435ba888071e7320e70b793203c87f79eda11c0ff8dd824dc2eeaed24421f"
EXPECTED_CAPTURE_ROWS_SHA256 = "62efa8385666f725f707b4f02d2372ce91ec228fc770d9049738c2337f974e30"
EXPECTED_CAPTURE_BATCH = "tibiawiki-item-census:389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"
EXPECTED_CAPTURE_CENSUS_SHA256 = "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"
EXPECTED_CAPTURE_ROWS = 6918
EXPECTED_CURRENT_COLLECTOR_SHA256 = "ef160e7b76458029064d7f50d7808fa80add99da96d2a5ebacbbc7c6426ba748"
EXPECTED_CURRENT_STABLE_SHA256 = "34a906252a2644ea70fdbbe9c52c6f142ea21c8955b40ffb4c4aa2032f130c29"
EXPECTED_CROSSWALK_SHA256 = "004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d"
EXPECTED_ITEM_COUNT = 38157
EXPECTED_WIKI_MATCHED = 22
MIN_NON_TITLE_AGREEMENTS = 2
PILOT_BINDING_LIMIT = 1
DISPOSITIONS = {"EXACT", "ACCEPTED_ALIAS", "PROBABLE_MATCH", "AMBIGUOUS", "CONFLICT", "NO_MATCH", "UNRESOLVED_SOURCE_SHAPE"}
COMPARABLE_FIELDS = {
    "attack": "attack", "defense": "defense", "extra_defense": "defensemod",
    "range": "range", "hit_chance": "hit", "armor": "armor",
    "charge_count": "charges", "capacity": "volume",
}
HEX = re.compile(r"^[0-9a-f]{64}$")
SOURCE_KEY_PATTERN = re.compile(r"^[a-z][a-z0-9_-]*:[a-z][a-z0-9._-]*$")


class PilotError(ValueError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def read_json(path: Path) -> tuple[dict[str, Any], bytes]:
    payload = path.read_bytes()
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PilotError(f"JSON_INVALID:{path.name}") from exc
    if not isinstance(value, dict):
        raise PilotError(f"ROOT_NOT_OBJECT:{path.name}")
    return value, payload


def assert_digest(actual: str, expected: str, label: str) -> None:
    if actual != expected:
        raise PilotError(f"{label}_DIGEST_MISMATCH:{actual}")


def verify_capture(capture: dict[str, Any], manifest: dict[str, Any]) -> dict[str, dict[str, Any]]:
    if capture.get("schema") != CAPTURE_SCHEMA or manifest.get("schema") != CAPTURE_MANIFEST_SCHEMA:
        raise PilotError("CAPTURE_SCHEMA_MISMATCH")
    if manifest.get("status") != "SOURCE_PROVENANCE_ONLY_NO_CANONICAL_BINDING_OR_PROMOTION":
        raise PilotError("CAPTURE_STATUS_INVALID")
    if manifest.get("source_capture_batch_id") != EXPECTED_CAPTURE_BATCH:
        raise PilotError("CAPTURE_BATCH_MISMATCH")
    if manifest.get("source_census_digest") != EXPECTED_CAPTURE_CENSUS_SHA256:
        raise PilotError("CAPTURE_CENSUS_DIGEST_MISMATCH")
    if manifest.get("captured_row_count") != EXPECTED_CAPTURE_ROWS or capture.get("source_page_count") != EXPECTED_CAPTURE_ROWS:
        raise PilotError("CAPTURE_ROW_COUNT_MISMATCH")
    artifact_info = manifest.get("artifact")
    if not isinstance(artifact_info, dict):
        raise PilotError("CAPTURE_ARTIFACT_METADATA_INVALID")
    assert_digest(sha256(canonical_bytes(capture)), EXPECTED_CAPTURE_SHA256, "CAPTURE")
    assert_digest(sha256(canonical_bytes(capture)), artifact_info.get("sha256", ""), "CAPTURE_MANIFEST_ARTIFACT")
    rows = capture.get("rows")
    if not isinstance(rows, list) or len(rows) != EXPECTED_CAPTURE_ROWS:
        raise PilotError("CAPTURE_ROWS_INVALID")
    assert_digest(sha256(canonical_bytes(rows)), EXPECTED_CAPTURE_ROWS_SHA256, "CAPTURE_ROWS")
    assert_digest(sha256(canonical_bytes(rows)), artifact_info.get("rows_sha256", ""), "CAPTURE_MANIFEST_ROWS")
    if manifest.get("source_shape_counts") != {"INFOBOX_ITEM": 5475, "INFOBOX_ITEM_PARSE_ERROR": 7, "NO_INFOBOX_ITEM": 1436}:
        raise PilotError("CAPTURE_SOURCE_SHAPE_PARTITION_MISMATCH")
    by_id: dict[str, dict[str, Any]] = {}
    required = set(manifest.get("row_schema_fields", []))
    for row in rows:
        if not isinstance(row, dict) or set(row) != required:
            raise PilotError("CAPTURE_ROW_SCHEMA_INVALID")
        external_id = row.get("external_id")
        if not isinstance(external_id, str) or not external_id.isdecimal() or external_id.startswith("0"):
            raise PilotError("CAPTURE_EXTERNAL_ID_INVALID")
        if row.get("source") != SOURCE_ID or row.get("source_role") != SOURCE_ROLE or row.get("source_namespace") != "mediawiki/tibiawiki.com.br":
            raise PilotError("CAPTURE_SOURCE_IDENTITY_INVALID")
        if row.get("page_key") != PAGE_KEY_PREFIX + external_id:
            raise PilotError("CAPTURE_PAGE_KEY_INVALID")
        if not isinstance(row.get("revision_id"), int) or row["revision_id"] <= 0:
            raise PilotError("CAPTURE_REVISION_INVALID")
        if not isinstance(row.get("source_digest"), str) or not HEX.fullmatch(row["source_digest"]):
            raise PilotError("CAPTURE_SOURCE_DIGEST_INVALID")
        if external_id in by_id:
            raise PilotError("CAPTURE_DUPLICATE_EXTERNAL_ID")
        by_id[external_id] = row
    return by_id


def verify_current(current: dict[str, Any], manifest: dict[str, Any], protected: dict[str, Any]) -> tuple[list[dict[str, Any]], bool]:
    if current.get("schema") != CURRENT_SCHEMA or manifest.get("schema") != CURRENT_MANIFEST_SCHEMA:
        raise PilotError("CURRENT_SOURCE_SCHEMA_MISMATCH")
    if manifest.get("status") != "CURRENT_SOURCE_EVIDENCE_ONLY_NO_PROMOTION":
        raise PilotError("CURRENT_SOURCE_STATUS_INVALID")
    current_digest = sha256(canonical_bytes(current))
    assert_digest(current_digest, manifest.get("full_output", {}).get("sha256", ""), "CURRENT_SOURCE_SELF")
    if manifest.get("collector") != protected.get("collector") or manifest.get("collector", {}).get("sha256") != EXPECTED_CURRENT_COLLECTOR_SHA256:
        raise PilotError("CURRENT_SOURCE_COLLECTOR_NOT_PROTECTED")
    if protected.get("full_output", {}).get("stable_without_retrieval_timestamp_sha256") != EXPECTED_CURRENT_STABLE_SHA256:
        raise PilotError("PROTECTED_CURRENT_SOURCE_STABLE_DIGEST_INVALID")
    stable_digest = stable_current_digest(current)
    verify_stable_current_digest(stable_digest, manifest, protected)
    if manifest.get("counts") != protected.get("counts"):
        raise PilotError("CURRENT_SOURCE_PROTECTED_PARTITION_MISMATCH")
    if manifest.get("target_cut") != "2026-07-28" or current.get("target_cut") != "2026-07-28":
        raise PilotError("CURRENT_SOURCE_TARGET_CUT_INVALID")
    rows = current.get("records")
    pages = current.get("pages")
    if not isinstance(rows, list) or len(rows) != EXPECTED_ITEM_COUNT or not isinstance(pages, list):
        raise PilotError("CURRENT_SOURCE_ROWS_INVALID")
    matched = [row for row in rows if isinstance(row, dict) and row.get("current_source", {}).get("disposition") == "WIKI_MATCHED"]
    if len(matched) != EXPECTED_WIKI_MATCHED:
        raise PilotError("CURRENT_SOURCE_MATCHED_COUNT_MISMATCH")
    return matched, stable_digest == EXPECTED_CURRENT_STABLE_SHA256


def stable_current_digest(current: dict[str, Any]) -> str:
    stable = dict(current)
    stable.pop("retrieval_timestamp", None)
    return sha256(canonical_bytes(stable))


def verify_stable_current_digest(observed: str, manifest: dict[str, Any], protected: dict[str, Any]) -> bool:
    expected = protected.get("full_output", {}).get("stable_without_retrieval_timestamp_sha256")
    manifest_digest = manifest.get("full_output", {}).get("stable_without_retrieval_timestamp_sha256")
    if expected != EXPECTED_CURRENT_STABLE_SHA256:
        raise PilotError("PROTECTED_CURRENT_SOURCE_STABLE_DIGEST_INVALID")
    if manifest_digest != observed:
        raise PilotError("CURRENT_SOURCE_MANIFEST_STABLE_DIGEST_MISMATCH")
    # A real, internally consistent current-source drift is evidence to report,
    # not permission to relax the protected pin or abort before artifact upload.
    return observed == expected


def disposition(*, matched: list[str], contradicted: list[str], page_conflict: bool = False, target_ambiguous: bool = False) -> tuple[str, str]:
    if page_conflict or contradicted:
        return "CONFLICT", "STABLE_SIGNAL_CONTRADICTION" if contradicted else "ONE_PAGE_ID_CANDIDATE_FOR_MULTIPLE_CANONICAL_TARGETS"
    if target_ambiguous:
        return "AMBIGUOUS", "ONE_CANONICAL_TARGET_HAS_MULTIPLE_PAGE_ID_CANDIDATES"
    if len(matched) >= MIN_NON_TITLE_AGREEMENTS:
        return "EXACT", "UNIQUE_TARGET_TWO_OR_MORE_NON_TITLE_AGREEMENTS"
    if matched:
        return "PROBABLE_MATCH", "UNIQUE_TARGET_ONLY_ONE_NON_TITLE_AGREEMENT"
    return "NO_MATCH", "NO_COMPARABLE_NON_TITLE_AGREEMENT"


def captured_identity_tuple(row: dict[str, Any]) -> dict[str, Any]:
    """Preserve the exact source tuple, including mutable page revision data."""
    required = ("source", "source_role", "source_namespace", "external_id", "page_key", "title", "revision_id", "revision_timestamp", "source_digest")
    if any(key not in row for key in required):
        raise PilotError("CAPTURE_IDENTITY_TUPLE_INCOMPLETE")
    if not SOURCE_KEY_PATTERN.fullmatch(PROJECT_SOURCE_KEY):
        raise PilotError("PROJECT_SOURCE_KEY_INVALID")
    return {key: row[key] for key in required}


def integer_signal(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and re.fullmatch(r"[+-]?\d+", value.strip()):
        return int(value.strip())
    return None


def protected_signals(record: dict[str, Any], profiles: dict[str, dict[str, Any]]) -> dict[str, int]:
    profile = profiles.get(record.get("source_profile_id"))
    if not isinstance(profile, dict) or not isinstance(profile.get("candidate_observations"), list):
        raise PilotError("SOURCE_PROFILE_MISSING")
    result: dict[str, int] = {}
    conflicts: set[str] = set()
    for observation in profile["candidate_observations"]:
        if not isinstance(observation, dict):
            raise PilotError("SOURCE_PROFILE_OBSERVATION_INVALID")
        field = COMPARABLE_FIELDS.get(observation.get("native_field"))
        if field is None or observation.get("nested_values") not in (None, []):
            continue
        value = integer_signal(observation.get("source_value"))
        if value is None:
            continue
        if field in result and result[field] != value:
            conflicts.add(field)
        result[field] = value
    for field in conflicts:
        result.pop(field, None)
    return result


def compare_fields(target: dict[str, Any], page: dict[str, Any], profiles: dict[str, dict[str, Any]]) -> tuple[list[str], list[str]]:
    fields = page.get("normalized_fields")
    if not isinstance(fields, dict):
        return [], []
    matched: list[str] = []
    contradicted: list[str] = []
    for field, expected in sorted(protected_signals(target, profiles).items()):
        observation = fields.get(field)
        if not isinstance(observation, dict) or observation.get("state") != "VALUE":
            continue
        actual = observation.get("value")
        if isinstance(actual, bool) or not isinstance(actual, int):
            continue
        (matched if actual == expected else contradicted).append(field)
    return matched, contradicted


def typed_binding_candidate(target_key: str, source_revision: str, external_id: str) -> dict[str, Any]:
    """Return exactly the deny_unknown_fields carrier, with no audit metadata."""
    return {
        "source_key": PROJECT_SOURCE_KEY,
        "source_revision": source_revision,
        "identity_namespace": IDENTITY_NAMESPACE,
        "external_id": external_id,
        "target": {"family": "Item", "key": target_key, "revision": "definition-r1"},
        "disposition": "EXACT",
    }


def compile_pilot(capture: dict[str, Any], capture_manifest: dict[str, Any], current: dict[str, Any], current_manifest: dict[str, Any], protected_current_manifest: dict[str, Any], crosswalk: dict[str, Any], crosswalk_raw: bytes) -> tuple[dict[str, Any], dict[str, Any]]:
    if crosswalk.get("schema") != CROSSWALK_SCHEMA:
        raise PilotError("CROSSWALK_SCHEMA_MISMATCH")
    assert_digest(sha256(crosswalk_raw), EXPECTED_CROSSWALK_SHA256, "CROSSWALK")
    capture_by_id = verify_capture(capture, capture_manifest)
    matched_rows, protected_current_source_matches = verify_current(current, current_manifest, protected_current_manifest)
    pages_by_id: dict[int, dict[str, Any]] = {}
    for page in current["pages"]:
        if not isinstance(page, dict) or not isinstance(page.get("page_id"), int):
            raise PilotError("CURRENT_PAGE_ROW_INVALID")
        if page["page_id"] in pages_by_id:
            raise PilotError("CURRENT_PAGE_DUPLICATE_ID")
        pages_by_id[page["page_id"]] = page
    profiles: dict[str, dict[str, Any]] = {}
    for profile in crosswalk.get("source_profiles", []):
        if not isinstance(profile, dict) or not isinstance(profile.get("profile_id"), str) or profile["profile_id"] in profiles:
            raise PilotError("CROSSWALK_PROFILE_INVALID")
        profiles[profile["profile_id"]] = profile

    target_page_ids: dict[str, set[str]] = defaultdict(set)
    page_targets: dict[str, set[str]] = defaultdict(set)
    preliminary: list[dict[str, Any]] = []
    if not protected_current_source_matches:
        observed_digest = stable_current_digest(current)
        drift_reason = "CURRENT_SOURCE_STABLE_OUTPUT_DRIFT: expected " + EXPECTED_CURRENT_STABLE_SHA256 + ", observed " + observed_digest
        for candidate in matched_rows:
            current_result = candidate.get("current_source")
            ids = current_result.get("candidate_page_ids") if isinstance(current_result, dict) else None
            if not isinstance(ids, list) or len(ids) != 1 or not isinstance(ids[0], int):
                raise PilotError("MATCHED_CANDIDATE_PAGE_PARTITION_INVALID")
            page_id = ids[0]
            external_id = str(page_id)
            target_key = candidate.get("native_key")
            source_id = candidate.get("source_item_id")
            if not isinstance(target_key, str) or not target_key.startswith("oteryn:item.") or not isinstance(source_id, int):
                raise PilotError("MATCHED_CANONICAL_IDENTITY_INVALID")
            page = pages_by_id.get(page_id)
            capture_row = capture_by_id.get(external_id)
            pinned_tuple = captured_identity_tuple(capture_row) if capture_row is not None else None
            observed_tuple = None
            if page is not None:
                observed_tuple = {
                    "source": page.get("source"), "source_role": page.get("source_role"),
                    "source_namespace": "mediawiki/tibiawiki.com.br", "external_id": external_id,
                    "page_key": PAGE_KEY_PREFIX + external_id, "title": page.get("title"),
                    "revision_id": page.get("revision_id"), "revision_timestamp": page.get("revision_timestamp"),
                    "source_digest": page.get("source_digest"),
                }
            matched_signals, contradicted_signals = compare_fields(candidate, page or {}, profiles)
            identity_tuple = pinned_tuple or observed_tuple or {
                "source": SOURCE_ID, "source_role": SOURCE_ROLE,
                "source_namespace": "mediawiki/tibiawiki.com.br", "external_id": external_id,
                "page_key": PAGE_KEY_PREFIX + external_id, "title": None,
                "revision_id": None, "revision_timestamp": None, "source_digest": None,
            }
            preliminary.append({
                **identity_tuple,
                "target_key": target_key,
                "source_item_id": source_id,
                "status": "UNRESOLVED_SOURCE_SHAPE",
                "reason": drift_reason,
                "matched_signals": matched_signals,
                "contradicted_signals": contradicted_signals,
                "capture_identity_tuple": pinned_tuple,
                "observed_identity_tuple": observed_tuple,
                "selected_for_pilot": False,
            })
        if len(preliminary) != EXPECTED_WIKI_MATCHED:
            raise PilotError("DRIFT_REVALIDATION_CANDIDATE_COUNT_MISMATCH")
        source_batch = capture.get("batch_id")
        source_batch_digest = capture.get("source_batch_digest")
        if source_batch != EXPECTED_CAPTURE_BATCH or source_batch_digest != EXPECTED_CAPTURE_CENSUS_SHA256:
            raise PilotError("CAPTURE_SOURCE_REVISION_INVALID")
        evidence = {
            "schema": OUTPUT_SCHEMA,
            "profile": PROFILE,
            "status": "REVALIDATION_REQUIRED_ZERO_BINDING_CANDIDATES",
            "source_capture": {
                "artifact_sha256": EXPECTED_CAPTURE_SHA256,
                "rows_sha256": EXPECTED_CAPTURE_ROWS_SHA256,
                "batch_id": source_batch,
                "source_batch_digest": source_batch_digest,
                "captured_rows": EXPECTED_CAPTURE_ROWS,
            },
            "canonical_inputs": {
                "canonical_item_count": EXPECTED_ITEM_COUNT,
                "canonical_definition_revision": "definition-r1",
                "protected_crosswalk_sha256": EXPECTED_CROSSWALK_SHA256,
                "current_source_sha256": sha256(canonical_bytes(current)),
                "protected_current_source_stable_sha256": EXPECTED_CURRENT_STABLE_SHA256,
                "current_source_stable_sha256": observed_digest,
            },
            "selection": {
                "candidate_pool": "the exact 22 observed WIKI_MATCHED rows from the current-source partition",
                "candidate_pool_count": len(matched_rows),
                "selected_count": 0,
                "maximum_binding_candidates": PILOT_BINDING_LIMIT,
            },
            "dispositions": {"UNRESOLVED_SOURCE_SHAPE": len(preliminary)},
            "rows": sorted(preliminary, key=lambda row: (int(row["external_id"]), row["target_key"])),
            "typed_binding_candidates": [],
            "invariants": {
                "stable_current_source_pin_weakened": False,
                "drift_requires_revalidation": True,
                "title_used_as_discovery_only": True,
                "external_numeric_ids_used_as_identity_equality": False,
                "canonical_item_mutation": False,
                "project_v2_source_population": False,
                "presentation_asset_runtime_client_id_mutation": False,
                "gameplay_field_promotion": False,
                "raw_wikitext_or_prose_retained": False,
                "second_wiki_corroboration": "UNKNOWN",
                "ots_identity_or_semantics": "HYPOTHESIS_ONLY",
            },
        }
        manifest = {
            "schema": MANIFEST_SCHEMA,
            "status": evidence["status"],
            "artifact": {"schema": OUTPUT_SCHEMA, "sha256": sha256(canonical_bytes(evidence)), "bytes": len(canonical_bytes(evidence)), "committed_bulk_corpus": False},
            "candidate_count": len(evidence["rows"]),
            "selected_binding_candidate_count": 0,
            "dispositions": evidence["dispositions"],
            "source_capture_artifact_id": 10829932700,
            "source_capture_run_id": 36050042631,
            "source_capture_archive_sha256": "b28f0e5939a669ffe38ee14dc9e57e1b958f0c6ddc89ad75cf99908582469b16",
            "revalidation_required": True,
            "non_claims": ["NO_ITEM_DEFINITION_MUTATION", "NO_PROJECT_V2_SOURCE_BINDING_POPULATION", "NO_GAMEPLAY_FIELD_PROMOTION", "NO_PRESENTATION_ASSET_RUNTIME_OR_CLIENT_ID_MUTATION", "NO_PRODUCTION_AUTHORITY"],
        }
        return evidence, manifest
    for candidate in matched_rows:
        current_result = candidate.get("current_source")
        ids = current_result.get("candidate_page_ids") if isinstance(current_result, dict) else None
        if not isinstance(ids, list) or len(ids) != 1 or not isinstance(ids[0], int):
            raise PilotError("MATCHED_CANDIDATE_PAGE_PARTITION_INVALID")
        page_id = ids[0]
        external_id = str(page_id)
        target_key = candidate.get("native_key")
        source_id = candidate.get("source_item_id")
        if not isinstance(target_key, str) or not target_key.startswith("oteryn:item.") or not isinstance(source_id, int):
            raise PilotError("MATCHED_CANONICAL_IDENTITY_INVALID")
        target_page_ids[target_key].add(external_id)
        page_targets[external_id].add(target_key)
        page = pages_by_id.get(page_id)
        capture_row = capture_by_id.get(external_id)
        if capture_row is None:
            if page is None:
                raise PilotError("CANDIDATE_SOURCE_IDENTITY_ABSENT_FROM_BOTH_INPUTS")
            observed_tuple = {
                "source": page.get("source"), "source_role": page.get("source_role"),
                "source_namespace": "mediawiki/tibiawiki.com.br", "external_id": external_id,
                "page_key": PAGE_KEY_PREFIX + external_id, "title": page.get("title"),
                "revision_id": page.get("revision_id"), "revision_timestamp": page.get("revision_timestamp"),
                "source_digest": page.get("source_digest"),
            }
            preliminary.append({**observed_tuple, "target_key": target_key, "source_item_id": source_id, "status": "UNRESOLVED_SOURCE_SHAPE", "reason": "PAGE_ABSENT_FROM_PINNED_856_CAPTURE", "matched_signals": [], "contradicted_signals": [], "capture_identity_tuple": None})
            continue
        pinned_tuple = captured_identity_tuple(capture_row)
        if page is None:
            preliminary.append({**pinned_tuple, "target_key": target_key, "source_item_id": source_id, "status": "UNRESOLVED_SOURCE_SHAPE", "reason": "PAGE_ABSENT_FROM_CURRENT_SOURCE_OBSERVATION", "matched_signals": [], "contradicted_signals": [], "capture_identity_tuple": pinned_tuple})
            continue
        if (page.get("revision_id") != capture_row.get("revision_id")
                or page.get("source_digest") != capture_row.get("source_digest")
                or page.get("title") != capture_row.get("title")
                or page.get("revision_timestamp") != capture_row.get("revision_timestamp")):
            observed_tuple = {
                "source": page.get("source"), "source_role": page.get("source_role"),
                "source_namespace": "mediawiki/tibiawiki.com.br", "external_id": external_id,
                "page_key": PAGE_KEY_PREFIX + external_id, "title": page.get("title"),
                "revision_id": page.get("revision_id"), "revision_timestamp": page.get("revision_timestamp"),
                "source_digest": page.get("source_digest"),
            }
            preliminary.append({**pinned_tuple, "target_key": target_key, "source_item_id": source_id, "status": "UNRESOLVED_SOURCE_SHAPE", "reason": "EXACT_CAPTURE_REVISION_OR_DIGEST_DRIFT", "matched_signals": [], "contradicted_signals": [], "capture_identity_tuple": pinned_tuple, "observed_identity_tuple": observed_tuple})
            continue
        if page.get("infobox_present") is not True:
            preliminary.append({**pinned_tuple, "target_key": target_key, "source_item_id": source_id, "status": "UNRESOLVED_SOURCE_SHAPE", "reason": "NO_ITEM_INFOBOX_AT_PINNED_REVISION", "matched_signals": [], "contradicted_signals": [], "capture_identity_tuple": pinned_tuple})
            continue
        matched_signals, contradicted_signals = compare_fields(candidate, page, profiles)
        preliminary.append({
            "target_key": target_key,
            "source_item_id": source_id,
            **pinned_tuple,
            "status": "PENDING",
            "reason": "MULTI_SIGNAL_EVALUATION",
            "matched_signals": matched_signals,
            "contradicted_signals": contradicted_signals,
            "capture_identity_tuple": pinned_tuple,
        })

    for row in preliminary:
        if row["status"] == "UNRESOLVED_SOURCE_SHAPE":
            continue
        status, reason = disposition(
            matched=row["matched_signals"],
            contradicted=row["contradicted_signals"],
            page_conflict=row["external_id"] in page_targets and len(page_targets[row["external_id"]]) > 1,
            target_ambiguous=row["target_key"] in target_page_ids and len(target_page_ids[row["target_key"]]) > 1,
        )
        row.update(status=status, reason=reason)

    if not preliminary:
        raise PilotError("NO_PROTECTED_WIKI_MATCHED_CANDIDATES")
    # The legal pilot is one binding maximum: choose the strongest exact row,
    # then tie-break on lexical external ID and canonical key.
    exact = sorted((row for row in preliminary if row["status"] == "EXACT"), key=lambda row: (-len(row["matched_signals"]), int(row["external_id"]), row["target_key"]))
    selected = exact[:PILOT_BINDING_LIMIT]
    for row in preliminary:
        row["selected_for_pilot"] = row in selected
    source_batch = capture.get("batch_id")
    source_batch_digest = capture.get("source_batch_digest")
    if source_batch != EXPECTED_CAPTURE_BATCH or source_batch_digest != EXPECTED_CAPTURE_CENSUS_SHA256:
        raise PilotError("CAPTURE_SOURCE_REVISION_INVALID")
    evidence = {
        "schema": OUTPUT_SCHEMA,
        "profile": PROFILE,
        "source_capture": {
            "artifact_sha256": EXPECTED_CAPTURE_SHA256,
            "rows_sha256": EXPECTED_CAPTURE_ROWS_SHA256,
            "batch_id": source_batch,
            "source_batch_digest": source_batch_digest,
            "captured_rows": EXPECTED_CAPTURE_ROWS,
        },
        "canonical_inputs": {
            "canonical_item_count": EXPECTED_ITEM_COUNT,
            "canonical_definition_revision": "definition-r1",
            "protected_crosswalk_sha256": EXPECTED_CROSSWALK_SHA256,
            "current_source_sha256": sha256(canonical_bytes(current)),
            "current_source_stable_sha256": stable_current_digest(current),
        },
        "selection": {
            "candidate_pool": "the exact 22 protected WIKI_MATCHED rows from the current-source partition",
            "candidate_pool_count": len(matched_rows),
            "selection_rule": "sort eligible EXACT rows by descending independent signal count, then decimal MediaWiki page_id, then canonical key; select at most one",
            "selected_count": len(selected),
            "maximum_binding_candidates": PILOT_BINDING_LIMIT,
            "minimum_non_title_agreements": MIN_NON_TITLE_AGREEMENTS,
        },
        "dispositions": dict(sorted(Counter(row["status"] for row in preliminary).items())),
        "rows": sorted(preliminary, key=lambda row: (int(row["external_id"]), row["target_key"])),
        "typed_binding_candidates": [
            typed_binding_candidate(row["target_key"], source_batch, row["external_id"])
            for row in selected
        ],
        "invariants": {
            "title_used_as_discovery_only": True,
            "external_numeric_ids_used_as_identity_equality": False,
            "exact_requires_unique_target": True,
            "exact_requires_two_non_title_agreements": True,
            "exact_requires_zero_contradictions": True,
            "non_exact_binding_candidates_emitted": False,
            "canonical_item_mutation": False,
            "project_v2_source_population": False,
            "presentation_asset_runtime_client_id_mutation": False,
            "gameplay_field_promotion": False,
            "raw_wikitext_or_prose_retained": False,
            "second_wiki_corroboration": "UNKNOWN",
            "ots_identity_or_semantics": "HYPOTHESIS_ONLY",
        },
        "status": "CANDIDATE_EVIDENCE_ONLY_NO_REPO_POPULATION",
    }
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "status": evidence["status"],
        "artifact": {"schema": OUTPUT_SCHEMA, "sha256": sha256(canonical_bytes(evidence)), "bytes": len(canonical_bytes(evidence)), "committed_bulk_corpus": False},
        "candidate_count": len(evidence["rows"]),
        "selected_binding_candidate_count": len(evidence["typed_binding_candidates"]),
        "dispositions": evidence["dispositions"],
        "source_capture_artifact_id": 10829932700,
        "source_capture_run_id": 36050042631,
        "source_capture_archive_sha256": "b28f0e5939a669ffe38ee14dc9e57e1b958f0c6ddc89ad75cf99908582469b16",
        "non_claims": ["NO_ITEM_DEFINITION_MUTATION", "NO_PROJECT_V2_SOURCE_BINDING_POPULATION", "NO_GAMEPLAY_FIELD_PROMOTION", "NO_PRESENTATION_ASSET_RUNTIME_OR_CLIENT_ID_MUTATION", "NO_PRODUCTION_AUTHORITY"],
    }
    return evidence, manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-capture", type=Path, required=True)
    parser.add_argument("--source-manifest", type=Path, required=True)
    parser.add_argument("--current-source", type=Path, required=True)
    parser.add_argument("--current-source-manifest", type=Path, required=True)
    parser.add_argument("--protected-current-source-manifest", type=Path, required=True)
    parser.add_argument("--crosswalk", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    capture, _ = read_json(args.source_capture)
    capture_manifest, _ = read_json(args.source_manifest)
    current, _ = read_json(args.current_source)
    current_manifest, _ = read_json(args.current_source_manifest)
    protected_manifest, _ = read_json(args.protected_current_source_manifest)
    crosswalk, crosswalk_raw = read_json(args.crosswalk)
    evidence, manifest = compile_pilot(capture, capture_manifest, current, current_manifest, protected_manifest, crosswalk, crosswalk_raw)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(evidence))
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    print(f"G4 Item exact-binding pilot: PASS candidates={manifest['candidate_count']} selected={manifest['selected_binding_candidate_count']} dispositions={manifest['dispositions']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
