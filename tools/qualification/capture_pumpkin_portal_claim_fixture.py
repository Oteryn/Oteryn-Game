#!/usr/bin/env python3
"""Materialize native Pumpkin portal play bytes captured by the real server test.

This tool never constructs protocol bytes. The producer supplies hex directly from
production world-map encoders over the explicitly qualified derived native room.
"""
import argparse
import hashlib
import json
from pathlib import Path

SPEC_SHA256 = "48b645d0d99acc8e7424ab44407ce43fa0370bf4108785b7433941c5f8476c86"
PAYLOADS = ("closed_snapshot_hex", "open_delta_hex")
MAX_PAYLOAD_BYTES = 1_048_576


def materialize(capture):
    source = capture.get("source")
    if not isinstance(source, dict) or source.get("derived_spec_sha256") != SPEC_SHA256:
        raise ValueError("capture must identify the explicit derived native room spec SHA-256")
    if source.get("kind") != "PumpkinPortal" or source.get("layout") != "DERIVED_NATIVE_ROOM" or source.get("source_item") != 1949:
        raise ValueError("source-kind/layout/portal qualification")
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
    output = {"schema": "OTERYN_NATIVE_PUMPKIN_PORTAL_PLAY/v1", "source": source}
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
    claim = capture.get("claim_presentation")
    if claim is not None:
        if claim.get("fresh_speech") != "Happy Halloween!" or claim.get("effect_id") != 28 or claim.get("replay_cues") != 0:
            raise ValueError("actual source claim speech/effect/replay contract")
        runtime = claim["observer_runtime"]
        native = claim["observer_native"]
        if any(type(runtime.get(k)) is not int for k in ("x", "y", "floor")) or not 0 <= runtime["floor"] <= 15 or native != dict(x=runtime["x"], y=runtime["y"], floor=-runtime["floor"]):
            raise ValueError("independent current reward-room viewer")
        actor = claim["observer_actor_hex"]
        if len(actor) != 32 or bytes.fromhex(actor).hex() != actor:
            raise ValueError("actual claim observer")
        selected = claim["selected_capabilities"]
        if not isinstance(selected, list) or any(type(bit) is not int or bit <= 0 for bit in selected) or sorted(set(selected)) != selected:
            raise ValueError("actual selected capability closure")
        claim = dict(claim)
        for group in ("snapshots", "deltas", "frames", "wire_frames"):
            rows = claim[group]
            if not isinstance(rows, list) or len(rows) > 1024:
                raise ValueError("bounded actual claim record list")
            sealed = []
            for row in rows:
                row = dict(row)
                for key, value in tuple(row.items()):
                    if key.endswith("_hex"):
                        if not isinstance(value, str) or len(value) > MAX_PAYLOAD_BYTES * 2:
                            raise ValueError("bounded actual claim payload")
                        raw = bytes.fromhex(value)
                        if not raw and key != "payload_hex":
                            raise ValueError("empty actual claim record")
                        row[key] = raw.hex()
                        row[key + "_sha256"] = hashlib.sha256(raw).hexdigest()
                sealed.append(row)
            claim[group] = sealed
        if not {9, 12, 17}.issubset({r["domain"] for r in claim["snapshots"]}) or not any(r["domain"] == 9 for r in claim["deltas"]):
            raise ValueError("actual claim inventory/chat/map bootstrap and inventory delta")
        for key in ("knife_definition_ref", "original_command_id"):
            if type(claim[key]) is not int or claim[key] <= 0:
                raise ValueError("actual claim source/reference")
        reward = claim["claimed_reward"]
        if any(type(reward[k]) is not int or reward[k] <= 0 for k in ("item_definition_ref", "count", "source_item_id")):
            raise ValueError("actual native minted reward")
        reply = bytes.fromhex(claim["use_result_hex"])
        if not reply:
            raise ValueError("actual committed use result")
        claim["use_result_hex_sha256"] = hashlib.sha256(reply).hexdigest()
        output["claim_presentation"] = claim
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
            raise SystemExit("native Pumpkin portal fixture is stale")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(result, encoding="utf-8")
    print(f"native Pumpkin portal fixture {'verified' if args.check else 'generated'}: {args.output}")


if __name__ == "__main__":
    main()
