#!/usr/bin/env python3
"""Tests for the appearance tables staging script."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("stage_appearance_tables", HERE / "stage_appearance_tables.py")
if spec is None or spec.loader is None:
    raise RuntimeError("stage import failed")
stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage)
ROOT = HERE.parents[1]


def ev(n: int) -> bytes:
    out = b""
    while True:
        byte, n = n & 0x7F, n >> 7
        out += bytes([byte | (0x80 if n else 0)])
        if not n:
            return out


def v(field: int, n: int) -> bytes:
    return ev(field << 3) + ev(n)


def b(field: int, data: bytes) -> bytes:
    return ev(field << 3 | 2) + ev(len(data)) + data


def record(rid: int) -> bytes:
    info = v(1, 1) + v(2, 1) + v(3, 1) + v(4, 1) + v(5, 100) + v(5, 101) + v(8, 0)
    return v(1, rid) + b(2, v(1, 0) + v(2, 0) + b(3, info)) + b(3, b(23, v(1, 4) + v(2, 9)) + v(29, 1))


def run_case(top: bytes) -> None:
    saved = (stage.TABLES, stage.EXPECTED_OBJECTS, stage.EXPECTED_EXTRA)
    stage.TABLES = {2: ("outfits", "Outfit", 2), 3: ("effects", "Effect", 1), 4: ("missiles", "Missile", 1)}
    stage.EXPECTED_OBJECTS, stage.EXPECTED_EXTRA = 1, 1
    try:
        stage.build(top)
    finally:
        stage.TABLES, stage.EXPECTED_OBJECTS, stage.EXPECTED_EXTRA = saved


def expect_reject(top: bytes, label: str) -> None:
    try:
        run_case(top)
    except ValueError:
        return
    raise AssertionError(f"not rejected: {label}")


GOOD = b(1, v(1, 7)) + b(2, record(1)) + b(2, record(2)) + b(3, record(1)) + b(4, record(1)) + b(5, b"x")


def main() -> None:
    run_case(GOOD)
    row = stage.stage_record(0, record(5), "outfits")
    assert row["frame_groups"][0]["sprite_ids"] == [100, 101] and row["frame_groups"][0]["sprite_count"] == 2
    assert row["flags"][0]["varint_fields"] == [[1, 4], [2, 9]] and row["flags"][1]["value"] == 1
    assert row["source_record_sha256"] == stage.digest(record(5))
    expect_reject(GOOD + b(6, b"x"), "unknown top-level field")
    expect_reject(GOOD + v(2, 1), "bad top-level wire type")
    expect_reject(GOOD.replace(b(2, record(2)), b(2, record(1))), "duplicate id")
    expect_reject(GOOD + b(2, record(3)), "wrong count")
    expect_reject(b(1, v(1, 7)) + b(2, v(1, 1) + v(9, 1)), "unknown record field")
    expect_reject(GOOD[:-1], "truncated input")
    input_dir = ROOT / "content/assets/files"
    if input_dir.is_dir() and (ROOT / stage.DEFAULT_OUTPUT).is_dir():
        assert stage.main(["--check", "--input-dir", str(input_dir), "--output", str(ROOT / stage.DEFAULT_OUTPUT)]) == 0
        real = stage.base.read_input(input_dir, stage.INPUT_ROLE)
        with tempfile.TemporaryDirectory() as tmp:
            expected = stage.build(real)
            for rel, data in expected.items():
                path = Path(tmp) / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            assert stage.compare(Path(tmp), expected) == []
            (Path(tmp) / "missiles/manifest.json").write_bytes(b"{}\n")
            assert stage.compare(Path(tmp), expected) == ["differs or missing: missiles/manifest.json"]
            (Path(tmp) / "effects/extra.json").write_bytes(b"{}\n")
            assert "unexpected file effects/extra.json" in stage.compare(Path(tmp), expected)
        tampered = bytearray(real)
        tampered[100] ^= 1
        with tempfile.TemporaryDirectory() as tmp:
            name = f"appearances-{stage.base.INPUTS[stage.INPUT_ROLE][1]}.dat"
            (Path(tmp) / name).write_bytes(bytes(tampered))
            try:
                stage.base.read_input(Path(tmp), stage.INPUT_ROLE)
            except ValueError:
                pass
            else:
                raise AssertionError("tampered input accepted")
    print("appearance table staging self-test: ok")


if __name__ == "__main__":
    main()
