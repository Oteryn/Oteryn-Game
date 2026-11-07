#!/usr/bin/env python3
"""Pin actual committed House dragon chat, magic, vitals and map producer bytes."""
import argparse
import hashlib
import json
from pathlib import Path

CRYSTAL = "201189f871d23597b82caf88fd611454a2ddefb8"
KEYS = ("world_map_snapshot_hex", "chat_snapshot_hex", "chat_line_hex", "effect_batch_hex", "vitals_hex")


def materialize(capture):
    source = capture.get("source")
    viewer = capture.get("viewer")
    if not isinstance(source, dict) or source.get("crystal_sha") != CRYSTAL or not source.get("producer_test"):
        raise ValueError("actual pinned donor and committed producer test required")
    if not isinstance(viewer, dict) or not viewer.get("house"):
        raise ValueError("actual independent House viewer required")
    position = viewer.get("position")
    if (not isinstance(position, list) or len(position) != 3
            or any(type(value) is not int for value in position) or not 0 <= position[2] <= 15):
        raise ValueError("actual positive native runtime viewer coordinates required")
    output = {"schema": "OTERYN_NATIVE_SANTA_GIFT_PLAY/v1", "source": source, "viewer": viewer}
    digests = {}
    for key in KEYS:
        value = capture.get(key)
        if not isinstance(value, str) or len(value) > 2_097_152:
            raise ValueError(f"{key}: missing or oversized actual producer bytes")
        payload = bytes.fromhex(value)
        # An actual CHAT v1 snapshot with no joined rooms is an empty proto3 message.
        if not payload and key != "chat_snapshot_hex":
            raise ValueError(f"{key}: empty producer payload")
        output[key] = payload.hex()
        digests[key] = hashlib.sha256(payload).hexdigest()
    output["payload_sha256"] = digests
    return json.dumps(output, indent=2, sort_keys=True) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    value = materialize(json.loads(args.capture.read_text(encoding="utf-8")))
    if args.check:
        if args.output.read_text(encoding="utf-8") != value:
            raise SystemExit("native Santa gift fixture is stale")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(value, encoding="utf-8")
    print(f"native Santa gift fixture {'verified' if args.check else 'generated'}: {args.output}")


if __name__ == "__main__":
    main()
