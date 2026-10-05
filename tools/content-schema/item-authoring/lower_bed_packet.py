"""Lower the BED-CONTENT-1 bed facts (BED-0 §3) from Canary `items.xml` into one evidence packet.

Canary (D384 pin, OTS_HYPOTHESIS_ONLY) is the only source. Per `type="bed"` Item the packet
carries `part` (`bedpart`: `pillow` = head, `blanket` = foot), `partner_direction`
(`partnerdirection`), `partner_item_key` (`bedpartof`) and the raw `male_transform_to` /
`female_transform_to` targets. Item names stay in the definitions; this tool adds no name.

It does not classify an Item as the free or the occupied form. Canary's transform links are
mutual (the free form names the occupied form and the occupied form names the free one back), so
a derivation from the XML alone is a guess; `free_type` and the per-sex occupied types need an
architect rule (ITEM-SEM-BED-1) or the placed map. A fact the source does not state is `null`,
and the Item is listed under `holds`.

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
TRANSFORMS = (
    ("male_transform_to", "maletransformto"),
    ("female_transform_to", "femaletransformto"),
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
        partner = _item_id(attrs.get("bedpartof"))
        reasons = []
        if part is None:
            reasons.append("bedpart_unknown")
        if direction not in DIRECTIONS:
            direction = None
            reasons.append("partnerdirection_unknown")
        if partner is None:
            reasons.append("bedpartof_missing")
        elif partner not in beds:
            reasons.append("partner_not_a_bed_item")
        row = {
            "item_key": _key(item_id),
            "part": part,
            "partner_direction": direction,
            "partner_item_key": _key(partner) if partner is not None else None,
        }
        for field, attr in TRANSFORMS:
            target = _item_id(attrs.get(attr))
            row[field] = _key(target) if target is not None else None
            if target is not None and target not in beds:
                reasons.append(f"{field}_not_a_bed_item")
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
