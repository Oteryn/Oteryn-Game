#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import export as producer


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value: Any) -> bytes:
    return producer.canonical_bytes(value)


def verify(root: Path) -> dict[str, Any]:
    product_path = root / "product.json"
    if not product_path.is_file():
        raise producer.ProductError("missing product.json")
    envelope = json.loads(product_path.read_text(encoding="utf-8"))
    if envelope.get("capability") != producer.CAPABILITY or envelope.get("source") != producer._source_identity():
        raise producer.ProductError("product envelope identity mismatch")
    files = envelope.get("files")
    expected_files = {"manifest.json", "census.json", "object-programs.jsonl", "outfit-programs.jsonl"}
    if not isinstance(files, dict) or set(files) != expected_files:
        raise producer.ProductError("unexpected product file set")
    for name, meta in files.items():
        path = root / name
        if not path.is_file():
            raise producer.ProductError(f"missing product file {name}")
        payload = path.read_bytes()
        if len(payload) != int(meta.get("bytes", -1)) or sha(payload) != meta.get("sha256"):
            raise producer.ProductError(f"product file digest/size mismatch for {name}")

    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("contract_id") != producer.CONTRACT_ID or manifest.get("capability") != producer.CAPABILITY:
        raise producer.ProductError("manifest contract mismatch")
    if manifest.get("source") != producer._source_identity():
        raise producer.ProductError("manifest source mismatch")

    object_programs = [json.loads(line) for line in (root / "object-programs.jsonl").read_text(encoding="utf-8").splitlines()]
    outfit_programs = [json.loads(line) for line in (root / "outfit-programs.jsonl").read_text(encoding="utf-8").splitlines()]
    seen: set[str] = set()
    for row in object_programs + outfit_programs:
        program_id = row.pop("program_id", None)
        expected = "animation-program:sha256:" + sha(canonical(row))
        row["program_id"] = program_id
        if program_id != expected:
            raise producer.ProductError("animation program content identity mismatch")
        if program_id in seen:
            raise producer.ProductError("duplicate animation program identity")
        seen.add(program_id)

    core = dict(manifest)
    claimed_root = core.pop("product_root", None)
    root_payload = b"OTERYN-ANIMATED-APPEARANCES-V1\0" + canonical(core)
    for row in object_programs:
        root_payload += canonical(row)
    for row in outfit_programs:
        root_payload += canonical(row)
    expected_root = "sha256:" + sha(root_payload)
    if claimed_root != expected_root or envelope.get("product_root") != expected_root:
        raise producer.ProductError("animation product root mismatch")

    stats = manifest.get("statistics", {})
    if int(stats.get("animated_object_programs", -1)) != len(object_programs):
        raise producer.ProductError("object program count mismatch")
    if int(stats.get("outfit_programs", -1)) != len(outfit_programs):
        raise producer.ProductError("outfit program count mismatch")
    return {"status": "PASS", "product_root": expected_root, "animated_object_programs": len(object_programs), "outfit_programs": len(outfit_programs)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("product", type=Path)
    args = parser.parse_args()
    try:
        result = verify(args.product)
    except (producer.ProductError, OSError, json.JSONDecodeError, ValueError) as exc:
        raise SystemExit(f"ERROR: {exc}") from exc
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
