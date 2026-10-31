"""Market parity with Crystal: every defined Item whose Crystal appearance has the market flag.

The accepted owner rule is that Crystal's market list wins over Canary and Wiki observations.
Crystal's truth is `flags.market` in `data/items/appearances.dat` at the pinned revision. This
packet is the successor of the Market-true v2 packet: it promotes `trade_restrictions.marketable`
for each Crystal market id that has an Oteryn Item definition and that v2 held back (name
conflict, Wiki opposition, missing binding). Ids that cannot be promoted (no Oteryn definition,
sealed definition, pinned known-false) are recorded as exceptions with their reason.

The Crystal checkout is an input of this generator only (`--donor`); the packet carries the full
Crystal market id list so the parity test needs no donor.
"""

import argparse
import json
from pathlib import Path

import d289_holds
from engine_items import decode_appearance_object, protobuf_fields
from lower_wiki_stack_default_packet import sha

ROOT = Path(__file__).resolve().parents[3]
COMPILER = (
    "tools/content-schema/item-authoring/lower_client_market_crystal_parity_packet.py"
)
OUTPUT = (
    ROOT / "docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json"
)
V2 = "docs/agents/evidence/OTV2-20261003-item-market-true-promotion-v2.json"
DONOR_REPOSITORY = "zimbadev/crystalserver"
DONOR_REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
DONOR_PATH = "data/items/appearances.dat"
DONOR_SHA = "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
NO_DEFINITION = "NO_OTERYN_ITEM_DEFINITION"
# D289/D310: these Items' accepted definitions are digest-sealed by other evidence (the quest
# reward admission and the reward stack proof). A marketable fact would break the seal, so they
# stay as they are until the owner re-seals them.
SEALED = "SEALED_NATIVE_STATE_D289_D310"
SEALED_KEYS = d289_holds.NATIVE_CORE_HOLD_KEYS | d289_holds.SEALED_STATE_MARKET_KEYS
# i3321 carries an explicit Wiki `marketable: false` that Crystal contradicts. The Market-true v2
# hold list reads that false, and the sealed successor8 current-parent receipt pins the v2 bytes,
# so flipping it needs an owner re-seal of that receipt.
KNOWN_FALSE_PINNED = "KNOWN_FALSE_PINNED_BY_MARKET_V2_RECEIPT"
KNOWN_FALSE_KEYS = frozenset({"oteryn:item.tibia.i3321"})


def item_key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def defined_keys(root):
    index = json.loads((root / "content/items/index.json").read_text())
    return {
        r["definition"]["identity"]["key"]
        for shard in index["shards"]
        for r in json.loads((root / shard).read_text())["records"]
    }


def crystal_market_objects(data):
    if sha(data) != DONOR_SHA:
        raise ValueError("Crystal appearances.dat digest drift")
    found = {}
    for tag, raw in protobuf_fields(data):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in found:
                raise ValueError("duplicate Crystal appearance identity")
            if obj["flags"].get("flags.market") is True:
                found[obj["id"]] = sha(raw)
    return found


def build(data, root=ROOT):
    market = crystal_market_objects(data)
    v2 = json.loads((root / V2).read_bytes())
    promoted = {r["item_key"] for r in v2["promotions"]}
    defined = defined_keys(root)
    rows, exceptions = [], []
    for item_id in sorted(market):
        key = item_key(item_id)
        if key in promoted:
            continue
        if key not in defined:
            exceptions.append({"source_item_id": item_id, "reason": NO_DEFINITION})
            continue
        if key in SEALED_KEYS:
            exceptions.append({"source_item_id": item_id, "reason": SEALED})
            continue
        if key in KNOWN_FALSE_KEYS:
            exceptions.append({"source_item_id": item_id, "reason": KNOWN_FALSE_PINNED})
            continue
        rows.append(
            {
                "source_item_id": item_id,
                "item_key": key,
                "object_sha256": market[item_id],
                "marketable": True,
            }
        )
    return {
        "schema": "OTERYN_ITEM_MARKET_CRYSTAL_PARITY/v1",
        "compiler": {"path": COMPILER, "sha256": sha((root / COMPILER).read_bytes())},
        "donor": {
            "repository": DONOR_REPOSITORY,
            "revision": DONOR_REVISION,
            "path": DONOR_PATH,
            "sha256": DONOR_SHA,
            "flag": "flags.market",
        },
        "base_packet": {"path": V2, "promotions": len(promoted)},
        "counts": {
            "crystal_market_ids": len(market),
            "promotions": len(rows),
            "exceptions": len(exceptions),
        },
        "crystal_market_ids": sorted(market),
        "promotions": rows,
        "exceptions": exceptions,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--donor", required=True, type=Path, help="Crystal checkout")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build((args.donor / DONOR_PATH).read_bytes())
    data = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("Market Crystal parity packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]), sha(data))


if __name__ == "__main__":
    main()
