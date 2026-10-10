"""Lower the BED-CONTENT-1 bed facts (BED-0 §3, group 19) from Canary `items.xml` into one packet.

Rule: ITEM-SEM-BED-PACKET-1 §1.5. Canary (D384 pin, OTS_HYPOTHESIS_ONLY) is the only source. Per
`type="bed"` Item: `part` (`bedpart`: `pillow` = Head, `blanket` = Foot), `partner_direction`
(`partnerdirection`) and `occupied_male` / `occupied_female` (`<sex>transformto`, falling back to
the other sex's key only when this one is absent). The engine shows a target only when it is a
`type="bed"` Item, so a target that is `0` or not a bed lowers to the record's own Item ("no
change") and is listed under `no_change_targets` with its source id and source type. The free look
is the placed Item and `bedpartof` is not read.

An Item is held, gets no group 19 and is listed under `holds` by reason when its part or direction
is missing or unknown, when it has no content definition, or when a target has another part,
another direction or no group 19 (the set rule, to a fixed point). The packet is applied by
`item_bed_promotion.rs`; `--check` rebuilds it in memory and fails on any byte difference.
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
SCHEMA = "OTERYN_ITEM_BED_PROMOTION/v1"
PARTS = {"pillow": "HEAD", "blanket": "FOOT"}
DIRECTIONS = {"north": "NORTH", "east": "EAST", "south": "SOUTH", "west": "WEST"}
SEXES = (("occupied_male", "maletransformto"), ("occupied_female", "femaletransformto"))


def _key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def _number(raw):
    return int(raw) if raw is not None and raw.isdigit() else None


def build(canary, content_ids):
    """(promotions, holds, no_change_targets, counts) for the Canary `type="bed"` Items."""
    beds = {i: a for i, a in canary.items() if a.get("type") == "bed"}
    held, no_change = {}, []
    facts = {}
    for item_id in sorted(beds):
        attrs = beds[item_id]
        part = PARTS.get(attrs.get("bedpart"))
        direction = DIRECTIONS.get(attrs.get("partnerdirection"))
        reasons = []
        if part is None:
            reasons.append("bedpart_unknown")
        if direction is None:
            reasons.append("partnerdirection_unknown")
        if item_id not in content_ids:
            reasons.append("no_content_definition")
        if reasons:
            held[item_id] = reasons
            continue
        targets = {}
        for field, attr in SEXES:
            other = dict(SEXES)[
                ("occupied_female" if field == "occupied_male" else "occupied_male")
            ]
            raw = attrs.get(attr)
            if raw is None:
                raw = attrs.get(other)
            target = _number(raw)
            if target is not None and beds.get(target) is not None:
                targets[field] = target
            else:
                targets[field] = item_id
                if raw is not None:
                    no_change.append(
                        {
                            "item_key": _key(item_id),
                            "field": field,
                            "source_target": raw,
                            "source_type": canary.get(target, {}).get("type")
                            if target
                            else None,
                        }
                    )
        facts[item_id] = (part, direction, targets)
    changed = True
    while changed:  # the set rule, to a fixed point: a held target holds its sources
        changed = False
        for item_id, (part, direction, targets) in sorted(facts.items()):
            for target in targets.values():
                if target == item_id:
                    continue
                if (
                    target in held
                    or target in facts
                    and facts[target][:2] != (part, direction)
                ):
                    held[item_id] = ["target_part_or_direction_differs"]
                    changed = True
                elif target not in facts:
                    held[item_id] = ["target_not_lowered"]
                    changed = True
                if item_id in held:
                    break
            if item_id in held:
                del facts[item_id]
                break
    promotions = [
        {
            "item_key": _key(item_id),
            "field_path": "bed",
            "typed_value": {
                "kind": "BED",
                "value": {
                    "part": part,
                    "partner_direction": direction,
                    "occupied_male": _key(targets["occupied_male"]),
                    "occupied_female": _key(targets["occupied_female"]),
                },
            },
        }
        for item_id, (part, direction, targets) in sorted(facts.items())
    ]
    holds = [{"item_key": _key(i), "reasons": r} for i, r in sorted(held.items())]
    no_change = [r for r in no_change if int(r["item_key"].rsplit(".i", 1)[1]) in facts]
    counts = Counter(
        {
            "bed_types": len(beds),
            "items": len(promotions),
            "fields": len(promotions),
            "holds": len(holds),
            "no_change_targets": len(no_change),
        }
    )
    return promotions, holds, no_change, dict(sorted(counts.items()))


def packet_bytes(canary, content_ids, pin):
    promotions, holds, no_change, counts = build(canary, content_ids)
    packet = {
        "schema": SCHEMA,
        "decision": "ITEM-SEM-BED-PACKET-1 §1.5",
        "task_id": "OTV2-20261004-bed-content-1",
        "source": pin,
        "counts": counts,
        "promotions": promotions,
        "holds": holds,
        "no_change_targets": no_change,
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
