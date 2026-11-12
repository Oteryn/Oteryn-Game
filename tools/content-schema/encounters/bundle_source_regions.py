#!/usr/bin/env python3
"""Package independently pinned source documents for the native map carrier."""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = Path(__file__).resolve().parents[1] / "native-gameplay"
    channel = (source / "canary-thalom-world.json").read_bytes()
    primal = (source / "crystal-primal-region.json").read_bytes()
    if hashlib.sha256(channel).hexdigest() != "44f860d20712a32e6bc22aa269d3e220d39d164b7738d3c150a406579849b5f5":
        raise ValueError("unqualified channel source document")
    region = json.loads(primal)
    if region["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8" or region["map_git_blob"] != "e61658ec0ab7b70ec35025ae969426d6fc57ae75":
        raise ValueError("unqualified Primal source capture")
    result = json.dumps({"schema": "OTERYN_NATIVE_SOURCE_REGION_BUNDLE/v1", "channel": channel.decode(), "primal": primal.decode()}, sort_keys=True, separators=(",", ":")).encode() + b"\n"
    if len(result) > 8 * 1024 * 1024:
        raise ValueError("source carrier capacity")
    args.output.write_bytes(result)
    print(f"Source region bundle: {len(result)} bytes, sha256={hashlib.sha256(result).hexdigest()}")


if __name__ == "__main__":
    main()
