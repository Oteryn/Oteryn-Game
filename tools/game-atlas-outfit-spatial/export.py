#!/usr/bin/env python3
"""Game-owned exact-15.32 outfit spatial/presentation sidecar for Atlas.

The animation catalog deliberately focuses on frame programs. This bounded
sidecar preserves appearance-level outfit flags that materially affect static
pixel placement. Unknown reverse-addon ordering is carried explicitly so the
consumer can fail closed for affected directions instead of inventing an
ordering rule.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
from typing import Any, Iterator

HERE = Path(__file__).resolve().parent
APPEARANCE_EXPORT = HERE.parent / "game-atlas-appearances" / "export.py"
CAPABILITY = "outfit-spatial-v1"
CONTRACT_ID = "oteryn-game-atlas-outfit-spatial-v1"
ANCHOR_POLICY = "tile-bottom-right-minus-sprite-overhang-and-displacement-v1"
REVERSE_FIELDS = {49: "east", 50: "west", 51: "south", 52: "north"}
MAX_OUTFITS = 10_000


class SpatialError(RuntimeError):
    pass


def _load_appearance_module():
    spec = importlib.util.spec_from_file_location("game_atlas_appearance_for_spatial", APPEARANCE_EXPORT)
    if spec is None or spec.loader is None:
        raise SpatialError("unable to load appearance source verifier")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def canonical(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def _sha(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _varint(data: bytes, offset: int) -> tuple[int, int]:
    value = 0
    shift = 0
    while offset < len(data) and shift < 70:
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7f) << shift
        if byte < 0x80:
            return value, offset
        shift += 7
    raise SpatialError("invalid protobuf varint")


def _fields(data: bytes) -> Iterator[tuple[int, int, int | bytes]]:
    offset = 0
    while offset < len(data):
        key, offset = _varint(data, offset)
        field, wire = key >> 3, key & 7
        if field <= 0:
            raise SpatialError("invalid protobuf field")
        if wire == 0:
            value, offset = _varint(data, offset)
        elif wire == 2:
            size, offset = _varint(data, offset)
            if offset + size > len(data):
                raise SpatialError("truncated protobuf field")
            value = data[offset:offset + size]
            offset += size
        elif wire == 1:
            if offset + 8 > len(data):
                raise SpatialError("truncated fixed64")
            value = data[offset:offset + 8]
            offset += 8
        elif wire == 5:
            if offset + 4 > len(data):
                raise SpatialError("truncated fixed32")
            value = data[offset:offset + 4]
            offset += 4
        else:
            raise SpatialError(f"unsupported protobuf wire type {wire}")
        yield field, wire, value


def _values(data: bytes) -> dict[int, list[int | bytes]]:
    result: dict[int, list[int | bytes]] = {}
    for field, _wire, value in _fields(data):
        result.setdefault(field, []).append(value)
    return result


def _int(values: dict[int, list[int | bytes]], field: int, default: int = 0) -> int:
    entries = values.get(field)
    return int(entries[0]) if entries and isinstance(entries[0], int) else default


def decode_outfits(appearance_bytes: bytes) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    seen: set[int] = set()
    for field, wire, payload in _fields(appearance_bytes):
        if field != 2 or wire != 2 or not isinstance(payload, bytes):
            continue
        appearance = _values(payload)
        look_type = _int(appearance, 1)
        if look_type <= 0 or look_type in seen:
            raise SpatialError(f"invalid/duplicate outfit appearance id {look_type}")
        flags_payload = next((value for value in appearance.get(3, ()) if isinstance(value, bytes)), None)
        flags = _values(flags_payload) if flags_payload is not None else {}
        shift_payload = next((value for value in flags.get(26, ()) if isinstance(value, bytes)), None)
        shift = _values(shift_payload) if shift_payload is not None else {}
        displacement = {"x": _int(shift, 1), "y": _int(shift, 2)}
        reverse = {
            direction: bool(_int(flags, source_field))
            for source_field, direction in sorted(REVERSE_FIELDS.items(), key=lambda item: item[1])
        }
        core = {
            "anchor_policy": ANCHOR_POLICY,
            "animate_always": bool(_int(flags, 29)),
            "displacement": displacement,
            "look_type": look_type,
            "reverse_addons": reverse,
        }
        records.append({**core, "spatial_record_id": "outfit-spatial:sha256:" + _sha(canonical(core))})
        seen.add(look_type)
        if len(records) > MAX_OUTFITS:
            raise SpatialError("outfit count exceeds bound")
    records.sort(key=lambda row: row["look_type"])
    return records


def build_from_bytes(appearance_bytes: bytes, source: dict[str, str]) -> dict[str, Any]:
    records = decode_outfits(appearance_bytes)
    stats = {
        "outfits": len(records),
        "shift_present_or_zero_records": sum(1 for row in records if row["displacement"] != {"x": 0, "y": 0}),
        "nonzero_displacement_records": sum(1 for row in records if row["displacement"]["x"] or row["displacement"]["y"]),
        "animate_always_records": sum(1 for row in records if row["animate_always"]),
        "reverse_addons_true": {
            direction: sum(1 for row in records if row["reverse_addons"][direction])
            for direction in ("north", "east", "south", "west")
        },
    }
    # Exact presence of a shift message is useful census evidence and cannot be
    # reconstructed after decoding zero-valued shifts, so calculate it directly.
    shift_message_count = 0
    for field, wire, payload in _fields(appearance_bytes):
        if field != 2 or wire != 2 or not isinstance(payload, bytes):
            continue
        appearance = _values(payload)
        flags_payload = next((v for v in appearance.get(3, ()) if isinstance(v, bytes)), None)
        if flags_payload is not None and 26 in _values(flags_payload):
            shift_message_count += 1
    stats["shift_flag_records"] = shift_message_count
    core = {
        "capability": CAPABILITY,
        "contract_id": CONTRACT_ID,
        "anchor_policy": ANCHOR_POLICY,
        "reverse_addon_policy": "preserve-source-flags; south=true is unsupported for static addon composition until a renderer rule is proven",
        "source": source,
        "statistics": stats,
    }
    root_payload = b"OTERYN-OUTFIT-SPATIAL-V1\0" + canonical(core) + b"".join(canonical(row) for row in records)
    manifest = {**core, "product_root": "sha256:" + _sha(root_payload)}
    return {"manifest": manifest, "records": records}


def write_product(product: dict[str, Any], output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    records = b"".join(canonical(row) for row in product["records"])
    manifest = canonical(product["manifest"])
    (output / "manifest.json").write_bytes(manifest)
    (output / "outfits.jsonl").write_bytes(records)
    envelope = {
        "capability": CAPABILITY,
        "files": {
            "manifest.json": {"bytes": len(manifest), "sha256": _sha(manifest)},
            "outfits.jsonl": {"bytes": len(records), "sha256": _sha(records)},
        },
        "product_root": product["manifest"]["product_root"],
        "source": product["manifest"]["source"],
    }
    (output / "product.json").write_bytes(canonical(envelope))


def load_index(product_dir: Path) -> tuple[dict[str, Any], dict[int, dict[str, Any]]]:
    manifest = json.loads((product_dir / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("capability") != CAPABILITY or manifest.get("contract_id") != CONTRACT_ID:
        raise SpatialError("unsupported outfit spatial product")
    rows = [json.loads(line) for line in (product_dir / "outfits.jsonl").read_text(encoding="utf-8").splitlines()]
    index: dict[int, dict[str, Any]] = {}
    for row in rows:
        core = dict(row)
        claimed = core.pop("spatial_record_id", None)
        expected = "outfit-spatial:sha256:" + _sha(canonical(core))
        if claimed != expected:
            raise SpatialError("outfit spatial record identity mismatch")
        look_type = int(row["look_type"])
        if look_type in index:
            raise SpatialError(f"duplicate outfit spatial record {look_type}")
        index[look_type] = row
    core = dict(manifest)
    claimed_root = core.pop("product_root", None)
    payload = b"OTERYN-OUTFIT-SPATIAL-V1\0" + canonical(core) + b"".join(canonical(row) for row in rows)
    if claimed_root != "sha256:" + _sha(payload):
        raise SpatialError("outfit spatial product root mismatch")
    return manifest, index


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset_zip", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    appearance_module = _load_appearance_module()
    try:
        _catalog, appearance = appearance_module.read_exact_source_zip(args.asset_zip)
        product = build_from_bytes(appearance, appearance_module._source_identity())
        write_product(product, args.output)
    except (SpatialError, appearance_module.ProductError) as exc:
        raise SystemExit(f"ERROR: {exc}") from exc
    print(json.dumps({"product_root": product["manifest"]["product_root"], **product["manifest"]["statistics"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
