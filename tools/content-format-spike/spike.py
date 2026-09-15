#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import difflib
import hashlib
import json
import os
import platform
import shutil
import sqlite3
import struct
import tempfile
import time
import tracemalloc
import zlib
from collections.abc import Callable
from pathlib import Path
from typing import Any

INVARIANT = "SPIKE_RESULT != OWNER_FORMAT_DECISION"
MAGIC = b"OTSPIKE1"
BUNDLE_VERSION = 1
HEADER = struct.Struct(">8sHBBHII")
INDEX = struct.Struct(">hiiQII32s")
PROJECTION = {"server": 1, "client": 2}
PROJECTION_BY_ID = {value: key for key, value in PROJECTION.items()}
KNOWN_CRITICAL = {"chunk-index-v1", "projection-v1", "composite-presentation-v1"}
MAX_ARTIFACT_BYTES = 64 * 1024 * 1024
MAX_CHUNK_RAW_BYTES = 2 * 1024 * 1024
MAX_CHUNKS = 4096
MAX_STRING_BYTES = 512
MAX_DEPTH = 16
MAX_COLLECTION = 100_000
MAX_DECOMPRESSION_RATIO = 64.0


class SpikeError(ValueError):
    pass


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_pretty_json(value: Any) -> bytes:
    return (
        json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
    ).encode("utf-8")


def validate_tree(value: Any, depth: int = 0) -> None:
    if depth > MAX_DEPTH:
        raise SpikeError("nesting depth exceeded")
    if isinstance(value, str):
        if len(value.encode("utf-8")) > MAX_STRING_BYTES:
            raise SpikeError("string byte limit exceeded")
    elif isinstance(value, dict):
        if len(value) > MAX_COLLECTION:
            raise SpikeError("object member limit exceeded")
        for key, child in value.items():
            validate_tree(key, depth + 1)
            validate_tree(child, depth + 1)
    elif isinstance(value, list):
        if len(value) > MAX_COLLECTION:
            raise SpikeError("array entry limit exceeded")
        for child in value:
            validate_tree(child, depth + 1)


def validate_fixture(fixture: dict[str, Any]) -> None:
    if fixture.get("schema_version") != 1:
        raise SpikeError("unsupported fixture schema")
    critical = fixture.get("critical_features", [])
    unknown = set(critical) - KNOWN_CRITICAL
    if unknown:
        raise SpikeError(f"unknown critical feature: {min(unknown)}")
    cells = fixture.get("cells")
    if not isinstance(cells, list) or not cells:
        raise SpikeError("fixture requires cells")
    if len(cells) > MAX_COLLECTION:
        raise SpikeError("fixture cell limit exceeded")
    validate_tree(fixture)


def make_fixture(side: int) -> dict[str, Any]:
    if side < 1 or side > 256:
        raise SpikeError("fixture side outside spike-only bound")
    cells = []
    for y in range(side):
        for x in range(side):
            cells.append(
                {
                    "x": x,
                    "y": y,
                    "z": 7,
                    "terrain": "stone" if (x + y) % 11 == 0 else "grass",
                    "collision": "blocked" if (x * 17 + y) % 97 == 0 else "walkable",
                    "placement": "oteryn:item.fixture_crate"
                    if (x * 7 + y) % 211 == 0
                    else None,
                }
            )
    fixture = {
        "schema_version": 1,
        "world_id": f"spike-world-{side}",
        "critical_features": sorted(KNOWN_CRITICAL),
        "provenance": {
            "source": "synthetic",
            "revision": "fixture-v1",
            "license": "project-owned",
        },
        "definitions": {
            "structure": {
                "key": "oteryn:structure.synthetic_fountain",
                "anchor": [4, 4, 7],
                "collision_footprint": [[4, 4, 7]],
                "visual_fragments": [
                    {"dx": 0, "dy": 0, "token": "synthetic://fountain/nw"},
                    {"dx": 1, "dy": 0, "token": "synthetic://fountain/ne"},
                    {"dx": 0, "dy": 1, "token": "synthetic://fountain/sw"},
                    {"dx": 1, "dy": 1, "token": "synthetic://fountain/se"},
                ],
            }
        },
        "server_only": {"loot_weight": 37, "spawn_policy": "fixture-server-only"},
        "cells": cells,
    }
    validate_fixture(fixture)
    return fixture


def chunk_payloads(
    fixture: dict[str, Any], chunk_size: int
) -> dict[tuple[int, int, int], dict[str, Any]]:
    if chunk_size not in (8, 16, 32, 64):
        raise SpikeError("unsupported spike chunk size")
    grouped: dict[tuple[int, int, int], list[dict[str, Any]]] = {}
    for cell in fixture["cells"]:
        key = (cell["x"] // chunk_size, cell["y"] // chunk_size, cell["z"])
        grouped.setdefault(key, []).append(cell)
    if len(grouped) > MAX_CHUNKS:
        raise SpikeError("chunk count exceeded")
    return {
        key: {
            "schema_version": 1,
            "chunk_key": list(key),
            "cells": sorted(cells, key=lambda c: (c["z"], c["y"], c["x"])),
        }
        for key, cells in sorted(grouped.items())
    }


def atomic_write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(
        prefix=path.name + ".", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, path)
    finally:
        if os.path.exists(temp_name):
            os.unlink(temp_name)


def artifact_files(path: Path) -> dict[str, bytes]:
    if path.is_file():
        return {path.name: path.read_bytes()}
    return {
        file.relative_to(path).as_posix(): file.read_bytes()
        for file in sorted(path.rglob("*"))
        if file.is_file()
    }


def artifact_digest(path: Path) -> str:
    digest = hashlib.sha256()
    if path.is_file():
        data = path.read_bytes()
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
        return digest.hexdigest()
    for name, data in sorted(artifact_files(path).items()):
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
    return digest.hexdigest()


def artifact_size(path: Path) -> int:
    return sum(len(data) for data in artifact_files(path).values())


def manifest_from_fixture(
    fixture: dict[str, Any], chunk_size: int, projection: str | None = None
) -> dict[str, Any]:
    manifest = {
        "project_format_version": 1,
        "world_schema_version": 1,
        "world_id": fixture["world_id"],
        "chunk_size": chunk_size,
        "critical_features": fixture["critical_features"],
        "provenance": fixture["provenance"],
        "definitions": fixture["definitions"],
    }
    if projection is not None:
        manifest["projection"] = projection
    if projection != "client":
        manifest["server_only"] = fixture["server_only"]
    return manifest


def _decode_json(data: bytes) -> Any:
    if len(data) > MAX_CHUNK_RAW_BYTES:
        raise SpikeError("json input byte limit exceeded")
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise SpikeError("invalid json") from exc
    validate_tree(value)
    return value


def _validate_manifest(manifest: dict[str, Any]) -> None:
    if (
        manifest.get("project_format_version") != 1
        or manifest.get("world_schema_version") != 1
    ):
        raise SpikeError("unsupported manifest version")
    unknown = set(manifest.get("critical_features", [])) - KNOWN_CRITICAL
    if unknown:
        raise SpikeError("unknown critical manifest feature")
    validate_tree(manifest)


def write_json_project(path: Path, fixture: dict[str, Any], chunk_size: int) -> None:
    validate_fixture(fixture)
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True)
    chunks = chunk_payloads(fixture, chunk_size)
    manifest = manifest_from_fixture(fixture, chunk_size)
    entries = []
    for key, payload in chunks.items():
        raw = canonical_pretty_json(payload)
        if len(raw) > MAX_CHUNK_RAW_BYTES:
            raise SpikeError("chunk byte limit exceeded")
        rel = f"chunks/z{key[2]}/c{key[0]}_{key[1]}.json"
        atomic_write(path / rel, raw)
        entries.append({"key": list(key), "path": rel, "sha256": sha256(raw)})
    manifest["chunks"] = entries
    atomic_write(path / "manifest.json", canonical_pretty_json(manifest))


def _validate_json_chunk_entries(manifest: dict[str, Any]) -> None:
    chunks = manifest.get("chunks")
    if not isinstance(chunks, list):
        raise SpikeError("manifest chunks must be a list")
    if len(chunks) > MAX_CHUNKS:
        raise SpikeError("manifest chunk count exceeded")
    seen: set[tuple[int, int, int]] = set()
    for row in chunks:
        if not isinstance(row, dict):
            raise SpikeError("manifest chunk entry must be an object")
        key = row.get("key")
        if (
            not isinstance(key, list)
            or len(key) != 3
            or any(
                not isinstance(value, int) or isinstance(value, bool) for value in key
            )
        ):
            raise SpikeError("manifest chunk key is invalid")
        normalized = (key[0], key[1], key[2])
        if normalized in seen:
            raise SpikeError("duplicate manifest chunk key")
        seen.add(normalized)
        relative = row.get("path")
        digest = row.get("sha256")
        if not isinstance(relative, str) or not relative:
            raise SpikeError("invalid chunk path")
        if not isinstance(digest, str) or len(digest) != 64:
            raise SpikeError("invalid chunk digest")
        try:
            if len(bytes.fromhex(digest)) != 32:
                raise ValueError
        except ValueError as exc:
            raise SpikeError("invalid chunk digest") from exc


def read_json_manifest(path: Path) -> dict[str, Any]:
    manifest_path = path / "manifest.json"
    if (
        not manifest_path.is_file()
        or manifest_path.stat().st_size > MAX_CHUNK_RAW_BYTES
    ):
        raise SpikeError("manifest missing or oversized")
    manifest = _decode_json(manifest_path.read_bytes())
    if not isinstance(manifest, dict):
        raise SpikeError("manifest must be object")
    _validate_manifest(manifest)
    _validate_json_chunk_entries(manifest)
    return manifest


def read_json_chunk(path: Path, key: tuple[int, int, int]) -> dict[str, Any]:
    manifest = read_json_manifest(path)
    entry = next((row for row in manifest["chunks"] if tuple(row["key"]) == key), None)
    if entry is None:
        raise SpikeError("chunk missing")
    relative = entry.get("path")
    if not isinstance(relative, str) or not relative:
        raise SpikeError("invalid chunk path")
    project_root = path.resolve()
    chunk_path = (path / relative).resolve()
    try:
        chunk_path.relative_to(project_root)
    except ValueError as exc:
        raise SpikeError("chunk path escapes project root") from exc
    if not chunk_path.is_file() or chunk_path.stat().st_size > MAX_CHUNK_RAW_BYTES:
        raise SpikeError("chunk missing or oversized")
    raw = chunk_path.read_bytes()
    if sha256(raw) != entry["sha256"]:
        raise SpikeError("chunk integrity mismatch")
    payload = _decode_json(raw)
    if not isinstance(payload, dict) or tuple(payload.get("chunk_key", [])) != key:
        raise SpikeError("chunk key mismatch")
    return payload


def write_sqlite_project(path: Path, fixture: dict[str, Any], chunk_size: int) -> None:
    validate_fixture(fixture)
    if path.exists():
        path.unlink()
    path.parent.mkdir(parents=True, exist_ok=True)
    chunks = chunk_payloads(fixture, chunk_size)
    conn = sqlite3.connect(path)
    try:
        conn.execute("PRAGMA page_size=4096")
        conn.execute("PRAGMA journal_mode=OFF")
        conn.execute("PRAGMA synchronous=OFF")
        conn.execute("PRAGMA auto_vacuum=NONE")
        conn.execute("PRAGMA user_version=1")
        conn.execute(
            "CREATE TABLE meta(key TEXT PRIMARY KEY, value BLOB NOT NULL) WITHOUT ROWID"
        )
        conn.execute(
            "CREATE TABLE chunks(z INTEGER NOT NULL, cx INTEGER NOT NULL, cy INTEGER NOT NULL, "
            "payload BLOB NOT NULL, digest BLOB NOT NULL, PRIMARY KEY(z,cx,cy)) WITHOUT ROWID"
        )
        manifest = manifest_from_fixture(fixture, chunk_size)
        conn.execute(
            "INSERT INTO meta(key,value) VALUES(?,?)",
            ("manifest", canonical_json(manifest)),
        )
        for key, payload in sorted(chunks.items()):
            raw = canonical_json(payload)
            if len(raw) > MAX_CHUNK_RAW_BYTES:
                raise SpikeError("sqlite chunk byte limit exceeded")
            conn.execute(
                "INSERT INTO chunks(z,cx,cy,payload,digest) VALUES(?,?,?,?,?)",
                (key[2], key[0], key[1], raw, bytes.fromhex(sha256(raw))),
            )
        conn.commit()
        conn.execute("VACUUM")
    finally:
        conn.close()
    if path.stat().st_size > MAX_ARTIFACT_BYTES:
        raise SpikeError("sqlite artifact byte limit exceeded")


def _sqlite_connect_ro(path: Path) -> sqlite3.Connection:
    if not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
        raise SpikeError("sqlite artifact missing or oversized")
    return sqlite3.connect(f"file:{path.resolve().as_posix()}?mode=ro", uri=True)


def read_sqlite_manifest(path: Path) -> dict[str, Any]:
    conn = _sqlite_connect_ro(path)
    try:
        if conn.execute("PRAGMA user_version").fetchone()[0] != 1:
            raise SpikeError("unsupported sqlite project version")
        row = conn.execute("SELECT value FROM meta WHERE key='manifest'").fetchone()
        if row is None:
            raise SpikeError("sqlite manifest missing")
        manifest = _decode_json(bytes(row[0]))
        if not isinstance(manifest, dict):
            raise SpikeError("sqlite manifest invalid")
        _validate_manifest(manifest)
        return manifest
    finally:
        conn.close()


def read_sqlite_chunk(path: Path, key: tuple[int, int, int]) -> dict[str, Any]:
    conn = _sqlite_connect_ro(path)
    try:
        row = conn.execute(
            "SELECT payload,digest FROM chunks WHERE z=? AND cx=? AND cy=?",
            (key[2], key[0], key[1]),
        ).fetchone()
        if row is None:
            raise SpikeError("sqlite chunk missing")
        raw, digest = bytes(row[0]), bytes(row[1])
        if len(raw) > MAX_CHUNK_RAW_BYTES or hashlib.sha256(raw).digest() != digest:
            raise SpikeError("sqlite chunk integrity mismatch")
        payload = _decode_json(raw)
        if tuple(payload.get("chunk_key", [])) != key:
            raise SpikeError("sqlite chunk key mismatch")
        return payload
    finally:
        conn.close()


def bounded_decompress(
    compressed: bytes,
    expected_raw_size: int,
    max_raw_size: int = MAX_CHUNK_RAW_BYTES,
    max_ratio: float = MAX_DECOMPRESSION_RATIO,
) -> bytes:
    if not compressed:
        raise SpikeError("empty compressed payload")
    if expected_raw_size < 0 or expected_raw_size > max_raw_size:
        raise SpikeError("decompressed size limit exceeded")
    if expected_raw_size / len(compressed) > max_ratio:
        raise SpikeError("decompression ratio limit exceeded")
    decoder = zlib.decompressobj()
    try:
        raw = decoder.decompress(compressed, max_raw_size + 1)
        if decoder.unconsumed_tail or len(raw) > max_raw_size:
            raise SpikeError("decompressed size limit exceeded")
        remaining = max_raw_size + 1 - len(raw)
        raw += decoder.flush(remaining)
    except zlib.error as exc:
        raise SpikeError("invalid compressed payload") from exc
    if decoder.unused_data or decoder.unconsumed_tail or not decoder.eof:
        raise SpikeError("compressed payload is incomplete or has trailing data")
    if len(raw) != expected_raw_size or len(raw) > max_raw_size:
        raise SpikeError("decompressed size mismatch")
    return raw


def write_binary_bundle(
    path: Path,
    fixture: dict[str, Any],
    chunk_size: int,
    projection: str,
) -> None:
    validate_fixture(fixture)
    if projection not in PROJECTION:
        raise SpikeError("unsupported projection")
    chunks = chunk_payloads(fixture, chunk_size)
    manifest = manifest_from_fixture(fixture, chunk_size, projection)
    manifest_bytes = canonical_json(manifest)
    if len(manifest_bytes) > MAX_CHUNK_RAW_BYTES:
        raise SpikeError("bundle manifest byte limit exceeded")
    encoded: list[tuple[tuple[int, int, int], bytes, int, bytes]] = []
    for key, payload in sorted(chunks.items()):
        raw = canonical_json(payload)
        if len(raw) > MAX_CHUNK_RAW_BYTES:
            raise SpikeError("bundle chunk byte limit exceeded")
        compressed = zlib.compress(raw, 6)
        encoded.append((key, compressed, len(raw), hashlib.sha256(raw).digest()))
    payload_offset = HEADER.size + len(manifest_bytes) + INDEX.size * len(encoded)
    entries = []
    cursor = payload_offset
    for key, compressed, raw_size, digest in encoded:
        entries.append(
            INDEX.pack(
                key[2], key[0], key[1], cursor, len(compressed), raw_size, digest
            )
        )
        cursor += len(compressed)
    if cursor > MAX_ARTIFACT_BYTES:
        raise SpikeError("bundle artifact byte limit exceeded")
    header = HEADER.pack(
        MAGIC,
        BUNDLE_VERSION,
        PROJECTION[projection],
        0,
        chunk_size,
        len(manifest_bytes),
        len(encoded),
    )
    atomic_write(
        path,
        header
        + manifest_bytes
        + b"".join(entries)
        + b"".join(row[1] for row in encoded),
    )


def _parse_bundle(
    path: Path,
) -> tuple[
    dict[str, Any], dict[tuple[int, int, int], tuple[int, int, int, bytes]], bytes
]:
    if not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
        raise SpikeError("bundle missing or oversized")
    data = path.read_bytes()
    if len(data) < HEADER.size:
        raise SpikeError("truncated bundle header")
    magic, version, projection_id, reserved, chunk_size, manifest_len, chunk_count = (
        HEADER.unpack_from(data, 0)
    )
    if magic != MAGIC or version != BUNDLE_VERSION or reserved != 0:
        raise SpikeError("unsupported bundle header")
    projection = PROJECTION_BY_ID.get(projection_id)
    if projection is None or chunk_size not in (8, 16, 32, 64):
        raise SpikeError("unsupported bundle metadata")
    if manifest_len > MAX_CHUNK_RAW_BYTES or chunk_count > MAX_CHUNKS:
        raise SpikeError("bundle metadata limit exceeded")
    manifest_start = HEADER.size
    manifest_end = manifest_start + manifest_len
    index_end = manifest_end + INDEX.size * chunk_count
    if manifest_end > len(data) or index_end > len(data):
        raise SpikeError("truncated bundle table")
    manifest = _decode_json(data[manifest_start:manifest_end])
    if not isinstance(manifest, dict):
        raise SpikeError("bundle manifest invalid")
    _validate_manifest(manifest)
    if (
        manifest.get("projection") != projection
        or manifest.get("chunk_size") != chunk_size
    ):
        raise SpikeError("bundle manifest/header mismatch")
    entries: dict[tuple[int, int, int], tuple[int, int, int, bytes]] = {}
    previous_end = index_end
    for index in range(chunk_count):
        start = manifest_end + index * INDEX.size
        z, cx, cy, offset, compressed_len, raw_len, digest = INDEX.unpack_from(
            data, start
        )
        key = (cx, cy, z)
        end = offset + compressed_len
        if (
            key in entries
            or offset < index_end
            or offset < previous_end
            or end > len(data)
        ):
            raise SpikeError("invalid bundle index")
        if compressed_len == 0 or raw_len > MAX_CHUNK_RAW_BYTES:
            raise SpikeError("bundle chunk size limit exceeded")
        entries[key] = (offset, compressed_len, raw_len, digest)
        previous_end = end
    if previous_end != len(data):
        raise SpikeError("bundle trailing or missing payload bytes")
    return manifest, entries, data


def read_binary_manifest(path: Path) -> dict[str, Any]:
    manifest, _, _ = _parse_bundle(path)
    return manifest


def read_binary_chunk(path: Path, key: tuple[int, int, int]) -> dict[str, Any]:
    _, entries, data = _parse_bundle(path)
    entry = entries.get(key)
    if entry is None:
        raise SpikeError("bundle chunk missing")
    offset, compressed_len, raw_len, digest = entry
    compressed = data[offset : offset + compressed_len]
    raw = bounded_decompress(compressed, raw_len)
    if hashlib.sha256(raw).digest() != digest:
        raise SpikeError("bundle chunk integrity mismatch")
    payload = _decode_json(raw)
    if not isinstance(payload, dict) or tuple(payload.get("chunk_key", [])) != key:
        raise SpikeError("bundle chunk key mismatch")
    return payload


def _timed(call: Callable[[], Any]) -> tuple[Any, float, int]:
    tracemalloc.start()
    started = time.perf_counter_ns()
    try:
        value = call()
        elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000.0
        _, peak = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()
    return value, elapsed_ms, peak


def _load_stats(call: Callable[[], Any], iterations: int) -> tuple[float, int]:
    if iterations < 1:
        raise SpikeError("load iterations must be positive")
    samples = []
    peak = 0
    for _ in range(iterations):
        _, elapsed_ms, measured_peak = _timed(call)
        samples.append(elapsed_ms)
        peak = max(peak, measured_peak)
    samples.sort()
    return samples[len(samples) // 2], peak


def _copy_artifact(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination) if destination.is_dir() else destination.unlink()
    if source.is_dir():
        shutil.copytree(source, destination)
    else:
        shutil.copy2(source, destination)


def _generic_change_metrics(before: Path, after: Path) -> tuple[int, int]:
    if before.is_file() and after.is_file():
        changed = before.read_bytes() != after.read_bytes()
        return (1 if changed else 0, after.stat().st_size if changed else 0)
    left = artifact_files(before)
    right = artifact_files(after)
    changed_names = [
        name
        for name in sorted(set(left) | set(right))
        if left.get(name) != right.get(name)
    ]
    changed_bytes = sum(len(right.get(name, b"")) for name in changed_names)
    return len(changed_names), changed_bytes


def _bundle_patch_bytes(before: Path, after: Path) -> tuple[int, int]:
    _, left_entries, left_data = _parse_bundle(before)
    _, right_entries, right_data = _parse_bundle(after)
    changed = 0
    patch_bytes = 0
    for key in sorted(set(left_entries) | set(right_entries)):
        left = left_entries.get(key)
        right = right_entries.get(key)
        if left is None or right is None:
            changed += 1
            if right is not None:
                patch_bytes += right[1] + INDEX.size
            continue
        lo, lc, _, _ = left
        ro, rc, _, _ = right
        if left_data[lo : lo + lc] != right_data[ro : ro + rc]:
            changed += 1
            patch_bytes += rc + INDEX.size
    return changed, patch_bytes


def _mutated_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    changed = copy.deepcopy(fixture)
    index = len(changed["cells"]) // 2
    cell = changed["cells"][index]
    cell["terrain"] = "marble" if cell["terrain"] != "marble" else "grass"
    return changed


def _corruption_rejected(
    candidate: str, artifact: Path, key: tuple[int, int, int], root: Path
) -> bool:
    damaged = root / f"{candidate}-corrupt"
    if candidate == "sqlite-project":
        damaged = damaged.with_suffix(".sqlite")
    elif candidate == "indexed-zlib-bundle":
        damaged = damaged.with_suffix(".bundle")
    _copy_artifact(artifact, damaged)
    try:
        if candidate == "chunked-json-tree":
            manifest = read_json_manifest(damaged)
            entry = next(row for row in manifest["chunks"] if tuple(row["key"]) == key)
            target = damaged / entry["path"]
            raw = bytearray(target.read_bytes())
            raw[-2] ^= 1
            target.write_bytes(raw)
            read_json_chunk(damaged, key)
        elif candidate == "sqlite-project":
            conn = sqlite3.connect(damaged)
            try:
                row = conn.execute(
                    "SELECT payload FROM chunks WHERE z=? AND cx=? AND cy=?",
                    (key[2], key[0], key[1]),
                ).fetchone()
                if row is None:
                    raise SpikeError("sqlite chunk missing during corruption injection")
                raw = bytearray(bytes(row[0]))
                raw[-2] ^= 1
                conn.execute(
                    "UPDATE chunks SET payload=? WHERE z=? AND cx=? AND cy=?",
                    (sqlite3.Binary(bytes(raw)), key[2], key[0], key[1]),
                )
                conn.commit()
            finally:
                conn.close()
            read_sqlite_chunk(damaged, key)
        else:
            _, entries, data = _parse_bundle(damaged)
            offset, compressed_len, _, _ = entries[key]
            raw = bytearray(data)
            raw[offset + compressed_len - 1] ^= 1
            damaged.write_bytes(raw)
            read_binary_chunk(damaged, key)
    except (SpikeError, sqlite3.DatabaseError, StopIteration):
        return True
    return False


def _candidate_paths(root: Path, candidate: str) -> tuple[Path, Path, Path]:
    if candidate == "chunked-json-tree":
        return root / "a", root / "b", root / "mutated"
    suffix = ".sqlite" if candidate == "sqlite-project" else ".bundle"
    return root / f"a{suffix}", root / f"b{suffix}", root / f"mutated{suffix}"


def _write_candidate(
    candidate: str, path: Path, fixture: dict[str, Any], chunk_size: int
) -> None:
    if candidate == "chunked-json-tree":
        write_json_project(path, fixture, chunk_size)
    elif candidate == "sqlite-project":
        write_sqlite_project(path, fixture, chunk_size)
    elif candidate == "indexed-zlib-bundle":
        write_binary_bundle(path, fixture, chunk_size, "server")
    else:
        raise SpikeError("unknown candidate")


def _read_candidate_chunk(
    candidate: str, path: Path, key: tuple[int, int, int]
) -> dict[str, Any]:
    if candidate == "chunked-json-tree":
        return read_json_chunk(path, key)
    if candidate == "sqlite-project":
        return read_sqlite_chunk(path, key)
    if candidate == "indexed-zlib-bundle":
        return read_binary_chunk(path, key)
    raise SpikeError("unknown candidate")


def _review_diff_lines(before: Path, after: Path) -> int | None:
    if not before.is_dir() or not after.is_dir():
        return None
    left = artifact_files(before)
    right = artifact_files(after)
    changed_lines = 0
    for name in sorted(set(left) | set(right)):
        if left.get(name) == right.get(name):
            continue
        old_lines = left.get(name, b"").decode("utf-8").splitlines()
        new_lines = right.get(name, b"").decode("utf-8").splitlines()
        for line in difflib.unified_diff(old_lines, new_lines, lineterm=""):
            if line.startswith(("+++", "---", "@@")):
                continue
            if line.startswith(("+", "-")):
                changed_lines += 1
    return changed_lines


def _measure_candidate(
    root: Path,
    candidate: str,
    fixture: dict[str, Any],
    chunk_size: int,
    load_iterations: int,
) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    a, b, mutated_path = _candidate_paths(root, candidate)
    _, build_ms, build_peak = _timed(
        lambda: _write_candidate(candidate, a, fixture, chunk_size)
    )
    _write_candidate(candidate, b, fixture, chunk_size)
    deterministic = artifact_digest(a) == artifact_digest(b)
    chunks = chunk_payloads(fixture, chunk_size)
    key = sorted(chunks)[len(chunks) // 2]
    load_ms, load_peak = _load_stats(
        lambda: _read_candidate_chunk(candidate, a, key), load_iterations
    )
    changed_fixture = _mutated_fixture(fixture)
    _write_candidate(candidate, mutated_path, changed_fixture, chunk_size)
    if candidate == "indexed-zlib-bundle":
        changed_units, patch_bytes = _bundle_patch_bytes(a, mutated_path)
    else:
        changed_units, patch_bytes = _generic_change_metrics(a, mutated_path)
    corruption_rejected = _corruption_rejected(candidate, a, key, root)
    artifact_bytes = artifact_size(a)
    source_bytes = len(canonical_json(fixture))
    measurement = {
        "candidate": candidate,
        "representation_role": "compiled-runtime"
        if candidate == "indexed-zlib-bundle"
        else "editable-project",
        "fixture_side": int(fixture["world_id"].split("-")[-1]),
        "cell_count": len(fixture["cells"]),
        "chunk_size": chunk_size,
        "chunk_count": len(chunks),
        "artifact_bytes": artifact_bytes,
        "artifact_to_source_ratio": round(artifact_bytes / source_bytes, 6),
        "artifact_sha256": artifact_digest(a),
        "deterministic_exact_bytes": deterministic,
        "build_ms": round(build_ms, 3),
        "build_peak_bytes": build_peak,
        "median_chunk_load_ms": round(load_ms, 3),
        "chunk_load_peak_bytes": load_peak,
        "changed_storage_units_after_one_cell_edit": changed_units,
        "estimated_patch_bytes_after_one_cell_edit": patch_bytes,
        "review_diff_lines_after_one_cell_edit": _review_diff_lines(a, mutated_path),
        "corruption_rejected": corruption_rejected,
    }
    return measurement


def _bundle_client_projection_evidence(
    root: Path, fixture: dict[str, Any], chunk_size: int
) -> dict[str, Any]:
    path = root / "client.bundle"
    write_binary_bundle(path, fixture, chunk_size, "client")
    manifest = read_binary_manifest(path)
    return {
        "artifact_bytes": path.stat().st_size,
        "artifact_sha256": artifact_digest(path),
        "projection": manifest.get("projection"),
        "server_only_absent": "server_only" not in manifest,
    }


def run_benchmarks(
    root: Path,
    scales: list[tuple[int, int]],
    load_iterations: int = 9,
) -> dict[str, Any]:
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    measurements = []
    fixtures = []
    client_projection = []
    for side, chunk_size in scales:
        fixture = make_fixture(side)
        fixture_bytes = canonical_json(fixture)
        fixtures.append(
            {
                "side": side,
                "cell_count": len(fixture["cells"]),
                "chunk_size": chunk_size,
                "sha256": sha256(fixture_bytes),
                "canonical_json_bytes": len(fixture_bytes),
            }
        )
        scale_root = root / f"s{side}-c{chunk_size}"
        for candidate in ("chunked-json-tree", "sqlite-project", "indexed-zlib-bundle"):
            measurements.append(
                _measure_candidate(
                    scale_root / candidate,
                    candidate,
                    fixture,
                    chunk_size,
                    load_iterations,
                )
            )
        client_projection.append(
            {
                "side": side,
                "chunk_size": chunk_size,
                **_bundle_client_projection_evidence(scale_root, fixture, chunk_size),
            }
        )
    negatives = _negative_evidence(root)
    return {
        "schema_version": 1,
        "spike_invariant": INVARIANT,
        "environment": {
            "python": platform.python_version(),
            "sqlite": sqlite3.sqlite_version,
            "zlib": zlib.ZLIB_VERSION,
            "platform": platform.platform(),
        },
        "configuration": {
            "load_iterations": load_iterations,
            "max_artifact_bytes": MAX_ARTIFACT_BYTES,
            "max_chunk_raw_bytes": MAX_CHUNK_RAW_BYTES,
            "max_chunks": MAX_CHUNKS,
            "max_string_bytes": MAX_STRING_BYTES,
            "max_depth": MAX_DEPTH,
            "max_collection": MAX_COLLECTION,
            "max_decompression_ratio": MAX_DECOMPRESSION_RATIO,
            "compression": "zlib level 6 for indexed-zlib-bundle",
        },
        "fixtures": fixtures,
        "measurements": measurements,
        "client_projection_evidence": client_projection,
        "negative_evidence": negatives,
    }


def _negative_evidence(root: Path) -> dict[str, bool]:
    results: dict[str, bool] = {}
    compressed = zlib.compress(b"A" * 4096, 9)
    try:
        bounded_decompress(compressed, 4096, 4096, 2.0)
        results["decompression_ratio_rejected"] = False
    except SpikeError:
        results["decompression_ratio_rejected"] = True
    fixture = make_fixture(8)
    fixture["critical_features"] = ["future-critical"]
    try:
        validate_fixture(fixture)
        results["unknown_critical_rejected"] = False
    except SpikeError:
        results["unknown_critical_rejected"] = True
    bundle = root / "negative.bundle"
    write_binary_bundle(bundle, make_fixture(8), 8, "server")
    truncated = root / "negative-truncated.bundle"
    truncated.write_bytes(bundle.read_bytes()[:-7])
    try:
        read_binary_manifest(truncated)
        results["truncated_bundle_rejected"] = False
    except SpikeError:
        results["truncated_bundle_rejected"] = True
    oversized_json = canonical_json({"x": "A" * (MAX_STRING_BYTES + 1)})
    try:
        _decode_json(oversized_json)
        results["oversized_string_rejected"] = False
    except SpikeError:
        results["oversized_string_rejected"] = True
    nested: object = "leaf"
    for _ in range(MAX_DEPTH + 2):
        nested = {"x": nested}
    try:
        validate_tree(nested)
        results["nesting_depth_rejected"] = False
    except SpikeError:
        results["nesting_depth_rejected"] = True
    try:
        validate_tree([0] * (MAX_COLLECTION + 1))
        results["collection_count_rejected"] = False
    except SpikeError:
        results["collection_count_rejected"] = True
    project = root / "negative-json-project"
    write_json_project(project, make_fixture(8), 8)
    manifest = read_json_manifest(project)
    entry = manifest["chunks"][0]
    key = tuple(entry["key"])
    payload = read_json_chunk(project, key)
    escape = root / "negative-escape.json"
    escape.write_bytes(canonical_pretty_json(payload))
    entry["path"] = "../negative-escape.json"
    entry["sha256"] = sha256(escape.read_bytes())
    (project / "manifest.json").write_bytes(canonical_pretty_json(manifest))
    try:
        read_json_chunk(project, key)
        results["path_traversal_rejected"] = False
    except SpikeError:
        results["path_traversal_rejected"] = True
    return results


def default_scales() -> list[tuple[int, int]]:
    return [(32, 32), (64, 32), (64, 64), (128, 32), (128, 64)]


def write_results(path: Path, result: dict[str, Any]) -> None:
    atomic_write(
        path,
        json.dumps(result, sort_keys=True, indent=2, ensure_ascii=False).encode("utf-8")
        + b"\n",
    )


def render_dossier(result: dict[str, Any], exact_base_sha: str) -> str:
    env = result["environment"]
    config = result["configuration"]
    rows = result["measurements"]
    lines = [
        "# OTV2 Content Format Spike — Decision Dossier",
        "",
        f"- Exact worker base: `{exact_base_sha}`",
        f"- Spike invariant: **`{result['spike_invariant']}`**",
        "- Authority: evidence only; permanent World Project / World Bundle format remains owner-gated.",
        "",
        "## Reproducibility",
        "",
        f"- Python: `{env['python']}`",
        f"- SQLite: `{env['sqlite']}`",
        f"- zlib: `{env['zlib']}`",
        f"- Platform: `{env['platform']}`",
        f"- Load iterations per cell: `{config['load_iterations']}`",
        f"- Decompression ratio hard fence in spike: `{config['max_decompression_ratio']}:1`",
        "",
        "## Measured evidence",
        "",
        "| Candidate | Role | Side | Chunk | Bytes | Build ms | Load ms | Edit units | Patch bytes | Diff lines | Deterministic | Corruption rejected |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|",
    ]
    for row in rows:
        lines.append(
            f"| `{row['candidate']}` | {row['representation_role']} | {row['fixture_side']} | "
            f"{row['chunk_size']} | {row['artifact_bytes']} | {row['build_ms']:.3f} | "
            f"{row['median_chunk_load_ms']:.3f} | {row['changed_storage_units_after_one_cell_edit']} | "
            f"{row['estimated_patch_bytes_after_one_cell_edit']} | "
            f"{row['review_diff_lines_after_one_cell_edit'] if row['review_diff_lines_after_one_cell_edit'] is not None else '-'} | "
            f"{'yes' if row['deterministic_exact_bytes'] else 'NO'} | "
            f"{'yes' if row['corruption_rejected'] else 'NO'} |"
        )

    lines.extend(
        [
            "",
            "## Trade-off matrix",
            "",
            "| Concern | `chunked-json-tree` | `sqlite-project` | `indexed-zlib-bundle` |",
            "|---|---|---|---|",
            "| Primary fit | Editable source/project | Editable transactional container | Compiled runtime artifact |",
            "| Git review / merge | Strong: per-chunk canonical text files | Weak: single binary database | Weak as source; not intended for authoring |",
            "| Partial / atomic save | Per-file atomic replace; journal still needed for multi-file save | Transaction-capable in principle; benchmark uses `journal_mode=OFF`, so crash recovery is not proven | Read-only build artifact; compiler atomically replaces whole artifact |",
            "| Random chunk access | Direct file lookup after manifest | Indexed SQL primary key | Explicit bounded binary index |",
            "| Corruption fence | Manifest per-chunk SHA-256 | Per-row SHA-256 checked by loader | Per-chunk SHA-256 plus zlib decode bounds |",
            "| Patch locality | Changed chunk files | Container-level unless SQLite-aware delta tooling exists | Chunk payloads are independently indexed; patch protocol remains unselected |",
            "| Interoperability | Very high | High | Requires published schema/container contract |",
            "| Studio ergonomics | Simple inspectability; many-file lifecycle complexity | Strong transactional query/edit model | Runtime-oriented, not an editor source |",
            "| Schema evolution | Explicit versions/critical features; final unknown-field policy unfrozen | Same semantic envelope, DB migrations required | Explicit bundle version/critical features; final compatibility policy unfrozen |",
            "| Crash recovery | Multi-file recovery journal not implemented | Not evaluated with WAL/rollback journal in this deterministic-byte benchmark | Immutable rebuild/replace model only; rollout recovery not evaluated |",
        ]
    )
    negatives = result["negative_evidence"]
    projections = result["client_projection_evidence"]
    lines.extend(
        [
            "",
            "## Fail-closed and projection evidence",
            "",
            f"- Corruption rejected for every measured candidate: **{'yes' if all(r['corruption_rejected'] for r in rows) else 'NO'}**.",
            f"- Decompression-ratio adversarial case rejected: **{'yes' if negatives['decompression_ratio_rejected'] else 'NO'}**.",
            f"- Truncated bundle rejected: **{'yes' if negatives['truncated_bundle_rejected'] else 'NO'}**.",
            f"- Oversized string rejected: **{'yes' if negatives['oversized_string_rejected'] else 'NO'}**.",
            f"- Unknown critical feature rejected: **{'yes' if negatives['unknown_critical_rejected'] else 'NO'}**.",
            f"- Nesting-depth overflow rejected: **{'yes' if negatives['nesting_depth_rejected'] else 'NO'}**.",
            f"- Collection-count overflow rejected: **{'yes' if negatives['collection_count_rejected'] else 'NO'}**.",
            f"- JSON chunk path traversal rejected: **{'yes' if negatives['path_traversal_rejected'] else 'NO'}**.",
            f"- Client projection excludes `server_only` data for every measured scale: **{'yes' if all(r['server_only_absent'] for r in projections) else 'NO'}**.",
            "",
            "## Migration and provenance boundary",
            "",
            "The fixtures are deterministic project-owned synthetic data. This spike does **not** prove Crystal/OTBM semantic parity, broad import completeness, or redistribution rights. Any real importer must retain pinned source digests, conversion diagnostics, unresolved/lossy semantics and zero-silent-loss reporting before format selection.",
            "",
            "## Not proven by this spike",
            "",
            "- SQLite crash recovery/WAL behavior is not measured; the deterministic-byte prototype disables journaling during one-shot artifact construction.",
            "- The binary prototype has per-chunk SHA-256 and bounded zlib decoding, but no separate manifest checksum/signature, release signing, CDN layout or production patch protocol.",
            "- Real Crystal/OTBM import parity, exact item/appearance catalog binding and zero-silent-loss corpus conversion remain outside this synthetic benchmark.",
            "- Final unknown-optional-field compatibility rules, schema migration tooling, Studio concurrent-edit UX and autosave journals remain unfrozen.",
            "- The synthetic composite fountain proves semantic/visual-footprint separation can be represented; renderer correctness and real multi-tile import recognition are not evaluated here.",
            "",
            "## Evidence candidate recommendation",
            "",
            "**RECOMMENDATION — not a format decision:** keep the editable-project and runtime-bundle concerns separate. The measured `chunked-json-tree` is the clearest baseline for Git review and bounded parallel authoring; `sqlite-project` is a credible Studio-oriented alternative when transactional multi-object edits dominate; `indexed-zlib-bundle` is the strongest of these three prototypes for a compiled runtime artifact because it is deterministic, indexed, bounded and per-chunk integrity checked.",
            "",
            "This recommendation does not freeze extensions, physical schemas, chunk dimensions, compression, patch protocol, signing, CDN layout, or compatibility policy.",
            "",
            "## Owner decision required",
            "",
            "The owner must separately **select / rework / defer** the permanent World Project and World Bundle physical formats after reviewing this dossier and any additional Studio/import/runtime evidence. `SPIKE_RESULT != OWNER_FORMAT_DECISION` remains binding.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Bounded Oteryn content-format evidence spike"
    )
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--dossier", type=Path)
    parser.add_argument("--base-sha")
    parser.add_argument("--iterations", type=int, default=9)
    args = parser.parse_args()
    if args.dossier is not None and not args.base_sha:
        parser.error("--base-sha is required when --dossier is supplied")
    result = run_benchmarks(args.work_dir, default_scales(), args.iterations)
    write_results(args.results, result)
    if args.dossier is not None:
        dossier = render_dossier(result, exact_base_sha=args.base_sha)
        atomic_write(args.dossier, dossier.rstrip("\n").encode("utf-8") + b"\n")
    print(f"{INVARIANT}")
    print(f"measurements={len(result['measurements'])}")
    print(f"results={args.results}")
    if args.dossier is not None:
        print(f"dossier={args.dossier}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
