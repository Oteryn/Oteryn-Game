#!/usr/bin/env python3
"""Deterministic TibiaWiki-first Item identity crosswalk.

Consumes a protected wiki-first census, the protected Oteryn/Crystal identity
closure, and the exact pinned Crystal items.xml. TibiaWiki is discovery/reference
evidence only. This compiler never mints identities or promotes gameplay semantics.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
from typing import Any
import xml.etree.ElementTree as ET

SCHEMA = "OTERYN_ITEM_WIKI_FIRST_IDENTITY_CROSSWALK/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_IDENTITY_CROSSWALK_MANIFEST/v1"
PROFILE = "OTERYN_ITEM_WIKI_FIRST_IDENTITY_CROSSWALK_COMPILER/v1"
CENSUS_SCHEMA = "OTERYN_ITEM_WIKI_FIRST_CENSUS/v1"
CENSUS_STABLE_SHA256 = "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a"
CLASSIFICATION_SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK/v1"
CLASSIFICATION_SHA256 = "004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d"
CRYSTAL_REPOSITORY = "zimbadev/crystalserver"
CRYSTAL_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
CRYSTAL_PATH = "data/items/items.xml"
CRYSTAL_ITEMS_SHA256 = "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"
EXPECTED_SOURCE_COUNT = 38_157
EXPECTED_WIKI_COUNT = 6_918
DISPOSITIONS = (
    "EXACT_MATCH",
    "PROBABLE_MATCH",
    "AMBIGUOUS",
    "CONFLICT",
    "NO_MATCH",
    "ALIAS_OR_DUPLICATE",
)
STRUCTURAL_FIELDS = {
    "attack": "attack",
    "defense": "defense",
    "extra_defense": "defensemod",
    "range": "range",
    "hit_chance": "hit",
    "armor": "armor",
    "charge_count": "charges",
    "capacity": "volume",
}
_ALIAS_SPLIT_RE = re.compile(r"\s*(?:,|;|\s/\s)\s*")


class CrosswalkError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def normalized_name(value: str) -> str:
    if not isinstance(value, str):
        raise CrosswalkError("NAME_NOT_STRING")
    encoded = value.encode("utf-8")
    if len(encoded) > 512:
        raise CrosswalkError("NAME_MAX_PLUS_ONE")
    return " ".join(value.casefold().split())


def load_json(path: Path) -> tuple[dict[str, Any], bytes]:
    payload = path.read_bytes()
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise CrosswalkError(f"JSON_INVALID:{path.name}") from exc
    if not isinstance(value, dict):
        raise CrosswalkError(f"JSON_ROOT_INVALID:{path.name}")
    return value, payload


def verify_census(census: dict[str, Any]) -> str:
    if census.get("schema") != CENSUS_SCHEMA:
        raise CrosswalkError("CENSUS_SCHEMA_MISMATCH")
    pages = census.get("pages")
    if not isinstance(pages, list) or len(pages) != EXPECTED_WIKI_COUNT:
        actual = len(pages) if isinstance(pages, list) else "invalid"
        raise CrosswalkError(f"CENSUS_COUNT_MISMATCH:{actual}")
    stable = dict(census)
    stable.pop("retrieval_timestamp", None)
    actual = sha256_bytes(canonical_bytes(stable))
    if actual != CENSUS_STABLE_SHA256:
        raise CrosswalkError(f"CENSUS_STABLE_DIGEST_MISMATCH:{actual}")
    return actual


def verify_classification(value: dict[str, Any], payload: bytes) -> str:
    if value.get("schema") != CLASSIFICATION_SCHEMA:
        raise CrosswalkError("CLASSIFICATION_SCHEMA_MISMATCH")
    actual = sha256_bytes(payload)
    if actual != CLASSIFICATION_SHA256:
        raise CrosswalkError(f"CLASSIFICATION_DIGEST_MISMATCH:{actual}")
    records = value.get("records")
    profiles = value.get("source_profiles")
    if not isinstance(records, list) or len(records) != EXPECTED_SOURCE_COUNT:
        raise CrosswalkError("CLASSIFICATION_RECORD_COUNT_MISMATCH")
    if not isinstance(profiles, list):
        raise CrosswalkError("CLASSIFICATION_PROFILES_INVALID")
    return actual


def _node_ids(node: ET.Element) -> list[int]:
    if "id" in node.attrib:
        if "fromid" in node.attrib or "toid" in node.attrib:
            raise CrosswalkError("CRYSTAL_ID_AND_RANGE")
        try:
            return [int(node.attrib["id"])]
        except ValueError as exc:
            raise CrosswalkError("CRYSTAL_ID_INVALID") from exc
    if "fromid" not in node.attrib or "toid" not in node.attrib:
        raise CrosswalkError("CRYSTAL_IDENTITY_MISSING")
    try:
        start, end = int(node.attrib["fromid"]), int(node.attrib["toid"])
    except ValueError as exc:
        raise CrosswalkError("CRYSTAL_RANGE_INVALID") from exc
    if start > end:
        return []
    return list(range(start, end + 1))


def parse_crystal_names(payload: bytes) -> dict[int, str | None]:
    actual = sha256_bytes(payload)
    if actual != CRYSTAL_ITEMS_SHA256:
        raise CrosswalkError(f"CRYSTAL_ITEMS_DIGEST_MISMATCH:{actual}")
    try:
        root = ET.fromstring(payload)
    except ET.ParseError as exc:
        raise CrosswalkError("CRYSTAL_ITEMS_XML_INVALID") from exc
    if root.tag != "items":
        raise CrosswalkError("CRYSTAL_ITEMS_ROOT_INVALID")
    result: dict[int, str | None] = {}
    for node in root.findall("item"):
        raw_name = node.attrib.get("name")
        name = (
            normalized_name(raw_name.strip())
            if isinstance(raw_name, str) and raw_name.strip()
            else None
        )
        for source_id in _node_ids(node):
            if source_id in result:
                raise CrosswalkError(f"CRYSTAL_DUPLICATE_SOURCE_ID:{source_id}")
            result[source_id] = name
    if len(result) != EXPECTED_SOURCE_COUNT:
        raise CrosswalkError(f"CRYSTAL_SOURCE_COUNT_MISMATCH:{len(result)}")
    return result


def _scalar(value: Any) -> int | str | bool | None:
    if isinstance(value, float):
        return None
    return value if isinstance(value, (int, str, bool)) else None


def wiki_value(page: dict[str, Any], key: str) -> int | str | bool | None:
    fields = page.get("normalized_fields")
    item = fields.get(key) if isinstance(fields, dict) else None
    if not isinstance(item, dict) or item.get("state") != "VALUE":
        return None
    return _scalar(item.get("value"))


def discovery_names(page: dict[str, Any]) -> list[dict[str, str]]:
    values: list[dict[str, str]] = []
    title = page.get("title")
    if isinstance(title, str) and title.strip():
        values.append({"origin": "WIKI_TITLE", "name": normalized_name(title)})
    infobox_name = wiki_value(page, "name")
    if isinstance(infobox_name, str) and infobox_name.strip():
        values.append(
            {"origin": "INFOBOX_NAME", "name": normalized_name(infobox_name)}
        )
    aliases = wiki_value(page, "aliases")
    if (
        isinstance(aliases, str)
        and aliases.strip()
        and len(aliases.encode("utf-8")) <= 2048
    ):
        for part in _ALIAS_SPLIT_RE.split(aliases):
            part = part.strip()
            if part and len(part.encode("utf-8")) <= 512:
                values.append(
                    {"origin": "WIKI_ALIAS", "name": normalized_name(part)}
                )
    dedup = {(item["origin"], item["name"]): item for item in values}
    return [dedup[key] for key in sorted(dedup)]


def parse_int(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and re.fullmatch(r"[+-]?\d+", value.strip()):
        return int(value.strip())
    return None


def profile_structural_signals(
    profile: dict[str, Any],
) -> tuple[dict[str, int], list[str]]:
    observations = profile.get("candidate_observations")
    if not isinstance(observations, list):
        raise CrosswalkError("PROFILE_OBSERVATIONS_INVALID")
    by_wiki: dict[str, set[int]] = defaultdict(set)
    for observation in observations:
        if not isinstance(observation, dict):
            raise CrosswalkError("PROFILE_OBSERVATION_INVALID")
        wiki_field = STRUCTURAL_FIELDS.get(observation.get("native_field"))
        if (
            wiki_field is None
            or observation.get("nested_values") not in (None, [])
        ):
            continue
        parsed = parse_int(observation.get("source_value"))
        if parsed is not None:
            by_wiki[wiki_field].add(parsed)
    signals: dict[str, int] = {}
    conflicts: list[str] = []
    for key in sorted(by_wiki):
        values = by_wiki[key]
        if len(values) == 1:
            signals[key] = next(iter(values))
        elif values:
            conflicts.append(key)
    return signals, conflicts


def compare_candidate(
    page: dict[str, Any], profile: dict[str, Any]
) -> dict[str, Any]:
    signals, donor_conflicts = profile_structural_signals(profile)
    matched: list[str] = []
    contradicted: list[str] = []
    observed: list[str] = []
    for key, expected in sorted(signals.items()):
        actual = wiki_value(page, key)
        if actual is None:
            continue
        parsed_actual = parse_int(actual)
        if parsed_actual is None:
            continue
        observed.append(key)
        if parsed_actual == expected:
            matched.append(key)
        else:
            contradicted.append(key)
    return {
        "matched_structural_signals": matched,
        "contradicted_structural_signals": contradicted,
        "observed_comparable_signals": observed,
        "donor_conflicting_fields_ignored": donor_conflicts,
    }


def build_identity_index(
    classification: dict[str, Any], crystal_names: dict[int, str | None]
) -> tuple[
    dict[int, dict[str, Any]], dict[str, list[int]], dict[str, dict[str, Any]]
]:
    profiles: dict[str, dict[str, Any]] = {}
    for profile in classification["source_profiles"]:
        if not isinstance(profile, dict) or not isinstance(
            profile.get("profile_id"), str
        ):
            raise CrosswalkError("PROFILE_INVALID")
        profile_id = profile["profile_id"]
        if profile_id in profiles:
            raise CrosswalkError(f"DUPLICATE_PROFILE:{profile_id}")
        profiles[profile_id] = profile

    identities: dict[int, dict[str, Any]] = {}
    name_index: dict[str, list[int]] = defaultdict(list)
    native_keys: set[str] = set()
    for record in classification["records"]:
        if not isinstance(record, dict):
            raise CrosswalkError("IDENTITY_RECORD_INVALID")
        source_id = record.get("source_item_id")
        native_key = record.get("native_key")
        profile_id = record.get("source_profile_id")
        if not isinstance(source_id, int) or isinstance(source_id, bool):
            raise CrosswalkError("SOURCE_ID_INVALID")
        if not isinstance(native_key, str) or not native_key.startswith(
            "oteryn:item."
        ):
            raise CrosswalkError("NATIVE_KEY_INVALID")
        if not isinstance(profile_id, str) or profile_id not in profiles:
            raise CrosswalkError("SOURCE_PROFILE_INVALID")
        if source_id in identities:
            raise CrosswalkError(f"DUPLICATE_SOURCE_ID:{source_id}")
        if native_key in native_keys:
            raise CrosswalkError(f"DUPLICATE_NATIVE_KEY:{native_key}")
        native_keys.add(native_key)
        if source_id not in crystal_names:
            raise CrosswalkError(f"CRYSTAL_SOURCE_ID_MISSING:{source_id}")
        crystal_name = crystal_names[source_id]
        identities[source_id] = {
            "source_item_id": source_id,
            "native_key": native_key,
            "source_profile_id": profile_id,
            "identity_origin": record.get("identity_origin"),
            "crystal_name": crystal_name,
        }
        if crystal_name:
            name_index[crystal_name].append(source_id)
    for key in list(name_index):
        name_index[key] = sorted(name_index[key])
    if len(identities) != EXPECTED_SOURCE_COUNT:
        raise CrosswalkError("IDENTITY_CLOSURE_COUNT_MISMATCH")
    return identities, dict(name_index), profiles


def classify_page(
    page: dict[str, Any],
    *,
    identities: dict[int, dict[str, Any]],
    name_index: dict[str, list[int]],
    profiles: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    names = discovery_names(page)
    candidate_ids: set[int] = set()
    matched_name_origins: dict[int, set[str]] = defaultdict(set)
    for item in names:
        for source_id in name_index.get(item["name"], []):
            candidate_ids.add(source_id)
            matched_name_origins[source_id].add(item["origin"])

    candidates: list[dict[str, Any]] = []
    for source_id in sorted(candidate_ids):
        identity = identities[source_id]
        comparison = compare_candidate(
            page, profiles[identity["source_profile_id"]]
        )
        candidates.append(
            {
                "source_item_id": source_id,
                "native_key": identity["native_key"],
                "identity_origin": identity.get("identity_origin"),
                "crystal_name": identity.get("crystal_name"),
                "name_match_origins": sorted(matched_name_origins[source_id]),
                **comparison,
            }
        )

    base = {
        "page_id": page.get("page_id"),
        "title": page.get("title"),
        "source_shape": page.get("source_shape"),
        "candidate_count": len(candidates),
        "candidates": candidates,
        "selected_source_item_id": None,
        "selected_native_key": None,
    }
    if not candidates:
        return {
            **base,
            "disposition": "NO_MATCH",
            "reason": "NO_EXACT_NAME_CANDIDATE",
        }

    if len(candidates) == 1:
        candidate = candidates[0]
        contradicted = candidate["contradicted_structural_signals"]
        matched = candidate["matched_structural_signals"]
        if contradicted:
            return {
                **base,
                "disposition": "CONFLICT",
                "reason": "STRUCTURAL_CONTRADICTION",
            }
        if matched:
            return {
                **base,
                "disposition": "EXACT_MATCH",
                "reason": "EXACT_NAME_PLUS_INDEPENDENT_STRUCTURAL_SIGNAL",
                "selected_source_item_id": candidate["source_item_id"],
                "selected_native_key": candidate["native_key"],
            }
        return {
            **base,
            "disposition": "PROBABLE_MATCH",
            "reason": "UNIQUE_EXACT_NAME_WITHOUT_INDEPENDENT_SIGNAL",
            "selected_source_item_id": candidate["source_item_id"],
            "selected_native_key": candidate["native_key"],
        }

    viable = [
        candidate
        for candidate in candidates
        if not candidate["contradicted_structural_signals"]
    ]
    corroborated = [
        candidate for candidate in viable if candidate["matched_structural_signals"]
    ]
    contradicted = [
        candidate
        for candidate in candidates
        if candidate["contradicted_structural_signals"]
    ]
    if not viable:
        return {
            **base,
            "disposition": "CONFLICT",
            "reason": "ALL_NAME_CANDIDATES_STRUCTURALLY_CONTRADICT",
        }
    if (
        len(viable) == 1
        and len(corroborated) == 1
        and len(contradicted) == len(candidates) - 1
    ):
        selected = viable[0]
        return {
            **base,
            "disposition": "EXACT_MATCH",
            "reason": "UNIQUE_STRUCTURAL_SURVIVOR_AMONG_NAME_COLLISIONS",
            "selected_source_item_id": selected["source_item_id"],
            "selected_native_key": selected["native_key"],
        }
    return {
        **base,
        "disposition": "AMBIGUOUS",
        "reason": "MULTIPLE_PLAUSIBLE_NAME_CANDIDATES",
    }


def apply_duplicate_target_protection(records: list[dict[str, Any]]) -> None:
    by_target: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if (
            record["disposition"] in {"EXACT_MATCH", "PROBABLE_MATCH"}
            and isinstance(record.get("selected_native_key"), str)
        ):
            by_target[record["selected_native_key"]].append(record)

    for target, group in sorted(by_target.items()):
        if len(group) <= 1:
            continue
        canonical_like: list[dict[str, Any]] = []
        for record in group:
            selected_id = record.get("selected_source_item_id")
            selected = next(
                (
                    candidate
                    for candidate in record["candidates"]
                    if candidate["source_item_id"] == selected_id
                ),
                None,
            )
            if (
                selected is not None
                and isinstance(record.get("title"), str)
                and selected.get("crystal_name")
                == normalized_name(record["title"])
            ):
                canonical_like.append(record)
        keep = canonical_like[0] if len(canonical_like) == 1 else None
        for record in group:
            if record is keep:
                continue
            record["disposition"] = "ALIAS_OR_DUPLICATE"
            record["reason"] = (
                "MULTIPLE_WIKI_PAGES_TARGET_ONE_EXISTING_IDENTITY"
            )
            record["duplicate_target_native_key"] = target


def validate_source_pages(pages: list[dict[str, Any]]) -> None:
    seen_ids: set[int] = set()
    seen_titles: set[str] = set()
    for page in pages:
        if not isinstance(page, dict):
            raise CrosswalkError("WIKI_PAGE_INVALID")
        page_id = page.get("page_id")
        title = page.get("title")
        if not isinstance(page_id, int) or isinstance(page_id, bool):
            raise CrosswalkError("WIKI_PAGE_ID_INVALID")
        if not isinstance(title, str) or not title:
            raise CrosswalkError("WIKI_TITLE_INVALID")
        if page_id in seen_ids:
            raise CrosswalkError(f"DUPLICATE_WIKI_PAGE_ID:{page_id}")
        if title in seen_titles:
            raise CrosswalkError(f"DUPLICATE_WIKI_TITLE:{title}")
        seen_ids.add(page_id)
        seen_titles.add(title)


def compile_crosswalk(
    census: dict[str, Any],
    classification: dict[str, Any],
    crystal_names: dict[int, str | None],
) -> dict[str, Any]:
    pages = census.get("pages")
    if not isinstance(pages, list):
        raise CrosswalkError("CENSUS_PAGES_INVALID")
    validate_source_pages(pages)
    identities, name_index, profiles = build_identity_index(
        classification, crystal_names
    )
    records = [
        classify_page(
            page,
            identities=identities,
            name_index=name_index,
            profiles=profiles,
        )
        for page in sorted(
            pages,
            key=lambda page: (
                normalized_name(page["title"]),
                page["title"],
                page["page_id"],
            ),
        )
    ]
    apply_duplicate_target_protection(records)
    counts = Counter(record["disposition"] for record in records)
    if set(counts) - set(DISPOSITIONS):
        raise CrosswalkError("DISPOSITION_INVALID")
    if (
        len(records) != EXPECTED_WIKI_COUNT
        or sum(counts.values()) != EXPECTED_WIKI_COUNT
    ):
        raise CrosswalkError("DISPOSITION_PARTITION_FAILED")
    for record in records:
        if record["disposition"] != "EXACT_MATCH":
            continue
        selected = [
            candidate
            for candidate in record["candidates"]
            if candidate["source_item_id"]
            == record["selected_source_item_id"]
        ]
        if (
            len(selected) != 1
            or not selected[0]["matched_structural_signals"]
        ):
            raise CrosswalkError("NAME_ONLY_EXACT_MATCH_FORBIDDEN")

    native_keys = {row["native_key"] for row in identities.values()}
    selected_keys = {
        record["selected_native_key"]
        for record in records
        if isinstance(record.get("selected_native_key"), str)
    }
    if not selected_keys.issubset(native_keys):
        raise CrosswalkError("IDENTITY_MINTING_DETECTED")

    reasons = Counter(record["reason"] for record in records)
    candidate_links = sum(record["candidate_count"] for record in records)
    resolved = (
        counts["EXACT_MATCH"]
        + counts["PROBABLE_MATCH"]
        + counts["ALIAS_OR_DUPLICATE"]
    )
    exact = counts["EXACT_MATCH"]
    unresolved = (
        counts["AMBIGUOUS"] + counts["CONFLICT"] + counts["NO_MATCH"]
    )
    targeted = {
        record["selected_native_key"]
        for record in records
        if isinstance(record.get("selected_native_key"), str)
    }
    source_shapes = Counter(record["source_shape"] for record in records)
    return {
        "schema": SCHEMA,
        "compiler_profile": PROFILE,
        "authority": {
            "canonical_identity_space": "PROTECTED_OTERYN_ITEM_IDENTITIES",
            "tibiawiki_role": "STRUCTURED_REFERENCE_DATA",
            "crystal_role": "OTS_HYPOTHESIS_ONLY",
            "identity_minting": "FORBIDDEN",
            "semantic_promotion": "NOT_PERFORMED",
        },
        "inputs": {
            "wiki_first_census_stable_sha256": CENSUS_STABLE_SHA256,
            "classification_crosswalk_sha256": CLASSIFICATION_SHA256,
            "crystal_items_sha256": CRYSTAL_ITEMS_SHA256,
            "crystal_repository": CRYSTAL_REPOSITORY,
            "crystal_revision": CRYSTAL_REVISION,
            "crystal_path": CRYSTAL_PATH,
        },
        "records": records,
        "counts": {
            "source_pages": len(records),
            "dispositions": {
                name: counts[name] for name in DISPOSITIONS
            },
            "reasons": dict(sorted(reasons.items())),
            "source_shapes": dict(sorted(source_shapes.items())),
            "candidate_links": candidate_links,
            "pages_with_candidates": sum(
                record["candidate_count"] > 0 for record in records
            ),
            "pages_without_candidates": sum(
                record["candidate_count"] == 0 for record in records
            ),
            "selected_target_native_identities": len(targeted),
            "exact_coverage_percent": round(
                exact * 100.0 / len(records), 6
            ),
            "resolved_or_probable_coverage_percent": round(
                resolved * 100.0 / len(records), 6
            ),
            "unresolved_pages": unresolved,
        },
    }


def next_source_recommendation(counts: dict[str, Any]) -> dict[str, str]:
    dispositions = counts["dispositions"]
    if dispositions["NO_MATCH"] >= max(
        dispositions["PROBABLE_MATCH"], dispositions["AMBIGUOUS"]
    ):
        return {
            "source_class": (
                "ADDITIONAL_STABLE_NAME_ALIAS_CATALOG_WITH_ITEM_ID_BRIDGE"
            ),
            "why": (
                "NO_MATCH is the largest residual; another donor catalogue "
                "can create candidates that exact-name Crystal discovery misses."
            ),
        }
    if dispositions["PROBABLE_MATCH"] >= dispositions["AMBIGUOUS"]:
        return {
            "source_class": "INDEPENDENT_ITEM_ID_OR_STRUCTURAL_BRIDGE",
            "why": (
                "PROBABLE_MATCH is the largest residual; name candidates "
                "already exist and need one independent identity signal."
            ),
        }
    return {
        "source_class": "STABLE_CLIENT_SERVER_ITEM_ID_BRIDGE",
        "why": (
            "AMBIGUOUS name collisions dominate the residual and require "
            "a non-name identity discriminator."
        ),
    }


def build_manifest(
    full: dict[str, Any], *, compiler_sha256: str
) -> dict[str, Any]:
    if full.get("schema") != SCHEMA:
        raise CrosswalkError("FULL_SCHEMA_INVALID")
    counts = full["counts"]
    dispositions = counts["dispositions"]
    if sum(dispositions.values()) != EXPECTED_WIKI_COUNT:
        raise CrosswalkError("MANIFEST_PARTITION_INVALID")
    return {
        "schema": MANIFEST_SCHEMA,
        "status": (
            "WIKI_FIRST_IDENTITY_CROSSWALK_COMPLETE_NO_SEMANTIC_PROMOTION"
        ),
        "compiler": {
            "profile": PROFILE,
            "path": (
                "tools/reference-world-corridor-census/"
                "item_wiki_first_identity_crosswalk.py"
            ),
            "sha256": compiler_sha256,
        },
        "input_digests": full["inputs"],
        "counts": counts,
        "full_output": {
            "schema": SCHEMA,
            "sha256": sha256_bytes(canonical_bytes(full)),
            "committed_bulk_corpus": False,
        },
        "invariants": {
            "source_population_exact_6918": (
                counts["source_pages"] == EXPECTED_WIKI_COUNT
            ),
            "disposition_partition_complete": (
                sum(dispositions.values()) == EXPECTED_WIKI_COUNT
            ),
            "name_only_exact_match": False,
            "negative_evidence_fail_closed": True,
            "duplicate_target_protection": True,
            "no_infobox_item_retained": (
                counts["source_shapes"].get("NO_INFOBOX_ITEM", 0) > 0
            ),
            "parse_error_item_retained": (
                counts["source_shapes"].get(
                    "INFOBOX_ITEM_PARSE_ERROR", 0
                )
                > 0
            ),
            "conflicting_parse_field_used_as_identity_proof": False,
            "identity_minting_performed": False,
            "semantic_promotion_performed": False,
        },
        "limitations": [
            (
                "TibiaWiki remains structured reference evidence and is not "
                "gameplay truth."
            ),
            (
                "Crystal exact names are discovery signals only; name-only "
                "candidates remain PROBABLE_MATCH rather than EXACT_MATCH."
            ),
            (
                "Only structurally equivalent integer fields with "
                "already-established semantics are used as independent "
                "corroboration in this generation."
            ),
            (
                "Pages without a usable Item infobox remain in the population "
                "and can only resolve through other admitted identity evidence."
            ),
            (
                "The compiler does not mint missing identities, merge aliases, "
                "or promote any gameplay field."
            ),
        ],
        "next_source_recommendation": next_source_recommendation(counts),
        "next_gate": "WIKI_FIRST_ITEM_FIELD_VERIFICATION",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--census", type=Path, required=True)
    parser.add_argument(
        "--classification-crosswalk", type=Path, required=True
    )
    parser.add_argument("--crystal-items", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()

    census, _ = load_json(args.census)
    classification, classification_bytes = load_json(
        args.classification_crosswalk
    )
    verify_census(census)
    verify_classification(classification, classification_bytes)
    crystal_names = parse_crystal_names(args.crystal_items.read_bytes())
    full = compile_crosswalk(census, classification, crystal_names)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(full))
    compiler_sha = sha256_bytes(
        Path(__file__).read_bytes().replace(b"\r\n", b"\n")
    )
    manifest = build_manifest(full, compiler_sha256=compiler_sha)
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))

    dispositions = manifest["counts"]["dispositions"]
    print(
        "item-wiki-first-identity-crosswalk: PASS "
        + " ".join(
            f"{name}={dispositions[name]}" for name in DISPOSITIONS
        )
        + f" sha256={manifest['full_output']['sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
