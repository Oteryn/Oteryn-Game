#!/usr/bin/env python3
"""Tests for the map areas staging script."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import struct
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location("stage_map_areas", HERE / "stage_map_areas.py")
if spec is None or spec.loader is None:
    raise RuntimeError("stage import failed")
stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage)

HASH = "a" * 64


def enc_varint(n: int) -> bytes:
    out = b""
    while True:
        byte, n = n & 0x7F, n >> 7
        out += bytes([byte | (0x80 if n else 0)])
        if not n:
            return out


def v(field: int, n: int) -> bytes:
    return enc_varint(field << 3) + enc_varint(n)


def b(field: int, data: bytes | str) -> bytes:
    data = data.encode() if isinstance(data, str) else data
    return enc_varint(field << 3 | 2) + enc_varint(len(data)) + data


def pos(x: int = 100, y: int = 200, z: int = 7) -> bytes:
    return v(1, x) + v(2, y) + v(3, z)


def area(area_id: int, members: tuple[int, ...] = (), extra: bytes = b"") -> bytes:
    return b(1, v(1, area_id) + b(2, f"Area {area_id}") + v(3, 2) + b"".join(v(4, m) for m in members) + extra)


def layer_subarea(area_id: int, file: str | None = None) -> bytes:
    file = file or f"subarea-{area_id:04d}-{HASH}.bmp.lzma"
    return b(3, v(1, 0) + b(2, pos()) + b(3, file) + v(4, 10) + v(5, 20) + v(6, area_id))


def layer_satellite() -> bytes:
    body = v(1, 1) + b(2, pos()) + b(3, f"satellite-16-0996-0968-00-{HASH}.bmp.lzma") + v(4, 256) + v(5, 256)
    return b(3, body + bytes([0x39]) + struct.pack("<d", 0.0625))


def un(blob: bytes) -> bytes:
    return stage.parse(blob)[0][2]  # body of a single top-level record


def expect_reject(fn, *args) -> None:
    try:
        fn(*args)
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_stage_synthetic() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        (directory / f"subarea-0002-{HASH}.bmp.lzma").write_bytes(b"x")
        (directory / f"satellite-16-0996-0968-00-{HASH}.bmp.lzma").write_bytes(b"y")
        raws = [un(layer_subarea(2)), un(layer_satellite())]  # strip tag+len (short bodies)
        layers = stage.stage_layers(raws, {1, 2}, directory)
        assert [r["layer_kind"] for r in layers] == [0, 1] and layers[0]["area_id"] == 2 and layers[1]["scale"] == 0.0625
        assert layers[0]["image"]["file_sha256"] == stage.digest(b"x") and layers[0]["image"]["name_hash"] == HASH
        rows = stage.stage_areas([un(area(1, (2,))), un(area(2, (), b(5, pos(1, 2, 3))))], {2: layers[0]["image"]})
        assert rows[0]["subarea_ids"] == [2] and rows[1]["position"] == {"x": 1, "y": 2, "z": 3} and rows[1]["subarea_image"]
        markers = stage.stage_markers([b(1, "M") + b(2, pos()) + v(3, 9)])
        assert markers[0]["icon_id"] == 9 and markers[0]["position"]["z"] == 7


def test_rejections() -> None:
    body = un
    expect_reject(stage.stage_areas, [body(area(1)), body(area(1))], {})  # duplicate area id
    expect_reject(stage.stage_areas, [body(area(1, (9,)))], {})  # dangling subarea id
    expect_reject(stage.stage_areas, [body(area(1, (1, 1)))], {})  # duplicate id in list
    expect_reject(stage.stage_areas, [body(area(1, (), v(9, 1)))], {})  # unknown field
    expect_reject(stage.stage_areas, [body(area(1, (), b(3, b"x")))], {})  # wrong wire type / repeated
    expect_reject(stage.stage_areas, [body(area(1, (), b(5, v(1, 1))))], {})  # position shape
    expect_reject(stage.stage_areas, [body(area(1, (), b(5, pos(1, 2, 99))))], {})  # z out of range
    expect_reject(stage.stage_markers, [b(1, "M") + b(2, pos())])  # missing icon
    expect_reject(stage.stage_markers, [b(1, "M") + b(2, pos()) + v(3, 1) + v(8, 1)])  # unknown field
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        (directory / f"subarea-0002-{HASH}.bmp.lzma").write_bytes(b"x")
        good = un(layer_subarea(2))
        expect_reject(stage.stage_layers, [good, good], {2}, directory)  # duplicate subarea id
        expect_reject(stage.stage_layers, [good], {1}, directory)  # dangling subarea id
        expect_reject(stage.stage_layers, [un(layer_subarea(3))], {3}, directory)  # image file missing
        expect_reject(stage.stage_layers, [un(layer_subarea(2, "evil.bmp"))], {2}, directory)  # file name shape
        expect_reject(stage.stage_layers, [un(layer_subarea(2, f"subarea-0005-{HASH}.bmp.lzma"))], {2}, directory)  # id mismatch
        expect_reject(stage.stage_layers, [good + bytes([0x39]) + struct.pack("<d", 1.0)], {2}, directory)  # scale on subarea
    expect_reject(stage.split_tables, b(9, b"x"))  # unknown top-level field
    expect_reject(stage.split_tables, v(1, 1))  # top-level wrong wire type
    expect_reject(stage.split_tables, b(4, pos()))  # bounds incomplete
    expect_reject(stage.parse, b"\x0d")  # unsupported wire type


def test_tampered_input_rejected() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / stage.MAP_FILE).write_bytes(b"tampered")
        expect_reject(stage.read_map, Path(tmp))


def test_count_mismatch_rejected() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        expect_reject(stage.build, b(4, pos()) + b(5, pos()), Path(tmp))


def test_committed_bytes_regenerate() -> None:
    inputs = ROOT / stage.DEFAULT_INPUT_DIR
    expected = stage.build(stage.read_map(inputs), inputs)
    assert stage.compare(ROOT / stage.DEFAULT_OUTPUT, expected) == []
    tampered = dict(expected)
    tampered["areas/manifest.json"] += b" "
    assert stage.compare(ROOT / stage.DEFAULT_OUTPUT, tampered)


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("stage_map_areas self-test: ok")
