#!/usr/bin/env python3
"""Deterministic, evidence-only disposition of the pinned G3 UNKNOWN cohort."""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import pathlib
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile

LEDGER_SHA256 = "117c3e6ae487b4d29a68042d9c3507ae965d569b31d5472c6b4e886a96789392"
G3_UNIVERSE_SHA256 = "1562382c66ad471a46309eaa5ae2fe9e7b3b1e3c8c2d5fc047ee8faa65bd1666"
EXPECTED_SHAPES = {
    "STRUCTURED_PRIMARY": 3803,
    "NO_INFOBOX_ITEM": 1399,
    "STRUCTURED_ALTERNATE": 187,
    "REDIRECT": 104,
    "SOURCE_CLASSIFICATION_UNRESOLVED": 12,
    "INFOBOX_ITEM_PARSE_ERROR": 7,
}
EXPECTED_TEMPLATE_PARTITION = {
    "object_only": 3121,
    "object_world_quest_overlap": 1,
    "hunts": 521,
    "world_change": 12,
    "world_quest_only": 17,
    "residual": 131,
}
OBJECT_TEMPLATE = "Predefinição:Infobox Object"
WORLD_QUEST_TEMPLATE = "Predefinição:Infobox World Quest"
HUNTS_TEMPLATE = "Predefinição:Infobox Hunts"
WORLD_CHANGE_TEMPLATE = "Predefinição:Infobox World Change"
WORLD_CHANGE_SIGNATURE = {
    "family": "Encounter",
    "root": "world-changes",
    "surface": "Zmiany świata",
    "template": WORLD_CHANGE_TEMPLATE,
}
API = "https://www.tibiawiki.com.br/api.php"
MAX_CONTINUATIONS = 128
BATCH = 20


def canonical(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def validate_ledger(ledger: dict) -> list[dict]:
    if ledger.get("schema") != "OTERYN_UNKNOWN_DISPOSITION_LEDGER/v1":
        raise ValueError("unexpected ledger schema")
    source = ledger.get("source_artifact", {})
    if source.get("artifact_id") != 10801778929 or source.get("universe_sha256") != G3_UNIVERSE_SHA256:
        raise ValueError("pinned G3 identity mismatch")
    pages = ledger.get("pages")
    if not isinstance(pages, list) or len(pages) != 5512:
        raise ValueError("UNKNOWN row count mismatch")
    ids = [row.get("page_id") for row in pages]
    if any(not isinstance(v, int) or isinstance(v, bool) or v <= 0 for v in ids) or len(ids) != len(set(ids)):
        raise ValueError("UNKNOWN page IDs missing or duplicated")
    if any(row.get("source_family") != "UNKNOWN" or row.get("target_identity") is not None for row in pages):
        raise ValueError("pinned input contains a family or target assignment")
    forbidden = {"Kalkulatory", "Narzędzie do nasycania", "Dostawca"}
    if any(
        title.get("title") in forbidden
        for row in pages
        for title in row.get("title_observations", [])
    ):
        raise ValueError("hard-excluded title present in UNKNOWN ledger")
    shapes = collections.Counter(row.get("primary_source_shape") for row in pages)
    if dict(sorted(shapes.items())) != dict(sorted(EXPECTED_SHAPES.items())):
        raise ValueError(f"source-shape partition mismatch: {dict(shapes)}")
    if sha256(canonical(ledger)) != LEDGER_SHA256:
        raise ValueError("pinned ledger digest mismatch")
    return sorted(pages, key=lambda row: row["page_id"])


def _set(values) -> set[str]:
    return set(values) if isinstance(values, list) else set()


def pinned_template_partition(pages: list[dict]) -> dict[str, int]:
    primary = [r for r in pages if r["primary_source_shape"] == "STRUCTURED_PRIMARY"]
    counts = collections.Counter()
    for row in primary:
        p = row["lane_observations"][0]
        templates = _set(p.get("templates"))
        obj = OBJECT_TEMPLATE in templates
        quest = WORLD_QUEST_TEMPLATE in templates
        if obj and quest:
            counts["object_world_quest_overlap"] += 1
        elif obj:
            counts["object_only"] += 1
        elif HUNTS_TEMPLATE in templates:
            counts["hunts"] += 1
        elif WORLD_CHANGE_TEMPLATE in templates:
            counts["world_change"] += 1
        elif quest:
            counts["world_quest_only"] += 1
        else:
            counts["residual"] += 1
    if dict(sorted(counts.items())) != dict(sorted(EXPECTED_TEMPLATE_PARTITION.items())):
        raise ValueError(f"template partition mismatch: {dict(counts)}")
    return dict(sorted(counts.items()))


def _api_get(params: dict[str, str], timeout: int = 20) -> dict:
    url = API + "?" + urllib.parse.urlencode(params)
    request = urllib.request.Request(url, headers={"User-Agent": "Oteryn-G3-Unknown-Closure/1.0", "Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        value = json.load(response)
    if not isinstance(value, dict):
        raise ValueError("MediaWiki response is not an object")
    if isinstance(value.get("error"), dict):
        code = value["error"].get("code", "UNKNOWN")
        raise ValueError(f"MEDIAWIKI_API_ERROR:{code}")
    return value


def refresh_batch(page_ids: list[int]) -> dict[int, dict]:
    wanted = set(page_ids)
    params = {
        "action": "query",
        "pageids": "|".join(str(x) for x in page_ids),
        "prop": "info|revisions|categories|templates",
        "rvprop": "ids|timestamp",
        "cllimit": "max",
        "tllimit": "max",
        "format": "json",
        "formatversion": "2",
    }
    records: dict[int, dict] = {}
    for _ in range(MAX_CONTINUATIONS):
        result = _api_get(params)
        pages = result.get("query", {}).get("pages")
        if not isinstance(pages, list):
            raise ValueError("MediaWiki response lacks query.pages")
        for page in pages:
            if not isinstance(page, dict):
                raise ValueError("malformed MediaWiki page row")
            page_id = page.get("pageid")
            if page_id not in wanted:
                raise ValueError("MediaWiki returned an unrequested page ID")
            revision_rows = page.get("revisions", [])
            if len(revision_rows) != 1 or not isinstance(revision_rows[0], dict):
                raise ValueError(f"current revision missing for {page_id}")
            revision = revision_rows[0]
            categories = page.get("categories", [])
            templates = page.get("templates", [])
            if not isinstance(categories, list) or not isinstance(templates, list):
                raise ValueError(f"current category/template shape invalid for {page_id}")
            if not isinstance(page.get("title"), str):
                raise ValueError(f"current title invalid for {page_id}")
            current = {
                "page_id": page_id,
                "title": page.get("title"),
                "revision_id": revision.get("revid"),
                "revision_timestamp": revision.get("timestamp"),
                "redirect": bool(page.get("redirect") is True),
                "categories": sorted({r.get("title") for r in categories if isinstance(r, dict) and isinstance(r.get("title"), str)}),
                "templates": sorted({r.get("title") for r in templates if isinstance(r, dict) and isinstance(r.get("title"), str)}),
            }
            previous = records.get(page_id)
            if previous:
                if any(previous[k] != current[k] for k in ("title", "revision_id", "revision_timestamp", "redirect")):
                    raise ValueError(f"source drift during paginated refresh for {page_id}")
                previous["categories"] = sorted(set(previous["categories"]) | set(current["categories"]))
                previous["templates"] = sorted(set(previous["templates"]) | set(current["templates"]))
            else:
                records[page_id] = current
        continuation = result.get("continue")
        if not continuation:
            break
        if not isinstance(continuation, dict) or not continuation:
            raise ValueError("malformed continuation")
        params = {**params, **{str(k): str(v) for k, v in continuation.items()}}
    else:
        raise ValueError("MediaWiki continuation limit exceeded")
    if set(records) != wanted:
        raise ValueError("current source page-ID partition mismatch")
    return records


def refresh_universe(pages: list[dict], *, sleep_seconds: float = 0.05) -> tuple[dict[int, dict], dict]:
    observations: dict[int, dict] = {}
    failures = collections.Counter()
    ordered = [row["page_id"] for row in pages]
    for index in range(0, len(ordered), BATCH):
        ids = ordered[index:index + BATCH]
        try:
            observations.update(refresh_batch(ids))
        except urllib.error.HTTPError as exc:
            reason = f"HTTP_{exc.code}"
            failures[reason] += len(ids)
            if exc.code in (401, 403, 429):
                failures["REMAINING_UNQUERIED_AFTER_ACCESS_LIMIT"] += len(ordered) - index - len(ids)
                break
            for page_id in ids:
                observations[page_id] = {"refresh_error": reason}
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            reason = "NETWORK_UNAVAILABLE"
            failures[reason] += len(ids)
            for page_id in ids:
                observations[page_id] = {"refresh_error": reason, "detail": type(exc).__name__}
        except (ValueError, TypeError, KeyError, json.JSONDecodeError) as exc:
            error_text = str(exc)
            if error_text.startswith("MEDIAWIKI_API_ERROR:"):
                code = error_text.split(":", 1)[1]
                reason = f"API_{code.upper()}"
            else:
                reason = "INVALID_OR_DRIFTED_RESPONSE"
            failures[reason] += len(ids)
            if reason in {"API_READAPIDENIED", "API_MAXLAG", "API_RATELIMITED", "API_THROTTLED"}:
                failures["REMAINING_UNQUERIED_AFTER_ACCESS_LIMIT"] += len(ordered) - index - len(ids)
                break
            for page_id in ids:
                observations[page_id] = {"refresh_error": reason}
        if sleep_seconds:
            time.sleep(sleep_seconds)
    for page_id in ordered:
        observations.setdefault(page_id, {"refresh_error": "NOT_QUERIED_AFTER_ACCESS_LIMIT"})
    meta = {
        "attempted_pages": len(ordered) - failures.get("REMAINING_UNQUERIED_AFTER_ACCESS_LIMIT", 0),
        "verified_same_revision_pages": 0,
        "revision_drift_pages": 0,
        "source_unavailable_pages": sum(1 for row in observations.values() if "refresh_error" in row),
        "refresh_failures": dict(sorted(failures.items())),
    }
    return observations, meta


def current_shape(source: dict) -> str:
    if source.get("redirect") is True:
        return "REDIRECT"
    templates = _set(source.get("templates"))
    if any("infobox" in t.casefold() for t in templates):
        return "STRUCTURED_PRIMARY"
    if templates or source.get("categories"):
        return "STRUCTURED_ALTERNATE"
    return "NO_DIRECT_STRUCTURED_SIGNATURE"


def _matches_world_change(p: dict, current: dict) -> bool:
    current_templates = _set(current.get("templates"))
    return (
        p.get("source_shape") == "STRUCTURED_PRIMARY"
        and WORLD_CHANGE_SIGNATURE["root"] in _set(p.get("discovery_roots"))
        and WORLD_CHANGE_SIGNATURE["surface"] in _set(p.get("source_surfaces"))
        and WORLD_CHANGE_SIGNATURE["template"] in current_templates
        and not ({OBJECT_TEMPLATE, WORLD_QUEST_TEMPLATE, HUNTS_TEMPLATE} & current_templates)
    )


def pinned_content_disposition(row: dict) -> str:
    p = row["lane_observations"][0]
    templates = _set(p.get("templates"))
    if HUNTS_TEMPLATE in templates:
        return "EDITORIAL_RELATIONSHIP_ONLY"
    if OBJECT_TEMPLATE in templates and WORLD_QUEST_TEMPLATE in templates:
        return "MULTI_FAMILY_RELATIONSHIP_ONLY"
    if WORLD_QUEST_TEMPLATE in templates:
        return "MULTI_FAMILY_RELATIONSHIP_ONLY"
    if WORLD_CHANGE_TEMPLATE in templates:
        return "DYNAMIC_STATE_RELATIONSHIP_CANDIDATE"
    if OBJECT_TEMPLATE in templates:
        return "MULTI_FAMILY_DEFINITION_CANDIDATE"
    return {
        "STRUCTURED_ALTERNATE": "ALTERNATE_SOURCE_ONLY",
        "REDIRECT": "REDIRECT_TARGET_UNPROVEN",
        "INFOBOX_ITEM_PARSE_ERROR": "PARSER_EVIDENCE_REQUIRED",
        "SOURCE_CLASSIFICATION_UNRESOLVED": "SOURCE_CLASSIFIER_EVIDENCE_REQUIRED",
        "NO_INFOBOX_ITEM": "NO_PRIMARY_DEFINITION_SIGNATURE",
    }.get(row.get("primary_source_shape"), "AMBIGUOUS_FAMILY")


def _base_disposition(row: dict, current: dict | None) -> dict:
    p = row["lane_observations"][0]
    expected_revisions = row.get("pinned_revision_ids", [])
    if not current or current.get("refresh_error"):
        status = current.get("refresh_error", "CURRENT_SOURCE_UNAVAILABLE") if current else "CURRENT_SOURCE_UNAVAILABLE"
        return {
            "page_id": row["page_id"],
            "pinned_revision_ids": expected_revisions,
            "current_source_state": status,
            "current_page_title": None,
            "current_revision_id": None,
            "current_revision_timestamp": None,
            "current_source_shape": None,
            "source_revision_status": "UNVERIFIED",
            "family_disposition": "UNRESOLVED_NO_CURRENT_SOURCE",
            "definition_family": None,
            "content_disposition": pinned_content_disposition(row),
            "candidate_relationships": row.get("candidate_relationships_evidence_only", []),
            "reason": "Current MediaWiki evidence unavailable; pinned observation is not treated as current.",
        }
    rev = current.get("revision_id")
    if not isinstance(rev, int) or isinstance(rev, bool):
        raise ValueError(f"invalid current revision for page {row['page_id']}")
    observed_titles = {
        item.get("title")
        for item in row.get("title_observations", [])
        if isinstance(item, dict) and isinstance(item.get("title"), str)
    }
    if not isinstance(current.get("title"), str) or current["title"] not in observed_titles:
        return {
            "page_id": row["page_id"],
            "pinned_revision_ids": expected_revisions,
            "current_source_state": "SOURCE_TITLE_DRIFT",
            "current_page_title": current.get("title"),
            "current_revision_id": rev,
            "current_revision_timestamp": current.get("revision_timestamp"),
            "current_source_shape": current_shape(current),
            "source_revision_status": "TITLE_DRIFT",
            "family_disposition": "UNRESOLVED_TITLE_DRIFT",
            "definition_family": None,
            "content_disposition": pinned_content_disposition(row),
            "candidate_relationships": row.get("candidate_relationships_evidence_only", []),
            "reason": "Current exact page ID has a title absent from pinned page-title observations.",
        }
    if not expected_revisions:
        shape = current_shape(current)
        pinned_shape = row.get("primary_source_shape")
        content = {
            "INFOBOX_ITEM_PARSE_ERROR": "PARSER_EVIDENCE_REQUIRED",
            "NO_INFOBOX_ITEM": "NO_PRIMARY_DEFINITION_SIGNATURE",
            "REDIRECT": "REDIRECT_TARGET_UNPROVEN",
            "STRUCTURED_ALTERNATE": "ALTERNATE_SOURCE_ONLY",
            "SOURCE_CLASSIFICATION_UNRESOLVED": "SOURCE_CLASSIFIER_EVIDENCE_REQUIRED",
        }.get(pinned_shape, "AMBIGUOUS_FAMILY")
        return {
            "page_id": row["page_id"],
            "pinned_revision_ids": [],
            "current_source_state": "CURRENT_SOURCE_OBSERVED_WITHOUT_PINNED_BASELINE",
            "current_page_title": current.get("title"),
            "current_revision_id": rev,
            "current_revision_timestamp": current.get("revision_timestamp"),
            "current_source_shape": shape,
            "source_revision_status": "NO_PINNED_BASELINE",
            "family_disposition": "UNRESOLVED_NO_PINNED_BASELINE",
            "definition_family": None,
            "content_disposition": content,
            "candidate_relationships": row.get("candidate_relationships_evidence_only", []),
            "reason": "Current source was observed, but no pinned revision ID exists for an exact comparison.",
        }
    same_revision = expected_revisions == [rev]
    shape = current_shape(current)
    if not same_revision:
        return {
            "page_id": row["page_id"],
            "pinned_revision_ids": expected_revisions,
            "current_source_state": "SOURCE_REVISION_DRIFT",
            "current_page_title": current.get("title"),
            "current_revision_id": rev,
            "current_revision_timestamp": current.get("revision_timestamp"),
            "current_source_shape": shape,
            "source_revision_status": "DRIFT",
            "family_disposition": "UNRESOLVED_REVISION_DRIFT",
            "definition_family": None,
            "content_disposition": pinned_content_disposition(row),
            "candidate_relationships": row.get("candidate_relationships_evidence_only", []),
            "reason": "Current revision differs from the pinned G3 evidence; this batch does not infer across drift.",
        }
    if _matches_world_change(p, current):
        return {
            "page_id": row["page_id"],
            "pinned_revision_ids": expected_revisions,
            "current_source_state": "CURRENT_REVISION_VERIFIED",
            "current_page_title": current.get("title"),
            "current_revision_id": rev,
            "current_revision_timestamp": current.get("revision_timestamp"),
            "current_source_shape": shape,
            "source_revision_status": "CURRENT",
            "family_disposition": "ROUTED_EXACT_SOURCE_SIGNATURE",
            "definition_family": "Encounter",
            "content_disposition": "DYNAMIC_STATE_RELATIONSHIP_RETAINED",
            "candidate_relationships": row.get("candidate_relationships_evidence_only", []),
            "reason": "Exact World Change infobox + registered world-changes root/surface; no competing primary template.",
        }
    templates = _set(p.get("templates"))
    if HUNTS_TEMPLATE in templates:
        content = "EDITORIAL_RELATIONSHIP_ONLY"
        reason = "Infobox Hunts is an editorial guide; retain Area/Creature/Encounter references without assigning a primary definition family."
    elif WORLD_QUEST_TEMPLATE in templates:
        content = "MULTI_FAMILY_RELATIONSHIP_ONLY"
        reason = "World Quest source shape retains Quest/Encounter/Interaction relationship candidates; no forced primary family."
    elif OBJECT_TEMPLATE in templates:
        content = "MULTI_FAMILY_DEFINITION_CANDIDATE"
        reason = "Infobox Object subtype can span existing world/local/terrain/transition/item/presentation roles; no validated exact subtype-to-family map is available."
    elif row.get("primary_source_shape") == "STRUCTURED_ALTERNATE":
        content = "ALTERNATE_SOURCE_ONLY"
        reason = "Alternate structured evidence is retained, but is not promoted to a primary family."
    elif row.get("primary_source_shape") == "REDIRECT":
        content = "REDIRECT_TARGET_UNPROVEN"
        reason = "Pinned ledger does not provide exact target page-ID proof; no title-based redirect resolution."
    elif row.get("primary_source_shape") == "INFOBOX_ITEM_PARSE_ERROR":
        content = "PARSER_EVIDENCE_REQUIRED"
        reason = "Observed parse error remains distinct from absence and needs page-specific parser evidence."
    elif row.get("primary_source_shape") == "SOURCE_CLASSIFICATION_UNRESOLVED":
        content = "SOURCE_CLASSIFIER_EVIDENCE_REQUIRED"
        reason = "Pinned classifier leaves source shape unresolved."
    elif row.get("primary_source_shape") == "NO_INFOBOX_ITEM":
        content = "NO_PRIMARY_DEFINITION_SIGNATURE"
        reason = "No parseable Item infobox was observed; absence does not imply exclusion or another family."
    else:
        content = "AMBIGUOUS_FAMILY"
        reason = "No exact registered family signature; preserve all page-specific evidence and candidate relations."
    return {
        "page_id": row["page_id"],
        "pinned_revision_ids": expected_revisions,
        "current_source_state": "CURRENT_REVISION_VERIFIED",
        "current_page_title": current.get("title"),
        "current_revision_id": rev,
        "current_revision_timestamp": current.get("revision_timestamp"),
        "current_source_shape": shape,
        "source_revision_status": "CURRENT",
        "family_disposition": "UNRESOLVED_AMBIGUOUS_OR_NONDEFINITION",
        "definition_family": None,
        "content_disposition": content,
        "candidate_relationships": row.get("candidate_relationships_evidence_only", []),
        "reason": reason,
    }


def disposition(row: dict, current: dict | None) -> dict:
    result = _base_disposition(row, current)
    content = result["content_disposition"]
    candidates = row.get("candidate_relationships_evidence_only", [])
    relationship_states = {
        "EDITORIAL_RELATIONSHIP_ONLY",
        "MULTI_FAMILY_RELATIONSHIP_ONLY",
        "DYNAMIC_STATE_RELATIONSHIP_CANDIDATE",
        "DYNAMIC_STATE_RELATIONSHIP_RETAINED",
    }
    source_only_states = {
        "ALTERNATE_SOURCE_ONLY",
        "REDIRECT_TARGET_UNPROVEN",
        "PARSER_EVIDENCE_REQUIRED",
        "SOURCE_CLASSIFIER_EVIDENCE_REQUIRED",
        "NO_PRIMARY_DEFINITION_SIGNATURE",
    }
    result.update({
        "relationship_disposition": (
            "CANDIDATE_RELATIONSHIPS_PRESERVED"
            if candidates or content in relationship_states
            else "NO_DIRECT_RELATIONSHIP_CLAIM"
        ),
        "placement_disposition": "NO_EXACT_PLACEMENT_RECORD_IN_PINNED_CENSUS",
        "source_only_disposition": (
            "SOURCE_ONLY_OR_NONPROMOTABLE_EVIDENCE"
            if content in source_only_states
            else "NOT_SOURCE_ONLY"
        ),
        "exclusion_disposition": "NOT_EXCLUDED",
    })
    return result


def build_output(ledger: dict, refresh: dict[int, dict], refresh_meta: dict) -> tuple[dict, dict]:
    pages = validate_ledger(ledger)
    subtype_counts = pinned_template_partition(pages)
    rows = [disposition(row, refresh.get(row["page_id"])) for row in pages]
    ids = [row["page_id"] for row in rows]
    if len(ids) != 5512 or len(ids) != len(set(ids)):
        raise ValueError("output page-ID partition mismatch")
    source_counts = collections.Counter(row["source_revision_status"] for row in rows)
    family_counts = collections.Counter(row["family_disposition"] for row in rows)
    content_counts = collections.Counter(row["content_disposition"] for row in rows)
    relationship_counts = collections.Counter(row["relationship_disposition"] for row in rows)
    placement_counts = collections.Counter(row["placement_disposition"] for row in rows)
    source_only_counts = collections.Counter(row["source_only_disposition"] for row in rows)
    exclusion_counts = collections.Counter(row["exclusion_disposition"] for row in rows)
    direct_world = sum(row["definition_family"] == "Encounter" for row in rows)
    output = {
        "schema": "OTERYN_G3_UNKNOWN_CLOSURE/v1",
        "authority": {
            "source_family_disposition": "EVIDENCE_ONLY",
            "canonical_identity_selection": "NOT_PERFORMED",
            "production_key_minting": "NOT_PERFORMED",
            "source_identity_binding": "NOT_PERFORMED",
            "semantic_promotion": "NOT_PERFORMED",
            "world_placement": "NOT_PERFORMED",
            "production_data_mutation": "NOT_PERFORMED",
        },
        "input": {
            "g3_artifact_id": 10801778929,
            "g3_run_id": 35986883931,
            "g3_head_sha": "20e8e5d99b7b42b6fd008e67029623ea7a4a8e3a",
            "ledger_sha256": LEDGER_SHA256,
            "g3_universe_sha256": G3_UNIVERSE_SHA256,
        },
        "counts": {
            "input_unknown_pages": 5512,
            "output_pages": len(rows),
            "pinned_template_partition": subtype_counts,
            "source_revision_status": dict(sorted(source_counts.items())),
            "family_disposition": dict(sorted(family_counts.items())),
            "content_disposition": dict(sorted(content_counts.items())),
            "relationship_disposition": dict(sorted(relationship_counts.items())),
            "placement_disposition": dict(sorted(placement_counts.items())),
            "source_only_disposition": dict(sorted(source_only_counts.items())),
            "exclusion_disposition": dict(sorted(exclusion_counts.items())),
            "exact_signature_definition_family_routes": direct_world,
            "dropped_rows": 0,
            "duplicate_page_ids": False,
            "hard_excluded_rows": 0,
        },
        "source_refresh": refresh_meta,
        "rows": rows,
    }
    compact = {
        "schema": "OTERYN_G3_UNKNOWN_CLOSURE_MANIFEST/v1",
        "input": output["input"],
        "counts": output["counts"],
        "authority": output["authority"],
        "invariants": {
            "exact_5512_page_id_partition": len(rows) == 5512 and len(ids) == len(set(ids)),
            "no_title_only_family_assignment": True,
            "candidate_relationships_preserved": True,
            "redirect_target_must_be_exact_page_id": True,
            "revision_drift_fails_closed": True,
            "unavailable_source_fails_closed": True,
            "canonical_target_selection_performed": False,
            "production_population_performed": False,
            "hard_exclusions_absent": True,
        },
        "output_sha256": sha256(canonical(output)),
    }
    return output, compact


def load_pinned_ledger(path: pathlib.Path) -> dict:
    raw = path.read_bytes()
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            if set(archive.namelist()) != {"ledger.json", "manifest.json"}:
                raise ValueError("G3 ledger artifact member mismatch")
            ledger = json.loads(archive.read("ledger.json"))
            manifest = json.loads(archive.read("manifest.json"))
        if manifest.get("ledger_sha256") != LEDGER_SHA256:
            raise ValueError("G3 ledger artifact embedded digest mismatch")
    else:
        ledger = json.loads(raw)
    validate_ledger(ledger)
    return ledger


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-ledger", required=True, type=pathlib.Path)
    parser.add_argument("--out-dir", required=True, type=pathlib.Path)
    parser.add_argument("--offline-pinned", action="store_true", help="emit rows as unverified; for deterministic local reproduction only")
    args = parser.parse_args()
    ledger = load_pinned_ledger(args.input_ledger)
    pages = validate_ledger(ledger)
    if args.offline_pinned:
        refresh = {}
        meta = {"attempted_pages": 0, "verified_same_revision_pages": 0, "revision_drift_pages": 0, "source_unavailable_pages": 5512, "refresh_failures": {"OFFLINE_PINNED_ONLY": 5512}}
    else:
        refresh, meta = refresh_universe(pages)
        same = 0
        drift = 0
        for row in pages:
            current = refresh.get(row["page_id"], {})
            if current and "refresh_error" not in current:
                if row.get("pinned_revision_ids", []) == [current.get("revision_id")]:
                    same += 1
                else:
                    drift += 1
        meta["verified_same_revision_pages"] = same
        meta["revision_drift_pages"] = drift
    output, compact = build_output(ledger, refresh, meta)
    args.out_dir.mkdir(parents=True, exist_ok=True)
    (args.out_dir / "unknown-closure-rows.json").write_bytes(canonical(output))
    (args.out_dir / "manifest.json").write_bytes(canonical(compact))
    print(json.dumps(compact["counts"], ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
