#!/usr/bin/env python3
"""Pinned Crystal Thing order for existing native Item refs; no new definitions.

Publication is explicit (--write); default execution reports a dry-run digest.
Uses the existing activity-source appearance decoder and verifies whole donors.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re
import build_activity_source_world as activity

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SCHEMA = "OTERYN_NATIVE_ITEM_THING_ORDER/v1"
DEFAULT_KEYS = ROOT / "tools/content-schema/native-gameplay/item-keys.json"
DEFAULT_OUTPUT = Path(__file__).with_name("native-item-thing-orders.json")
SOURCES = {
    "data/items/appearances.dat": ("37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0", None),
    "src/items/items.cpp": ("bdb7b4b48aadfeb9e031311ca107aadb4ee6e30b", [203, 208]),
    "src/items/tile.cpp": ("54cc28b561209e3e0339b511a4325c81ae6f4e61", [1536, 1567]),
    "src/game/game.cpp": ("5f421b1b2361280abeb8b392bca0e0344fc04648", [2814, 2837]),
    "src/lua/creature/movement.cpp": ("eaea36a920514f5886fef3f5fd57270da3a5beee", [375, 405]),
    "src/protobuf/appearances.proto": ("7069b5ead685929c0a79cb6e8b0664bbe8f19a3a", [149, 151]),
}

def encode(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()

def inputs(cache, code):
    blobs = {}
    for path in SOURCES:
        candidate = code / path
        if not candidate.is_file():
            candidate = cache / Path(path).name
        blobs[path] = candidate.read_bytes()
    return blobs

def build(blobs, keys_raw):
    evidence = []
    for path, (expected, lines) in SOURCES.items():
        raw = blobs[path]
        blob = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()
        if blob != expected:
            raise ValueError("pinned Thing-order donor differs: " + path)
        row = {"path": path, "git_blob": blob, "sha256": hashlib.sha256(raw).hexdigest()}
        if lines is not None:
            row["lines"] = lines
        evidence.append(row)
    keys = json.loads(keys_raw)["records"]
    if not keys or len(keys) > 65535:
        raise ValueError("native Item index outside existing u16 source domain")
    ids = []
    for key, revision in keys:
        match = re.fullmatch(r"oteryn:item\.tibia\.i([0-9]+)", key)
        if not match or revision != "definition-r1":
            raise ValueError("native Item index lacks an exact pinned source identity")
        ids.append(int(match[1]))
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate native Item source identity")
    orders = activity.source_top_orders(blobs["data/items/appearances.dat"], set(ids))
    # Row n is existing Item definition-ref n+1; no new reference is allocated.
    records = [[item, orders[item]] for item in ids]
    return {"schema": SCHEMA, "source_revision": PIN, "sources": evidence,
            "native_item_index_sha256": hashlib.sha256(keys_raw).hexdigest(),
            "definition_revision": "definition-r1", "record_order": "native_item_definition_ref",
            "records": records}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-cache", type=Path, required=True)
    parser.add_argument("--source-code", type=Path, required=True)
    parser.add_argument("--keys", type=Path, default=DEFAULT_KEYS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    value = build(inputs(args.source_cache, args.source_code), args.keys.read_bytes())
    raw = encode(value)
    if args.check:
        if not args.output.is_file() or args.output.read_bytes() != raw:
            raise SystemExit("native Thing-order packet differs; regenerate with --write")
    elif args.write:
        args.output.write_bytes(raw)
    print(json.dumps({"records": len(value["records"]), "bytes": len(raw),
                      "sha256": hashlib.sha256(raw).hexdigest(),
                      "mode": "check" if args.check else "write" if args.write else "dry-run"}))

if __name__ == "__main__":
    main()
