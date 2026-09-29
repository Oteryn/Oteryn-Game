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
from contextlib import contextmanager
import hashlib
import importlib.util
import json
from pathlib import Path
import stat
import subprocess
import sys
from typing import Any, Iterable

SCHEMA = "OTERYN_CW2_B6_INTERACTION_BINDING_CATALOG/v1"
FAMILY_SCHEMA = "OTERYN_CW2_B6_INTERACTION_BINDING_FAMILY/v1"
FAMILY_MANIFEST_SCHEMA = "OTERYN_CW2_B6_INTERACTION_BINDING_FAMILY_MANIFEST/v1"
SHARD_SCHEMA = "OTERYN_CW2_B6_INTERACTION_BINDING_SHARD/v1"
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
    "tools/game-atlas-thais-fixture/export.py": "0e36ca9e43d78a919eb673f9dc423ed1c178c727",
}
EXPECTED_STREAM_COUNTS = {"map_header": 1, "tile": 18_997_668, "town": 33, "waypoint": 18}
EXPECTED_FAMILY_COUNTS = {
    "ACTION_ID": 0,
    "UNIQUE_ID": 0,
    "TELEPORT_DESTINATION": 2_405,
    "HOUSE_DOOR_ID": 4_527,
}
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
MAX_SHARD_BYTES = 600_000
SHARD_COUNTS = {"TELEPORT_DESTINATION": 3, "HOUSE_DOOR_ID": 5}

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


def _git_blob_sha1_file(path: Path) -> str:
    payload = path.read_bytes()
    digest = hashlib.sha1(usedforsecurity=False)
    digest.update(f"blob {len(payload)}\0".encode("ascii"))
    digest.update(payload)
    return digest.hexdigest()


def _reject_game_input_bytecode(path: Path, relative_path: str) -> None:
    active_cache = Path(importlib.util.cache_from_source(str(path)))
    for candidate in (
        active_cache,
        path.with_suffix(".pyc"),
        path.with_suffix(".pyo"),
    ):
        try:
            candidate.lstat()
        except FileNotFoundError:
            continue
        raise CatalogError(
            f"PROTECTED_GAME_INPUT_BYTECODE_CACHE: {relative_path}: {candidate}"
        )

    cache_root = path.parent / "__pycache__"
    try:
        cache_metadata = cache_root.lstat()
    except FileNotFoundError:
        return
    if stat.S_ISLNK(cache_metadata.st_mode) or not stat.S_ISDIR(cache_metadata.st_mode):
        raise CatalogError(
            f"PROTECTED_GAME_INPUT_BYTECODE_CACHE_NAMESPACE: {relative_path}"
        )

    prefix = f"{path.stem}."
    for candidate in cache_root.iterdir():
        if candidate.name.startswith(prefix) and candidate.suffix in {".pyc", ".pyo"}:
            raise CatalogError(
                f"PROTECTED_GAME_INPUT_BYTECODE_CACHE: {relative_path}: {candidate}"
            )


@contextmanager
def _without_bytecode_writes():
    previous = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        yield
    finally:
        sys.dont_write_bytecode = previous


def _regular_game_input_path(game_root: Path, relative_path: str) -> Path:
    current = game_root
    parts = Path(relative_path).parts
    if not parts:
        raise CatalogError("PROTECTED_GAME_INPUT_EMPTY_PATH")

    for part in parts:
        current = current / part
        try:
            metadata = current.lstat()
        except FileNotFoundError as exc:
            raise CatalogError(f"PROTECTED_GAME_INPUT_MISSING: {relative_path}") from exc
        if stat.S_ISLNK(metadata.st_mode):
            raise CatalogError(f"PROTECTED_GAME_INPUT_SYMLINK: {relative_path}")

    if not stat.S_ISREG(metadata.st_mode):
        raise CatalogError(f"PROTECTED_GAME_INPUT_NOT_REGULAR: {relative_path}")
    if metadata.st_nlink != 1:
        raise CatalogError(f"PROTECTED_GAME_INPUT_HARDLINK: {relative_path}")
    return current


def _verify_game_input_file(game_root: Path, relative_path: str, expected_blob: str) -> Path:
    path = _regular_game_input_path(game_root, relative_path)
    _reject_game_input_bytecode(path, relative_path)

    committed_blob = _git(game_root, "rev-parse", f"HEAD:{relative_path}")
    if committed_blob != expected_blob:
        raise CatalogError(f"PROTECTED_GAME_INPUT_BLOB_MISMATCH: {relative_path}")

    dirty = _git(
        game_root,
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        "--",
        relative_path,
    )
    if dirty:
        raise CatalogError(f"PROTECTED_GAME_INPUT_DIRTY: {relative_path}")

    working_blob = _git_blob_sha1_file(path)
    if working_blob != expected_blob:
        raise CatalogError(f"PROTECTED_GAME_INPUT_WORKTREE_BLOB_MISMATCH: {relative_path}")
    return path


def verify_game_inputs(game_root: Path) -> dict[str, Path]:
    game_root = game_root.resolve()
    if Path(_git(game_root, "rev-parse", "--show-toplevel")).resolve() != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if _git(game_root, "rev-parse", ADMISSION_MAIN) != ADMISSION_MAIN:
        raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")

    return {
        relative_path: _verify_game_input_file(game_root, relative_path, expected_blob)
        for relative_path, expected_blob in GAME_INPUT_BLOBS.items()
    }


def verify_parser_pins(legacy_root: Path) -> None:
    verify_repository(legacy_root, PARSER_REPOSITORY, PARSER_REVISION)
    for path, expected_blob in PARSER_BLOBS.items():
        actual = _git(legacy_root, "rev-parse", f"{PARSER_REVISION}:{path}")
        if actual != expected_blob:
            raise CatalogError(f"PARSER_BLOB_MISMATCH: {path}")
    _reject_parser_active_bytecode_caches(legacy_root)


def _reject_parser_active_bytecode_caches(legacy_root: Path) -> None:
    """Reject configured-prefix caches that the protected parser import can read."""
    for relative_path in PARSER_BLOBS:
        source_path = legacy_root / relative_path
        active_cache = Path(importlib.util.cache_from_source(str(source_path)))
        try:
            active_cache.lstat()
        except FileNotFoundError:
            continue
        raise CatalogError(
            f"PARSER_ACTIVE_BYTECODE_CACHE: {relative_path}: {active_cache}"
        )


def _verify_loaded_game_module(
    game_root: Path,
    relative_path: str,
    expected_blob: str,
    module: Any,
) -> Path:
    path = _verify_game_input_file(game_root, relative_path, expected_blob)
    expected_origin = path.resolve(strict=True)

    file_origin = getattr(module, "__file__", None)
    spec = getattr(module, "__spec__", None)
    spec_origin = getattr(spec, "origin", None)
    if not file_origin or not spec_origin:
        raise CatalogError(f"PROTECTED_GAME_INPUT_MODULE_ORIGIN_MISSING: {relative_path}")

    for origin in (file_origin, spec_origin):
        try:
            actual_origin = Path(origin).resolve(strict=True)
        except (OSError, RuntimeError) as exc:
            raise CatalogError(
                f"PROTECTED_GAME_INPUT_MODULE_ORIGIN_INVALID: {relative_path}"
            ) from exc
        if actual_origin != expected_origin:
            raise CatalogError(
                f"PROTECTED_GAME_INPUT_MODULE_ORIGIN_MISMATCH: {relative_path}"
            )
    return path


def _load_game_module(game_root: Path, relative_path: str, name: str) -> Any:
    expected_blob = GAME_INPUT_BLOBS[relative_path]
    path = _verify_game_input_file(game_root, relative_path, expected_blob)
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CatalogError(f"MODULE_LOAD_FAILED: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    with _without_bytecode_writes():
        spec.loader.exec_module(module)
    _verify_loaded_game_module(game_root, relative_path, expected_blob, module)
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


def build_catalog(
    observations: Iterable[dict[str, Any]],
    source_stream_counts: dict[str, int],
) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
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
    for family in FAMILIES:
        family_docs[family]["logical_product_digest_sha256"] = product_digest

    index = {
        **product_basis,
        "logical_product_digest_sha256": product_digest,
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


def validate_real_family_counts(family_docs: dict[str, dict[str, Any]]) -> None:
    actual = {family: int(family_docs[family]["counts"]["occurrences"]) for family in FAMILIES}
    if actual != EXPECTED_FAMILY_COUNTS:
        raise CatalogError(f"FAMILY_COUNT_MISMATCH: expected {EXPECTED_FAMILY_COUNTS}, got {actual}")


def _shard_filename(family: str, ordinal: int) -> str:
    stem = FAMILY_FILENAMES[family].removesuffix(".json")
    return f"{stem}-{ordinal:04d}.json"


def _build_family_shards(family_doc: dict[str, Any]) -> tuple[dict[str, Any], list[tuple[str, bytes]]]:
    family = str(family_doc["family"])
    records = list(family_doc["records"])
    requested = SHARD_COUNTS[family]
    shard_count = min(requested, len(records))
    if shard_count < 1:
        raise CatalogError(f"SHARDED_FAMILY_EMPTY: {family}")
    chunk_size = (len(records) + shard_count - 1) // shard_count
    shard_meta: list[dict[str, Any]] = []
    shard_payloads: list[tuple[str, bytes]] = []
    reconstructed: list[dict[str, Any]] = []

    for zero_index in range(shard_count):
        ordinal = zero_index + 1
        chunk = records[zero_index * chunk_size : (zero_index + 1) * chunk_size]
        if not chunk:
            raise CatalogError(f"EMPTY_SHARD: {family}:{ordinal}")
        records_digest = sha256_bytes(canonical_bytes(chunk))
        shard = {
            "schema": SHARD_SCHEMA,
            "mapper_profile": MAPPER_PROFILE,
            "family": family,
            "closure": CLOSURE,
            "evidence_status": EVIDENCE_STATUS,
            "logical_product_digest_sha256": family_doc["logical_product_digest_sha256"],
            "family_records_digest_sha256": family_doc["records_digest_sha256"],
            "ordinal": ordinal,
            "shard_count": shard_count,
            "record_count": len(chunk),
            "records_digest_sha256": records_digest,
            "records": chunk,
        }
        payload = canonical_bytes(shard)
        if len(payload) > MAX_SHARD_BYTES:
            raise CatalogError(
                f"SHARD_TOO_LARGE: {family}:{ordinal}: {len(payload)} > {MAX_SHARD_BYTES}"
            )
        filename = _shard_filename(family, ordinal)
        tracked_path = f"{FINAL_FAMILY_PREFIX}/{filename}"
        shard_meta.append(
            {
                "ordinal": ordinal,
                "tracked_path": tracked_path,
                "record_count": len(chunk),
                "bytes": len(payload),
                "sha256": sha256_bytes(payload),
                "records_digest_sha256": records_digest,
            }
        )
        shard_payloads.append((filename, payload))
        reconstructed.extend(chunk)

    if sha256_bytes(canonical_bytes(reconstructed)) != family_doc["records_digest_sha256"]:
        raise CatalogError(f"SHARD_RECONSTRUCTION_DIGEST_MISMATCH: {family}")

    manifest = {key: value for key, value in family_doc.items() if key != "records"}
    manifest["schema"] = FAMILY_MANIFEST_SCHEMA
    manifest["storage"] = {
        "mode": "SHARDED",
        "max_shard_bytes": MAX_SHARD_BYTES,
        "shard_count": shard_count,
        "shards": shard_meta,
    }
    return manifest, shard_payloads


def storage_payloads(
    index: dict[str, Any],
    family_docs: dict[str, dict[str, Any]],
) -> dict[str, bytes]:
    payloads: dict[str, bytes] = {}
    family_entries: list[dict[str, Any]] = []

    for family in FAMILIES:
        filename = FAMILY_FILENAMES[family]
        tracked_path = f"{FINAL_FAMILY_PREFIX}/{filename}"
        family_doc = family_docs[family]
        if family in SHARD_COUNTS and family_doc["records"]:
            stored_doc, shards = _build_family_shards(family_doc)
            payload = canonical_bytes(stored_doc)
            for shard_name, shard_payload in shards:
                payloads[f"{FAMILY_DIR_NAME}/{shard_name}"] = shard_payload
            storage_mode = "SHARDED"
            shard_count = len(shards)
        else:
            payload = canonical_bytes(family_doc)
            storage_mode = "INLINE"
            shard_count = 0

        payloads[f"{FAMILY_DIR_NAME}/{filename}"] = payload
        family_entries.append(
            {
                "family": family,
                "tracked_path": tracked_path,
                "storage_mode": storage_mode,
                "shard_count": shard_count,
                "bytes": len(payload),
                "sha256": sha256_bytes(payload),
                "records": family_doc["counts"]["occurrences"],
                "records_digest_sha256": family_doc["records_digest_sha256"],
            }
        )

    index["storage"] = {
        "index_tracked_path": FINAL_INDEX_PATH,
        "family_files": family_entries,
    }
    payloads[INDEX_NAME] = canonical_bytes(index)
    return payloads


def write_catalog(output_dir: Path, payloads: dict[str, bytes]) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    for relative_path, payload in sorted(payloads.items()):
        path = output_dir / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)


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

    producer_rel = "tools/game-atlas-fullworld-source/producer.py"
    census_rel = "tools/reference-world-corridor-census/census.py"
    bounded_export_rel = "tools/game-atlas-thais-fixture/export.py"

    producer = _load_game_module(
        game_root,
        producer_rel,
        "cw2_b6_fullworld_producer",
    )
    census = _load_game_module(
        game_root,
        census_rel,
        "cw2_b6_corridor_census",
    )
    if Path(getattr(producer, "BOUNDED_EXPORT_REL", "")) != Path(bounded_export_rel):
        raise CatalogError("PROTECTED_GAME_INPUT_TRANSITIVE_PATH_MISMATCH")

    _verify_game_input_file(
        game_root,
        bounded_export_rel,
        GAME_INPUT_BLOBS[bounded_export_rel],
    )
    with _without_bytecode_writes():
        runtime = producer.load_runtime(
            legacy_root=legacy_root,
            map_path=map_path,
            asset_zip=asset_zip,
            assets_dir=assets_dir,
            source_generation_profile_id=SOURCE_PROFILE,
        )
    _verify_loaded_game_module(
        game_root,
        bounded_export_rel,
        GAME_INPUT_BLOBS[bounded_export_rel],
        runtime.bounded,
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
    validate_real_family_counts(family_docs)
    payloads = storage_payloads(index, family_docs)
    write_catalog(args.output.resolve(), payloads)

    print(
        json.dumps(
            {
                "logical_product_digest_sha256": index["logical_product_digest_sha256"],
                "source_stream_counts": counts,
                "total_occurrences": index["total_occurrences"],
                "family_counts": {
                    family: family_docs[family]["counts"]["occurrences"]
                    for family in FAMILIES
                },
                "storage": {
                    entry["family"]: {
                        "storage_mode": entry["storage_mode"],
                        "shard_count": entry["shard_count"],
                        "bytes": entry["bytes"],
                        "sha256": entry["sha256"],
                        "records_digest_sha256": entry["records_digest_sha256"],
                    }
                    for entry in index["storage"]["family_files"]
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
