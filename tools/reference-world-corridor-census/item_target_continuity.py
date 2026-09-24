#!/usr/bin/env python3
"""Build a bounded target-continuity bridge for already corroborated Item fields.

This generation consumes protected Item verification outputs and reuses the protected
TibiaWiki Item parser/normalizer. It never resolves identity from names, never emits
PROVEN continuity, and never performs semantic promotion.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from collections import Counter
from pathlib import Path
from typing import Any, Iterable

TARGET_COUNT = 38_157
TARGET_DATE = "2026-07-28"
TARGET_DAY_START = "2026-07-28T00:00:00Z"
TARGET_DAY_END = "2026-07-28T23:59:59Z"

SCHEMA = "OTERYN_ITEM_TARGET_CONTINUITY/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_TARGET_CONTINUITY_MANIFEST/v1"
PROFILE = "OTERYN_ITEM_TARGET_CONTINUITY_BRIDGE/v1"

FIELD_VERIFICATION_SCHEMA = "OTERYN_ITEM_FIELD_VERIFICATION/v1"
FIELD_VERIFICATION_MANIFEST_SCHEMA = "OTERYN_ITEM_FIELD_VERIFICATION_MANIFEST/v1"
CURRENT_SOURCE_SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI/v1"
CURRENT_SOURCE_PROFILE = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_COLLECTOR/v1"
CURRENT_SOURCE_ID = "TIBIAWIKI_STRUCTURED"

# Protected lineage after #770/#771.
FIELD_VERIFICATION_MANIFEST_PATH = (
    "docs/agents/evidence/OTV2-20260922-content-world-item-field-verification.json"
)
CURRENT_SOURCE_COLLECTOR_PATH = (
    "tools/reference-world-corridor-census/item_current_source_tibiawiki.py"
)
PROTECTED_CURRENT_SOURCE_COLLECTOR_SHA256 = (
    "ef160e7b76458029064d7f50d7808fa80add99da96d2a5ebacbbc7c6426ba748"
)

MAX_CANDIDATE_FIELDS = 256
MAX_CANDIDATE_PAGES = 64
MAX_TARGET_DAY_REVISIONS = 128
MAX_HISTORY_CACHE_BYTES = 2 * 1024 * 1024

CONTINUITY_STATES = ("DERIVED", "UNKNOWN", "CONFLICT")


class ContinuityError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_json(path: Path) -> tuple[dict[str, Any], bytes]:
    payload = path.read_bytes()
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise ContinuityError(f"JSON_INVALID:{path}") from exc
    if not isinstance(value, dict):
        raise ContinuityError(f"JSON_ROOT_NOT_OBJECT:{path}")
    return value, payload


def load_protected_collector(game_root: Path):
    path = (game_root / CURRENT_SOURCE_COLLECTOR_PATH).resolve()
    payload = path.read_bytes()
    digest = sha256_bytes(payload.replace(b"\r\n", b"\n"))
    if digest != PROTECTED_CURRENT_SOURCE_COLLECTOR_SHA256:
        raise ContinuityError(
            f"PROTECTED_COLLECTOR_DIGEST_MISMATCH:{digest}"
        )
    spec = importlib.util.spec_from_file_location("oteryn_item_current_source", path)
    if spec is None or spec.loader is None:
        raise ContinuityError("PROTECTED_COLLECTOR_IMPORT_SPEC_FAILED")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def current_page_map(current: dict[str, Any]) -> dict[int, dict[str, Any]]:
    if current.get("schema") != CURRENT_SOURCE_SCHEMA:
        raise ContinuityError("CURRENT_SOURCE_SCHEMA_MISMATCH")
    if current.get("collector_profile") != CURRENT_SOURCE_PROFILE:
        raise ContinuityError("CURRENT_SOURCE_PROFILE_MISMATCH")
    if current.get("target_cut") != TARGET_DATE:
        raise ContinuityError("CURRENT_SOURCE_TARGET_MISMATCH")
    source = current.get("source")
    if not isinstance(source, dict) or source.get("id") != CURRENT_SOURCE_ID:
        raise ContinuityError("CURRENT_SOURCE_ID_MISMATCH")
    pages = current.get("pages")
    if not isinstance(pages, list):
        raise ContinuityError("CURRENT_SOURCE_PAGES_INVALID")
    out: dict[int, dict[str, Any]] = {}
    for page in pages:
        if not isinstance(page, dict):
            raise ContinuityError("CURRENT_PAGE_NOT_OBJECT")
        page_id = page.get("page_id")
        if (
            not isinstance(page_id, int)
            or isinstance(page_id, bool)
            or page_id <= 0
            or page_id in out
        ):
            raise ContinuityError("CURRENT_PAGE_ID_INVALID")
        out[page_id] = page
    return out


def current_record_map(current: dict[str, Any]) -> dict[str, dict[str, Any]]:
    records = current.get("records")
    if not isinstance(records, list) or len(records) != TARGET_COUNT:
        raise ContinuityError("CURRENT_SOURCE_RECORD_COUNT_MISMATCH")
    out: dict[str, dict[str, Any]] = {}
    source_ids: set[int] = set()
    for record in records:
        if not isinstance(record, dict):
            raise ContinuityError("CURRENT_RECORD_NOT_OBJECT")
        native_key = record.get("native_key")
        source_id = record.get("source_item_id")
        if (
            not isinstance(native_key, str)
            or not isinstance(source_id, int)
            or isinstance(source_id, bool)
            or native_key in out
            or source_id in source_ids
        ):
            raise ContinuityError("CURRENT_RECORD_IDENTITY_INVALID")
        out[native_key] = record
        source_ids.add(source_id)
    return out


def candidate_fields(
    verification: dict[str, Any],
    current: dict[str, Any],
) -> list[dict[str, Any]]:
    if verification.get("schema") != FIELD_VERIFICATION_SCHEMA:
        raise ContinuityError("FIELD_VERIFICATION_SCHEMA_MISMATCH")
    if verification.get("target_cut") != TARGET_DATE:
        raise ContinuityError("FIELD_VERIFICATION_TARGET_MISMATCH")
    records = verification.get("records")
    if not isinstance(records, list) or len(records) != TARGET_COUNT:
        raise ContinuityError("FIELD_VERIFICATION_RECORD_COUNT_MISMATCH")

    current_records = current_record_map(current)
    current_pages = current_page_map(current)
    candidates: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()

    for record in records:
        if not isinstance(record, dict):
            raise ContinuityError("FIELD_VERIFICATION_RECORD_NOT_OBJECT")
        native_key = record.get("native_key")
        source_id = record.get("source_item_id")
        if not isinstance(native_key, str) or not isinstance(source_id, int):
            raise ContinuityError("FIELD_VERIFICATION_IDENTITY_INVALID")
        current_record = current_records.get(native_key)
        if current_record is None or current_record.get("source_item_id") != source_id:
            raise ContinuityError("FIELD_CURRENT_IDENTITY_JOIN_MISMATCH")
        overrides = record.get("field_overrides")
        if not isinstance(overrides, dict):
            raise ContinuityError("FIELD_OVERRIDES_INVALID")
        current_disposition = current_record.get("current_source")
        if not isinstance(current_disposition, dict):
            raise ContinuityError("CURRENT_DISPOSITION_INVALID")
        page_ids = current_disposition.get("candidate_page_ids")
        if not isinstance(page_ids, list):
            raise ContinuityError("CURRENT_CANDIDATE_PAGE_IDS_INVALID")

        for field_path in sorted(overrides):
            field = overrides[field_path]
            if (
                not isinstance(field, dict)
                or field.get("field_state") != "CORROBORATED_CURRENT"
            ):
                continue
            if field.get("continuity_to_target") not in {"UNKNOWN", "DERIVED"}:
                continue
            observations = field.get("observations")
            if not isinstance(observations, list):
                raise ContinuityError("CORROBORATED_OBSERVATIONS_INVALID")
            wiki = [
                item
                for item in observations
                if isinstance(item, dict)
                and item.get("source") == CURRENT_SOURCE_ID
                and item.get("authority") == "STRUCTURED_REFERENCE_DATA"
            ]
            if len(wiki) != 1:
                raise ContinuityError("CORROBORATED_WIKI_OBSERVATION_CARDINALITY")
            observation = wiki[0]
            page_id = observation.get("page_id")
            source_field = observation.get("source_field")
            if (
                not isinstance(page_id, int)
                or page_id not in current_pages
                or page_id not in page_ids
                or not isinstance(source_field, str)
                or not source_field
            ):
                raise ContinuityError("CORROBORATED_WIKI_OBSERVATION_BINDING")
            key = (native_key, field_path)
            if key in seen:
                raise ContinuityError("DUPLICATE_CONTINUITY_CANDIDATE")
            seen.add(key)
            candidates.append(
                {
                    "source_item_id": source_id,
                    "native_key": native_key,
                    "field_path": field_path,
                    "page_id": page_id,
                    "source_field": source_field,
                    "current_value": observation.get("value"),
                    "current_revision_id": observation.get("revision_id"),
                    "current_revision_timestamp": observation.get("revision_timestamp"),
                }
            )

    if len(candidates) > MAX_CANDIDATE_FIELDS:
        raise ContinuityError(
            f"CANDIDATE_FIELD_LIMIT_EXCEEDED:{len(candidates)}"
        )
    page_count = len({item["page_id"] for item in candidates})
    if page_count > MAX_CANDIDATE_PAGES:
        raise ContinuityError(f"CANDIDATE_PAGE_LIMIT_EXCEEDED:{page_count}")
    return candidates


def _extract_revision_content(raw: dict[str, Any], collector) -> dict[str, Any]:
    revid = raw.get("revid")
    timestamp = raw.get("timestamp")
    slots = raw.get("slots")
    if (
        not isinstance(revid, int)
        or isinstance(revid, bool)
        or revid <= 0
        or not isinstance(timestamp, str)
        or not timestamp.endswith("Z")
        or not isinstance(slots, dict)
        or not isinstance(slots.get("main"), dict)
    ):
        raise ContinuityError("HISTORY_REVISION_SHAPE_INVALID")
    content = slots["main"].get("content")
    if not isinstance(content, str):
        return {
            "revision_id": revid,
            "revision_timestamp": timestamp,
            "source_digest": None,
            "parse_state": "UNPARSED",
            "parse_error": "WIKITEXT_NOT_STRING",
            "normalized_fields": {},
            "infobox_present": False,
        }
    source_digest = sha256_bytes(content.encode("utf-8"))
    try:
        collector.bounded_text(
            content,
            label="WIKITEXT",
            max_bytes=collector.MAX_WIKITEXT_BYTES,
        )
        extracted = collector.extract_infobox_item(content)
    except collector.CurrentSourceError as exc:
        return {
            "revision_id": revid,
            "revision_timestamp": timestamp,
            "source_digest": source_digest,
            "parse_state": "UNPARSED",
            "parse_error": str(exc).split(":", 1)[0],
            "normalized_fields": {},
            "infobox_present": False,
        }
    return {
        "revision_id": revid,
        "revision_timestamp": timestamp,
        "source_digest": source_digest,
        "parse_state": "PARSED",
        "normalized_fields": extracted["mapped"],
        "infobox_present": extracted["infobox_present"],
    }


def _fetch_revision_query(
    client,
    collector,
    *,
    page_id: int,
    rvstart: str,
    rvend: str | None,
    rvdir: str,
    rvlimit: int,
    allow_continuation_after_first_page: bool = False,
) -> list[dict[str, Any]]:
    params = {
        "action": "query",
        "prop": "revisions",
        "pageids": str(page_id),
        "rvprop": "ids|timestamp|content",
        "rvslots": "main",
        "rvstart": rvstart,
        "rvdir": rvdir,
        "rvlimit": str(rvlimit),
        "format": "json",
        "formatversion": "2",
    }
    if rvend is not None:
        params["rvend"] = rvend
    value = client.get_json(params)
    if not isinstance(value, dict):
        raise ContinuityError("HISTORY_API_ROOT_INVALID")
    if "continue" in value and not allow_continuation_after_first_page:
        raise ContinuityError("HISTORY_QUERY_CONTINUATION_EXCEEDS_BOUND")
    query = value.get("query")
    pages = query.get("pages") if isinstance(query, dict) else None
    if not isinstance(pages, list) or len(pages) != 1:
        raise ContinuityError("HISTORY_API_PAGE_PARTITION_INVALID")
    page = pages[0]
    if (
        not isinstance(page, dict)
        or page.get("pageid") != page_id
        or page.get("missing") is True
    ):
        raise ContinuityError("HISTORY_API_PAGE_ID_INVALID")
    revisions = page.get("revisions", [])
    if revisions is None:
        revisions = []
    if not isinstance(revisions, list) or len(revisions) > rvlimit:
        raise ContinuityError("HISTORY_API_REVISION_COUNT_INVALID")
    return [_extract_revision_content(raw, collector) for raw in revisions]


def fetch_page_history(client, collector, page_id: int) -> dict[str, Any]:
    pre = _fetch_revision_query(
        client,
        collector,
        page_id=page_id,
        rvstart="2026-07-27T23:59:59Z",
        rvend=None,
        rvdir="older",
        rvlimit=1,
        allow_continuation_after_first_page=True,
    )
    day = _fetch_revision_query(
        client,
        collector,
        page_id=page_id,
        rvstart=TARGET_DAY_START,
        rvend=TARGET_DAY_END,
        rvdir="newer",
        rvlimit=MAX_TARGET_DAY_REVISIONS,
    )
    if len(pre) > 1 or len(day) > MAX_TARGET_DAY_REVISIONS:
        raise ContinuityError("HISTORY_BOUND_INVARIANT_FAILED")
    return {
        "schema": "OTERYN_ITEM_TARGET_CONTINUITY_HISTORY_CACHE/v1",
        "target_date": TARGET_DATE,
        "page_id": page_id,
        "pre_target_revision": pre[0] if pre else None,
        "target_day_revisions": day,
    }


def cache_path(cache_dir: Path, page_id: int) -> Path:
    return cache_dir / f"page-{page_id}-target-{TARGET_DATE}.json"


def load_history_cache(cache_dir: Path, page_id: int) -> dict[str, Any] | None:
    path = cache_path(cache_dir, page_id)
    if not path.exists():
        return None
    payload = path.read_bytes()
    if len(payload) > MAX_HISTORY_CACHE_BYTES:
        raise ContinuityError("HISTORY_CACHE_MAX_PLUS_ONE")
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise ContinuityError("HISTORY_CACHE_JSON_INVALID") from exc
    if (
        not isinstance(value, dict)
        or value.get("page_id") != page_id
        or value.get("target_date") != TARGET_DATE
    ):
        raise ContinuityError("HISTORY_CACHE_BINDING_MISMATCH")
    return value


def write_history_cache(cache_dir: Path, value: dict[str, Any]) -> None:
    payload = canonical_bytes(value)
    if len(payload) > MAX_HISTORY_CACHE_BYTES:
        raise ContinuityError("HISTORY_CACHE_MAX_PLUS_ONE")
    cache_dir.mkdir(parents=True, exist_ok=True)
    path = cache_path(cache_dir, int(value["page_id"]))
    tmp = path.with_suffix(".tmp")
    tmp.write_bytes(payload)
    tmp.replace(path)


def collect_histories(
    page_ids: Iterable[int],
    *,
    cache_dir: Path,
    client,
    collector,
) -> dict[int, dict[str, Any]]:
    out: dict[int, dict[str, Any]] = {}
    for page_id in sorted(set(page_ids)):
        cached = load_history_cache(cache_dir, page_id)
        if cached is None:
            cached = fetch_page_history(client, collector, page_id)
            write_history_cache(cache_dir, cached)
        out[page_id] = cached
    return out


def field_value(revision: dict[str, Any], source_field: str) -> tuple[bool, Any]:
    fields = revision.get("normalized_fields")
    field = fields.get(source_field) if isinstance(fields, dict) else None
    if not isinstance(field, dict) or field.get("state") != "VALUE":
        return False, None
    return True, field.get("value")


def classify_candidate(
    candidate: dict[str, Any],
    history: dict[str, Any],
) -> dict[str, Any]:
    current_value = candidate["current_value"]
    source_field = candidate["source_field"]
    pre = history.get("pre_target_revision")
    day = history.get("target_day_revisions")
    if not isinstance(day, list):
        raise ContinuityError("HISTORY_DAY_REVISIONS_INVALID")

    evidence: list[dict[str, Any]] = []
    if not isinstance(pre, dict):
        return {
            "continuity_to_target": "UNKNOWN",
            "reason": "NO_PRE_TARGET_WIKI_REVISION",
            "promotion_bridge": "BLOCKED",
            "evidence": evidence,
        }

    revisions = [pre, *day]
    missing = False
    conflict = False
    for revision in revisions:
        if not isinstance(revision, dict):
            raise ContinuityError("HISTORY_REVISION_NOT_OBJECT")
        present, value = field_value(revision, source_field)
        evidence.append(
            {
                "revision_id": revision.get("revision_id"),
                "revision_timestamp": revision.get("revision_timestamp"),
                "source_digest": revision.get("source_digest"),
                "source_field": source_field,
                "field_present": present,
                "value": value if present else None,
            }
        )
        if not present:
            missing = True
        elif value != current_value:
            conflict = True

    if conflict:
        return {
            "continuity_to_target": "CONFLICT",
            "reason": "HISTORICAL_WIKI_VALUE_DIFFERS_ACROSS_TARGET_WINDOW",
            "promotion_bridge": "BLOCKED",
            "evidence": evidence,
        }
    if missing:
        return {
            "continuity_to_target": "UNKNOWN",
            "reason": "HISTORICAL_WIKI_FIELD_MISSING_OR_UNPARSED",
            "promotion_bridge": "BLOCKED",
            "evidence": evidence,
        }
    return {
        "continuity_to_target": "DERIVED",
        "reason": (
            "PRE_TARGET_AND_ALL_TARGET_DAY_WIKI_REVISIONS_MATCH_CURRENT_"
            "CORROBORATED_VALUE"
        ),
        "promotion_bridge": "ELIGIBLE_FOR_SEMANTIC_PROMOTION_GENERATION",
        "evidence": evidence,
    }


def compile_continuity(
    verification: dict[str, Any],
    current: dict[str, Any],
    histories: dict[int, dict[str, Any]],
    *,
    protected_inputs: dict[str, str],
) -> dict[str, Any]:
    candidates = candidate_fields(verification, current)
    results: list[dict[str, Any]] = []
    counts: Counter[str] = Counter()
    per_field: Counter[str] = Counter()

    for candidate in candidates:
        history = histories.get(candidate["page_id"])
        if history is None:
            raise ContinuityError("CANDIDATE_HISTORY_MISSING")
        result = classify_candidate(candidate, history)
        state = result["continuity_to_target"]
        if state not in CONTINUITY_STATES:
            raise ContinuityError("CONTINUITY_STATE_INVALID")
        counts[state] += 1
        per_field[f"{candidate['field_path']}:{state}"] += 1
        results.append(
            {
                **candidate,
                **result,
            }
        )

    results.sort(
        key=lambda item: (
            item["native_key"],
            item["field_path"],
            item["page_id"],
        )
    )
    if len(results) != len(candidates):
        raise ContinuityError("CONTINUITY_RESULT_COUNT_MISMATCH")
    if sum(counts.values()) != len(results):
        raise ContinuityError("CONTINUITY_PARTITION_MISMATCH")
    return {
        "schema": SCHEMA,
        "profile": PROFILE,
        "target_date": TARGET_DATE,
        "authority": {
            "semantic_promotion": "FORBIDDEN_IN_THIS_GENERATION",
            "proven_continuity": "FORBIDDEN",
            "identity_resolution": "EXACT_PROTECTED_BINDING_ONLY",
            "source_parser": CURRENT_SOURCE_COLLECTOR_PATH,
        },
        "protected_inputs": protected_inputs,
        "counts": {
            "candidate_fields": len(candidates),
            "candidate_pages": len({item["page_id"] for item in candidates}),
            "continuity": {
                key: counts[key] for key in CONTINUITY_STATES
            },
            "field_state_input": "CORROBORATED_CURRENT",
            "per_field": dict(sorted(per_field.items())),
        },
        "records": results,
        "invariants": {
            "only_corrob_current_examined": True,
            "name_only_identity_resolution": False,
            "same_page_identity_preserved": True,
            "target_day_revision_window_complete_within_bound": True,
            "current_value_auto_equals_target_value": False,
            "proven_continuity_emitted": False,
            "semantic_promotion_performed": False,
        },
    }


def build_manifest(full: dict[str, Any], *, compiler_sha256: str) -> dict[str, Any]:
    return {
        "schema": MANIFEST_SCHEMA,
        "status": "TARGET_CONTINUITY_BRIDGE_COMPLETE_NO_SEMANTIC_PROMOTION",
        "profile": PROFILE,
        "target_date": TARGET_DATE,
        "compiler": {
            "path": (
                "tools/reference-world-corridor-census/"
                "item_target_continuity.py"
            ),
            "sha256": compiler_sha256,
        },
        "protected_inputs": full["protected_inputs"],
        "counts": full["counts"],
        "full_output": {
            "schema": SCHEMA,
            "sha256": sha256_bytes(canonical_bytes(full)),
            "committed_bulk_corpus": False,
        },
        "invariants": full["invariants"],
        "continuity_policy": {
            "max_classification": "DERIVED",
            "derived_requires": [
                "protected #770 field state CORROBORATED_CURRENT",
                "same protected current-source page identity",
                "last pre-target-day revision field equals current value",
                "every 2026-07-28 revision field equals current value",
                "no missing/unparsed field in the historical target window",
            ],
            "different_value": "CONFLICT",
            "missing_history_or_field": "UNKNOWN",
        },
        "limitations": [
            (
                "Historical TibiaWiki continuity is structured-reference evidence, "
                "not direct target-day Global observation; this bridge therefore "
                "never emits PROVEN."
            ),
            (
                "Only fields already CORROBORATED_CURRENT by the protected #770 "
                "atomic verifier are examined."
            ),
            (
                "This generation performs no canonical Project/Reference semantic "
                "mutation."
            ),
        ],
        "next_gate": "SEMANTIC_PROMOTION_FROM_DERIVED_CONTINUITY_FIELDS",
    }


def compile_files(args: argparse.Namespace) -> None:
    game_root = args.game_root.resolve()
    verification, verification_bytes = load_json(args.field_verification)
    current, current_bytes = load_json(args.current_source)
    field_manifest, field_manifest_bytes = load_json(
        game_root / FIELD_VERIFICATION_MANIFEST_PATH
    )
    if field_manifest.get("schema") != FIELD_VERIFICATION_MANIFEST_SCHEMA:
        raise ContinuityError("PROTECTED_FIELD_MANIFEST_SCHEMA_MISMATCH")
    collector = load_protected_collector(game_root)
    candidates = candidate_fields(verification, current)
    histories = collect_histories(
        (item["page_id"] for item in candidates),
        cache_dir=args.cache_dir,
        client=collector.ApiClient(),
        collector=collector,
    )
    protected_inputs = {
        "field_verification_manifest_sha256": sha256_bytes(field_manifest_bytes),
        "field_verification_observation_sha256": sha256_bytes(verification_bytes),
        "current_source_observation_sha256": sha256_bytes(current_bytes),
        "protected_current_source_collector_sha256": (
            PROTECTED_CURRENT_SOURCE_COLLECTOR_SHA256
        ),
    }
    full = compile_continuity(
        verification,
        current,
        histories,
        protected_inputs=protected_inputs,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(full))
    compiler_sha = sha256_bytes(
        Path(__file__).read_bytes().replace(b"\r\n", b"\n")
    )
    manifest = build_manifest(full, compiler_sha256=compiler_sha)
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    counts = full["counts"]["continuity"]
    print(
        "item-target-continuity: PASS "
        f"fields={full['counts']['candidate_fields']} "
        f"pages={full['counts']['candidate_pages']} "
        f"derived={counts['DERIVED']} "
        f"unknown={counts['UNKNOWN']} "
        f"conflict={counts['CONFLICT']} "
        f"digest={manifest['full_output']['sha256']}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--field-verification", type=Path, required=True)
    parser.add_argument("--current-source", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    compile_files(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
