#!/usr/bin/env python3
"""Tests for the staticdata creature/bestiary-class/boss/quest-line staging script."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location("stage_cbq", HERE / "stage_staticdata_creatures_bosses_quests.py")
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


LOOK = v(1, 2) + b(2, v(1, 0) + v(2, 0)) + v(3, 0)


def creature(cid: int = 2, *, extra: bytes = b"") -> bytes:
    return v(1, cid) + b(2, "orc warlord") + b(3, LOOK) + v(4, 3) + v(5, 0) + v(6, 1) + v(7, 1) + extra


def boss(bid: int = 46) -> bytes:
    return v(1, bid) + b(2, "Black Knight") + b(3, LOOK) + v(4, 0)


def staticdata() -> bytes:
    return (b(1, creature()) + b(2, v(1, 1) + b(2, "Amphibic")) + b(5, boss()) + b(6, v(1, 3) + b(2, "The Paradox Tower")))


def test_stage_synthetic() -> None:
    data = staticdata()
    assert stage.stage_creatures(data)[0]["look"]["hex"] == LOOK.hex()
    assert stage.stage_creatures(data)[0]["f4"] == 3
    assert stage.stage_classes(data)[0]["name"] == "Amphibic"
    assert stage.stage_bosses(data)[0]["source_id"] == 46
    assert stage.stage_quests(data)[0]["name"] == "The Paradox Tower"


def expect_reject(fn, *args) -> None:
    try:
        fn(*args)
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_rejections() -> None:
    expect_reject(stage.stage_creatures, b(1, creature(extra=v(8, 1))))  # unknown field
    expect_reject(stage.stage_creatures, b(1, creature()) + b(1, creature()))  # duplicate id
    expect_reject(stage.stage_bosses, b(5, boss()) + b(5, boss()))  # duplicate id
    expect_reject(stage.stage_quests, b(6, v(1, 3) + v(2, 5)))  # wrong wire type
    expect_reject(stage.stage_creatures, b(1, creature()[:-3]))  # truncated
    expect_reject(stage.stage_classes, b(2, v(1, 1)))  # missing field
    expect_reject(stage.build, staticdata())  # wrong counts


def test_tampered_input_rejected() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / f"staticdata-{stage.SHA}.dat").write_bytes(b"tampered")
        expect_reject(stage.base.read_input, Path(tmp), "staticdata")


def test_committed_bytes_regenerate() -> None:
    expected = stage.build(stage.base.read_input(ROOT / stage.DEFAULT_INPUT_DIR, stage.ROLE))
    assert stage.base.compare(ROOT / stage.DEFAULT_OUTPUT, expected) == []


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("stage_staticdata_creatures_bosses_quests self-test: ok")
