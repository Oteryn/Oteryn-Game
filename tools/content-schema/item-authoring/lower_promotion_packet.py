"""Lower this package's converted, zero-validator-error Crystal Item bundles into
candidate rows for the Item semantic-promotion packet
`apps/game-server/src/content/cw2_b1_import.rs` decodes
(`OTERYN_ITEM_SEMANTIC_PROMOTION/v1`,
`docs/agents/evidence/OTV2-20260923-content-world-item-semantic-promotion.json`).

This is a v1 lowering *candidate*, not the wired packet. It walks the whole pinned
Crystal population the same way `population_census.py` does (`engine_items.
load_engine_sources`/`convert_item`), keeps only Items whose bundle validates with
zero `validate_item.validate` errors, and re-encodes their already-authored values for
the same 9 field paths that Rust decoder accepts today: `presentation.name`,
`weapon.attack`/`defense`/`extra_defense`/`range_cells`/`hit_chance`,
`protection.armor`, `charges.count`, `container.capacity`. `typed_value` below mirrors
`decode_item_semantic_promotion_value`'s exact accepted shape and bounds field by
field; `validate_row`/`validate_packet` mirror the same Rust function (plus the row
ordering/uniqueness/partition checks `protected_cw2_b1_promoted_item_family_import`
makes) in the opposite, checking direction, and run against every row this script
emits before it is written.

`schema`/`profile`/`status`/`next_action` are deliberately different literal strings
from the pinned Rust constants (`ITEM_SEMANTIC_PROMOTION_SCHEMA`/`_PROFILE`/`_STATUS`
and the `next_action` `validate_item_semantic_promotion_packet` requires), so this
candidate packet can never be mistaken for, or silently accepted as, the wired one.
Wiring it in — pinned constants, a bespoke apply function mirroring
`apply_item_semantic_promotion`, and a Rust integration test, per the existing
packet's own pattern — is left to the Content/World import role; this script and its
committed output make no Rust-side claim.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from collections import Counter
from pathlib import Path

import validate_item
from engine_items import convert_item, load_engine_sources

ROOT = Path(__file__).resolve().parent
DEFAULT_OUTPUT = ROOT / "samples" / "promotion-crystal-ff7ede5.json"
COMPILER_PATH = "tools/content-schema/item-authoring/lower_promotion_packet.py"

SCHEMA = "OTERYN_ITEM_SEMANTIC_PROMOTION_LOWERING/v1"
PROFILE = "OTERYN_ITEM_SEMANTIC_PROMOTION_LOWERING_COMPILER/v1"
STATUS = "CANDIDATE_ITEM_SEMANTIC_PROMOTION_LOWERING_PACKET"
NEXT_ACTION = "REVIEW_AND_WIRE_INTO_CW2_B1_IMPORT_RUST_DECODER_IF_ACCEPTED"
TARGET_DATE = "2026-09-27"

SIGNED_POINTS_FIELDS = (
    "weapon.attack",
    "weapon.defense",
    "weapon.extra_defense",
    "protection.armor",
)
FIELD_KIND = {
    "presentation.name": "TEXT",
    "weapon.attack": "SIGNED_POINTS",
    "weapon.defense": "SIGNED_POINTS",
    "weapon.extra_defense": "SIGNED_POINTS",
    "protection.armor": "SIGNED_POINTS",
    "weapon.range_cells": "CELLS",
    "weapon.hit_chance": "RATIONAL_PERCENT",
    "charges.count": "COUNT_U32",
    "container.capacity": "CAPACITY_U16",
}
TYPED_VALUE_KEYS = {
    "TEXT": ("kind", "value"),
    "SIGNED_POINTS": ("kind", "value"),
    "CELLS": ("kind", "value"),
    "RATIONAL_PERCENT": ("denominator", "kind", "numerator", "source_unit"),
    "COUNT_U32": ("kind", "value"),
    "CAPACITY_U16": ("kind", "value"),
}
ROW_KEYS = {"field_path", "native_key", "source_item_id", "source_value", "typed_value"}
I32_MIN, I32_MAX = -(2**31), 2**31 - 1
U16_MAX = 2**16 - 1
U32_MAX = 2**32 - 1


class LoweringError(RuntimeError):
    pass


def canonical_bytes(value):
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    ).encode("utf-8")


def sha256_bytes(value):
    return hashlib.sha256(value).hexdigest()


def is_plain_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def require_int(value, *, minimum, maximum):
    if not is_plain_int(value) or value < minimum or value > maximum:
        return None
    return value


# --- encode: authored Item value -> Rust decoder's typed representation ------------
#
# Each branch below is the encoding-direction mirror of one arm of
# `decode_item_semantic_promotion_value` in
# apps/game-server/src/content/cw2_b1_import.rs. It never raises: a value this
# package's own schema allows but the Rust decoder's narrower bounds do not is simply
# skipped (the caller counts the skip), because a rejected row would defeat the point
# of shipping a decoder-accepted candidate.


def typed_value(field_path, raw):
    if field_path == "presentation.name":
        if not isinstance(raw, str) or not raw or len(raw.encode("utf-8")) > 2048:
            return None, None
        return raw, {"kind": "TEXT", "value": raw}

    if field_path in SIGNED_POINTS_FIELDS:
        value = require_int(raw, minimum=I32_MIN, maximum=I32_MAX)
        if value is None:
            return None, None
        return value, {"kind": "SIGNED_POINTS", "value": value}

    if field_path == "weapon.range_cells":
        value = require_int(raw, minimum=0, maximum=U16_MAX)
        if value is None:
            return None, None
        return value, {"kind": "CELLS", "value": value}

    if field_path == "weapon.hit_chance":
        # `raw` is the authored (numerator, denominator) ratio (item.weapon.
        # hit_chance_modifier_percent). The Rust field is one plain PERCENT_POINTS
        # integer, so only an exact whole-percent ratio (denominator 1) is eligible.
        numerator, denominator = raw
        if denominator != 1:
            return None, None
        percent_points = require_int(numerator, minimum=I32_MIN, maximum=I32_MAX)
        if percent_points is None:
            return None, None
        divisor = math.gcd(abs(percent_points), 100)
        return percent_points, {
            "kind": "RATIONAL_PERCENT",
            "numerator": percent_points // divisor,
            "denominator": 100 // divisor,
            "source_unit": "PERCENT_POINTS",
        }

    if field_path == "charges.count":
        value = require_int(raw, minimum=0, maximum=U32_MAX)
        if value is None:
            return None, None
        return value, {"kind": "COUNT_U32", "value": value}

    if field_path == "container.capacity":
        value = require_int(raw, minimum=0, maximum=U16_MAX)
        if value is None:
            return None, None
        return value, {"kind": "CAPACITY_U16", "value": value}

    raise LoweringError(f"unsupported field_path {field_path!r}")


def item_field_values(item):
    """Yield (field_path, raw) for every one of the 9 lowering field paths this
    authored Item bundle carries a value for."""
    if "display_name" in item:
        yield "presentation.name", item["display_name"]
    weapon = item.get("weapon", {})
    for field_path, source_key in (
        ("weapon.attack", "attack"),
        ("weapon.defense", "defense"),
        ("weapon.extra_defense", "extra_defense"),
        ("weapon.range_cells", "range_cells"),
    ):
        if source_key in weapon:
            yield field_path, weapon[source_key]
    if "hit_chance_modifier_percent" in weapon:
        ratio = weapon["hit_chance_modifier_percent"]
        yield "weapon.hit_chance", (ratio["numerator"], ratio["denominator"])
    protection = item.get("protection", {})
    if "armor" in protection:
        yield "protection.armor", protection["armor"]
    charges = item.get("charges", {})
    if "count" in charges:
        yield "charges.count", charges["count"]
    container = item.get("container", {})
    if "capacity" in container:
        yield "container.capacity", container["capacity"]


# --- decode: mirror `decode_item_semantic_promotion_value` in the checking direction


def validate_row(row):
    field_path = row["field_path"]
    kind = FIELD_KIND.get(field_path)
    if kind is None:
        raise LoweringError(f"row {row!r}: unsupported field_path")
    typed = row["typed_value"]
    if not isinstance(typed, dict) or set(typed) != set(TYPED_VALUE_KEYS[kind]):
        raise LoweringError(f"row {row!r}: typed_value shape")
    if typed.get("kind") != kind:
        raise LoweringError(f"row {row!r}: typed_value kind")
    source = row["source_value"]

    if kind == "TEXT":
        value = typed["value"]
        if (
            not isinstance(value, str)
            or not isinstance(source, str)
            or value != source
            or not value
            or len(value.encode("utf-8")) > 2048
        ):
            raise LoweringError(f"row {row!r}: text value")
        return

    if kind in ("SIGNED_POINTS", "CELLS", "COUNT_U32", "CAPACITY_U16"):
        bounds = {
            "SIGNED_POINTS": (I32_MIN, I32_MAX),
            "CELLS": (0, U16_MAX),
            "COUNT_U32": (0, U32_MAX),
            "CAPACITY_U16": (0, U16_MAX),
        }[kind]
        value = typed["value"]
        if (
            not is_plain_int(value)
            or not is_plain_int(source)
            or value != source
            or not (bounds[0] <= value <= bounds[1])
        ):
            raise LoweringError(f"row {row!r}: {kind} value")
        return

    if kind == "RATIONAL_PERCENT":
        if typed.get("source_unit") != "PERCENT_POINTS":
            raise LoweringError(f"row {row!r}: rational percent source_unit")
        numerator = typed["numerator"]
        denominator = typed["denominator"]
        if (
            not is_plain_int(numerator)
            or not is_plain_int(denominator)
            or not is_plain_int(source)
            or denominator <= 0
            or numerator * 100 != source * denominator
            or not (I32_MIN <= numerator <= I32_MAX)
            or math.gcd(abs(numerator), denominator) != 1
        ):
            raise LoweringError(f"row {row!r}: rational percent value")
        return

    raise LoweringError(f"row {row!r}: unreachable kind {kind!r}")


def validate_packet(packet):
    """Fail-closed replica of every check
    `protected_cw2_b1_promoted_item_family_import`/`validate_item_semantic_promotion_
    packet` (apps/game-server/src/content/cw2_b1_import.rs) would apply to
    `packet["promotions"]`, run against this candidate before it is written or
    accepted by `--check`."""
    promotions = packet["promotions"]
    if not isinstance(promotions, list) or not promotions:
        raise LoweringError("promotions must be a non-empty list")

    seen_atoms = set()
    per_field = Counter()
    previous = None
    for row in promotions:
        if set(row) != ROW_KEYS:
            raise LoweringError(f"row {row!r}: unexpected keys")
        if not isinstance(row["native_key"], str) or not row["native_key"]:
            raise LoweringError(f"row {row!r}: native_key")
        if not is_plain_int(row["source_item_id"]) or row["source_item_id"] < 0:
            raise LoweringError(f"row {row!r}: source_item_id")
        ordering_key = (row["native_key"], row["field_path"], row["source_item_id"])
        if previous is not None and previous >= ordering_key:
            raise LoweringError(f"row {row!r}: ordering")
        previous = ordering_key
        atom = (row["native_key"], row["field_path"])
        if atom in seen_atoms:
            raise LoweringError(f"row {row!r}: duplicate atom")
        seen_atoms.add(atom)
        per_field[row["field_path"]] += 1
        validate_row(row)

    counts = packet["counts"]
    if counts.get("promoted_fields") != len(promotions):
        raise LoweringError("counts.promoted_fields mismatch")
    if counts.get("promoted_items") != len({row["native_key"] for row in promotions}):
        raise LoweringError("counts.promoted_items mismatch")
    if counts.get("per_field") != dict(sorted(per_field.items())):
        raise LoweringError("counts.per_field mismatch")


def build_packet(sources):
    """Return (packet, skipped): skipped counts, per field_path, values this
    package's own schema allowed but the Rust decoder's narrower bounds did not."""
    promotions = []
    skipped = Counter()
    bundles = {}

    for item_id in sorted(sources["items"]):
        item, dependencies, report = convert_item(sources, item_id)
        if item is None or not report.get("converted"):
            continue
        errors, _warnings = validate_item.validate(item, dependencies)
        if errors:
            continue
        key = item["identity"]["key"]
        bundles[key] = {"item": item, "dependencies": dependencies}
        for field_path, raw in item_field_values(item):
            source_value, typed = typed_value(field_path, raw)
            if typed is None:
                skipped[field_path] += 1
                continue
            promotions.append(
                {
                    "field_path": field_path,
                    "native_key": key,
                    "source_item_id": item_id,
                    "source_value": source_value,
                    "typed_value": typed,
                }
            )

    promotions.sort(
        key=lambda row: (row["native_key"], row["field_path"], row["source_item_id"])
    )
    per_field = Counter(row["field_path"] for row in promotions)
    bundle_digest = sha256_bytes(
        canonical_bytes({key: bundles[key] for key in sorted(bundles)})
    )
    compiler_sha256 = sha256_bytes(Path(__file__).read_bytes().replace(b"\r\n", b"\n"))

    packet = {
        "schema": SCHEMA,
        "profile": PROFILE,
        "target_date": TARGET_DATE,
        "status": STATUS,
        "compiler": {"path": COMPILER_PATH, "sha256": compiler_sha256},
        "protected_lineage": {
            "source_engine": "crystal",
            "source_profile": sources["profile"],
            "source_repository": sources["repository"],
            "source_revision": sources["revision"],
            "source_artifact_digests": sources["artifact_digests"],
            "source_population_bundle_digest": bundle_digest,
            "eligible_field_count": len(FIELD_KIND),
        },
        "counts": {
            "promoted_fields": len(promotions),
            "promoted_items": len({row["native_key"] for row in promotions}),
            "per_field": dict(sorted(per_field.items())),
        },
        "promotions": promotions,
        "invariants": {
            "identity_reminted": False,
            "mutable_wiki_revision_metadata_retained": False,
            "name_only_identity_resolution": False,
            "only_derived_eligible_fields_promoted": True,
            "sibling_fields_default_unknown_or_existing_state": True,
            "whole_item_promotion": False,
            "wired_into_rust_importer": False,
        },
        "next_action": NEXT_ACTION,
    }
    validate_packet(packet)
    return packet, skipped


SELF_CHECK_MAGIC_SWORD_ID = 3288
SELF_CHECK_MAGIC_SWORD_KEY = "oteryn:item.registry.i00003167"
SELF_CHECK_CONTAINER_ID = 116
SELF_CHECK_CONTAINER_KEY = "oteryn:item.registry.i00000037"
SELF_CHECK_CHARGES_ID = 814
SELF_CHECK_CHARGES_KEY = "oteryn:item.registry.i00000726"


def self_check(packet):
    rows = {(row["native_key"], row["field_path"]): row for row in packet["promotions"]}

    def row(key, field_path):
        found = rows.get((key, field_path))
        assert found is not None, (key, field_path)
        return found

    magic_sword_name = row(SELF_CHECK_MAGIC_SWORD_KEY, "presentation.name")
    assert magic_sword_name["source_item_id"] == SELF_CHECK_MAGIC_SWORD_ID, (
        magic_sword_name
    )
    assert magic_sword_name["typed_value"] == {
        "kind": "TEXT",
        "value": "magic sword",
    }, magic_sword_name

    magic_sword_attack = row(SELF_CHECK_MAGIC_SWORD_KEY, "weapon.attack")
    assert magic_sword_attack["typed_value"] == {
        "kind": "SIGNED_POINTS",
        "value": 48,
    }, magic_sword_attack

    magic_sword_defense = row(SELF_CHECK_MAGIC_SWORD_KEY, "weapon.defense")
    assert magic_sword_defense["typed_value"] == {
        "kind": "SIGNED_POINTS",
        "value": 35,
    }, magic_sword_defense

    container = row(SELF_CHECK_CONTAINER_KEY, "container.capacity")
    assert container["source_item_id"] == SELF_CHECK_CONTAINER_ID, container
    assert container["typed_value"] == {"kind": "CAPACITY_U16", "value": 15}, container

    charges = row(SELF_CHECK_CHARGES_KEY, "charges.count")
    assert charges["source_item_id"] == SELF_CHECK_CHARGES_ID, charges
    assert charges["typed_value"] == {"kind": "COUNT_U32", "value": 200}, charges

    print(
        json.dumps(
            {
                "self_check": "ok",
                "keys": [
                    SELF_CHECK_MAGIC_SWORD_KEY,
                    SELF_CHECK_CONTAINER_KEY,
                    SELF_CHECK_CHARGES_KEY,
                ],
            }
        )
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--source", type=Path, required=True, help="pinned Crystal checkout"
    )
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--self-check", action="store_true")
    parser.add_argument(
        "--check",
        action="store_true",
        help=(
            "regenerate the packet in memory and diff it against the committed "
            "samples file; exits 1 on drift and never writes"
        ),
    )
    args = parser.parse_args()

    sources = load_engine_sources("crystal", args.source)
    packet, skipped = build_packet(sources)
    candidate_bytes = canonical_bytes(packet)

    if args.check:
        if not args.out.is_file():
            raise SystemExit(f"no committed packet at {args.out} to check against")
        committed_bytes = args.out.read_bytes()
        if candidate_bytes != committed_bytes:
            raise SystemExit(
                f"lowering packet drift detected: regenerated packet differs from "
                f"committed {args.out}"
            )
        print(json.dumps({"check": "ok", "out": str(args.out)}))
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_bytes(candidate_bytes)
        print(
            json.dumps(
                {
                    "counts": packet["counts"],
                    "skipped": dict(sorted(skipped.items())),
                    "bytes": len(candidate_bytes),
                    "out": str(args.out),
                }
            )
        )

    if args.self_check:
        self_check(packet)


if __name__ == "__main__":
    main()
