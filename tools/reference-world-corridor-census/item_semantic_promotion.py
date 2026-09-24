#!/usr/bin/env python3
"""Compile the exact eligible Item semantic-promotion packet from target continuity.

This consumes the existing #774 continuity output. It does not fetch sources, resolve
identity, or define a second Item model/rule engine. Mutable wiki revision metadata is
intentionally excluded from the stable promotion packet.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from collections import Counter
from pathlib import Path
from typing import Any

TARGET_DATE = "2026-07-28"
CONTINUITY_SCHEMA = "OTERYN_ITEM_TARGET_CONTINUITY/v1"
CONTINUITY_PROFILE = "OTERYN_ITEM_TARGET_CONTINUITY_BRIDGE/v1"
PROTECTED_MANIFEST_SCHEMA = "OTERYN_ITEM_TARGET_CONTINUITY_MANIFEST/v1"
PROTECTED_CONTINUITY_COMPILER_SHA256 = (
    "3a3bd72f4c04c591c86764d7e2656d90bee051145a951de2848cef0a89d1f1d2"
)
PROMOTION_SCHEMA = "OTERYN_ITEM_SEMANTIC_PROMOTION/v1"
PROMOTION_PROFILE = "OTERYN_ITEM_SEMANTIC_PROMOTION_COMPILER/v1"

EXPECTED_PER_FIELD = {
    "charges.count": 1,
    "container.capacity": 1,
    "presentation.name": 22,
    "protection.armor": 3,
    "weapon.attack": 16,
    "weapon.defense": 13,
    "weapon.extra_defense": 7,
    "weapon.hit_chance": 2,
    "weapon.range_cells": 4,
}
EXPECTED_PROMOTED_FIELDS = sum(EXPECTED_PER_FIELD.values())
EXPECTED_CANDIDATE_PAGES = 23

SIGNED_POINT_FIELDS = {
    "weapon.attack",
    "weapon.defense",
    "weapon.extra_defense",
    "protection.armor",
}


class PromotionError(RuntimeError):
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
        raise PromotionError(f"JSON_INVALID:{path}") from exc
    if not isinstance(value, dict):
        raise PromotionError(f"JSON_ROOT_NOT_OBJECT:{path}")
    return value, payload


def require_int(value: Any, *, label: str, minimum: int, maximum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise PromotionError(f"{label}_NOT_INT")
    if value < minimum or value > maximum:
        raise PromotionError(f"{label}_OUT_OF_RANGE:{value}")
    return value


def typed_value(field_path: str, value: Any) -> dict[str, Any]:
    if field_path == "presentation.name":
        if not isinstance(value, str) or not value.strip():
            raise PromotionError("PRESENTATION_NAME_INVALID")
        if len(value.encode("utf-8")) > 2048:
            raise PromotionError("PRESENTATION_NAME_TOO_LARGE")
        return {"kind": "TEXT", "value": value}

    if field_path in SIGNED_POINT_FIELDS:
        return {
            "kind": "SIGNED_POINTS",
            "value": require_int(
                value,
                label=field_path.upper().replace(".", "_"),
                minimum=-(2**31),
                maximum=2**31 - 1,
            ),
        }

    if field_path == "weapon.range_cells":
        return {
            "kind": "CELLS",
            "value": require_int(
                value, label="WEAPON_RANGE_CELLS", minimum=0, maximum=2**16 - 1
            ),
        }

    if field_path == "weapon.hit_chance":
        percent_points = require_int(
            value,
            label="WEAPON_HIT_CHANCE_PERCENT_POINTS",
            minimum=-(2**31),
            maximum=2**31 - 1,
        )
        divisor = math.gcd(abs(percent_points), 100)
        return {
            "kind": "RATIONAL_PERCENT",
            "numerator": percent_points // divisor,
            "denominator": 100 // divisor,
            "source_unit": "PERCENT_POINTS",
        }

    if field_path == "charges.count":
        return {
            "kind": "COUNT_U32",
            "value": require_int(
                value, label="CHARGES_COUNT", minimum=0, maximum=2**32 - 1
            ),
        }

    if field_path == "container.capacity":
        return {
            "kind": "CAPACITY_U16",
            "value": require_int(
                value, label="CONTAINER_CAPACITY", minimum=0, maximum=2**16 - 1
            ),
        }

    raise PromotionError(f"UNSUPPORTED_PROMOTION_FIELD:{field_path}")


def validate_protected_manifest(manifest: dict[str, Any]) -> None:
    if manifest.get("schema") != PROTECTED_MANIFEST_SCHEMA:
        raise PromotionError("PROTECTED_MANIFEST_SCHEMA_MISMATCH")
    if manifest.get("target_date") != TARGET_DATE:
        raise PromotionError("PROTECTED_MANIFEST_TARGET_MISMATCH")
    compiler = manifest.get("compiler")
    if (
        not isinstance(compiler, dict)
        or compiler.get("sha256") != PROTECTED_CONTINUITY_COMPILER_SHA256
    ):
        raise PromotionError("PROTECTED_CONTINUITY_COMPILER_MISMATCH")
    counts = manifest.get("counts")
    if not isinstance(counts, dict):
        raise PromotionError("PROTECTED_MANIFEST_COUNTS_INVALID")
    if counts.get("candidate_fields") != EXPECTED_PROMOTED_FIELDS:
        raise PromotionError("PROTECTED_CANDIDATE_FIELD_COUNT_MISMATCH")
    if counts.get("candidate_pages") != EXPECTED_CANDIDATE_PAGES:
        raise PromotionError("PROTECTED_CANDIDATE_PAGE_COUNT_MISMATCH")
    if counts.get("continuity") != {
        "CONFLICT": 0,
        "DERIVED": EXPECTED_PROMOTED_FIELDS,
        "UNKNOWN": 0,
    }:
        raise PromotionError("PROTECTED_CONTINUITY_PARTITION_MISMATCH")
    if counts.get("per_field") != {
        f"{field}:DERIVED": count for field, count in sorted(EXPECTED_PER_FIELD.items())
    }:
        raise PromotionError("PROTECTED_PER_FIELD_PARTITION_MISMATCH")
    invariants = manifest.get("invariants")
    if (
        not isinstance(invariants, dict)
        or invariants.get("proven_continuity_emitted") is not False
        or invariants.get("semantic_promotion_performed") is not False
    ):
        raise PromotionError("PROTECTED_MANIFEST_INVARIANT_MISMATCH")


def compile_promotion(
    continuity: dict[str, Any],
    protected_manifest: dict[str, Any],
    *,
    protected_manifest_sha256: str,
    compiler_sha256: str,
) -> dict[str, Any]:
    validate_protected_manifest(protected_manifest)

    if continuity.get("schema") != CONTINUITY_SCHEMA:
        raise PromotionError("CONTINUITY_SCHEMA_MISMATCH")
    if continuity.get("profile") != CONTINUITY_PROFILE:
        raise PromotionError("CONTINUITY_PROFILE_MISMATCH")
    if continuity.get("target_date") != TARGET_DATE:
        raise PromotionError("CONTINUITY_TARGET_MISMATCH")

    counts = continuity.get("counts")
    if not isinstance(counts, dict):
        raise PromotionError("CONTINUITY_COUNTS_INVALID")
    if counts.get("candidate_fields") != EXPECTED_PROMOTED_FIELDS:
        raise PromotionError("CONTINUITY_FIELD_COUNT_MISMATCH")
    if counts.get("candidate_pages") != EXPECTED_CANDIDATE_PAGES:
        raise PromotionError("CONTINUITY_PAGE_COUNT_MISMATCH")
    if counts.get("continuity") != {
        "DERIVED": EXPECTED_PROMOTED_FIELDS,
        "UNKNOWN": 0,
        "CONFLICT": 0,
    }:
        raise PromotionError("CONTINUITY_PARTITION_MISMATCH")

    invariants = continuity.get("invariants")
    if (
        not isinstance(invariants, dict)
        or invariants.get("only_corrob_current_examined") is not True
        or invariants.get("name_only_identity_resolution") is not False
        or invariants.get("proven_continuity_emitted") is not False
        or invariants.get("semantic_promotion_performed") is not False
    ):
        raise PromotionError("CONTINUITY_INVARIANT_MISMATCH")

    records = continuity.get("records")
    if not isinstance(records, list) or len(records) != EXPECTED_PROMOTED_FIELDS:
        raise PromotionError("CONTINUITY_RECORD_COUNT_MISMATCH")

    promotions: list[dict[str, Any]] = []
    seen_fields: set[tuple[str, str]] = set()
    native_to_source: dict[str, int] = {}
    source_to_native: dict[int, str] = {}
    per_field: Counter[str] = Counter()

    for record in records:
        if not isinstance(record, dict):
            raise PromotionError("CONTINUITY_RECORD_NOT_OBJECT")
        if record.get("continuity_to_target") != "DERIVED":
            raise PromotionError("NON_DERIVED_RECORD_NOT_PROMOTABLE")
        if (
            record.get("promotion_bridge")
            != "ELIGIBLE_FOR_SEMANTIC_PROMOTION_GENERATION"
        ):
            raise PromotionError("PROMOTION_BRIDGE_NOT_ELIGIBLE")

        source_item_id = record.get("source_item_id")
        native_key = record.get("native_key")
        field_path = record.get("field_path")
        if (
            isinstance(source_item_id, bool)
            or not isinstance(source_item_id, int)
            or source_item_id <= 0
            or not isinstance(native_key, str)
            or not native_key
            or not isinstance(field_path, str)
            or field_path not in EXPECTED_PER_FIELD
        ):
            raise PromotionError("PROMOTION_IDENTITY_OR_FIELD_INVALID")

        prior_source = native_to_source.setdefault(native_key, source_item_id)
        prior_native = source_to_native.setdefault(source_item_id, native_key)
        if prior_source != source_item_id or prior_native != native_key:
            raise PromotionError("PROMOTION_IDENTITY_BINDING_CONFLICT")

        key = (native_key, field_path)
        if key in seen_fields:
            raise PromotionError("PROMOTION_FIELD_DUPLICATE")
        seen_fields.add(key)
        per_field[field_path] += 1

        source_value = record.get("current_value")
        promotions.append(
            {
                "source_item_id": source_item_id,
                "native_key": native_key,
                "field_path": field_path,
                "source_value": source_value,
                "typed_value": typed_value(field_path, source_value),
            }
        )

    if dict(sorted(per_field.items())) != EXPECTED_PER_FIELD:
        raise PromotionError("PROMOTION_PER_FIELD_PARTITION_MISMATCH")

    promotions.sort(
        key=lambda row: (row["native_key"], row["field_path"], row["source_item_id"])
    )
    return {
        "schema": PROMOTION_SCHEMA,
        "profile": PROMOTION_PROFILE,
        "target_date": TARGET_DATE,
        "status": "PARTIAL_CANONICAL_SEMANTIC_PROMOTION_PACKET",
        "compiler": {
            "path": "tools/reference-world-corridor-census/item_semantic_promotion.py",
            "sha256": compiler_sha256,
        },
        "protected_lineage": {
            "continuity_manifest_schema": PROTECTED_MANIFEST_SCHEMA,
            "continuity_manifest_sha256": protected_manifest_sha256,
            "continuity_compiler_sha256": PROTECTED_CONTINUITY_COMPILER_SHA256,
            "eligible_field_count": EXPECTED_PROMOTED_FIELDS,
        },
        "counts": {
            "promoted_fields": len(promotions),
            "promoted_items": len(native_to_source),
            "per_field": dict(sorted(per_field.items())),
        },
        "promotions": promotions,
        "invariants": {
            "identity_reminted": False,
            "name_only_identity_resolution": False,
            "whole_item_promotion": False,
            "mutable_wiki_revision_metadata_retained": False,
            "only_derived_eligible_fields_promoted": True,
            "sibling_fields_default_unknown_or_existing_state": True,
        },
        "next_action": "APPLY_TO_EXISTING_REFERENCE_ITEM_SEMANTICS_AND_COMPILE_ARTIFACT_V4",
    }


def compile_files(args: argparse.Namespace) -> None:
    continuity, _ = load_json(args.continuity)
    protected_manifest, protected_manifest_bytes = load_json(args.protected_manifest)
    compiler_sha = sha256_bytes(Path(__file__).read_bytes().replace(b"\r\n", b"\n"))
    result = compile_promotion(
        continuity,
        protected_manifest,
        protected_manifest_sha256=sha256_bytes(protected_manifest_bytes),
        compiler_sha256=compiler_sha,
    )
    payload = canonical_bytes(result)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(payload)
    print(
        "item-semantic-promotion: PASS "
        f"fields={result['counts']['promoted_fields']} "
        f"items={result['counts']['promoted_items']} "
        f"digest={sha256_bytes(payload)}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--continuity", type=Path, required=True)
    parser.add_argument("--protected-manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    compile_files(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
