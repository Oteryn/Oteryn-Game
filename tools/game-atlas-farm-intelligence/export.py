#!/usr/bin/env python3
"""Emit/verify the fail-closed farm-intelligence v1 availability product.

No authoritative source publication is available in this checkout.  Production
therefore has no caller-supplied input surface: it can only emit the canonical
blocked product below.  Rich validation helpers in this module are explicitly
test-fixture-only and can never produce the public contract identity.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
from typing import Any

CONTRACT_ID = "oteryn-game-atlas-farm-intelligence-v1"
SCHEMA_VERSION = 1
PRODUCER_REVISION = "farm-intelligence-v1"
TEST_CONTRACT_ID = "oteryn-game-atlas-farm-intelligence-test-fixture-v1"
TEST_AUTHORITY = "SYNTHETIC_TEST_FIXTURE_NOT_SOURCE_AUTHORITY"
STATES = {"COMPLETE", "PARTIAL", "UNSUPPORTED", "UNKNOWN"}
FAMILIES = (
    "item_identity", "creature_identity", "loot_probability", "loot_quantity",
    "placement_supply", "tasks", "weekly", "respawn",
)
# These bounds protect generated unit-test fixtures only. They are not emitted,
# accepted, or represented as production/public producer limits.
TEST_ONLY_LIMITS = {
    "max_records": 64, "max_string_bytes": 256, "max_pmf_points": 16,
    "max_count": 10_000, "max_probability_denominator": 1_000_000,
}
CREATURE = re.compile(r"^monster-entity:[0-9a-f]{32}$")
ITEM = re.compile(r"^oteryn:item\.[a-z0-9][a-z0-9._-]{0,127}$")


class ProductError(ValueError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True,
                       separators=(",", ":")) + "\n").encode("utf-8")


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def blocked_product() -> dict[str, Any]:
    """Return the sole production-publication candidate admitted by v1."""
    reason = "AUTHORITATIVE_NORMALIZED_GAME_PUBLICATION_UNAVAILABLE"
    capabilities = {
        family: {"state": "UNSUPPORTED", "reason_codes": [reason]}
        for family in FAMILIES
    }
    body = {
        "contract_id": CONTRACT_ID,
        "schema_version": SCHEMA_VERSION,
        "producer_revision": PRODUCER_REVISION,
        "publication_state": "BLOCKED_NO_ADMITTED_SOURCE",
        "source": {"state": "UNAVAILABLE", "reason_codes": [reason]},
        "capabilities": capabilities,
        "creatures": [], "items": [], "loot_relations": [], "tasks": [],
    }
    body["semantic_digest"] = digest(body)
    return body


def verify(product: Any) -> dict[str, Any]:
    """Accept only the exact canonical blocked product, not caller facts."""
    expected = blocked_product()
    if not isinstance(product, dict) or canonical_bytes(product) != canonical_bytes(expected):
        raise ProductError("v1 accepts only its canonical fail-closed product")
    return product


def load_blocked_product(path: Path) -> dict[str, Any]:
    """Read no more bytes than the exact, internally generated product needs."""
    expected_bytes = canonical_bytes(blocked_product())
    if not path.is_file() or path.stat().st_size != len(expected_bytes):
        raise ProductError("product byte length is not the canonical blocked product length")
    try:
        raw = path.read_bytes()
        if raw != expected_bytes:
            raise ProductError("product is not canonical")
        value = json.loads(raw)
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise ProductError("malformed UTF-8 JSON") from exc
    return verify(value)


def safe_output(path: str) -> Path:
    posix = PurePosixPath(path)
    if posix.is_absolute() or ".." in posix.parts or "\\" in path or len(posix.parts) != 1:
        raise ProductError("output must be a safe file name")
    return Path(path)


def write_blocked_product(path: Path) -> None:
    """Write the canonical product without following the final path component."""
    nofollow = getattr(os, "O_NOFOLLOW", None)
    if not isinstance(nofollow, int) or nofollow == 0:
        raise ProductError("safe no-follow output writes are unavailable")
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC | nofollow
    try:
        descriptor = os.open(path, flags, 0o666)
    except OSError as exc:
        raise ProductError("output cannot be opened without following links") from exc
    try:
        with os.fdopen(descriptor, "wb") as output:
            descriptor = -1
            output.write(canonical_bytes(blocked_product()))
    except OSError as exc:
        raise ProductError("canonical product write failed") from exc
    finally:
        if descriptor >= 0:
            os.close(descriptor)


# The helpers below exercise prospective semantics without accepting a source or
# producing CONTRACT_ID. They deliberately use small, separately named test-only
# limits so synthetic data cannot masquerade as a production census or product.
def _text(value: Any, where: str) -> str:
    if (not isinstance(value, str) or not value or
            len(value.encode()) > TEST_ONLY_LIMITS["max_string_bytes"]):
        raise ProductError(f"invalid test-fixture string: {where}")
    return value


def _quantity(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("model") not in {
            "FIXED", "EXACT_PMF", "BOUNDED_UNKNOWN", "UNSUPPORTED"}:
        raise ProductError("invalid test-fixture quantity")
    model = value["model"]
    if model == "FIXED":
        if set(value) != {"model", "count"} or not _test_count(value["count"]):
            raise ProductError("invalid test-fixture fixed quantity")
    elif model == "BOUNDED_UNKNOWN":
        if set(value) != {"model", "min_count", "max_count"}:
            raise ProductError("invalid test-fixture bounds")
        low, high = value["min_count"], value["max_count"]
        if not _test_count(low) or not _test_count(high) or low > high:
            raise ProductError("invalid test-fixture bounds")
    elif model == "EXACT_PMF":
        pmf = value.get("pmf")
        if set(value) != {"model", "pmf"} or not isinstance(pmf, list) or not pmf or len(pmf) > TEST_ONLY_LIMITS["max_pmf_points"]:
            raise ProductError("invalid test-fixture PMF")
        seen, denominator, total = set(), None, 0
        for point in pmf:
            if not isinstance(point, dict) or set(point) != {"count", "numerator", "denominator"}:
                raise ProductError("invalid test-fixture PMF point")
            count, numerator, point_denominator = point["count"], point["numerator"], point["denominator"]
            if (not _test_count(count) or count in seen or
                    not _test_integer(numerator) or not _test_integer(point_denominator) or
                    point_denominator <= 0 or point_denominator > TEST_ONLY_LIMITS["max_probability_denominator"] or
                    numerator < 0 or numerator > point_denominator):
                raise ProductError("invalid test-fixture PMF point")
            seen.add(count); denominator = denominator or point_denominator
            if denominator != point_denominator:
                raise ProductError("mixed test-fixture PMF denominator")
            total += numerator
        if total != denominator:
            raise ProductError("test-fixture PMF does not sum to one")
        value = {"model": model, "pmf": sorted(pmf, key=lambda point: point["count"])}
    elif set(value) != {"model"}:
        raise ProductError("unsupported test-fixture quantity has facts")
    return value


def _test_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _test_count(value: Any) -> bool:
    return _test_integer(value) and 0 <= value <= TEST_ONLY_LIMITS["max_count"]


def build_test_fixture(source: Any) -> dict[str, Any]:
    """Build a conspicuously non-publishable synthetic unit-test artifact."""
    if not isinstance(source, dict) or set(source) != {"test_authority", "creatures", "items", "loot_relations"}:
        raise ProductError("invalid synthetic fixture keys")
    if source["test_authority"] != TEST_AUTHORITY:
        raise ProductError("synthetic fixture lacks its fixed test-only marker")
    if not all(isinstance(source[key], list) for key in ("creatures", "items", "loot_relations")):
        raise ProductError("synthetic fixture families must be arrays")
    if sum(len(source[key]) for key in ("creatures", "items", "loot_relations")) > TEST_ONLY_LIMITS["max_records"]:
        raise ProductError("synthetic fixture record limit exceeded")
    creatures, creature_ids = [], set()
    for row in source["creatures"]:
        if not isinstance(row, dict) or set(row) != {"creature_id", "display_name"}:
            raise ProductError("invalid synthetic creature")
        cid = row["creature_id"]
        if not isinstance(cid, str) or not CREATURE.fullmatch(cid) or cid in creature_ids:
            raise ProductError("duplicate/invalid synthetic creature")
        creature_ids.add(cid); creatures.append({"creature_id": cid, "display_name": _text(row["display_name"], "creature")})
    items, item_ids = [], set()
    for row in source["items"]:
        if not isinstance(row, dict) or set(row) != {"item_id", "display_name"}:
            raise ProductError("invalid synthetic item")
        iid = row["item_id"]
        if not isinstance(iid, str) or not ITEM.fullmatch(iid) or iid in item_ids:
            raise ProductError("duplicate/invalid synthetic item")
        item_ids.add(iid); items.append({"item_id": iid, "display_name": _text(row["display_name"], "item")})
    loot, keys = [], set()
    for row in source["loot_relations"]:
        required = {"creature_id", "item_id", "item_display_name", "item_resolution_state", "probability", "quantity"}
        if not isinstance(row, dict) or set(row) != required or row["creature_id"] not in creature_ids:
            raise ProductError("invalid/dangling synthetic loot creature")
        iid = row["item_id"]
        if iid is not None and iid not in item_ids:
            raise ProductError("dangling synthetic loot item")
        state = row["item_resolution_state"]
        if state not in {"RESOLVED", "UNRESOLVED", "AMBIGUOUS", "UNKNOWN"} or (iid is None) == (state == "RESOLVED"):
            raise ProductError("inconsistent synthetic item resolution")
        probability = row["probability"]
        if not isinstance(probability, dict) or set(probability) != {"numerator", "denominator", "context"}:
            raise ProductError("invalid synthetic probability")
        n, d = probability["numerator"], probability["denominator"]
        if (not _test_integer(n) or not _test_integer(d) or d <= 0 or
                d > TEST_ONLY_LIMITS["max_probability_denominator"] or n < 0 or n > d or
                probability["context"] != "TEST_ONLY_STATIC_NOT_AUTHORITY"):
            raise ProductError("invalid synthetic probability")
        key = (row["creature_id"], iid, row["item_display_name"])
        if key in keys:
            raise ProductError("duplicate synthetic loot relation")
        keys.add(key)
        loot.append({**row, "item_display_name": _text(row["item_display_name"], "loot"),
                     "quantity": _quantity(row["quantity"])})
    body = {
        "contract_id": TEST_CONTRACT_ID, "publication_state": "TEST_ONLY_NON_PUBLISHABLE",
        "test_authority": TEST_AUTHORITY,
        "creatures": sorted(creatures, key=lambda row: row["creature_id"]),
        "items": sorted(items, key=lambda row: row["item_id"]),
        "loot_relations": sorted(loot, key=lambda row: (row["creature_id"], row["item_id"] or "", row["item_display_name"])),
    }
    body["semantic_digest"] = digest(body)
    return body


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("export", "verify"))
    parser.add_argument("path")
    args = parser.parse_args()
    if args.command == "export":
        output = safe_output(args.path)
        write_blocked_product(output)
        load_blocked_product(output)
    else:
        load_blocked_product(Path(args.path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
