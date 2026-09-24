#!/usr/bin/env python3
"""Deterministic CW2-B5 static NPC/service candidate catalogue.

The catalogue consumes exact Git objects and the protected Game Atlas, B1 and
B3 producers.  It never executes Lua, never translates dialogue/control flow,
and never promotes legacy names, numbers, display identifiers, Atlas ids,
coordinates, or hashes to native Oteryn identity.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Iterable

SCHEMA = "OTERYN_CW2_NPC_SERVICE_BINDING_CATALOG/v1"
MAPPER_PROFILE = "OTERYN_CW2_NPC_SERVICE_BINDING_MAPPER/v1"
TASK = "CW2-B5 NPC_SERVICE_BINDING_CATALOGUE_504"
ADMISSION_MAIN = "e026eeddd687709309589d4a15a796c28cadfe2c"
SOURCE_REPOSITORY = "blakinio/Otheryn"
SOURCE_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
SOURCE_CLASSIFICATION = "OTERYN_LEGACY / MIGRATION_EVIDENCE"
EVIDENCE_STATUS = "OTS_HYPOTHESIS_ONLY"
CLOSURE = "CANDIDATE_ONLY"
SHARD_SCHEMA = "OTERYN_CW2_NPC_SERVICE_BINDING_CATALOG_SHARD/v1"
SHARDED_STORAGE_SCHEMA = "OTERYN_CW2_NPC_SERVICE_BINDING_CATALOG_SHARDED_STORAGE/v1"
MAX_SHARD_BYTES = 600_000
SHARD_TARGET_BYTES = 560_000

NPC_ROOT = "vendor/map-analysis/crystalserver/data-global/npc"
WORLD_ROOT = "vendor/map-analysis/crystalserver/data-global/world"
PRIMARY_WORLD_FILE = f"{WORLD_ROOT}/world-npc.xml"
PRIMARY_WORLD_BLOB = "ae5b1bb683786baa8b0eaf426e57e913fbca029e"
PRIMARY_WORLD_SIZE = 136_335
PRIMARY_WORLD_SHA256 = "78960cfd793ca65b1311985861de18806f3d9f2381821c9ec2a920f1188c762f"
SOURCE_AGGREGATE = "ec40b52e72e1d00a747546a72c9929fba0960ae49cc7b07a763a53be03aec6f1"
NPC_WORLD_AGGREGATE = "905f206b618a810dc624e3504a93e98d09c6e62f1f231d8ac7e0f55faf8fd725"
STATIC_CREATURE_SEMANTIC_DIGEST = "sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4"

GAME_PINS: dict[str, tuple[str, str]] = {
    "tools/game-atlas-creatures/export.py": ("dcd4aa588ca9d93ada697b9135d64a82046fb079", "519b632921bf9058cb1eb982c71c67c65e4cea61ac68422e2cf0aefe28268325"),
    "tools/game-atlas-creatures/identity.py": ("ccebdcd2b88c147e20ed9be9e5dff524276591bd", "2f9bff2fed95038512a1d676dde274d9a66e9e2f95b9a3db7480c9245b2f75a5"),
    "tools/game-atlas-creatures/self_test.py": ("7a6a38d870e0421a139fb67a729abc68c7c39930", "047a0c6eeed4c7667fb283216525be4bf9b6e28954aa250be3adbbac846b1477"),
    "tools/game-atlas-creature-gameplay/export.py": ("8b8606bb6052dde3ce16ad3cfe0eb88290351b14", "e2cbd0c97fcc449c4ec1c2432e96446a0a5a1e57cd9dec7fccc5ed13711ff364"),
    "tools/game-atlas-creature-gameplay/self_test.py": ("b4cda247d80963dae44c73fdcee6ea4027f1d8ab", "e06a7eee87a956495c38110e49a0f4f79547de2178a5d3df370b455ddb3c1e33"),
    "tools/reference-world-corridor-census/item_identity_catalog.py": ("904d62e1277ceae76434f75bca104686a7303ff2", "320ce69f516de2a6e3cec669493ec59a6f3103f405e624478cf2a76acd3d01b6"),
    "tools/reference-world-corridor-census/loot_item_binding_catalog.py": ("821aa7386489309f98562b982c45d2d069e408c6", "0155337520b5af64f45bc2685e71d02f20f0c2d6468253f57f56b6856b2d56a8"),
}

B1_EVIDENCE_PATH = "docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
B1_EVIDENCE_BLOB = "2f0121f3ea6586477b4535840b9a1f1bc28c677c"
B1_PRODUCT_DIGEST = "d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc"
B3_EVIDENCE_PATH = "docs/agents/evidence/OTV2-20260919-content-world-cw2-b3-loot-item-bindings.json"
B3_EVIDENCE_BLOB = "184543847dc080fd4966e7bd8d3c8f04e9c4abbf"
B3_PRODUCT_DIGEST = "5251d211b531481f340a4093a2e70af473ea721f6b51c7109ee7e516b33b1460"

SEMANTIC_FILES: dict[str, tuple[str, str]] = {
    "data/items/appearances.dat": ("2cc2f4910af4f002f99f39e486d1a91b1b56a728", "RUNTIME_ITEM_NAME_BASE"),
    "data/items/items.xml": ("ce465362121def9e27ec48c7288f8125580c2910", "RUNTIME_ITEM_XML_OVERRIDE_AND_CROSSWALK"),
    "data/scripts/lib/register_monster_type.lua": ("6bb42593a2e529e25bb1e5b7df21a288a64ffa33", "SOURCE_ITEM_SELECTOR_PRECEDENCE"),
    "src/items/items.cpp": ("eecbf060ef9e85b452bb897277ff3d3d013a0cf9", "RUNTIME_ITEM_NAME_REGISTRY"),
    "src/protobuf/appearances.proto": ("f39de45976e372b2306f6ff9c4a40412f4debe4e", "APPEARANCE_OBJECT_SCHEMA"),
    "src/lua/functions/creatures/monster/loot_functions.cpp": ("f8a8749b45382683e9e36fee9fc9a7ae1125d6ba", "SOURCE_ITEM_NAME_RESOLUTION"),
    "src/canary_server.cpp": ("fcd0704a4ab0647d28655a88c0a771044c4a7269", "APPEARANCE_SOURCE_PATH"),
}

EXPECTED_COUNTS = {
    "definition_files": 1112, "parsed_definitions": 1103, "unparsed_definitions": 9,
    "profile_identities": 1049, "duplicate_identity_groups": 32,
    "duplicate_definition_extras": 54, "conflicting_profiles": 12,
    "placements": 1068, "placements_with_resolved_roles": 705,
    "role_ambiguity": 10, "shop_rows": 11800, "travel_destinations": 7,
}
EXPECTED_ROLE_OCCURRENCES = {"bank": 25, "travel": 51, "shop": 313, "quest": 432, "blessing": 26, "trainer": 54}
EXPECTED_SHOP_STATES = {"COMPLETE": 254, "PARTIAL": 51, "UNKNOWN": 732, "AMBIGUOUS": 12}
EXPECTED_SERVICE_STATES = {"PARTIAL": 360, "UNKNOWN": 677, "AMBIGUOUS": 12}
EXPECTED_TRAVEL_STATES = {"COMPLETE": 5, "PARTIAL": 43, "UNKNOWN": 989, "AMBIGUOUS": 12}
EXPECTED_SOURCE_ITEM_STATES = {"RESOLVED": 9355, "UNRESOLVED": 791, "AMBIGUOUS": 1654, "CONFLICT": 0}
EXPECTED_NATIVE_ITEM_STATES = {"RESOLVED": 0, "UNRESOLVED": 11800, "AMBIGUOUS": 0, "CONFLICT": 0}

SHARD_COLLECTIONS = (
    ("source-files", ("source_snapshot",), "files"),
    ("definition-identity", ("definition_identity",), "records"),
    ("placement-occurrences", ("placement_occurrences",), "records"),
    ("dialogue", ("dialogue",), "records"),
    ("service-types", ("service_types",), "records"),
    ("shop-profiles", ("shop_catalogues",), "profiles"),
    ("shop-rows", ("shop_catalogues",), "rows"),
    ("price-evidence", ("price_evidence",), "records"),
    ("travel-profiles", ("travel_destinations",), "profiles"),
    ("travel-destinations", ("travel_destinations",), "records"),
    ("travel-conditions", ("travel_conditions",), "records"),
)
EXPECTED_SHARD_COUNTS = {
    "source-files": 1, "definition-identity": 2,
    "placement-occurrences": 2, "dialogue": 1, "service-types": 1,
    "shop-profiles": 1, "shop-rows": 17, "price-evidence": 7,
    "travel-profiles": 1, "travel-destinations": 1,
    "travel-conditions": 1,
}


class CatalogError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def stable_id(kind: str, *parts: object) -> str:
    payload = "\0".join((kind, *(str(value) for value in parts))).encode("utf-8")
    return f"{kind}:{hashlib.sha256(payload).hexdigest()[:32]}"


def _git(repo: Path, *args: str, binary: bool = False) -> bytes | str:
    result = subprocess.run(("git", "-C", str(repo), *args), check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return result.stdout if binary else result.stdout.decode("utf-8").strip()


def _normalize_remote(url: str) -> str:
    value = url.strip().lower().replace("\\", "/")
    if value.endswith(".git"):
        value = value[:-4]
    if value.startswith("git@github.com:"):
        value = "https://github.com/" + value.split(":", 1)[1]
    return value.rstrip("/")


def _blob(repo: Path, revision: str, path: str, expected: str | None = None) -> tuple[str, bytes]:
    try:
        blob = str(_git(repo, "rev-parse", f"{revision}:{path}"))
        payload = _git(repo, "cat-file", "blob", blob, binary=True)
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError(f"GIT_OBJECT_UNAVAILABLE: {revision}:{path}") from exc
    assert isinstance(payload, bytes)
    if expected is not None and blob != expected:
        raise CatalogError(f"GIT_BLOB_MISMATCH: {path}: expected {expected}, got {blob}")
    return blob, payload


def _verify_product(payload: bytes, expected: str, label: str) -> dict[str, Any]:
    try:
        value = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CatalogError(f"{label}_EVIDENCE_INVALID") from exc
    basis = dict(value)
    recorded = basis.pop("product_digest_sha256", None)
    basis.pop("product_digest_scope", None)
    actual = sha256_bytes(canonical_bytes(basis))
    if recorded != expected or actual != expected:
        raise CatalogError(f"{label}_PRODUCT_DIGEST_MISMATCH")
    if value.get("closure") != "CANDIDATE_ONLY":
        raise CatalogError(f"{label}_CLOSURE_MISMATCH")
    return value


def verify_game_inputs(game_root: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    game_root = game_root.resolve()
    try:
        if Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve() != game_root:
            raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
        if str(_git(game_root, "rev-parse", ADMISSION_MAIN)) != ADMISSION_MAIN:
            raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("GAME_REPOSITORY_UNVERIFIABLE") from exc
    records = []
    for path, (expected_blob, expected_sha) in GAME_PINS.items():
        if str(_git(game_root, "status", "--porcelain=v1", "--", path)):
            raise CatalogError(f"PROTECTED_INPUT_DIRTY: {path}")
        blob, payload = _blob(game_root, ADMISSION_MAIN, path, expected_blob)
        if sha256_bytes(payload) != expected_sha:
            raise CatalogError(f"PROTECTED_INPUT_SHA256_MISMATCH: {path}")
        records.append({"path": path, "blob": blob, "sha256": expected_sha, "size": len(payload)})
    b1_blob, b1_payload = _blob(game_root, ADMISSION_MAIN, B1_EVIDENCE_PATH, B1_EVIDENCE_BLOB)
    b3_blob, b3_payload = _blob(game_root, ADMISSION_MAIN, B3_EVIDENCE_PATH, B3_EVIDENCE_BLOB)
    b1 = _verify_product(b1_payload, B1_PRODUCT_DIGEST, "B1")
    b3 = _verify_product(b3_payload, B3_PRODUCT_DIGEST, "B3")
    if b1.get("semantic_catalog", {}).get("counts", {}).get("native_RESOLVED") != 0:
        raise CatalogError("B1_NATIVE_COUNT_MISMATCH")
    if b3.get("counts", {}).get("admitted_b2_loot_rows") != 17086:
        raise CatalogError("B3_PROTECTED_COUNT_MISMATCH")
    protected = {
        "admission_main": ADMISSION_MAIN,
        "producer_files": sorted(records, key=lambda value: value["path"]),
        "b1": {"evidence_path": B1_EVIDENCE_PATH, "evidence_blob": b1_blob, "evidence_size": len(b1_payload), "evidence_sha256": sha256_bytes(b1_payload), "product_digest_sha256": B1_PRODUCT_DIGEST},
        "b3": {"evidence_path": B3_EVIDENCE_PATH, "evidence_blob": b3_blob, "evidence_size": len(b3_payload), "evidence_sha256": sha256_bytes(b3_payload), "product_digest_sha256": B3_PRODUCT_DIGEST},
        "preserved_static_creature_semantic_digest": STATIC_CREATURE_SEMANTIC_DIGEST,
    }
    return protected, b1, b3


def _tree_paths(repo: Path, root: str) -> list[str]:
    raw = _git(repo, "ls-tree", "-r", "-z", "--name-only", SOURCE_REVISION, "--", root, binary=True)
    assert isinstance(raw, bytes)
    return sorted(value.decode("utf-8") for value in raw.split(b"\0") if value)


def _aggregate(records: Iterable[dict[str, Any]], payloads: dict[str, bytes]) -> str:
    digest = hashlib.sha256()
    for record in sorted(records, key=lambda value: str(value["path"])):
        payload = payloads[str(record["path"])]
        digest.update(str(record["path"]).encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(record["blob"]).encode("ascii"))
        digest.update(b"\0")
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    return digest.hexdigest()


def verify_source(source_repo: Path) -> tuple[dict[str, Any], dict[str, bytes]]:
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
    npc_paths = [path for path in _tree_paths(source_repo, NPC_ROOT) if path.endswith(".lua")]
    world_paths = [path for path in _tree_paths(source_repo, WORLD_ROOT) if path.endswith("-npc.xml")]
    records: list[dict[str, Any]] = []
    payloads: dict[str, bytes] = {}
    for role, paths in (("NPC_DEFINITION_LUA", npc_paths), ("NPC_PLACEMENT_XML", world_paths)):
        for path in paths:
            blob, payload = _blob(source_repo, SOURCE_REVISION, path)
            payloads[path] = payload
            records.append({"path": path, "blob": blob, "size": len(payload), "sha256": sha256_bytes(payload), "role": role})
    semantic_records = []
    for path, (expected_blob, role) in SEMANTIC_FILES.items():
        blob, payload = _blob(source_repo, SOURCE_REVISION, path, expected_blob)
        payloads[path] = payload
        record = {"path": path, "blob": blob, "size": len(payload), "sha256": sha256_bytes(payload), "role": role}
        records.append(record)
        semantic_records.append(record)
    if len(npc_paths) != 1112 or len(world_paths) != 3:
        raise CatalogError("SOURCE_FILE_COUNT_MISMATCH")
    aggregate = _aggregate(records, payloads)
    npc_world_records = [value for value in records if value["role"] in {"NPC_DEFINITION_LUA", "NPC_PLACEMENT_XML"}]
    npc_world = _aggregate(npc_world_records, payloads)
    if aggregate != SOURCE_AGGREGATE or npc_world != NPC_WORLD_AGGREGATE:
        raise CatalogError("SOURCE_AGGREGATE_MISMATCH")
    primary = next(value for value in records if value["path"] == PRIMARY_WORLD_FILE)
    if (primary["blob"], primary["size"], primary["sha256"]) != (PRIMARY_WORLD_BLOB, PRIMARY_WORLD_SIZE, PRIMARY_WORLD_SHA256):
        raise CatalogError("PRIMARY_WORLD_FILE_MISMATCH")
    snapshot = {
        "repository": SOURCE_REPOSITORY, "revision": SOURCE_REVISION,
        "classification": SOURCE_CLASSIFICATION,
        "consumed_roots": [NPC_ROOT, WORLD_ROOT],
        "selection": {NPC_ROOT: "**/*.lua", WORLD_ROOT: "**/*-npc.xml"},
        "file_counts": {"npc_definition_lua": len(npc_paths), "npc_placement_xml": len(world_paths), "semantic": len(semantic_records), "total": len(records)},
        "aggregate_algorithm": "sha256(sorted(path UTF-8 + NUL + git blob ASCII + NUL + uint64be size + exact blob bytes))",
        "aggregate_source_bytes_sha256": aggregate,
        "npc_world_aggregate_source_bytes_sha256": npc_world,
        "primary_world_file": primary,
        "files": sorted(records, key=lambda value: value["path"]),
    }
    return snapshot, payloads


def _load(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CatalogError(f"MODULE_LOAD_FAILED: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def load_protected_modules(game_root: Path) -> tuple[Any, Any, Any, Any]:
    identity = _load(game_root / "tools/game-atlas-creatures/identity.py", "identity")
    physical = _load(game_root / "tools/game-atlas-creatures/export.py", "cw2_b5_physical")
    gameplay = _load(game_root / "tools/game-atlas-creature-gameplay/export.py", "cw2_b5_gameplay")
    b1_module = _load(game_root / "tools/reference-world-corridor-census/item_identity_catalog.py", "cw2_b5_b1")
    b3_module = _load(game_root / "tools/reference-world-corridor-census/loot_item_binding_catalog.py", "cw2_b5_b3")
    return physical, gameplay, b1_module, b3_module


def _materialize(root: Path, records: list[dict[str, Any]], payloads: dict[str, bytes]) -> None:
    for record in records:
        path = root / str(record["path"])
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payloads[str(record["path"])])


def dialogue_state(profile_id: str, source_refs: list[str], *, translate_runtime: bool = False) -> dict[str, Any]:
    if translate_runtime:
        raise CatalogError("DIALOGUE_RUNTIME_TRANSLITERATION_FORBIDDEN")
    return {"profile_id": profile_id, "state": "UNKNOWN", "support_state": "UNSUPPORTED", "reason_codes": ["DIALOGUE_RUNTIME_TRANSLITERATION_FORBIDDEN", "NO_STATIC_DIALOGUE_PROJECTION"], "source_file_refs": sorted(source_refs)}


def unresolved_native_npc(
    *,
    proposed_key: str | None = None,
    source: str,
    identity_inputs: dict[str, object] | None = None,
) -> dict[str, Any]:
    if proposed_key is not None or identity_inputs:
        raise CatalogError("NATIVE_NPC_IDENTITY_PROMOTION_FORBIDDEN")
    return {"disposition": "UNRESOLVED", "reason_code": "NO_EXPLICIT_PROTECTED_NATIVE_NPC_BINDING", "source_identity": source}


def parse_static_sections(payload: bytes, gameplay: Any) -> dict[str, Any]:
    try:
        text = gameplay._strip_line_comments(payload.decode("utf-8"))
    except UnicodeDecodeError as exc:
        raise CatalogError("NPC_SOURCE_UTF8_INVALID") from exc
    shop = gameplay._shop_profile(text, {})
    travel = gameplay._travel_profile(text)
    return {"shop": shop, "travel": travel, "services": gameplay._services_profile(text, shop, travel)}


def _count_states(records: Iterable[dict[str, Any]], key: str = "state") -> dict[str, int]:
    return dict(sorted(Counter(str(value[key]) for value in records).items()))


def _require_counts(actual: dict[str, int], expected: dict[str, int], label: str) -> None:
    complete = {key: actual.get(key, 0) for key in expected}
    if complete != expected or sum(actual.values()) != sum(expected.values()):
        raise CatalogError(f"{label}_PARTITION_MISMATCH: {actual}")


def build_catalog(source_snapshot: dict[str, Any], payloads: dict[str, bytes], protected: dict[str, Any], b1: dict[str, Any], game_root: Path, *, reverse_enumeration: bool = False) -> dict[str, Any]:
    physical, gameplay, b1_module, b3_module = load_protected_modules(game_root)
    source_records = list(source_snapshot["files"])
    if reverse_enumeration:
        source_records.reverse()
    npc_records = [value for value in source_records if value["role"] == "NPC_DEFINITION_LUA"]
    world_records = [value for value in source_records if value["role"] == "NPC_PLACEMENT_XML"]
    with tempfile.TemporaryDirectory(prefix="cw2-b5-") as raw_temp:
        temp = Path(raw_temp)
        _materialize(temp, npc_records + world_records, payloads)
        npc_root = temp / NPC_ROOT
        world_root = temp / WORLD_ROOT
        empty_monsters = temp / "empty-monsters"
        empty_monsters.mkdir()
        gameplay_product = gameplay.export_gameplay_profiles(npc_root, empty_monsters)
        physical_product = physical.export_creatures(world_root, npc_root, empty_monsters)

        file_by_rel = {str(value["path"])[len(NPC_ROOT) + 1:]: value for value in npc_records}
        candidates: dict[str, list[dict[str, Any]]] = defaultdict(list)
        definitions = []
        for path in sorted(npc_root.rglob("*.lua"), key=lambda value: value.relative_to(npc_root).as_posix(), reverse=reverse_enumeration):
            rel = path.relative_to(npc_root).as_posix()
            source = file_by_rel[rel]
            source_ref = stable_id("npc-definition-source", source["path"], source["blob"])
            profile = gameplay._npc_profile(path, {})
            parsed = physical._parse_definition(path, "npc")
            record: dict[str, Any] = {
                "definition_source_id": source_ref, "source_path": source["path"], "source_blob": source["blob"],
                "source_sha256": source["sha256"], "parse_state": "PARSED" if profile is not None and parsed is not None else "UNPARSED",
                "evidence_status": EVIDENCE_STATUS,
            }
            if profile is None or parsed is None:
                record["reason_codes"] = ["PROTECTED_EXPORTER_DEFINITION_UNPARSED"]
                record["native_mapping"] = unresolved_native_npc(source=source_ref)
            else:
                name, definition = parsed
                profile_id = str(profile["entity_id"])
                record.update({"source_declared_name": name, "candidate_profile_id": profile_id, "role_evidence": list(definition.roles), "native_mapping": unresolved_native_npc(source=source_ref)})
                candidates[profile_id].append({"source_file_ref": source_ref, "profile": profile})
            definitions.append(record)

        merged_profiles = {str(value["entity_id"]): value for value in gameplay_product["npcs"]}
        if set(merged_profiles) != set(candidates):
            raise CatalogError("PROTECTED_GAMEPLAY_PROFILE_REPLAY_MISMATCH")
        profile_sources = {key: sorted(value["source_file_ref"] for value in values) for key, values in candidates.items()}
        duplicate_groups = sum(len(values) > 1 for values in candidates.values())
        duplicate_extras = sum(max(0, len(values) - 1) for values in candidates.values())
        conflicts = sum(value["shop"]["state"] == "AMBIGUOUS" for value in merged_profiles.values())
        for record in definitions:
            profile_id = record.get("candidate_profile_id")
            if profile_id is None:
                continue
            cardinality = len(candidates[str(profile_id)])
            if cardinality == 1:
                record["identity_cardinality_state"] = "UNIQUE"
            elif merged_profiles[str(profile_id)]["shop"]["state"] == "AMBIGUOUS":
                record["identity_cardinality_state"] = "DUPLICATE_CONFLICT"
                record["reason_codes"] = ["DUPLICATE_PROFILE_CONFLICT"]
            else:
                record["identity_cardinality_state"] = "DUPLICATE_EQUIVALENT"
                record["reason_codes"] = ["DUPLICATE_SOURCE_IDENTITY_EQUIVALENT"]

        placements = []
        for value in physical_product["npcs"]:
            record_id = str(value["record_id"])
            record = {"placement_occurrence_id": record_id, "source_name": value["name"], "position": value["position"], "spawn_area": value["spawn_area"], "spawn_time_seconds": value["spawn_time_seconds"], "direction": value["direction"], "weight": value["weight"], "origin": value["origin"], "source_resolution_state": value["resolution_state"], "native_mapping": unresolved_native_npc(source=record_id), "evidence_status": EVIDENCE_STATUS}
            if "entity_id" in value:
                record["candidate_profile_id"] = value["entity_id"]
            if value.get("role_resolution_state") is not None:
                record["role_resolution_state"] = value["role_resolution_state"]
            if value.get("roles"):
                record["roles"] = value["roles"]
            reasons = []
            if value["resolution_state"] != "RESOLVED": reasons.append(f"SOURCE_PROFILE_{value['resolution_state']}")
            if value.get("role_resolution_state") == "AMBIGUOUS": reasons.append("ROLE_EVIDENCE_AMBIGUOUS")
            if reasons: record["reason_codes"] = reasons
            placements.append(record)

    b1_index = b3_module.build_b1_index(b1)
    registry = b3_module.build_runtime_item_registry(payloads["data/items/appearances.dat"], payloads["data/items/items.xml"], b1_module)
    dialogue = []
    service_records = []
    shop_profiles = []
    catalogue_rows = []
    price_records = []
    travel_profiles = []
    destinations = []
    conditions = []
    losses: Counter[str] = Counter()
    source_item_states: Counter[str] = Counter()
    native_item_states: Counter[str] = Counter()
    row_directions: Counter[str] = Counter()
    for profile_id in sorted(merged_profiles):
        profile = merged_profiles[profile_id]
        refs = profile_sources[profile_id]
        dialogue_record = dialogue_state(profile_id, refs)
        dialogue.append(dialogue_record)
        for reason in dialogue_record["reason_codes"]: losses[reason] += 1
        service = {"profile_id": profile_id, "state": profile["services"]["state"], "service_types": profile["services"]["values"], "reason_codes": profile["services"]["reason_codes"], "evidence_status": EVIDENCE_STATUS}
        service_records.append(service)
        shop = {"profile_id": profile_id, "state": profile["shop"]["state"], "reason_codes": profile["shop"]["reason_codes"], "evidence_status": EVIDENCE_STATUS}
        shop_profiles.append(shop)
        travel = {"profile_id": profile_id, "state": profile["travel"]["state"], "reason_codes": profile["travel"]["reason_codes"], "evidence_status": EVIDENCE_STATUS}
        travel_profiles.append(travel)
        for section in (service, shop, travel):
            for reason in section["reason_codes"]: losses[reason] += 1
        condition_id = stable_id("travel-condition", profile_id)
        condition = {"travel_condition_id": condition_id, "profile_id": profile_id, "state": "UNKNOWN", "support_state": "UNSUPPORTED", "reason_codes": ["TRAVEL_CONDITION_SEMANTICS_UNSUPPORTED", "PREMIUM_DISCOUNT_DYNAMIC_OR_CONDITIONAL_SEMANTICS_NOT_PRESERVED"], "evidence_status": EVIDENCE_STATUS}
        conditions.append(condition)
        for reason in condition["reason_codes"]: losses[reason] += 1
        for destination_index, value in enumerate(profile["travel"]["destinations"]):
            destination_id = stable_id("travel-destination", profile_id, destination_index, canonical_bytes(value).decode("utf-8"))
            destinations.append({"travel_destination_id": destination_id, "profile_id": profile_id, "label": value["label"], "position": value["position"], "declared_price": value["price"], "declared_currency": value["currency"], "condition_ref": condition_id, "evidence_status": EVIDENCE_STATUS})
        for direction, rows in (("SOURCE_BUY_FIELD", profile["shop"]["sells"]), ("SOURCE_SELL_FIELD", profile["shop"]["buys"])):
            for row_index, value in enumerate(rows):
                row_id = stable_id("shop-catalogue-row", profile_id, direction, row_index, canonical_bytes(value).decode("utf-8"))
                source_resolution = b3_module.resolve_source_item(item_name=value.get("item_name"), server_item_id=None, client_id=None, runtime_registry=registry, b1_index=b1_index)
                native_resolution = b3_module.resolve_native_item(source_resolution, b1_index)
                source_item_states[source_resolution["disposition"]] += 1
                native_item_states[native_resolution["disposition"]] += 1
                row_directions[direction] += 1
                catalogue_rows.append({"shop_catalogue_row_id": row_id, "profile_id": profile_id, "source_direction_field": direction, "source_item_name": value["item_name"], "source_item_resolution": source_resolution, "native_item_resolution": native_resolution, "price_evidence_ref": stable_id("price-evidence", row_id), "evidence_status": EVIDENCE_STATUS, "authority": "IMMUTABLE_CATALOGUE_EVIDENCE_ONLY"})
                price_records.append({"price_evidence_id": stable_id("price-evidence", row_id), "shop_catalogue_row_id": row_id, "state": "STATIC_LITERAL", "unit_price": value["unit_price"], "source_currency_label": value["currency"], "reason_codes": ["SOURCE_PRICE_ONLY_NO_CURRENCY_OR_TRADE_AUTHORITY"], "evidence_status": EVIDENCE_STATUS})
                if source_resolution["disposition"] != "RESOLVED": losses[str(source_resolution["reason_code"])] += 1
                if native_resolution["disposition"] != "RESOLVED": losses[str(native_resolution["reason_code"])] += 1

    definitions.sort(key=lambda value: value["definition_source_id"])
    placements.sort(key=lambda value: value["placement_occurrence_id"])
    catalogue_rows.sort(key=lambda value: value["shop_catalogue_row_id"])
    price_records.sort(key=lambda value: value["price_evidence_id"])
    destinations.sort(key=lambda value: value["travel_destination_id"])
    parsed = sum(value["parse_state"] == "PARSED" for value in definitions)
    role_counts = Counter(role for value in placements for role in value.get("roles", []))
    counts = {
        "definition_files": len(definitions), "parsed_definitions": parsed, "unparsed_definitions": len(definitions) - parsed,
        "profile_identities": len(merged_profiles), "duplicate_identity_groups": duplicate_groups,
        "duplicate_definition_extras": duplicate_extras, "conflicting_profiles": conflicts,
        "placements": len(placements), "placements_with_resolved_roles": sum(bool(value.get("roles")) for value in placements),
        "role_ambiguity": sum(value.get("role_resolution_state") == "AMBIGUOUS" for value in placements),
        "shop_rows": len(catalogue_rows), "shop_source_buy_field_rows": row_directions["SOURCE_BUY_FIELD"], "shop_source_sell_field_rows": row_directions["SOURCE_SELL_FIELD"],
        "price_records": len(price_records), "travel_destinations": len(destinations), "travel_conditions": len(conditions), "dialogue_records": len(dialogue), "service_records": len(service_records),
    }
    for key, expected in EXPECTED_COUNTS.items():
        if counts[key] != expected: raise CatalogError(f"EXPECTED_COUNT_MISMATCH: {key}: {counts[key]} != {expected}")
    if dict(role_counts) != EXPECTED_ROLE_OCCURRENCES: raise CatalogError(f"ROLE_OCCURRENCE_MISMATCH: {dict(role_counts)}")
    _require_counts(_count_states(shop_profiles), EXPECTED_SHOP_STATES, "SHOP_STATE")
    _require_counts(_count_states(service_records), EXPECTED_SERVICE_STATES, "SERVICE_STATE")
    _require_counts(_count_states(travel_profiles), EXPECTED_TRAVEL_STATES, "TRAVEL_STATE")
    _require_counts(dict(source_item_states), EXPECTED_SOURCE_ITEM_STATES, "SOURCE_ITEM")
    _require_counts(dict(native_item_states), EXPECTED_NATIVE_ITEM_STATES, "NATIVE_ITEM")
    value: dict[str, Any] = {
        "schema": SCHEMA, "mapper": {"profile": MAPPER_PROFILE, "path": "tools/reference-world-corridor-census/npc_service_binding_catalog.py"}, "task": TASK,
        "classification": EVIDENCE_STATUS, "closure": CLOSURE,
        "authority": {"reference_claims_proven": 0, "production_authority": "NONE", "reference_parity_claim": "NONE", "lua_execution": "FORBIDDEN", "dialogue_runtime_transliteration": "FORBIDDEN", "trade_or_currency_mutation": "OUT_OF_SCOPE", "licensing_and_access": {"state": "RECORDED_NON_BLOCKING", "basis": "protected PR #693"}},
        "identity_rules": {"native_npc_definition_mapping": "UNRESOLVED_WITHOUT_EXPLICIT_PROTECTED_BINDING", "native_placement_mapping": "UNRESOLVED_WITHOUT_EXPLICIT_PROTECTED_BINDING", "forbidden_identity_inputs": ["source_name", "normalized_name", "numeric_id", "display_id", "client_id", "atlas_entity_id", "record_id", "hash", "outfit", "coordinates"]},
        "source_snapshot": source_snapshot, "protected_inputs": protected,
        "definition_identity": {"records": definitions, "partition": {"PARSED": parsed, "UNPARSED": len(definitions) - parsed}},
        "placement_occurrences": {"records": placements, "partition": _count_states(placements, "source_resolution_state")},
        "dialogue": {"records": dialogue, "partition": {"UNKNOWN_UNSUPPORTED": len(dialogue)}},
        "service_types": {"records": service_records, "partition": _count_states(service_records)},
        "shop_catalogues": {"profiles": shop_profiles, "state_partition": _count_states(shop_profiles), "rows": catalogue_rows, "source_item_partition": {key: source_item_states.get(key, 0) for key in ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT")}, "native_item_partition": {key: native_item_states.get(key, 0) for key in ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT")}},
        "price_evidence": {"records": price_records, "partition": {"STATIC_LITERAL": len(price_records)}},
        "travel_destinations": {"profiles": travel_profiles, "state_partition": _count_states(travel_profiles), "records": destinations},
        "travel_conditions": {"records": conditions, "partition": {"UNKNOWN_UNSUPPORTED": len(conditions)}},
        "role_occurrences": dict(sorted(role_counts.items())), "loss_reason_counts": dict(sorted(losses.items())), "counts": counts,
        "count_invariants": [
            "1112 = 1103 PARSED + 9 UNPARSED definitions", "1103 = 1049 unique profiles + 54 duplicate extras",
            "11800 = 5744 SOURCE_BUY_FIELD + 6056 SOURCE_SELL_FIELD shop rows",
            "11800 = 9355 RESOLVED + 791 UNRESOLVED + 1654 AMBIGUOUS + 0 CONFLICT source-item joins",
            "11800 = 0 RESOLVED + 11800 UNRESOLVED + 0 AMBIGUOUS + 0 CONFLICT native-item joins",
            "1049 = 254 COMPLETE + 51 PARTIAL + 732 UNKNOWN + 12 AMBIGUOUS shop profiles",
            "1049 = 360 PARTIAL + 677 UNKNOWN + 12 AMBIGUOUS service profiles",
            "1049 = 5 COMPLETE + 43 PARTIAL + 989 UNKNOWN + 12 AMBIGUOUS travel profiles",
            "1049 dialogue and 1049 travel-condition records remain UNKNOWN/UNSUPPORTED",
        ],
    }
    value["product_digest_sha256"] = sha256_bytes(canonical_bytes(value))
    value["product_digest_scope"] = "canonical JSON with product_digest fields omitted"
    return value


def build_evidence(source_repo: Path, game_root: Path, *, reverse_enumeration: bool = False) -> dict[str, Any]:
    protected, b1, _b3 = verify_game_inputs(game_root)
    snapshot, payloads = verify_source(source_repo)
    return build_catalog(snapshot, payloads, protected, b1, game_root.resolve(), reverse_enumeration=reverse_enumeration)


def _nested(value: dict[str, Any], keys: tuple[str, ...]) -> dict[str, Any]:
    current = value
    for key in keys:
        child = current.get(key)
        if not isinstance(child, dict):
            raise CatalogError(f"SHARD_SECTION_MISSING: {'.'.join(keys)}")
        current = child
    return current


def _shard_partition(records: list[dict[str, Any]]) -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for field in (
        "role", "parse_state", "identity_cardinality_state",
        "source_resolution_state", "role_resolution_state", "state",
        "support_state", "source_direction_field",
    ):
        counts = Counter(str(record[field]) for record in records if field in record)
        if counts:
            result[field] = dict(sorted(counts.items()))
    for field in ("source_item_resolution", "native_item_resolution"):
        counts = Counter(
            str(record[field]["disposition"])
            for record in records
            if isinstance(record.get(field), dict) and "disposition" in record[field]
        )
        if counts:
            result[field] = dict(sorted(counts.items()))
    return result


def _shard_value(
    logical_digest: str,
    section: str,
    ordinal: int,
    total: int,
    records: list[dict[str, Any]],
) -> dict[str, Any]:
    value = {
        "schema": SHARD_SCHEMA,
        "logical_product_digest_sha256": logical_digest,
        "section": section,
        "ordinal": ordinal,
        "total_shards_for_section": total,
        "record_count": len(records),
        "partition": _shard_partition(records),
        "records": records,
        "payload_digest_sha256": sha256_bytes(canonical_bytes(records)),
    }
    return value


def shard_evidence(evidence: dict[str, Any]) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    logical_digest = evidence.get("product_digest_sha256")
    if logical_digest != "aa08643f21e4da424290599d7c6cc4527421c2f021e6441789c1691ba5f9fe14":
        raise CatalogError(f"LOGICAL_PRODUCT_DIGEST_MISMATCH: {logical_digest}")
    index = copy.deepcopy(evidence)
    shard_values: dict[str, dict[str, Any]] = {}
    manifest: list[dict[str, Any]] = []
    shard_root = "docs/agents/evidence/OTV2-20260920-content-world-cw2-b5-npc-service-bindings"
    for section, keys, record_key in SHARD_COLLECTIONS:
        records = _nested(index, keys).pop(record_key)
        if not isinstance(records, list) or not all(isinstance(record, dict) for record in records):
            raise CatalogError(f"SHARD_RECORDS_INVALID: {section}")
        chunks: list[list[dict[str, Any]]] = []
        current: list[dict[str, Any]] = []
        approximate = 0
        for record in records:
            record_size = len(canonical_bytes(record)) + 1
            if current and approximate + record_size > SHARD_TARGET_BYTES:
                chunks.append(current)
                current = []
                approximate = 0
            current.append(record)
            approximate += record_size
        if current:
            chunks.append(current)
        while True:
            oversized = None
            for offset, chunk in enumerate(chunks):
                if len(canonical_bytes(_shard_value(logical_digest, section, 1, len(chunks), chunk))) > MAX_SHARD_BYTES:
                    oversized = offset
                    break
            if oversized is None:
                break
            chunk = chunks.pop(oversized)
            if len(chunk) < 2:
                raise CatalogError(f"SINGLE_RECORD_EXCEEDS_SHARD_LIMIT: {section}")
            midpoint = len(chunk) // 2
            chunks[oversized:oversized] = [chunk[:midpoint], chunk[midpoint:]]
        if len(chunks) != EXPECTED_SHARD_COUNTS[section]:
            raise CatalogError(f"SHARD_COUNT_MISMATCH: {section}: {len(chunks)}")
        for ordinal, records_chunk in enumerate(chunks, 1):
            shard = _shard_value(logical_digest, section, ordinal, len(chunks), records_chunk)
            shard_bytes = canonical_bytes(shard)
            if len(shard_bytes) > MAX_SHARD_BYTES:
                raise CatalogError(f"SHARD_SIZE_LIMIT_EXCEEDED: {section}:{ordinal}")
            shard_path = f"{shard_root}/{section}-{ordinal:04d}.json"
            shard_values[shard_path] = shard
            manifest.append(
                {
                    "path": shard_path,
                    "section": section,
                    "ordinal": ordinal,
                    "record_count": len(records_chunk),
                    "partition": shard["partition"],
                    "payload_digest_sha256": shard["payload_digest_sha256"],
                    "file_sha256": sha256_bytes(shard_bytes),
                    "size": len(shard_bytes),
                }
            )
    index["logical_product_digest_sha256"] = index.pop("product_digest_sha256")
    index["logical_product_digest_scope"] = index.pop("product_digest_scope")
    index["storage_layout"] = {
        "schema": SHARDED_STORAGE_SCHEMA,
        "max_shard_bytes": MAX_SHARD_BYTES,
        "shard_count": len(manifest),
        "shards": manifest,
    }
    index["index_digest_sha256"] = sha256_bytes(canonical_bytes(index))
    index["index_digest_scope"] = "canonical index JSON with index_digest fields omitted"
    if len(shard_values) != 35:
        raise CatalogError(f"TOTAL_SHARD_COUNT_MISMATCH: {len(shard_values)}")
    return index, shard_values


def reconstruct_evidence(index: dict[str, Any], shards: dict[str, dict[str, Any]]) -> dict[str, Any]:
    basis = copy.deepcopy(index)
    recorded_index_digest = basis.pop("index_digest_sha256", None)
    basis.pop("index_digest_scope", None)
    if recorded_index_digest != sha256_bytes(canonical_bytes(basis)):
        raise CatalogError("INDEX_DIGEST_MISMATCH")
    layout = basis.pop("storage_layout", None)
    if not isinstance(layout, dict) or layout.get("schema") != SHARDED_STORAGE_SCHEMA:
        raise CatalogError("STORAGE_LAYOUT_INVALID")
    manifest = layout.get("shards")
    if not isinstance(manifest, list) or len(manifest) != 35:
        raise CatalogError("SHARD_MANIFEST_INVALID")
    if set(shards) != {str(value.get("path")) for value in manifest}:
        raise CatalogError("SHARD_PATH_SET_MISMATCH")
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for entry in manifest:
        path = str(entry["path"])
        shard = shards[path]
        shard_bytes = canonical_bytes(shard)
        if len(shard_bytes) > MAX_SHARD_BYTES or len(shard_bytes) != entry["size"]:
            raise CatalogError(f"SHARD_SIZE_MISMATCH: {path}")
        if sha256_bytes(shard_bytes) != entry["file_sha256"]:
            raise CatalogError(f"SHARD_FILE_DIGEST_MISMATCH: {path}")
        records = shard.get("records")
        if not isinstance(records, list):
            raise CatalogError(f"SHARD_RECORDS_INVALID: {path}")
        if shard.get("payload_digest_sha256") != sha256_bytes(canonical_bytes(records)):
            raise CatalogError(f"SHARD_PAYLOAD_DIGEST_MISMATCH: {path}")
        for field in ("section", "ordinal", "record_count", "partition", "payload_digest_sha256"):
            if shard.get(field) != entry.get(field):
                raise CatalogError(f"SHARD_MANIFEST_BINDING_MISMATCH: {path}:{field}")
        if shard.get("schema") != SHARD_SCHEMA or shard.get("logical_product_digest_sha256") != basis.get("logical_product_digest_sha256"):
            raise CatalogError(f"SHARD_HEADER_MISMATCH: {path}")
        grouped[str(entry["section"])].append(shard)
    for section, keys, record_key in SHARD_COLLECTIONS:
        ordered = sorted(grouped[section], key=lambda value: int(value["ordinal"]))
        if [int(value["ordinal"]) for value in ordered] != list(range(1, len(ordered) + 1)):
            raise CatalogError(f"SHARD_ORDINAL_MISMATCH: {section}")
        _nested(basis, keys)[record_key] = [record for shard in ordered for record in shard["records"]]
    basis["product_digest_sha256"] = basis.pop("logical_product_digest_sha256")
    basis["product_digest_scope"] = basis.pop("logical_product_digest_scope")
    digest_basis = dict(basis)
    recorded_product = digest_basis.pop("product_digest_sha256")
    digest_basis.pop("product_digest_scope")
    if recorded_product != sha256_bytes(canonical_bytes(digest_basis)):
        raise CatalogError("RECONSTRUCTED_PRODUCT_DIGEST_MISMATCH")
    return basis


def write_evidence(path: Path, evidence: dict[str, Any]) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    index, shards = shard_evidence(evidence)
    path.parent.mkdir(parents=True, exist_ok=True)
    shard_dir = path.with_suffix("")
    shard_dir.mkdir(parents=True, exist_ok=True)
    expected_names = {Path(value).name for value in shards}
    existing_names = {value.name for value in shard_dir.glob("*.json")}
    if existing_names - expected_names:
        raise CatalogError(f"UNEXPECTED_EXISTING_SHARDS: {sorted(existing_names - expected_names)}")
    for repository_path, shard in sorted(shards.items()):
        (shard_dir / Path(repository_path).name).write_bytes(canonical_bytes(shard))
    path.write_bytes(canonical_bytes(index))
    return index, shards


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-repo", type=Path, required=True)
    parser.add_argument("--game-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    evidence = build_evidence(args.source_repo, args.game_root)
    index, shards = write_evidence(args.output, evidence)
    print(json.dumps({"output": str(args.output), "logical_digest": evidence["product_digest_sha256"], "index_digest": index["index_digest_sha256"], "shards": len(shards), "max_shard_bytes": max(len(canonical_bytes(value)) for value in shards.values()), "counts": evidence["counts"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
