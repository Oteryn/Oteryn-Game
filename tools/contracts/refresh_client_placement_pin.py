#!/usr/bin/env python3
"""Validate the generated placement index, then refresh its client SHA pin.

Run after the official World placement producers, never on an unresolved merge:
    python3 tools/contracts/refresh_client_placement_pin.py
Use --check to validate without writing. This tool changes only the one derived
Rust constant; it does not regenerate placements, asset manifests or World pins.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re

ROOT = Path(__file__).resolve().parents[2]
INDEX = "content/world/placements/index.json"
CONSUMER = "crates/client-assets/src/world.rs"
MAX_INDEX_BYTES = 8 * 1024 * 1024
MAX_REGION_BYTES = 32 * 1024 * 1024
PIN = re.compile(r'(pub const PLACEMENT_INDEX_SHA256: &str =\s*")[0-9a-f]{64}(";)')


def checked_path(root: Path, relative: str) -> Path:
    path = root / relative
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"path leaves repository: {relative}")
    return path


def placement_digest(root: Path) -> str:
    path = checked_path(root, INDEX)
    with path.open("rb") as stream:
        raw = stream.read(MAX_INDEX_BYTES + 1)
    if len(raw) > MAX_INDEX_BYTES:
        raise ValueError("placement index exceeds client byte bound")
    document = json.loads(raw)
    if (not isinstance(document, dict) or document.get("schema") != "OTERYN_FAMILY_INDEX/v1"
            or document.get("family") != "WorldPlacement.Base"
            or document.get("codec") != "OTERYN_WORLD_REGION_B3/v1"):
        raise ValueError("unsupported placement index schema/family/codec")
    palette, regions = document.get("palette"), document.get("regions")
    if not isinstance(palette, list) or not palette or not isinstance(regions, list) or not regions:
        raise ValueError("placement index requires palette and regions")
    for item in palette:
        value = item.get("source_item_id") if isinstance(item, dict) else None
        if type(value) is not int or not 0 <= value <= 0xFFFFFFFF:
            raise ValueError("placement palette source_item_id is not u32")
    seen = set()
    for region in regions:
        if not isinstance(region, dict):
            raise ValueError("invalid pinned region record")
        name, expected = region.get("path"), region.get("sha256")
        if (not isinstance(name, str) or not name.startswith("content/world/placements/")
                or not name.endswith(".b3") or ".." in PurePosixPath(name).parts
                or name in seen or not isinstance(expected, str)
                or re.fullmatch(r"[0-9a-f]{64}", expected) is None):
            raise ValueError("invalid or duplicate pinned region")
        seen.add(name)
        digest, size = hashlib.sha256(), 0
        with checked_path(root, name).open("rb") as stream:
            while chunk := stream.read(64 * 1024):
                size += len(chunk)
                if size > MAX_REGION_BYTES:
                    raise ValueError(f"region exceeds client byte bound: {name}")
                digest.update(chunk)
        if digest.hexdigest() != expected:
            raise ValueError(f"region SHA mismatch: {name}")
    return hashlib.sha256(raw).hexdigest()


def refresh(root: Path, check: bool = False) -> str:
    digest = placement_digest(root)
    path = checked_path(root, CONSUMER)
    source = path.read_text(encoding="utf-8")
    updated, count = PIN.subn(lambda match: match[1] + digest + match[2], source)
    if count != 1:
        raise ValueError("expected exactly one client placement pin declaration")
    if check and updated != source:
        raise ValueError(f"stale client placement pin; regenerate with {Path(__file__).name}")
    if not check and updated != source:
        path.write_text(updated, encoding="utf-8")
    return digest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        digest = refresh(ROOT, args.check)
    except (OSError, ValueError, TypeError, AttributeError) as error:
        parser.exit(1, f"client placement pin: {error}\n")
    print(f"client placement pin {'checked' if args.check else 'refreshed'}: {digest}")


if __name__ == "__main__":
    main()
