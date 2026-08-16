#!/usr/bin/env python3
"""Non-canonical Game -> Atlas physical-profile research spike.

Stdlib-only, deterministic, proprietary-data-free evidence tooling.
It is not a public schema, production parser, or runtime implementation.
"""
from __future__ import annotations

import argparse
import difflib
import gzip
import hashlib
import json
import math
import struct
import zlib
from collections import defaultdict

SCHEMA = "oteryn-game-atlas-physical-readiness-spike-v1"
ENCODINGS = ("canonical-json-v0", "canonical-jsonl-v0", "binary-baseline-v0")
CHUNKS = (32, 64)
PACKINGS = ("per-floor", "packed-floors")
MAX_RECORDS = 200_000
MAX_CHUNK_BYTES = 64 * 1024 * 1024
MAX_STRING_BYTES = 1024
MAX_OBJECTS_PER_RECORD = 256
RECORD_KEYS = {"floor", "ground", "objects", "poi", "x", "y"}
GROUNDS = (
    "oteryn:item.terrain.grass",
    "oteryn:item.terrain.stone",
    "oteryn:item.terrain.sand",
    "oteryn:item.terrain.water",
)
OBJECTS = (
    "oteryn:item.environment.tree",
    "oteryn:item.environment.rock",
    "oteryn:item.environment.flower",
    "oteryn:item.environment.lantern",
)


def cjson(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def gz(data):
    return gzip.compress(data, compresslevel=6, mtime=0)


def bounded_chunk(raw):
    if len(raw) > MAX_CHUNK_BYTES:
        raise ValueError("research chunk byte cap exceeded")
    return raw


def require_record_count(count):
    if count > MAX_RECORDS:
        raise ValueError("research record count cap exceeded")


def require_string(value, label):
    if not isinstance(value, str):
        raise ValueError(f"{label} must be a string")
    if len(value.encode("utf-8")) > MAX_STRING_BYTES:
        raise ValueError(f"{label} exceeds research string cap")


def validate_record(record):
    if not isinstance(record, dict) or set(record) != RECORD_KEYS:
        raise ValueError("invalid research record shape")
    for field in ("x", "y", "floor"):
        if type(record[field]) is not int:
            raise ValueError(f"{field} must be an integer")
    require_string(record["ground"], "ground")
    objects = record["objects"]
    if not isinstance(objects, list) or len(objects) > MAX_OBJECTS_PER_RECORD:
        raise ValueError("objects exceed research object-count cap or are not a list")
    for obj in objects:
        require_string(obj, "object")
    if record["poi"] is not None:
        require_string(record["poi"], "poi")


def validate_records(records):
    if not isinstance(records, list):
        raise ValueError("research records must be a list")
    require_record_count(len(records))
    for record in records:
        validate_record(record)


def make_records(name, width=128, height=128, floors=6):
    total = width * height * floors
    require_record_count(total)
    dense = name.endswith("dense-v1")
    records = []
    for floor in range(floors):
        for y in range(height):
            for x in range(width):
                selector = (x * 31 + y * 17 + floor * 13) % 97
                objects = []
                if dense:
                    objects.append(OBJECTS[selector % len(OBJECTS)])
                    if selector % 3 == 0:
                        objects.append(OBJECTS[(selector + 1) % len(OBJECTS)])
                    if selector % 11 == 0:
                        objects.append("oteryn:item.environment.decorative_banner")
                else:
                    if selector % 7 == 0:
                        objects.append(OBJECTS[selector % len(OBJECTS)])
                    if selector % 29 == 0:
                        objects.append("oteryn:item.environment.decorative_banner")
                poi = None
                if (x * 7 + y * 5 + floor * 3) % 509 == 0:
                    poi = f"synthetic:poi:{floor}:{y:03d}:{x:03d}"
                records.append(
                    {
                        "floor": floor,
                        "ground": GROUNDS[(x // 8 + y // 8 + floor) % len(GROUNDS)],
                        "objects": objects,
                        "poi": poi,
                        "x": x,
                        "y": y,
                    }
                )
    validate_records(records)
    return records


def semantic_sha(records):
    validate_records(records)
    ordered = sorted(records, key=lambda r: (r["floor"], r["y"], r["x"]))
    return sha(cjson(ordered))


def group_records(records, chunk, packing):
    groups = defaultdict(list)
    for record in records:
        key = (
            record["x"] // chunk,
            record["y"] // chunk,
            record["floor"] if packing == "per-floor" else None,
        )
        groups[key].append(record)
    for values in groups.values():
        values.sort(key=lambda r: (r["floor"], r["y"], r["x"]))
    return dict(
        sorted(
            groups.items(),
            key=lambda pair: (
                -1 if pair[0][2] is None else pair[0][2],
                pair[0][1],
                pair[0][0],
            ),
        )
    )


def enc_string(value):
    require_string(value, "binary string")
    raw = value.encode("utf-8")
    if len(raw) > 0xFFFF:
        raise ValueError("binary string exceeds uint16 length")
    return struct.pack("<H", len(raw)) + raw


def dec_string(view, offset):
    if offset + 2 > len(view):
        raise ValueError("truncated string length")
    (length,) = struct.unpack_from("<H", view, offset)
    offset += 2
    if length > MAX_STRING_BYTES or offset + length > len(view):
        raise ValueError("invalid research string length")
    return bytes(view[offset : offset + length]).decode("utf-8"), offset + length


def enc_binary(records):
    validate_records(records)
    out = bytearray(b"GASB1\0")
    out += struct.pack("<I", len(records))
    for record in records:
        objects = record["objects"]
        out += struct.pack("<iihH", record["x"], record["y"], record["floor"], len(objects))
        out += enc_string(record["ground"])
        for obj in objects:
            out += enc_string(obj)
        out += b"\x00" if record["poi"] is None else b"\x01" + enc_string(record["poi"])
    return bounded_chunk(bytes(out))


def dec_binary(raw):
    view = memoryview(bounded_chunk(raw))
    if len(view) < 10 or bytes(view[:6]) != b"GASB1\0":
        raise ValueError("bad research binary header")
    (count,) = struct.unpack_from("<I", view, 6)
    require_record_count(count)
    offset = 10
    out = []
    for _ in range(count):
        if offset + 12 > len(view):
            raise ValueError("truncated research binary record")
        x, y, floor, object_count = struct.unpack_from("<iihH", view, offset)
        offset += 12
        if object_count > MAX_OBJECTS_PER_RECORD:
            raise ValueError("binary object count exceeds research cap")
        ground, offset = dec_string(view, offset)
        objects = []
        for _ in range(object_count):
            obj, offset = dec_string(view, offset)
            objects.append(obj)
        if offset >= len(view):
            raise ValueError("truncated poi flag")
        flag = view[offset]
        offset += 1
        if flag not in (0, 1):
            raise ValueError("invalid poi flag")
        poi = None
        if flag:
            poi, offset = dec_string(view, offset)
        out.append({"floor": floor, "ground": ground, "objects": objects, "poi": poi, "x": x, "y": y})
    if offset != len(view):
        raise ValueError("trailing research binary bytes")
    validate_records(out)
    return out


def encode(kind, records):
    validate_records(records)
    if kind == "canonical-json-v0":
        return bounded_chunk(cjson({"records": records, "spike_schema": SCHEMA}))
    if kind == "canonical-jsonl-v0":
        lines = [cjson({"spike_schema": SCHEMA, "type": "header"})]
        lines.extend(cjson(record) for record in records)
        return bounded_chunk(b"\n".join(lines) + b"\n")
    if kind == "binary-baseline-v0":
        return enc_binary(records)
    raise ValueError(kind)


def decode(kind, raw):
    raw = bounded_chunk(raw)
    if kind == "canonical-json-v0":
        value = json.loads(raw)
        if value.get("spike_schema") != SCHEMA or not isinstance(value.get("records"), list):
            raise ValueError("invalid research json")
        validate_records(value["records"])
        return value["records"]
    if kind == "canonical-jsonl-v0":
        lines = raw.splitlines()
        if not lines or json.loads(lines[0]) != {"spike_schema": SCHEMA, "type": "header"}:
            raise ValueError("invalid research jsonl")
        records = [json.loads(line) for line in lines[1:]]
        validate_records(records)
        return records
    if kind == "binary-baseline-v0":
        return dec_binary(raw)
    raise ValueError(kind)


def file_name(key, kind):
    cx, cy, floor = key
    ext = {"canonical-json-v0": "json", "canonical-jsonl-v0": "jsonl", "binary-baseline-v0": "bin"}[kind]
    floor_part = "all" if floor is None else f"f{floor:02d}"
    return f"chunks/{floor_part}/{cy:03d}/{cx:03d}.{ext}"


def package(fixture, records, kind, chunk, packing):
    chunks, files = [], {}
    for key, grouped in group_records(records, chunk, packing).items():
        raw = encode(kind, grouped)
        compressed = gz(raw)
        path = file_name(key, kind)
        files[path] = raw
        chunks.append(
            {
                "compressed_bytes": len(compressed),
                "key": list(key),
                "path": path,
                "raw_bytes": len(raw),
                "raw_sha256": sha(raw),
                "record_count": len(grouped),
            }
        )
    manifest = {
        "chunk_size": chunk,
        "chunks": chunks,
        "encoding": kind,
        "fixture": fixture,
        "floor_packing": packing,
        "semantic_sha256": semantic_sha(records),
        "spike_schema": SCHEMA,
    }
    files["manifest.json"] = bounded_chunk(cjson(manifest))
    return files, manifest


def mutate(records, width, height, floors):
    target = (width // 2, height // 2, min(3, floors - 1))
    out, seen = [], False
    for record in records:
        copy = dict(record)
        copy["objects"] = list(record["objects"])
        if (record["x"], record["y"], record["floor"]) == target:
            copy["objects"].append("oteryn:item.environment.spike_mutation_marker")
            seen = True
        out.append(copy)
    if not seen:
        raise AssertionError("mutation target missing")
    validate_records(out)
    return out


def p95(values):
    ordered = sorted(values)
    return 0 if not ordered else ordered[max(0, math.ceil(len(ordered) * 0.95) - 1)]


def paths_for(manifest, x0, x1, y0, y1, floor):
    size, result = manifest["chunk_size"], []
    for entry in manifest["chunks"]:
        cx, cy, packed_floor = entry["key"]
        intersects = cx * size < x1 and (cx + 1) * size > x0 and cy * size < y1 and (cy + 1) * size > y0
        if intersects and (packed_floor is None or packed_floor == floor):
            result.append(entry["path"])
    return sorted(result)


def text_diff_lines(before, after):
    left, right = before.decode("utf-8").splitlines(), after.decode("utf-8").splitlines()
    diff = difflib.unified_diff(left, right, n=1)
    return sum(
        1
        for line in diff
        if line[:1] in ("+", "-") and not line.startswith(("+++", "---"))
    )


def max_text_line_bytes(raw):
    return max((len(line) for line in raw.splitlines()), default=0)


def evaluate(fixture, records, kind, chunk, packing, width, height, floors):
    files1, manifest1 = package(fixture, records, kind, chunk, packing)
    files2, manifest2 = package(fixture, records, kind, chunk, packing)
    deterministic = files1 == files2 and manifest1 == manifest2

    decoded = []
    for entry in manifest1["chunks"]:
        decoded.extend(decode(kind, files1[entry["path"]]))
    roundtrip = semantic_sha(decoded) == manifest1["semantic_sha256"]

    mutated = mutate(records, width, height, floors)
    files3, _ = package(fixture, mutated, kind, chunk, packing)
    changed_package = [path for path in files1 if files1[path] != files3[path]]
    changed_data = [path for path in changed_package if path != "manifest.json"]
    if kind == "binary-baseline-v0":
        diff_lines = None
        text_max_line_bytes = None
    else:
        diff_lines = sum(text_diff_lines(files1[path], files3[path]) for path in changed_data)
        text_max_line_bytes = max(max_text_line_bytes(files1[path]) for path in changed_data)

    first = manifest1["chunks"][0]
    raw = files1[first["path"]]
    corrupt = bytearray(raw)
    corrupt[len(corrupt) // 2] ^= 1
    digest_detects = sha(bytes(corrupt)) != first["raw_sha256"]
    corrupt_gzip = bytearray(gz(raw))
    corrupt_gzip[len(corrupt_gzip) // 2] ^= 1
    gzip_detects = False
    try:
        gzip.decompress(bytes(corrupt_gzip))
    except (OSError, EOFError, zlib.error):
        gzip_detects = True

    by_path = {entry["path"]: entry for entry in manifest1["chunks"]}
    floor = min(3, floors - 1)
    viewport = paths_for(
        manifest1,
        width // 8,
        width // 8 + min(96, width // 2),
        height // 8,
        height // 8 + min(96, height // 2),
        floor,
    )
    point = paths_for(manifest1, width // 2, width // 2 + 1, height // 2, height // 2 + 1, floor)
    raw_sizes = [entry["raw_bytes"] for entry in manifest1["chunks"]]
    gzip_sizes = [len(gz(files1[entry["path"]])) for entry in manifest1["chunks"]]

    return {
        "chunk_count": len(raw_sizes),
        "chunk_size": chunk,
        "corruption_digest_detected": digest_detects,
        "deterministic_bytes": deterministic,
        "encoding": kind,
        "fixture": fixture,
        "floor_packing": packing,
        "gzip_corruption_detected": gzip_detects,
        "gzip_total_bytes": sum(gzip_sizes) + len(gz(files1["manifest.json"])),
        "local_edit_changed_data_files": len(changed_data),
        "local_edit_changed_package_files": len(changed_package),
        "local_edit_changed_gzip_bytes": sum(len(gz(files3[path])) for path in changed_data),
        "local_edit_text_diff_lines": diff_lines,
        "local_edit_text_max_line_bytes": text_max_line_bytes,
        "max_chunk_gzip_bytes": max(gzip_sizes),
        "max_chunk_raw_bytes": max(raw_sizes),
        "p95_chunk_gzip_bytes": p95(gzip_sizes),
        "p95_chunk_raw_bytes": p95(raw_sizes),
        "point_access_chunk_files": len(point),
        "point_access_gzip_bytes": sum(by_path[path]["compressed_bytes"] for path in point),
        "raw_total_bytes": sum(raw_sizes) + len(files1["manifest.json"]),
        "roundtrip_semantic_identity": roundtrip,
        "semantic_sha256": manifest1["semantic_sha256"],
        "viewport_chunk_files": len(viewport),
        "viewport_gzip_bytes": sum(by_path[path]["compressed_bytes"] for path in viewport),
    }


def raises_value_error(call):
    try:
        call()
    except ValueError:
        return True
    return False


def cap_negative_checks():
    base = make_records("synthetic-sparse-v1", 1, 1, 1)[0]
    long_record = dict(base)
    long_record["objects"] = list(base["objects"])
    long_record["ground"] = "x" * (MAX_STRING_BYTES + 1)
    many_objects = dict(base)
    many_objects["objects"] = ["x"] * (MAX_OBJECTS_PER_RECORD + 1)
    checks = []
    for kind in ENCODINGS:
        checks.append(raises_value_error(lambda kind=kind: encode(kind, [long_record])))
        checks.append(raises_value_error(lambda kind=kind: encode(kind, [many_objects])))
    bad_json = cjson({"records": [long_record], "spike_schema": SCHEMA})
    bad_jsonl = cjson({"spike_schema": SCHEMA, "type": "header"}) + b"\n" + cjson(long_record) + b"\n"
    checks.append(raises_value_error(lambda: decode("canonical-json-v0", bad_json)))
    checks.append(raises_value_error(lambda: decode("canonical-jsonl-v0", bad_jsonl)))
    excessive_count = b"GASB1\0" + struct.pack("<I", MAX_RECORDS + 1)
    checks.append(raises_value_error(lambda: decode("binary-baseline-v0", excessive_count)))
    checks.append(raises_value_error(lambda: bounded_chunk(b"x" * (MAX_CHUNK_BYTES + 1))))
    return all(checks)


def report():
    width, height, floors = 128, 128, 6
    rows, fixtures = [], []
    for fixture in ("synthetic-sparse-v1", "synthetic-dense-v1"):
        records = make_records(fixture, width, height, floors)
        fixtures.append(
            {
                "name": fixture,
                "width": width,
                "height": height,
                "floors": floors,
                "record_count": len(records),
                "semantic_sha256": semantic_sha(records),
            }
        )
        for chunk in CHUNKS:
            for packing in PACKINGS:
                for kind in ENCODINGS:
                    rows.append(evaluate(fixture, records, kind, chunk, packing, width, height, floors))
    cap_checks = cap_negative_checks()
    matrix_checks = all(
        row["deterministic_bytes"]
        and row["roundtrip_semantic_identity"]
        and row["corruption_digest_detected"]
        and row["gzip_corruption_detected"]
        and row["local_edit_changed_data_files"] == 1
        and row["local_edit_changed_package_files"] == 2
        and row["point_access_chunk_files"] == 1
        and (
            row["encoding"] == "binary-baseline-v0"
            or (row["local_edit_text_diff_lines"] == 2 and row["local_edit_text_max_line_bytes"] > 0)
        )
        for row in rows
    )
    return {
        "spike_schema": SCHEMA,
        "research_only": True,
        "fixture_policy": "project-owned deterministic synthetic data only; no OTBM/Crystal/Canary/Tibia assets",
        "internal_spike_caps_not_production_limits": {
            "max_records": MAX_RECORDS,
            "max_chunk_bytes": MAX_CHUNK_BYTES,
            "max_string_bytes": MAX_STRING_BYTES,
            "max_objects_per_record": MAX_OBJECTS_PER_RECORD,
        },
        "cap_negative_checks_pass": cap_checks,
        "candidates": {
            "encodings": list(ENCODINGS),
            "chunk_sizes": list(CHUNKS),
            "floor_packing": list(PACKINGS),
        },
        "fixtures": fixtures,
        "rows": rows,
        "spike_checks_pass": matrix_checks and cap_checks,
        "warning": "Non-canonical evidence only. Does not define public schema, coordinate authority, production limits, compression, chunk geometry, or serializer.",
    }


def self_test(result):
    assert result["spike_checks_pass"]
    assert result["cap_negative_checks_pass"]
    assert len(result["rows"]) == 2 * len(ENCODINGS) * len(CHUNKS) * len(PACKINGS)
    for row in result["rows"]:
        expected = (16 if row["chunk_size"] == 32 else 4) * (6 if row["floor_packing"] == "per-floor" else 1)
        assert row["chunk_count"] == expected
        assert row["raw_total_bytes"] > 0 and row["gzip_total_bytes"] > 0
        assert row["local_edit_changed_data_files"] == 1
        assert row["local_edit_changed_package_files"] == 2
        if row["encoding"] == "binary-baseline-v0":
            assert row["local_edit_text_diff_lines"] is None
            assert row["local_edit_text_max_line_bytes"] is None
        else:
            assert row["local_edit_text_diff_lines"] == 2
            assert row["local_edit_text_max_line_bytes"] > 0


def print_summary(result):
    print("SPIKE_SUMMARY")
    print(f"schema={result['spike_schema']}")
    print(f"checks_pass={str(result['spike_checks_pass']).lower()}")
    print(f"cap_negative_checks_pass={str(result['cap_negative_checks_pass']).lower()}")
    print(f"rows={len(result['rows'])}")
    print(
        "fixture|encoding|chunk|packing|raw|gzip|max_gzip|point_gzip|"
        "viewport_gzip|edit_data|edit_package|diff_lines|max_text_line"
    )
    for row in result["rows"]:
        print(
            f"{row['fixture']}|{row['encoding']}|{row['chunk_size']}|{row['floor_packing']}|"
            f"{row['raw_total_bytes']}|{row['gzip_total_bytes']}|{row['max_chunk_gzip_bytes']}|"
            f"{row['point_access_gzip_bytes']}|{row['viewport_gzip_bytes']}|"
            f"{row['local_edit_changed_data_files']}|{row['local_edit_changed_package_files']}|"
            f"{row['local_edit_text_diff_lines']}|{row['local_edit_text_max_line_bytes']}"
        )
    print("SPIKE_SUMMARY_END")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--summary", action="store_true")
    args = parser.parse_args()
    result = report()
    if args.self_test:
        self_test(result)
    if args.output:
        with open(args.output, "w", encoding="utf-8", newline="\n") as handle:
            json.dump(result, handle, sort_keys=True, indent=2)
            handle.write("\n")
    if args.summary:
        print_summary(result)
    return 0 if result["spike_checks_pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
