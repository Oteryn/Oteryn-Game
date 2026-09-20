#!/usr/bin/env python3
"""Deterministic CW2-B6 static spatial interaction candidate catalogue.

This generator reuses the protected Game full-world producer plus the protected
corridor census structural-observation seam. It never parses OTBM itself, never
executes Lua, and never promotes source-local IDs or coordinates to canonical
Oteryn interaction/runtime authority.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from typing import Any, Iterable

SCHEMA = "OTERYN_CW2_B6_INTERACTION_BINDING_CATALOG/v1"
FAMILY_SCHEMA = "OTERYN_CW2_B6_INTERACTION_BINDING_FAMILY/v1"
MAPPER_PROFILE = "OTERYN_CW2_B6_INTERACTION_BINDING_MAPPER/v1"
TASK = "CW2-B6 STATIC_SPATIAL_INTERACTION_BINDING_CATALOGUE_504"
ADMISSION_MAIN = "f4f1292544b1fffbb09e9df6ebefd5c8bb1f7879"

SOURCE_REPOSITORY = "zimbadev/crystalserver"
SOURCE_REVISION = "ff7ede593c69d4c658b382c97443e8155926924a"
SOURCE_MAP_PATH = "data-global/world/world.otbm"
SOURCE_MAP_BYTES = 52_267_895
SOURCE_MAP_BLOB = "e95e8f7c7a95d1b634b49a5dea5a5dc76021406b"
SOURCE_MAP_SHA256 = "09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb"
SOURCE_PROFILE = "oteryn-crystalserver-fresh-source-generation-v2"
SOURCE_PROFILE_REVISION = 2

PARSER_REPOSITORY = "blakinio/Otheryn"
PARSER_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
PARSER_BLOBS = {
    "tools/otbm_atlas/__init__.py": "047d1274022e1d2a71d6aa23d6efcf420a24535d",
    "tools/otbm_atlas/assets.py": "25ed2400813bb3ccdc54482967ed05197eb1a850",
    "tools/otbm_atlas/semantic.py": "a11343a472145aee4d9cf65c6ce28b3e4a71a2b3",
    "tools/otbm_atlas/nodefile.py": "bed6f7a803d9de485c1f03cbdca4be0cb1521d30",
}

ASSET_ZIP_SHA256 = "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f"
ASSET_CATALOG_SHA256 = "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85"
ASSET_APPEARANCE_SHA256 = "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075"

GAME_INPUT_BLOBS = {
    "tools/game-atlas-fullworld-source/producer.py": "740a96fe1b1d97f32c56e278725ab9c2de6e724b",
    "tools/reference-world-corridor-census/census.py": "f056063ad1ae1476fcb73d97e918553a977a0a15",
}
EXPECTED_STREAM_COUNTS = {"map_header": 1, "tile": 18_997_668, "town": 33, "waypoint": 18}
FAMILIES = ("ACTION_ID", "UNIQUE_ID", "TELEPORT_DESTINATION", "HOUSE_DOOR_ID")
FAMILY_FILENAMES = {
    "ACTION_ID": "action-id.json",
    "UNIQUE_ID": "unique-id.json",
    "TELEPORT_DESTINATION": "teleport-destination.json",
    "HOUSE_DOOR_ID": "house-door-id.json",
}
DISPOSITIONS = ("UNKNOWN", "UNSUPPORTED", "AMBIGUOUS", "CONFLICT", "LOSS")
EVIDENCE_STATUS = "OTS_HYPOTHESIS_ONLY"
CLOSURE = "CANDIDATE_ONLY"
INDEX_NAME = "OTV2-20260920-content-world-cw2-b6-interaction-bindings.json"
FAMILY_DIR_NAME = "OTV2-20260920-content-world-cw2-b6-interaction-bindings"
FINAL_INDEX_PATH = f"docs/agents/evidence/{INDEX_NAME}"
FINAL_FAMILY_PREFIX = f"docs/agents/evidence/{FAMILY_DIR_NAME}"

REASON_CODES = {
    "ACTION_ID": "SOURCE_LOCAL_ACTION_ID_NOT_CANONICAL_INTERACTION_IDENTITY",
    "UNIQUE_ID": "SOURCE_LOCAL_UNIQUE_ID_NOT_CANONICAL_INTERACTION_IDENTITY",
    "TELEPORT_DESTINATION": "SOURCE_TELEPORT_DESTINATION_NOT_RUNTIME_OR_TARGET_AUTHORITY",
    "HOUSE_DOOR_ID": "SOURCE_LOCAL_HOUSE_DOOR_ID_NOT_HOUSE_ACL_OR_RUNTIME_AUTHORITY",
}


class CatalogError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def stable_id(kind: str, value: Any) -> str:
    digest = sha256_bytes(canonical_bytes({"kind": kind, "value": value}))
    return f"{kind.lower()}:{digest[:32]}"


def _git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ("git", "-C", str(repo), *args),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "git command failed"
        raise CatalogError(f"GIT_VALIDATION_FAILED: {detail}")
    return completed.stdout.strip()


def _normalize_remote(url: str) -> str:
    value = url.strip().lower().replace("\\", "/")
    if value.endswith(".git"):
        value = value[:-4]
    if value.startswith("git@github.com:"):
        value = "https://github.com/" + value.split(":", 1)[1]
    return value.rstrip("/")


def verify_repository(repo: Path, expected_repository: str, expected_revision: str) -> None:
    repo = repo.resolve()
    if Path(_git(repo, "rev-parse", "--show-toplevel")).resolve() != repo:
        raise CatalogError("REPOSITORY_ROOT_MISMATCH")
    if _git(repo, "rev-parse", "HEAD") != expected_revision:
        raise CatalogError(f"REPOSITORY_REVISION_MISMATCH: {expected_repository}")
    remote = _normalize_remote(_git(repo, "remote", "get-url", "origin"))
    if remote != f"https://github.com/{expected_repository}".lower():
        raise CatalogError(f"REPOSITORY_REMOTE_MISMATCH: {expected_repository}")
    if _git(repo, "status", "--porcelain=v1", "--untracked-files=all"):
        raise CatalogError(f"REPOSITORY_DIRTY: {expected_repository}")


def verify_game_inputs(game_root: Path) -> None:
    game_root = game_root.resolve()
    if Path(_git(game_root, "rev-parse", "--show-toplevel")).resolve() != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if _git(game_root, "rev-parse", ADMISSION_MAIN) != ADMISSION_MAIN:
        raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")
    for path, expected_blob in GAME_INPUT_BLOBS.items():
        actual = _git(game_root, "rev-parse", f"HEAD:{path}")
        if actual != expected_blob:
            raise CatalogError(f"PROTECTED_GAME_INPUT_BLOB_MISMATCH: {path}")


def verify_parser_pins(legacy_root: Path) -> None:
    verify_repository(legacy_root, PARSER_REPOSITORY, PARSER_REVISION)
    for path, expected_blob in PARSER_BLOBS.items():
        actual = _git(legacy_root, "rev-parse", f"{PARSER_REVISION}:{path}")
        if actual != expected_blob:
            raise CatalogError(f"PARSER_BLOB_MISMATCH: {path}")


def _load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CatalogError(f"MODULE_LOAD_FAILED: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def normalize_observation(observation: dict[str, Any]) -> dict[str, Any]:
    family = observation.get("structural_kind")
    if family not in FAMILIES:
        raise CatalogError(f"UNSUPPORTED_STRUCTURAL_FAMILY: {family!r}")
    position = observation.get("position")
    if not isinstance(position, dict) or set(position) != {"floor", "x", "y"}:
        raise CatalogError("MALFORMED_POSITION")
    for key in ("floor", "x", "y"):
        if not isinstance(position[key], int):
            raise CatalogError("MALFORMED_POSITION")
    item_order = observation.get("item_order")
    source_item_id = observation.get("source_item_id")
    if not isinstance(item_order, int) or item_order < 0:
        raise CatalogError("MALFORMED_ITEM_ORDER")
    if not isinstance(source_item_id, int) or source_item_id < 0:
        raise CatalogError("MALFORMED_SOURCE_ITEM_ID")
    source_value = observation.get("source_value")
    if family == "TELEPORT_DESTINATION":
        if not isinstance(source_value, dict) or set(source_value) != {"floor", "x", "y"}:
            raise CatalogError("MALFORMED_TELEPORT_DESTINATION")
        if not all(isinstance(source_value[key], int) for key in ("floor", "x", "y")):
            raise CatalogError("MALFORMED_TELEPORT_DESTINATION")
    elif not isinstance(source_value, int):
        raise CatalogError("MALFORMED_SOURCE_VALUE")

    identity_basis = {
        "family": family,
        "position": {"floor": int(position["floor"]), "x": int(position["x"]), "y": int(position["y"])},
        "item_order": item_order,
        "source_item_id": source_item_id,
        "source_value": source_value,
    }
    return {
        "record_id": stable_id("interaction-binding", identity_basis),
        **identity_basis,
        "source_identity_class": "LEGACY_STRUCTURAL_BINDING",
        "candidate_semantics": "SOURCE_STRUCTURAL_BINDING_ONLY",
        "evidence_status": EVIDENCE_STATUS,
        "native_binding_disposition": "UNRESOLVED",
        "semantic_disposition": "UNKNOWN",
        "reason_codes": [REASON_CODES[family]],
        "executable_eligible": False,
        "reference_parity_claim": "NONE",
        "runtime_authority": "NONE",
    }


def _record_sort_key(record: dict[str, Any]) -> tuple[Any, ...]:
    return (
        record["position"]["floor"],
        record["position"]["y"],
        record["position"]["x"],
        record["family"],
        record["item_order"],
        record["source_item_id"],
        json.dumps(record["source_value"], ensure_ascii=False, sort_keys=True, separators=(",", ":")),
        record["record_id"],
    )


def _source_value_key(record: dict[str, Any]) -> str:
    return json.dumps(record["source_value"], ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def build_catalog(observations: Iterable[dict[str, Any]], source_stream_counts: dict[str, int]) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    normalized = [normalize_observation(value) for value in observations]
    normalized.sort(key=_record_sort_key)

    seen_ids: set[str] = set()
    for record in normalized:
        record_id = str(record["record_id"])
        if record_id in seen_ids:
            raise CatalogError(f"DUPLICATE_RECORD_ID: {record_id}")
        seen_ids.add(record_id)

    family_docs: dict[str, dict[str, Any]] = {}
    product_families: dict[str, Any] = {}
    total_dispositions = Counter({name: 0 for name in DISPOSITIONS})

    for family in FAMILIES:
        records = [record for record in normalized if record["family"] == family]
        value_counts = Counter(_source_value_key(record) for record in records)
        disposition_counts = Counter({name: 0 for name in DISPOSITIONS})
        disposition_counts.update(record["semantic_disposition"] for record in records)
        total_dispositions.update(record["semantic_disposition"] for record in records)
        records_digest = sha256_bytes(canonical_bytes(records))
        counts = {
            "occurrences": len(records),
            "unique_source_values": len(value_counts),
            "source_value_reuse_groups": sum(1 for count in value_counts.values() if count > 1),
            "max_source_value_reuse": max(value_counts.values(), default=0),
            "native_binding_resolved": 0,
            "native_binding_unresolved": len(records),
            "executable_eligible": 0,
            "dispositions": {name: int(disposition_counts[name]) for name in DISPOSITIONS},
        }
        family_docs[family] = {
            "schema": FAMILY_SCHEMA,
            "mapper_profile": MAPPER_PROFILE,
            "family": family,
            "closure": CLOSURE,
            "source": {
                "repository": SOURCE_REPOSITORY,
                "revision": SOURCE_REVISION,
                "path": SOURCE_MAP_PATH,
                "bytes": SOURCE_MAP_BYTES,
                "git_blob": SOURCE_MAP_BLOB,
                "sha256": SOURCE_MAP_SHA256,
                "profile": SOURCE_PROFILE,
                "profile_revision": SOURCE_PROFILE_REVISION,
            },
            "evidence_status": EVIDENCE_STATUS,
            "counts": counts,
            "records_digest_sha256": records_digest,
            "records": records,
        }
        product_families[family] = {
            "counts": counts,
            "records_digest_sha256": records_digest,
        }

    product_basis = {
        "schema": SCHEMA,
        "mapper_profile": MAPPER_PROFILE,
        "task": TASK,
        "closure": CLOSURE,
        "source": {
            "repository": SOURCE_REPOSITORY,
            "revision": SOURCE_REVISION,
            "path": SOURCE_MAP_PATH,
            "bytes": SOURCE_MAP_BYTES,
            "git_blob": SOURCE_MAP_BLOB,
            "sha256": SOURCE_MAP_SHA256,
            "profile": SOURCE_PROFILE,
            "profile_revision": SOURCE_PROFILE_REVISION,
        },
        "parser": {
            "repository": PARSER_REPOSITORY,
            "revision": PARSER_REVISION,
            "blobs": PARSER_BLOBS,
        },
        "game_inputs": GAME_INPUT_BLOBS,
        "source_stream_counts": source_stream_counts,
        "families": product_families,
        "total_occurrences": len(normalized),
        "dispositions": {name: int(total_dispositions[name]) for name in DISPOSITIONS},
        "silent_drop": 0,
        "unclassified": 0,
        "native_promotions": 0,
        "executable_promotions": 0,
        "quest_runtime_promotions": 0,
    }
    product_digest = sha256_bytes(canonical_bytes(product_basis))

    files = []
    for family in FAMILIES:
        family_docs[family]["logical_product_digest_sha256"] = product_digest
        payload = canonical_bytes(family_docs[family])
        files.append(
            {
                "family": family,
                "tracked_path": f"{FINAL_FAMILY_PREFIX}/{FAMILY_FILENAMES[family]}",
                "bytes": len(payload),
                "sha256": sha256_bytes(payload),
                "records": family_docs[family]["counts"]["occurrences"],
                "records_digest_sha256": family_docs[family]["records_digest_sha256"],
            }
        )

    index = {
        **product_basis,
        "logical_product_digest_sha256": product_digest,
        "storage": {
            "index_tracked_path": FINAL_INDEX_PATH,
            "family_files": files,
        },
        "authority": {
            "reference_parity_claim": "NONE",
            "native_identity_promotion": "NONE",
            "runtime_authority": "NONE",
            "quest_runtime_authority": "NONE",
            "durable_value_authority": "NONE",
            "lua_execution": False,
            "storage_integer_promotion": False,
        },
    }
    return index, family_docs


def validate_real_source_counts(counts: dict[str, int]) -> None:
    if counts != EXPECTED_STREAM_COUNTS:
        raise CatalogError(f"SOURCE_STREAM_COUNT_MISMATCH: expected {EXPECTED_STREAM_COUNTS}, got {counts}")


def write_catalog(output_dir: Path, index: dict[str, Any], family_docs: dict[str, dict[str, Any]]) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    family_dir = output_dir / FAMILY_DIR_NAME
    family_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / INDEX_NAME).write_bytes(canonical_bytes(index))
    for family in FAMILIES:
        (family_dir / FAMILY_FILENAMES[family]).write_bytes(canonical_bytes(family_docs[family]))


def collect_real_observations(
    *,
    game_root: Path,
    legacy_root: Path,
    source_root: Path,
    asset_zip: Path,
    assets_dir: Path,
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    verify_game_inputs(game_root)
    verify_parser_pins(legacy_root)
    verify_repository(source_root, SOURCE_REPOSITORY, SOURCE_REVISION)

    map_path = source_root / SOURCE_MAP_PATH
    if not map_path.is_file():
        raise CatalogError(f"SOURCE_MAP_MISSING: {map_path}")

    producer = _load_module(
        game_root / "tools/game-atlas-fullworld-source/producer.py",
        "cw2_b6_fullworld_producer",
    )
    census = _load_module(
        game_root / "tools/reference-world-corridor-census/census.py",
        "cw2_b6_corridor_census",
    )
    runtime = producer.load_runtime(
        legacy_root=legacy_root,
        map_path=map_path,
        asset_zip=asset_zip,
        assets_dir=assets_dir,
        source_generation_profile_id=SOURCE_PROFILE,
    )

    counts = {"map_header": 0, "tile": 0, "town": 0, "waypoint": 0}
    observations: list[dict[str, Any]] = []
    for record in producer.iter_records(runtime, strict=True):
        if producer.is_map_header(runtime, record):
            counts["map_header"] += 1
        elif producer.is_tile(runtime, record):
            counts["tile"] += 1
            observations.extend(census._structural_observations(producer, record))
        elif producer.is_town(runtime, record):
            counts["town"] += 1
        elif producer.is_waypoint(runtime, record):
            counts["waypoint"] += 1
        else:
            raise CatalogError(f"UNEXPECTED_SOURCE_RECORD: {type(record)!r}")
    validate_real_source_counts(counts)
    return observations, counts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--legacy-root", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--asset-zip", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--enumeration", choices=("normal", "reverse"), default="normal")
    args = parser.parse_args()

    observations, counts = collect_real_observations(
        game_root=args.game_root.resolve(),
        legacy_root=args.legacy_root.resolve(),
        source_root=args.source_root.resolve(),
        asset_zip=args.asset_zip.resolve(),
        assets_dir=args.assets.resolve(),
    )
    if args.enumeration == "reverse":
        observations.reverse()
    index, family_docs = build_catalog(observations, counts)
    write_catalog(args.output.resolve(), index, family_docs)
    print(
        json.dumps(
            {
                "logical_product_digest_sha256": index["logical_product_digest_sha256"],
                "source_stream_counts": counts,
                "total_occurrences": index["total_occurrences"],
                "family_counts": {
                    family: family_docs[family]["counts"]["occurrences"] for family in FAMILIES
                },
                "dispositions": index["dispositions"],
                "silent_drop": index["silent_drop"],
                "unclassified": index["unclassified"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
