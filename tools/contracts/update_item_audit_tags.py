#!/usr/bin/env python3
"""Refresh the native one-item operation tag golden from its prost declarations."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "crates/gameplay/durability/src/durability/item_mint_audit.rs"
block = SOURCE.read_text().split("pub enum OneItemOperationV1 {", 1)[1].split(
    "// End tag-sorted variants.", 1
)[0]
tags = [int(tag) for tag in re.findall(r'#\[prost\(message, tag = "(\d+)"\)\]', block)]
if tags != sorted(set(tags)):
    raise SystemExit("operation tags must be sorted and unique")
SOURCE.with_suffix(".tags").write_text("".join(f"{tag}\n" for tag in tags))


def varint(value):
    result = bytearray()
    while value >= 128:
        result.append((value & 127) | 128)
        value >>= 7
    result.append(value)
    return result


# Additive WRITE-0 fixture: revisions 129 -> 130, all other fields absent.
# Existing captured fixtures remain unchanged.
write_once_tag = int(re.search(
    r'#\[prost\(message, tag = "(\d+)"\)\]\s+WriteOnce\(', block
).group(1))
body = varint(5 << 3) + varint(129) + varint(6 << 3) + varint(130)
wire = varint(1 << 3) + varint(1) + varint((write_once_tag << 3) | 2) + varint(len(body)) + body
(SOURCE.parent / "item_mint_audit/tests/write_once_wire.hex").write_text(wire.hex() + "\n")

# The audit architecture test inventories the same production-only source roots.
# Keep this explicit artifact separate from runtime admission and preserve the
# test's tuple-binding and no-public-constructor assertions.
roots = [ROOT / 'apps/game-server/src'] + [
    ROOT / f'crates/gameplay/{package}/src'
    for package in ('foundation', 'content', 'durability')
]
sites = []
for path in sorted(path for root in roots for path in root.rglob('*.rs')):
    text = path.read_text()
    for marker in re.finditer(r'\n#\[cfg\(test\)\]\n', text):
        line = next((line.strip() for line in text[marker.end():].splitlines()
                     if not line.strip().startswith('#')), '')
        if 'mod ' in line:
            text = text[:marker.start()]
            break
    if 'INSERT INTO game_item_audit_outbox(' in text:
        sites.append(path.relative_to(ROOT).as_posix())
import json
(ROOT / 'tools/contracts/item_type2_insert_sites.json').write_text(
    json.dumps(sites, indent=2) + '\n')
print(f'Type2 insert-site inventory regenerated: {len(sites)} source files')
