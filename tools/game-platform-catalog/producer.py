from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
import tempfile
from contextlib import contextmanager
from datetime import datetime, timezone
from itertools import pairwise
from pathlib import Path
from typing import Any

CONTRACT_ID = "oteryn.game-platform-catalog"
SCHEMA_VERSION = "1.0.0"
CONTENT_AUTHORITY_ID = "oteryn-native"
PUBLIC_VIEW_PROFILE = "oteryn:profile.public-content-view-v1"
PUBLIC_VIEW_EPOCH = "preproduction-unallocated"
KNOWN_CAPABILITIES = frozenset(
    {
        "item",
        "creature",
        "creature_loot",
        "npc",
        "npc_shop",
        "spell",
        "quest",
        "achievement",
    }
)
MAX_FILE_BYTES = 256 * 1024 * 1024
MAX_CAPABILITIES = 256
MAX_ENTITIES = 200_000
MAX_RELATIONS = 1_000_000
MAX_TOMBSTONES = 200_000
MAX_STRING_BYTES = 2_048
MAX_NESTING_DEPTH = 16
MAX_OBJECT_MEMBERS = 4_096
MAX_ARRAY_ENTRIES = 200_000
MAX_INT = (1 << 63) - 1
MAX_U32 = (1 << 32) - 1
MAX_U16 = (1 << 16) - 1

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
TOKEN_RE = re.compile(r"^[a-z0-9][a-z0-9._-]{0,127}$")
CAPABILITY_RE = re.compile(r"^[a-z][a-z0-9._-]{0,63}$")
CONTENT_KEY_RE = re.compile(r"^[a-z][a-z0-9_.-]*:[a-z][a-z0-9_.-]*$")
UTC_SECONDS_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")


class CatalogValidationError(ValueError):
    pass


_CANONICAL_ENCODER = json.JSONEncoder(
    ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
)


def _iter_canonical_json_bytes(value: Any):
    for chunk in _CANONICAL_ENCODER.iterencode(value):
        yield chunk.encode("utf-8")


def canonical_json_bytes(value: Any) -> bytes:
    return b"".join(_iter_canonical_json_bytes(value))


def _bounded_canonical_digest(value: Any, reserve_bytes: int = 0) -> str:
    digest = hashlib.sha256()
    total = 0
    for chunk in _iter_canonical_json_bytes(value):
        total += len(chunk)
        if total + reserve_bytes > MAX_FILE_BYTES:
            raise CatalogValidationError("snapshot exceeds file size limit")
        digest.update(chunk)
    return digest.hexdigest()


def _require_string(
    value: Any, name: str, pattern: re.Pattern[str] | None = None
) -> str:
    if not isinstance(value, str) or not value:
        raise CatalogValidationError(f"{name} must be a non-empty string")
    if len(value.encode("utf-8")) > MAX_STRING_BYTES:
        raise CatalogValidationError(f"{name} exceeds UTF-8 string limit")
    if pattern is not None and pattern.fullmatch(value) is None:
        raise CatalogValidationError(f"invalid {name}")
    return value


def _require_utc_seconds(value: Any) -> str:
    text = _require_string(value, "generated_at")
    if UTC_SECONDS_RE.fullmatch(text) is None:
        raise CatalogValidationError("generated_at must be RFC3339 UTC seconds")
    try:
        datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError as exc:
        raise CatalogValidationError("invalid generated_at") from exc
    return text


def _validate_json_value(value: Any, path: str, depth: int = 0) -> None:
    if depth > MAX_NESTING_DEPTH:
        raise CatalogValidationError(f"{path} exceeds nesting depth limit")
    if value is None or isinstance(value, bool):
        return
    if isinstance(value, int):
        if value < -MAX_INT - 1 or value > MAX_INT:
            raise CatalogValidationError(f"{path} integer exceeds signed 64-bit range")
        return
    if isinstance(value, float):
        raise CatalogValidationError(f"{path} floating-point values are forbidden")
    if isinstance(value, str):
        if len(value.encode("utf-8")) > MAX_STRING_BYTES:
            raise CatalogValidationError(f"{path} exceeds UTF-8 string limit")
        return
    if isinstance(value, list):
        if len(value) > MAX_ARRAY_ENTRIES:
            raise CatalogValidationError(f"{path} exceeds array entry limit")
        for index, item in enumerate(value):
            _validate_json_value(item, f"{path}[{index}]", depth + 1)
        return
    if isinstance(value, dict):
        if len(value) > MAX_OBJECT_MEMBERS:
            raise CatalogValidationError(f"{path} exceeds object member limit")
        for key, item in value.items():
            _require_string(key, f"{path} object key")
            # Owner/Platform CATALOG rule: even nested record data is public-only.
            # G-2 hidden records must be omitted by the source view, never repaired here.
            if key == "visibility" and item != "public":
                raise CatalogValidationError(f"{path}.visibility must be public")
            _validate_json_value(item, f"{path}.{key}", depth + 1)
        return
    raise CatalogValidationError(f"{path} contains unsupported JSON value type")


def _require_exact_keys(
    value: Any, name: str, required: set[str], optional: set[str] | None = None
) -> dict:
    if not isinstance(value, dict):
        raise CatalogValidationError(f"{name} must be an object")
    optional = optional or set()
    keys = set(value)
    missing = required - keys
    unknown = keys - required - optional
    if missing:
        raise CatalogValidationError(
            f"{name} missing fields: {', '.join(sorted(missing))}"
        )
    if unknown:
        raise CatalogValidationError(
            f"{name} unknown fields: {', '.join(sorted(unknown))}"
        )
    return value


def _require_list(value: Any, name: str, maximum: int) -> list:
    if not isinstance(value, list):
        raise CatalogValidationError(f"{name} must be an array")
    if len(value) > maximum:
        raise CatalogValidationError(f"{name} exceeds entry limit")
    return value


def _normalize_required_capabilities(value: Any) -> list[str]:
    values = _require_list(value, "required_capabilities", MAX_CAPABILITIES)
    result: list[str] = []
    seen: set[str] = set()
    for raw in values:
        capability = _require_string(raw, "required capability", CAPABILITY_RE)
        if capability in seen:
            raise CatalogValidationError(f"duplicate required capability: {capability}")
        seen.add(capability)
        result.append(capability)
    return sorted(result)


def _normalize_capability_manifest(value: Any) -> tuple[list[dict], dict[str, str]]:
    values = _require_list(value, "capability_manifest", MAX_CAPABILITIES)
    result: list[dict] = []
    support_by_id: dict[str, str] = {}
    for index, raw in enumerate(values):
        entry = _require_exact_keys(
            raw, f"capability_manifest[{index}]", {"capability_id", "support"}
        )
        capability = _require_string(
            entry["capability_id"], "capability_id", CAPABILITY_RE
        )
        if capability not in KNOWN_CAPABILITIES:
            raise CatalogValidationError(f"unknown v1 capability: {capability}")
        support = _require_string(entry["support"], "capability support")
        if support not in {"supported", "unsupported"}:
            raise CatalogValidationError(f"invalid capability support: {support}")
        if capability in support_by_id:
            raise CatalogValidationError(f"duplicate capability: {capability}")
        support_by_id[capability] = support
        result.append({"capability_id": capability, "support": support})
    result.sort(key=lambda item: item["capability_id"])
    return result, support_by_id


def _normalize_completeness_manifest(value: Any) -> tuple[list[dict], dict[str, str]]:
    values = _require_list(value, "completeness_manifest", MAX_CAPABILITIES)
    result: list[dict] = []
    state_by_id: dict[str, str] = {}
    for index, raw in enumerate(values):
        entry = _require_exact_keys(
            raw, f"completeness_manifest[{index}]", {"capability_id", "state"}
        )
        capability = _require_string(
            entry["capability_id"], "capability_id", CAPABILITY_RE
        )
        state = _require_string(entry["state"], "completeness state")
        if state not in {"complete", "partial", "unknown"}:
            raise CatalogValidationError(f"invalid completeness state: {state}")
        if capability in state_by_id:
            raise CatalogValidationError(
                f"duplicate completeness capability: {capability}"
            )
        state_by_id[capability] = state
        result.append({"capability_id": capability, "state": state})
    result.sort(key=lambda item: item["capability_id"])
    return result, state_by_id


def _validate_manifest_alignment(
    support: dict[str, str], completeness: dict[str, str]
) -> None:
    if set(support) != set(completeness):
        raise CatalogValidationError(
            "capability and completeness manifests must cover identical capability IDs"
        )
    for capability, support_state in support.items():
        if support_state == "unsupported" and completeness[capability] != "unknown":
            raise CatalogValidationError(
                f"unsupported capability {capability} must have unknown completeness"
            )


def _require_supported_capability(
    capability: str, support: dict[str, str], context: str
) -> None:
    if capability not in support:
        raise CatalogValidationError(
            f"{context} references undeclared capability: {capability}"
        )
    if support[capability] != "supported":
        raise CatalogValidationError(
            f"{context} references unsupported capability: {capability}"
        )


def _normalize_entities(
    value: Any, support: dict[str, str]
) -> tuple[list[dict], set[str]]:
    values = _require_list(value, "entities", MAX_ENTITIES)
    result: list[dict] = []
    keys: set[str] = set()
    required = {"type", "content_key", "capability_id", "data"}
    for index, raw in enumerate(values):
        entry = _require_exact_keys(raw, f"entities[{index}]", required)
        entity_type = _require_string(entry["type"], "entity type", CAPABILITY_RE)
        content_key = _require_string(
            entry["content_key"], "content_key", CONTENT_KEY_RE
        )
        capability = _require_string(
            entry["capability_id"], "capability_id", CAPABILITY_RE
        )
        _require_supported_capability(capability, support, f"entity {content_key}")
        if content_key in keys:
            raise CatalogValidationError(f"duplicate entity: {content_key}")
        data = entry["data"]
        if not isinstance(data, dict):
            raise CatalogValidationError(f"entity {content_key} data must be an object")
        _validate_json_value(data, f"entity {content_key}.data")
        keys.add(content_key)
        result.append(
            {
                "type": entity_type,
                "content_key": content_key,
                "capability_id": capability,
                "data": data,
            }
        )
    result.sort(key=lambda item: (item["type"], item["content_key"]))
    return result, keys


def _normalize_relations(
    value: Any, support: dict[str, str], entity_keys: set[str]
) -> list[dict]:
    values = _require_list(value, "relations", MAX_RELATIONS)
    result: list[dict] = []
    relation_keys: set[str] = set()
    required = {"type", "relation_key", "capability_id", "source", "target", "data"}
    for index, raw in enumerate(values):
        entry = _require_exact_keys(raw, f"relations[{index}]", required)
        relation_type = _require_string(entry["type"], "relation type", CAPABILITY_RE)
        relation_key = _require_string(
            entry["relation_key"], "relation_key", CONTENT_KEY_RE
        )
        capability = _require_string(
            entry["capability_id"], "capability_id", CAPABILITY_RE
        )
        _require_supported_capability(capability, support, f"relation {relation_key}")
        if relation_key in relation_keys:
            raise CatalogValidationError(f"duplicate relation: {relation_key}")
        source_key = _require_string(entry["source"], "relation source", CONTENT_KEY_RE)
        if source_key not in entity_keys:
            raise CatalogValidationError(f"dangling relation source: {source_key}")
        target_raw = entry["target"]
        target_key = (
            None
            if target_raw is None
            else _require_string(target_raw, "relation target", CONTENT_KEY_RE)
        )
        if target_key is not None and target_key not in entity_keys:
            raise CatalogValidationError(f"dangling relation target: {target_key}")
        data = entry["data"]
        if not isinstance(data, dict):
            raise CatalogValidationError(
                f"relation {relation_key} data must be an object"
            )
        _validate_json_value(data, f"relation {relation_key}.data")
        relation_keys.add(relation_key)
        result.append(
            {
                "type": relation_type,
                "relation_key": relation_key,
                "capability_id": capability,
                "source": source_key,
                "target": target_key,
                "data": data,
            }
        )
    result.sort(key=lambda item: (item["type"], item["relation_key"]))
    return result


def _normalize_tombstones(
    value: Any,
    support: dict[str, str],
    completeness: dict[str, str],
    entity_keys: set[str],
) -> list[dict]:
    values = _require_list(value, "tombstones", MAX_TOMBSTONES)
    result: list[dict] = []
    tombstone_keys: set[str] = set()
    required = {"content_key", "capability_id", "reason"}
    for index, raw in enumerate(values):
        entry = _require_exact_keys(raw, f"tombstones[{index}]", required)
        content_key = _require_string(
            entry["content_key"], "content_key", CONTENT_KEY_RE
        )
        capability = _require_string(
            entry["capability_id"], "capability_id", CAPABILITY_RE
        )
        _require_supported_capability(capability, support, f"tombstone {content_key}")
        if completeness[capability] != "complete":
            raise CatalogValidationError(
                f"tombstone requires complete capability coverage: {capability}"
            )
        if content_key in entity_keys:
            raise CatalogValidationError(f"contradictory tombstone: {content_key}")
        if content_key in tombstone_keys:
            raise CatalogValidationError(f"duplicate tombstone: {content_key}")
        reason = _require_string(entry["reason"], "tombstone reason")
        tombstone_keys.add(content_key)
        result.append(
            {"content_key": content_key, "capability_id": capability, "reason": reason}
        )
    result.sort(key=lambda item: item["content_key"])
    return result


def _public_int(
    value: Any, name: str, minimum: int = 0, maximum: int = MAX_INT
) -> None:
    if type(value) is not int or not minimum <= value <= maximum:
        raise CatalogValidationError(f"invalid public payload {name}")


def _public_revision(value: Any) -> None:
    revision = _require_string(value, "definition_revision")
    lower = revision.lower()
    if (
        re.fullmatch(r"[!-~]{1,512}", revision) is None
        or any(
            marker in lower
            for marker in ("fixture", "synthetic", "evidence", "test-only")
        )
        or "test" in re.split(r"[:./_-]", lower)
    ):
        raise CatalogValidationError("invalid public payload definition_revision")


def _public_source_text(value: Any, name: str) -> str:
    text = _require_string(value, name)
    if text.strip() != text or re.search(r"[\x00-\x1f\x7f-\x9f]", text):
        raise CatalogValidationError(f"invalid public payload {name}")
    return text


def _require_increasing(values: list[str], name: str) -> None:
    if any(left >= right for left, right in pairwise(values)):
        raise CatalogValidationError(f"public {name} must be sorted and unique")


def _validate_public_view_payloads(
    entities: list[dict],
    relations: list[dict],
    tombstones: list[dict],
    support: dict[str, str],
    completeness: dict[str, str],
) -> None:
    # These checks apply to the Game-owned public projection profile. Other v1
    # profiles retain the generic bounded-carrier validation.
    observed = {row["capability_id"] for row in entities + relations}
    if observed != {cap for cap, state in support.items() if state == "supported"}:
        raise CatalogValidationError(
            "public view support must match emitted capabilities"
        )
    if set(support) != KNOWN_CAPABILITIES or any(
        completeness[cap] == "unknown" for cap in observed
    ):
        raise CatalogValidationError(
            "public view requires coverage states for all v1 capabilities"
        )
    if tombstones:
        raise CatalogValidationError("public view has no native tombstone source")

    core = {"payload_revision", "definition_revision"}
    for row in entities:
        cap, data = row["capability_id"], row["data"]
        if row["type"] != cap or cap not in {"item", "creature", "npc"}:
            raise CatalogValidationError("invalid public entity type or capability")
        if cap == "item":
            _require_exact_keys(
                data, "item data", core | {"materializable", "stack_class"}, {"name"}
            )
            if type(data["materializable"]) is not bool or data["stack_class"] not in (
                "Unknown",
                "StackCapable",
                "NonStackable",
            ):
                raise CatalogValidationError("invalid public item payload")
            if data["materializable"] and data["stack_class"] == "Unknown":
                raise CatalogValidationError(
                    "public identity-only item cannot be materializable"
                )
            if (
                "name" in data
                and len(_require_string(data["name"], "name").encode("utf-8")) > 46
            ):
                raise CatalogValidationError("public item name exceeds 46 UTF-8 bytes")
        elif cap == "creature":
            stats = {"health", "experience", "speed", "armor"}
            _require_exact_keys(
                data,
                "creature data",
                core,
                stats | {"name", "resistances", "immunities"},
            )
            for field in stats & data.keys():
                _public_int(
                    data[field],
                    field,
                    minimum=int(field == "health"),
                    maximum=MAX_U32 if field in {"speed", "armor"} else MAX_INT,
                )
            if "name" in data:
                _public_source_text(data["name"], "name")
            damage_types = []
            for entry in _require_list(
                data.get("resistances", []), "resistances", MAX_ARRAY_ENTRIES
            ):
                _require_exact_keys(entry, "resistance", {"damage_type", "percent"})
                damage_types.append(
                    _public_source_text(entry["damage_type"], "resistance damage_type")
                )
                percent = _require_exact_keys(
                    entry["percent"], "percent", {"numerator", "denominator"}
                )
                _public_int(percent["numerator"], "numerator", -MAX_INT - 1)
                _public_int(percent["denominator"], "denominator", 1)
                if math.gcd(percent["numerator"], percent["denominator"]) != 1:
                    raise CatalogValidationError(
                        "public resistances must use reduced ratios"
                    )
            _require_increasing(damage_types, "resistances")
            immunities = _require_list(
                data.get("immunities", []), "immunities", MAX_ARRAY_ENTRIES
            )
            _require_increasing(
                [_public_source_text(value, "immunity") for value in immunities],
                "immunities",
            )
        else:
            _require_exact_keys(data, "npc data", core)
        _public_revision(data["definition_revision"])
        _public_int(data["payload_revision"], "payload_revision", 1, 1)

    entity_kinds = {row["content_key"]: row["capability_id"] for row in entities}
    for row in relations:
        cap, data = row["capability_id"], row["data"]
        source_kind = {"creature_loot": "creature", "npc_shop": "npc"}.get(cap)
        if (
            row["type"] != cap
            or source_kind is None
            or entity_kinds[row["source"]] != source_kind
            or entity_kinds.get(row["target"]) != "item"
        ):
            raise CatalogValidationError(
                "invalid public relation capability or endpoint type"
            )
        kind = ".loot." if cap == "creature_loot" else ".row."
        prefix, _, ordinal = row["relation_key"].rpartition(kind)
        if cap == "creature_loot":
            matches_source = prefix == row["source"]
        else:
            shop_prefix = row["source"] + ".shop."
            # The carrier has already checked the key alphabet. An encoded
            # Service key must have a separator with a part on each side.
            service_key = prefix.removeprefix(shop_prefix)
            matches_source = prefix.startswith(shop_prefix) and "." in service_key[1:-1]
        if not matches_source or re.fullmatch(r"0|[1-9][0-9]*", ordinal) is None:
            raise CatalogValidationError(
                "public relation_key must identify a source row and ordinal"
            )
        if cap == "creature_loot":
            _require_exact_keys(
                data,
                "loot data",
                {
                    "payload_revision",
                    "algorithm",
                    "min_count",
                    "max_count",
                    "probability_ppm",
                },
            )
            if data["algorithm"] != "IndependentBernoulliPpm":
                raise CatalogValidationError("invalid public loot algorithm")
            _public_int(data["min_count"], "min_count", 1, MAX_U32)
            _public_int(data["max_count"], "max_count", data["min_count"], MAX_U32)
            _public_int(data["probability_ppm"], "probability_ppm", 0, 1_000_000)
        else:
            _require_exact_keys(
                data,
                "shop data",
                {
                    "payload_revision",
                    "direction",
                    "unit_price",
                    "count",
                    "sub_type",
                    "currency",
                },
            )
            if data["direction"] not in ("BuyFromPlayer", "SellToPlayer"):
                raise CatalogValidationError("invalid public shop direction")
            _public_int(data["unit_price"], "unit_price")
            if data["count"] is not None:
                _public_int(data["count"], "count", 1, MAX_U32)
            if data["sub_type"] is not None:
                _public_int(data["sub_type"], "sub_type", 0, MAX_U16)
            if data["currency"] is not None:
                currency = _require_string(data["currency"], "currency", CONTENT_KEY_RE)
                if entity_kinds.get(currency) != "item":
                    raise CatalogValidationError(
                        "public shop currency must reference an exported item"
                    )
        _public_int(data["payload_revision"], "payload_revision", 1, 1)


def _normalize_source(source: Any) -> tuple[dict, str]:
    required = {
        "authority_epoch",
        "source_revision",
        "generated_at",
        "ruleset_id",
        "content_profile_id",
        "required_capabilities",
        "capability_manifest",
        "completeness_manifest",
        "entities",
        "relations",
        "tombstones",
    }
    source = _require_exact_keys(source, "source", required)
    authority_epoch = _require_string(
        source["authority_epoch"], "authority_epoch", TOKEN_RE
    )
    source_revision = _require_string(
        source["source_revision"], "source_revision", SHA_RE
    )
    generated_at = _require_utc_seconds(source["generated_at"])
    ruleset_id = _require_string(source["ruleset_id"], "ruleset_id", CONTENT_KEY_RE)
    content_profile_id = _require_string(
        source["content_profile_id"], "content_profile_id", CONTENT_KEY_RE
    )
    required_capabilities = _normalize_required_capabilities(
        source["required_capabilities"]
    )
    capability_manifest, support = _normalize_capability_manifest(
        source["capability_manifest"]
    )
    completeness_manifest, completeness = _normalize_completeness_manifest(
        source["completeness_manifest"]
    )
    _validate_manifest_alignment(support, completeness)
    for capability in required_capabilities:
        if support.get(capability) != "supported":
            raise CatalogValidationError(
                f"required capability {capability} is unsupported"
            )

    entities, entity_keys = _normalize_entities(source["entities"], support)
    relations = _normalize_relations(source["relations"], support, entity_keys)
    tombstones = _normalize_tombstones(
        source["tombstones"], support, completeness, entity_keys
    )
    if content_profile_id == PUBLIC_VIEW_PROFILE:
        if authority_epoch != PUBLIC_VIEW_EPOCH:
            raise CatalogValidationError(
                f"public view authority_epoch must be {PUBLIC_VIEW_EPOCH}"
            )
        _validate_public_view_payloads(
            entities, relations, tombstones, support, completeness
        )
    semantic = {
        "contract_id": CONTRACT_ID,
        "schema_version": SCHEMA_VERSION,
        "content_authority_id": CONTENT_AUTHORITY_ID,
        "authority_epoch": authority_epoch,
        "source_revision": source_revision,
        "ruleset_id": ruleset_id,
        "content_profile_id": content_profile_id,
        "required_capabilities": required_capabilities,
        "capability_manifest": capability_manifest,
        "completeness_manifest": completeness_manifest,
        "entities": entities,
        "relations": relations,
        "tombstones": tombstones,
    }
    return semantic, generated_at


def build_snapshot(source: Any) -> dict:
    semantic, generated_at = _normalize_source(source)
    integrity_payload = dict(semantic)
    integrity_payload["generated_at"] = generated_at
    payload_hex = _bounded_canonical_digest(integrity_payload)
    digest = f"sha256:{payload_hex}"
    snapshot = dict(integrity_payload)
    snapshot["snapshot_id"] = digest
    snapshot["payload_digest"] = digest
    _bounded_canonical_digest(snapshot, reserve_bytes=1)
    return snapshot


def verify_snapshot(snapshot: Any) -> None:
    required = {
        "contract_id",
        "schema_version",
        "snapshot_id",
        "content_authority_id",
        "authority_epoch",
        "source_revision",
        "generated_at",
        "ruleset_id",
        "content_profile_id",
        "required_capabilities",
        "capability_manifest",
        "completeness_manifest",
        "entities",
        "relations",
        "tombstones",
        "payload_digest",
    }
    snapshot = _require_exact_keys(snapshot, "snapshot", required)
    if snapshot["contract_id"] != CONTRACT_ID:
        raise CatalogValidationError("unsupported contract_id")
    if snapshot["schema_version"] != SCHEMA_VERSION:
        raise CatalogValidationError("unsupported schema_version")
    if snapshot["content_authority_id"] != CONTENT_AUTHORITY_ID:
        raise CatalogValidationError("unsupported content_authority_id")
    payload_digest = _require_string(
        snapshot["payload_digest"], "payload_digest", DIGEST_RE
    )
    snapshot_id = _require_string(snapshot["snapshot_id"], "snapshot_id", DIGEST_RE)

    source_keys = required - {
        "contract_id",
        "schema_version",
        "snapshot_id",
        "content_authority_id",
        "payload_digest",
    }
    source = {key: snapshot[key] for key in source_keys}
    semantic, generated_at = _normalize_source(source)
    if snapshot["generated_at"] != generated_at:
        raise CatalogValidationError("generated_at normalization mismatch")
    for key, value in semantic.items():
        if snapshot[key] != value:
            raise CatalogValidationError(f"snapshot is not canonical at {key}")
    integrity_payload = dict(semantic)
    integrity_payload["generated_at"] = generated_at
    expected = "sha256:" + _bounded_canonical_digest(integrity_payload)
    if payload_digest != expected:
        raise CatalogValidationError("payload_digest mismatch")
    if snapshot_id != expected:
        raise CatalogValidationError("snapshot_id mismatch")


def _reject_duplicate_object_pairs(pairs: list[tuple[str, Any]]) -> dict:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CatalogValidationError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def load_json_file(path: Path) -> Any:
    try:
        size = path.stat().st_size
    except OSError as exc:
        raise CatalogValidationError(f"cannot stat input file: {path}") from exc
    if size > MAX_FILE_BYTES:
        raise CatalogValidationError("input exceeds file size limit")
    try:
        raw = path.read_bytes()
        text = raw.decode("utf-8", errors="strict")
        return json.loads(text, object_pairs_hook=_reject_duplicate_object_pairs)
    except UnicodeDecodeError as exc:
        raise CatalogValidationError("input is not valid UTF-8") from exc
    except json.JSONDecodeError as exc:
        raise CatalogValidationError(f"input is not valid JSON: {exc.msg}") from exc


def _stage_snapshot(path: Path, snapshot: dict) -> str:
    digest = hashlib.sha256()
    total = 0
    with path.open("wb") as handle:
        for chunk in _iter_canonical_json_bytes(snapshot):
            total += len(chunk)
            if total + 1 > MAX_FILE_BYTES:
                raise CatalogValidationError("snapshot exceeds file size limit")
            handle.write(chunk)
            digest.update(chunk)
        handle.write(b"\n")
        digest.update(b"\n")
    return digest.hexdigest()


@contextmanager
def _publication_lock(directory: Path):
    if os.name == "nt":
        import msvcrt

        # Keep the lock file so overlapping writers always lock the same file.
        with (directory / ".catalogue.lock").open("a+b") as lock:
            msvcrt.locking(lock.fileno(), msvcrt.LK_LOCK, 1)
            yield
        return

    import fcntl

    parent = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
    try:
        fcntl.flock(parent, fcntl.LOCK_EX)
        yield
    finally:
        os.close(parent)


def write_snapshot_files(snapshot: dict, output: Path) -> None:
    verify_snapshot(snapshot)
    if not output.parent.exists():
        raise CatalogValidationError(
            f"output directory does not exist: {output.parent}"
        )
    try:
        with tempfile.TemporaryDirectory(
            prefix=".catalogue-", dir=output.parent
        ) as staging:
            artifact = Path(staging) / "snapshot"
            digest = Path(staging) / "digest"
            artifact_hex = _stage_snapshot(artifact, snapshot)
            digest.write_bytes((artifact_hex + "\n").encode("ascii"))
            with _publication_lock(output.parent):
                artifact.replace(output)
                digest.replace(output.with_suffix(output.suffix + ".sha256"))
    except OSError as exc:
        raise CatalogValidationError(f"cannot write output file: {output}") from exc


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Build or verify native Game -> Platform catalog snapshots"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    produce = subparsers.add_parser(
        "produce", help="validate normalized native input and emit a snapshot"
    )
    produce.add_argument("source", type=Path)
    produce.add_argument("output", type=Path)

    verify = subparsers.add_parser(
        "verify", help="verify a previously emitted snapshot"
    )
    verify.add_argument("snapshot", type=Path)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        if args.command == "produce":
            snapshot = build_snapshot(load_json_file(args.source))
            write_snapshot_files(snapshot, args.output)
            print(snapshot["payload_digest"])
            return 0
        snapshot = load_json_file(args.snapshot)
        verify_snapshot(snapshot)
        print(snapshot["payload_digest"])
        return 0
    except CatalogValidationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    import sys

    raise SystemExit(main())
