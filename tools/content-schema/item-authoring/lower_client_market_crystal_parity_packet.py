"""Market parity with Crystal: every defined Item whose Crystal appearance has the market flag.

The accepted owner rule is that Crystal's market list wins over Canary and Wiki observations.
Crystal's truth is `flags.market` in `data/items/appearances.dat` at the pinned revision. This
packet is the successor of the Market-true v2 packet: it promotes `trade_restrictions.marketable`
for each Crystal market id that has an Oteryn Item definition and that v2 held back (name
conflict, Wiki opposition, missing binding). Ids that cannot be promoted (no Oteryn definition)
are recorded as exceptions with their reason. Owner decision 2026-10-31: the D289/D310 sealed
Items (i901, i3450) are re-sealed and promoted here, and a Wave 1 Wiki `marketable: false`
(i3321) is superseded by Crystal; the row says so, so the materializer may flip that one false.

The Crystal checkout is an input of this generator only (`--donor`); the packet carries the full
Crystal market id list so the parity test needs no donor.
"""

import argparse
import json
from pathlib import Path

from engine_items import decode_appearance_object, protobuf_fields
from lower_wiki_stack_default_packet import sha

ROOT = Path(__file__).resolve().parents[3]
COMPILER = (
    "tools/content-schema/item-authoring/lower_client_market_crystal_parity_packet.py"
)
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json"
V2 = "docs/agents/evidence/OTV2-20261003-item-market-true-promotion-v2.json"
DONOR_REPOSITORY = "zimbadev/crystalserver"
DONOR_REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
DONOR_PATH = "data/items/appearances.dat"
DONOR_SHA = "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
NO_DEFINITION = "NO_OTERYN_ITEM_DEFINITION"
WAVE1 = "imports/tibiawiki/facts/items-wave1.json"
SUPERSEDES_WAVE1_FALSE = "WAVE1_WIKI_MARKETABLE_FALSE"


def item_key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def wave1_false_keys(root):
    """Items whose Wave 1 Wiki fact is `marketable: false`; Crystal's market flag wins over it."""
    return {
        record["target"]["key"]
        for record in json.loads((root / WAVE1).read_text())["records"]
        for fact in record["definition_facts"]
        if fact["field_path"] == "trade_restrictions.marketable"
        and fact["value"] is False
    }


MINT = "docs/agents/evidence/OTV2-20261031-item-client-market-mint-v1.json"


def defined_keys(root):
    """Item keys in the content tree plus the client-only Items the E06 mint packet adds."""
    index = json.loads((root / "content/items/index.json").read_text())
    minted = json.loads((root / MINT).read_text())["records"]
    return {
        r["definition"]["identity"]["key"]
        for shard in index["shards"]
        for r in json.loads((root / shard).read_text())["records"]
    } | {r["definition"]["identity"]["key"] for r in minted}


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
    wave1_false = wave1_false_keys(root)
    rows, exceptions = [], []
    for item_id in sorted(market):
        key = item_key(item_id)
        if key in promoted:
            continue
        if key not in defined:
            exceptions.append({"source_item_id": item_id, "reason": NO_DEFINITION})
            continue
        row = {
            "source_item_id": item_id,
            "item_key": key,
            "object_sha256": market[item_id],
            "marketable": True,
        }
        if key in wave1_false:
            row["supersedes"] = SUPERSEDES_WAVE1_FALSE
        rows.append(row)
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
