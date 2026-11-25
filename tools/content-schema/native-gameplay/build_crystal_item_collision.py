#!/usr/bin/env python3
"""Project Crystal's item-loader collision flags into an indexed byte per item id."""

import argparse
import hashlib
from pathlib import Path

from build_source_world import fields, policies

# Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data/items.
SOURCE_SHA256 = {
    "appearances.dat": "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
    "items.xml": "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
}


def load_source(directory, name):
    data = (directory / name).read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256[name]:
        raise ValueError(f"Crystal source digest mismatch: {name}")
    return data


def project(directory):
    appearances = load_source(directory, "appearances.dat")
    xml = load_source(directory, "items.xml")
    ids = {fields(record)[1][0] for record in fields(appearances)[1]}
    flags = bytearray(max(ids) + 1)
    for item in policies(appearances, xml, ids):
        value = 1  # Source id is present, even if all flags are false.
        for bit, flag in enumerate(("block_solid", "block_projectile", "block_pathfind"), 1):
            value |= int(item[flag]) << bit
        value |= int(item["dynamic_kind"] == "door") << 4
        flags[item["server_item_id"]] = value
    return flags


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Crystal data/items directory")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.write_bytes(project(args.source))


if __name__ == "__main__":
    main()
