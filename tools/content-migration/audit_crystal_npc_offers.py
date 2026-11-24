"""Census literal buy/sell offers in the Crystal 201189f8 Global NPC scripts."""
import argparse
import collections
import csv
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DECLARATIONS = ROOT / "content/world/definitions/declarations.json"
FIELDS = re.compile(r"\b(clientid|buy|sell)\s*=\s*(\d+)\s*(?=[,}])", re.I)
TABLE_ROW = re.compile(r"\{[^{}]+\}")
DIRECTIONS = {"buy": "SellToPlayer", "sell": "BuyFromPlayer"}


def source_offers(root):
    offers = collections.defaultdict(list)
    scripts = set()
    for path in sorted((root / "data-global/npc").rglob("*.lua")):
        script = re.sub(r"[^a-z0-9]+", "_", path.stem.lower()).strip("_")
        scripts.add(script)
        for line_number, line in enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1):
            for row in TABLE_ROW.findall(line.split("--", 1)[0]):
                fields = {key.lower(): int(value) for key, value in FIELDS.findall(row)}
                if "clientid" not in fields:
                    continue
                for side, direction in DIRECTIONS.items():
                    if fields.get(side, 0) > 0:
                        offers[script, fields["clientid"], direction].append(
                            (fields[side], f"{path.relative_to(root)}:{line_number}")
                        )
    return offers, scripts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crystal", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    donor, scripts = source_offers(args.crystal)
    declarations = json.loads(DECLARATIONS.read_bytes())["records"]
    native = collections.defaultdict(list)
    item_keys = {
        row["definition"]["identity"]["key"]
        for shard in (ROOT / "content/items/definitions").glob("items-*.json")
        for row in json.loads(shard.read_bytes())["records"]
    }
    services = set()
    for row in declarations:
        service = row["identity"]["key"]
        if not service.startswith("oteryn:service.trade."):
            continue
        merchant = service.removeprefix("oteryn:service.trade.")
        services.add(merchant)
        for offer in row.get("offers", []):
            key = offer["item"]["key"]
            item = re.fullmatch(r"oteryn:item.tibia.i(\d+)", key)
            item_id = int(item[1]) if item else key
            native[merchant, item_id, offer["direction"]].append(offer["unit_price"])
    counts = collections.Counter()
    with args.output.open("w", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(("class", "merchant", "item", "direction", "oteryn_gold", "crystal_gold", "source"))
        for key in sorted(native.keys() | donor.keys(), key=str):
            merchant, item, side = key
            ours, theirs = native.get(key, []), donor.get(key, [])
            if ours and theirs:
                continue
            if ours:
                reason = "oteryn_only_no_named_donor_script" if merchant not in scripts else "oteryn_only_no_literal_match"
            elif merchant not in services:
                reason = "crystal_only_no_native_trade_service"
            elif f"oteryn:item.tibia.i{item}" not in item_keys:
                reason = "crystal_only_no_native_item"
            else:
                reason = "crystal_only_native_service_and_item"
            counts[reason] += 1
            writer.writerow((reason, merchant, item, side, ",".join(map(str, ours)),
                             ",".join(str(price) for price, _ in theirs),
                             ",".join(source for _, source in theirs)))
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
