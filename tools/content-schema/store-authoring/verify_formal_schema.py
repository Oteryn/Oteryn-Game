"""Focused positive/negative cases for the Store offer authoring schema candidate v1.

Regenerates `synthetic-valid-store-offer.json` (a small hand-built catalog, not
transcribed from either engine) and checks that each mutation is rejected for the
intended reason.
"""

import copy
import json
import sys
from pathlib import Path

from validate_store import validate

ROOT = Path(__file__).resolve().parent


def identity(key):
    return {"key": key, "revision": "definition-r1"}


def item_ref(n):
    return {
        "family": "Item",
        "key": f"oteryn:item.tibia.i{n}",
        "revision": "definition-r1",
    }


CATALOG = {
    "categories": [
        {
            "identity": identity("oteryn:store.category.consumables"),
            "display_name": "Consumables",
            "parent": None,
            "icons": ["Category_Consumables.png"],
            "rookgaard": True,
            "state": "none",
            "shape": "group",
            "subclass_names": ["Potions"],
        },
        {
            "identity": identity("oteryn:store.category.potions"),
            "display_name": "Potions",
            "parent": "oteryn:store.category.consumables",
            "icons": ["Category_Potions.png"],
            "rookgaard": True,
            "state": "none",
            "shape": "leaf",
            "subclass_names": None,
        },
        {
            "identity": identity("oteryn:store.category.cosmetics_mounts"),
            "display_name": "Mounts",
            "parent": None,
            "icons": ["Category_Mounts.png"],
            "rookgaard": True,
            "state": "none",
            "shape": "leaf",
            "subclass_names": None,
        },
    ],
    "offers": [
        {
            "identity": identity("oteryn:store.offer.potions.great_health_potion"),
            "category": "oteryn:store.category.potions",
            "display_name": "Great Health Potion",
            "icons": ["Great_Health_Potion.png"],
            "price_coins": 18,
            "coin_type": "transferable",
            "offer_type": "stackable",
            "state": "none",
            "description": "Restores hit points.",
            "home": False,
            "hireling_slot_number": None,
            "engine_offer_id": None,
            "product": {
                "kind": "item",
                "item": {"item_refs": [item_ref(239)], "count": 100, "charges": None},
            },
        },
        {
            "identity": identity("oteryn:store.offer.mounts.armoured_war_horse"),
            "category": "oteryn:store.category.cosmetics_mounts",
            "display_name": "Armoured War Horse",
            "icons": ["Armoured_War_Horse.png"],
            "price_coins": 870,
            "coin_type": "transferable",
            "offer_type": "mount",
            "state": "none",
            "description": None,
            "home": False,
            "hireling_slot_number": None,
            "engine_offer_id": 23,
            "product": {"kind": "mount", "mount": {"mount_id": 23}},
        },
        {
            "identity": identity("oteryn:store.offer.potions.twist_of_fate"),
            "category": "oteryn:store.category.potions",
            "display_name": "Twist of Fate",
            "icons": ["Twist_of_Fate.png"],
            "price_coins": 8,
            "coin_type": "transferable",
            "offer_type": "blessings",
            "state": "none",
            "description": "A regular blessing.",
            "home": False,
            "hireling_slot_number": None,
            "engine_offer_id": None,
            "product": {"kind": "blessing", "blessing": {"bless_id": 1, "count": 1}},
        },
        {
            "identity": identity("oteryn:store.offer.potions.character_name_change"),
            "category": "oteryn:store.category.potions",
            "display_name": "Character Name Change",
            "icons": ["Name_Change.png"],
            "price_coins": 250,
            "coin_type": "transferable",
            "offer_type": "namechange",
            "state": "none",
            "description": None,
            "home": False,
            "hireling_slot_number": None,
            "engine_offer_id": 65002,
            "product": {"kind": "service"},
        },
    ],
}


def check(name, mutate, fragment):
    data = copy.deepcopy(CATALOG)
    mutate(data)
    errors = validate(data)
    joined = "\n".join(errors)
    if not errors:
        print(f"FAIL {name}: expected an error containing {fragment!r}, got none")
        return False
    if fragment not in joined:
        print(f"FAIL {name}: expected {fragment!r} in errors, got:\n{joined}")
        return False
    print(f"ok   {name}")
    return True


def main():
    (ROOT / "synthetic-valid-store-offer.json").write_text(
        json.dumps(CATALOG, indent=2) + "\n", encoding="utf-8"
    )

    ok = True
    errors = validate(copy.deepcopy(CATALOG))
    if errors:
        print("FAIL positive: valid catalog rejected:\n" + "\n".join(errors))
        ok = False
    else:
        print("ok   positive")

    def dup_category_key(d):
        d["categories"][1]["identity"]["key"] = d["categories"][0]["identity"]["key"]

    def unknown_parent(d):
        d["categories"][1]["parent"] = "oteryn:store.category.nope"

    def parent_not_group(d):
        d["categories"][1]["parent"] = "oteryn:store.category.cosmetics_mounts"

    def leaf_with_subclasses(d):
        d["categories"][1]["subclass_names"] = ["X"]

    def group_without_subclasses(d):
        d["categories"][0]["subclass_names"] = None

    def offer_bad_category(d):
        d["offers"][0]["category"] = "oteryn:store.category.nope"

    def offer_category_is_group(d):
        d["offers"][0]["category"] = "oteryn:store.category.consumables"

    def dup_offer_key(d):
        d["offers"][1]["identity"]["key"] = d["offers"][0]["identity"]["key"]

    def offer_type_kind_mismatch(d):
        d["offers"][1]["offer_type"] = "namechange"

    def item_count_and_charges(d):
        d["offers"][0]["product"]["item"]["charges"] = 5

    def item_count_and_charges_both_null(d):
        d["offers"][0]["product"]["item"]["count"] = None

    def negative_price(d):
        d["offers"][0]["price_coins"] = -1

    def bad_offer_type_enum(d):
        d["offers"][0]["offer_type"] = "not_a_real_type"

    def bad_icon_name(d):
        d["offers"][0]["icons"] = ["not-an-icon"]

    ok &= check("dup_category_key", dup_category_key, "duplicate category key")
    ok &= check("unknown_parent", unknown_parent, "does not resolve to a category")
    ok &= check("parent_not_group", parent_not_group, "is not a group category")
    ok &= check(
        "leaf_with_subclasses", leaf_with_subclasses, "must not carry subclass_names"
    )
    ok &= check(
        "group_without_subclasses",
        group_without_subclasses,
        "must carry non-empty subclass_names",
    )
    ok &= check("offer_bad_category", offer_bad_category, "does not resolve")
    ok &= check(
        "offer_category_is_group", offer_category_is_group, "cannot carry offers"
    )
    ok &= check("dup_offer_key", dup_offer_key, "duplicate offer key")
    ok &= check(
        "offer_type_kind_mismatch", offer_type_kind_mismatch, "expects product.kind"
    )
    ok &= check(
        "item_count_and_charges", item_count_and_charges, "exactly one of count/charges"
    )
    ok &= check(
        "item_count_and_charges_both_null",
        item_count_and_charges_both_null,
        "exactly one of count/charges",
    )
    ok &= check("negative_price", negative_price, "minimum")
    ok &= check("bad_offer_type_enum", bad_offer_type_enum, "not_a_real_type")
    ok &= check("bad_icon_name", bad_icon_name, "does not match")

    if not ok:
        sys.exit(1)
    print("PASS")


if __name__ == "__main__":
    main()
