#!/usr/bin/env python3
"""Capture the exact source-pinned Magnolia record for owning-schema comparison."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
KEY = "oteryn:creature.the_moonsnow_magnolia"
EXPECTED = "4dce44039198d58531d9f59868e4d35ea54dfd3c7aed33c204438b0a716a15ca"
SOURCE = ROOT / "content/creatures/definitions/spell-native-profiles.json"
OUTPUT = ROOT / "tools/content-schema/native-gameplay/magnolia-source-native-record.json"


def captured_bytes():
    document = json.loads(SOURCE.read_text())
    records = [row for row in document["records"] if row["profile"]["target"]["key"] == KEY]
    if len(records) != 1:
        raise ValueError("Magnolia source record must be unique")
    record = records[0]
    canonical = json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    if hashlib.sha256(canonical).hexdigest() != EXPECTED:
        raise ValueError("Pinned Magnolia source record changed; recapture requires source review")
    return (json.dumps(record, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = captured_bytes()
    if args.check:
        if OUTPUT.read_bytes() != expected:
            raise SystemExit("Magnolia source-record seal is stale")
    else:
        OUTPUT.write_bytes(expected)
    print("Magnolia source-record seal: " + EXPECTED)


if __name__ == "__main__":
    main()
