"""Market-true v2: the unchanged v1 qualification with the D289 Native core holds applied."""

import argparse
import json

import d289_holds
import lower_client_market_packet as v1

ROOT = v1.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_client_market_d289_packet.py"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261003-item-market-true-promotion-v2.json"


def build(root=ROOT):
    # Every v1 source, scope and count guard (5120 records, 4893 qualified rows) runs first.
    packet = v1.build(root)
    rows, holds = [], list(packet["holds"])
    for row in packet["promotions"]:
        if row["item_key"] in d289_holds.NATIVE_CORE_HOLD_KEYS:
            holds.append(
                {
                    "item_key": row["item_key"],
                    "object_sha256": row["object_sha256"],
                    "reasons": [d289_holds.NATIVE_CORE_HOLD],
                    "source_item_id": row["source_item_id"],
                }
            )
        else:
            rows.append(row)
    d289_holds.require_hits(
        d289_holds.NATIVE_CORE_HOLD_KEYS,
        [h["item_key"] for h in holds if d289_holds.NATIVE_CORE_HOLD in h["reasons"]],
        "Market",
    )
    if len(rows) != 4892 or len(holds) != 228:
        raise ValueError(f"closed Market D289 scope drift: {len(rows)}/{len(holds)}")
    return packet | {
        "compiler": {
            "path": COMPILER,
            "sha256": v1.sha((root / COMPILER).read_bytes()),
        },
        "base_compiler": packet["compiler"],
        "decision": d289_holds.DECISION,
        "counts": {"promotions": len(rows), "holds": len(holds)},
        "promotions": rows,
        "holds": holds,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build()
    data = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("Market D289 packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
