#!/usr/bin/env python3
"""Tests for the staticdata House/Achievement staging script."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location("stage_staticdata", HERE / "stage_staticdata_houses_achievements.py")
if spec is None or spec.loader is None:
    raise RuntimeError("stage import failed")
stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage)


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


def pos(x: int, y: int, z: int) -> bytes:
    return v(1, x) + v(2, y) + v(3, z)


def house(house_id: int = 7, *, extra: bytes = b"") -> bytes:
    return (v(1, house_id) + b(2, "Test House") + b(3, "") + v(4, 50000) + v(5, 2) + b(6, pos(1, 2, 3))
            + v(7, 20) + v(8, 0) + b(9, "Thais") + v(10, 1) + extra)


def layout(*, skip_total: int = 1) -> bytes:
    cells = b(3, b(1, v(1, 1283)) + v(2, 0)) + b(3, v(2, skip_total))
    return b(1, pos(10, 20, 6)) + b(2, v(1, 3) + v(2, 1) + v(3, 1)) + b(3, b(2, cells))


def staticmap(house_id: int = 7, *, skip_total: int = 1) -> bytes:
    return b(1, v(1, house_id) + b(2, layout(skip_total=skip_total)))


def staticdata() -> bytes:
    return b(4, house()) + b(3, v(1, 13) + b(2, "Trader") + b(3, "Sells things") + v(4, 2))


def test_stage_synthetic() -> None:
    houses = stage.stage_houses(staticdata(), staticmap())
    assert houses[0]["rent_gold"] == 50000 and houses[0]["town"] == "Thais" and houses[0]["shop"] is True
    assert houses[0]["layout"]["cells"] == [{"items": [1283], "skip": 0}, {"items": [], "skip": 1}]
    assert stage.stage_achievements(staticdata())[0]["grade"] == 2


def expect_reject(fn, *args) -> None:
    try:
        fn(*args)
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_rejections() -> None:
    expect_reject(stage.stage_houses, b(4, house(extra=v(11, 1))), staticmap())  # unknown field
    expect_reject(stage.stage_houses, b(4, house()) + b(4, house()), staticmap())  # duplicate id
    expect_reject(stage.stage_houses, staticdata(), staticmap(house_id=8))  # id sets differ
    expect_reject(stage.stage_houses, staticdata(), staticmap(skip_total=2))  # cell count identity broken
    expect_reject(stage.stage_houses, b(4, house()[:-3]), staticmap())  # truncated
    expect_reject(stage.parse, b"\x0b")  # unsupported wire type


def test_tampered_input_rejected() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        for role, sha in stage.INPUTS.items():
            (Path(tmp) / f"{role}-{sha}.dat").write_bytes(b"tampered")
        expect_reject(stage.read_input, Path(tmp), "staticdata")


def test_committed_bytes_regenerate() -> None:
    inputs = ROOT / stage.DEFAULT_INPUT_DIR
    expected = stage.build(*(stage.read_input(inputs, role) for role in stage.INPUTS))
    assert stage.compare(ROOT / stage.DEFAULT_OUTPUT, expected) == []


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("stage_staticdata_houses_achievements self-test: ok")
