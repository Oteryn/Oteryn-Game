#!/usr/bin/env python3
"""CW2-B2 deterministic creature/spawn source and native-binding catalogue.

The batch consumes exact pinned legacy Git objects through the already-protected
Game-owned creature exporters. Legacy paths, names, hashes, Atlas entity IDs and
spawn record IDs remain migration/source identity only and never mint native
Oteryn Content identity.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Iterable

SCHEMA = "OTERYN_CW2_CREATURE_SPAWN_BINDING_BATCH/v1"
MAPPER_PROFILE = "OTERYN_CW2_CREATURE_SPAWN_BINDING_MAPPER/v1"
TASK = "CW2-B2 CREATURE_SPAWN_NATIVE_BINDING_BATCH_504"
ADMISSION_MAIN = "ebc860d7cd12bb855228a48759c4cc37b828da63"
SOURCE_REPOSITORY = "blakinio/Otheryn"
SOURCE_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
SOURCE_CLASSIFICATION = "OTERYN_LEGACY / MIGRATION_EVIDENCE"
EVIDENCE_STATUS = "OTS_HYPOTHESIS_ONLY"
CLOSURE = "CANDIDATE_ONLY"
WORLD_ROOT = "vendor/map-analysis/crystalserver/data-global/world"
MONSTER_ROOT = "vendor/map-analysis/crystalserver/data-global/monster"
PRIMARY_WORLD_FILE = f"{WORLD_ROOT}/world-monster.xml"
PRIMARY_WORLD_BLOB = "3829f7dcd9091ace8549bfb36a00186726076898"
PRIMARY_WORLD_SIZE = 10_097_048

GAME_PRODUCER_PINS = {
    "tools/game-atlas-creatures/export.py": "dcd4aa588ca9d93ada697b9135d64a82046fb079",
    "tools/game-atlas-creatures/identity.py": "ccebdcd2b88c147e20ed9be9e5dff524276591bd",
    "tools/game-atlas-creatures/self_test.py": "7a6a38d870e0421a139fb67a729abc68c7c39930",
    "tools/game-atlas-creature-gameplay/export.py": "8b8606bb6052dde3ce16ad3cfe0eb88290351b14",
    "tools/game-atlas-creature-gameplay/self_test.py": "b4cda247d80963dae44c73fdcee6ea4027f1d8ab",
}

ADMITTED_NATIVE_CREATURE_BINDINGS: tuple[dict[str, Any], ...] = ()
ADMITTED_NATIVE_SPAWN_BINDINGS: tuple[dict[str, Any], ...] = ()


class CatalogError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _git(repo: Path, *args: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(
        ("git", "-C", str(repo), *args),
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if binary:
        return result.stdout
    return result.stdout.decode("utf-8").strip()


def _normalize_remote(url: str) -> str:
    value = url.strip().lower().replace("\\", "/")
    if value.endswith(".git"):
        value = value[:-4]
    if value.startswith("git@github.com:"):
        value = "https://github.com/" + value.split(":", 1)[1]
    return value.rstrip("/")


def _tree_paths(repo: Path, root: str) -> list[str]:
    raw = _git(
        repo,
        "ls-tree",
        "-r",
        "-z",
        "--name-only",
        SOURCE_REVISION,
        "--",
        root,
        binary=True,
    )
    assert isinstance(raw, bytes)
    return sorted(
        value.decode("utf-8")
        for value in raw.split(b"\0")
        if value
    )


def _source_payload(repo: Path, path: str) -> tuple[str, bytes]:
    try:
        blob = str(_git(repo, "rev-parse", f"{SOURCE_REVISION}:{path}"))
        payload = _git(repo, "cat-file", "blob", blob, binary=True)
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError(f"SOURCE_OBJECT_UNAVAILABLE: {path}") from exc
    assert isinstance(payload, bytes)
    return blob, payload


def verify_source_repository(
    source_repo: Path,
) -> tuple[dict[str, Any], dict[str, bytes]]:
    source_repo = source_repo.resolve()
    try:
        top = Path(str(_git(source_repo, "rev-parse", "--show-toplevel"))).resolve()
        remote = _normalize_remote(str(_git(source_repo, "remote", "get-url", "origin")))
        revision = str(_git(source_repo, "rev-parse", SOURCE_REVISION))
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("SOURCE_REPOSITORY_UNVERIFIABLE") from exc
    if top != source_repo:
        raise CatalogError("SOURCE_REPOSITORY_ROOT_MISMATCH")
    if remote != f"https://github.com/{SOURCE_REPOSITORY}".lower():
        raise CatalogError(f"SOURCE_REPOSITORY_REMOTE_MISMATCH: {remote}")
    if revision != SOURCE_REVISION:
        raise CatalogError("SOURCE_REVISION_MISMATCH")

    world_paths = [
        path for path in _tree_paths(source_repo, WORLD_ROOT)
        if path.endswith("-monster.xml")
    ]
    monster_paths = [
        path for path in _tree_paths(source_repo, MONSTER_ROOT)
        if path.endswith(".lua")
    ]
    if PRIMARY_WORLD_FILE not in world_paths:
        raise CatalogError("PRIMARY_WORLD_FILE_MISSING")

    payloads: dict[str, bytes] = {}
    files: list[dict[str, Any]] = []
    for role, prefix, paths in (
        ("MONSTER_SPAWN_XML", "world", world_paths),
        ("MONSTER_DEFINITION_LUA", "monster", monster_paths),
    ):
        for index, path in enumerate(paths):
            blob, payload = _source_payload(source_repo, path)
            payloads[path] = payload
            files.append(
                {
                    "source_file_ref": f"{prefix}:{index:04d}",
                    "path": path,
                    "blob": blob,
                    "size": len(payload),
                    "sha256": sha256_bytes(payload),
                    "role": role,
                }
            )

    primary = next(value for value in files if value["path"] == PRIMARY_WORLD_FILE)
    if primary["blob"] != PRIMARY_WORLD_BLOB:
        raise CatalogError(f"PRIMARY_WORLD_BLOB_MISMATCH: {primary['blob']}")
    if primary["size"] != PRIMARY_WORLD_SIZE:
        raise CatalogError(f"PRIMARY_WORLD_SIZE_MISMATCH: {primary['size']}")

    aggregate = hashlib.sha256()
    for record in sorted(files, key=lambda value: value["path"]):
        payload = payloads[str(record["path"])]
        aggregate.update(str(record["path"]).encode("utf-8"))
        aggregate.update(b"\0")
        aggregate.update(str(record["blob"]).encode("ascii"))
        aggregate.update(b"\0")
        aggregate.update(len(payload).to_bytes(8, "big"))
        aggregate.update(payload)

    snapshot = {
        "repository": SOURCE_REPOSITORY,
        "revision": SOURCE_REVISION,
        "classification": SOURCE_CLASSIFICATION,
        "consumed_roots": [WORLD_ROOT, MONSTER_ROOT],
        "selection": {
            WORLD_ROOT: "*-monster.xml",
            MONSTER_ROOT: "*.lua",
        },
        "files": files,
        "file_counts": {
            "monster_spawn_xml": len(world_paths),
            "monster_definition_lua": len(monster_paths),
            "total": len(files),
        },
        "aggregate_source_bytes_sha256": aggregate.hexdigest(),
        "aggregate_algorithm": (
            "sha256(sorted(path_utf8 + NUL + git_blob_ascii + NUL + "
            "uint64be_size + exact_git_blob_bytes))"
        ),
        "primary_world_file": {
            "path": PRIMARY_WORLD_FILE,
            "blob": PRIMARY_WORLD_BLOB,
            "size": PRIMARY_WORLD_SIZE,
        },
    }
    return snapshot, payloads


def verify_game_producers(game_root: Path) -> dict[str, Any]:
    game_root = game_root.resolve()
    try:
        top = Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve()
        base = str(_git(game_root, "rev-parse", ADMISSION_MAIN))
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("GAME_REPOSITORY_UNVERIFIABLE") from exc
    if top != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if base != ADMISSION_MAIN:
        raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")

    producers = []
    for path, expected_blob in GAME_PRODUCER_PINS.items():
        status = str(_git(game_root, "status", "--porcelain=v1", "--", path))
        if status:
            raise CatalogError(f"GAME_PRODUCER_PATH_DIRTY: {path}")
        blob = str(_git(game_root, "rev-parse", f"{ADMISSION_MAIN}:{path}"))
        if blob != expected_blob:
            raise CatalogError(f"GAME_PRODUCER_BLOB_MISMATCH: {path}: {blob}")
        payload = _git(game_root, "cat-file", "blob", blob, binary=True)
        assert isinstance(payload, bytes)
        producers.append(
            {
                "path": path,
                "blob": blob,
                "sha256": sha256_bytes(payload),
            }
        )
    return {
        "admission_main": ADMISSION_MAIN,
        "producer_blobs": producers,
    }


def _load_game_modules(game_root: Path) -> tuple[Any, Any, Any]:
    creature_dir = game_root / "tools/game-atlas-creatures"
    gameplay_path = game_root / "tools/game-atlas-creature-gameplay/export.py"
    if str(creature_dir) not in sys.path:
        sys.path.insert(0, str(creature_dir))

    identity_spec = importlib.util.spec_from_file_location(
        "cw2_b2_creature_identity", creature_dir / "identity.py"
    )
    creature_spec = importlib.util.spec_from_file_location(
        "cw2_b2_creature_export", creature_dir / "export.py"
    )
    gameplay_spec = importlib.util.spec_from_file_location(
        "cw2_b2_gameplay_export", gameplay_path
    )
    if (
        identity_spec is None
        or identity_spec.loader is None
        or creature_spec is None
        or creature_spec.loader is None
        or gameplay_spec is None
        or gameplay_spec.loader is None
    ):
        raise CatalogError("GAME_PRODUCER_IMPORT_FAILED")

    identity = importlib.util.module_from_spec(identity_spec)
    sys.modules[identity_spec.name] = identity
    identity_spec.loader.exec_module(identity)

    creature = importlib.util.module_from_spec(creature_spec)
    sys.modules[creature_spec.name] = creature
    creature_spec.loader.exec_module(creature)

    gameplay = importlib.util.module_from_spec(gameplay_spec)
    sys.modules[gameplay_spec.name] = gameplay
    gameplay_spec.loader.exec_module(gameplay)
    return identity, creature, gameplay


def resolve_native_mapping(
    source_identity: str,
    binding_candidates: Iterable[dict[str, Any]],
) -> dict[str, Any]:
    matches = [
        value for value in binding_candidates
        if value.get("source_identity") == source_identity
    ]
    if not matches:
        return {
            "disposition": "UNRESOLVED",
            "content_key": None,
            "evidence_refs": [],
            "reason": "no admitted Game-owned explicit source-to-native binding",
        }

    validated: list[tuple[str, str, str]] = []
    for value in matches:
        target = value.get("content_key")
        evidence_ref = value.get("evidence_ref")
        state = value.get("state", "ASSERTED")
        if not isinstance(target, str) or not target.startswith("oteryn:"):
            raise CatalogError("BINDING_TARGET_INVALID")
        if target.startswith("oteryn:vsl."):
            raise CatalogError("SYNTHETIC_VSL_TARGET_FORBIDDEN")
        if not isinstance(evidence_ref, str) or not evidence_ref:
            raise CatalogError("BINDING_EVIDENCE_REF_REQUIRED")
        if state not in {"ASSERTED", "CANDIDATE"}:
            raise CatalogError("BINDING_STATE_INVALID")
        validated.append((target, evidence_ref, state))

    asserted = sorted({target for target, _, state in validated if state == "ASSERTED"})
    candidates = sorted({target for target, _, state in validated if state == "CANDIDATE"})
    refs = sorted({ref for _, ref, _ in validated})

    if len(asserted) > 1 or (
        len(asserted) == 1 and any(value != asserted[0] for value in candidates)
    ):
        return {
            "disposition": "CONFLICT",
            "content_key": None,
            "candidate_content_keys": sorted(set(asserted + candidates)),
            "evidence_refs": refs,
            "reason": "admitted explicit binding evidence disagrees",
        }
    if len(asserted) == 1:
        return {
            "disposition": "RESOLVED",
            "content_key": asserted[0],
            "evidence_refs": refs,
            "reason": "explicit admitted Game-owned binding",
        }
    if len(candidates) > 1:
        return {
            "disposition": "AMBIGUOUS",
            "content_key": None,
            "candidate_content_keys": candidates,
            "evidence_refs": refs,
            "reason": "more than one candidate native target remains",
        }
    return {
        "disposition": "UNRESOLVED",
        "content_key": None,
        "candidate_content_keys": candidates,
        "evidence_refs": refs,
        "reason": "candidate-only evidence cannot establish native identity",
    }


def ensure_unique_source_identities(
    source_identities: Iterable[str],
    label: str,
) -> None:
    seen: set[str] = set()
    for value in source_identities:
        if value in seen:
            raise CatalogError(f"DUPLICATE_{label}_SOURCE_IDENTITY: {value}")
        seen.add(value)


def definition_name_collisions(
    records: Iterable[dict[str, Any]],
) -> list[dict[str, Any]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        grouped[str(record["normalized_name"])].append(record)
    result = []
    for normalized_name in sorted(grouped):
        values = grouped[normalized_name]
        if len(values) <= 1:
            continue
        result.append(
            {
                "normalized_name": normalized_name,
                "atlas_entity_id": values[0]["atlas_entity_id"],
                "source_file_refs": sorted(
                    str(value["source_file_ref"]) for value in values
                ),
                "classification": "SOURCE_NAME_COLLISION_NOT_NATIVE_AUTHORITY",
            }
        )
    return result


def canonical_record_list(
    records: Iterable[dict[str, Any]],
) -> list[dict[str, Any]]:
    return sorted((dict(record) for record in records), key=canonical_bytes)


def _materialize_sources(
    root: Path,
    snapshot: dict[str, Any],
    payloads: dict[str, bytes],
) -> tuple[Path, Path, Path]:
    world_root = root / "world"
    monster_root = root / "monster"
    npc_root = root / "empty-npc"
    world_root.mkdir(parents=True)
    monster_root.mkdir(parents=True)
    npc_root.mkdir(parents=True)

    for record in snapshot["files"]:
        path = str(record["path"])
        if path.startswith(WORLD_ROOT + "/"):
            relative = path[len(WORLD_ROOT) + 1 :]
            target = world_root / relative
        elif path.startswith(MONSTER_ROOT + "/"):
            relative = path[len(MONSTER_ROOT) + 1 :]
            target = monster_root / relative
        else:
            raise CatalogError(f"UNEXPECTED_SOURCE_PATH: {path}")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payloads[path])
    return world_root, monster_root, npc_root


def _presentation_candidate(parsed: Any) -> dict[str, Any]:
    name, definition = parsed
    outfit = definition.outfit
    if int(outfit.look_type) <= 0:
        return {
            "state": "UNKNOWN",
            "reason_codes": ["NO_STATIC_POSITIVE_LOOKTYPE"],
        }
    return {
        "state": "CANDIDATE",
        "outfit": {
            "look_type": int(outfit.look_type),
            "head": int(outfit.head),
            "body": int(outfit.body),
            "legs": int(outfit.legs),
            "feet": int(outfit.feet),
            "addons": int(outfit.addons),
        },
        "reason_codes": [],
    }


def _compact_native_mapping(mapping: dict[str, Any]) -> dict[str, Any]:
    result = {"disposition": mapping["disposition"]}
    if mapping.get("content_key") is not None:
        result["content_key"] = mapping["content_key"]
    if mapping.get("candidate_content_keys"):
        result["candidate_content_keys"] = mapping["candidate_content_keys"]
    if mapping.get("evidence_refs"):
        result["evidence_refs"] = mapping["evidence_refs"]
    return result


def build_definition_catalog(
    snapshot: dict[str, Any],
    monster_root: Path,
    identity: Any,
    creature: Any,
    gameplay: Any,
) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    profiles: dict[str, dict[str, Any]] = {}
    unparsed: list[str] = []
    mapping_counts: Counter[str] = Counter()
    state_counts: dict[str, Counter[str]] = {
        "presentation": Counter(),
        "stats": Counter(),
        "resistances": Counter(),
        "loot": Counter(),
    }
    loot_rows = 0
    unresolved_loot_rows = 0

    source_records = [
        value for value in snapshot["files"]
        if value["role"] == "MONSTER_DEFINITION_LUA"
    ]
    for source in source_records:
        source_path = str(source["path"])
        relative = source_path[len(MONSTER_ROOT) + 1 :]
        path = monster_root / relative
        presentation_parsed = creature._parse_definition(path, "monster")
        gameplay_profile = gameplay._monster_profile(path, {})
        if (presentation_parsed is None) != (gameplay_profile is None):
            raise CatalogError(f"PINNED_EXPORTER_PARSER_DISAGREEMENT: {source_path}")
        if presentation_parsed is None:
            unparsed.append(str(source["source_file_ref"]))
            continue

        name = str(presentation_parsed[0])
        if str(gameplay_profile["name"]).casefold() != name.casefold():
            raise CatalogError(f"PINNED_EXPORTER_NAME_DISAGREEMENT: {source_path}")
        normalized_name = name.casefold()
        atlas_entity_id = identity.stable_creature_entity_id(
            "monster", normalized_name
        )
        if gameplay_profile["entity_id"] != atlas_entity_id:
            raise CatalogError(f"PINNED_EXPORTER_IDENTITY_DISAGREEMENT: {source_path}")

        source_identity = f"definition:{source['source_file_ref']}"
        mapping = resolve_native_mapping(
            source_identity, ADMITTED_NATIVE_CREATURE_BINDINGS
        )
        mapping_counts[mapping["disposition"]] += 1
        candidate_profile = {
            "classification": EVIDENCE_STATUS,
            "presentation": _presentation_candidate(presentation_parsed),
            "stats": gameplay_profile["stats"],
            "resistances": gameplay_profile["resistances"],
            "loot": gameplay_profile["loot"],
        }
        profile_id = sha256_bytes(canonical_bytes(candidate_profile))
        profiles.setdefault(
            profile_id,
            {"profile_id": profile_id, **candidate_profile},
        )
        for key in state_counts:
            state_counts[key][str(candidate_profile[key]["state"])] += 1
        loot_entries = list(candidate_profile["loot"].get("entries", []))
        loot_rows += len(loot_entries)
        unresolved_loot_rows += sum(
            1
            for row in loot_entries
            if row.get("item_resolution_state") != "RESOLVED"
        )
        records.append(
            {
                "source_file_ref": source["source_file_ref"],
                "source_identity": source_identity,
                "display_name": name,
                "normalized_name": normalized_name,
                "atlas_entity_id": atlas_entity_id,
                "native_mapping": _compact_native_mapping(mapping),
                "candidate_profile_id": profile_id,
            }
        )

    records = canonical_record_list(records)
    ensure_unique_source_identities(
        (str(value["source_identity"]) for value in records),
        "DEFINITION",
    )
    collisions = definition_name_collisions(records)
    return {
        "records": records,
        "candidate_profiles": [
            profiles[key] for key in sorted(profiles)
        ],
        "unparsed_source_file_refs": sorted(unparsed),
        "normalized_name_collisions": collisions,
        "native_mapping_totals": {
            state: mapping_counts[state]
            for state in ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT")
        },
        "candidate_state_totals": {
            key: dict(sorted(counter.items()))
            for key, counter in state_counts.items()
        },
        "deferred_b3_loot": {
            "observed_rows": loot_rows,
            "unresolved_or_non_native_rows": unresolved_loot_rows,
        },
    }


def build_spawn_catalog(
    snapshot: dict[str, Any],
    payloads: dict[str, bytes],
    scratch: Path,
    identity: Any,
    creature: Any,
) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    source_identities: list[str] = []
    mapping_counts: Counter[str] = Counter()
    origin_counts: Counter[str] = Counter()

    world_sources = [
        value for value in snapshot["files"]
        if value["role"] == "MONSTER_SPAWN_XML"
    ]
    for source in world_sources:
        path = str(source["path"])
        relative = path[len(WORLD_ROOT) + 1 :]
        isolated_root = (
            scratch
            / "spawn-files"
            / str(source["source_file_ref"]).replace(":", "-")
        )
        target = isolated_root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payloads[path])

        parsed = creature.parse_spawns(isolated_root, "monster")
        for row in parsed:
            record_id = str(row["record_id"])
            source_identity = (
                f"spawn:{source['source_file_ref']}:{record_id}"
            )
            source_identities.append(source_identity)
            mapping = resolve_native_mapping(
                source_identity, ADMITTED_NATIVE_SPAWN_BINDINGS
            )
            mapping_counts[mapping["disposition"]] += 1
            normalized_name = str(row["name"]).casefold()
            atlas_entity_id = identity.stable_creature_entity_id(
                "monster", normalized_name
            )
            origin_counts[str(row["origin"])] += 1
            record = {
                "source_file_ref": source["source_file_ref"],
                "record_id": record_id,
                "display_name": row["name"],
                "normalized_name": normalized_name,
                "atlas_entity_id": atlas_entity_id,
                "native_disposition": mapping["disposition"],
                "position": row["position"],
                "spawn_area": row["spawn_area"],
                "spawn_time_seconds": row["spawn_time_seconds"],
                "direction": row["direction"],
                "weight": row["weight"],
                "origin": row["origin"],
            }
            if mapping.get("content_key") is not None:
                record["native_content_key"] = mapping["content_key"]
            if mapping.get("candidate_content_keys"):
                record["candidate_content_keys"] = mapping[
                    "candidate_content_keys"
                ]
            records.append(record)

    ensure_unique_source_identities(source_identities, "SPAWN")
    return {
        "records": canonical_record_list(records),
        "native_mapping_totals": {
            state: mapping_counts[state]
            for state in ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT")
        },
        "origin_totals": dict(sorted(origin_counts.items())),
    }


def _partition_total(mapping_totals: dict[str, int]) -> int:
    return sum(
        int(mapping_totals[state])
        for state in ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT")
    )


def build_evidence(source_repo: Path, game_root: Path) -> dict[str, Any]:
    source_snapshot, payloads = verify_source_repository(source_repo)
    producers = verify_game_producers(game_root)
    identity, creature, gameplay = _load_game_modules(game_root)

    with tempfile.TemporaryDirectory(prefix="oteryn-cw2-b2-") as tmp:
        scratch = Path(tmp)
        _, monster_root, _ = _materialize_sources(
            scratch / "full-source",
            source_snapshot,
            payloads,
        )
        definitions = build_definition_catalog(
            source_snapshot,
            monster_root,
            identity,
            creature,
            gameplay,
        )
        spawns = build_spawn_catalog(
            source_snapshot,
            payloads,
            scratch,
            identity,
            creature,
        )

    definition_count = len(definitions["records"])
    spawn_count = len(spawns["records"])
    if _partition_total(definitions["native_mapping_totals"]) != definition_count:
        raise CatalogError("CREATURE_NATIVE_MAPPING_PARTITION_INVARIANT_FAILED")
    if _partition_total(spawns["native_mapping_totals"]) != spawn_count:
        raise CatalogError("SPAWN_NATIVE_MAPPING_PARTITION_INVARIANT_FAILED")

    mapper_payload = Path(__file__).read_bytes()
    value: dict[str, Any] = {
        "schema": SCHEMA,
        "task": TASK,
        "source_snapshot": source_snapshot,
        "game_owned_producers": producers,
        "mapper": {
            "profile": MAPPER_PROFILE,
            "path": (
                "tools/reference-world-corridor-census/"
                "creature_spawn_binding_catalog.py"
            ),
            "sha256": sha256_bytes(mapper_payload),
            "final_pr_head": "RECORDED_EXTERNALLY_BY_LIVE_PR_READBACK",
            "final_pr_head_note": (
                "a tracked Git object cannot contain its own final commit SHA "
                "without self-reference"
            ),
        },
        "classification": {
            "source": SOURCE_CLASSIFICATION,
            "evidence_status": EVIDENCE_STATUS,
            "production_authority": "NONE",
            "reference_parity_claim": "NONE",
        },
        "closure": CLOSURE,
        "identity_boundaries": {
            "source_definition_identity": (
                "exact source_file_ref -> pinned repository/revision/path/blob"
            ),
            "source_spawn_identity": (
                "source_file_ref + existing Game-owned Atlas record_id"
            ),
            "atlas_export_entity_identity": (
                "monster-entity:<hash> from protected identity.py; "
                "public/export seam only"
            ),
            "native_content_identity": (
                "only an admitted explicit Game-owned binding may supply an "
                "oteryn:* Content identity"
            ),
            "display_name_is_native_identity": False,
            "normalized_name_is_native_identity": False,
            "atlas_entity_id_is_native_identity": False,
            "spawn_record_id_is_native_identity": False,
            "coordinates_are_native_identity": False,
            "vsl_fixture_keys_are_import_binding_authority": False,
        },
        "monster_definitions": definitions,
        "monster_spawns": spawns,
        "field_classification": {
            "presentation_outfit": {
                "classification": EVIDENCE_STATUS,
                "producer": "tools/game-atlas-creatures/export.py",
                "reason_codes": [
                    "MIGRATION_SOURCE_NOT_TARGET_PARITY",
                    "LOOKTYPE_NOT_CREATURE_IDENTITY",
                ],
            },
            "health_experience_speed_armor_defense": {
                "classification": EVIDENCE_STATUS,
                "producer": "tools/game-atlas-creature-gameplay/export.py",
                "reason_codes": ["STATIC_EXTRACTOR_CANDIDATE_ONLY"],
            },
            "resistances_immunities": {
                "classification": EVIDENCE_STATUS,
                "producer": "tools/game-atlas-creature-gameplay/export.py",
                "reason_codes": ["STATIC_EXTRACTOR_CANDIDATE_ONLY"],
            },
            "spawn_position_area_time_direction_weight_origin": {
                "classification": EVIDENCE_STATUS,
                "producer": "tools/game-atlas-creatures/export.py",
                "reason_codes": [
                    "SOURCE_PLACEMENT_ONLY",
                    "COORDINATES_NOT_NATIVE_SPAWN_IDENTITY",
                ],
            },
            "loot_item_relations": {
                "classification": "DEFERRED_CW2_B3",
                "producer": "tools/game-atlas-creature-gameplay/export.py",
                "reason_codes": [
                    "NATIVE_ITEM_KEY_BINDING_OUTSIDE_B2",
                    "DISPLAY_NAME_OR_CLIENT_ID_NOT_ITEM_IDENTITY",
                ],
            },
            "abilities_attacks_corpse_reward": {
                "classification": "UNKNOWN",
                "reason_codes": [
                    "NOT_STRUCTURALLY_EXPOSED_BY_PINNED_B2_EXPORTERS"
                ],
            },
        },
        "counts": {
            "source_monster_definition_files_consumed": source_snapshot[
                "file_counts"
            ]["monster_definition_lua"],
            "source_monster_spawn_xml_files_consumed": source_snapshot[
                "file_counts"
            ]["monster_spawn_xml"],
            "monster_definition_source_identities": definition_count,
            "monster_spawn_occurrences": spawn_count,
            "unparsed_monster_definition_files": len(
                definitions["unparsed_source_file_refs"]
            ),
            "normalized_name_collision_groups": len(
                definitions["normalized_name_collisions"]
            ),
            "deferred_b3_loot_rows": definitions["deferred_b3_loot"][
                "observed_rows"
            ],
        },
        "count_invariants": {
            "definition_mapping_partition_complete": (
                _partition_total(definitions["native_mapping_totals"])
                == definition_count
            ),
            "spawn_mapping_partition_complete": (
                _partition_total(spawns["native_mapping_totals"])
                == spawn_count
            ),
            "definition_source_identity_unique": True,
            "spawn_source_identity_unique": True,
            "closure_is_candidate_only": CLOSURE == "CANDIDATE_ONLY",
        },
    }
    value["product_digest_sha256"] = sha256_bytes(canonical_bytes(value))
    value["product_digest_scope"] = (
        "canonical JSON with product_digest fields omitted"
    )
    return value


def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical_bytes(evidence))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-repo", type=Path, required=True)
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    evidence = build_evidence(args.source_repo, args.game_root)
    write_evidence(args.output, evidence)
    counts = evidence["counts"]
    creature_map = evidence["monster_definitions"]["native_mapping_totals"]
    spawn_map = evidence["monster_spawns"]["native_mapping_totals"]
    print(
        "creature-spawn-binding-catalog: PASS "
        f"definitions={counts['monster_definition_source_identities']} "
        f"spawns={counts['monster_spawn_occurrences']} "
        f"creature_unresolved={creature_map['UNRESOLVED']} "
        f"spawn_unresolved={spawn_map['UNRESOLVED']} "
        f"digest={evidence['product_digest_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
