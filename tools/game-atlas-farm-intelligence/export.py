#!/usr/bin/env python3
"""Build and verify the bounded Game-owned Atlas farm-intelligence read model.

Input is a normalized, already-public Game read model.  This program deliberately
does not parse or execute gameplay source files and does not access the network.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
from typing import Any

CONTRACT_ID = "oteryn-game-atlas-farm-intelligence-v1"
SCHEMA_VERSION = 1
PRODUCER_REVISION = "farm-intelligence-v1"
STATES = {"COMPLETE", "PARTIAL", "UNSUPPORTED", "UNKNOWN"}
QUANTITY_MODELS = {"FIXED", "EXACT_PMF", "BOUNDED_UNKNOWN", "UNSUPPORTED"}
PROBABILITY_CONTEXTS = {"STATIC_MIGRATION_PROFILE_NOT_LIVE_CURRENT", "EXACT_RULESET_PROFILE_BASE"}
FAMILIES = (
    "item_identity", "creature_identity", "loot_probability", "loot_quantity",
    "placement_supply", "tasks", "weekly", "respawn",
)
LIMITS = {
    "max_input_bytes": 4_194_304, "max_product_bytes": 4_194_304,
    "max_records": 16_384, "max_string_bytes": 512, "max_pmf_points": 128,
    "max_count": 1_000_000, "max_probability_denominator": 1_000_000_000,
}
SHA = re.compile(r"^[0-9a-f]{40}$")
CREATURE = re.compile(r"^monster-entity:[0-9a-f]{32}$")
ITEM = re.compile(r"^oteryn:item\.[a-z0-9][a-z0-9._-]{0,127}$")


class ProductError(ValueError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True,
                       separators=(",", ":")) + "\n").encode("utf-8")


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def _text(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or len(value.encode()) > LIMITS["max_string_bytes"]:
        raise ProductError(f"invalid string: {where}")
    return value


def _exact(obj: Any, required: set[str], optional: set[str], where: str) -> dict[str, Any]:
    if not isinstance(obj, dict) or not required <= obj.keys() or obj.keys() - required - optional:
        raise ProductError(f"invalid keys: {where}")
    return obj


def _capabilities(raw: Any) -> dict[str, Any]:
    obj = _exact(raw, set(FAMILIES), set(), "capabilities")
    result = {}
    for family in FAMILIES:
        cap = _exact(obj[family], {"state", "reason_codes"}, set(), family)
        if cap["state"] not in STATES or not isinstance(cap["reason_codes"], list):
            raise ProductError(f"invalid capability: {family}")
        reasons = sorted({_text(x, f"{family}.reason") for x in cap["reason_codes"]})
        if cap["state"] != "COMPLETE" and not reasons:
            raise ProductError(f"non-complete capability needs a reason: {family}")
        result[family] = {"state": cap["state"], "reason_codes": reasons}
    return result


def _probability(raw: Any, where: str) -> dict[str, Any]:
    value = _exact(raw, {"numerator", "denominator", "context"}, set(), where)
    n, d = value["numerator"], value["denominator"]
    if not isinstance(n, int) or isinstance(n, bool) or not isinstance(d, int) or isinstance(d, bool):
        raise ProductError(f"non-integer probability: {where}")
    if d <= 0 or d > LIMITS["max_probability_denominator"] or n < 0 or n > d:
        raise ProductError(f"out-of-range probability: {where}")
    context = _text(value["context"], f"{where}.context")
    if context not in PROBABILITY_CONTEXTS and context != "PER_KILL_QUANTITY":
        raise ProductError(f"unsupported probability context: {where}")
    return {"numerator": n, "denominator": d, "context": context}


def _quantity(raw: Any, where: str) -> dict[str, Any]:
    value = _exact(raw, {"model"}, {"count", "min_count", "max_count", "pmf"}, where)
    model = value["model"]
    if model not in QUANTITY_MODELS:
        raise ProductError(f"invalid quantity model: {where}")
    if model == "FIXED":
        if set(value) != {"model", "count"} or not isinstance(value["count"], int) or isinstance(value["count"], bool) or not 0 <= value["count"] <= LIMITS["max_count"]:
            raise ProductError(f"invalid fixed quantity: {where}")
    elif model == "BOUNDED_UNKNOWN":
        if set(value) != {"model", "min_count", "max_count"}:
            raise ProductError(f"invalid bounded quantity: {where}")
        low, high = value["min_count"], value["max_count"]
        if not all(isinstance(x, int) and not isinstance(x, bool) for x in (low, high)) or not 0 <= low <= high <= LIMITS["max_count"]:
            raise ProductError(f"invalid quantity bounds: {where}")
    elif model == "EXACT_PMF":
        if set(value) != {"model", "pmf"} or not isinstance(value["pmf"], list) or not value["pmf"] or len(value["pmf"]) > LIMITS["max_pmf_points"]:
            raise ProductError(f"invalid exact PMF: {where}")
        points, seen, total_n, denominator = [], set(), 0, None
        for index, point in enumerate(value["pmf"]):
            point = _exact(point, {"count", "numerator", "denominator"}, set(), f"{where}.pmf[{index}]")
            count = point["count"]
            probability = _probability({"numerator": point["numerator"],
                                        "denominator": point["denominator"],
                                        "context": "PER_KILL_QUANTITY"}, where)
            if not isinstance(count, int) or isinstance(count, bool) or not 0 <= count <= LIMITS["max_count"] or count in seen:
                raise ProductError(f"invalid PMF count: {where}")
            seen.add(count); denominator = denominator or probability["denominator"]
            if probability["denominator"] != denominator:
                raise ProductError(f"mixed PMF denominator: {where}")
            total_n += probability["numerator"]
            points.append({"count": count, "numerator": probability["numerator"], "denominator": denominator})
        if total_n != denominator:
            raise ProductError(f"PMF does not sum to one: {where}")
        value = {"model": model, "pmf": sorted(points, key=lambda x: x["count"])}
    elif set(value) != {"model"}:
        raise ProductError(f"unsupported quantity has extra facts: {where}")
    return value


def build(source: dict[str, Any]) -> dict[str, Any]:
    source = _exact(source, {"source", "capabilities", "creatures", "items", "loot_relations"}, set(), "source")
    provenance = _exact(source["source"], {"repository", "revision", "semantic_digest", "generation"}, set(), "source.provenance")
    if not SHA.fullmatch(str(provenance["revision"])) or not re.fullmatch(r"sha256:[0-9a-f]{64}", str(provenance["semantic_digest"])):
        raise ProductError("invalid source revision/digest")
    generation = _text(provenance["generation"], "source.generation")
    provenance = {"repository": _text(provenance["repository"], "source.repository"), **{k: provenance[k] for k in ("revision", "semantic_digest")}, "generation": generation}
    capabilities = _capabilities(source["capabilities"])
    if not all(isinstance(source[x], list) for x in ("creatures", "items", "loot_relations")):
        raise ProductError("record families must be arrays")
    if sum(len(source[x]) for x in ("creatures", "items", "loot_relations")) > LIMITS["max_records"]:
        raise ProductError("record limit exceeded")
    creatures, creature_ids = [], set()
    for row in source["creatures"]:
        row = _exact(row, {"creature_id", "display_name", "generation"}, set(), "creature")
        cid = row["creature_id"]
        if not isinstance(cid, str) or not CREATURE.fullmatch(cid) or cid in creature_ids or row["generation"] != generation:
            raise ProductError("duplicate/invalid/mixed-generation creature")
        creature_ids.add(cid); creatures.append({"creature_id": cid, "display_name": _text(row["display_name"], "creature.display_name")})
    items, item_ids = [], set()
    for row in source["items"]:
        row = _exact(row, {"item_id", "display_name", "generation"}, set(), "item")
        iid = row["item_id"]
        if not isinstance(iid, str) or not ITEM.fullmatch(iid) or iid in item_ids or row["generation"] != generation:
            raise ProductError("duplicate/invalid/mixed-generation item")
        item_ids.add(iid); items.append({"item_id": iid, "display_name": _text(row["display_name"], "item.display_name")})
    loot, keys = [], set()
    if source["loot_relations"] and capabilities["loot_probability"]["state"] not in {"COMPLETE", "PARTIAL"}:
        raise ProductError("loot relations require supported or partial probability semantics")
    for index, row in enumerate(source["loot_relations"]):
        row = _exact(row, {"creature_id", "item_id", "item_display_name", "item_resolution_state", "probability", "quantity", "generation"}, set(), f"loot[{index}]")
        if row["generation"] != generation or row["creature_id"] not in creature_ids:
            raise ProductError("dangling or mixed-generation loot creature")
        iid = row["item_id"]
        if iid is not None and iid not in item_ids:
            raise ProductError("dangling loot item")
        if row["item_resolution_state"] not in {"RESOLVED", "UNRESOLVED", "AMBIGUOUS", "UNKNOWN"} or (iid is None) == (row["item_resolution_state"] == "RESOLVED"):
            raise ProductError("inconsistent item resolution")
        key = (row["creature_id"], iid, row["item_display_name"])
        if key in keys: raise ProductError("duplicate loot relation")
        keys.add(key)
        quantity = _quantity(row["quantity"], f"loot[{index}].quantity")
        if quantity["model"] in {"FIXED", "EXACT_PMF"} and capabilities["loot_quantity"]["state"] != "COMPLETE":
            raise ProductError("exact quantity model requires complete source proof")
        loot.append({"creature_id": row["creature_id"], "item_id": iid,
                     "item_display_name": _text(row["item_display_name"], "loot.item_display_name"),
                     "item_resolution_state": row["item_resolution_state"],
                     "probability": _probability(row["probability"], f"loot[{index}].probability"),
                     "quantity": quantity})
    body = {"contract_id": CONTRACT_ID, "schema_version": SCHEMA_VERSION,
            "producer_revision": PRODUCER_REVISION, "source": provenance,
            "capabilities": capabilities, "creatures": sorted(creatures, key=lambda x: x["creature_id"]),
            "items": sorted(items, key=lambda x: x["item_id"]),
            "loot_relations": sorted(loot, key=lambda x: (x["creature_id"], x["item_id"] or "", x["item_display_name"])),
            "tasks": [], "limits": LIMITS}
    body["semantic_digest"] = digest(body)
    if len(canonical_bytes(body)) > LIMITS["max_product_bytes"]: raise ProductError("product byte limit exceeded")
    return body


def verify(product: dict[str, Any]) -> dict[str, Any]:
    if product.get("contract_id") != CONTRACT_ID or product.get("schema_version") != SCHEMA_VERSION or product.get("producer_revision") != PRODUCER_REVISION:
        raise ProductError("unsupported product revision")
    claimed = product.get("semantic_digest")
    unsigned = dict(product); unsigned.pop("semantic_digest", None)
    if claimed != digest(unsigned): raise ProductError("semantic digest mismatch")
    generation = product["source"]["generation"]
    rebuilt = build({"source": product["source"], "capabilities": product["capabilities"],
                     "creatures": [{**row, "generation": generation} for row in product["creatures"]],
                     "items": [{**row, "generation": generation} for row in product["items"]],
                     "loot_relations": [{**row, "generation": generation} for row in product["loot_relations"]]})
    if canonical_bytes(rebuilt) != canonical_bytes(product): raise ProductError("non-canonical or invalid product")
    return product


def load(path: Path, maximum: int) -> Any:
    if not path.is_file() or path.stat().st_size > maximum: raise ProductError("missing or oversized JSON")
    try: return json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc: raise ProductError("malformed UTF-8 JSON") from exc


def safe_output(path: str) -> Path:
    posix = PurePosixPath(path)
    if posix.is_absolute() or ".." in posix.parts or "\\" in path or len(posix.parts) != 1:
        raise ProductError("output must be a safe file name")
    return Path(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("export", "verify")); parser.add_argument("input")
    parser.add_argument("output", nargs="?")
    args = parser.parse_args()
    if args.command == "export":
        if not args.output: parser.error("export requires output")
        product = build(load(Path(args.input), LIMITS["max_input_bytes"]))
        output = safe_output(args.output); output.write_bytes(canonical_bytes(product)); verify(product)
    else:
        verify(load(Path(args.input), LIMITS["max_product_bytes"]))
    return 0


if __name__ == "__main__": raise SystemExit(main())
