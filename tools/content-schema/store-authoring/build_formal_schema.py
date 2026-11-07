"""Builds the Store offer authoring schema candidate v1
(docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md). Mirrors the
Canary/Crystal GameStore catalog model (category/offer, `store_constants.OFFER_TYPES`);
not a WorldProject/v2 contract or runtime activation. Semantic rules live in
`validate_store.py`."""

import json
from pathlib import Path

from store_constants import COIN_TYPES, OFFER_TYPES, STATES

ROOT = Path(__file__).resolve().parent
DIALECT = "https://json-schema.org/draft/2020-12/schema"
ID = "urn:oteryn:store-authoring:candidate:1"
MONSTER_ID = "urn:oteryn:monster-authoring:candidate:1"


def obj(props, required=(), **extra):
    return {
        "type": "object",
        "additionalProperties": False,
        "properties": props,
        "required": list(required),
        **extra,
    }


def array(item, minimum=0, unique=False, **extra):
    return {
        "type": "array",
        "items": item,
        "minItems": minimum,
        **({"uniqueItems": True} if unique else {}),
        **extra,
    }


def integer(minimum=0, maximum=None, **extra):
    return {
        "type": "integer",
        "minimum": minimum,
        **({"maximum": maximum} if maximum is not None else {}),
        **extra,
    }


def text(minimum=1, **extra):
    return {"type": "string", "minLength": minimum, **extra}


def enum(*values):
    return {"enum": list(values)}


def use(name):
    return {"$ref": "#/$defs/" + name}


def monster(name):
    return {"$ref": MONSTER_ID + "#/$defs/" + name}


d: dict = {}
for name in ("key", "revision", "identity", "bool", "ItemRef"):
    d[name] = monster(name)

d["icons"] = array(
    text(pattern=r"^[^/\\]+\.png$"),
    minimum=1,
    description="Client icon asset *names* only (no path separators), never asset bytes.",
)
d["state"] = {
    **enum(*STATES),
    "description": "GameStore.States; both engines agree on the four values and their ids.",
}
d["coin_type"] = {
    **enum(*COIN_TYPES),
    "description": (
        "GameStore.CoinType. Canary always defaults an offer with no explicit coinType to "
        "transferable; Crystal defaults to coin unless the offer description contains the "
        "literal marker `{transferableprice}`, in which case transferable. Neither engine's "
        "catalog data sets coinType explicitly; the converter always derives it."
    ),
}
d["offer_type"] = {
    **enum(*OFFER_TYPES),
    "description": (
        "Canonical union of Canary `GameStore.OfferTypes` (27 values, version 2.0) and "
        "Crystal's (28 values, version 1.1); `store_constants.OFFER_TYPES` documents each "
        "member's per-engine numeric id and product-payload kind."
    ),
}
d["CategoryRef"] = use("key")
d["ItemQuantity"] = obj(
    {
        "item_refs": array(
            use("ItemRef"),
            minimum=1,
            unique=True,
            description="1 element for every offer_type except item_bed (2: the item's two unwrap boxes).",
        ),
        "count": {"anyOf": [integer(minimum=1), {"type": "null"}]},
        "charges": {"anyOf": [integer(minimum=1), {"type": "null"}]},
    },
    ("item_refs", "count", "charges"),
    description="Exactly one of count/charges is non-null (validate_store.py); count is stack/delivered "
    "quantity, charges is the item's use count (offer_type charges only).",
)
d["OutfitPayload"] = obj(
    {
        "female_look_type": integer(minimum=0),
        "male_look_type": integer(minimum=0),
        "addon": integer(minimum=0, maximum=3),
    },
    ("female_look_type", "male_look_type", "addon"),
)
d["HirelingPayload"] = obj(
    {"female_look_type": integer(minimum=0), "male_look_type": integer(minimum=0)},
    ("female_look_type", "male_look_type"),
)
d["MountPayload"] = obj({"mount_id": integer(minimum=0)}, ("mount_id",))
d["BlessingPayload"] = obj(
    {"bless_id": integer(minimum=1), "count": integer(minimum=1)}, ("bless_id", "count")
)
d["BlessingBundlePayload"] = obj({"count": integer(minimum=1)}, ("count",))
d["PremiumTimePayload"] = obj({"days": integer(minimum=1)}, ("days",))
d["HirelingTypedPayload"] = obj(
    {"typed_id": {"anyOf": [integer(minimum=0), {"type": "null"}]}},
    ("typed_id",),
    description="typed_id is null when the source `id` is an engine lookup-table reference this reader "
    "cannot statically resolve (HIRELING_SKILLS.*/HIRELING_OUTFITS.*); never guessed.",
)

d["Product"] = {
    "oneOf": [
        obj({"kind": {"const": "service"}}, ("kind",)),
        obj({"kind": {"const": "item"}, "item": use("ItemQuantity")}, ("kind", "item")),
        obj(
            {"kind": {"const": "outfit"}, "outfit": use("OutfitPayload")},
            ("kind", "outfit"),
        ),
        obj(
            {"kind": {"const": "mount"}, "mount": use("MountPayload")},
            ("kind", "mount"),
        ),
        obj(
            {"kind": {"const": "hireling"}, "hireling": use("HirelingPayload")},
            ("kind", "hireling"),
        ),
        obj(
            {
                "kind": {"const": "hireling_typed"},
                "hireling_typed": use("HirelingTypedPayload"),
            },
            ("kind", "hireling_typed"),
        ),
        obj(
            {"kind": {"const": "blessing"}, "blessing": use("BlessingPayload")},
            ("kind", "blessing"),
        ),
        obj(
            {
                "kind": {"const": "blessing_bundle"},
                "blessing_bundle": use("BlessingBundlePayload"),
            },
            ("kind", "blessing_bundle"),
        ),
        obj(
            {
                "kind": {"const": "premium_time"},
                "premium_time": use("PremiumTimePayload"),
            },
            ("kind", "premium_time"),
        ),
    ]
}

d["Category"] = obj(
    {
        "identity": use("identity"),
        "display_name": text(),
        "parent": {"anyOf": [use("CategoryRef"), {"type": "null"}]},
        "icons": use("icons"),
        "rookgaard": use("bool"),
        "state": use("state"),
        "shape": {
            **enum("leaf", "group"),
            "description": "leaf carries offers (via Offer.category); group carries subclass_names only.",
        },
        "subclass_names": {
            "anyOf": [array(text(), minimum=1, unique=True), {"type": "null"}]
        },
    },
    (
        "identity",
        "display_name",
        "parent",
        "icons",
        "rookgaard",
        "state",
        "shape",
        "subclass_names",
    ),
)

d["Offer"] = obj(
    {
        "identity": use("identity"),
        "category": use("CategoryRef"),
        "display_name": text(),
        "icons": use("icons"),
        "price_coins": integer(minimum=0),
        "coin_type": use("coin_type"),
        "offer_type": use("offer_type"),
        "state": use("state"),
        "description": {"anyOf": [text(minimum=0), {"type": "null"}]},
        "home": use("bool"),
        "hireling_slot_number": {"anyOf": [integer(minimum=0), {"type": "null"}]},
        "engine_offer_id": {
            "anyOf": [integer(minimum=0), {"type": "null"}],
            "description": "Engine `id`; a legacy/incidental store action id for most offer types, never used for identity. Null when absent or non-literal in source.",
        },
        "product": use("Product"),
    },
    (
        "identity",
        "category",
        "display_name",
        "icons",
        "price_coins",
        "coin_type",
        "offer_type",
        "state",
        "description",
        "home",
        "hireling_slot_number",
        "engine_offer_id",
        "product",
    ),
)

SCHEMA = {
    "$schema": DIALECT,
    "$id": ID,
    "title": "Store offer authoring schema candidate v1 — structural validation",
    "description": (
        "CANDIDATE authoring schema (docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md). "
        "Not a WorldProject/v2 contract or runtime activation; semantic rules live in validate_store.py."
    ),
    "type": "object",
    "additionalProperties": False,
    "properties": {
        "categories": array(use("Category"), minimum=1),
        "offers": array(use("Offer"), minimum=1),
    },
    "required": ["categories", "offers"],
    "$defs": d,
}


def main():
    out = ROOT / "store-offer.schema.json"
    out.write_text(
        json.dumps(SCHEMA, indent=2, sort_keys=False) + "\n", encoding="utf-8"
    )
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
