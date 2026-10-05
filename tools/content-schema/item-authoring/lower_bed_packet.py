"""Lower the BED-CONTENT-1 bed facts (BED-0 §3, group 19 per #1847) from Canary `items.xml`.

Canary (D384 pin, OTS_HYPOTHESIS_ONLY) is the only source. Per `type="bed"` Item the packet
carries `part` (`bedpart`: `pillow` = head, `blanket` = foot), `partner_direction`
(`partnerdirection`), `occupied_male` and `occupied_female`. The placed Item is the free look;
halves pair by direction and `bedpartof` is not read. `occupied_<sex>` is the `<sex>transformto`
target, falling back to the other sex's key; a missing, 0 or non-bed target means no change
(`null`). Item names stay in the definitions; this tool adds no name.

A part or direction the source does not state is `null`, and the Item is listed under `holds`.
The packet is a candidate: no definition carries the `bed` group until ITEM-SEM-BED-1 admits it,
and no runtime reads these facts before BED-1. `--check` rebuilds the packet in memory and fails
on any byte difference.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from lower_equip_abilities_packet import canary_pin, load_canary_top_level
from lower_wiki_stats_packet import ROOT, content_item_ids

OUTPUT = ROOT / "docs" / "agents" / "evidence" / "OTV2-20261005-bed-facts-v1.json"
SCHEMA = "OTERYN_ITEM_BED_FACTS/v1"
PARTS = {"pillow": "head", "blanket": "foot"}
DIRECTIONS = {"north", "east", "south", "west"}
OCCUPIED = (
    ("occupied_male", "maletransformto"),
    ("occupied_female", "femaletransformto"),
)


def _key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def _item_id(raw):
    return int(raw) if raw is not None and raw.isdigit() else None


def build(canary, content_ids):
    """(rows, holds, counts) for every `type="bed"` Canary Item that has a content definition."""
    beds = {i: a for i, a in canary.items() if a.get("type") == "bed"}
    rows, holds, counts = [], [], Counter()
    for item_id in sorted(beds):
        attrs = beds[item_id]
        if item_id not in content_ids:
            counts["not_in_content"] += 1
            continue
        part = PARTS.get(attrs.get("bedpart"))
        direction = attrs.get("partnerdirection")
        reasons = []
        if part is None:
            reasons.append("bedpart_unknown")
        if direction not in DIRECTIONS:
            direction = None
            reasons.append("partnerdirection_unknown")
        targets = {}
        for field, attr in OCCUPIED:
            target = _item_id(attrs.get(attr))
            targets[field] = target if target in beds else None
        row = {"item_key": _key(item_id), "part": part, "partner_direction": direction}
        for field, other in zip(targets, reversed(list(targets))):
            chosen = targets[field] if targets[field] is not None else targets[other]
            row[field] = _key(chosen) if chosen is not None else None
        rows.append(row)
        counts[f"part_{part}"] += 1
        if reasons:
            holds.append({"item_key": row["item_key"], "reasons": sorted(reasons)})
    counts["rows"] = len(rows)
    counts["holds"] = len(holds)
    return rows, holds, dict(sorted(counts.items()))


def packet_bytes(canary, content_ids, pin):
    rows, holds, counts = build(canary, content_ids)
    packet = {
        "schema": SCHEMA,
        "decision": "BED-0 §3",
        "task_id": "OTV2-20261004-bed-content-1",
        "source": pin,
        "counts": counts,
        "rows": rows,
        "holds": holds,
    }
    return (json.dumps(packet, indent=2, sort_keys=True) + "\n").encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args(argv)
    data = packet_bytes(load_canary_top_level(), content_item_ids(), canary_pin())
    if args.check:
        if args.output.read_bytes() != data:
            print(f"packet drift against {args.output}", file=sys.stderr)
            return 1
        print(json.dumps({"check": "PASS", "bytes": len(data)}))
        return 0
    args.output.write_bytes(data)
    print(json.dumps({"counts": json.loads(data)["counts"], "bytes": len(data)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
