"""Apply Crystal 201189f equipment facts to existing canonical Item identities."""

import json
import xml.etree.ElementTree as ET

from build_source_item_profiles import NA, K, U, profile
from engine_items import build_resistances, ratio_value
from import_cyclopedia_global import apply_profile_weights
from lower_elemental_magic_modifier_packet import native_order

FACTS = "imports/crystalserver/facts/equipment-gap-quarter-3.json"
BINDINGS = "imports/crystalserver/bindings/items.json"
SLOTS = {
    1: "HEAD",
    2: "TORSO",
    3: "LEGS",
    4: "FEET",
    5: "WEAPON",
    6: "SHIELD",
    7: "AMULET",
    8: "RING",
}
OFF_HAND_GROUP = "oteryn:equipment-group.non_quiver_left_hand"
RESISTANCE_ORDER = (
    "DEATH",
    "DROWN",
    "EARTH",
    "ENERGY",
    "FIRE",
    "HOLY",
    "ICE",
    "LIFE_DRAIN",
    "MANA_DRAIN",
    "PHYSICAL",
)
POINTS = {
    "skillaxe": "SKILL_AXE",
    "skillclub": "SKILL_CLUB",
    "skilldist": "SKILL_DISTANCE",
    "skillshield": "SKILL_SHIELD",
    "skillsword": "SKILL_SWORD",
    "speed": "SPEED",
    "magiclevelpoints": "MAGIC_LEVEL_POINTS",
    "magicshieldcapacityflat": "MAGIC_SHIELD_CAPACITY_FLAT",
    "reflectdamage": "REFLECT_DAMAGE",
    "perfectshotdamage": "PERFECT_SHOT_DAMAGE",
    **{
        f"{element}magiclevelpoints": f"{element.upper()}_MAGIC_LEVEL_POINTS"
        for element in ("death", "earth", "energy", "fire", "healing", "holy", "ice")
    },
}
PERCENTS = {
    "cleavepercent": ("CLEAVE_PERCENT", 1),
    "magicshieldcapacitypercent": ("MAGIC_SHIELD_CAPACITY_PERCENT", 1),
    "criticalhitchance": ("CRITICAL_HIT_CHANCE", 100),
    "criticalhitdamage": ("CRITICAL_HIT_DAMAGE", 100),
    # Crystal game.cpp:8627-8629 rolls whole-percent chance; the amount uses /10000.
    "lifeleechamount": ("LIFE_LEECH_AMOUNT", 100),
    "lifeleechchance": ("LIFE_LEECH_CHANCE", 1),
}


def modifiers(attrs, order):
    values = {
        kind: ("SIGNED_POINTS", int(attrs[name]))
        for name, kind in POINTS.items()
        if name in attrs
    }
    for name, (kind, scale) in PERCENTS.items():
        if name in attrs:
            values[kind] = ("RATIONAL_PERCENT", ratio_value(attrs[name], scale))
    if "perfectshotrange" in attrs:
        values["PERFECT_SHOT_RANGE"] = ("CELLS", int(attrs["perfectshotrange"]))
    return K(
        {
            "modifiers": K(
                [
                    {
                        "kind": kind,
                        "target_domain": U,
                        "evaluation_phase": U,
                        "priority": U,
                        "parameter": K(
                            {"kind": values[kind][0], "value": values[kind][1]}
                        ),
                    }
                    for kind in sorted(values, key=order.__getitem__)
                ]
            )
        }
    )


def lower(fact, bindings, order):
    item = fact["id"]
    attrs = fact["xml"]
    script = fact["script"]
    weapon = script.get("weapontype", attrs.get("weapontype"))
    xml = ET.Element("item", name=fact["name"])
    for key, value in attrs.items():
        ET.SubElement(xml, "attribute", key=key, value=value)
    nested = ET.SubElement(xml, "attribute", key="script")
    for key, value in script.items():
        ET.SubElement(nested, "attribute", key=key, value=value)
    physical = {
        "movable": fact["movable"],
        "pickupable": fact["pickupable"],
        "block_solid": fact["blocks_movement"],
        "block_projectile": fact["blocks_projectile"],
        "has_height": fact["has_height"],
    }
    record = profile(
        item,
        xml,
        {"flags": {"flags.cumulative": fact["stackable"]}},
        physical,
        bindings,
        None,
    )
    semantics = record["semantics"]
    slot = SLOTS[fact["slot"]]
    two_handed = (
        attrs.get("slottype") == "two-handed" or script.get("slot") == "two-handed"
    )
    launcher = weapon == "distance" and "ammotype" in attrs
    semantics["equipment"] = K(
        {
            "patterns": K(
                [
                    {
                        "pattern_id": 1,
                        "primary_slot": K(slot),
                        "additional_reserved_slots": K(
                            ["SHIELD"] if two_handed and not launcher else []
                        ),
                        "mutually_exclusive_groups": K(
                            [OFF_HAND_GROUP] if slot == "SHIELD" or launcher else []
                        ),
                        "level": K(int(script.get("level", 0))),
                        "vocations": K(fact["vocations"]),
                        "compatibility_rule": NA,
                    }
                ]
            )
        }
    )
    if weapon == "spellbook":
        # The existing defence and swing owners distinguish spellbooks from shields.
        semantics["weapon"] = K(
            {
                "weapon_type": K("SPELLBOOK"),
                "attack": K(0),
                "defense": K(int(attrs["defense"])),
                "extra_defense": K(0),
                "range": K(1),
                "hit_chance": NA,
                "max_hit_chance": NA,
                "ammunition": NA,
                "elemental": K([]),
            }
        )
    elif not weapon:
        semantics["weapon"] = NA
    resistances = {
        entry["damage_type"].upper(): entry["reduction_percent"]
        for entry in build_resistances(attrs)
    }
    semantics["protection"] = K(
        {
            "armor": K(int(attrs.get("armor", 0))),
            "resistances": K(
                [
                    {"kind": kind, "percent": K(resistances[kind])}
                    for kind in RESISTANCE_ORDER
                    if kind in resistances
                ]
            ),
        }
    )
    semantics["skill_modifiers"] = modifiers(attrs, order)
    if "imbuementslot" in attrs:
        semantics["imbuement"] = K(
            {
                "slot_count": K(int(attrs["imbuementslot"])),
                "allowed_family_tiers": U,
                "excluded_families": U,
            }
        )
    top = fact["top_order"]
    record["attributes"].update(always_on_top=bool(top), always_on_top_order=top)
    if launcher:
        # Crystal weapons.cpp:810-819 uses a 90% cap when the launcher takes ammunition.
        record["authoring"]["ranged"] = {
            "kind": "DISTANCE",
            "attack": int(attrs["attack"]),
            "range_cells": int(attrs["range"]),
            "ammunition": attrs["ammotype"].upper(),
            "hit_chance": int(attrs.get("hitchance", 0)),
            "max_hit_chance": int(attrs.get("maxhitchance", 90)),
        }
    elif weapon == "wand":
        record["authoring"]["ranged"] = {
            "kind": "ROD" if attrs.get("primarytype") == "rods" else "WAND",
            "element": script["wandtype"].upper(),
            "mana_per_shot": int(script["mana"]),
            "damage_min": int(script["fromdamage"]),
            "damage_max": int(script["todamage"]),
            "range_cells": int(attrs.get("range", 1)),
        }
    binding = bindings[str(item)]
    target = binding["target"]
    record["production_binding"] = binding
    record["production_definition"] = {
        "family": target["family"],
        "production_key": target["key"],
        "revision_ref": target["revision"],
    }
    return record


def source_profiles(root):
    document = json.loads((root / FACTS).read_bytes())
    rows = document["records"]
    if (
        document["schema"] != "OTERYN_CRYSTAL_EQUIPMENT_GAP_FACTS/v1"
        or document["source"]["revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        or len(rows) != 179
        or [row["id"] for row in rows] != sorted({row["id"] for row in rows})
        or any(not (row["movable"] and row["pickupable"]) for row in rows)
    ):
        raise ValueError("Crystal equipment quarter 3 source closure")
    bindings = {
        row["external_id"]: row
        for row in json.loads((root / BINDINGS).read_bytes())["bindings"]
    }
    order = native_order(root)
    profiles = {
        bindings[str(row["id"])]["target"]["key"]: lower(row, bindings, order)
        for row in rows
        if "held" not in row
    }
    apply_profile_weights(list(profiles.values()), root)
    return profiles


def apply_semantics(current, source):
    for name in (
        "physical",
        "stack",
        "container",
        "charges",
        "temporal",
        "equipment",
        "weapon",
        "protection",
        "skill_modifiers",
    ):
        current[name] = source[name]
    # Keep pre-existing imbuement family/tier and exclusion policies. This source
    # only supplies the confirmed slot count.
    if source["imbuement"]["state"] == "KNOWN":
        imbuement = current.get("imbuement", U)
        if imbuement["state"] == "KNOWN":
            imbuement["value"]["slot_count"] = source["imbuement"]["value"][
                "slot_count"
            ]
        else:
            current["imbuement"] = source["imbuement"]


def apply_admissions(definitions, root):
    profiles = source_profiles(root)
    applied = set()
    for definition in definitions:
        key = definition["identity"]["key"]
        if key not in profiles:
            continue
        source = profiles[key]
        current = definition.setdefault("semantics", {})
        for name, value in source["semantics"].items():
            current.setdefault(name, value)
        apply_semantics(current, source["semantics"])
        definition["materializable"] = True
        definition["stack_class"] = source["admission"]["stack_class"]
        applied.add(key)
    if applied != profiles.keys():
        raise ValueError(
            f"Crystal equipment quarter 3 missing canonical identities: {profiles.keys() - applied}"
        )


def apply_native(root, items, order):
    profiles = source_profiles(root)
    existing = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    for key, source in profiles.items():
        if key not in existing:
            items["records"].append(source)
            continue
        current = existing[key]
        for field in ("production_binding", "production_definition", "admission"):
            if current[field] != source[field]:
                raise ValueError(f"Crystal equipment quarter 3 conflict: {key}/{field}")
        apply_semantics(current["semantics"], source["semantics"])
        current["authoring"].update(
            {key: value for key, value in source["authoring"].items() if key != "item"}
        )
        current["attributes"]["speed_bonus"] = source["attributes"]["speed_bonus"]
    items["records"].sort(key=order)
