#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import difflib
import hashlib
import importlib.util
import json
import os
import platform
import shutil
import sqlite3
import struct
import subprocess
import sys
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


D3_REAL_BATCH_PROFILE = "OTV2_CONTENT_WORLD_D3_REAL_BATCH/v1"
D3_CARRIER_SCHEMA = 1
D3_LOGICAL_INDEX_PROFILE = "OTV2_D3_SOURCE_CELL_INDEX/v1"
D3_SOURCE_CLASSIFICATION = "MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY"
D3_FRESH_SOURCE_PROFILE = "oteryn-crystalserver-fresh-source-generation-v2"
D3_MEASUREMENT_CHUNK_SIZE = 32
D3_COMPRESSIONS = {"none", "zlib"}
D3_WINDOWS = (
    ("newhaven", 32512, 32544, 32512, 32544, -7, "f-7-r1016-c1016"),
    ("targuna", 31904, 31936, 31904, 31936, -7, "f-7-r997-c997"),
)
D3_SHARED_TOP_LEVEL_FIELDS = frozenset(
    {"schema_version", "world_id", "critical_features", "provenance", "definitions", "cells"}
)
D3_PROVENANCE_FIELDS = frozenset(
    {
        "measurement_profile",
        "classification",
        "source_generation_profile_id",
        "source_generation_profile_revision",
        "source_repository",
        "source_repository_sha",
        "world_otbm_sha256",
        "world_otbm_git_blob",
        "world_otbm_bytes",
        "asset_zip_sha256",
        "asset_catalog_sha256",
        "asset_appearance_sha256",
        "parser_repository",
        "parser_repository_sha",
        "game_measurement_head",
        "game_readonly_code",
        "selection",
    }
)
D3_IDENTITY_PROVENANCE_FIELDS = (
    "measurement_profile",
    "classification",
    "source_generation_profile_id",
    "source_generation_profile_revision",
    "source_repository",
    "source_repository_sha",
    "world_otbm_sha256",
    "world_otbm_git_blob",
    "world_otbm_bytes",
    "asset_zip_sha256",
    "asset_catalog_sha256",
    "asset_appearance_sha256",
    "parser_repository",
    "parser_repository_sha",
)
D3_DEFINITION_FIELDS = frozenset(
    {"definition_kind", "appearance_source_id", "identity_disposition", "production_authority"}
)
D3_CELL_FIELDS = frozenset({"x", "y", "z", "source_placements"})
D3_PLACEMENT_FIELDS = frozenset(
    {
        "source_occurrence_ref",
        "appearance_source_id",
        "source_role",
        "source_presentation_order",
        "identity_disposition",
        "typed_definition_ref",
        "placement_key",
        "target_sensitive_fields",
    }
)


def _load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SpikeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _git_output(root: Path, *args: str) -> str:
    return subprocess.run(
        ("git", "-C", str(root), *args),
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.strip()

def _verify_d3_readonly_code(game_root: Path) -> dict[str, Any]:
    game_root = game_root.resolve()
    if Path(_git_output(game_root, "rev-parse", "--show-toplevel")).resolve() != game_root:
        raise SpikeError("D3 Game root is not repository root")
    relatives = (
        "tools/game-atlas-fullworld-source/producer.py",
        "tools/reference-world-corridor-census/content_source_batch.py",
    )
    status = _git_output(
        game_root, "status", "--porcelain=v1", "--untracked-files=all", "--", *relatives
    )
    if status:
        raise SpikeError("D3 read-only producer/batch paths are dirty")
    result: dict[str, Any] = {"game_head": _git_output(game_root, "rev-parse", "HEAD"), "files": {}}
    for relative in relatives:
        working_blob = _git_output(game_root, "hash-object", "--", relative)
        committed_blob = _git_output(game_root, "rev-parse", f"HEAD:{relative}")
        if working_blob != committed_blob:
            raise SpikeError(f"D3 read-only code blob mismatch: {relative}")
        result["files"][relative] = {
            "blob": committed_blob,
            "last_commit": _git_output(game_root, "log", "-1", "--format=%H", "--", relative),
        }
    return result


def _d3_inside_tile(producer: Any, tile: Any, window: tuple[Any, ...]) -> bool:
    _name, xmin, xmax, ymin, ymax, floor, _shard = window
    return (
        xmin <= tile.position.x < xmax
        and ymin <= tile.position.y < ymax
        and producer.native_floor(tile) == floor
    )

def _collect_d3_tiles(producer: Any, runtime: Any) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    selected: dict[str, list[dict[str, Any]]] = {row[0]: [] for row in D3_WINDOWS}
    stream = {"map_header": 0, "tile": 0, "town": 0, "waypoint": 0}
    for record in producer.iter_records(runtime, strict=True):
        if producer.is_map_header(runtime, record):
            stream["map_header"] += 1
            continue
        if producer.is_town(runtime, record):
            stream["town"] += 1
            continue
        if producer.is_waypoint(runtime, record):
            stream["waypoint"] += 1
            continue
        if not producer.is_tile(runtime, record):
            continue
        stream["tile"] += 1
        for window in D3_WINDOWS:
            if _d3_inside_tile(producer, record, window):
                projected, _stats = producer.project_tile(runtime, record)
                selected[window[0]].append(projected)
                break
    combined: list[dict[str, Any]] = []
    window_evidence: list[dict[str, Any]] = []
    for name, xmin, xmax, ymin, ymax, floor, shard in D3_WINDOWS:
        rows = selected[name]
        rows.sort(key=lambda row: (row["position"]["floor"], row["position"]["y"], row["position"]["x"]))
        raw = b"".join(producer.canonical_tile_bytes(runtime, row) for row in rows)
        window_evidence.append({
            "name": name, "bounds": [xmin, xmax, ymin, ymax], "floor": floor,
            "retained_shard": shard, "tile_records": len(rows),
            "source_occurrences": sum(len(row.get("presentation", [])) for row in rows),
            "ordered_stream_sha256": sha256(raw), "ordered_stream_bytes": len(raw),
        })
        combined.extend(rows)
    if not combined:
        raise SpikeError("D3 fixed windows produced no tile records")
    return combined, {"stream_counts": stream, "windows": window_evidence}

def _d3_source_summary(
    producer: Any,
    profile: Any,
    code_provenance: dict[str, Any],
) -> dict[str, Any]:
    producer_rel = "tools/game-atlas-fullworld-source/producer.py"
    return {
        "classification": D3_SOURCE_CLASSIFICATION,
        "phase_a_result": "PASS",
        "phase_b_target_parity": "NOT_PERFORMED",
        "producer": {
            "api": producer.PRODUCER_API,
            "code_commit": code_provenance["files"][producer_rel]["last_commit"],
            "repository": "Oteryn/Oteryn-Game",
        },
        "source": {
            "appearance_sha256": profile.asset_appearance_sha256,
            "asset_zip_sha256": profile.asset_zip_sha256,
            "catalog_sha256": profile.asset_catalog_sha256,
            "legacy_revision": profile.parser_repository_sha,
            "world_otbm_sha256": profile.world_otbm_sha256,
        },
    }


def _d3_definition_key(appearance_source_id: int) -> str:
    return f"source-appearance:{appearance_source_id}"


def _d3_occurrence_sort_key(row: dict[str, Any]) -> tuple[bytes, str]:
    return (
        canonical_json(row.get("source_presentation_order")),
        str(row.get("source_occurrence_ref", "")),
    )

def d3_fixture_from_typed_batch(
    typed_batch: dict[str, Any],
    *,
    source_profile: dict[str, Any],
    selection_evidence: dict[str, Any],
    typed_batch_sha256: str,
    typed_batch_bytes: int,
) -> dict[str, Any]:
    if typed_batch.get("production_authority") != "NONE":
        raise SpikeError("D3 typed batch unexpectedly carries production authority")
    if typed_batch.get("reference_parity_claim") != "NONE":
        raise SpikeError("D3 typed batch unexpectedly carries Reference parity")
    cells: dict[tuple[int, int, int], dict[str, Any]] = {}
    definitions: dict[str, dict[str, Any]] = {}
    deferred = typed_batch.get("deferred_requires_phase_b")
    if not isinstance(deferred, list) or not deferred:
        raise SpikeError("D3 typed batch lacks deferred target-sensitive fields")

    def add_occurrence(source: dict[str, Any], *, disposition: str, typed_ref: Any, placement_key: Any) -> None:
        pos = source.get("source_native_position")
        if not isinstance(pos, dict):
            raise SpikeError("D3 source occurrence lacks native source position")
        key = (int(pos["x"]), int(pos["y"]), int(pos["floor"]))
        cell = cells.setdefault(key, {"x": key[0], "y": key[1], "z": key[2], "source_placements": []})
        appearance_id = int(source["appearance_source_id"])
        def_key = _d3_definition_key(appearance_id)
        definitions.setdefault(def_key, {
            "definition_kind": "SOURCE_APPEARANCE_REFERENCE",
            "appearance_source_id": appearance_id,
            "identity_disposition": "SOURCE_ID_ONLY_NOT_CANONICAL",
            "production_authority": "NONE",
        })

        cell["source_placements"].append({
            "source_occurrence_ref": source["source_occurrence_ref"],
            "appearance_source_id": appearance_id,
            "source_role": source.get("source_role"),
            "source_presentation_order": source.get("source_presentation_order"),
            "identity_disposition": disposition,
            "typed_definition_ref": typed_ref,
            "placement_key": placement_key,
            "target_sensitive_fields": {field: "DEFERRED_REQUIRES_PHASE_B" for field in deferred},
        })

    for row in typed_batch.get("unresolved_source_occurrences", []):
        add_occurrence(row, disposition="UNRESOLVED_SOURCE_IDENTITY", typed_ref=None, placement_key=None)
    for row in typed_batch.get("placements", []):
        source = row.get("source_provenance")
        if not isinstance(source, dict):
            raise SpikeError("D3 bound placement lacks source provenance")
        add_occurrence(
            source,
            disposition="EXPLICITLY_BOUND",
            typed_ref=row.get("typed_definition_ref"),
            placement_key=row.get("placement_key"),
        )
    fixture = {
        "schema_version": 1,
        "world_id": "d3-fresh-crystal-bounded-real-batch",
        "critical_features": ["chunk-index-v1", "projection-v1"],
        "provenance": {
            "measurement_profile": D3_REAL_BATCH_PROFILE,
            "classification": D3_SOURCE_CLASSIFICATION,
            **source_profile,
            "selection": selection_evidence,
        },
        "definitions": definitions,
        "server_only": {
            "typed_batch_schema": typed_batch.get("schema"),
            "typed_batch_sha256": typed_batch_sha256,
            "typed_batch_bytes": typed_batch_bytes,
            "typed_batch_counts": typed_batch.get("counts"),
            "deferred_requires_phase_b": deferred,
            "production_authority": "NONE",
            "reference_parity_claim": "NONE",
        },
        "cells": list(cells.values()),
    }
    return d3_normalize_fixture(fixture)

def d3_normalize_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    normalized = copy.deepcopy(fixture)
    cells = normalized.get("cells")
    if not isinstance(cells, list):
        raise SpikeError("D3 fixture cells must be a list")
    for cell in cells:
        placements = cell.get("source_placements")
        if not isinstance(placements, list):
            raise SpikeError("D3 fixture cell lacks source placements")
        placements.sort(key=_d3_occurrence_sort_key)
    cells.sort(key=lambda row: (row["z"], row["y"], row["x"]))
    definitions = normalized.get("definitions")
    if not isinstance(definitions, dict):
        raise SpikeError("D3 definitions must be indexed object")
    normalized["definitions"] = {key: definitions[key] for key in sorted(definitions)}
    return normalized


def validate_d3_fixture(fixture: dict[str, Any]) -> None:
    fixture = d3_normalize_fixture(fixture)
    validate_fixture(fixture)
    allowed_top = D3_SHARED_TOP_LEVEL_FIELDS | {"server_only"}
    if not D3_SHARED_TOP_LEVEL_FIELDS.issubset(fixture) or not set(fixture).issubset(allowed_top):
        raise SpikeError("D3 fixture contains non-allowlisted top-level fields")

    provenance = fixture.get("provenance")
    if not isinstance(provenance, dict) or provenance.get("classification") != D3_SOURCE_CLASSIFICATION:
        raise SpikeError("D3 source classification mismatch")
    if provenance.get("measurement_profile") != D3_REAL_BATCH_PROFILE:
        raise SpikeError("D3 measurement profile mismatch")
    if not set(provenance).issubset(D3_PROVENANCE_FIELDS):
        raise SpikeError("D3 provenance contains non-allowlisted fields")

    server_only = fixture.get("server_only")
    if server_only is not None:
        if not isinstance(server_only, dict):
            raise SpikeError("D3 server-only metadata must be an object")
        if server_only.get("production_authority") != "NONE":
            raise SpikeError("D3 server-only metadata carries production authority")
        if server_only.get("reference_parity_claim") != "NONE":
            raise SpikeError("D3 server-only metadata carries Reference parity")

    definitions = fixture.get("definitions")
    if not isinstance(definitions, dict):
        raise SpikeError("D3 definitions must be an object")
    for definition in definitions.values():
        if not isinstance(definition, dict) or set(definition) != D3_DEFINITION_FIELDS:
            raise SpikeError("D3 definition contains non-allowlisted fields")
        if definition.get("identity_disposition") != "SOURCE_ID_ONLY_NOT_CANONICAL":
            raise SpikeError("D3 source definition promoted canonical identity")
        if definition.get("production_authority") != "NONE":
            raise SpikeError("D3 source definition carries production authority")

    seen: set[str] = set()
    for cell in fixture["cells"]:
        if set(cell) != D3_CELL_FIELDS:
            raise SpikeError("D3 cell contains non-allowlisted fields")
        for placement in cell["source_placements"]:
            if not isinstance(placement, dict) or set(placement) != D3_PLACEMENT_FIELDS:
                raise SpikeError("D3 placement contains non-allowlisted fields")
            ref = placement.get("source_occurrence_ref")
            if not isinstance(ref, str) or not ref or ref in seen:
                raise SpikeError("D3 source occurrence identity is missing or duplicated")
            seen.add(ref)
            if not isinstance(placement.get("appearance_source_id"), int) or placement["appearance_source_id"] <= 0:
                raise SpikeError("D3 appearance source ID must be positive")
            disposition = placement.get("identity_disposition")
            typed_ref = placement.get("typed_definition_ref")
            placement_key = placement.get("placement_key")
            if disposition == "UNRESOLVED_SOURCE_IDENTITY":
                if typed_ref is not None or placement_key is not None:
                    raise SpikeError("D3 unresolved source identity carries a target binding")
            elif disposition == "EXPLICITLY_BOUND":
                if not isinstance(typed_ref, dict) or not isinstance(placement_key, str) or not placement_key:
                    raise SpikeError("D3 explicit source binding is incomplete")
            else:
                raise SpikeError("D3 source identity disposition is unsupported")
            target_fields = placement.get("target_sensitive_fields")
            if not isinstance(target_fields, dict) or not target_fields:
                raise SpikeError("D3 placement lacks deferred target-sensitive fields")
            if set(target_fields.values()) != {"DEFERRED_REQUIRES_PHASE_B"}:
                raise SpikeError("D3 target-sensitive field was promoted")


def d3_logical_identity(fixture: dict[str, Any]) -> str:
    normalized = d3_normalize_fixture(fixture)
    validate_d3_fixture(normalized)
    provenance = normalized["provenance"]
    identity_payload = {
        "schema_version": normalized["schema_version"],
        "world_id": normalized["world_id"],
        "critical_features": normalized["critical_features"],
        "provenance": {
            key: provenance[key]
            for key in D3_IDENTITY_PROVENANCE_FIELDS
            if key in provenance
        },
        "definitions": normalized["definitions"],
        "cells": normalized["cells"],
    }
    return sha256(canonical_json(identity_payload))


def _d3_chunk_path(key: tuple[int, int, int]) -> str:
    return f"chunks/z{key[2]}/c{key[0]}_{key[1]}.bin"


def write_d3_carrier(
    path: Path,
    fixture: dict[str, Any],
    chunk_size: int,
    *,
    compression: str,
    projection: str,
) -> None:
    normalized = d3_normalize_fixture(fixture)
    validate_d3_fixture(normalized)
    if compression not in D3_COMPRESSIONS:
        raise SpikeError("unsupported D3 compression")
    if projection not in PROJECTION:
        raise SpikeError("unsupported D3 projection")
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True)
    chunks = chunk_payloads(normalized, chunk_size)
    manifest = manifest_from_fixture(normalized, chunk_size, projection)
    manifest.update({
        "d3_carrier_schema": D3_CARRIER_SCHEMA,
        "d3_profile": D3_REAL_BATCH_PROFILE,
        "logical_index_profile": D3_LOGICAL_INDEX_PROFILE,
        "logical_identity_sha256": d3_logical_identity(normalized),
        "compression": compression,
    })

    entries: list[dict[str, Any]] = []
    placement_index: dict[str, list[int]] = {}
    for key, payload in sorted(chunks.items()):
        raw = canonical_json(payload)
        if len(raw) > MAX_CHUNK_RAW_BYTES:
            raise SpikeError("D3 raw chunk byte limit exceeded")
        stored = raw if compression == "none" else zlib.compress(raw, 6)
        relative = _d3_chunk_path(key)
        atomic_write(path / relative, stored)
        entries.append({
            "key": list(key),
            "path": relative,
            "sha256": sha256(stored),
            "stored_bytes": len(stored),
            "raw_bytes": len(raw),
            "raw_sha256": sha256(raw),
        })
        for cell in payload["cells"]:
            for placement in cell["source_placements"]:
                ref = placement["source_occurrence_ref"]
                if ref in placement_index:
                    raise SpikeError("duplicate D3 placement index key")
                placement_index[ref] = list(key)
    manifest["chunks"] = entries
    manifest["placement_index"] = {key: placement_index[key] for key in sorted(placement_index)}
    manifest_bytes = canonical_json(manifest) + b"\n"
    if len(manifest_bytes) > MAX_CHUNK_RAW_BYTES:
        raise SpikeError("D3 manifest byte limit exceeded")
    atomic_write(path / "manifest.json", manifest_bytes)
    if artifact_size(path) > MAX_ARTIFACT_BYTES:
        raise SpikeError("D3 carrier artifact byte limit exceeded")

def read_d3_manifest(path: Path, *, expected_source_profile: str) -> dict[str, Any]:
    manifest = read_json_manifest(path)
    if manifest.get("d3_carrier_schema") != D3_CARRIER_SCHEMA:
        raise SpikeError("unsupported D3 carrier version")
    if manifest.get("d3_profile") != D3_REAL_BATCH_PROFILE:
        raise SpikeError("wrong D3 carrier profile")
    if manifest.get("logical_index_profile") != D3_LOGICAL_INDEX_PROFILE:
        raise SpikeError("wrong D3 logical index profile")
    compression = manifest.get("compression")
    if compression not in D3_COMPRESSIONS:
        raise SpikeError("unsupported D3 compression")
    provenance = manifest.get("provenance")
    if not isinstance(provenance, dict):
        raise SpikeError("D3 manifest lacks provenance")
    if provenance.get("source_generation_profile_id") != expected_source_profile:
        raise SpikeError("D3 source generation profile mismatch")
    placement_index = manifest.get("placement_index")
    if not isinstance(placement_index, dict) or len(placement_index) > MAX_COLLECTION:
        raise SpikeError("D3 placement index invalid")
    for ref, key in placement_index.items():
        if not isinstance(ref, str) or not ref:
            raise SpikeError("D3 placement index key invalid")
        if not isinstance(key, list) or len(key) != 3 or any(not isinstance(v, int) for v in key):
            raise SpikeError("D3 placement index chunk key invalid")
    return manifest


def _d3_entry(manifest: dict[str, Any], key: tuple[int, int, int]) -> dict[str, Any]:
    entry = next((row for row in manifest["chunks"] if tuple(row["key"]) == key), None)
    if entry is None:
        raise SpikeError("D3 chunk missing")
    return entry

def read_d3_chunk(
    path: Path,
    key: tuple[int, int, int],
    *,
    expected_source_profile: str,
) -> dict[str, Any]:
    manifest = read_d3_manifest(path, expected_source_profile=expected_source_profile)
    entry = _d3_entry(manifest, key)
    relative = entry["path"]
    root = path.resolve()
    chunk_path = (path / relative).resolve()
    try:
        chunk_path.relative_to(root)
    except ValueError as exc:
        raise SpikeError("D3 chunk path escapes carrier root") from exc
    if not chunk_path.is_file() or chunk_path.stat().st_size != entry.get("stored_bytes"):
        raise SpikeError("D3 stored chunk size mismatch")
    stored = chunk_path.read_bytes()
    if sha256(stored) != entry["sha256"]:
        raise SpikeError("D3 stored chunk checksum mismatch")
    raw_bytes = entry.get("raw_bytes")
    raw_sha = entry.get("raw_sha256")
    if not isinstance(raw_bytes, int) or raw_bytes < 0 or raw_bytes > MAX_CHUNK_RAW_BYTES:
        raise SpikeError("D3 raw chunk size invalid")
    if not isinstance(raw_sha, str) or len(raw_sha) != 64:
        raise SpikeError("D3 raw chunk checksum invalid")
    if manifest["compression"] == "none":
        raw = stored
        if len(raw) != raw_bytes:
            raise SpikeError("D3 uncompressed raw-size mismatch")
    else:
        raw = bounded_decompress(stored, raw_bytes)
    if sha256(raw) != raw_sha:
        raise SpikeError("D3 raw chunk checksum mismatch")
    payload = _decode_json(raw)
    if not isinstance(payload, dict) or tuple(payload.get("chunk_key", [])) != key:
        raise SpikeError("D3 chunk key mismatch")
    return payload

def read_d3_cell(
    path: Path,
    position: tuple[int, int, int],
    *,
    expected_source_profile: str,
) -> dict[str, Any]:
    manifest = read_d3_manifest(path, expected_source_profile=expected_source_profile)
    x, y, z = position
    chunk_size = int(manifest["chunk_size"])
    payload = read_d3_chunk(
        path,
        (x // chunk_size, y // chunk_size, z),
        expected_source_profile=expected_source_profile,
    )
    cell = next((row for row in payload["cells"] if (row["x"], row["y"], row["z"]) == position), None)
    if cell is None:
        raise SpikeError("D3 cell missing")
    return cell


def read_d3_placement(
    path: Path,
    source_occurrence_ref: str,
    *,
    expected_source_profile: str,
) -> dict[str, Any]:
    manifest = read_d3_manifest(path, expected_source_profile=expected_source_profile)
    key = manifest["placement_index"].get(source_occurrence_ref)
    if key is None:
        raise SpikeError("D3 placement missing")
    payload = read_d3_chunk(path, tuple(key), expected_source_profile=expected_source_profile)
    for cell in payload["cells"]:
        for placement in cell["source_placements"]:
            if placement["source_occurrence_ref"] == source_occurrence_ref:
                return placement
    raise SpikeError("D3 placement index is stale")

def read_d3_definition(
    path: Path,
    definition_key: str,
    *,
    expected_source_profile: str,
) -> dict[str, Any]:
    manifest = read_d3_manifest(path, expected_source_profile=expected_source_profile)
    definitions = manifest.get("definitions")
    if not isinstance(definitions, dict):
        raise SpikeError("D3 definitions index invalid")
    definition = definitions.get(definition_key)
    if not isinstance(definition, dict):
        raise SpikeError("D3 definition missing")
    return definition


def reconstruct_d3_fixture(path: Path, *, expected_source_profile: str) -> dict[str, Any]:
    manifest = read_d3_manifest(path, expected_source_profile=expected_source_profile)
    cells: list[dict[str, Any]] = []
    for row in manifest["chunks"]:
        payload = read_d3_chunk(
            path, tuple(row["key"]), expected_source_profile=expected_source_profile
        )
        cells.extend(payload["cells"])
    fixture = {
        "schema_version": 1,
        "world_id": manifest["world_id"],
        "critical_features": manifest["critical_features"],
        "provenance": manifest["provenance"],
        "definitions": manifest["definitions"],
        "cells": cells,
    }
    if "server_only" in manifest:
        fixture["server_only"] = manifest["server_only"]
    return d3_normalize_fixture(fixture)


def d3_client_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    normalized = d3_normalize_fixture(fixture)
    validate_d3_fixture(normalized)
    return {
        key: copy.deepcopy(normalized[key])
        for key in (
            "schema_version",
            "world_id",
            "critical_features",
            "provenance",
            "definitions",
            "cells",
        )
    }

def d3_index_signature(manifest: dict[str, Any]) -> str:
    logical_entries = [
        {
            "key": row["key"],
            "path": row["path"],
            "raw_bytes": row["raw_bytes"],
            "raw_sha256": row["raw_sha256"],
        }
        for row in manifest["chunks"]
    ]
    value = {
        "chunks": logical_entries,
        "placement_index": manifest["placement_index"],
        "definition_keys": sorted(manifest["definitions"]),
    }
    return sha256(canonical_json(value))


def _decoded_field_count(value: Any) -> int:
    if isinstance(value, dict):
        return len(value) + sum(_decoded_field_count(child) for child in value.values())
    if isinstance(value, list):
        return sum(_decoded_field_count(child) for child in value)
    return 0


def _d3_record_size_stats(fixture: dict[str, Any]) -> dict[str, int]:
    cells = fixture["cells"]
    placements = [
        placement
        for cell in cells
        for placement in cell["source_placements"]
    ]
    definitions = list(fixture["definitions"].values())
    return {
        "max_source_cell_encoded_bytes": max(len(canonical_json(cell)) for cell in cells),
        "max_source_placement_encoded_bytes": max(
            len(canonical_json(placement)) for placement in placements
        ),
        "max_source_definition_encoded_bytes": max(
            len(canonical_json(definition)) for definition in definitions
        ),
    }


def _d3_changed_artifact_metrics(before: Path, after: Path) -> tuple[list[str], int]:
    left = artifact_files(before)
    right = artifact_files(after)
    changed = [
        name for name in sorted(set(left) | set(right))
        if left.get(name) != right.get(name)
    ]
    return changed, sum(len(right.get(name, b"")) for name in changed)


def _d3_mutated_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    changed = d3_normalize_fixture(fixture)
    candidates = [
        placement
        for cell in changed["cells"]
        for placement in cell["source_placements"]
    ]
    if not candidates:
        raise SpikeError("D3 update probe requires one source placement")
    placement = candidates[len(candidates) // 2]
    source_role = placement.get("source_role")
    if not isinstance(source_role, str) or not source_role:
        raise SpikeError("D3 update probe requires a source role")
    placement["source_role"] = source_role + "|D3_UPDATE_PROBE"
    return changed

def _d3_corruption_rejected(
    carrier: Path,
    target_key: tuple[int, int, int],
    root: Path,
    *,
    expected_source_profile: str,
    mode: str,
) -> bool:
    damaged = root / f"negative-{mode}"
    _copy_artifact(carrier, damaged)
    manifest = read_d3_manifest(damaged, expected_source_profile=expected_source_profile)
    entry = _d3_entry(manifest, target_key)
    target = damaged / entry["path"]
    raw = target.read_bytes()
    if mode == "corrupt":
        value = bytearray(raw)
        value[-1] ^= 1
        target.write_bytes(value)
    elif mode == "truncate":
        target.write_bytes(raw[:-1])
    else:
        raise SpikeError("unknown D3 negative mode")
    try:
        read_d3_chunk(damaged, target_key, expected_source_profile=expected_source_profile)
    except SpikeError:
        return True
    return False


def _d3_manifest_negative(
    carrier: Path,
    root: Path,
    *,
    expected_source_profile: str,
    mutation: str,
) -> bool:
    damaged = root / f"manifest-negative-{mutation}"
    _copy_artifact(carrier, damaged)
    manifest_path = damaged / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    target_key: tuple[int, int, int] | None = None

    if mutation == "profile":
        manifest["provenance"]["source_generation_profile_id"] = "wrong-profile"
    elif mutation == "version":
        manifest["d3_carrier_schema"] = D3_CARRIER_SCHEMA + 1
    elif mutation == "critical":
        manifest["critical_features"] = sorted(set(manifest["critical_features"]) | {"unknown-critical"})
    elif mutation == "placement-index":
        first = next(iter(manifest["placement_index"]))
        manifest["placement_index"][first] = ["bad"]
    elif mutation == "raw-size":
        first_entry = manifest["chunks"][0]
        first_entry["raw_bytes"] = MAX_CHUNK_RAW_BYTES + 1
        target_key = tuple(first_entry["key"])
    else:
        raise SpikeError("unknown D3 manifest negative")
    manifest_path.write_bytes(canonical_json(manifest) + b"\n")
    try:
        if target_key is None:
            read_d3_manifest(damaged, expected_source_profile=expected_source_profile)
        else:
            read_d3_chunk(
                damaged,
                target_key,
                expected_source_profile=expected_source_profile,
            )
    except SpikeError:
        return True
    return False


def _d3_carrier_stats(path: Path, manifest: dict[str, Any]) -> dict[str, Any]:
    stored = sum(int(row["stored_bytes"]) for row in manifest["chunks"])
    raw = sum(int(row["raw_bytes"]) for row in manifest["chunks"])
    logical_index = canonical_json({
        "chunks": [
            {"key": row["key"], "path": row["path"], "raw_bytes": row["raw_bytes"], "raw_sha256": row["raw_sha256"]}
            for row in manifest["chunks"]
        ],
        "placement_index": manifest["placement_index"],
        "definition_keys": sorted(manifest["definitions"]),
    })
    return {
        "artifact_bytes": artifact_size(path),
        "manifest_bytes": (path / "manifest.json").stat().st_size,
        "logical_index_bytes": len(logical_index),
        "stored_chunk_bytes": stored,
        "raw_chunk_bytes": raw,
        "compression_ratio_raw_to_stored": round(raw / stored, 6) if stored else None,
        "chunk_count": len(manifest["chunks"]),
        "max_stored_chunk_bytes": max(int(row["stored_bytes"]) for row in manifest["chunks"]),
        "max_raw_chunk_bytes": max(int(row["raw_bytes"]) for row in manifest["chunks"]),
    }

def _measure_d3_carrier(
    root: Path,
    fixture: dict[str, Any],
    *,
    compression: str,
    expected_source_profile: str,
    load_iterations: int,
) -> dict[str, Any]:
    label = "indexed-uncompressed-baseline" if compression == "none" else "indexed-zlib"
    carrier_root = root / label
    server_a = carrier_root / "server-a"
    server_b = carrier_root / "server-b"
    client = carrier_root / "client"
    mutated = carrier_root / "mutated"
    _value, build_ms, build_peak = _timed(
        lambda: write_d3_carrier(
            server_a,
            fixture,
            D3_MEASUREMENT_CHUNK_SIZE,
            compression=compression,
            projection="server",
        )
    )
    write_d3_carrier(
        server_b,
        fixture,
        D3_MEASUREMENT_CHUNK_SIZE,
        compression=compression,
        projection="server",
    )
    write_d3_carrier(
        client,
        d3_client_fixture(fixture),
        D3_MEASUREMENT_CHUNK_SIZE,
        compression=compression,
        projection="client",
    )
    server_manifest = read_d3_manifest(
        server_a, expected_source_profile=expected_source_profile
    )
    client_manifest = read_d3_manifest(
        client, expected_source_profile=expected_source_profile
    )

    normalized = d3_normalize_fixture(fixture)
    server_roundtrip = reconstruct_d3_fixture(
        server_a, expected_source_profile=expected_source_profile
    )
    client_roundtrip = reconstruct_d3_fixture(
        client, expected_source_profile=expected_source_profile
    )
    if canonical_json(server_roundtrip) != canonical_json(normalized):
        raise SpikeError("D3 server carrier semantic round-trip mismatch")
    expected_client = d3_client_fixture(normalized)
    if canonical_json(client_roundtrip) != canonical_json(expected_client):
        raise SpikeError("D3 client carrier semantic round-trip mismatch")

    cells = normalized["cells"]
    target_cell = cells[len(cells) // 2]
    position = (target_cell["x"], target_cell["y"], target_cell["z"])
    if not target_cell["source_placements"]:
        target_cell = next(
            (cell for cell in cells if cell["source_placements"]), None
        )
        if target_cell is None:
            raise SpikeError("D3 random-access probe requires one source placement")
        position = (target_cell["x"], target_cell["y"], target_cell["z"])
    occurrence = target_cell["source_placements"][0]["source_occurrence_ref"]
    definition_key = sorted(normalized["definitions"])[0]

    cell_ms, cell_peak = _load_stats(
        lambda: read_d3_cell(
            server_a, position, expected_source_profile=expected_source_profile
        ),
        load_iterations,
    )
    placement_ms, placement_peak = _load_stats(
        lambda: read_d3_placement(
            server_a, occurrence, expected_source_profile=expected_source_profile
        ),
        load_iterations,
    )

    definition_ms, definition_peak = _load_stats(
        lambda: read_d3_definition(
            server_a, definition_key, expected_source_profile=expected_source_profile
        ),
        load_iterations,
    )
    changed_fixture = _d3_mutated_fixture(normalized)
    write_d3_carrier(
        mutated,
        changed_fixture,
        D3_MEASUREMENT_CHUNK_SIZE,
        compression=compression,
        projection="server",
    )
    changed_names, changed_bytes = _d3_changed_artifact_metrics(server_a, mutated)
    target_key = (
        position[0] // D3_MEASUREMENT_CHUNK_SIZE,
        position[1] // D3_MEASUREMENT_CHUNK_SIZE,
        position[2],
    )
    stats = _d3_carrier_stats(server_a, server_manifest)
    aggregate_placements = sum(
        len(cell["source_placements"]) for cell in normalized["cells"]
    )
    result = {
        "carrier": label,
        "compression": compression,
        "compression_configuration": "none" if compression == "none" else "zlib level 6",
        "chunk_size": D3_MEASUREMENT_CHUNK_SIZE,
        **stats,
        "artifact_sha256": artifact_digest(server_a),
        "client_artifact_bytes": artifact_size(client),
        "client_artifact_sha256": artifact_digest(client),
        "logical_identity_sha256": server_manifest["logical_identity_sha256"],
        "logical_index_signature": d3_index_signature(server_manifest),
        "client_logical_index_signature": d3_index_signature(client_manifest),
        "server_client_logical_index_equal": (
            d3_index_signature(server_manifest) == d3_index_signature(client_manifest)
        ),
        "deterministic_repeat_exact_bytes": artifact_digest(server_a) == artifact_digest(server_b),
        "source_model_encode_decode_equivalent": True,
        "client_projection_equivalent": True,
        "client_server_only_absent": "server_only" not in client_manifest,
        "build_ms": round(build_ms, 3),
        "build_peak_bytes": build_peak,
        "median_cell_load_ms": round(cell_ms, 3),
        "cell_load_peak_bytes": cell_peak,
        "median_placement_load_ms": round(placement_ms, 3),
        "placement_load_peak_bytes": placement_peak,
        "median_definition_load_ms": round(definition_ms, 3),
        "definition_load_peak_bytes": definition_peak,

        "random_access_probe": {
            "cell": list(position),
            "source_occurrence_ref": occurrence,
            "source_definition_key": definition_key,
            "without_full_bundle_interpretation": True,
        },
        "changed_storage_units_after_one_source_placement_edit": changed_names,
        "changed_storage_unit_count": len(changed_names),
        "changed_manifest_index_units": int("manifest.json" in changed_names),
        "rebuilt_or_patch_bytes_after_one_source_placement_edit": changed_bytes,
        "corruption_rejected": _d3_corruption_rejected(
            server_a,
            target_key,
            carrier_root,
            expected_source_profile=expected_source_profile,
            mode="corrupt",
        ),
        "truncation_rejected": _d3_corruption_rejected(
            server_a,
            target_key,
            carrier_root,
            expected_source_profile=expected_source_profile,
            mode="truncate",
        ),
        "decoded_field_count": _decoded_field_count(normalized),
        "cell_count": len(cells),
        "source_definition_count": len(normalized["definitions"]),
        "aggregate_source_placements": aggregate_placements,
        "max_source_placements_per_cell": max(
            len(cell["source_placements"]) for cell in cells
        ),
    }
    if not all(
        (
            result["deterministic_repeat_exact_bytes"],
            result["source_model_encode_decode_equivalent"],
            result["client_projection_equivalent"],
            result["client_server_only_absent"],
            result["corruption_rejected"],
            result["truncation_rejected"],
        )
    ):
        raise SpikeError(f"D3 carrier qualification failed: {label}")
    return result

def _d3_ratio_negative() -> bool:
    raw = b"A" * 4096
    compressed = zlib.compress(raw, 9)
    try:
        bounded_decompress(
            compressed,
            len(raw),
            max_raw_size=len(raw),
            max_ratio=2.0,
        )
    except SpikeError:
        return True
    return False


def run_d3_real_batch(
    *,
    game_root: Path,
    legacy_root: Path,
    map_path: Path,
    asset_zip: Path,
    assets_dir: Path,
    source_generation_profile_id: str,
    measurement_head: str,
    work_dir: Path,
    load_iterations: int,
) -> dict[str, Any]:
    if source_generation_profile_id != D3_FRESH_SOURCE_PROFILE:
        raise SpikeError("D3 requires the exact admitted fresh source-generation profile")
    if load_iterations < 1:
        raise SpikeError("D3 load iterations must be positive")
    code_provenance = _verify_d3_readonly_code(game_root)
    if code_provenance["game_head"] != measurement_head:
        raise SpikeError("D3 measurement head does not match checked-out Game head")

    producer = _load_module(
        "d3_exact_fullworld_producer",
        game_root / "tools/game-atlas-fullworld-source/producer.py",
    )
    batch_module = _load_module(
        "d3_exact_content_source_batch",
        game_root / "tools/reference-world-corridor-census/content_source_batch.py",
    )
    profile = producer.source_generation_profile(source_generation_profile_id)
    runtime = producer.load_runtime(
        legacy_root=legacy_root,
        map_path=map_path,
        asset_zip=asset_zip,
        assets_dir=assets_dir,
        source_generation_profile_id=source_generation_profile_id,
    )

    tile_records, selection_evidence = _collect_d3_tiles(producer, runtime)
    source_summary = _d3_source_summary(producer, profile, code_provenance)
    typed_batch = batch_module.build_content_source_batch(
        producer=producer,
        tile_records=tile_records,
        bindings={},
        phase_a_summary=source_summary,
        adapter_revision=measurement_head,
    )
    typed_bytes = batch_module.canonical_batch_bytes(typed_batch)
    source_profile = {
        "source_generation_profile_id": profile.profile_id,
        "source_generation_profile_revision": profile.revision,
        "source_repository": profile.source_repository,
        "source_repository_sha": profile.source_repository_sha,
        "world_otbm_sha256": profile.world_otbm_sha256,
        "world_otbm_git_blob": profile.world_otbm_git_blob,
        "world_otbm_bytes": profile.world_otbm_bytes,
        "asset_zip_sha256": profile.asset_zip_sha256,
        "asset_catalog_sha256": profile.asset_catalog_sha256,
        "asset_appearance_sha256": profile.asset_appearance_sha256,
        "parser_repository": profile.parser_repository,
        "parser_repository_sha": profile.parser_repository_sha,
        "game_measurement_head": measurement_head,
        "game_readonly_code": code_provenance["files"],
    }
    fixture = d3_fixture_from_typed_batch(
        typed_batch,
        source_profile=source_profile,
        selection_evidence=selection_evidence,
        typed_batch_sha256=sha256(typed_bytes),
        typed_batch_bytes=len(typed_bytes),
    )
    fixture_bytes = canonical_json(fixture)
    logical_identity = d3_logical_identity(fixture)

    reverse_batch = batch_module.build_content_source_batch(
        producer=producer,
        tile_records=list(reversed(tile_records)),
        bindings={},
        phase_a_summary=source_summary,
        adapter_revision=measurement_head,
    )
    reverse_bytes = batch_module.canonical_batch_bytes(reverse_batch)
    reverse_fixture = d3_fixture_from_typed_batch(
        reverse_batch,
        source_profile=source_profile,
        selection_evidence=selection_evidence,
        typed_batch_sha256=sha256(reverse_bytes),
        typed_batch_bytes=len(reverse_bytes),
    )

    enumeration_independent = (
        typed_bytes == reverse_bytes
        and canonical_json(fixture) == canonical_json(reverse_fixture)
        and logical_identity == d3_logical_identity(reverse_fixture)
    )
    if not enumeration_independent:
        raise SpikeError("D3 identity changed under source enumeration reorder")

    shard_variant = copy.deepcopy(fixture)
    selection = shard_variant["provenance"].get("selection")
    if not isinstance(selection, dict) or not isinstance(selection.get("windows"), list):
        raise SpikeError("D3 source selection lacks shard evidence")
    for window in selection["windows"]:
        window["retained_shard"] = f"repartition-probe:{window['name']}"
    source_shard_identity_stable = (
        d3_logical_identity(shard_variant) == logical_identity
    )
    if not source_shard_identity_stable:
        raise SpikeError("D3 logical identity changed under source shard metadata")

    if work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True)
    measurements_root = work_dir / "carriers"
    carriers = [
        _measure_d3_carrier(
            measurements_root,
            fixture,
            compression=compression,
            expected_source_profile=source_generation_profile_id,
            load_iterations=load_iterations,
        )
        for compression in ("none", "zlib")
    ]
    if carriers[0]["logical_index_signature"] != carriers[1]["logical_index_signature"]:
        raise SpikeError("D3 carrier logical index differs by compression")

    rechunk = work_dir / "rechunk-identity"
    write_d3_carrier(
        rechunk,
        fixture,
        16,
        compression="none",
        projection="server",
    )
    rechunk_manifest = read_d3_manifest(
        rechunk, expected_source_profile=source_generation_profile_id
    )
    rechunk_identity_stable = rechunk_manifest["logical_identity_sha256"] == logical_identity
    if not rechunk_identity_stable:
        raise SpikeError("D3 logical identity changed under rechunking")

    baseline_root = measurements_root / "indexed-uncompressed-baseline" / "server-a"
    negatives = {
        "wrong_source_profile_rejected": _d3_manifest_negative(
            baseline_root, work_dir,
            expected_source_profile=source_generation_profile_id, mutation="profile"
        ),
        "wrong_carrier_version_rejected": _d3_manifest_negative(
            baseline_root, work_dir,
            expected_source_profile=source_generation_profile_id, mutation="version"
        ),
        "unknown_critical_feature_rejected": _d3_manifest_negative(
            baseline_root, work_dir,
            expected_source_profile=source_generation_profile_id, mutation="critical"
        ),
        "malformed_placement_index_rejected": _d3_manifest_negative(
            baseline_root, work_dir,
            expected_source_profile=source_generation_profile_id, mutation="placement-index"
        ),
        "oversized_raw_chunk_rejected": _d3_manifest_negative(
            baseline_root, work_dir,
            expected_source_profile=source_generation_profile_id, mutation="raw-size"
        ),
        "decompression_ratio_rejected": _d3_ratio_negative(),
    }
    if not all(negatives.values()):
        raise SpikeError("D3 negative-boundary qualification failed")

    counts = typed_batch["counts"]
    return {
        "schema": "OTV2_CONTENT_WORLD_D3_REAL_BATCH_BUNDLE_MEASUREMENT/v1",
        "spike_invariant": INVARIANT,
        "measurement_head": measurement_head,
        "environment": {
            "python": platform.python_version(),
            "zlib": zlib.ZLIB_VERSION,
            "platform": platform.platform(),
        },
        "configuration": {
            "load_iterations": load_iterations,
            "measured_chunk_size": D3_MEASUREMENT_CHUNK_SIZE,
            "measured_chunk_size_is_production_maximum": False,
            "carrier_types": 2,
            "compressions": ["none", "zlib level 6"],
            "max_artifact_bytes": MAX_ARTIFACT_BYTES,
            "max_chunk_raw_bytes": MAX_CHUNK_RAW_BYTES,
            "max_decompression_ratio": MAX_DECOMPRESSION_RATIO,
        },

        "source_provenance": source_profile,
        "source_selection": selection_evidence,
        "typed_input": {
            "schema": typed_batch["schema"],
            "canonical_bytes": len(typed_bytes),
            "canonical_sha256": sha256(typed_bytes),
            "counts": counts,
            "deferred_requires_phase_b": typed_batch["deferred_requires_phase_b"],
            "production_authority": typed_batch["production_authority"],
            "reference_parity_claim": typed_batch["reference_parity_claim"],
        },
        "logical_input": {
            "canonical_bytes": len(fixture_bytes),
            "canonical_sha256": sha256(fixture_bytes),
            "logical_identity_sha256": logical_identity,
            "source_cell_count": len(fixture["cells"]),
            "source_definition_count": len(fixture["definitions"]),
            "aggregate_source_placements": sum(
                len(cell["source_placements"]) for cell in fixture["cells"]
            ),
            "max_source_placements_per_cell": max(
                len(cell["source_placements"]) for cell in fixture["cells"]
            ),
            "decoded_field_count": _decoded_field_count(fixture),
            **_d3_record_size_stats(fixture),
        },
        "determinism": {
            "source_enumeration_order_independent": enumeration_independent,
            "source_shard_identity_independent": source_shard_identity_stable,
            "rechunk_identity_independent": rechunk_identity_stable,
            "two_carriers_same_logical_index": (
                carriers[0]["logical_index_signature"]
                == carriers[1]["logical_index_signature"]
            ),
            "per_carrier_repeat_exact_bytes": all(
                row["deterministic_repeat_exact_bytes"] for row in carriers
            ),
        },
        "carriers": carriers,
        "negative_evidence": negatives,
        "classification": D3_SOURCE_CLASSIFICATION,
        "production_authority": "NONE",
        "reference_parity_claim": "NONE",
        "registry_maxima_selected": False,
        "owner_format_decision": "NOT_MADE",
    }

def render_d3_dossier(result: dict[str, Any], exact_head: str) -> str:
    source = result["source_provenance"]
    typed = result["typed_input"]
    logical = result["logical_input"]
    determinism = result["determinism"]
    lines = [
        "# Content/World D3 real-batch bundle measurement",
        "",
        f"- Exact Game measurement head: `{exact_head}`",
        f"- Source: `{source['source_repository']}@{source['source_repository_sha']}`",
        f"- Fresh source profile: `{source['source_generation_profile_id']}` revision `{source['source_generation_profile_revision']}`",
        f"- world.otbm SHA-256: `{source['world_otbm_sha256']}`",
        f"- Parser: `{source['parser_repository']}@{source['parser_repository_sha']}`",
        f"- Classification: **{result['classification']}**",
        f"- Spike invariant: **`{result['spike_invariant']}`**",
        "- Authority: measurement evidence only; production authority and Reference parity remain NONE.",
        "",
        "## Fresh real input",
        "",
    ]
    for window in result["source_selection"]["windows"]:
        lines.append(
            f"- {window['name']}: {window['tile_records']} source cells, "
            f"{window['source_occurrences']} source occurrences, "
            f"ordered stream SHA-256 `{window['ordered_stream_sha256']}`."
        )
    lines.extend([
        f"- Typed CW2/pre-promotion batch: {typed['canonical_bytes']} bytes, SHA-256 `{typed['canonical_sha256']}`.",
        f"- Typed counts: `{json.dumps(typed['counts'], sort_keys=True)}`.",
        f"- D3 normalized logical input: {logical['canonical_bytes']} bytes, SHA-256 `{logical['canonical_sha256']}`.",
        f"- Source cells: {logical['source_cell_count']}; source definitions: {logical['source_definition_count']}; source placements: {logical['aggregate_source_placements']}; max source placements/cell: {logical['max_source_placements_per_cell']}.",
        f"- Encoded record maxima: cell {logical['max_source_cell_encoded_bytes']} B; placement {logical['max_source_placement_encoded_bytes']} B; source definition {logical['max_source_definition_encoded_bytes']} B.",
        "",
        "No canonical target identity, target coordinates, collision, order or footprint truth is inferred by this measurement. "
        "When CW2 has no accepted SourceIdentityBinding, the carrier records the real source occurrence as unresolved and the appearance ID as source provenance only.",
        "",
        "## Two measured runtime carriers",
        "",
        "| Carrier | Compression | Artifact B | Raw chunk B | Stored chunk B | Ratio raw/stored | Build ms | Cell ms | Placement ms | Definition ms | Build peak B | Load peak B | Patch B | Changed units |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ])

    for row in result["carriers"]:
        load_peak = max(
            row["cell_load_peak_bytes"],
            row["placement_load_peak_bytes"],
            row["definition_load_peak_bytes"],
        )
        lines.append(
            f"| `{row['carrier']}` | {row['compression_configuration']} | {row['artifact_bytes']} | "
            f"{row['raw_chunk_bytes']} | {row['stored_chunk_bytes']} | {row['compression_ratio_raw_to_stored']} | "
            f"{row['build_ms']:.3f} | {row['median_cell_load_ms']:.3f} | "
            f"{row['median_placement_load_ms']:.3f} | {row['median_definition_load_ms']:.3f} | "
            f"{row['build_peak_bytes']} | {load_peak} | "
            f"{row['rebuilt_or_patch_bytes_after_one_source_placement_edit']} | {row['changed_storage_unit_count']} |"
        )
    lines.extend([
        "",
        "Both carrier types use the same logical/index structure. The second changes only bounded payload compression to zlib level 6.",
        "",
        "## Determinism and locality",
        "",
        f"- Source enumeration reorder preserves the typed/logical batch: **{'PASS' if determinism['source_enumeration_order_independent'] else 'FAIL'}**.",
        f"- Source-shard metadata does not alter logical identity: **{'PASS' if determinism['source_shard_identity_independent'] else 'FAIL'}**.",
        f"- Rechunking preserves logical identity: **{'PASS' if determinism['rechunk_identity_independent'] else 'FAIL'}**.",
        f"- Both physical carriers have the same logical index signature: **{'PASS' if determinism['two_carriers_same_logical_index'] else 'FAIL'}**.",
        f"- Independent repeated builds are byte-identical per carrier: **{'PASS' if determinism['per_carrier_repeat_exact_bytes'] else 'FAIL'}**.",
        "- Random-access probes resolve one source cell, one source occurrence and one source-definition reference by index without decoding every chunk.",
        "- The one-record update probe mutates one source-role field only in scratch output; it measures physical rebuild locality and is not claimed as source or gameplay truth.",
        "",
        "## Fail-closed evidence",
        "",
    ])
    for key, passed in sorted(result["negative_evidence"].items()):
        lines.append(f"- {key}: **{'PASS' if passed else 'FAIL'}**")
    for row in result["carriers"]:
        lines.append(
            f"- {row['carrier']}: corruption={'PASS' if row['corruption_rejected'] else 'FAIL'}, "
            f"truncation={'PASS' if row['truncation_rejected'] else 'FAIL'}, "
            f"server/client allowlist={'PASS' if row['client_projection_equivalent'] and row['client_server_only_absent'] else 'FAIL'}."
        )

    lines.extend([
        "",
        "## Boundary / disposition",
        "",
        "- The measured 32-cell chunk dimension is a bounded evidence configuration, not a production hard maximum.",
        "- The 64 MiB spike artifact fence and 2 MiB raw-chunk fence are harness safety limits, not selected production resource maxima.",
        "- zlib is the single mature compression candidate measured here; no serializer/compressor zoo was introduced.",
        "- No Phase-B target parity was performed. All target-sensitive facts remain `DEFERRED_REQUIRES_PHASE_B`.",
        "- No production registry, contract, runtime, client, CW2/CW3 implementation or source-profile file is changed.",
        "- `SPIKE_RESULT != OWNER_FORMAT_DECISION`: this result does not select the permanent World Project/World Bundle format.",
        "",
        "## Result",
        "",
        "D3 now has reproducible physical-layout/resource evidence for the fresh exact CrystalServer source generation. "
        "The evidence can inform a later owner/control-plane format decision, but it does not itself make that decision or grant CW4/production authority.",
        "",
    ])
    return "\n".join(lines)

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
    parser.add_argument("--d3-real-batch", action="store_true")
    parser.add_argument("--game-root", type=Path)
    parser.add_argument("--legacy-root", type=Path)
    parser.add_argument("--map", dest="map_path", type=Path)
    parser.add_argument("--asset-zip", type=Path)
    parser.add_argument("--assets", type=Path)
    parser.add_argument("--source-generation-profile-id")

    args = parser.parse_args()
    if args.dossier is not None and not args.base_sha:
        parser.error("--base-sha is required when --dossier is supplied")

    if args.d3_real_batch:
        if not args.base_sha:
            parser.error("--base-sha is required for --d3-real-batch")
        required = {
            "--game-root": args.game_root,
            "--legacy-root": args.legacy_root,
            "--map": args.map_path,
            "--asset-zip": args.asset_zip,
            "--assets": args.assets,
            "--source-generation-profile-id": args.source_generation_profile_id,
        }
        missing = [name for name, value in required.items() if value is None]
        if missing:
            parser.error("D3 real-batch mode requires " + ", ".join(missing))
        result = run_d3_real_batch(
            game_root=args.game_root,
            legacy_root=args.legacy_root,
            map_path=args.map_path,
            asset_zip=args.asset_zip,
            assets_dir=args.assets,
            source_generation_profile_id=args.source_generation_profile_id,
            measurement_head=args.base_sha,
            work_dir=args.work_dir,
            load_iterations=args.iterations,
        )
        write_results(args.results, result)
        if args.dossier is not None:
            dossier = render_d3_dossier(result, exact_head=args.base_sha)
            atomic_write(
                args.dossier, dossier.rstrip("\n").encode("utf-8") + b"\n"
            )
        print(INVARIANT)
        print("carrier_types=2")
        print(f"results={args.results}")
        if args.dossier is not None:
            print(f"dossier={args.dossier}")
        return 0

    result = run_benchmarks(args.work_dir, default_scales(), args.iterations)
    write_results(args.results, result)
    if args.dossier is not None:
        dossier = render_dossier(result, exact_base_sha=args.base_sha)
        atomic_write(
            args.dossier, dossier.rstrip("\n").encode("utf-8") + b"\n"
        )
    print(f"{INVARIANT}")
    print(f"measurements={len(result['measurements'])}")
    print(f"results={args.results}")
    if args.dossier is not None:
        print(f"dossier={args.dossier}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
