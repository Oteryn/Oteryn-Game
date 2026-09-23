#!/usr/bin/env python3
"""Compile deterministic atomic Item field verification from protected Item evidence.

This is an evidence/rule compiler. It never imports Crystal/B1 source bytes, mints identities,
or mutates canonical Item semantics. Bulk source/current corpora remain scratch inputs; only the
compact manifest is intended for Git.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

TARGET_COUNT = 38_157
TARGET_CUT = "2026-07-28"
SCHEMA = "OTERYN_ITEM_FIELD_VERIFICATION/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_FIELD_VERIFICATION_MANIFEST/v1"
RULE_PROFILE = "OTERYN_ITEM_FIELD_RULE_ENGINE/v1"
CROSSWALK_SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK/v1"
CURRENT_SOURCE_SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI/v1"
CURRENT_SOURCE_MANIFEST_SCHEMA = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_MANIFEST/v1"
CURRENT_SOURCE_PROFILE = "OTERYN_ITEM_CURRENT_SOURCE_TIBIAWIKI_COLLECTOR/v1"
CURRENT_SOURCE_ID = "TIBIAWIKI_STRUCTURED"

# Immutable protected lineage inputs for this bounded generation.
CROSSWALK_FULL_SHA256 = "004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d"
SCHEMA_READINESS_SHA256 = "c69b7626ca052b5494d14c034848088175f79a662aa7d9e41876d3452917eb78"
PROTECTED_CURRENT_SOURCE_MANIFEST_SHA256 = "06e40dd1472cd3650e641e9e8b33af8e930d3c4345967aec917a08baf73022ed"
PROTECTED_CURRENT_SOURCE_SHA256 = "005761fa0c464da0f64cedcb7efcfc7afb5f0dc19eded97ed935013d72d095d0"

FIELD_STATES = (
    "CONFIRMED_CURRENT",
    "CORROBORATED_CURRENT",
    "OTS_ONLY",
    "CONFLICT",
    "UNKNOWN",
    "NOT_APPLICABLE",
    "DECLARED_OTERYN_DIFFERENCE",
)
CONTINUITY_STATES = ("PROVEN", "DERIVED", "UNKNOWN", "CONFLICT")

# These are the only current-source scalar fields for which #767 already defined a
# deterministic B1 comparison in its protected collector. Do not widen this map here.
COMPARABLE_CURRENT_TO_B1 = {
    "attack": "attack",
    "defense": "defense",
    "defensemod": "extra_defense",
    "range": "range",
    "hit": "hit_chance",
    "armor": "armor",
    "charges": "charge_count",
    "volume": "capacity",
}
B1_TO_CURRENT = {native: current for current, native in COMPARABLE_CURRENT_TO_B1.items()}

# These typed destinations are vector/container carriers. Distinct B1 atoms that share the
# carrier are not competing values and must remain independently verifiable.
AGGREGATE_TYPED_DESTINATIONS = {
    "skill_modifiers.modifiers",
    "protection.resistances",
    "weapon.elemental",
}

# Current-source fields can be retained for verification even when no promotion mapping has
# been accepted. Those remain UNKNOWN unless a stronger rule below explicitly qualifies them.
CURRENT_LOGICAL_PATH = {
    "name": "presentation.name",
    "aliases": "presentation.aliases",
    "itemclass": "classification.item_type",
    "primarytype": "classification.item_type",
    "secondarytype": "classification.item_type",
    "weight": "physical.weight",
    "stackable": "stack.stackable",
    "slottype": "equipment.slot",
    "hands": "equipment.patterns",
    "vocrequired": "equipment.vocation_requirement",
    "levelrequired": "equipment.level_requirement",
    "attack": "weapon.attack",
    "defense": "weapon.defense",
    "defensemod": "weapon.extra_defense",
    "range": "weapon.range_cells",
    "hit": "weapon.hit_chance",
    "armor": "protection.armor",
    "charges": "charges.count",
    "duration": "temporal.duration_ms",
    "volume": "container.capacity",
    "imbuement": "imbuement.slot_count",
    "resist": "protection.resistances",
    "skillboost": "skill_modifiers.modifiers",
}

# Logical coverage explicitly required by the Item programme even if the protected inputs carry
# no admitted observation for a path. Absence is UNKNOWN, never false/zero.
REQUIRED_LOGICAL_FIELDS = {
    "presentation.name",
    "presentation.aliases",
    "presentation.description",
    "classification.item_type",
    "classification.capabilities",
    "physical.weight",
    "physical.movable",
    "physical.pickupable",
    "stack.stackable",
    "stack.stack_max",
    "equipment.slot",
    "equipment.patterns",
    "equipment.vocation_requirement",
    "equipment.level_requirement",
    "weapon.weapon_type",
    "weapon.ammunition",
    "weapon.attack",
    "weapon.defense",
    "weapon.extra_defense",
    "weapon.range_cells",
    "weapon.hit_chance",
    "weapon.max_hit_chance",
    "weapon.elemental",
    "protection.armor",
    "protection.resistances",
    "skill_modifiers.modifiers",
    "skill_modifiers.mantra",
    "charges.count",
    "temporal.duration_ms",
    "temporal.consumption_mode",
    "temporal.decay_target_ordinal",
    "container.capacity",
    "imbuement.slot_count",
    "imbuement.allowed_family_tiers",
    "use_transform.relations",
}


class VerificationError(RuntimeError):
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
        raise VerificationError(f"JSON_INVALID:{path}") from exc
    if not isinstance(value, dict):
        raise VerificationError(f"JSON_ROOT_NOT_OBJECT:{path}")
    return value, payload


def normalized_text(value: str) -> str:
    return " ".join(value.casefold().split())


def simple_int(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str) and re.fullmatch(r"[+-]?\d+", value.strip()):
        return int(value.strip())
    return None


def json_scalar_key(value: Any) -> bytes:
    return canonical_bytes(value)


def field_paths_for_destination(native_field: str, destination: str) -> list[str]:
    if destination.startswith("EXPLICIT_UNSUPPORTED_"):
        return [f"unsupported.{native_field}"]
    if destination == "temporal.duration_ms+consumption_mode":
        # The duration source value can verify the duration atom only. Mode is an independent
        # semantic atom and remains UNKNOWN absent its own evidence.
        return ["temporal.duration_ms"]
    if destination in AGGREGATE_TYPED_DESTINATIONS:
        # Preserve source-atom identity inside aggregate typed carriers. Different resistance,
        # elemental and skill-modifier atoms coexist; they are not a conflict merely because
        # the canonical schema stores them in one vector.
        return [f"{destination}[{native_field}]"]
    return [destination]


def validate_inputs(
    crosswalk: dict[str, Any],
    current: dict[str, Any],
    schema: dict[str, Any],
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]], set[str]]:
    if crosswalk.get("schema") != CROSSWALK_SCHEMA:
        raise VerificationError("CROSSWALK_SCHEMA_MISMATCH")
    if current.get("schema") != CURRENT_SOURCE_SCHEMA:
        raise VerificationError("CURRENT_SOURCE_SCHEMA_MISMATCH")
    if current.get("target_cut") != TARGET_CUT:
        raise VerificationError("CURRENT_SOURCE_TARGET_CUT_MISMATCH")
    if current.get("collector_profile") != CURRENT_SOURCE_PROFILE:
        raise VerificationError("CURRENT_SOURCE_PROFILE_MISMATCH")
    source = current.get("source")
    if (
        not isinstance(source, dict)
        or source.get("id") != CURRENT_SOURCE_ID
        or source.get("role") != "STRUCTURED_REFERENCE_DATA"
    ):
        raise VerificationError("CURRENT_SOURCE_ROLE_MISMATCH")
    authority = current.get("authority")
    if not isinstance(authority, dict) or authority.get("semantic_promotion") != "FORBIDDEN":
        raise VerificationError("CURRENT_SOURCE_AUTHORITY_MISMATCH")
    cross_records = crosswalk.get("records")
    current_records = current.get("records")
    profiles = crosswalk.get("source_profiles")
    if not isinstance(cross_records, list) or len(cross_records) != TARGET_COUNT:
        raise VerificationError("CROSSWALK_COUNT_MISMATCH")
    if not isinstance(current_records, list) or len(current_records) != TARGET_COUNT:
        raise VerificationError("CURRENT_SOURCE_COUNT_MISMATCH")
    if not isinstance(profiles, list):
        raise VerificationError("CROSSWALK_PROFILES_INVALID")

    coverage = schema.get("b1_candidate_field_coverage", {}).get("destinations")
    if not isinstance(coverage, dict) or len(coverage) != 90:
        raise VerificationError("SCHEMA_DESTINATION_MATRIX_INVALID")

    profile_map: dict[str, dict[str, Any]] = {}
    for profile in profiles:
        if not isinstance(profile, dict):
            raise VerificationError("SOURCE_PROFILE_NOT_OBJECT")
        profile_id = profile.get("profile_id")
        if (
            not isinstance(profile_id, str)
            or not profile_id
            or profile_id in profile_map
        ):
            raise VerificationError("SOURCE_PROFILE_ID_INVALID")
        profile_map[profile_id] = profile

    current_by_native: dict[str, dict[str, Any]] = {}
    source_ids: set[int] = set()
    for row in current_records:
        if not isinstance(row, dict):
            raise VerificationError("CURRENT_ROW_NOT_OBJECT")
        native_key, source_id = row.get("native_key"), row.get("source_item_id")
        if (
            not isinstance(native_key, str)
            or not isinstance(source_id, int)
            or isinstance(source_id, bool)
        ):
            raise VerificationError("CURRENT_ROW_IDENTITY_INVALID")
        if native_key in current_by_native or source_id in source_ids:
            raise VerificationError("CURRENT_ROW_IDENTITY_DUPLICATE")
        current_by_native[native_key] = row
        source_ids.add(source_id)

    vocabulary = set(REQUIRED_LOGICAL_FIELDS)
    for native_field, destination in coverage.items():
        if not isinstance(native_field, str) or not isinstance(destination, str):
            raise VerificationError("SCHEMA_DESTINATION_ENTRY_INVALID")
        vocabulary.update(field_paths_for_destination(native_field, destination))
    vocabulary.update(CURRENT_LOGICAL_PATH.values())

    # Crosswalk/current identity closure must be exact; no name or order fallback.
    seen_native: set[str] = set()
    seen_source: set[int] = set()
    for row in cross_records:
        if not isinstance(row, dict):
            raise VerificationError("CROSSWALK_ROW_NOT_OBJECT")
        native_key = row.get("native_key")
        source_id = row.get("source_item_id")
        profile_id = row.get("source_profile_id")
        if (
            not isinstance(native_key, str)
            or not isinstance(source_id, int)
            or isinstance(source_id, bool)
        ):
            raise VerificationError("CROSSWALK_ROW_IDENTITY_INVALID")
        if native_key in seen_native or source_id in seen_source:
            raise VerificationError("CROSSWALK_ROW_IDENTITY_DUPLICATE")
        seen_native.add(native_key)
        seen_source.add(source_id)
        current_row = current_by_native.get(native_key)
        if current_row is None or current_row.get("source_item_id") != source_id:
            raise VerificationError("CROSSWALK_CURRENT_IDENTITY_MISMATCH")
        if profile_id not in profile_map:
            raise VerificationError("CROSSWALK_PROFILE_MISSING")
    if len(seen_native) != TARGET_COUNT or len(seen_source) != TARGET_COUNT:
        raise VerificationError("IDENTITY_CLOSURE_FAILED")
    return profile_map, current_by_native, vocabulary


def profile_rules(
    profile: dict[str, Any],
    coverage: dict[str, str],
) -> dict[str, dict[str, Any]]:
    observations = profile.get("candidate_observations")
    if not isinstance(observations, list):
        raise VerificationError("PROFILE_OBSERVATIONS_INVALID")
    by_native: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for observation in observations:
        if not isinstance(observation, dict):
            raise VerificationError("PROFILE_OBSERVATION_NOT_OBJECT")
        native_field = observation.get("native_field")
        if native_field not in coverage:
            raise VerificationError(
                f"PROFILE_FIELD_WITHOUT_DESTINATION:{native_field}"
            )
        if observation.get("authority") != "OTS_HYPOTHESIS_ONLY":
            raise VerificationError(f"PROFILE_AUTHORITY_ESCALATION:{native_field}")
        by_native[native_field].append(observation)

    out: dict[str, dict[str, Any]] = {}
    for native_field in sorted(by_native):
        obs_list = by_native[native_field]
        distinct = {
            json_scalar_key(
                {
                    "source_value": observation.get("source_value"),
                    "nested_values": observation.get("nested_values", []),
                }
            )
            for observation in obs_list
        }
        state = "CONFLICT" if len(distinct) > 1 else "OTS_ONLY"
        continuity = "CONFLICT" if state == "CONFLICT" else "UNKNOWN"
        for field_path in field_paths_for_destination(
            native_field, coverage[native_field]
        ):
            candidate = {
                "field_path": field_path,
                "field_state": state,
                "continuity_to_target": continuity,
                "promotion": "BLOCKED",
                "reason": (
                    "CONFLICTING_OTS_HYPOTHESES"
                    if state == "CONFLICT"
                    else "OTS_HYPOTHESIS_ONLY"
                ),
                "source_native_fields": [native_field],
                "observations": [
                    {
                        "source": "PINNED_OTS",
                        "authority": "OTS_HYPOTHESIS_ONLY",
                        "source_key": observation.get("source_key"),
                        "value": observation.get("source_value"),
                        "nested_values": observation.get("nested_values", []),
                    }
                    for observation in obs_list
                ],
            }
            existing = out.get(field_path)
            if existing is None:
                out[field_path] = candidate
            else:
                # Multiple native fields may converge on one aggregate typed path. Preserve all
                # evidence and fail closed unless their complete observation payload is identical.
                existing["source_native_fields"] = sorted(
                    set(existing["source_native_fields"] + [native_field])
                )
                existing["observations"].extend(candidate["observations"])
                payloads = {
                    json_scalar_key(
                        {
                            "value": item.get("value"),
                            "nested_values": item.get("nested_values", []),
                        }
                    )
                    for item in existing["observations"]
                }
                if len(payloads) > 1:
                    existing["field_state"] = "CONFLICT"
                    existing["continuity_to_target"] = "CONFLICT"
                    existing["reason"] = "CONVERGED_SOURCE_FIELDS_CONFLICT"
    return {key: out[key] for key in sorted(out)}


def page_map(current: dict[str, Any]) -> dict[int, dict[str, Any]]:
    pages = current.get("pages")
    if not isinstance(pages, list):
        raise VerificationError("CURRENT_PAGES_INVALID")
    out: dict[int, dict[str, Any]] = {}
    for page in pages:
        if not isinstance(page, dict):
            raise VerificationError("CURRENT_PAGE_NOT_OBJECT")
        page_id = page.get("page_id")
        if (
            not isinstance(page_id, int)
            or isinstance(page_id, bool)
            or page_id <= 0
            or page_id in out
        ):
            raise VerificationError("CURRENT_PAGE_ID_INVALID")
        out[page_id] = page
    return out


def current_value(page: dict[str, Any], key: str) -> Any | None:
    fields = page.get("normalized_fields")
    field = fields.get(key) if isinstance(fields, dict) else None
    if not isinstance(field, dict) or field.get("state") != "VALUE":
        return None
    return field.get("value")


def ots_simple_values(profile: dict[str, Any], native_field: str) -> set[int]:
    values: set[int] = set()
    observations = profile.get("candidate_observations", [])
    for observation in observations:
        if (
            isinstance(observation, dict)
            and observation.get("native_field") == native_field
            and observation.get("nested_values") in (None, [])
        ):
            parsed = simple_int(observation.get("source_value"))
            if parsed is not None:
                values.add(parsed)
    return values


def current_overlay(
    row: dict[str, Any],
    profile: dict[str, Any],
    pages: dict[int, dict[str, Any]],
    coverage: dict[str, str],
) -> dict[str, dict[str, Any]]:
    current_source = row.get("current_source")
    if not isinstance(current_source, dict):
        raise VerificationError("CURRENT_SOURCE_DISPOSITION_MISSING")
    disposition = current_source.get("disposition")
    page_ids = current_source.get("candidate_page_ids")
    if disposition not in {
        "WIKI_MATCHED",
        "WIKI_NOT_FOUND",
        "WIKI_AMBIGUOUS",
        "WIKI_CONFLICT",
    }:
        raise VerificationError(
            f"CURRENT_SOURCE_DISPOSITION_INVALID:{disposition}"
        )
    if not isinstance(page_ids, list) or any(
        not isinstance(item, int) for item in page_ids
    ):
        raise VerificationError("CURRENT_SOURCE_PAGE_IDS_INVALID")

    # Never bind ambiguous/not-found pages to an Item field. Name/title alone remains
    # insufficient. For WIKI_CONFLICT only explicitly contradicted non-name signals become field
    # conflicts; other page values remain unbound.
    if disposition in {"WIKI_NOT_FOUND", "WIKI_AMBIGUOUS"}:
        return {}

    if disposition == "WIKI_MATCHED" and len(page_ids) != 1:
        raise VerificationError("MATCHED_PAGE_CARDINALITY_INVALID")
    selected_pages = [pages[item] for item in page_ids if item in pages]
    if len(selected_pages) != len(page_ids):
        raise VerificationError("CURRENT_PAGE_RECORD_MISSING")

    out: dict[str, dict[str, Any]] = {}
    if disposition == "WIKI_CONFLICT":
        contradicted = current_source.get("contradicted_non_name_signals", [])
        matched = current_source.get("matched_non_name_signals", [])
        if not isinstance(contradicted, list) or not isinstance(matched, list):
            raise VerificationError("CURRENT_SOURCE_SIGNAL_LIST_INVALID")
        for wiki_key in contradicted:
            native_field = COMPARABLE_CURRENT_TO_B1.get(wiki_key)
            if native_field is None or native_field not in coverage:
                continue
            for page in selected_pages:
                value = current_value(page, wiki_key)
                if value is None:
                    continue
                path = field_paths_for_destination(
                    native_field, coverage[native_field]
                )[0]
                out[path] = {
                    "field_path": path,
                    "field_state": "CONFLICT",
                    "continuity_to_target": "CONFLICT",
                    "promotion": "BLOCKED",
                    "reason": (
                        "CURRENT_STRUCTURED_REFERENCE_CONTRADICTS_PINNED_SIGNAL"
                    ),
                    "observations": [
                        {
                            "source": "TIBIAWIKI_STRUCTURED",
                            "authority": "STRUCTURED_REFERENCE_DATA",
                            "page_id": page["page_id"],
                            "revision_id": page.get("revision_id"),
                            "revision_timestamp": page.get("revision_timestamp"),
                            "source_field": wiki_key,
                            "value": value,
                        }
                    ],
                }

        # One contradicted field must not erase an independently corroborated field. Keep this
        # narrow: only an exact one-page candidate and only non-name signals that independently
        # equal the pinned observation may survive the Item-level conflict.
        if len(selected_pages) == 1:
            page = selected_pages[0]
            for wiki_key in matched:
                native_field = COMPARABLE_CURRENT_TO_B1.get(wiki_key)
                if native_field is None or native_field not in coverage:
                    continue
                value = current_value(page, wiki_key)
                parsed_current = simple_int(value)
                if parsed_current is None:
                    continue
                if ots_simple_values(profile, native_field) != {parsed_current}:
                    continue
                path = field_paths_for_destination(
                    native_field, coverage[native_field]
                )[0]
                if path in out:
                    continue
                continuity = page.get("target_continuity")
                if continuity not in CONTINUITY_STATES:
                    continuity = "UNKNOWN"
                promotion = (
                    "ELIGIBLE"
                    if continuity in {"PROVEN", "DERIVED"}
                    else "BLOCKED"
                )
                out[path] = {
                    "field_path": path,
                    "field_state": "CORROBORATED_CURRENT",
                    "continuity_to_target": continuity,
                    "promotion": promotion,
                    "reason": (
                        "CURRENT_SIGNAL_CORROBORATED_DESPITE_SIBLING_FIELD_CONFLICT"
                        if promotion == "BLOCKED"
                        else "CURRENT_SIGNAL_CORROBORATED_DESPITE_SIBLING_FIELD_CONFLICT_WITH_TARGET_CONTINUITY"
                    ),
                    "observations": [
                        {
                            "source": "TIBIAWIKI_STRUCTURED",
                            "authority": "STRUCTURED_REFERENCE_DATA",
                            "page_id": page["page_id"],
                            "revision_id": page.get("revision_id"),
                            "revision_timestamp": page.get("revision_timestamp"),
                            "source_field": wiki_key,
                            "value": value,
                        }
                    ],
                }
        return {key: out[key] for key in sorted(out)}

    page = selected_pages[0]
    normalized = page.get("normalized_fields")
    if not isinstance(normalized, dict):
        raise VerificationError("MATCHED_PAGE_FIELDS_INVALID")

    discovery_names = row.get("discovery_names")
    if not isinstance(discovery_names, list):
        discovery_names = []
    for wiki_key in sorted(normalized):
        entry = normalized[wiki_key]
        if not isinstance(entry, dict) or entry.get("state") != "VALUE":
            continue
        value = entry.get("value")
        path = CURRENT_LOGICAL_PATH.get(wiki_key)
        if path is None:
            continue
        state = "UNKNOWN"
        reason = "CURRENT_STRUCTURED_REFERENCE_ONLY"
        continuity = page.get("target_continuity")
        if continuity not in CONTINUITY_STATES:
            continuity = "UNKNOWN"

        native_field = COMPARABLE_CURRENT_TO_B1.get(wiki_key)
        if native_field is not None:
            ots_values = ots_simple_values(profile, native_field)
            parsed_current = simple_int(value)
            if parsed_current is not None and ots_values == {parsed_current}:
                state = "CORROBORATED_CURRENT"
                reason = (
                    "CURRENT_STRUCTURED_REFERENCE_CORROBORATED_BY_PINNED_SIGNAL"
                )
            elif (
                parsed_current is not None
                and ots_values
                and parsed_current not in ots_values
            ):
                state = "CONFLICT"
                continuity = "CONFLICT"
                reason = (
                    "CURRENT_STRUCTURED_REFERENCE_CONTRADICTS_PINNED_SIGNAL"
                )
            # Use exact schema destination rather than the logical alias when this comparison is
            # admitted by the protected matrix.
            path = field_paths_for_destination(
                native_field, coverage[native_field]
            )[0]
        elif wiki_key == "name" and isinstance(value, str):
            if any(
                normalized_text(value) == normalized_text(name)
                for name in discovery_names
                if isinstance(name, str)
            ):
                state = "CORROBORATED_CURRENT"
                reason = "CURRENT_NAME_CORROBORATED_AFTER_NON_NAME_IDENTITY_MATCH"

        promotion = (
            "ELIGIBLE"
            if state in {"CONFIRMED_CURRENT", "CORROBORATED_CURRENT"}
            and continuity in {"PROVEN", "DERIVED"}
            else "BLOCKED"
        )
        out[path] = {
            "field_path": path,
            "field_state": state,
            "continuity_to_target": continuity,
            "promotion": promotion,
            "reason": (
                reason
                if promotion == "BLOCKED"
                else f"{reason}_WITH_TARGET_CONTINUITY"
            ),
            "observations": [
                {
                    "source": "TIBIAWIKI_STRUCTURED",
                    "authority": "STRUCTURED_REFERENCE_DATA",
                    "page_id": page["page_id"],
                    "revision_id": page.get("revision_id"),
                    "revision_timestamp": page.get("revision_timestamp"),
                    "source_field": wiki_key,
                    "value": value,
                }
            ],
        }
    return {key: out[key] for key in sorted(out)}


def compile_verification(
    crosswalk: dict[str, Any],
    current: dict[str, Any],
    schema: dict[str, Any],
    *,
    source_digests: dict[str, str] | None = None,
) -> dict[str, Any]:
    profile_map, current_by_native, vocabulary = validate_inputs(
        crosswalk, current, schema
    )
    coverage: dict[str, str] = schema["b1_candidate_field_coverage"][
        "destinations"
    ]
    pages = page_map(current)

    compiled_profiles: dict[str, dict[str, Any]] = {}
    for profile_id in sorted(profile_map):
        fields = profile_rules(profile_map[profile_id], coverage)
        compiled_profiles[profile_id] = {
            "profile_id": profile_id,
            "fields": fields,
        }

    records: list[dict[str, Any]] = []
    expanded_field_states: Counter[str] = Counter()
    expanded_continuity: Counter[str] = Counter()
    promotion_counts: Counter[str] = Counter()

    for cross_row in crosswalk["records"]:
        native_key = cross_row["native_key"]
        current_row = current_by_native[native_key]
        profile_id = cross_row["source_profile_id"]
        overlay = current_overlay(
            current_row, profile_map[profile_id], pages, coverage
        )

        # Expand only for counts/invariants; full output retains profile + sparse overlay.
        effective = dict(compiled_profiles[profile_id]["fields"])
        effective.update(overlay)
        for field_path in vocabulary:
            field = effective.get(field_path)
            if field is None:
                expanded_field_states["UNKNOWN"] += 1
                expanded_continuity["UNKNOWN"] += 1
                promotion_counts["BLOCKED"] += 1
            else:
                state = field["field_state"]
                continuity = field["continuity_to_target"]
                promotion = field["promotion"]
                if state not in FIELD_STATES or continuity not in CONTINUITY_STATES:
                    raise VerificationError("RULE_ENGINE_OUTPUT_ENUM_INVALID")
                expanded_field_states[state] += 1
                expanded_continuity[continuity] += 1
                promotion_counts[promotion] += 1

        records.append(
            {
                "source_item_id": cross_row["source_item_id"],
                "native_key": native_key,
                "source_profile_id": profile_id,
                "identity_origin": cross_row.get("identity_origin"),
                "current_source_disposition": current_row["current_source"][
                    "disposition"
                ],
                "field_overrides": overlay,
            }
        )

    if len(records) != TARGET_COUNT:
        raise VerificationError("VERIFICATION_RECORD_COUNT_MISMATCH")
    total_slots = TARGET_COUNT * len(vocabulary)
    if sum(expanded_field_states.values()) != total_slots:
        raise VerificationError("FIELD_STATE_PARTITION_MISMATCH")
    if sum(expanded_continuity.values()) != total_slots:
        raise VerificationError("CONTINUITY_PARTITION_MISMATCH")
    if sum(promotion_counts.values()) != total_slots:
        raise VerificationError("PROMOTION_PARTITION_MISMATCH")

    return {
        "schema": SCHEMA,
        "rule_profile": RULE_PROFILE,
        "target_cut": TARGET_CUT,
        "authority": {
            "identity_minting": "FORBIDDEN",
            "source_reimport": "FORBIDDEN",
            "semantic_promotion": "FORBIDDEN_IN_THIS_GENERATION",
            "runtime": "NONE",
            "client": "NONE",
        },
        "inputs": source_digests or {},
        "field_vocabulary": sorted(vocabulary),
        "default_field_disposition": {
            "field_state": "UNKNOWN",
            "continuity_to_target": "UNKNOWN",
            "promotion": "BLOCKED",
            "reason": "NO_ADMITTED_FIELD_EVIDENCE",
        },
        "profile_field_rules": [
            compiled_profiles[key] for key in sorted(compiled_profiles)
        ],
        "records": records,
        "counts": {
            "records": len(records),
            "source_profiles": len(compiled_profiles),
            "field_vocabulary": len(vocabulary),
            "expanded_field_slots": total_slots,
            "field_states": {
                key: expanded_field_states[key] for key in FIELD_STATES
            },
            "continuity": {
                key: expanded_continuity[key] for key in CONTINUITY_STATES
            },
            "promotion": dict(sorted(promotion_counts.items())),
        },
        "invariants": {
            "records_exact_38157": True,
            "identity_join_is_exact_key_and_source_id": True,
            "name_only_identity_resolution": False,
            "missing_coerced_to_false_or_zero": False,
            "majority_voting": False,
            "weaker_evidence_overwrites_stronger": False,
            "conflicts_masked": False,
            "current_value_auto_equals_target_value": False,
            "semantic_promotion_performed": False,
        },
    }


def build_manifest(
    full: dict[str, Any], *, compiler_sha256: str
) -> dict[str, Any]:
    return {
        "schema": MANIFEST_SCHEMA,
        "status": (
            "FIELD_VERIFICATION_AND_RULE_ENGINE_COMPLETE_NO_SEMANTIC_PROMOTION"
        ),
        "rule_profile": RULE_PROFILE,
        "target_cut": TARGET_CUT,
        "compiler": {
            "path": (
                "tools/reference-world-corridor-census/"
                "item_field_verification.py"
            ),
            "sha256": compiler_sha256,
        },
        "inputs": full["inputs"],
        "counts": full["counts"],
        "full_output": {
            "schema": SCHEMA,
            "sha256": sha256_bytes(canonical_bytes(full)),
            "committed_bulk_corpus": False,
        },
        "invariants": full["invariants"],
        "promotion_policy": {
            "eligible_only_when": [
                (
                    "field_state in "
                    "{CONFIRMED_CURRENT,CORROBORATED_CURRENT}"
                ),
                "continuity_to_target in {PROVEN,DERIVED}",
            ],
            "ots_only_promotable": False,
            "conflict_promotable": False,
            "unknown_promotable": False,
        },
        "limitations": [
            (
                "The protected #767 current-source snapshot and later fresh "
                "observations retain target_continuity=UNKNOWN unless separately "
                "proven; current September values are never auto-promoted to the "
                "July 28 target."
            ),
            (
                "TibiaWiki structured data remains Reference evidence, not "
                "gameplay truth by itself."
            ),
            (
                "Fields not represented by admitted evidence remain explicit "
                "UNKNOWN through the global field vocabulary plus default "
                "disposition."
            ),
            (
                "This generation performs no canonical Project/Reference "
                "semantic mutation."
            ),
        ],
        "next_gate": "SEMANTIC_PROMOTION_FROM_ELIGIBLE_FIELDS",
    }


def compile_files(args: argparse.Namespace) -> None:
    crosswalk, cross_bytes = load_json(args.classification_crosswalk)
    current, current_bytes = load_json(args.current_source)
    protected_current_manifest, protected_current_manifest_bytes = load_json(
        args.protected_current_source_manifest
    )
    schema, schema_bytes = load_json(args.schema_readiness)
    if sha256_bytes(cross_bytes) != CROSSWALK_FULL_SHA256:
        raise VerificationError("PROTECTED_CROSSWALK_DIGEST_MISMATCH")
    if sha256_bytes(schema_bytes) != SCHEMA_READINESS_SHA256:
        raise VerificationError("PROTECTED_SCHEMA_READINESS_DIGEST_MISMATCH")
    if (
        sha256_bytes(protected_current_manifest_bytes)
        != PROTECTED_CURRENT_SOURCE_MANIFEST_SHA256
    ):
        raise VerificationError("PROTECTED_CURRENT_SOURCE_MANIFEST_DIGEST_MISMATCH")
    if protected_current_manifest.get("schema") != CURRENT_SOURCE_MANIFEST_SCHEMA:
        raise VerificationError("PROTECTED_CURRENT_SOURCE_MANIFEST_SCHEMA_MISMATCH")
    if protected_current_manifest.get("target_cut") != TARGET_CUT:
        raise VerificationError("PROTECTED_CURRENT_SOURCE_TARGET_CUT_MISMATCH")
    protected_current_output = protected_current_manifest.get("full_output")
    if not isinstance(protected_current_output, dict):
        raise VerificationError("PROTECTED_CURRENT_SOURCE_OUTPUT_MISSING")
    protected_current_sha = protected_current_output.get("sha256")
    if not isinstance(protected_current_sha, str) or len(protected_current_sha) != 64:
        raise VerificationError("PROTECTED_CURRENT_SOURCE_SHA_INVALID")
    if protected_current_sha != PROTECTED_CURRENT_SOURCE_SHA256:
        raise VerificationError("PROTECTED_CURRENT_SOURCE_SHA_MISMATCH")
    protected_collector = protected_current_manifest.get("collector")
    if (
        not isinstance(protected_collector, dict)
        or protected_collector.get("profile") != CURRENT_SOURCE_PROFILE
    ):
        raise VerificationError("PROTECTED_CURRENT_SOURCE_PROFILE_MISMATCH")
    observed_current_sha = sha256_bytes(current_bytes)
    observation_status = (
        "EXACT_PROTECTED_REPRODUCTION"
        if observed_current_sha == protected_current_sha
        else "FRESH_OBSERVATION_DIVERGED_FROM_PROTECTED_SNAPSHOT"
    )
    source_digests = {
        "classification_crosswalk_sha256": sha256_bytes(cross_bytes),
        "protected_current_source_manifest_sha256": sha256_bytes(
            protected_current_manifest_bytes
        ),
        "protected_current_source_sha256": protected_current_sha,
        "observed_current_source_sha256": observed_current_sha,
        "current_source_observation_status": observation_status,
        "schema_readiness_sha256": sha256_bytes(schema_bytes),
    }
    full = compile_verification(
        crosswalk, current, schema, source_digests=source_digests
    )
    full_bytes = canonical_bytes(full)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(full_bytes)
    compiler_sha = sha256_bytes(Path(__file__).read_bytes().replace(b"\r\n", b"\n"))
    manifest = build_manifest(full, compiler_sha256=compiler_sha)
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    print(
        "item-field-verification: PASS "
        f"records={full['counts']['records']} "
        f"fields={full['counts']['field_vocabulary']} "
        f"eligible={full['counts']['promotion'].get('ELIGIBLE', 0)} "
        f"digest={manifest['full_output']['sha256']}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--classification-crosswalk", type=Path, required=True
    )
    parser.add_argument("--current-source", type=Path, required=True)
    parser.add_argument(
        "--protected-current-source-manifest", type=Path, required=True
    )
    parser.add_argument("--schema-readiness", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    args = parser.parse_args()
    compile_files(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
