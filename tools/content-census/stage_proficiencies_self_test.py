#!/usr/bin/env python3
"""Tests for the weapon proficiency staging script."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location("stage_proficiencies", HERE / "stage_proficiencies.py")
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


def obj(object_id: int, proficiency: int | None, name: str | None = "test sword") -> bytes:
    flags = v(13, 1) + (b(61, v(1, proficiency)) if proficiency is not None else b"")
    return b(1, v(1, object_id) + b(3, flags) + (b(4, name) if name else b""))


def definitions(*ids: int) -> bytes:
    return json.dumps([
        {"Levels": [{"Perks": [{"Type": 3, "Value": 1}, {"SkillId": 8, "Type": 3, "Value": 0.5}]}],
         "Name": f"P{i}", "ProficiencyId": i, "Version": 1} for i in ids]).encode()


def expect_reject(fn, *args) -> None:
    try:
        fn(*args)
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_stage_synthetic() -> None:
    rows = stage.stage_definitions(definitions(6, 8))
    assert [r["source_id"] for r in rows] == [6, 8] and rows[0]["levels"][0]["Perks"][1]["Value"] == 0.5
    bindings = stage.stage_bindings(obj(10, 6) + obj(11, None) + obj(12, 8, None), {6, 8})
    assert [(r["source_id"], r["proficiency_id"], r["name"]) for r in bindings] == [(10, 6, "test sword"), (12, 8, None)]


def test_rejections() -> None:
    expect_reject(stage.stage_definitions, definitions(6, 6))  # duplicate id
    expect_reject(stage.stage_definitions, json.dumps([{"Name": "x"}]).encode())  # unexpected keys
    bad_perk = json.loads(definitions(6))
    bad_perk[0]["Levels"][0]["Perks"][0]["Extra"] = 1
    expect_reject(stage.stage_definitions, json.dumps(bad_perk).encode())
    expect_reject(stage.stage_bindings, obj(10, 99), {6})  # dangling proficiency id
    expect_reject(stage.stage_bindings, obj(10, 6) + obj(10, 6), {6})  # duplicate object id
    expect_reject(stage.stage_bindings, b(1, b(3, b(61, v(1, 6)))), {6})  # object without id
    expect_reject(stage.stage_bindings, b(1, v(1, 1) + b(3, b(61, v(1, 6) + v(2, 1)))), {6})  # flag shape
    expect_reject(stage.stage_bindings, obj(10, 6)[:-2], {6})  # truncated
    expect_reject(stage.parse, b"\x0d")  # unsupported wire type


def test_tampered_input_rejected() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        for role, (ext, sha) in stage.INPUTS.items():
            (Path(tmp) / f"{role}-{sha}.{ext}").write_bytes(b"tampered")
        expect_reject(stage.read_input, Path(tmp), "proficiencies")
        expect_reject(stage.read_input, Path(tmp), "appearances")


def test_committed_bytes_regenerate() -> None:
    inputs = ROOT / stage.DEFAULT_INPUT_DIR
    expected = stage.build(*(stage.read_input(inputs, role) for role in stage.INPUTS))
    assert stage.compare(ROOT / stage.DEFAULT_OUTPUT, expected) == []


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("stage_proficiencies self-test: ok")
