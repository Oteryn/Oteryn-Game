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
ITEM_NAME = re.compile(r'\bitemname\s*=\s*"([^"]+)"', re.I)
CONDITIONAL_FIELDS = re.compile(r"\b(?:count|subtype|actionid|storage)\s*=", re.I)
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
                            (
                                fields[side],
                                f"{path.relative_to(root)}:{line_number}",
                                ITEM_NAME.search(row)[1].casefold()
                                if ITEM_NAME.search(row) and not CONDITIONAL_FIELDS.search(row)
                                else None,
                            )
                        )
    return offers, scripts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crystal", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--import-exact", action="store_true",
                        help="admit uniquely named materializable offers to existing merchants")
    args = parser.parse_args()
    donor, scripts = source_offers(args.crystal)
    document = json.loads(DECLARATIONS.read_bytes())
    declarations = document["records"]
    native = collections.defaultdict(list)
    items = {
        row["definition"]["identity"]["key"]: row["definition"]
        for shard in (ROOT / "content/items/definitions").glob("items-*.json")
        for row in json.loads(shard.read_bytes())["records"]
    }
    services = {}
    for row in declarations:
        service = row["identity"]["key"]
        if not service.startswith("oteryn:service.trade."):
            continue
        merchant = service.removeprefix("oteryn:service.trade.")
        services[merchant] = row
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
            elif f"oteryn:item.tibia.i{item}" not in items:
                reason = "crystal_only_no_native_item"
            else:
                definition = items[f"oteryn:item.tibia.i{item}"]
                name = (definition.get("semantics", {}).get("presentation", {})
                        .get("value", {}).get("name", {}).get("value", ""))
                if not definition["materializable"]:
                    reason = "crystal_only_item_not_materializable"
                elif len(theirs) != 1 or not name or theirs[0][2] != name.casefold():
                    reason = "crystal_only_variant_or_name_mismatch"
                elif not services[merchant].get("offers"):
                    reason = "crystal_only_native_service_held"
                else:
                    reason = "crystal_only_exact_native_offer"
                    if args.import_exact:
                        services[merchant]["offers"].append({
                            "item": definition["identity"],
                            "direction": side,
                            "unit_price": theirs[0][0],
                        })
            counts[reason] += 1
            writer.writerow((reason, merchant, item, side, ",".join(map(str, ours)),
                             ",".join(str(price) for price, _, _ in theirs),
                             ",".join(source for _, source, _ in theirs)))
    if args.import_exact and counts["crystal_only_exact_native_offer"]:
        from questpc7_world_seal import refresh
        from world_project_v2_to_tree import git_blob_sha, write_services

        DECLARATIONS.write_text(json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n")
        refresh(ROOT, ROOT / "content/world")
        write_services(document, git_blob_sha(DECLARATIONS))
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
