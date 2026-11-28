"""Qualify the Global Crusher against the existing identity and appearance bank."""
import hashlib
import json
from pathlib import Path

from import_cyclopedia_global import load_weights
from native_profile_composition import encode_native_profile
from qualification_inputs import load_appearance_objects

ITEM = 46627
KEY = f"oteryn:item.tibia.i{ITEM}"
APPEARANCE_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
REFERENCE = Path("content/items/definitions/items-26000-26499.json")
BINDINGS = Path("imports/crystalserver/bindings/items.json")
NA = {"state": "NOT_APPLICABLE"}


def known(value):
    return {"state": "KNOWN", "value": value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    reference = next(row["definition"] for row in json.loads((root / REFERENCE).read_bytes())["records"]
                     if row["definition"]["identity"]["key"] == KEY)
    identity = reference["identity"]
    reference_facts = reference["semantics"]
    binding = next(row for row in json.loads((root / BINDINGS).read_bytes())["bindings"]
                   if row["external_id"] == str(ITEM))
    if binding["target"] != identity or binding["disposition"] != "EXACT":
        raise ValueError("Crusher identity drift")

    bank = (root / f"content/assets/files/appearances-{APPEARANCE_SHA}.dat").read_bytes()
    if hashlib.sha256(bank).hexdigest() != APPEARANCE_SHA:
        raise ValueError("Crusher appearance digest")
    appearance = load_appearance_objects(bank)[ITEM]
    flags = appearance["flags"]
    # Global Wiki's Crusher entry and the official appearance identify the
    # same ten-use, single-instance item sold by equipment shopkeepers.
    if (appearance["name"] != "crusher"
            or not all(flags.get(f"flags.{flag}") for flag in ("take", "multiuse", "wearout", "market"))
            or any(flags.get(f"flags.{flag}") for flag in ("unmove", "cumulative", "container"))
            or flags.get("market.trade_as_object_id") != ITEM
            or set(flags.get("npcsaledata.sale_price", ())) != {500}
            or reference_facts["charges"] != known({"count": known(10)})):
        raise ValueError("Crusher Global facts drift")
    weight = load_weights(root)[ITEM]
    record = {
        "admission": {"legal_destinations": ["CharacterInventory", "Ground"],
                      "materializable": True, "stack_class": "NonStackable"},
        "attributes": {"always_on_top": False, "always_on_top_order": 0, "speed_bonus": 0},
        "authoring": {"item": identity, "taxonomy": {"primary": "Tools"}},
        "production_binding": binding,
        "production_definition": {"family": "Item", "production_key": KEY,
                                  "revision_ref": identity["revision"]},
        "semantics": {
            "charges": reference_facts["charges"],
            "container": NA, "equipment": NA, "temporal": NA,
            "physical": known({"movable": known(True), "pickupable": known(True),
                               "weight": known(weight)}),
            "presentation": reference_facts["presentation"],
            "stack": known({"stackable": known(False), "stack_max": NA}),
            "trade_restrictions": reference_facts["trade_restrictions"],
        },
    }
    document = json.loads(generated[profile_path])
    if any(row["production_definition"]["production_key"] == KEY for row in document["records"]):
        raise ValueError("Duplicate Crusher native profile")
    document["records"] = sorted([*document["records"], record], key=order)
    raw = encode_native_profile(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    generated[profile_path] = raw
    generated[manifest_path] = canonical(manifest)
