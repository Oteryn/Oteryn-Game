#!/usr/bin/env python3
"""Compile the protected Item identity closure into a classification crosswalk.

The compiler consumes already-published Oteryn evidence.  It does not parse the
upstream XML, mint identities from source values, or promote OTS observations to
Reference truth.  Its full output is reproducible scratch evidence; the optional
manifest is the compact, commit-suitable proof of the full-record compilation.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
from typing import Any, Iterable


SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK/v1"
MANIFEST_SCHEMA = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK_MANIFEST/v1"
PROFILE = "OTERYN_ITEM_CLASSIFICATION_CROSSWALK_COMPILER/v1"
SOURCE_CLASSIFICATION = "CRYSTAL_OTS / OTS_HYPOTHESIS_ONLY"
TARGET_COUNT = 38_157
PRESERVED_COUNT = 64
OPAQUE_COUNT = 38_093
REVISION = "definition-r1"
ALLOCATION_DIGEST = "ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966"
NATIVE_MAP_SCHEMA = "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1"
NATIVE_MAP_MAX_BYTES = 5_789_755

PINNED_INPUTS = {
    "b1_catalog": (
        "docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json",
        "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7",
    ),
    "native_batch": (
        "docs/agents/evidence/OTV2-20260921-content-world-cw2-native-item-batch.json",
        "120e736120af7b804b4a859b765ad892e5331e52d16ca9b8c3166d700d487ccd",
    ),
    "family_registry": (
        "docs/agents/evidence/OTV2-20260921-content-world-item-family-scale-registry.json",
        "dac74e78e3c7fc687f9cae7990d071c1ab5b247f40dec2baeea28c081b0114f6",
    ),
    "schema_readiness": (
        "docs/agents/evidence/OTV2-20260922-content-world-item-schema-readiness.json",
        "c69b7626ca052b5494d14c034848088175f79a662aa7d9e41876d3452917eb78",
    ),
}

CAPABILITIES = (
    "presentation", "classification", "physical", "stack", "equipment", "weapon",
    "protection", "skill_modifiers", "charges", "temporal", "container", "imbuement",
    "use_transform", "trade_restrictions", "fluid", "readable_writeable",
)
CLASSIFICATION_CAPABILITIES = (
    "weapon", "armor", "helmet", "legs", "boots", "shield", "ammo", "rune",
    "container", "consumable", "currency", "material", "loot", "decoration", "key",
    "book_readable", "fluid", "usable", "transformable", "stackable", "charge_based",
    "imbueable", "presentation_only", "other",
)
OUTCOMES = ("EXACT_ONE", "ZERO_MATCH", "AMBIGUOUS", "CONFLICT")


class CrosswalkError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_pinned(root: Path, name: str) -> dict[str, Any]:
    relative, expected = PINNED_INPUTS[name]
    payload = (root / relative).read_bytes()
    actual = digest(payload)
    if actual != expected:
        raise CrosswalkError(f"PINNED_INPUT_DIGEST_MISMATCH:{name}:{actual}")
    value = json.loads(payload)
    if not isinstance(value, dict):
        raise CrosswalkError(f"PINNED_INPUT_ROOT_INVALID:{name}")
    return value


def read_native_map(path: Path) -> tuple[dict[str, Any], bytes]:
    with path.open("rb") as source:
        payload = source.read(NATIVE_MAP_MAX_BYTES + 1)
    if len(payload) > NATIVE_MAP_MAX_BYTES:
        raise CrosswalkError(f"CANONICAL_NATIVE_MAP_MAX_PLUS_ONE:{len(payload)}")
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise CrosswalkError("CANONICAL_NATIVE_MAP_JSON_INVALID") from exc
    if not isinstance(value, dict):
        raise CrosswalkError("CANONICAL_NATIVE_MAP_ROOT_INVALID")
    return value, payload


def classify_candidates(candidates: Iterable[dict[str, Any]]) -> dict[str, Any]:
    values = list(candidates)
    if not values:
        return {"outcome": "ZERO_MATCH", "native_key": None, "candidate_native_keys": []}
    keys: list[str] = []
    asserted = False
    for value in values:
        key = value.get("native_key")
        state = value.get("state")
        if not isinstance(key, str) or not key.startswith("oteryn:item."):
            raise CrosswalkError("CROSSWALK_NATIVE_KEY_INVALID")
        if state not in {"ASSERTED", "CANDIDATE"}:
            raise CrosswalkError("CROSSWALK_CANDIDATE_STATE_INVALID")
        asserted |= state == "ASSERTED"
        keys.append(key)
    unique = sorted(set(keys))
    if len(values) == 1:
        return {"outcome": "EXACT_ONE", "native_key": unique[0], "candidate_native_keys": unique}
    if len(unique) == 1:
        return {"outcome": "CONFLICT", "native_key": None, "candidate_native_keys": unique,
                "reason": "duplicate source-to-native binding rows"}
    outcome = "CONFLICT" if asserted else "AMBIGUOUS"
    return {"outcome": outcome, "native_key": None, "candidate_native_keys": unique}


def _source_id(binding: dict[str, Any]) -> int:
    identity = binding.get("source_identity")
    if not isinstance(identity, str) or not identity.startswith("crystal:item:"):
        raise CrosswalkError("PRESERVED_SOURCE_IDENTITY_INVALID")
    try:
        return int(identity.rsplit(":", 1)[1])
    except ValueError as exc:
        raise CrosswalkError("PRESERVED_SOURCE_IDENTITY_INVALID") from exc


def validate_native_map(rows: list[dict[str, Any]], bindings: list[dict[str, Any]], registry: dict[str, Any], native_map: dict[str, Any]) -> tuple[list[dict[str, Any]], str]:
    if len(rows) != TARGET_COUNT:
        raise CrosswalkError(f"IDENTITY_COUNT_MISMATCH:{len(rows)}")
    if len(bindings) != PRESERVED_COUNT:
        raise CrosswalkError(f"PRESERVED_BINDING_COUNT_MISMATCH:{len(bindings)}")
    admitted = registry.get("identity_admission", {})
    if admitted.get("target_count") != TARGET_COUNT or admitted.get("opaque_identity_only_bindings") != OPAQUE_COUNT:
        raise CrosswalkError("FAMILY_REGISTRY_COUNT_MISMATCH")

    if native_map.get("schema") != NATIVE_MAP_SCHEMA:
        raise CrosswalkError("CANONICAL_NATIVE_MAP_SCHEMA_MISMATCH")
    if native_map.get("producer") != "protected_cw2_b1_full_item_family_import":
        raise CrosswalkError("CANONICAL_NATIVE_MAP_PRODUCER_MISMATCH")
    if native_map.get("profile") != "OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORTER/v1":
        raise CrosswalkError("CANONICAL_NATIVE_MAP_PROFILE_MISMATCH")
    if native_map.get("protected_catalog_sha256") != PINNED_INPUTS["b1_catalog"][1]:
        raise CrosswalkError("CANONICAL_NATIVE_MAP_CATALOG_MISMATCH")
    if (native_map.get("item_count") != TARGET_COUNT
            or native_map.get("preserved_semantic_bindings") != PRESERVED_COUNT
            or native_map.get("opaque_registry_allocations") != OPAQUE_COUNT):
        raise CrosswalkError("CANONICAL_NATIVE_MAP_DECLARED_COUNTS_MISMATCH")
    map_rows = native_map.get("records")
    if not isinstance(map_rows, list) or len(map_rows) != TARGET_COUNT:
        raise CrosswalkError("CANONICAL_NATIVE_MAP_COUNT_MISMATCH")
    if native_map.get("allocation_digest_sha256") != ALLOCATION_DIGEST:
        raise CrosswalkError("CANONICAL_NATIVE_MAP_ALLOCATION_DIGEST_MISMATCH")

    preserved: dict[int, dict[str, Any]] = {}
    for binding in bindings:
        source_id = _source_id(binding)
        if source_id in preserved:
            raise CrosswalkError(f"DUPLICATE_PRESERVED_SOURCE_ID:{source_id}")
        preserved[source_id] = binding

    allocations: list[dict[str, Any]] = []
    seen_source: set[int] = set()
    seen_native: set[str] = set()
    previous: int | None = None
    allocation_digest_input = bytearray()
    for index, row in enumerate(rows):
        source_id = row.get("source_item_id")
        if not isinstance(source_id, int) or isinstance(source_id, bool):
            raise CrosswalkError("SOURCE_ITEM_ID_INVALID")
        if source_id in seen_source:
            raise CrosswalkError(f"DUPLICATE_SOURCE_ITEM_ID:{source_id}")
        if previous is not None and source_id <= previous:
            raise CrosswalkError(f"SOURCE_ITEM_ID_ORDER:{source_id}")
        previous = source_id
        seen_source.add(source_id)
        exported = map_rows[index]
        if not isinstance(exported, dict) or exported.get("source_item_id") != source_id:
            raise CrosswalkError(f"CANONICAL_NATIVE_MAP_SOURCE_MISMATCH:{source_id}")
        native_key = exported.get("native_key")
        origin = exported.get("identity_origin")
        if exported.get("native_revision") != REVISION:
            raise CrosswalkError(f"CANONICAL_NATIVE_MAP_REVISION_MISMATCH:{source_id}")
        binding = preserved.get(source_id)
        if binding is not None:
            native = binding.get("native_identity", {})
            if native_key != native.get("key"):
                raise CrosswalkError(f"PRESERVED_BINDING_REMAP:{source_id}")
            if row.get("source_node_digest") != binding.get("source_node_sha256") or row.get("field_profile_id") != binding.get("field_profile_sha256"):
                raise CrosswalkError(f"PRESERVED_BINDING_PROVENANCE_MISMATCH:{source_id}")
            if origin != "PRESERVED_SEMANTIC_BINDING":
                raise CrosswalkError(f"PRESERVED_BINDING_ORIGIN_MISMATCH:{source_id}")
        elif origin != "OPAQUE_REGISTRY_ALLOCATION":
            raise CrosswalkError(f"OPAQUE_BINDING_ORIGIN_MISMATCH:{source_id}")
        if not isinstance(native_key, str) or not native_key.startswith("oteryn:item."):
            raise CrosswalkError(f"NATIVE_KEY_INVALID:{source_id}")
        if native_key in seen_native:
            raise CrosswalkError(f"DUPLICATE_NATIVE_KEY:{native_key}")
        seen_native.add(native_key)
        allocation_digest_input.extend(str(source_id).encode())
        allocation_digest_input.append(0)
        allocation_digest_input.extend(native_key.encode())
        allocation_digest_input.append(10)
        allocations.append({
            "source_item_id": source_id,
            "source_node_digest": row.get("source_node_digest"),
            "field_profile_id": row.get("field_profile_id"),
            "native_key": native_key,
            "native_revision": REVISION,
            "identity_origin": origin,
        })
    if (len(seen_source) != TARGET_COUNT or len(seen_native) != TARGET_COUNT
            or sum(row["identity_origin"] == "PRESERVED_SEMANTIC_BINDING" for row in allocations) != PRESERVED_COUNT
            or sum(row["identity_origin"] == "OPAQUE_REGISTRY_ALLOCATION" for row in allocations) != OPAQUE_COUNT):
        raise CrosswalkError("IDENTITY_CLOSURE_FAILED")
    actual_digest = digest(bytes(allocation_digest_input))
    expected_digest = registry.get("validation", {}).get("allocation_digest_sha256")
    if expected_digest != ALLOCATION_DIGEST or actual_digest != ALLOCATION_DIGEST:
        raise CrosswalkError(f"ALLOCATION_REMAP:{actual_digest}")
    return allocations, actual_digest


def destination_capability(destination: str) -> tuple[str | None, str]:
    if destination.startswith("EXPLICIT_UNSUPPORTED_"):
        return None, destination
    capability = destination.split(".", 1)[0]
    aliases = {"transform": "use_transform", "read_write": "readable_writeable"}
    capability = aliases.get(capability, capability)
    if capability not in CAPABILITIES:
        raise CrosswalkError(f"SCHEMA_DESTINATION_UNKNOWN:{destination}")
    return capability, "TYPED_DESTINATION_AVAILABLE_NOT_PROMOTED"


def classification_signals(observations: list[dict[str, Any]]) -> set[str]:
    signals: set[str] = set()
    transform_fields = {
        "rotate_target_source_id", "wrap_target_source_id", "use_target_source_id",
        "equip_target_source_id", "deequip_target_source_id", "male_transform_target_source_id",
        "female_transform_target_source_id", "destroy_target_source_id",
    }
    for observation in observations:
        field = observation.get("native_field")
        value = observation.get("source_value")
        if field == "weapon_type":
            signals.add("weapon")
            if value == "shield":
                signals.add("shield")
            if value == "ammunition":
                signals.add("ammo")
        elif field == "ammo_type":
            signals.add("ammo")
        elif field == "slot_claim" and value == "head":
            signals.add("helmet")
        elif field == "item_type" and value in {"rune", "container", "key"}:
            signals.add(str(value))
        elif field == "capacity":
            signals.add("container")
        elif field == "readable":
            signals.add("book_readable")
        elif field == "fluid_source":
            signals.add("fluid")
        elif field == "charge_count":
            signals.add("charge_based")
        elif field == "slot_and_allowed_family_tier":
            signals.add("imbueable")
        if field in transform_fields:
            signals.add("transformable")
        if field == "use_target_source_id":
            signals.add("usable")
    return signals


def source_profiles(b1: dict[str, Any], schema: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], dict[str, str], dict[str, int]]:
    catalog = b1.get("semantic_catalog", {})
    semantic_records = catalog.get("semantic_candidate_node_records")
    identity_rows = catalog.get("identity_records")
    if not isinstance(semantic_records, list) or not isinstance(identity_rows, list):
        raise CrosswalkError("B1_SEMANTIC_CATALOG_INVALID")
    coverage = schema.get("b1_candidate_field_coverage", {}).get("destinations")
    if not isinstance(coverage, dict) or len(coverage) != 90:
        raise CrosswalkError("SCHEMA_FIELD_COVERAGE_INVALID")
    by_node: dict[str, list[dict[str, Any]]] = {}
    for record in semantic_records:
        node = record.get("source_node_digest")
        fields = record.get("candidate_fields")
        if not isinstance(node, str) or not isinstance(fields, list) or node in by_node:
            raise CrosswalkError("B1_SEMANTIC_NODE_INVALID")
        by_node[node] = fields

    profiles: dict[str, dict[str, Any]] = {}
    node_to_profile: dict[str, str] = {}
    counts: Counter[str] = Counter()
    for row in identity_rows:
        node = row.get("source_node_digest")
        if not isinstance(node, str):
            raise CrosswalkError("B1_SOURCE_NODE_DIGEST_INVALID")
        if node in node_to_profile:
            continue
        observations = by_node.get(node, [])
        observed_capabilities: set[str] = set()
        compact_observations: list[dict[str, Any]] = []
        classification_hypotheses: list[dict[str, Any]] = []
        for observation in observations:
            native_field = observation.get("native_field")
            if native_field not in coverage:
                raise CrosswalkError(f"B1_FIELD_WITHOUT_SCHEMA_DISPOSITION:{native_field}")
            destination = coverage[native_field]
            capability, disposition = destination_capability(destination)
            if capability is not None:
                observed_capabilities.add(capability)
            compact = {
                "native_field": native_field,
                "source_key": observation.get("normalized_source_key"),
                "source_value": observation.get("source_value"),
                "nested_values": observation.get("nested_values", []),
                "schema_destination": destination,
                "disposition": disposition,
                "authority": "OTS_HYPOTHESIS_ONLY",
            }
            compact_observations.append(compact)
            if native_field == "item_type":
                classification_hypotheses.append({
                    "source_value": observation.get("source_value"),
                    "authority": "OTS_HYPOTHESIS_ONLY",
                    "accepted_reference_state": "UNKNOWN",
                })
        compact_observations.sort(key=canonical_bytes)
        classification_hypotheses.sort(key=canonical_bytes)
        capability_states = {
            name: {
                "source_observation_state": "PRESENT_OTS_ONLY" if name in observed_capabilities else "ABSENT_IN_PINNED_OTS_NODE",
                "accepted_reference_state": "UNKNOWN",
                "current_source_state": "NOT_EVALUATED",
            }
            for name in CAPABILITIES
        }
        category_signals = classification_signals(observations)
        classification_capability_states = {
            name: {
                "source_observation_state": "PRESENT_OTS_ONLY" if name in category_signals else "ABSENT_IN_PINNED_OTS_NODE",
                "accepted_reference_state": "UNKNOWN",
                "current_source_state": "NOT_EVALUATED",
            }
            for name in CLASSIFICATION_CAPABILITIES
        }
        profile = {
            "source_node_digest": node,
            "source_classification": SOURCE_CLASSIFICATION,
            "candidate_observations": compact_observations,
            "classification_hypotheses": classification_hypotheses,
            "accepted_reference_classification": "UNKNOWN",
            "capability_states": capability_states,
            "classification_capability_states": classification_capability_states,
        }
        profile_id = digest(canonical_bytes(profile))
        profile["profile_id"] = profile_id
        profiles.setdefault(profile_id, profile)
        node_to_profile[node] = profile_id
        counts["nodes_with_ots_candidates" if observations else "nodes_without_ots_candidates"] += 1
        counts["ots_candidate_observations"] += len(observations)
        counts["classification_hypotheses"] += len(classification_hypotheses)
        for name in observed_capabilities:
            counts[f"capability_present:{name}"] += 1
        for name in category_signals:
            counts[f"classification_capability_profile_present:{name}"] += 1
    if len(node_to_profile) != len({row["source_node_digest"] for row in identity_rows}):
        raise CrosswalkError("SOURCE_PROFILE_CLOSURE_FAILED")
    return profiles, node_to_profile, dict(sorted(counts.items()))


def compile_crosswalk(b1: dict[str, Any], batch: dict[str, Any], registry: dict[str, Any], schema: dict[str, Any], native_map: dict[str, Any], native_map_sha256: str) -> dict[str, Any]:
    rows = b1.get("semantic_catalog", {}).get("identity_records")
    bindings = batch.get("binding_map")
    if not isinstance(rows, list) or not isinstance(bindings, list):
        raise CrosswalkError("PROTECTED_INPUT_SHAPE_INVALID")
    allocations, allocation_digest = validate_native_map(rows, bindings, registry, native_map)
    profiles, node_to_profile, profile_counts = source_profiles(b1, schema)
    candidates_by_source: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for allocation in allocations:
        candidates_by_source[allocation["source_item_id"]].append({
            "native_key": allocation["native_key"], "state": "ASSERTED",
        })
    records: list[dict[str, Any]] = []
    outcomes: Counter[str] = Counter()
    classification_capability_records: Counter[str] = Counter()
    native_keys: set[str] = set()
    for allocation in allocations:
        source_id = allocation["source_item_id"]
        crosswalk = classify_candidates(candidates_by_source[source_id])
        outcomes[crosswalk["outcome"]] += 1
        if crosswalk["outcome"] != "EXACT_ONE" or crosswalk["native_key"] != allocation["native_key"]:
            raise CrosswalkError(f"PRODUCTION_CROSSWALK_NOT_EXACT_ONE:{source_id}")
        if allocation["native_key"] in native_keys:
            raise CrosswalkError(f"DUPLICATE_OUTPUT_NATIVE_KEY:{allocation['native_key']}")
        native_keys.add(allocation["native_key"])
        profile = profiles[node_to_profile[allocation["source_node_digest"]]]
        for name, state in profile["classification_capability_states"].items():
            if state["source_observation_state"] == "PRESENT_OTS_ONLY":
                classification_capability_records[name] += 1
        records.append({
            **allocation,
            "crosswalk_outcome": crosswalk["outcome"],
            "source_profile_id": node_to_profile[allocation["source_node_digest"]],
            "accepted_reference_classification": "UNKNOWN",
            "current_source_status": "NOT_EVALUATED",
        })
    if len(records) != TARGET_COUNT or outcomes != Counter({"EXACT_ONE": TARGET_COUNT}):
        raise CrosswalkError("CROSSWALK_PARTITION_FAILED")
    zero_signal_capabilities = [
        name for name in CLASSIFICATION_CAPABILITIES
        if classification_capability_records[name] == 0
    ]
    value = {
        "schema": SCHEMA,
        "compiler_profile": PROFILE,
        "classification": "OTS_HYPOTHESIS_ONLY / NO_REFERENCE_VALUE_PROMOTION",
        "authority": {"production": "NONE", "runtime": "NONE", "identity_remap": "FORBIDDEN"},
        "source_policy": {
            "issue": "Oteryn/Oteryn-Game#504",
            "comment_ids": [5771546143, 5771561550, 5773340420],
            "name_only_heuristic": "FORBIDDEN",
            "missing_means_false_or_zero": False,
            "current_source_without_input": "NOT_EVALUATED/UNKNOWN",
        },
        "protected_inputs": {name: {"path": path, "sha256": sha} for name, (path, sha) in PINNED_INPUTS.items()},
        "canonical_native_map": {
            "schema": NATIVE_MAP_SCHEMA,
            "producer": "protected_cw2_b1_full_item_family_import",
            "transport": "scratch export from protected canonical Rust importer",
            "sha256": native_map_sha256,
            "retained_bulk_corpus": False,
        },
        "allocation_digest_sha256": allocation_digest,
        "counts": {
            "records": len(records),
            "preserved_semantic_bindings": sum(r["identity_origin"] == "PRESERVED_SEMANTIC_BINDING" for r in records),
            "opaque_registry_allocations": sum(r["identity_origin"] == "OPAQUE_REGISTRY_ALLOCATION" for r in records),
            "source_profiles": len(profiles),
            "crosswalk_outcomes": {name: outcomes[name] for name in OUTCOMES},
            **profile_counts,
            **{
                f"classification_capability_present:{name}": classification_capability_records[name]
                for name in CLASSIFICATION_CAPABILITIES
            },
        },
        "capability_state_contract": {
            "families": list(CAPABILITIES),
            "source_presence_is_reference_truth": False,
            "absence_is_false_or_zero": False,
            "accepted_reference_default": "UNKNOWN",
            "current_source_default": "NOT_EVALUATED",
            "classification_capabilities": list(CLASSIFICATION_CAPABILITIES),
            "classification_source_signal_rule": "exact admitted field/value signals only; no names or absence inference",
        },
        "classification_scope": {
            "source_signal_census_complete": True,
            "accepted_reference_classification_complete": False,
            "accepted_reference_state": "UNKNOWN",
            "zero_direct_ots_signal_capabilities": zero_signal_capabilities,
            "remaining_gap": "evidence-backed accepted Reference category/capability decisions",
        },
        "source_profiles": [profiles[key] for key in sorted(profiles)],
        "records": records,
    }
    return value


def manifest(full: dict[str, Any], full_bytes: bytes, producer_path: Path) -> dict[str, Any]:
    records = full["records"]
    profiles = full["source_profiles"]
    records_bytes = canonical_bytes(records)
    profiles_bytes = canonical_bytes(profiles)
    producer = producer_path.read_bytes()
    return {
        "schema": MANIFEST_SCHEMA,
        "task": "OTV2-20260922-content-world-item-classification-crosswalk-504",
        "status": "CANDIDATE_EVIDENCE_NOT_PRODUCTION_AUTHORITY",
        "compiler": {"profile": PROFILE, "path": "tools/reference-world-corridor-census/item_classification_crosswalk.py", "sha256": digest(producer)},
        "protected_inputs": full["protected_inputs"],
        "canonical_native_map": full["canonical_native_map"],
        "full_record_output": {
            "schema": SCHEMA,
            "byte_length": len(full_bytes),
            "sha256": digest(full_bytes),
            "records_sha256": digest(records_bytes),
            "source_profiles_sha256": digest(profiles_bytes),
            "committed_bulk_corpus": False,
            "reproduction": [
                "cargo +1.94.0 run --locked -p oteryn-game-server --example export_reference_item_identity_map -- <native-map.json>",
                "python tools/reference-world-corridor-census/item_classification_crosswalk_self_test.py --native-map <native-map.json>",
                "python tools/reference-world-corridor-census/item_classification_crosswalk.py --game-root . --native-map <native-map.json> --output <full.json> --manifest-output <manifest.json>",
            ],
        },
        "counts": full["counts"],
        "classification_scope": full["classification_scope"],
        "allocation_digest_sha256": full["allocation_digest_sha256"],
        "invariants": {
            "every_native_identity_exactly_once": True,
            "duplicate_source_rejected": True,
            "missing_source_rejected": True,
            "duplicate_native_key_rejected": True,
            "allocation_remap_rejected": True,
            "name_only_resolution": False,
            "source_observations_promoted": False,
            "missing_coerced_to_false_or_zero": False,
            "current_source_status_without_input": "NOT_EVALUATED/UNKNOWN",
        },
        "limitations": [
            "No current-source catalogue was supplied or searched.",
            "No OTS observation is accepted as Reference gameplay truth.",
            "Classification and all typed capabilities remain UNKNOWN pending evidence-backed promotion.",
            "The complete 24-capability source-signal census is not an accepted Item classification result.",
            "This evidence changes no runtime, client, registry, protocol or persistence path.",
        ],
        "non_claims": ["NO_REFERENCE_PARITY", "NO_GAMEPLAY_VALUE_PROMOTION", "NO_CURRENT_SOURCE_VERIFICATION", "NO_PRODUCTION_AUTHORITY"],
    }


def write(path: Path, value: dict[str, Any]) -> bytes:
    payload = canonical_bytes(value)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)
    return payload


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path)
    parser.add_argument("--native-map", type=Path, required=True)
    args = parser.parse_args()
    root = args.game_root.resolve()
    inputs = {name: read_pinned(root, name) for name in PINNED_INPUTS}
    native_map, native_map_bytes = read_native_map(args.native_map)
    full = compile_crosswalk(inputs["b1_catalog"], inputs["native_batch"], inputs["family_registry"], inputs["schema_readiness"], native_map, digest(native_map_bytes))
    full_bytes = write(args.output, full)
    if args.manifest_output is not None:
        write(args.manifest_output, manifest(full, full_bytes, Path(__file__).resolve()))
    print(f"item-classification-crosswalk: PASS records={full['counts']['records']} profiles={full['counts']['source_profiles']} sha256={digest(full_bytes)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
