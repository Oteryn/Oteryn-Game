#!/usr/bin/env python3
"""CW2-B3 deterministic loot-row to B1/native item binding catalogue.

This batch is candidate/source evidence only. It follows the exact pinned
Otheryn loot registration semantics far enough to recover server-item source
identity, then requires exact canonical source-node equality with protected B1
before calling the source-item join RESOLVED. Native identity is copied only
from the protected B1 disposition; this module never mints an Oteryn ItemKey.
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
from typing import Any, Iterable
import xml.etree.ElementTree as ET

SCHEMA = "OTERYN_CW2_LOOT_ITEM_BINDING_BATCH/v1"
MAPPER_PROFILE = "OTERYN_CW2_LOOT_ITEM_BINDING_MAPPER/v1"
TASK = "CW2-B3 LOOT_TO_NATIVE_ITEMKEY_BINDING_BATCH_504"
ADMISSION_MAIN = "c7688069bc22ac3cde46e48e6b05d8051418fed1"
CLOSURE = "CANDIDATE_ONLY"
SOURCE_REPOSITORY = "blakinio/Otheryn"
SOURCE_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
SOURCE_CLASSIFICATION = "OTERYN_LEGACY / MIGRATION_EVIDENCE"
EVIDENCE_STATUS = "OTS_HYPOTHESIS_ONLY"

B1_PR = 670
B1_HEAD = "4fea32f7d7efc604eeef8a0dc7ed79341eb7d972"
B1_MERGE = "715a22f26f6ec5472597f63cf5d6b939d7583cc1"
B1_PRODUCT_DIGEST = "d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc"
B1_EVIDENCE_PATH = (
    "docs/agents/evidence/"
    "OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
)
B1_EVIDENCE_BLOB = "2f0121f3ea6586477b4535840b9a1f1bc28c677c"
B1_TOOL_PATH = "tools/reference-world-corridor-census/item_identity_catalog.py"
B1_TOOL_BLOB = "904d62e1277ceae76434f75bca104686a7303ff2"

B2_PR = 674
B2_HEAD = "0086702ebb2477ade22c82a65b9ad48f80fceee1"
B2_MERGE = ADMISSION_MAIN
B2_PRODUCT_DIGEST = "f69a941a4953c9f3b2532e90ab6589c7a89126b1e8123b0ff2e8d88777f7853f"
B2_EVIDENCE_PATH = (
    "docs/agents/evidence/"
    "OTV2-20260919-content-world-cw2-b2-creature-spawn-bindings.json"
)
B2_EVIDENCE_BLOB = "b631fa51a4dfa958d11da399e59c07292bffa8c8"
B2_TOOL_PATH = "tools/reference-world-corridor-census/creature_spawn_binding_catalog.py"
B2_TOOL_BLOB = "fdbccc8674ad166f1f18117eabcd804c50dad2b5"

GAMEPLAY_PATH = "tools/game-atlas-creature-gameplay/export.py"
GAMEPLAY_BLOB = "8b8606bb6052dde3ce16ad3cfe0eb88290351b14"

SOURCE_SEMANTIC_FILES: dict[str, tuple[str, str]] = {
    "data/items/appearances.dat": (
        "2cc2f4910af4f002f99f39e486d1a91b1b56a728",
        "RUNTIME_ITEM_NAME_BASE",
    ),
    "data/items/items.xml": (
        "ce465362121def9e27ec48c7288f8125580c2910",
        "RUNTIME_ITEM_XML_OVERRIDE_AND_CROSSWALK",
    ),
    "data/scripts/lib/register_monster_type.lua": (
        "6bb42593a2e529e25bb1e5b7df21a288a64ffa33",
        "LOOT_SELECTOR_PRECEDENCE",
    ),
    "src/items/items.cpp": (
        "eecbf060ef9e85b452bb897277ff3d3d013a0cf9",
        "RUNTIME_ITEM_NAME_REGISTRY",
    ),
    "src/protobuf/appearances.proto": (
        "f39de45976e372b2306f6ff9c4a40412f4debe4e",
        "APPEARANCE_OBJECT_SCHEMA",
    ),
    "src/lua/functions/creatures/monster/loot_functions.cpp": (
        "f8a8749b45382683e9e36fee9fc9a7ae1125d6ba",
        "LOOT_ID_AND_NAME_RESOLUTION",
    ),
    "src/canary_server.cpp": (
        "fcd0704a4ab0647d28655a88c0a771044c4a7269",
        "APPEARANCE_SOURCE_PATH",
    ),
}

RESOLUTION_STATES = ("RESOLVED", "UNRESOLVED", "AMBIGUOUS", "CONFLICT")


class CatalogError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_repository_text_bytes(value: bytes) -> bytes:
    """Canonicalize checkout line endings to repository LF bytes."""
    without_crlf = value.replace(b"\r\n", b"")
    if b"\r" in without_crlf:
        raise CatalogError("UNSUPPORTED_MAPPER_LINE_ENDING")
    return value.replace(b"\r\n", b"\n")


def canonical_record_list(
    records: Iterable[dict[str, Any]],
) -> list[dict[str, Any]]:
    return sorted((dict(record) for record in records), key=canonical_bytes)


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


def _git_object_blob(
    repo: Path,
    revision: str,
    path: str,
    expected_blob: str | None = None,
) -> tuple[str, bytes]:
    try:
        blob = str(_git(repo, "rev-parse", f"{revision}:{path}"))
        payload = _git(repo, "cat-file", "blob", blob, binary=True)
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError(f"GIT_OBJECT_UNAVAILABLE: {revision}:{path}") from exc
    assert isinstance(payload, bytes)
    if expected_blob is not None and blob != expected_blob:
        raise CatalogError(
            f"GIT_BLOB_MISMATCH: {path}: expected {expected_blob}, got {blob}"
        )
    return blob, payload


def _verify_product_digest(value: dict[str, Any], expected: str, label: str) -> None:
    copy = dict(value)
    actual = copy.pop("product_digest_sha256", None)
    copy.pop("product_digest_scope", None)
    recomputed = sha256_bytes(canonical_bytes(copy))
    if actual != expected or recomputed != expected:
        raise CatalogError(
            f"{label}_PRODUCT_DIGEST_MISMATCH: expected {expected}, "
            f"recorded {actual}, recomputed {recomputed}"
        )


def _load_json_blob(payload: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CatalogError(f"{label}_JSON_INVALID") from exc
    if not isinstance(value, dict):
        raise CatalogError(f"{label}_JSON_ROOT_INVALID")
    return value


def verify_game_products(
    game_root: Path,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    game_root = game_root.resolve()
    try:
        top = Path(str(_git(game_root, "rev-parse", "--show-toplevel"))).resolve()
        admission = str(_git(game_root, "rev-parse", ADMISSION_MAIN))
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("GAME_REPOSITORY_UNVERIFIABLE") from exc
    if top != game_root:
        raise CatalogError("GAME_REPOSITORY_ROOT_MISMATCH")
    if admission != ADMISSION_MAIN:
        raise CatalogError("ADMISSION_MAIN_UNAVAILABLE")

    protected_paths = {
        B1_EVIDENCE_PATH: B1_EVIDENCE_BLOB,
        B2_EVIDENCE_PATH: B2_EVIDENCE_BLOB,
        B1_TOOL_PATH: B1_TOOL_BLOB,
        B2_TOOL_PATH: B2_TOOL_BLOB,
        GAMEPLAY_PATH: GAMEPLAY_BLOB,
    }
    protected_files: list[dict[str, Any]] = []
    payload_by_path: dict[str, bytes] = {}
    for path, expected_blob in protected_paths.items():
        status = str(_git(game_root, "status", "--porcelain=v1", "--", path))
        if status:
            raise CatalogError(f"PROTECTED_INPUT_DIRTY: {path}")
        blob, payload = _git_object_blob(
            game_root, ADMISSION_MAIN, path, expected_blob
        )
        protected_files.append(
            {
                "path": path,
                "blob": blob,
                "size": len(payload),
                "sha256": sha256_bytes(payload),
            }
        )
        payload_by_path[path] = payload

    for head, path, expected_blob, label in (
        (B1_HEAD, B1_EVIDENCE_PATH, B1_EVIDENCE_BLOB, "B1"),
        (B2_HEAD, B2_EVIDENCE_PATH, B2_EVIDENCE_BLOB, "B2"),
    ):
        try:
            head_blob = str(_git(game_root, "rev-parse", f"{head}:{path}"))
        except (OSError, subprocess.CalledProcessError) as exc:
            raise CatalogError(f"{label}_PRODUCER_HEAD_UNAVAILABLE") from exc
        if head_blob != expected_blob:
            raise CatalogError(
                f"{label}_PRODUCER_HEAD_EVIDENCE_MISMATCH: {head_blob}"
            )

    for merge, label in ((B1_MERGE, "B1"), (B2_MERGE, "B2")):
        try:
            merge_obj = str(_git(game_root, "rev-parse", f"{merge}^{{commit}}"))
        except (OSError, subprocess.CalledProcessError) as exc:
            raise CatalogError(f"{label}_MERGE_COMMIT_UNAVAILABLE") from exc
        if merge_obj != merge:
            raise CatalogError(f"{label}_MERGE_COMMIT_MISMATCH")

    b1 = _load_json_blob(payload_by_path[B1_EVIDENCE_PATH], "B1_EVIDENCE")
    b2 = _load_json_blob(payload_by_path[B2_EVIDENCE_PATH], "B2_EVIDENCE")
    _verify_product_digest(b1, B1_PRODUCT_DIGEST, "B1")
    _verify_product_digest(b2, B2_PRODUCT_DIGEST, "B2")
    if b1.get("closure") != "CANDIDATE_ONLY":
        raise CatalogError("B1_CLOSURE_NOT_CANDIDATE_ONLY")
    if b2.get("closure") != "CANDIDATE_ONLY":
        raise CatalogError("B2_CLOSURE_NOT_CANDIDATE_ONLY")
    b1_counts = b1.get("semantic_catalog", {}).get("counts", {})
    if (
        b1_counts.get("total_emitted_source_identity_records") != 38157
        or b1_counts.get("native_RESOLVED") != 0
        or b1_counts.get("native_UNRESOLVED") != 38157
    ):
        raise CatalogError("B1_PROTECTED_COUNTS_MISMATCH")
    if b2.get("counts", {}).get("deferred_b3_loot_rows") != 17086:
        raise CatalogError("B2_PROTECTED_LOOT_COUNT_MISMATCH")

    summary = {
        "admission_main": ADMISSION_MAIN,
        "protected_files": protected_files,
        "b1": {
            "pr": B1_PR,
            "producer_head": B1_HEAD,
            "merge_sha": B1_MERGE,
            "evidence_path": B1_EVIDENCE_PATH,
            "evidence_blob": B1_EVIDENCE_BLOB,
            "evidence_sha256": sha256_bytes(payload_by_path[B1_EVIDENCE_PATH]),
            "product_digest_sha256": B1_PRODUCT_DIGEST,
        },
        "b2": {
            "pr": B2_PR,
            "producer_head": B2_HEAD,
            "merge_sha": B2_MERGE,
            "evidence_path": B2_EVIDENCE_PATH,
            "evidence_blob": B2_EVIDENCE_BLOB,
            "evidence_sha256": sha256_bytes(payload_by_path[B2_EVIDENCE_PATH]),
            "product_digest_sha256": B2_PRODUCT_DIGEST,
        },
    }
    return summary, b1, b2


def _load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CatalogError(f"MODULE_IMPORT_SPEC_FAILED: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def load_protected_modules(game_root: Path) -> tuple[Any, Any]:
    return (
        _load_module(game_root / B1_TOOL_PATH, "cw2_b3_b1_item_catalog"),
        _load_module(game_root / GAMEPLAY_PATH, "cw2_b3_gameplay_export"),
    )


def _source_payload(
    source_repo: Path,
    path: str,
    expected_blob: str | None = None,
) -> tuple[str, bytes]:
    return _git_object_blob(source_repo, SOURCE_REVISION, path, expected_blob)


def _semantic_source_assertions(payloads: dict[str, bytes]) -> None:
    required = {
        "src/canary_server.cpp": (
            b'loadAppearanceProtobuf(coreFolder + "/items/appearances.dat")',
        ),
        "src/items/items.cpp": (
            b"iType.name = object.name();",
            b"nameToItems.insert({ asLowerCaseString(iType.name), iType.id });",
            b"itemType.name = xmlName;",
        ),
        "src/lua/functions/creatures/monster/loot_functions.cpp": (
            b"luaLootSetter<uint16_t, &LootBlock::id>",
            b"nameToItems.equal_range(asLowerCaseString(name))",
            b"loot->lootBlock.id = ids.first->second;",
        ),
        "data/scripts/lib/register_monster_type.lua": (
            b"if loot.name then",
            b"parent:setIdFromName(loot.name)",
            b"parent:setId(loot.id)",
        ),
        "src/protobuf/appearances.proto": (
            b"repeated Appearance object = 1;",
            b"optional uint32 id = 1;",
            b"optional AppearanceFlags flags = 3;",
            b"optional bytes name = 4;",
        ),
    }
    for path, needles in required.items():
        data = payloads[path]
        for needle in needles:
            if needle not in data:
                raise CatalogError(
                    f"SOURCE_SEMANTIC_ASSERTION_FAILED: {path}: "
                    f"{needle.decode('utf-8', 'replace')}"
                )


def _batch_read_blobs(
    source_repo: Path,
    records: Iterable[dict[str, Any]],
) -> tuple[dict[str, bytes], list[dict[str, Any]]]:
    ordered = sorted((dict(record) for record in records), key=lambda x: x["path"])
    if not ordered:
        return {}, []
    request = "".join(f"{record['blob']}\n" for record in ordered).encode("ascii")
    try:
        result = subprocess.run(
            ("git", "-C", str(source_repo), "cat-file", "--batch"),
            input=request,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CatalogError("SOURCE_BATCH_READ_FAILED") from exc
    raw = result.stdout
    pos = 0
    payloads: dict[str, bytes] = {}
    verified: list[dict[str, Any]] = []
    for record in ordered:
        newline = raw.find(b"\n", pos)
        if newline < 0:
            raise CatalogError("SOURCE_BATCH_HEADER_TRUNCATED")
        header = raw[pos:newline].decode("ascii").split()
        pos = newline + 1
        if len(header) != 3 or header[1] != "blob":
            raise CatalogError(
                f"SOURCE_BATCH_OBJECT_INVALID: {record['path']}: {header}"
            )
        blob, _, size_text = header
        size = int(size_text)
        payload = raw[pos : pos + size]
        pos += size
        if raw[pos : pos + 1] != b"\n":
            raise CatalogError("SOURCE_BATCH_FRAMING_INVALID")
        pos += 1
        if blob != record["blob"]:
            raise CatalogError(
                f"SOURCE_MONSTER_BLOB_MISMATCH: {record['path']}: {blob}"
            )
        if size != int(record["size"]) or len(payload) != size:
            raise CatalogError(
                f"SOURCE_MONSTER_SIZE_MISMATCH: {record['path']}"
            )
        digest = sha256_bytes(payload)
        if digest != record["sha256"]:
            raise CatalogError(
                f"SOURCE_MONSTER_SHA256_MISMATCH: {record['path']}: {digest}"
            )
        payloads[str(record["source_file_ref"])] = payload
        verified.append(
            {
                "source_file_ref": record["source_file_ref"],
                "path": record["path"],
                "blob": blob,
                "size": size,
                "sha256": digest,
                "role": record["role"],
            }
        )
    if pos != len(raw):
        raise CatalogError("SOURCE_BATCH_TRAILING_BYTES")
    return payloads, verified


def _source_aggregate(records_and_payloads: Iterable[tuple[dict[str, Any], bytes]]) -> str:
    digest = hashlib.sha256()
    for record, payload in sorted(
        records_and_payloads, key=lambda pair: str(pair[0]["path"])
    ):
        digest.update(str(record["path"]).encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(record["blob"]).encode("ascii"))
        digest.update(b"\0")
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    return digest.hexdigest()


def verify_source_repository(
    source_repo: Path,
    b2: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, bytes], dict[str, bytes]]:
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
    b2_snapshot = b2.get("source_snapshot", {})
    if (
        b2_snapshot.get("repository") != SOURCE_REPOSITORY
        or b2_snapshot.get("revision") != SOURCE_REVISION
    ):
        raise CatalogError("B2_SOURCE_SNAPSHOT_IDENTITY_MISMATCH")

    semantic_payloads: dict[str, bytes] = {}
    semantic_records: list[dict[str, Any]] = []
    for path, (expected_blob, role) in SOURCE_SEMANTIC_FILES.items():
        blob, payload = _source_payload(source_repo, path, expected_blob)
        semantic_payloads[path] = payload
        semantic_records.append(
            {
                "path": path,
                "blob": blob,
                "size": len(payload),
                "sha256": sha256_bytes(payload),
                "role": role,
            }
        )
    _semantic_source_assertions(semantic_payloads)

    definition_refs = {
        str(record["source_file_ref"])
        for record in b2.get("monster_definitions", {}).get("records", [])
    }
    source_files = {
        str(record["source_file_ref"]): record
        for record in b2_snapshot.get("files", [])
        if record.get("role") == "MONSTER_DEFINITION_LUA"
    }
    missing = sorted(definition_refs - set(source_files))
    if missing:
        raise CatalogError(
            f"B2_DEFINITION_SOURCE_REFS_MISSING: {missing[:8]}"
        )
    selected = [source_files[ref] for ref in sorted(definition_refs)]
    monster_payloads, monster_records = _batch_read_blobs(source_repo, selected)

    aggregate_inputs: list[tuple[dict[str, Any], bytes]] = []
    for record in semantic_records:
        aggregate_inputs.append((record, semantic_payloads[str(record["path"])]))
    for record in monster_records:
        aggregate_inputs.append(
            (record, monster_payloads[str(record["source_file_ref"])])
        )
    snapshot = {
        "repository": SOURCE_REPOSITORY,
        "revision": SOURCE_REVISION,
        "classification": SOURCE_CLASSIFICATION,
        "semantic_files": semantic_records,
        "monster_definition_files": monster_records,
        "direct_file_count": len(semantic_records) + len(monster_records),
        "aggregate_algorithm": (
            "sha256(path UTF-8 + NUL + blob ASCII + NUL + "
            "8-byte big-endian size + exact blob bytes), sorted by path"
        ),
        "aggregate_direct_source_bytes_sha256": _source_aggregate(aggregate_inputs),
    }
    return snapshot, semantic_payloads, monster_payloads


def _read_varint(data: bytes, offset: int) -> tuple[int, int]:
    value = 0
    shift = 0
    start = offset
    for _ in range(10):
        if offset >= len(data):
            raise CatalogError(f"PROTOBUF_VARINT_TRUNCATED: {start}")
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return value, offset
        shift += 7
    raise CatalogError(f"PROTOBUF_VARINT_TOO_LONG: {start}")


def _protobuf_fields(data: bytes) -> Iterable[tuple[int, int, int | bytes]]:
    offset = 0
    while offset < len(data):
        key, offset = _read_varint(data, offset)
        field = key >> 3
        wire = key & 7
        if field == 0:
            raise CatalogError("PROTOBUF_FIELD_ZERO")
        if wire == 0:
            value, offset = _read_varint(data, offset)
        elif wire == 1:
            if offset + 8 > len(data):
                raise CatalogError("PROTOBUF_FIXED64_TRUNCATED")
            value = data[offset : offset + 8]
            offset += 8
        elif wire == 2:
            size, offset = _read_varint(data, offset)
            if offset + size > len(data):
                raise CatalogError("PROTOBUF_BYTES_TRUNCATED")
            value = data[offset : offset + size]
            offset += size
        elif wire == 5:
            if offset + 4 > len(data):
                raise CatalogError("PROTOBUF_FIXED32_TRUNCATED")
            value = data[offset : offset + 4]
            offset += 4
        else:
            raise CatalogError(f"PROTOBUF_WIRE_UNSUPPORTED: {wire}")
        yield field, wire, value


def _appearance_base_items(appearance_bytes: bytes) -> dict[int, dict[str, Any]]:
    items: dict[int, dict[str, Any]] = {}
    for field, wire, payload in _protobuf_fields(appearance_bytes):
        if field != 1 or wire != 2 or not isinstance(payload, bytes):
            continue
        values: dict[int, list[tuple[int, int | bytes]]] = defaultdict(list)
        for inner_field, inner_wire, value in _protobuf_fields(payload):
            values[inner_field].append((inner_wire, value))
        if 1 not in values or 3 not in values:
            continue
        id_wire, raw_id = values[1][0]
        if id_wire != 0 or not isinstance(raw_id, int):
            raise CatalogError("APPEARANCE_OBJECT_ID_INVALID")
        if not 0 <= raw_id <= 65535:
            raise CatalogError(f"APPEARANCE_OBJECT_ID_OUT_OF_RANGE: {raw_id}")
        if raw_id in items:
            raise CatalogError(f"APPEARANCE_OBJECT_ID_DUPLICATE: {raw_id}")
        name = ""
        if 4 in values:
            name_wire, raw_name = values[4][0]
            if name_wire != 2 or not isinstance(raw_name, bytes):
                raise CatalogError("APPEARANCE_OBJECT_NAME_INVALID")
            try:
                name = raw_name.decode("utf-8")
            except UnicodeDecodeError as exc:
                raise CatalogError("APPEARANCE_OBJECT_NAME_UTF8_INVALID") from exc
        items[raw_id] = {
            "server_item_id": raw_id,
            "runtime_name": name,
            "loaded_from_xml": False,
            "source_node_digest": None,
        }
    return items


def build_runtime_item_registry(
    appearance_bytes: bytes,
    items_xml: bytes,
    b1_module: Any,
) -> dict[str, Any]:
    items = _appearance_base_items(appearance_bytes)
    appearance_count = len(items)
    names: dict[str, set[int]] = defaultdict(set)
    for item_id, record in items.items():
        if record["runtime_name"]:
            names[str(record["runtime_name"]).casefold()].add(item_id)

    try:
        root = ET.fromstring(items_xml)
    except ET.ParseError as exc:
        raise CatalogError("OTHERYN_ITEMS_XML_PARSE_FAILED") from exc
    if root.tag != "items":
        raise CatalogError(f"OTHERYN_ITEMS_XML_ROOT_INVALID: {root.tag}")

    reversed_ranges = 0
    skipped_missing_appearance = 0
    xml_applied = 0
    for node in root.findall("item"):
        source_ids, exclusion = b1_module._node_ids(node)
        if exclusion is not None:
            reversed_ranges += 1
            continue
        node_digest = b1_module.sha256_bytes(
            b1_module.canonical_bytes(b1_module._canonical_node(node))
        )
        for item_id in source_ids:
            record = items.setdefault(
                item_id,
                {
                    "server_item_id": 0,
                    "runtime_name": "",
                    "loaded_from_xml": False,
                    "source_node_digest": None,
                },
            )
            runtime_name = str(record["runtime_name"])
            if (
                item_id >= 100
                and int(record["server_item_id"]) == 0
                and (
                    not runtime_name
                    or runtime_name.casefold() == "reserved sprite"
                )
            ):
                skipped_missing_appearance += 1
                continue
            record["server_item_id"] = item_id
            if record["loaded_from_xml"]:
                raise CatalogError(f"OTHERYN_XML_DUPLICATE_ITEM_ID: {item_id}")
            xml_name = node.attrib.get("name", "")
            if xml_name and runtime_name != xml_name:
                if runtime_name:
                    names[runtime_name.casefold()].discard(item_id)
                record["runtime_name"] = xml_name
                names[xml_name.casefold()].add(item_id)
            record["loaded_from_xml"] = True
            record["source_node_digest"] = node_digest
            xml_applied += 1

    clean_names = {
        key: sorted(values)
        for key, values in names.items()
        if values
    }
    collision_groups = {
        key: values for key, values in clean_names.items() if len(values) > 1
    }
    return {
        "items": items,
        "name_to_server_item_ids": clean_names,
        "counts": {
            "appearance_objects_with_flags": appearance_count,
            "runtime_name_keys": len(clean_names),
            "runtime_name_collision_groups": len(collision_groups),
            "xml_item_members_applied": xml_applied,
            "xml_item_members_skipped_missing_appearance": skipped_missing_appearance,
            "xml_reversed_ranges_skipped": reversed_ranges,
        },
    }


def build_b1_index(b1: dict[str, Any]) -> dict[int, dict[str, Any]]:
    records = b1.get("semantic_catalog", {}).get("identity_records", [])
    index: dict[int, dict[str, Any]] = {}
    for value in records:
        source_id = value.get("source_item_id")
        if not isinstance(source_id, int):
            raise CatalogError("B1_SOURCE_ITEM_ID_INVALID")
        if source_id in index:
            raise CatalogError(f"B1_SOURCE_ITEM_ID_DUPLICATE: {source_id}")
        disposition = value.get("native_disposition")
        if disposition not in RESOLUTION_STATES:
            raise CatalogError(
                f"B1_NATIVE_DISPOSITION_INVALID: {source_id}: {disposition}"
            )
        index[source_id] = value
    if len(index) != 38157:
        raise CatalogError(f"B1_INDEX_COUNT_MISMATCH: {len(index)}")
    return index


def _source_unresolved(
    reason_code: str,
    **details: Any,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "disposition": "UNRESOLVED",
        "reason_code": reason_code,
    }
    result.update(details)
    return result


def exact_source_crosswalk(
    server_item_id: int,
    runtime_items: dict[int, dict[str, Any]],
    b1_index: dict[int, dict[str, Any]],
) -> dict[str, Any]:
    runtime = runtime_items.get(server_item_id)
    if runtime is None or int(runtime.get("server_item_id", 0)) != server_item_id:
        return _source_unresolved(
            "SERVER_ITEM_UNAVAILABLE",
            server_item_id=server_item_id,
        )
    source_digest = runtime.get("source_node_digest")
    if not isinstance(source_digest, str) or not source_digest:
        return _source_unresolved(
            "NO_OTHERYN_XML_SOURCE_IDENTITY",
            server_item_id=server_item_id,
        )
    b1_record = b1_index.get(server_item_id)
    if b1_record is None:
        return _source_unresolved(
            "NO_B1_SOURCE_ITEM_IDENTITY",
            server_item_id=server_item_id,
            otheryn_source_node_digest=source_digest,
        )
    b1_digest = b1_record.get("source_node_digest")
    if source_digest != b1_digest:
        return _source_unresolved(
            "EXACT_SOURCE_NODE_MISMATCH",
            server_item_id=server_item_id,
            otheryn_source_node_digest=source_digest,
            b1_source_node_digest=b1_digest,
        )
    return {
        "disposition": "RESOLVED",
        "reason_code": "EXACT_CANONICAL_SOURCE_NODE_CROSSWALK",
        "server_item_id": server_item_id,
        "b1_source_item_id": server_item_id,
        "b1_source_node_digest": b1_digest,
    }


def resolve_source_item(
    *,
    item_name: str | None,
    server_item_id: int | None,
    client_id: int | None,
    runtime_registry: dict[str, Any],
    b1_index: dict[int, dict[str, Any]],
) -> dict[str, Any]:
    runtime_items = runtime_registry["items"]
    names = runtime_registry["name_to_server_item_ids"]
    provenance: dict[str, Any] = {}
    if server_item_id is not None:
        provenance["raw_server_item_id"] = server_item_id
    if client_id is not None:
        provenance["raw_client_id"] = client_id

    if item_name is not None:
        provenance["selector"] = "NAME"
        candidates = list(names.get(item_name.casefold(), []))
        if not candidates:
            return {
                **_source_unresolved(
                    "SOURCE_NAME_NOT_IN_RUNTIME_REGISTRY",
                    item_name=item_name,
                ),
                **provenance,
            }
        if len(candidates) > 1:
            return {
                "disposition": "AMBIGUOUS",
                "reason_code": "SOURCE_NAME_AMBIGUOUS",
                "item_name": item_name,
                "candidate_server_item_ids": candidates,
                **provenance,
            }
        result = exact_source_crosswalk(candidates[0], runtime_items, b1_index)
        result.update(provenance)
        result["item_name"] = item_name
        if server_item_id is not None:
            result["raw_server_item_id_semantics"] = (
                "ignored because pinned register_monster_type.lua selects "
                "loot.name before loot.id"
            )
        if client_id is not None:
            result["raw_client_id_semantics"] = "provenance only; no admitted crosswalk"
        return result

    if server_item_id is not None:
        result = exact_source_crosswalk(server_item_id, runtime_items, b1_index)
        result.update(provenance)
        result["selector"] = "SERVER_ITEM_ID"
        if client_id is not None:
            result["raw_client_id_semantics"] = "provenance only; no admitted crosswalk"
        return result

    if client_id is not None:
        return {
            **_source_unresolved(
                "CLIENT_ID_HAS_NO_ADMITTED_SERVER_ID_CROSSWALK",
            ),
            **provenance,
            "selector": "CLIENT_ID_UNSUPPORTED_FOR_IDENTITY",
        }
    raise CatalogError("LOOT_ROW_WITHOUT_SOURCE_ITEM_SELECTOR")


def resolve_native_item(
    source_resolution: dict[str, Any],
    b1_index: dict[int, dict[str, Any]],
) -> dict[str, Any]:
    if source_resolution.get("disposition") != "RESOLVED":
        return {
            "disposition": "UNRESOLVED",
            "reason_code": "NO_SINGLE_RESOLVED_B1_SOURCE_IDENTITY",
        }
    source_id = source_resolution.get("b1_source_item_id")
    if not isinstance(source_id, int) or source_id not in b1_index:
        raise CatalogError("RESOLVED_SOURCE_MISSING_B1_RECORD")
    record = b1_index[source_id]
    disposition = record["native_disposition"]
    detail = record.get("native_mapping_detail", {})
    if detail is None:
        detail = {}
    if not isinstance(detail, dict):
        raise CatalogError("B1_NATIVE_MAPPING_DETAIL_INVALID")

    if disposition == "UNRESOLVED":
        if detail.get("content_key") is not None:
            raise CatalogError("B1_UNRESOLVED_CARRIES_NATIVE_TARGET")
        return {
            "disposition": "UNRESOLVED",
            "reason_code": "B1_NATIVE_DISPOSITION_UNRESOLVED",
            "b1_source_item_id": source_id,
        }

    if disposition == "RESOLVED":
        target = detail.get("content_key")
        if not isinstance(target, str) or not target.startswith("oteryn:"):
            raise CatalogError("B1_RESOLVED_NATIVE_TARGET_INVALID")
        if target.startswith("oteryn:vsl."):
            raise CatalogError("FAKE_NATIVE_VSL_KEY_FORBIDDEN")
        return {
            "disposition": "RESOLVED",
            "reason_code": "PROTECTED_B1_EXPLICIT_NATIVE_BINDING",
            "b1_source_item_id": source_id,
            "content_key": target,
            "evidence_refs": sorted(detail.get("evidence_refs", [])),
        }

    if disposition in {"AMBIGUOUS", "CONFLICT"}:
        if detail.get("content_key") is not None:
            raise CatalogError("B1_NONRESOLVED_DISPOSITION_CARRIES_NATIVE_TARGET")
        result = {
            "disposition": disposition,
            "reason_code": f"B1_NATIVE_DISPOSITION_{disposition}",
            "b1_source_item_id": source_id,
        }
        candidate_keys = detail.get("candidate_content_keys")
        if candidate_keys:
            if not isinstance(candidate_keys, list) or not all(
                isinstance(value, str) and value.startswith("oteryn:")
                for value in candidate_keys
            ):
                raise CatalogError("B1_NATIVE_CANDIDATE_KEYS_INVALID")
            result["candidate_content_keys"] = sorted(set(candidate_keys))
        return result
    raise CatalogError(f"B1_NATIVE_DISPOSITION_UNEXPECTED: {disposition}")


def _admitted_rows(
    stripped_text: str,
    gameplay: Any,
) -> list[tuple[int, dict[str, str], dict[str, Any]]]:
    assignments = gameplay._table_assignments(stripped_text, "monster.loot")
    if not assignments or len(assignments) != 1 or assignments[0] is None:
        return []
    rows, _ = gameplay._table_entries(assignments[0])
    admitted: list[tuple[int, dict[str, str], dict[str, Any]]] = []
    for raw_index, row in enumerate(rows):
        one = gameplay._loot_profile(f"monster.loot = {{{row}}}", {})
        entries = list(one.get("entries", []))
        if not entries:
            continue
        if len(entries) != 1:
            raise CatalogError("SINGLE_SOURCE_LOOT_ROW_EMITTED_MULTIPLE_ENTRIES")
        fields, fields_ok = gameplay._field_tokens(row)
        if not fields_ok:
            raise CatalogError("ADMITTED_LOOT_ROW_FIELD_PARSE_DISAGREEMENT")
        admitted.append((raw_index, fields, entries[0]))
    return admitted


def _int_field(gameplay: Any, fields: dict[str, str], key: str) -> int | None:
    return gameplay._literal_int(fields.get(key))


def build_binding_catalog(
    b1: dict[str, Any],
    b2: dict[str, Any],
    monster_payloads: dict[str, bytes],
    runtime_registry: dict[str, Any],
    gameplay: Any,
) -> dict[str, Any]:
    b1_index = build_b1_index(b1)
    source_files = {
        str(record["source_file_ref"]): record
        for record in b2["source_snapshot"]["files"]
        if record.get("role") == "MONSTER_DEFINITION_LUA"
    }
    profiles = {
        str(profile["profile_id"]): profile
        for profile in b2["monster_definitions"]["candidate_profiles"]
    }
    if len(profiles) != len(b2["monster_definitions"]["candidate_profiles"]):
        raise CatalogError("B2_CANDIDATE_PROFILE_ID_DUPLICATE")

    records: list[dict[str, Any]] = []
    profile_issues: list[dict[str, Any]] = []
    source_totals: Counter[str] = Counter()
    native_totals: Counter[str] = Counter()
    source_reasons: Counter[str] = Counter()
    native_reasons: Counter[str] = Counter()
    display_names: Counter[str] = Counter()
    row_digests: Counter[str] = Counter()
    client_ids: dict[int, set[str]] = defaultdict(set)
    seen_row_ids: set[str] = set()

    definitions = sorted(
        b2["monster_definitions"]["records"],
        key=lambda value: str(value["source_identity"]),
    )
    for definition in definitions:
        source_ref = str(definition["source_file_ref"])
        source = source_files.get(source_ref)
        payload = monster_payloads.get(source_ref)
        if source is None or payload is None:
            raise CatalogError(f"B2_MONSTER_SOURCE_UNAVAILABLE: {source_ref}")
        profile_id = str(definition["candidate_profile_id"])
        profile = profiles.get(profile_id)
        if profile is None:
            raise CatalogError(f"B2_CANDIDATE_PROFILE_MISSING: {profile_id}")
        expected_loot = profile["loot"]
        try:
            raw_text = payload.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise CatalogError(f"MONSTER_SOURCE_UTF8_INVALID: {source_ref}") from exc
        stripped = gameplay._strip_line_comments(raw_text)
        actual_loot = gameplay._loot_profile(stripped, {})
        if actual_loot != expected_loot:
            raise CatalogError(f"B2_LOOT_PROFILE_REPLAY_MISMATCH: {source_ref}")
        admitted = _admitted_rows(stripped, gameplay)
        expected_entries = list(expected_loot.get("entries", []))
        if [entry for _, _, entry in admitted] != expected_entries:
            raise CatalogError(f"B2_LOOT_ROW_REPLAY_MISMATCH: {source_ref}")

        state = str(expected_loot.get("state"))
        reason_codes = sorted(str(value) for value in expected_loot.get("reason_codes", []))
        if state != "COMPLETE" or reason_codes:
            profile_issues.append(
                {
                    "monster_source_identity": definition["source_identity"],
                    "source_file_ref": source_ref,
                    "loot_state": state,
                    "reason_codes": reason_codes,
                    "admitted_rows": len(admitted),
                }
            )

        for emitted_index, (raw_index, fields, entry) in enumerate(admitted):
            item_name = gameplay._literal_string(fields.get("name"))
            explicit_id = _int_field(gameplay, fields, "id")
            client_id = _int_field(gameplay, fields, "clientId")
            source_resolution = resolve_source_item(
                item_name=item_name,
                server_item_id=explicit_id,
                client_id=client_id,
                runtime_registry=runtime_registry,
                b1_index=b1_index,
            )
            native_resolution = resolve_native_item(source_resolution, b1_index)
            source_totals[source_resolution["disposition"]] += 1
            native_totals[native_resolution["disposition"]] += 1
            source_reasons[source_resolution["reason_code"]] += 1
            native_reasons[native_resolution["reason_code"]] += 1

            row_id = f"{definition['source_identity']}:loot:{emitted_index:04d}"
            if row_id in seen_row_ids:
                raise CatalogError(f"LOOT_ROW_ID_DUPLICATE: {row_id}")
            seen_row_ids.add(row_id)
            source_row_digest = sha256_bytes(
                canonical_bytes(
                    {
                        "monster_source_identity": definition["source_identity"],
                        "source_file_ref": source_ref,
                        "raw_row_index": raw_index,
                        "source_fields": dict(sorted(fields.items())),
                    }
                )
            )
            row_digests[source_row_digest] += 1
            display_names[str(entry["item_name"]).casefold()] += 1
            if client_id is not None:
                client_ids[client_id].add(
                    json.dumps(
                        {
                            "name": item_name,
                            "id": explicit_id,
                            "source_resolution": source_resolution["disposition"],
                        },
                        sort_keys=True,
                        separators=(",", ":"),
                    )
                )

            records.append(
                {
                    "loot_row_identity": row_id,
                    "monster_source_identity": definition["source_identity"],
                    "source_file_ref": source_ref,
                    "raw_row_index": raw_index,
                    "emitted_row_index": emitted_index,
                    "source_row_digest": source_row_digest,
                    "source_item_provenance": {
                        "item_name": item_name,
                        "server_item_id": explicit_id,
                        "client_id": client_id,
                    },
                    "loot_candidate": {
                        "item_label": entry["item_name"],
                        "chance_ppm": entry["chance_ppm"],
                        "min_count": entry["min_count"],
                        "max_count": entry["max_count"],
                        "b2_item_resolution_state": entry[
                            "item_resolution_state"
                        ],
                    },
                    "b2_loot_state": state,
                    "b2_reason_codes": reason_codes,
                    "source_item_resolution": source_resolution,
                    "native_item_resolution": native_resolution,
                }
            )

    records = sorted(records, key=lambda value: value["loot_row_identity"])
    expected_total = int(b2["counts"]["deferred_b3_loot_rows"])
    if len(records) != expected_total:
        raise CatalogError(
            f"B3_LOOT_ROW_COUNT_MISMATCH: expected {expected_total}, got {len(records)}"
        )
    for state_name in RESOLUTION_STATES:
        source_totals[state_name] += 0
        native_totals[state_name] += 0
    if sum(source_totals.values()) != len(records):
        raise CatalogError("SOURCE_ITEM_RESOLUTION_PARTITION_INCOMPLETE")
    if sum(native_totals.values()) != len(records):
        raise CatalogError("NATIVE_ITEM_RESOLUTION_PARTITION_INCOMPLETE")
    if len(seen_row_ids) != len(records):
        raise CatalogError("LOOT_ROW_IDENTITY_NOT_UNIQUE")

    duplicate_display_groups = sum(value > 1 for value in display_names.values())
    duplicate_row_digest_groups = sum(value > 1 for value in row_digests.values())
    conflicting_client_groups = sum(len(values) > 1 for values in client_ids.values())
    return {
        "records": records,
        "source_item_resolution_totals": {
            state: source_totals[state] for state in RESOLUTION_STATES
        },
        "native_item_resolution_totals": {
            state: native_totals[state] for state in RESOLUTION_STATES
        },
        "source_resolution_reason_totals": dict(sorted(source_reasons.items())),
        "native_resolution_reason_totals": dict(sorted(native_reasons.items())),
        "source_profile_issue_records": canonical_record_list(profile_issues),
        "collision_accounting": {
            "loot_display_name_collision_groups": duplicate_display_groups,
            "duplicate_source_row_digest_groups": duplicate_row_digest_groups,
            "runtime_item_name_collision_groups": runtime_registry["counts"][
                "runtime_name_collision_groups"
            ],
            "client_id_rows": sum(
                1
                for record in records
                if record["source_item_provenance"]["client_id"] is not None
            ),
            "distinct_client_ids": len(client_ids),
            "client_id_conflicting_source_evidence_groups": conflicting_client_groups,
            "loot_row_identity_duplicates": 0,
        },
    }


def build_evidence(source_repo: Path, game_root: Path) -> dict[str, Any]:
    protected, b1, b2 = verify_game_products(game_root)
    b1_module, gameplay = load_protected_modules(game_root)
    source_snapshot, semantic_payloads, monster_payloads = verify_source_repository(
        source_repo, b2
    )
    runtime_registry = build_runtime_item_registry(
        semantic_payloads["data/items/appearances.dat"],
        semantic_payloads["data/items/items.xml"],
        b1_module,
    )
    bindings = build_binding_catalog(
        b1, b2, monster_payloads, runtime_registry, gameplay
    )

    source_totals = bindings["source_item_resolution_totals"]
    native_totals = bindings["native_item_resolution_totals"]
    if source_totals["RESOLVED"] <= 0:
        raise CatalogError("B3_EXPECTED_EXACT_SOURCE_CROSSWALKS_ABSENT")
    if native_totals != {
        "RESOLVED": 0,
        "UNRESOLVED": len(bindings["records"]),
        "AMBIGUOUS": 0,
        "CONFLICT": 0,
    }:
        raise CatalogError("B3_NATIVE_FAIL_CLOSED_INVARIANT_FAILED")

    mapper_payload = canonical_repository_text_bytes(Path(__file__).read_bytes())
    value: dict[str, Any] = {
        "schema": SCHEMA,
        "task": TASK,
        "closure": CLOSURE,
        "classification": {
            "source": SOURCE_CLASSIFICATION,
            "evidence_status": EVIDENCE_STATUS,
            "production_authority": "NONE",
            "reference_parity_claim": "NONE",
        },
        "protected_products": protected,
        "external_source_snapshot": source_snapshot,
        "mapper": {
            "profile": MAPPER_PROFILE,
            "path": (
                "tools/reference-world-corridor-census/"
                "loot_item_binding_catalog.py"
            ),
            "sha256": sha256_bytes(mapper_payload),
            "sha256_semantics": "repository text bytes with CRLF canonicalized to LF",
            "final_pr_head": "RECORDED_EXTERNALLY_BY_LIVE_PR_READBACK",
            "final_pr_head_note": (
                "a tracked Git object cannot contain its own final commit SHA "
                "without self-reference"
            ),
        },
        "identity_rules": {
            "loot_name_is_native_identity": False,
            "normalized_loot_name_is_native_identity": False,
            "server_numeric_id_is_native_identity": False,
            "client_id_is_server_item_id": False,
            "b1_numeric_equality_alone_resolves_source_join": False,
            "atlas_item_id_is_native_identity": False,
            "path_order_hash_is_native_identity": False,
            "appearance_id_is_native_identity": False,
            "creature_name_or_hash_is_item_identity": False,
            "auto_mint_native_item_key": False,
            "source_join_rule": (
                "pinned Otheryn loot selector semantics must identify one "
                "server item, and that item's applied items.xml canonical "
                "source node must exactly equal the protected B1 source node "
                "for the same source member; numeric equality alone is insufficient"
            ),
            "native_join_rule": (
                "copy only protected B1 native disposition/detail; never derive "
                "or hash a source value into an oteryn:* key"
            ),
            "name_collision_rule": (
                "multiple runtime nameToItems candidates remain AMBIGUOUS; "
                "runtime insertion order is not import identity authority"
            ),
        },
        "runtime_source_item_registry": {
            "counts": runtime_registry["counts"],
            "name_registry_materialization": (
                "pinned appearances.dat object name base plus pinned items.xml "
                "name overrides, following pinned items.cpp"
            ),
        },
        "loot_bindings": bindings,
        "counts": {
            "admitted_b2_loot_rows": len(bindings["records"]),
            "source_item_RESOLVED": source_totals["RESOLVED"],
            "source_item_UNRESOLVED": source_totals["UNRESOLVED"],
            "source_item_AMBIGUOUS": source_totals["AMBIGUOUS"],
            "source_item_CONFLICT": source_totals["CONFLICT"],
            "native_item_RESOLVED": native_totals["RESOLVED"],
            "native_item_UNRESOLVED": native_totals["UNRESOLVED"],
            "native_item_AMBIGUOUS": native_totals["AMBIGUOUS"],
            "native_item_CONFLICT": native_totals["CONFLICT"],
            "source_profile_issue_records": len(
                bindings["source_profile_issue_records"]
            ),
        },
        "count_invariants": {
            "every_b2_loot_row_has_one_source_resolution": (
                sum(source_totals.values()) == len(bindings["records"])
            ),
            "every_b2_loot_row_has_one_native_resolution": (
                sum(native_totals.values()) == len(bindings["records"])
            ),
            "loot_row_identity_unique": (
                bindings["collision_accounting"]["loot_row_identity_duplicates"] == 0
            ),
            "native_resolved_requires_protected_b1_resolved": (
                native_totals["RESOLVED"] == 0
            ),
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
    print(
        "loot-item-binding-catalog: PASS "
        f"rows={counts['admitted_b2_loot_rows']} "
        f"source_resolved={counts['source_item_RESOLVED']} "
        f"source_unresolved={counts['source_item_UNRESOLVED']} "
        f"source_ambiguous={counts['source_item_AMBIGUOUS']} "
        f"native_resolved={counts['native_item_RESOLVED']} "
        f"native_unresolved={counts['native_item_UNRESOLVED']} "
        f"digest={evidence['product_digest_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
