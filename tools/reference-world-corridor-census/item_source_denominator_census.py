#!/usr/bin/env python3
"""Structural denominator census for the pinned Crystal items.xml catalogue.

This tool does not promote Item semantics. It answers a narrower question:
how many XML definitions exist before/after range expansion, how many distinct
names they expose, and how those definitions partition by already-admitted B1
field dispositions.

Signal flags:
- G: at least one B1 GAME_ITEM_CANDIDATE field
- P: at least one primarytype taxonomy observation
- W: at least one B1 UNSUPPORTED world/interaction field

The flags are descriptive only; none of them is canonical Item identity.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import statistics
import xml.etree.ElementTree as ET

SCHEMA = "OTERYN_ITEM_SOURCE_DENOMINATOR_CENSUS/v1"
DEFAULT_EXPECTED_SHA256 = "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"


def canonical_bytes(value: object) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def normalize_name(value: str) -> str:
    return " ".join(value.strip().casefold().split())


def percentile(values: list[int], fraction: float) -> int | None:
    if not values:
        return None
    index = min(len(values) - 1, int((len(values) - 1) * fraction))
    return values[index]


def load_b1_mapper():
    path = Path(__file__).with_name("item_identity_catalog.py")
    spec = importlib.util.spec_from_file_location("item_identity_catalog", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("unable to load item_identity_catalog.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def classify_node(node: ET.Element, mapper) -> tuple[bool, bool, bool]:
    gameplay = False
    primarytype = False
    world = False
    for child in node.findall("attribute"):
        key = child.attrib.get("key")
        if not isinstance(key, str) or not key:
            continue
        normalized = key.casefold()
        if normalized == "primarytype":
            primarytype = True
        disposition = mapper.field_disposition(normalized)["disposition"]
        if disposition == "GAME_ITEM_CANDIDATE":
            gameplay = True
        elif disposition == "UNSUPPORTED":
            world = True
    return gameplay, primarytype, world


def expanded_count(node: ET.Element) -> tuple[int, bool]:
    if "id" in node.attrib:
        return 1, False
    if "fromid" not in node.attrib or "toid" not in node.attrib:
        raise RuntimeError("source node without complete numeric identity")
    start = int(node.attrib["fromid"])
    end = int(node.attrib["toid"])
    if start > end:
        return 0, True
    return end - start + 1, False


def compile_census(payload: bytes) -> dict[str, object]:
    mapper = load_b1_mapper()
    root = ET.fromstring(payload.decode("iso-8859-1"))
    nodes = list(root.findall("item"))

    direct_nodes = 0
    range_nodes = 0
    reversed_ranges = 0
    valid_definitions = 0
    expanded_identities = 0
    range_lengths: list[int] = []
    exact_names: Counter[str] = Counter()
    normalized_names: Counter[str] = Counter()
    signal_partition: dict[str, dict[str, int]] = {}

    for node in nodes:
        count, reversed_range = expanded_count(node)
        if "id" in node.attrib:
            direct_nodes += 1
        else:
            range_nodes += 1
        if reversed_range:
            reversed_ranges += 1
            continue

        valid_definitions += 1
        expanded_identities += count
        if "fromid" in node.attrib:
            range_lengths.append(count)

        name = node.attrib.get("name", "").strip()
        exact_names[name] += 1
        normalized_names[normalize_name(name)] += 1

        gameplay, primarytype, world = classify_node(node, mapper)
        key = (
            ("G" if gameplay else "-")
            + ("P" if primarytype else "-")
            + ("W" if world else "-")
        )
        entry = signal_partition.setdefault(
            key, {"definitions": 0, "expanded_ids": 0}
        )
        entry["definitions"] += 1
        entry["expanded_ids"] += count

    range_lengths.sort()
    duplicate_name_groups = {
        name: count for name, count in normalized_names.items() if count > 1
    }

    aggregate = {
        "gameplay_no_world": {
            "definitions": sum(
                signal_partition.get(key, {}).get("definitions", 0)
                for key in ("GP-", "G--")
            ),
            "expanded_ids": sum(
                signal_partition.get(key, {}).get("expanded_ids", 0)
                for key in ("GP-", "G--")
            ),
        },
        "gameplay_with_world_overlap": {
            "definitions": sum(
                signal_partition.get(key, {}).get("definitions", 0)
                for key in ("G-W", "GPW")
            ),
            "expanded_ids": sum(
                signal_partition.get(key, {}).get("expanded_ids", 0)
                for key in ("G-W", "GPW")
            ),
        },
        "taxonomy_only_no_world": {
            "definitions": signal_partition.get("-P-", {}).get("definitions", 0),
            "expanded_ids": signal_partition.get("-P-", {}).get("expanded_ids", 0),
        },
        "world_only_no_gameplay": {
            "definitions": sum(
                signal_partition.get(key, {}).get("definitions", 0)
                for key in ("-PW", "--W")
            ),
            "expanded_ids": sum(
                signal_partition.get(key, {}).get("expanded_ids", 0)
                for key in ("-PW", "--W")
            ),
        },
        "residual_no_gameplay_taxonomy_or_world_signal": {
            "definitions": signal_partition.get("---", {}).get("definitions", 0),
            "expanded_ids": signal_partition.get("---", {}).get("expanded_ids", 0),
        },
    }

    top_duplicate_names = sorted(
        duplicate_name_groups.items(), key=lambda pair: (-pair[1], pair[0])
    )[:40]

    return {
        "schema": SCHEMA,
        "source": {
            "repository": "zimbadev/crystalserver",
            "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
            "path": "data/items/items.xml",
            "git_blob": "0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f",
            "bytes": len(payload),
            "sha256": sha256_bytes(payload),
        },
        "counts": {
            "source_xml_item_nodes": len(nodes),
            "direct_id_nodes": direct_nodes,
            "range_nodes": range_nodes,
            "reversed_range_nodes_excluded": reversed_ranges,
            "valid_xml_definitions": valid_definitions,
            "expanded_source_item_identities": expanded_identities,
        },
        "range_distribution": {
            "min": range_lengths[0] if range_lengths else None,
            "median": int(statistics.median(range_lengths)) if range_lengths else None,
            "p90": percentile(range_lengths, 0.90),
            "p95": percentile(range_lengths, 0.95),
            "p99": percentile(range_lengths, 0.99),
            "max": range_lengths[-1] if range_lengths else None,
            "length_le_2": sum(value <= 2 for value in range_lengths),
            "length_le_5": sum(value <= 5 for value in range_lengths),
            "length_le_10": sum(value <= 10 for value in range_lengths),
            "length_gt_100": sum(value > 100 for value in range_lengths),
        },
        "name_census": {
            "unique_exact_names": len(exact_names),
            "unique_normalized_names": len(normalized_names),
            "duplicate_normalized_name_groups": len(duplicate_name_groups),
            "definitions_in_duplicate_normalized_name_groups": sum(
                duplicate_name_groups.values()
            ),
            "max_definitions_for_one_normalized_name": (
                top_duplicate_names[0][1] if top_duplicate_names else 0
            ),
            "top_duplicate_normalized_names": [
                {"name": name, "definitions": count}
                for name, count in top_duplicate_names
            ],
        },
        "signal_contract": {
            "G": "at least one field classified GAME_ITEM_CANDIDATE by protected B1 mapper",
            "P": "at least one primarytype taxonomy observation",
            "W": "at least one field classified UNSUPPORTED world/interaction semantics by protected B1 mapper",
            "warning": (
                "These are source-shape signals, not canonical product categories. "
                "Range members, transform/decay states and world objects may still share ItemType transport."
            ),
        },
        "signal_partition": dict(sorted(signal_partition.items())),
        "aggregate_partition": aggregate,
        "interpretation": {
            "expanded_38157_is_not_distinct_player_item_count": True,
            "preferred_structural_denominator": "valid_xml_definitions",
            "name_vocabulary_metric": "unique_normalized_names",
            "canonical_item_identity_decision": "NOT_PERFORMED",
            "semantic_promotion": "NOT_PERFORMED",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-xml", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected-sha256", default=DEFAULT_EXPECTED_SHA256)
    args = parser.parse_args()

    payload = args.source_xml.read_bytes()
    actual = sha256_bytes(payload)
    if actual != args.expected_sha256:
        raise SystemExit(
            f"source sha256 mismatch: expected {args.expected_sha256}, got {actual}"
        )
    census = compile_census(payload)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(census))
    print(
        "item-source-denominator-census: PASS "
        f"definitions={census['counts']['valid_xml_definitions']} "
        f"expanded={census['counts']['expanded_source_item_identities']} "
        f"names={census['name_census']['unique_normalized_names']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
