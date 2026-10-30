#!/usr/bin/env python3
"""Build the runtime Store catalogue (rulesets/economy/store/catalogue.json).

Offers come from the pinned Canary GameStore catalogue. Only item offers whose Item is admitted
for the Character inventory as one non-stackable instance, with default charges equal to the
offer's charges, are admitted: the native Inbox mint delivers exactly one such instance.
TibiaWiki (Exercise Weapons, Tibia Coins) agrees with every admitted price and charge count.

  python build_runtime_catalogue.py --canary <canary@52550d5e>   # regenerate
  python build_runtime_catalogue.py --check                      # admission against content
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "rulesets/economy/store/catalogue.json"
PROFILES = ROOT / "content/items/definitions/spell-native-profiles.json"
CANARY_COMMIT = "52550d5e"
SOURCE = "data/modules/scripts/gamestore/catalog/consumables_exercise_weapons.lua"
CATEGORY = "consumables.exercise-weapons"
REVISION = "store-catalogue-r1"


def admitted_items() -> dict[str, tuple[str, int]]:
    """Item key -> (revision, default charges) for admitted non-stackable inventory items."""
    records = json.loads(PROFILES.read_text(encoding="utf-8"))["records"]
    items = {}
    for record in records:
        admission = record.get("admission") or {}
        charges = record["semantics"].get("charges", {})
        if (
            admission.get("materializable")
            and admission.get("stack_class") == "NonStackable"
            and "CharacterInventory" in admission.get("legal_destinations", [])
            and charges.get("state") == "KNOWN"
            and charges["value"]["count"].get("state") == "KNOWN"
        ):
            item = record["authoring"]["item"]
            items[item["key"]] = (item["revision"], charges["value"]["count"]["value"])
    return items


def parse_offers(lua: str) -> list[dict]:
    """The offer blocks of one catalogue file, with the line of each offer's name."""
    offers = []
    lines = lua.split("\n")
    for number, line in enumerate(lines, 1):
        name = re.match(r'\s*name = "([^"]+)",$', line)
        if not name or number < 8:
            continue
        block = "\n".join(lines[number - 1 : number + 6])
        fields = {k: int(v) for k, v in re.findall(r"(price|itemtype|charges) = (\d+),", block)}
        if {"price", "itemtype", "charges"} <= fields.keys():
            offers.append({"name": name.group(1), "line": number, **fields})
    return offers


def build(lua: str) -> dict:
    items = admitted_items()
    offers = []
    for offer in parse_offers(lua):
        key = f"oteryn:item.tibia.i{offer['itemtype']}"
        admitted = items.get(key)
        if admitted is None or admitted[1] != offer["charges"]:
            continue
        offers.append(
            {
                "key": "store:" + offer["name"].lower().replace(" ", "-"),
                "category": CATEGORY,
                "title": offer["name"],
                "price": offer["price"],
                "currency": "TIBIA_COIN",
                "item": {"key": key, "revision": admitted[0], "charges": offer["charges"]},
                "source_line": offer["line"],
            }
        )
    offers.sort(key=lambda offer: offer["key"])
    return {
        "schema": "OTERYN_STORE_RUNTIME_CATALOGUE/v1",
        "catalog_revision": REVISION,
        "source": {"repository": "opentibiabr/canary", "commit": CANARY_COMMIT, "path": SOURCE},
        "offers": offers,
    }


def check() -> list[str]:
    catalogue = json.loads(OUT.read_text(encoding="utf-8"))
    items = admitted_items()
    errors = []
    keys = [offer["key"] for offer in catalogue["offers"]]
    if keys != sorted(set(keys)) or not keys:
        errors.append("offer keys are not unique and sorted")
    for offer in catalogue["offers"]:
        item = offer["item"]
        admitted = items.get(item["key"])
        if admitted != (item["revision"], item["charges"]):
            errors.append(f"{offer['key']}: item not admitted with these charges")
        if not 0 < offer["price"] <= 100_000 or offer["currency"] != "TIBIA_COIN":
            errors.append(f"{offer['key']}: price or currency")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--canary", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.canary:
        lua = (args.canary / SOURCE).read_text(encoding="utf-8")
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(build(lua), indent=2) + "\n", encoding="utf-8")
    errors = check()
    for error in errors:
        print(error, file=sys.stderr)
    print("FAIL" if errors else "PASS")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
