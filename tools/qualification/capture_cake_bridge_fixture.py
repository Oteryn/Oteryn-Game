#!/usr/bin/env python3
"""Materialize native Cake bridge play bytes captured by the real server test.

This tool never constructs protocol bytes. The producer supplies hex directly from
production world-map snapshot/delta encoders over the pinned tower B3 projection.
"""
import argparse
import hashlib
import json
from pathlib import Path

B3_SHA256 = "2798be76221d6dc00695258d46d1eb375a64e804a7a84265014e2af0c679b05d"
PAYLOADS = ("closed_snapshot_hex", "open_delta_hex", "cleanup_delta_hex")
MAX_PAYLOAD_BYTES = 1_048_576


def materialize(capture):
    source = capture.get("source")
    if not isinstance(source, dict) or source.get("b3_sha256") != B3_SHA256:
        raise ValueError("capture must identify the qualified original tower B3 SHA-256")
    if not source.get("producer_test"):
        raise ValueError("capture must name its actual production projection test")
    runtime = source.get("observer_runtime")
    native = source.get("observer_native")
    if not isinstance(runtime, dict) or not isinstance(native, dict):
        raise ValueError("capture requires the independently observed runtime/native viewer")
    for key in ("x", "y", "floor"):
        if type(runtime.get(key)) is not int or type(native.get(key)) is not int:
            raise ValueError("viewer coordinates must be actual integer observations")
    if not (0 <= runtime["floor"] <= 15) or native != {
        "x": runtime["x"], "y": runtime["y"], "floor": -runtime["floor"]
    }:
        raise ValueError("native viewer must convert the runtime floor exactly once")
    for key, length in (("observer_actor_hex", 32), ("world_id_hex", 32),
                        ("bundle_digest_hex", 64), ("bundle_sha256", 64)):
        value = source.get(key)
        if not isinstance(value, str) or len(value) != length:
            raise ValueError(f"{key}: missing actual capture binding")
        bytes.fromhex(value)
    output = {"schema": "OTERYN_NATIVE_CAKE_BRIDGE_PLAY/v1", "source": source}
    digests = {}
    for key in PAYLOADS:
        value = capture.get(key)
        if not isinstance(value, str) or len(value) > MAX_PAYLOAD_BYTES * 2:
            raise ValueError(f"{key}: missing or over-limit captured hex")
        payload = bytes.fromhex(value)
        if not payload or len(payload) > MAX_PAYLOAD_BYTES:
            raise ValueError(f"{key}: empty or over-limit protocol payload")
        output[key] = payload.hex()
        digests[key] = hashlib.sha256(payload).hexdigest()
    output["payload_sha256"] = digests
    return json.dumps(output, indent=2, sort_keys=True) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path, help="JSON emitted by the real server projection test")
    parser.add_argument("output", type=Path, help="native play fixture path")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = materialize(json.loads(args.capture.read_text(encoding="utf-8")))
    if args.check:
        if args.output.read_text(encoding="utf-8") != result:
            raise SystemExit("native Cake bridge fixture is stale")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(result, encoding="utf-8")
    print(f"native Cake bridge fixture {'verified' if args.check else 'generated'}: {args.output}")


if __name__ == "__main__":
    main()
