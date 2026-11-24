"""Replay the Crystal 201189f8 NPC price audit against canonical service declarations."""
import argparse
import collections
import csv
import json

from questpc7_world_seal import refresh
from world_project_v2_to_tree import ROOT, git_blob_sha, write_services

AUDIT = ROOT / "imports/crystalserver/facts/npc-prices-201189f8.tsv"
WORLD = ROOT / "content/world"
DECLARATIONS = WORLD / "definitions/declarations.json"
# Current-era TibiaWiki fares checked against the April 25, 2026 revisions.
CURRENT_FARES = {
    "captain_breezelda": (
        "https://tibia.fandom.com/wiki/Captain_Breezelda",
        {"carlin": 110, "thais": 180},
    ),
    "captain_pelagia": (
        "https://tibia.fandom.com/wiki/Captain_Pelagia",
        {"darashia": 110, "edron": 100, "issavi": 70, "oramond": 60, "venore": 110},
    ),
    # The Dwarven Ring waives the 50 gold fare; zero is not the base fare.
    "buddel": (
        "https://tibia.fandom.com/wiki/Buddel",
        dict.fromkeys(("camp", "helheim", "okolnir", "tyrsung"), 50),
    ),
}
# Crystal reuses these client IDs for differently named variants; these prices
# select the named native Item, or the empty vial rather than milk (count 9).
NAMED_VARIANTS = {
    2998: ("rocking horse kit", 30),
    30061: ("giant sapphire", 50000),
    22057: ("werebear fur", 185),
    36820: ("bashmu fang", 600),
    2874: ("vial", 20),
    25696: ("colourful snail shell", 250),
    25697: ("green bandage", 180),
}
TRAVEL = ROOT / "crates/gameplay/content/src/content/npc_catalogue/travel_routes.json"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    document = json.loads(DECLARATIONS.read_bytes())
    overlays = json.loads(TRAVEL.read_bytes())
    services = {record["identity"]["key"]: record for record in document["records"]}
    native_routes = {route["key"]: route for route in overlays}
    counts = collections.Counter()
    seen = set()
    with AUDIT.open(newline="") as source:
        for row in csv.DictReader(source, delimiter="\t"):
            kind, service, key = (row[field] for field in ("type", "service", "offer_or_route"))
            evidence = CURRENT_FARES.get(service.removeprefix("oteryn:service.travel."))
            retained = evidence is not None and kind == "travel" and key in evidence[1]
            variant = None
            if kind == "trade" and row["status"] == "ambiguous":
                item_id = int(key.split("/", 1)[0].removeprefix("oteryn:item.tibia.i"))
                variant = NAMED_VARIANTS.get(item_id)
                retained = variant is not None
            if row["status"] == "ambiguous" and not retained:
                counts[kind, "ambiguous"] += 1
                continue
            if (row["status"] != "different" and not retained) or (service, key) in seen:
                raise ValueError(f"Unexpected or repeated price audit row: {service}/{key}")
            seen.add((service, key))
            record = services[service]
            if kind == "trade":
                item, side = key.split("/")
                match, = [
                    offer for offer in record["offers"]
                    if offer["item"]["key"] == item and offer["direction"] == side
                ]
                targets = [match]
                price = "unit_price"
            elif kind == "travel":
                match, = [route for route in record["routes"] if route["key"] == key]
                targets = [match]
                overlay_key = f"{service}/{key}"
                if overlay_key in native_routes or not retained:
                    targets.append(native_routes[overlay_key])
                price = "price"
            else:
                raise ValueError(f"Unknown audit family: {kind}")
            old = int(row["content_gold"])
            donor_prices = {int(value) for value in row["donor_gold"].split(",")}
            if variant is not None:
                new = variant[1]
                if new not in donor_prices:
                    raise ValueError(f"Named variant price absent from audit: {service}/{key}")
            elif retained:
                new = evidence[1][key]
            else:
                new = int(row["donor_gold"])
            if any(target[price] not in (donor_prices | {old, new}) for target in targets):
                raise ValueError(f"Price has drifted: {service}/{key}")
            differs = any(target[price] != new for target in targets)
            status = "different" if differs else (
                "variant-supported" if variant else "wiki-supported" if retained else "matched"
            )
            counts[kind, status] += 1
            for target in targets:
                target[price] = new
    if args.write:
        DECLARATIONS.write_text(json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n")
        TRAVEL.write_text(json.dumps(overlays, ensure_ascii=False, indent=2) + "\n")
        refresh(ROOT, WORLD)
        write_services(document, git_blob_sha(DECLARATIONS))
    for (kind, status), count in sorted(counts.items()):
        print(f"{kind} {status} {count}")
    return int(not args.write and any(status == "different" for _, status in counts))


if __name__ == "__main__":
    raise SystemExit(main())
