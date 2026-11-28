"""Build the runtime Store catalogue (rulesets/economy/store/catalogue.json).

Offers come from pinned Canary and the retained Crystal charged-item extract.
The native Inbox mint delivers one non-stackable instance with the native definition's
default charges, so charged offers must agree with those defaults.

  python build_runtime_catalogue.py --canary <canary@52550d5e>   # regenerate
  python build_runtime_catalogue.py --check                      # admission against content
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import runpy
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
STORE = ROOT / "rulesets/economy/store"
OUT = STORE / "catalogue.json"
PLATFORM_OUT = OUT.with_name("platform-catalogue.json")
PROFILES = ROOT / "content/items/definitions/spell-native-profiles.json"
CANARY_COMMIT = "52550d5e"
SOURCE = "data/modules/scripts/gamestore/catalog/consumables_exercise_weapons.lua"
CATEGORY = "consumables.exercise-weapons"
PREY_EXPANSION = ROOT / "rulesets/economy/store/prey-list-expansion.json"
HUNTING_OFFERS = STORE / "hunting-offers.json"
BLESSINGS = ROOT / "rulesets/character/blessings/catalogue.json"
CRYSTAL_CHARGED = Path(__file__).with_name("crystal-charged-offers.json")
CONVERTER_VARIANTS = {"25719", "28526"}


def expansion_offer(config: dict) -> dict:
    """Owner-priced permanent Character entitlement; null is a disabled placeholder."""
    if config.get("schema") != "OTERYN_PREY_LIST_EXPANSION_OFFER/v1":
        raise ValueError("invalid Prey expansion offer schema")
    if (
        config.get("key") != "store:prey-list-expansion"
        or config.get("unlock_key") != "prey-list-expansion"
    ):
        raise ValueError("invalid Prey expansion identity")
    if config.get("currency") != "TIBIA_COIN" or not isinstance(
        config.get("enabled"), bool
    ):
        raise ValueError("invalid Prey expansion currency or enabled flag")
    price = config.get("price")
    if price is not None and (
        isinstance(price, bool)
        or not isinstance(price, int)
        or not 0 < price <= 100_000
    ):
        raise ValueError("invalid Prey expansion price")
    if config["enabled"] and (
        price is None or config.get("price_status") != "OWNER_APPROVED"
    ):
        raise ValueError("enabled Prey expansion needs an approved positive price")
    return {
        key: config[key]
        for key in (
            "key",
            "category",
            "title",
            "price",
            "enabled",
            "currency",
            "unlock_key",
        )
    } | {"source_line": 0}


def with_expansion(catalogue: dict) -> dict:
    config = json.loads(PREY_EXPANSION.read_text(encoding="utf-8"))
    offer = expansion_offer(config)
    offers = [row for row in catalogue["offers"] if row["key"] != offer["key"]]
    offers.append(offer)
    offers.sort(key=lambda row: row["key"])
    return catalogue | {"offers": offers}


def with_hunting_offers(catalogue: dict) -> dict:
    authored = json.loads(HUNTING_OFFERS.read_bytes())
    keys = {row["key"] for row in authored}
    offers = [row for row in catalogue["offers"] if row["key"] not in keys]
    return catalogue | {"offers": sorted(offers + authored, key=lambda row: row["key"])}


def crystal_converter_ids(extract: dict) -> set[str]:
    return CONVERTER_VARIANTS | {
        row["item_key"].rsplit(".i", 1)[1]
        for row in extract["offers"]
        if row["title"].endswith("Converter")
    }


def refresh_crystal_extract(lua: Path, items_xml: Path) -> None:
    """Capture the fields Crystal's charged purchase dispatch actually reads."""
    extract = json.loads(CRYSTAL_CHARGED.read_bytes())
    raw = lua.read_bytes()
    if hashlib.sha256(raw).hexdigest() != extract["source"]["sha256"]:
        raise ValueError("Crystal charged offers source digest mismatch")
    lines = raw.decode().splitlines()
    for row in extract["offers"]:
        body = []
        for line in lines[row["source_line"] - 1 :]:
            if line.strip() == "},":
                break
            body.append(line)
        block = "\n".join(body)
        if f'name = "{row["title"]}"' not in block:
            raise ValueError(f"Crystal charged offer source line drift: {row['title']}")
        for field in ("charges", "count"):
            value = re.search(rf"^\s*{field}\s*=\s*(\d+),$", block, re.MULTILINE)
            if value:
                row[field] = int(value[1])
            else:
                row.pop(field, None)
    item_source = runpy.run_path(
        str(
            ROOT / "tools/content-schema/native-gameplay/build_spell_item_identities.py"
        )
    )
    raw = items_xml.read_bytes()
    if (
        extract["source"]["commit"] != item_source["INDEPENDENT_CRYSTAL_PIN"]
        or hashlib.sha256(raw).hexdigest() != item_source["INDEPENDENT_CRYSTAL_XML_SHA"]
    ):
        raise ValueError("Crystal converter item source digest mismatch")
    converter_ids = crystal_converter_ids(extract)
    weights = {}
    for item in ET.fromstring(raw).findall("item"):
        if item.get("id") in converter_ids:
            weight = item.find("attribute[@key='weight']")
            weights[item.get("id")] = (
                int(weight.attrib["value"]) if weight is not None else 0
            )
    if not converter_ids or set(weights) != converter_ids:
        raise ValueError("Crystal converter item definitions missing")
    extract["converter_weights"] = weights
    CRYSTAL_CHARGED.write_text(json.dumps(extract, indent=2) + "\n", encoding="utf-8")


def with_crystal_charged(catalogue: dict, items: dict[str, tuple[str, int]]) -> dict:
    extract = json.loads(CRYSTAL_CHARGED.read_text(encoding="utf-8"))
    source = catalogue["source"].copy()
    previous = set(source.get("crystal_charged_items", {}).get("offer_keys", []))
    offers = [row for row in catalogue["offers"] if row["key"] not in previous]
    keys = {row["key"] for row in offers}
    crystal_keys = []
    for row in extract["offers"]:
        # Crystal dispatch reads only charges. A donor row that supplies count
        # instead has no deliverable purchase and must stay off the active list.
        if "charges" not in row:
            continue
        key = "store:" + row["title"].lower().replace(" ", "-")
        item_key = row["item_key"]
        admitted = items.get(item_key)
        if admitted is None or admitted[1] != row["charges"]:
            raise ValueError(f"{key}: no admitted item with {row['charges']} charges")
        if key in keys:
            raise ValueError(f"{key}: duplicate Store offer")
        keys.add(key)
        crystal_keys.append(key)
        offers.append(
            {
                "key": key,
                "category": row["category"],
                "title": row["title"],
                "price": row["price"],
                "currency": "TIBIA_COIN",
                "item": {
                    "key": item_key,
                    "revision": admitted[0],
                    "charges": row["charges"],
                },
                "source_line": row["source_line"],
            }
        )
    source["crystal_charged_items"] = extract["source"] | {
        "offer_keys": sorted(crystal_keys)
    }
    offers.sort(key=lambda row: row["key"])
    return catalogue | {"source": source, "offers": offers}


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
            item = record["production_definition"]
            items[item["production_key"]] = (
                item["revision_ref"],
                charges["value"]["count"]["value"],
            )
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
        fields = {
            k: int(v)
            for k, v in re.findall(r"(price|itemtype|charges) = (\d+),", block)
        }
        if {"price", "itemtype", "charges"} <= fields.keys():
            offers.append({"name": name.group(1), "line": number, **fields})
    return offers


def build(lua: str) -> dict:
    items = admitted_items()
    catalogue = with_crystal_charged(
        {
            "schema": "OTERYN_STORE_RUNTIME_CATALOGUE/v1",
            "source": {
                "repository": "opentibiabr/canary",
                "commit": CANARY_COMMIT,
                "path": SOURCE,
            },
            "offers": [],
        },
        items,
    )
    offers = catalogue["offers"]
    crystal_keys = {offer["key"] for offer in offers}
    for offer in parse_offers(lua):
        offer_key = "store:" + offer["name"].lower().replace(" ", "-")
        if offer_key in crystal_keys:
            continue
        key = f"oteryn:item.tibia.i{offer['itemtype']}"
        admitted = items.get(key)
        if admitted is None or admitted[1] != offer["charges"]:
            continue
        offers.append(
            {
                "key": offer_key,
                "category": CATEGORY,
                "title": offer["name"],
                "price": offer["price"],
                "currency": "TIBIA_COIN",
                "item": {
                    "key": key,
                    "revision": admitted[0],
                    "charges": offer["charges"],
                },
                "source_line": offer["line"],
            }
        )
    return with_hunting_offers(with_expansion(catalogue))


def platform_catalogue(catalogue: dict, platform_base: dict | None = None) -> bytes:
    """Project all enabled quotes onto Platform's exact S-5 import shape."""
    blessing_offers = []
    for blessing in json.loads(BLESSINGS.read_bytes())["blessings"]:
        storage_key = blessing["storage_key"]
        prefix = "oteryn:blessing."
        suffix = storage_key.removeprefix(prefix)
        if not suffix or suffix == storage_key:
            raise ValueError(f"invalid blessing storage key: {storage_key}")
        blessing_offers.append(
            {
                "key": f"store:blessing:{suffix}",
                "price": blessing["store_tibia_coins"],
                "unlock_key": f"store-blessing:{storage_key}",
            }
        )
    catalogues = [
        catalogue,
        *(
            json.loads((STORE / f"{name}.json").read_bytes())
            for name in ("cosmetics", "hirelings")
        ),
        {"offers": blessing_offers},
    ]
    current_offers = sorted(
        (
            row
            for document in catalogues
            for row in document["offers"]
            if row.get("enabled", True)
        ),
        key=lambda row: row["key"],
    )
    offers = []
    for row in current_offers:
        kind = "COSMETIC"
        product = "character_service"
        revision = "1"
        grant = row.get("unlock_key", row["key"])
        if item := row.get("item"):
            kind = "ITEM"
            product = "item_grant"
            grant = item["key"]
            revision = item["revision"].removeprefix("definition-r")
        elif preview := row.get("preview"):
            if preview["kind"] in (2, 3):
                product = "outfit_unlock" if preview["kind"] == 2 else "mount_unlock"
        offers.append(
            {
                "offer_key": row["key"],
                "currency": "COINS",
                "unit_price": str(row["price"]),
                "max_quantity": 1,
                "purchasable": True,
                "units": [
                    {
                        "product_id": f"oteryn.store.{product}",
                        "product_version": "1",
                        "unit_kind": kind,
                        "grant_key": grant,
                        "grant_revision": revision,
                        "quantity": 1,
                    }
                ],
            }
        )
    if platform_base is not None:
        if platform_base["contract_version"] != 1:
            raise ValueError("expected an active Platform S-5 catalogue")
        offers.extend(
            offer
            for offer in platform_base["offers"]
            if offer["units"]
            and all(unit["unit_kind"] == "PREMIUM_TIME" for unit in offer["units"])
        )
    if len({offer["offer_key"] for offer in offers}) != len(offers):
        raise ValueError("duplicate offer across Store catalogues")
    offers.sort(key=lambda offer: offer["offer_key"])
    return json.dumps(
        {"contract_version": 1, "offers": offers}, separators=(",", ":")
    ).encode()


def write_platform_catalogue(document: bytes) -> None:
    if PLATFORM_OUT.is_file():
        previous = PLATFORM_OUT.read_bytes()
        if previous == document:
            return
        archive = PLATFORM_OUT.with_name("platform-predecessors.json")
        retained = json.loads(archive.read_bytes()) if archive.is_file() else []
        snapshot = previous.decode("utf-8")
        if snapshot not in retained:
            retained.append(snapshot)
            archive.write_text(json.dumps(retained, indent=2) + "\n", encoding="utf-8")
    PLATFORM_OUT.write_bytes(document)


def check(platform_base: dict | None = None) -> list[str]:
    catalogue = json.loads(OUT.read_text(encoding="utf-8"))
    items = admitted_items()
    errors = []
    keys = [offer["key"] for offer in catalogue["offers"]]
    if keys != sorted(set(keys)) or not keys:
        errors.append("offer keys are not unique and sorted")
    expected = with_expansion(catalogue)
    if catalogue != expected:
        errors.append("Prey expansion offer needs regeneration")
    if catalogue != with_hunting_offers(catalogue):
        errors.append("Store hunting offers need regeneration")
    hunting_keys = {row["key"] for row in json.loads(HUNTING_OFFERS.read_bytes())}
    if catalogue != with_crystal_charged(catalogue, items):
        errors.append("Crystal charged offers need regeneration")
    extract = json.loads(CRYSTAL_CHARGED.read_bytes())
    weights = extract["converter_weights"]
    profiles = {
        row["production_definition"]["production_key"]: row
        for row in json.loads(PROFILES.read_bytes())["records"]
    }
    for item_id in sorted(crystal_converter_ids(extract)):
        profile = profiles[f"oteryn:item.tibia.i{item_id}"]
        weight = profile["semantics"]["physical"]["value"]["weight"]
        if item_id not in weights or weight != {
            "state": "KNOWN",
            "value": weights[item_id],
        }:
            errors.append(f"{item_id}: native converter weight differs from Crystal")
    for offer in catalogue["offers"]:
        if offer.get("unlock_key") == "prey-list-expansion":
            continue
        if offer.get("unlock_key") == "charm-expansion" or offer["key"] in hunting_keys:
            if offer.get("item") is not None:
                errors.append(f"{offer['key']}: entitlement cannot deliver an item")
        else:
            item = offer["item"]
            admitted = items.get(item["key"])
            if admitted != (item["revision"], item["charges"]):
                errors.append(f"{offer['key']}: item not admitted with these charges")
        if not 0 < offer["price"] <= 100_000 or offer["currency"] != "TIBIA_COIN":
            errors.append(f"{offer['key']}: price or currency")
    if not errors and (
        not PLATFORM_OUT.is_file()
        or PLATFORM_OUT.read_bytes() != platform_catalogue(catalogue, platform_base)
    ):
        errors.append("Platform active catalogue needs regeneration")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--canary", type=Path)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--refresh-owner-offers", action="store_true")
    parser.add_argument(
        "--platform-base",
        type=Path,
        help="active Platform S-5 import to retain Premium Time offers from",
    )
    parser.add_argument("--crystal-extract", nargs=2, type=Path, metavar=("LUA", "XML"))
    args = parser.parse_args()
    if args.crystal_extract:
        refresh_crystal_extract(*args.crystal_extract)
    if args.canary:
        lua = (args.canary / SOURCE).read_text(encoding="utf-8")
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(build(lua), indent=2) + "\n", encoding="utf-8")
    if args.refresh_owner_offers:
        catalogue = json.loads(OUT.read_text(encoding="utf-8"))
        catalogue = with_crystal_charged(with_hunting_offers(with_expansion(catalogue)), admitted_items())
        OUT.write_text(json.dumps(catalogue, indent=2) + "\n", encoding="utf-8")
    platform_base = (
        json.loads(args.platform_base.read_bytes()) if args.platform_base else None
    )
    if (
        args.canary
        or args.refresh_owner_offers
        or (args.platform_base and not args.check)
    ):
        write_platform_catalogue(
            platform_catalogue(json.loads(OUT.read_bytes()), platform_base)
        )
    errors = check(platform_base)
    for error in errors:
        print(error, file=sys.stderr)
    print("FAIL" if errors else "PASS")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
