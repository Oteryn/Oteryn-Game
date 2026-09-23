#!/usr/bin/env python3
"""Deterministic source-role census for the protected Crystal Item catalogue.

This is a denominator/scaling audit only. It never promotes donor values, remaps
canonical Oteryn identities, or decides gameplay semantics from names alone.
"""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import subprocess
from typing import Any
import xml.etree.ElementTree as ET

SCHEMA = "OTERYN_ITEM_SOURCE_ROLE_CENSUS/v1"
SOURCE_REPOSITORY = "zimbadev/crystalserver"
SOURCE_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
SOURCE_PATH = "data/items/items.xml"
SOURCE_BLOB = "0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f"
SOURCE_BYTES = 3_819_874
SOURCE_SHA256 = "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"

BUCKETS = (
    "PLAYER_CATALOG_STRONG",
    "WORLD_OBJECT_INTERACTION",
    "TECHNICAL_PLACEHOLDER",
    "STATE_VARIANT_DECAY",
    "STATE_VARIANT_TRANSFORM",
    "RANGE_VARIANT_NO_FIELDS",
    "DIRECT_SIMPLE_NO_FIELDS",
    "SIMPLE_PRESENTATION_CANDIDATE",
    "RESIDUAL_OTHER",
)

PLAYER_PRIMARY_TYPES = frozenset(
    {
        "soul cores",
        "quest items",
        "creature products",
        "containers",
        "valuables",
        "food",
        "club weapons",
        "sword weapons",
        "armors",
        "axe weapons",
        "documents and papers",
        "helmets",
        "tools",
        "amulets and necklaces",
        "distance weapons",
        "shields",
        "liquids",
        "legs",
        "rings",
        "boots",
        "natural products",
        "taming items",
        "game tokens",
        "rods",
        "ammunition",
        "fist weapons",
        "musical instruments",
        "attack runes",
        "party items",
        "wands",
        "books",
        "exercise weapons",
        "fluid containers",
        "magical items",
        "spellbooks",
        "keys",
        "tournament rewards",
        "quivers",
        "training weapons",
        "blessing charms",
        "support runes",
        "healing runes",
        "enchanted items",
        "clothing accessories",
        "painting equipment",
        "other items",
        "fansite items",
        "trophies",
        "plants and herbs",
        "dolls and bears",
        "kitchen tools",
    }
)

WORLD_PRIMARY_TYPES = frozenset(
    {
        "furniture",
        "decoration",
        "walls",
        "constructions",
        "artificial tiles",
        "natural tiles",
        "rocks",
        "floor decorations",
        "tools (objects)",
        "light sources",
        "statues",
        "machines (objects)",
        "wall hangings",
        "plants",
        "pillars",
        "trees",
        "windows",
        "quest objects",
        "remains",
        "tables",
        "fields",
        "teleporters",
        "flowers",
        "mushrooms",
        "animals",
        "rubbish",
        "refuse",
        "dropdowns",
        "shrines and altars",
        "grass",
        "doors",
        "stairs",
        "portals",
        "traps",
        "closets",
        "flags",
        "illumination",
        "transportation",
        "signs",
        "blobs",
        "event creatures",
        "casks",
        "skeletons",
        "flora and minerals",
        "bushes",
        "coffins",
        "torture instruments",
        "cactuses",
        "dragons",
        "bats",
        "demons",
        "arachnids",
        "ferns",
        "metals",
        "ungulates",
        "bears",
        "dreamhaunters",
        "hive born",
        "machines",
        "magicfield",
        "outlaws",
        "annelids",
        "astral shapers",
        "birds",
        "canines",
        "ghosts",
        "glires",
        "ladders",
        "mollusks",
    }
)

WORLD_TYPES = frozenset(
    {
        "door",
        "bed",
        "carpet",
        "depot",
        "teleport",
        "magicfield",
        "trashholder",
        "ladder",
        "dummy",
        "mailbox",
        "rewardchest",
    }
)

WORLD_KEYS = frozenset(
    {
        "bedpart",
        "partnerdirection",
        "bedpartof",
        "floorchange",
        "leveldoor",
        "usedbyhouseguests",
        "blocking",
        "blockprojectile",
        "walkstack",
        "replaceable",
        "field",
    }
)

STRONG_PLAYER_KEYS = frozenset(
    {
        "weapontype",
        "ammotype",
        "attack",
        "defense",
        "extradef",
        "armor",
        "charges",
        "imbuementslot",
        "slottype",
        "slot",
        "vocation",
        "level",
        "runespellname",
        "mantra",
        "elementalbond",
    }
)

TRANSFORM_KEYS = frozenset(
    {
        "wrapableto",
        "rotateto",
        "transformonuse",
        "transformequipto",
        "transformdeequipto",
        "maletransformto",
        "femaletransformto",
        "destroyto",
    }
)

SIMPLE_PRESENTATION_KEYS = frozenset(
    {
        "description",
        "weight",
        "primarytype",
        "showcount",
        "showduration",
        "showattributes",
        "showcharges",
        "loottype",
    }
)

TECHNICAL_NAME = re.compile(
    r"^(reserved sprite|unknown(?: item(?: \([^)]*\))?)?|unknow(?: item)?|"
    r"old tibia item|deprecated item|empty sprite)$"
)

SAMPLE_LIMIT = 12


class CensusError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def normalize(value: str | None) -> str:
    return " ".join((value or "").strip().casefold().split())


def git(source_root: Path, *args: str) -> str:
    result = subprocess.run(
        ("git", "-C", str(source_root), *args),
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout.decode("utf-8").strip()


def normalize_remote(url: str) -> str:
    value = url.strip().casefold().replace("\\", "/")
    if value.endswith(".git"):
        value = value[:-4]
    if value.startswith("git@github.com:"):
        value = "https://github.com/" + value.split(":", 1)[1]
    return value.rstrip("/")


def read_source(source_root: Path) -> bytes:
    root = source_root.resolve()
    try:
        top = Path(git(root, "rev-parse", "--show-toplevel")).resolve()
        head = git(root, "rev-parse", "HEAD")
        remote = normalize_remote(git(root, "remote", "get-url", "origin"))
        blob = git(root, "rev-parse", f"HEAD:{SOURCE_PATH}")
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CensusError("SOURCE_REPOSITORY_UNVERIFIABLE") from exc
    if top != root:
        raise CensusError("SOURCE_REPOSITORY_ROOT_MISMATCH")
    if head != SOURCE_REVISION:
        raise CensusError(f"SOURCE_REVISION_MISMATCH:{head}")
    if remote != f"https://github.com/{SOURCE_REPOSITORY}".casefold():
        raise CensusError(f"SOURCE_REMOTE_MISMATCH:{remote}")
    if blob != SOURCE_BLOB:
        raise CensusError(f"SOURCE_BLOB_MISMATCH:{blob}")

    path = root / SOURCE_PATH
    payload = path.read_bytes()
    if len(payload) != SOURCE_BYTES:
        raise CensusError(f"SOURCE_SIZE_MISMATCH:{len(payload)}")
    digest = sha256_bytes(payload)
    if digest != SOURCE_SHA256:
        raise CensusError(f"SOURCE_SHA256_MISMATCH:{digest}")
    return payload


def field_map(node: ET.Element) -> dict[str, list[str]]:
    values: dict[str, list[str]] = defaultdict(list)
    for child in node.findall("attribute"):
        key = normalize(child.attrib.get("key"))
        if not key:
            continue
        values[key].append(child.attrib.get("value", ""))
    return dict(values)


def expanded_identity_count(node: ET.Element) -> tuple[int | None, dict[str, Any] | None]:
    if "id" in node.attrib:
        try:
            value = int(node.attrib["id"])
        except ValueError as exc:
            raise CensusError("SOURCE_ID_NOT_INTEGER") from exc
        return 1, None
    if "fromid" not in node.attrib or "toid" not in node.attrib:
        raise CensusError("SOURCE_NODE_WITHOUT_COMPLETE_IDENTITY")
    try:
        start = int(node.attrib["fromid"])
        end = int(node.attrib["toid"])
    except ValueError as exc:
        raise CensusError("SOURCE_RANGE_NOT_INTEGER") from exc
    if start > end:
        return None, {
            "name": node.attrib.get("name", ""),
            "fromid": start,
            "toid": end,
        }
    return end - start + 1, None


def classify(node: ET.Element, fields: dict[str, list[str]]) -> tuple[str, str]:
    name = normalize(node.attrib.get("name"))
    if TECHNICAL_NAME.fullmatch(name):
        return "TECHNICAL_PLACEHOLDER", "placeholder_name"

    keys = set(fields)
    primary_type = normalize(fields.get("primarytype", [""])[0])
    item_type = normalize(fields.get("type", [""])[0])

    if primary_type in WORLD_PRIMARY_TYPES:
        return "WORLD_OBJECT_INTERACTION", f"primarytype:{primary_type}"
    if item_type in WORLD_TYPES:
        return "WORLD_OBJECT_INTERACTION", f"type:{item_type}"
    if keys & WORLD_KEYS:
        return "WORLD_OBJECT_INTERACTION", "world_field"

    if primary_type in PLAYER_PRIMARY_TYPES:
        return "PLAYER_CATALOG_STRONG", f"primarytype:{primary_type}"
    if keys & STRONG_PLAYER_KEYS:
        return "PLAYER_CATALOG_STRONG", "strong_gameplay_field"

    if {"decayto", "duration"} <= keys:
        return "STATE_VARIANT_DECAY", "decay_duration"
    if keys & TRANSFORM_KEYS:
        return "STATE_VARIANT_TRANSFORM", "transform_relation"

    if not keys and "fromid" in node.attrib:
        return "RANGE_VARIANT_NO_FIELDS", "range_no_fields"
    if not keys:
        return "DIRECT_SIMPLE_NO_FIELDS", "direct_no_fields"
    if keys <= SIMPLE_PRESENTATION_KEYS:
        return "SIMPLE_PRESENTATION_CANDIDATE", "simple_presentation"
    return "RESIDUAL_OTHER", "other"


def source_locator(node: ET.Element) -> dict[str, Any]:
    if "id" in node.attrib:
        return {"id": int(node.attrib["id"])}
    return {
        "fromid": int(node.attrib["fromid"]),
        "toid": int(node.attrib["toid"]),
    }


def build_census(payload: bytes) -> dict[str, Any]:
    try:
        root = ET.fromstring(payload)
    except ET.ParseError as exc:
        raise CensusError("SOURCE_XML_INVALID") from exc
    if root.tag != "items":
        raise CensusError("SOURCE_XML_ROOT_INVALID")

    bucket_counts = {
        bucket: {
            "definition_nodes": 0,
            "expanded_source_ids": 0,
            "direct_nodes": 0,
            "range_nodes": 0,
        }
        for bucket in BUCKETS
    }
    bucket_names: dict[str, set[str]] = {bucket: set() for bucket in BUCKETS}
    reason_counts: Counter[str] = Counter()
    primary_type_nodes: Counter[str] = Counter()
    primary_type_ids: Counter[str] = Counter()
    name_to_definitions: dict[str, int] = defaultdict(int)
    name_to_ids: dict[str, int] = defaultdict(int)
    samples: dict[str, list[dict[str, Any]]] = {bucket: [] for bucket in BUCKETS}
    exclusions: list[dict[str, Any]] = []

    source_nodes = 0
    direct_nodes = 0
    range_nodes = 0
    admitted_nodes = 0
    admitted_direct_nodes = 0
    admitted_range_nodes = 0
    expanded_ids = 0
    range_member_ids = 0

    for node in root.findall("item"):
        source_nodes += 1
        is_direct = "id" in node.attrib
        if is_direct:
            direct_nodes += 1
        else:
            range_nodes += 1

        identity_count, exclusion = expanded_identity_count(node)
        if exclusion is not None:
            exclusions.append(exclusion)
            continue
        assert identity_count is not None
        admitted_nodes += 1
        expanded_ids += identity_count
        if is_direct:
            admitted_direct_nodes += 1
        else:
            admitted_range_nodes += 1
            range_member_ids += identity_count

        fields = field_map(node)
        bucket, reason = classify(node, fields)
        counts = bucket_counts[bucket]
        counts["definition_nodes"] += 1
        counts["expanded_source_ids"] += identity_count
        counts["direct_nodes" if is_direct else "range_nodes"] += 1
        reason_counts[f"{bucket}:{reason}"] += 1

        name = normalize(node.attrib.get("name"))
        bucket_names[bucket].add(name)
        name_to_definitions[name] += 1
        name_to_ids[name] += identity_count

        primary_type = normalize(fields.get("primarytype", [""])[0]) or "<none>"
        primary_type_nodes[primary_type] += 1
        primary_type_ids[primary_type] += identity_count

        if len(samples[bucket]) < SAMPLE_LIMIT:
            samples[bucket].append(
                {
                    "name": node.attrib.get("name", ""),
                    "source": source_locator(node),
                    "expanded_source_ids": identity_count,
                    "reason": reason,
                }
            )

    if source_nodes != direct_nodes + range_nodes:
        raise CensusError("SOURCE_NODE_PARTITION_FAILED")
    if admitted_nodes + len(exclusions) != source_nodes:
        raise CensusError("ADMITTED_NODE_PARTITION_FAILED")
    if sum(v["definition_nodes"] for v in bucket_counts.values()) != admitted_nodes:
        raise CensusError("BUCKET_DEFINITION_PARTITION_FAILED")
    if sum(v["expanded_source_ids"] for v in bucket_counts.values()) != expanded_ids:
        raise CensusError("BUCKET_ID_PARTITION_FAILED")

    for bucket in BUCKETS:
        bucket_counts[bucket]["distinct_normalized_names"] = len(bucket_names[bucket])

    obvious_non_catalog_buckets = (
        "WORLD_OBJECT_INTERACTION",
        "TECHNICAL_PLACEHOLDER",
        "STATE_VARIANT_DECAY",
        "STATE_VARIANT_TRANSFORM",
        "RANGE_VARIANT_NO_FIELDS",
    )
    unresolved_catalog_buckets = (
        "DIRECT_SIMPLE_NO_FIELDS",
        "SIMPLE_PRESENTATION_CANDIDATE",
        "RESIDUAL_OTHER",
    )
    obvious_non_catalog_definitions = sum(
        bucket_counts[b]["definition_nodes"] for b in obvious_non_catalog_buckets
    )
    obvious_non_catalog_ids = sum(
        bucket_counts[b]["expanded_source_ids"] for b in obvious_non_catalog_buckets
    )
    unresolved_catalog_definitions = sum(
        bucket_counts[b]["definition_nodes"] for b in unresolved_catalog_buckets
    )
    unresolved_catalog_ids = sum(
        bucket_counts[b]["expanded_source_ids"] for b in unresolved_catalog_buckets
    )
    strong_player_definitions = bucket_counts["PLAYER_CATALOG_STRONG"]["definition_nodes"]
    strong_player_ids = bucket_counts["PLAYER_CATALOG_STRONG"]["expanded_source_ids"]

    return {
        "schema": SCHEMA,
        "status": "SOURCE_ROLE_CENSUS_NO_SEMANTIC_PROMOTION",
        "source": {
            "repository": SOURCE_REPOSITORY,
            "revision": SOURCE_REVISION,
            "path": SOURCE_PATH,
            "blob": SOURCE_BLOB,
            "bytes": SOURCE_BYTES,
            "sha256": SOURCE_SHA256,
            "classification": "CRYSTAL_OTS / OTS_HYPOTHESIS_ONLY",
        },
        "counts": {
            "source_xml_item_nodes": source_nodes,
            "source_direct_id_nodes": direct_nodes,
            "source_range_nodes": range_nodes,
            "reversed_ranges_excluded": len(exclusions),
            "admitted_definition_nodes": admitted_nodes,
            "admitted_direct_definition_nodes": admitted_direct_nodes,
            "admitted_range_definition_nodes": admitted_range_nodes,
            "expanded_source_ids": expanded_ids,
            "range_member_source_ids": range_member_ids,
            "range_expansion_extra_ids_over_definitions": expanded_ids - admitted_nodes,
            "distinct_normalized_names": len(name_to_definitions),
            "definition_name_collision_groups": sum(
                count > 1 for count in name_to_definitions.values()
            ),
            "expanded_id_name_collision_groups": sum(
                count > 1 for count in name_to_ids.values()
            ),
        },
        "role_buckets": bucket_counts,
        "catalog_denominator": {
            "strong_player_definition_lower_bound": strong_player_definitions,
            "strong_player_expanded_ids": strong_player_ids,
            "unresolved_direct_or_simple_definitions": unresolved_catalog_definitions,
            "unresolved_direct_or_simple_expanded_ids": unresolved_catalog_ids,
            "strong_plus_unresolved_definition_band_upper": (
                strong_player_definitions + unresolved_catalog_definitions
            ),
            "strong_plus_unresolved_expanded_id_band_upper": (
                strong_player_ids + unresolved_catalog_ids
            ),
            "obvious_world_technical_state_or_fieldless_range_definitions": (
                obvious_non_catalog_definitions
            ),
            "obvious_world_technical_state_or_fieldless_range_ids": (
                obvious_non_catalog_ids
            ),
            "note": (
                "The band is a source-role scaling estimate, not Reference truth. "
                "Fieldless range variants may still be player-visible presentation variants; "
                "mixed residuals remain deliberately unforced."
            ),
        },
        "reversed_range_exclusions": sorted(
            exclusions,
            key=lambda value: (value["fromid"], value["toid"], value["name"]),
        ),
        "reason_counts": dict(sorted(reason_counts.items())),
        "primary_type_top_by_definition_nodes": [
            {"primary_type": key, "definition_nodes": count}
            for key, count in primary_type_nodes.most_common(40)
        ],
        "primary_type_top_by_expanded_ids": [
            {"primary_type": key, "expanded_source_ids": count}
            for key, count in primary_type_ids.most_common(40)
        ],
        "samples": samples,
        "invariants": {
            "all_admitted_definitions_partitioned_once": True,
            "all_expanded_source_ids_partitioned_once": True,
            "name_alone_grants_player_catalog_status": False,
            "source_role_census_is_gameplay_truth": False,
            "identity_remapping_performed": False,
            "semantic_promotion_performed": False,
            "runtime_or_client_mutation_performed": False,
        },
        "limitations": [
            "This census classifies source-role signals, not official Tibia catalogue membership.",
            "Player/catalog status requires a strong source taxonomy or gameplay-field signal; mixed cases remain residual.",
            "World/interactable includes map/world objects that may still be represented by ItemType in the donor engine.",
            "Range members are counted as source IDs but are not automatically treated as independent semantic definitions.",
            "Appearance/sprite equivalence is outside this census.",
        ],
        "next_action": (
            "Use admitted definition nodes, not 38,157 expanded source IDs, as the first bulk-import denominator; "
            "resolve residual/simple candidates and appearance variants only where they affect a real player catalogue batch."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    payload = read_source(args.source_root)
    census = build_census(payload)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(census))

    counts = census["counts"]
    denom = census["catalog_denominator"]
    print(
        "item-source-role-census: PASS "
        f"xml_nodes={counts['source_xml_item_nodes']} "
        f"definitions={counts['admitted_definition_nodes']} "
        f"expanded_ids={counts['expanded_source_ids']} "
        f"strong_player_defs={denom['strong_player_definition_lower_bound']} "
        f"catalog_band_upper={denom['strong_plus_unresolved_definition_band_upper']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
