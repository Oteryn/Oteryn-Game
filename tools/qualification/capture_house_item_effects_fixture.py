#!/usr/bin/env python3
"""Pin exact item-effect batches emitted after real committed House USE cases.

No protocol bytes are constructed here. Input is the production publisher capture.
"""
import argparse
import hashlib
import json
from pathlib import Path

CRYSTAL_SHA = "201189f871d23597b82caf88fd611454a2ddefb8"
EXPECTED = {"fish": 175, "harp": 19}


def materialize(capture):
    if not isinstance(capture, list) or len(capture) != len(EXPECTED):
        raise ValueError("input must contain the two actual committed producer records")
    seen = set()
    effects = []
    tests = []
    for record in capture:
        source = record.get("source")
        if not isinstance(source, dict) or source.get("crystal_sha") != CRYSTAL_SHA:
            raise ValueError("capture must identify the pinned Crystal source")
        producer = source.get("producer_test")
        if not isinstance(producer, str) or not producer:
            raise ValueError("capture must identify the actual committed producer test")
        tests.append(producer)
        viewer = record.get("viewer")
        if not isinstance(viewer, dict) or not isinstance(viewer.get("house"), str):
            raise ValueError("capture requires the independent actual House viewer")
        position = viewer.get("position")
        if (not isinstance(position, list) or len(position) != 3
                or any(type(value) is not int for value in position)
                or not 0 <= position[2] <= 15
                or type(viewer.get("ownership_generation")) is not int
                or viewer["ownership_generation"] <= 0):
            raise ValueError("capture requires exact native runtime viewer coordinates and lease")
        map_hex = record.get("world_map_snapshot_hex")
        if not isinstance(map_hex, str) or len(map_hex) > 2_097_152:
            raise ValueError("capture requires the actual bounded House map snapshot")
        map_bytes = bytes.fromhex(map_hex)
        if not map_bytes:
            raise ValueError("actual House map snapshot is empty")
        rows = record.get("effects")
        if not isinstance(rows, list) or len(rows) != 1:
            raise ValueError("each actual producer must supply one committed source batch")
        row = rows[0]
        name = row.get("name")
        if name in seen or name not in EXPECTED or row.get("expected_effect_id") != EXPECTED[name]:
            raise ValueError("duplicate, unknown or incorrect source effect")
        seen.add(name)
        raw = row.get("payload_hex")
        if not isinstance(raw, str) or len(raw) > 131136 * 2:
            raise ValueError("missing or over-limit production item-effect batch")
        payload = bytes.fromhex(raw)
        if not payload or len(payload) > 131136:
            raise ValueError("empty or over-limit production item-effect batch")
        effects.append({"name": name, "expected_effect_id": EXPECTED[name],
                        "payload_hex": payload.hex(),
                        "payload_sha256": hashlib.sha256(payload).hexdigest(),
                        "producer_test": producer, "viewer": viewer,
                        "world_map_snapshot_hex": map_bytes.hex(),
                        "world_map_snapshot_sha256": hashlib.sha256(map_bytes).hexdigest()})
    return json.dumps({"schema": "OTERYN_NATIVE_HOUSE_ITEM_EFFECTS_PLAY/v1",
                       "source": {"crystal_sha": CRYSTAL_SHA, "producer_tests": sorted(tests)},
                       "effects": sorted(effects, key=lambda row: row["name"])},
                      indent=2, sort_keys=True) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = materialize(json.loads(args.capture.read_text(encoding="utf-8")))
    if args.check:
        if args.output.read_text(encoding="utf-8") != result:
            raise SystemExit("native House item-effect fixture is stale")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(result, encoding="utf-8")
    print(f"native House item-effect fixture {'verified' if args.check else 'generated'}: {args.output}")


if __name__ == "__main__":
    main()
