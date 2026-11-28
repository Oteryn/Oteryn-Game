"""Admit the first Crystal equipment audit quarter through the native Item provider.

The bounded XML and appearance extract retains Crystal 201189f8 item IDs and
line numbers. Only exact existing Item bindings are eligible. The paired timed
items also require their carried forms and the expired Firewalker boots.
"""

import copy
import json

from house_decoration_actions import KINDS, apply_top_order
from house_decoration_actions import PROFILES as HOUSE_PROFILES
from import_cyclopedia_global import apply_profile_weights
from ranged_item_profiles import item_rows

FACTS = "imports/crystalserver/facts/equipment-audit-quarter-1.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
NA = {"state": "NOT_APPLICABLE"}
UNKNOWN = {"state": "UNKNOWN"}

SLOTS = {
    "head": "HEAD",
    "armor": "TORSO",
    "legs": "LEGS",
    "feet": "FEET",
    "hand": "WEAPON",
    "shield": "SHIELD",
    "necklace": "AMULET",
    "ring": "RING",
}
NON_QUIVER_HAND = "oteryn:equipment-group.non_quiver_left_hand"
VOCATIONS = {
    "none": "NONE",
    "knight": "KNIGHT",
    "paladin": "PALADIN",
    "monk": "MONK",
    "sorcerer": "SORCERER",
    "druid": "DRUID",
    "elite knight": "KNIGHT",
    "royal paladin": "PALADIN",
    "exalted monk": "MONK",
    "master sorcerer": "SORCERER",
    "elder druid": "DRUID",
}
SKILLS = {
    "magiclevelpoints": "MAGIC_LEVEL_POINTS",
    "skillaxe": "SKILL_AXE",
    "skillclub": "SKILL_CLUB",
    "skilldist": "SKILL_DISTANCE",
    "skillshield": "SKILL_SHIELD",
    "skillsword": "SKILL_SWORD",
}
RESISTANCES = {
    "absorbpercentdeath": "DEATH",
    "absorbpercentdrown": "DROWN",
    "absorbpercentpoison": "EARTH",
    "absorbpercentenergy": "ENERGY",
    "absorbpercentfire": "FIRE",
    "absorbpercentholy": "HOLY",
    "absorbpercentice": "ICE",
    "absorbpercentphysical": "PHYSICAL",
    "fieldabsorbpercentfire": "FIRE_FIELD",
}


def known(value):
    return {"state": "KNOWN", "value": value}


def ratio(value):
    return {"numerator": int(value), "denominator": 1}


def target(identifier):
    return {"key": f"oteryn:item.tibia.i{identifier}", "revision": "definition-r1"}


def weapon_kind(fact):
    # Crystal does not declare a wield kind for the ordinary spellbook. Its
    # exact TibiaWiki identity (3101, Spellbook) supplies the shield-hand slot.
    if fact["id"] == 3101:
        return "SPELLBOOK"
    kind = fact["attributes"].get("weapontype")
    return kind.upper() if kind else None


def equipment(fact, kind):
    attrs = fact["attributes"]
    two_handed = attrs.get("slottype") == "two-handed"
    slot = SLOTS.get(attrs.get("slot"))
    if slot is None and kind:
        slot = "SHIELD" if kind in ("SHIELD", "SPELLBOOK") else "WEAPON"
    if slot is None and fact["flags"].get("clothes.slot") == 1:
        slot = "HEAD"
    if fact["id"] == 9020 and fact["flags"].get("clothes.slot") == 8:
        # The wear-budget owner transforms the equipped instance in place.
        slot = "FEET"
    if slot is None:
        raise ValueError(f"No qualified equipment slot for {fact['id']}")
    bow = two_handed and kind == "DISTANCE"
    # The current TibiaWiki Depth Galea requirement supplies the missing
    # level on Crystal's alternate appearance (14258).
    level = int(attrs.get("level", 150 if fact["id"] == 14258 else 0))
    vocations = sorted(
        {
            VOCATIONS[name.split(";")[0].strip().lower()]
            for name in attrs.get("vocation", "").split(",")
            if name
        }
    )
    return known(
        {
            "patterns": known(
                [
                    {
                        "pattern_id": 1,
                        "primary_slot": known(slot),
                        "additional_reserved_slots": known(
                            ["SHIELD"] if two_handed and not bow else []
                        ),
                        "mutually_exclusive_groups": known(
                            [NON_QUIVER_HAND] if bow or slot == "SHIELD" else []
                        ),
                        "vocations": known(vocations),
                        "level": known(level),
                        "compatibility_rule": NA,
                    }
                ]
            )
        }
    )


def ranged(kind, attrs):
    cells = int(attrs.get("range", 1))
    if kind == "WAND":
        return {
            "kind": "WAND",
            "range_cells": cells,
            "element": attrs["wandtype"].upper(),
            "damage_min": int(attrs["fromdamage"]),
            "damage_max": int(attrs["todamage"]),
            "mana_per_shot": int(attrs["mana"]),
        }
    return {
        "kind": "DISTANCE",
        "range_cells": cells,
        "attack": int(attrs.get("attack", 0)),
        "ammunition": attrs["ammotype"].upper(),
        "hit_chance": int(attrs.get("hitchance", 0)),
        "max_hit_chance": int(attrs.get("maxhitchance", 90)),
    }


def modifier(kind, parameter_kind, value):
    return {
        "kind": kind,
        "parameter": known({"kind": parameter_kind, "value": value}),
        "target_domain": UNKNOWN,
        "evaluation_phase": UNKNOWN,
        "priority": UNKNOWN,
    }


def complete(record, fact):
    attrs = fact["attributes"]
    flags = fact["flags"]
    identifier = fact["id"]
    if (
        flags.get("flags.take") is not True
        or flags.get("flags.unmove")
        or flags.get("flags.cumulative")
    ):
        raise ValueError(f"Crystal equipment physical facts differ: {identifier}")
    semantics = record["semantics"]
    kind = weapon_kind(fact)
    semantics["equipment"] = equipment(fact, kind)
    semantics["physical"] = known(
        {
            "weight": known(int(attrs["weight"])),
            "movable": known(True),
            "pickupable": known(True),
        }
    )
    semantics["stack"] = known({"stackable": known(False), "stack_max": NA})
    semantics["container"] = NA
    semantics["charges"] = NA
    semantics["temporal"] = NA
    if "duration" in attrs:
        decay = int(attrs["decayto"])
        semantics["temporal"] = known(
            {
                "consumption_mode": known("AUTHORITATIVE_ACTIVE_TIME_BUDGET"),
                "duration": known(1000 * int(attrs["duration"])),
                "stop_duration": known(False),
                "decay_target": known(target(decay)) if decay else NA,
            }
        )
    transforms = {
        transform: known(target(int(attrs[source])))
        for transform, source in [
            ("EQUIP", "transformequipto"),
            ("DEEQUIP", "transformdeequipto"),
        ]
        if source in attrs
    }
    if transforms:
        previous = {
            entry["kind"]: entry["target"]
            for entry in semantics["use_transform"].get("value", {}).get("targets", [])
        }
        semantics["use_transform"] = known(
            {
                "targets": [
                    {
                        "kind": kind,
                        "target": transforms.get(kind, previous.get(kind, UNKNOWN)),
                    }
                    for kind in KINDS
                ]
            }
        )
    if kind:
        semantics["weapon"] = known(
            {
                "weapon_type": known(kind),
                "attack": known(int(attrs.get("attack", 0))),
                "defense": known(int(attrs.get("defense", 0))),
                "extra_defense": known(int(attrs.get("extradef", 0))),
                "range": known(int(attrs.get("range", 1))),
                "hit_chance": known(ratio(attrs["hitchance"]))
                if "hitchance" in attrs
                else NA,
                "max_hit_chance": known(ratio(attrs["maxhitchance"]))
                if "maxhitchance" in attrs
                else NA,
                "ammunition": known(attrs["ammotype"].upper())
                if "ammotype" in attrs
                else NA,
                "elemental": known([]),
            }
        )
        if kind in ("DISTANCE", "WAND"):
            record["authoring"]["ranged"] = ranged(kind, attrs)
    resistances = [
        {"kind": kind, "percent": known(ratio(attrs[source]))}
        for source, kind in RESISTANCES.items()
        if source in attrs
    ]
    if "armor" in attrs or resistances:
        semantics["protection"] = known(
            {
                "armor": known(int(attrs["armor"])) if "armor" in attrs else NA,
                "resistances": known(resistances),
            }
        )
    # Worn abilities own the active form. The timed host reads that form while
    # the base form remains in durable custody.
    modifiers = [
        modifier(kind, "SIGNED_POINTS", int(attrs[source]))
        for source, kind in SKILLS.items()
        if source in attrs
    ]
    if identifier in (5460, 14258):
        modifiers.append(modifier("SUPPRESS_DROWN", "BOOLEAN", True))
    if modifiers or "transformequipto" in attrs:
        semantics["skill_modifiers"] = known({"modifiers": known(modifiers)})
    speed = int(attrs.get("speed", 0))
    record["attributes"].update(
        speed_bonus=speed,
        blocks_movement=bool(flags.get("flags.unpass")),
        blocks_projectile=bool(flags.get("flags.block_sight")),
        immovable_block_solid=False,
        has_height=bool(flags.get("flags.height")),
    )


def apply(root, document, order):
    facts = json.loads((root / FACTS).read_bytes())
    rows = facts["records"]
    identifiers = [row["id"] for row in rows]
    if (
        facts["schema"] != "OTERYN_CRYSTAL_EQUIPMENT_AUDIT_QUARTER/v1"
        or facts["revision"] != REVISION
        or len(identifiers) != 182
        or facts["companions"] != [6299, 9019, 9020]
        or identifiers != sorted(set(identifiers))
        or identifiers[-1] != 17810
    ):
        raise ValueError("Crystal equipment audit quarter differs")
    selected = {row["id"]: row for row in rows if "held" not in row}
    keys = {target(identifier)["key"] for identifier in selected}
    definitions = {
        row["definition"]["identity"]["key"]: row
        for row in item_rows(root)
        if row["definition"]["identity"]["key"] in keys
    }
    bindings = {
        row["external_id"]: row
        for row in json.loads(
            (root / "imports/crystalserver/bindings/items.json").read_bytes()
        )["bindings"]
    }
    profiles = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    completed = []
    for identifier, fact in selected.items():
        key = target(identifier)["key"]
        definition = definitions[key]["definition"]
        binding = bindings[str(identifier)]
        if (
            binding["disposition"] != "EXACT"
            or binding["target"] != definition["identity"]
        ):
            raise ValueError(f"Crystal equipment binding differs: {identifier}")
        record = profiles.get(key)
        if record is None:
            authoring = copy.deepcopy(definitions[key].get("authoring", {}))
            # Forge eligibility has a separate durable admission projection.
            authoring.pop("forge", None)
            record = {
                "authoring": authoring,
                "semantics": copy.deepcopy(definition["semantics"]),
                "attributes": {},
                "admission": {
                    "materializable": True,
                    "stack_class": "NonStackable",
                    "legal_destinations": ["CharacterInventory", "Ground"],
                },
                "production_definition": {
                    "family": "Item",
                    "production_key": key,
                    "revision_ref": definition["identity"]["revision"],
                },
                "production_binding": binding,
                "production_binding_qualification": None,
            }
            record["authoring"]["item"] = definition["identity"]
            document["records"].append(record)
        elif record["production_binding"] != binding or record["admission"] != {
            "materializable": True,
            "stack_class": "NonStackable",
            "legal_destinations": ["CharacterInventory", "Ground"],
        }:
            raise ValueError(
                f"Crystal equipment previous custody differs: {identifier}"
            )
        complete(record, fact)
        completed.append(record)
    # Match the rendering facts SRX-1 applies to the same native base.
    apply_profile_weights(completed, root)
    apply_top_order(completed, json.loads((root / HOUSE_PROFILES).read_bytes()))
    document["records"].sort(key=order)
