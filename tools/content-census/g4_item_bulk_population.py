#!/usr/bin/env python3
"""Compile one live G4 Item source-to-canonical crosswalk batch.

The complete current Wiki census remains an ephemeral input. This compiler
retains only source identity/revision/digest, bounded match signals, typed
binding candidates and independently typed candidate field promotions.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
from typing import Any

PROFILE = "OTERYN_G4_ITEM_BULK_POPULATION/v1"
OUTPUT_SCHEMA = "OTERYN_G4_ITEM_BULK_POPULATION/v1"
MANIFEST_SCHEMA = "OTERYN_G4_ITEM_BULK_POPULATION_MANIFEST/v1"
CENSUS_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS/v1"
CENSUS_MANIFEST_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS_MANIFEST/v1"
CROSSWALK_SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK/v1"
NATIVE_MAP_SCHEMA = "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1"
SOURCE_KEY = "oteryn:source.tibiawiki"
SOURCE_ID = "TIBIAWIKI_STRUCTURED"
SOURCE_NAMESPACE = "mediawiki/tibiawiki.com.br"
IDENTITY_NAMESPACE = "mediawiki/page_id"
PAGE_KEY_PREFIX = SOURCE_NAMESPACE + "/page_id/"
EXPECTED_CANONICAL_ITEMS = 38_157
EXPECTED_DISCOVERY_PAGES = 6_918
MIN_NON_TITLE_AGREEMENTS = 2
MAX_ROWS = 20_000
MAX_EVIDENCE_CANDIDATES_PER_ROW = 64
HEX = re.compile(r"^[0-9a-f]{64}$")
KEY = re.compile(r"^oteryn:item\.[a-z0-9._-]+$")
REVISION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")

# Exactly the simple source fields already admitted for Item identity
# corroboration. No external numeric IDs or titles participate in equality.
COMPARABLE_FIELDS = {
    "attack": "attack",
    "defense": "defense",
    "extra_defense": "defensemod",
    "range": "range",
    "hit_chance": "hit",
    "armor": "armor",
    "charge_count": "charges",
    "capacity": "volume",
}

# Existing Reference Item semantic destinations. These are field promotion
# candidates only after the page has a globally unique EXACT binding.
PROMOTION_FIELDS = {
    "name": "presentation.name",
    "attack": "weapon.attack",
    "defense": "weapon.defense",
    "defensemod": "weapon.extra_defense",
    "range": "weapon.range_cells",
    "hit": "weapon.hit_chance",
    "armor": "protection.armor",
    "charges": "charges.count",
    "volume": "container.capacity",
}


class BulkError(ValueError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def read_object(path: Path) -> tuple[dict[str, Any], bytes]:
    payload = path.read_bytes()
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise BulkError(f"JSON_INVALID:{path.name}") from exc
    if not isinstance(value, dict):
        raise BulkError(f"JSON_ROOT_NOT_OBJECT:{path.name}")
    return value, payload


def require_digest(value: Any, label: str) -> str:
    if not isinstance(value, str) or not HEX.fullmatch(value):
        raise BulkError(f"{label}_INVALID")
    return value


def simple_integer(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and re.fullmatch(r"[+-]?\d+", value.strip()):
        try:
            return int(value.strip())
        except ValueError:
            return None
    return None


def protected_signals(record: dict[str, Any], profiles: dict[str, dict[str, Any]]) -> dict[str, int]:
    profile = profiles.get(record.get("source_profile_id"))
    observations = profile.get("candidate_observations") if isinstance(profile, dict) else None
    if not isinstance(observations, list):
        raise BulkError("SOURCE_PROFILE_MISSING")
    values: dict[str, int] = {}
    conflicts: set[str] = set()
    reverse = {native_field: source_field for source_field, native_field in COMPARABLE_FIELDS.items()}
    for observation in observations:
        if not isinstance(observation, dict) or observation.get("nested_values") not in (None, []):
            continue
        source_field = reverse.get(observation.get("native_field"))
        if source_field is None:
            continue
        number = simple_integer(observation.get("source_value"))
        if number is None:
            continue
        if source_field in values and values[source_field] != number:
            conflicts.add(source_field)
        values[source_field] = number
    for field in conflicts:
        values.pop(field, None)
    return values


def field_values(page: dict[str, Any]) -> dict[str, Any]:
    raw = page.get("normalized_fields")
    if not isinstance(raw, dict):
        return {}
    values: dict[str, Any] = {}
    for field, observation in raw.items():
        if isinstance(observation, dict) and observation.get("state") == "VALUE":
            values[field] = observation.get("value")
    return values


def verify_inputs(census: dict[str, Any], census_manifest: dict[str, Any], crosswalk: dict[str, Any], native_map: dict[str, Any]) -> tuple[list[dict[str, Any]], dict[str, dict[str, Any]], str, str]:
    if census.get("schema") != CENSUS_SCHEMA or census_manifest.get("schema") != CENSUS_MANIFEST_SCHEMA:
        raise BulkError("CENSUS_SCHEMA_INVALID")
    if census_manifest.get("status") != "WIKI_FIRST_SOURCE_EVIDENCE_ONLY_NO_IDENTITY_PROMOTION":
        raise BulkError("CENSUS_STATUS_INVALID")
    source = census.get("source")
    if not isinstance(source, dict) or source.get("id") != SOURCE_ID or source.get("role") != "STRUCTURED_REFERENCE_DATA":
        raise BulkError("CENSUS_SOURCE_INVALID")
    discovery = source.get("discovery")
    if not isinstance(discovery, dict) or discovery.get("kind") != "MEDIAWIKI_CATEGORYMEMBERS" or discovery.get("namespace") != 0:
        raise BulkError("CENSUS_DISCOVERY_INVALID")
    census_sha = digest(canonical_bytes(census))
    census_full = census_manifest.get("full_output")
    if not isinstance(census_full, dict) or census_full.get("sha256") != census_sha:
        raise BulkError("CENSUS_FULL_DIGEST_MISMATCH")
    stable = dict(census)
    stable.pop("retrieval_timestamp", None)
    stable_sha = digest(canonical_bytes(stable))
    if census_full.get("stable_without_retrieval_timestamp_sha256") != stable_sha:
        raise BulkError("CENSUS_STABLE_DIGEST_MISMATCH")
    pages = census.get("pages")
    counts = census.get("counts")
    if not isinstance(pages, list) or not pages or len(pages) > MAX_ROWS or not isinstance(counts, dict):
        raise BulkError("CENSUS_PAGES_INVALID")
    if len(pages) != counts.get("discovered_pages") or len(pages) != counts.get("fetched_pages"):
        raise BulkError("CENSUS_PARTITION_MISMATCH")
    if len(pages) != EXPECTED_DISCOVERY_PAGES:
        raise BulkError(f"LIVE_DISCOVERY_COUNT_DRIFT:{len(pages)}")
    seen_pages: set[int] = set()
    for page in pages:
        if not isinstance(page, dict):
            raise BulkError("PAGE_ROW_INVALID")
        page_id = page.get("page_id")
        revision_id = page.get("revision_id")
        title = page.get("title")
        if isinstance(page_id, bool) or not isinstance(page_id, int) or page_id <= 0 or page_id in seen_pages:
            raise BulkError("PAGE_ID_INVALID_OR_DUPLICATE")
        if isinstance(revision_id, bool) or not isinstance(revision_id, int) or revision_id <= 0:
            raise BulkError("PAGE_REVISION_INVALID")
        if not isinstance(title, str) or not title.strip() or len(title.encode("utf-8")) > 256:
            raise BulkError("PAGE_TITLE_INVALID")
        require_digest(page.get("source_digest"), "PAGE_SOURCE_DIGEST")
        if not isinstance(page.get("normalized_fields"), dict):
            raise BulkError("PAGE_NORMALIZED_FIELDS_INVALID")
        seen_pages.add(page_id)
    if crosswalk.get("schema") != CROSSWALK_SCHEMA:
        raise BulkError("CROSSWALK_SCHEMA_INVALID")
    records = crosswalk.get("records")
    profile_rows = crosswalk.get("source_profiles")
    if not isinstance(records, list) or len(records) != EXPECTED_CANONICAL_ITEMS or not isinstance(profile_rows, list):
        raise BulkError("CROSSWALK_PARTITION_INVALID")
    if native_map.get("schema") != NATIVE_MAP_SCHEMA:
        raise BulkError("NATIVE_MAP_SCHEMA_INVALID")
    map_rows = native_map.get("records")
    if not isinstance(map_rows, list) or len(map_rows) != EXPECTED_CANONICAL_ITEMS or native_map.get("item_count") != EXPECTED_CANONICAL_ITEMS:
        raise BulkError("NATIVE_MAP_PARTITION_INVALID")
    profiles: dict[str, dict[str, Any]] = {}
    for profile in profile_rows:
        if not isinstance(profile, dict) or not isinstance(profile.get("profile_id"), str) or profile["profile_id"] in profiles:
            raise BulkError("CROSSWALK_PROFILE_INVALID")
        profiles[profile["profile_id"]] = profile
    by_source: dict[int, dict[str, Any]] = {}
    for row in map_rows:
        if not isinstance(row, dict):
            raise BulkError("NATIVE_MAP_ROW_INVALID")
        source_id, key, revision = row.get("source_item_id"), row.get("native_key"), row.get("native_revision")
        if isinstance(source_id, bool) or not isinstance(source_id, int) or source_id <= 0 or source_id in by_source:
            raise BulkError("NATIVE_MAP_SOURCE_ID_INVALID_OR_DUPLICATE")
        if not isinstance(key, str) or not KEY.fullmatch(key) or not isinstance(revision, str) or not REVISION.fullmatch(revision):
            raise BulkError("NATIVE_MAP_TARGET_INVALID")
        by_source[source_id] = row
    if len({row["native_key"] for row in by_source.values()}) != EXPECTED_CANONICAL_ITEMS:
        raise BulkError("NATIVE_MAP_TARGET_DUPLICATE")
    candidate_by_key: dict[str, dict[str, Any]] = {}
    for record in records:
        if not isinstance(record, dict) or record.get("crosswalk_outcome") != "EXACT_ONE":
            raise BulkError("CROSSWALK_RECORD_INVALID")
        source_id = record.get("source_item_id")
        target = by_source.get(source_id) if isinstance(source_id, int) and not isinstance(source_id, bool) else None
        if target is None or record.get("native_key") != target["native_key"]:
            raise BulkError("CROSSWALK_NATIVE_MAP_JOIN_MISMATCH")
        if record.get("source_profile_id") not in profiles:
            raise BulkError("CROSSWALK_RECORD_PROFILE_MISSING")
        key = record["native_key"]
        if key in candidate_by_key:
            raise BulkError("CROSSWALK_TARGET_DUPLICATE")
        candidate_by_key[key] = {**record, "native_revision": target["native_revision"]}
    if len(candidate_by_key) != EXPECTED_CANONICAL_ITEMS:
        raise BulkError("CROSSWALK_TARGET_COVERAGE_INVALID")
    return pages, candidate_by_key, stable_sha, census_sha


def compile_bulk(census: dict[str, Any], census_manifest: dict[str, Any], crosswalk: dict[str, Any], native_map: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    pages, targets, stable_sha, census_sha = verify_inputs(census, census_manifest, crosswalk, native_map)
    profiles = {profile["profile_id"]: profile for profile in crosswalk["source_profiles"]}

    signals_by_target: dict[str, dict[str, int]] = {}
    inverted: dict[str, dict[int, list[str]]] = {field: defaultdict(list) for field in COMPARABLE_FIELDS}
    for target_key, target in targets.items():
        signals = protected_signals(target, profiles)
        signals_by_target[target_key] = signals
        for field, value in signals.items():
            inverted[field][value].append(target_key)

    provisional: list[dict[str, Any]] = []
    page_exact_claims: dict[str, set[str]] = defaultdict(set)
    target_claims: dict[str, set[str]] = defaultdict(set)
    page_payloads: dict[int, dict[str, Any]] = {}
    for page in pages:
        page_id = page["page_id"]
        external_id = str(page_id)
        values = field_values(page)
        numeric = {field: simple_integer(values.get(wiki_field)) for field, wiki_field in COMPARABLE_FIELDS.items()}
        numeric = {field: value for field, value in numeric.items() if value is not None}
        candidate_keys: set[str] = set()
        for field, value in numeric.items():
            candidate_keys.update(inverted[field].get(value, []))
        evaluated: list[dict[str, Any]] = []
        for target_key in sorted(candidate_keys):
            expected = signals_by_target[target_key]
            shared = sorted(set(expected) & set(numeric))
            matches = [field for field in shared if expected[field] == numeric[field]]
            contradictions = [field for field in shared if expected[field] != numeric[field]]
            if len(shared) < MIN_NON_TITLE_AGREEMENTS:
                strength = "WEAK"
            elif contradictions and len(matches) >= MIN_NON_TITLE_AGREEMENTS:
                strength = "CONFLICT"
            elif len(matches) >= MIN_NON_TITLE_AGREEMENTS:
                strength = "EXACT_CANDIDATE"
            else:
                strength = "PROBABLE"
            evaluated.append({
                "target_key": target_key,
                "target_revision": targets[target_key]["native_revision"],
                "matched_signals": matches,
                "contradicted_signals": contradictions,
                "comparable_signal_count": len(shared),
                "strength": strength,
            })
        exact = [row for row in evaluated if row["strength"] == "EXACT_CANDIDATE"]
        conflict = [row for row in evaluated if row["strength"] == "CONFLICT"]
        probable = [row for row in evaluated if row["strength"] == "PROBABLE"]
        if conflict:
            status, reason = "CONFLICT", "ONE_OR_MORE_MULTI_SIGNAL_TARGET_CONTRADICTIONS"
        elif len(exact) > 1:
            status, reason = "AMBIGUOUS", "MULTIPLE_MULTI_SIGNAL_TARGETS"
        elif len(exact) == 1:
            status, reason = "PENDING_UNIQUENESS", "ONE_MULTI_SIGNAL_TARGET_AWAITING_REVERSE_CHECK"
        elif probable:
            status, reason = "PROBABLE_MATCH", "ONE_OR_MORE_TARGETS_HAVE_ONLY_ONE_AGREEMENT"
        elif not candidate_keys:
            status, reason = "NO_MATCH", "NO_SHARED_NON_TITLE_SIGNAL"
        else:
            status, reason = "NO_MATCH", "INSUFFICIENT_NON_TITLE_AGREEMENTS"
        # Register every multi-signal exact candidate before deciding whether
        # this page itself is unique. An ambiguous page can still contest a
        # target claimed by an otherwise unique page, so reverse uniqueness
        # must be computed from the complete candidate relation.
        for candidate in exact:
            target_key = candidate["target_key"]
            page_exact_claims[external_id].add(target_key)
            target_claims[target_key].add(external_id)
        row = {
            "source": SOURCE_ID,
            "source_role": "STRUCTURED_REFERENCE_DATA",
            "source_namespace": SOURCE_NAMESPACE,
            "identity_namespace": IDENTITY_NAMESPACE,
            "external_id": external_id,
            "page_key": PAGE_KEY_PREFIX + external_id,
            "title": page["title"],
            "revision_id": page["revision_id"],
            "revision_timestamp": page["revision_timestamp"],
            "source_digest": page["source_digest"],
            "source_shape": page.get("source_shape"),
            "status": status,
            "reason": reason,
            "candidate_count": len(evaluated),
            "candidate_digest_sha256": digest(canonical_bytes(evaluated)),
            "candidates": evaluated[:MAX_EVIDENCE_CANDIDATES_PER_ROW],
        }
        provisional.append(row)
        page_payloads[page_id] = page

    binding_candidates: list[dict[str, Any]] = []
    by_external = {row["external_id"]: row for row in provisional}
    for external_id, target_keys_for_page in page_exact_claims.items():
        row = by_external[external_id]
        if row["status"] == "CONFLICT":
            continue
        if len(target_keys_for_page) != 1:
            row.update(status="AMBIGUOUS", reason="MULTIPLE_MULTI_SIGNAL_TARGETS")
            continue
        target_key = next(iter(target_keys_for_page))
        if len(target_claims.get(target_key, set())) != 1:
            row.update(status="AMBIGUOUS", reason="MULTIPLE_SOURCE_PAGES_CLAIM_ONE_TARGET")
            continue
        if row["status"] != "PENDING_UNIQUENESS":
            raise BulkError("EXACT_CANDIDATE_STATUS_INVALID")
        row.update(status="EXACT", reason="UNIQUE_TARGET_TWO_OR_MORE_NON_TITLE_AGREEMENTS_ZERO_CONFLICTS")
        binding_candidates.append({
            "source_key": SOURCE_KEY,
            "source_revision": "tibiawiki-item-census:" + stable_sha,
            "identity_namespace": IDENTITY_NAMESPACE,
            "external_id": external_id,
            "target": {"family": "Item", "key": target_key, "revision": targets[target_key]["native_revision"]},
            "disposition": "EXACT",
        })
    binding_candidates.sort(key=lambda row: (int(row["external_id"]), row["target"]["key"]))
    target_keys = [(row["source_key"], row["source_revision"], row["identity_namespace"], row["external_id"]) for row in binding_candidates]
    if len(set(target_keys)) != len(target_keys) or len({row["target"]["key"] for row in binding_candidates}) != len(binding_candidates):
        raise BulkError("BINDING_GLOBAL_UNIQUENESS_FAILED")

    semantic_rows: list[dict[str, Any]] = []
    for row in provisional:
        if row["status"] != "EXACT":
            continue
        page = page_payloads[int(row["external_id"])]
        values = field_values(page)
        target_binding = next(binding for binding in binding_candidates if binding["external_id"] == row["external_id"])
        for source_field, field_path in PROMOTION_FIELDS.items():
            if source_field not in values:
                continue
            source_value = values[source_field]
            typed = typed_value(field_path, source_value)
            semantic_rows.append({
                "target": target_binding["target"],
                "field_path": field_path,
                "source_key": SOURCE_KEY,
                "source_revision": "tibiawiki-item-census:" + stable_sha,
                "source_identity_namespace": IDENTITY_NAMESPACE,
                "source_external_id": row["external_id"],
                "source_page_revision_id": row["revision_id"],
                "source_page_digest": row["source_digest"],
                "source_value": source_value,
                "typed_value": typed,
                "promotion_state": "FIELD_VERIFIED_CANDIDATE_NOT_APPLIED",
            })
    semantic_rows.sort(key=lambda row: (row["target"]["key"], row["field_path"]))
    if len({(row["target"]["key"], row["field_path"]) for row in semantic_rows}) != len(semantic_rows):
        raise BulkError("SEMANTIC_PROMOTION_DUPLICATE_FIELD")

    dispositions = dict(sorted(Counter(row["status"] for row in provisional).items()))
    evidence = {
        "schema": OUTPUT_SCHEMA,
        "profile": PROFILE,
        "status": "LIVE_BULK_CROSSWALK_CANDIDATES_NOT_PRODUCTION_POPULATED",
        "source": {
            "key": SOURCE_KEY,
            "id": SOURCE_ID,
            "identity_namespace": IDENTITY_NAMESPACE,
            "source_revision": "tibiawiki-item-census:" + stable_sha,
            "census_sha256": census_sha,
            "stable_census_sha256": stable_sha,
            "page_count": len(provisional),
        },
        "canonical_targets": {
            "count": len(targets),
            "native_map_sha256": crosswalk.get("canonical_native_map", {}).get("sha256"),
            "crosswalk_records_sha256": crosswalk.get("full_record_output", {}).get("records_sha256"),
            "allocation_digest_sha256": crosswalk.get("allocation_digest_sha256"),
            "opaque_registry_allocations": crosswalk.get("counts", {}).get("opaque_registry_allocations"),
        },
        "matching": {
            "algorithm": "INVERTED_INDEXED_MULTI_SIGNAL_COMPARISON",
            "evaluated_page_count": len(provisional),
            "canonical_target_count": len(targets),
            "theoretical_pair_space": len(provisional) * len(targets),
            "minimum_non_title_agreements": MIN_NON_TITLE_AGREEMENTS,
            "external_numeric_identity_equality_used": False,
            "title_used_as_identity_signal": False,
            "candidate_cap_per_row": MAX_EVIDENCE_CANDIDATES_PER_ROW,
        },
        "counts": {
            "dispositions": dispositions,
            "typed_binding_candidates": len(binding_candidates),
            "semantic_field_candidates": len(semantic_rows),
            "semantic_field_candidates_by_path": dict(sorted(Counter(row["field_path"] for row in semantic_rows).items())),
        },
        "rows": sorted(provisional, key=lambda row: (int(row["external_id"]), row["title"])),
        "typed_binding_candidates": binding_candidates,
        "semantic_field_candidates": semantic_rows,
        "invariants": {
            "one_live_census_used_for_identity_and_fields": True,
            "raw_wikitext_or_long_form_prose_retained": False,
            "new_canonical_identity_minted": False,
            "v2_source_bindings_persisted": False,
            "reference_item_semantics_applied": False,
            "existing_known_reference_values_overwritten": False,
            "ots_identity_or_semantics_authority": "HYPOTHESIS_ONLY",
            "second_wiki_corroboration": "UNKNOWN",
            "ambiguous_conflict_probable_no_match_not_bound": True,
        },
        "non_claims": ["NO_PROJECT_V2_PACKAGE_POPULATION", "NO_REFERENCE_RUNTIME_PROMOTION", "NO_PRESENTATION_ASSET_RUNTIME_OR_CLIENT_ID_CHANGE", "NO_PRODUCTION_AUTHORITY"],
    }
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "status": evidence["status"],
        "artifact": {"schema": OUTPUT_SCHEMA, "sha256": digest(canonical_bytes(evidence)), "bytes": len(canonical_bytes(evidence)), "committed_bulk_corpus": False},
        "source_stable_sha256": stable_sha,
        "source_page_count": len(provisional),
        "canonical_target_count": len(targets),
        "dispositions": dispositions,
        "typed_binding_candidate_count": len(binding_candidates),
        "semantic_field_candidate_count": len(semantic_rows),
        "reproduction": "tools/content-census/g4_item_bulk_population.py",
    }
    return evidence, manifest


def typed_value(field_path: str, value: Any) -> dict[str, Any]:
    if field_path == "presentation.name":
        if not isinstance(value, str) or not value.strip() or len(value.encode("utf-8")) > 2048:
            raise BulkError("PRESENTATION_NAME_INVALID")
        return {"kind": "TEXT", "value": value}
    if field_path in {"weapon.attack", "weapon.defense", "weapon.extra_defense", "protection.armor"}:
        number = simple_integer(value)
        if number is None or number < -(2**31) or number > 2**31 - 1:
            raise BulkError("SIGNED_POINTS_INVALID")
        return {"kind": "SIGNED_POINTS", "value": number}
    if field_path == "weapon.range_cells":
        number = simple_integer(value)
        if number is None or number < 0 or number > 2**16 - 1:
            raise BulkError("CELLS_INVALID")
        return {"kind": "CELLS", "value": number}
    if field_path == "weapon.hit_chance":
        number = simple_integer(value)
        if number is None:
            raise BulkError("RATIONAL_PERCENT_INVALID")
        import math
        divisor = math.gcd(abs(number), 100)
        return {"kind": "RATIONAL_PERCENT", "numerator": number // divisor, "denominator": 100 // divisor, "source_unit": "PERCENT_POINTS"}
    if field_path == "charges.count":
        number = simple_integer(value)
        if number is None or number < 0 or number > 2**32 - 1:
            raise BulkError("COUNT_INVALID")
        return {"kind": "COUNT_U32", "value": number}
    if field_path == "container.capacity":
        number = simple_integer(value)
        if number is None or number < 0 or number > 2**16 - 1:
            raise BulkError("CAPACITY_INVALID")
        return {"kind": "CAPACITY_U16", "value": number}
    raise BulkError("UNSUPPORTED_PROMOTION_FIELD")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--census", type=Path, required=True)
    parser.add_argument("--census-manifest", type=Path, required=True)
    parser.add_argument("--crosswalk", type=Path, required=True)
    parser.add_argument("--native-map", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    parser.add_argument("--promotion-output", type=Path, required=True)
    args = parser.parse_args()
    census, _ = read_object(args.census)
    census_manifest, _ = read_object(args.census_manifest)
    crosswalk, _ = read_object(args.crosswalk)
    native_map, _ = read_object(args.native_map)
    evidence, manifest = compile_bulk(census, census_manifest, crosswalk, native_map)
    promotion = {
        "schema": "OTERYN_G4_ITEM_BULK_SEMANTIC_PROMOTION_CANDIDATES/v1",
        "status": "CANDIDATES_NOT_APPLIED",
        "source": evidence["source"],
        "candidate_count": len(evidence["semantic_field_candidates"]),
        "candidates": evidence["semantic_field_candidates"],
        "invariants": {"only_unique_exact_identity_rows": True, "current_source_fields_only": True, "known_reference_fields_overwritten": False, "application_performed": False},
    }
    for path, value in ((args.output, evidence), (args.manifest_output, manifest), (args.promotion_output, promotion)):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(canonical_bytes(value))
    print("g4-item-bulk-population: PASS " + json.dumps({"pages": manifest["source_page_count"], "targets": manifest["canonical_target_count"], "bindings": manifest["typed_binding_candidate_count"], "semantic_candidates": manifest["semantic_field_candidate_count"], "dispositions": manifest["dispositions"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
