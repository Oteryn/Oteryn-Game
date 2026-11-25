"""Lower Crystal's pinned equipment slice through the existing native Item provider."""

import json
import sys
import xml.etree.ElementTree as ET

from crystal_equipment_quarter_1 import NON_QUIVER_HAND
from imbuement_item_profiles import ORDER, TIERS
from import_cyclopedia_global import apply_profile_weights

FACTS = "imports/crystalserver/facts/equipment-slice-two.json"
# Missing combat owners: level-gated quiver ammo, area projectiles, innate
# melee elemental damage, and deadline-bearing stacks.
HELD = {22047, 22048, 22049, 22119, 22759, 22760, 25757, 25758}
# These quest rewards can already have a native Item row when the quest units land first.
QUEST_REWARDS = {19356, 19362, 19363, 19366, 19373, 19391}
# Crystal's simple-arrow range has no ammo type. The exact-ID TibiaWiki
# observations identify Arrow (Weak), Bolt (Weak) and Simple Arrow (Weak).
WEAK_AMMO = {22043: "ARROW", 22046: "BOLT", 22050: "ARROW"}
SOURCES = {
    "data/items/items.xml": "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
    "data/items/appearances.dat": "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
}
SLOTS = {
    0: "WEAPON",
    1: "HEAD",
    2: "AMULET",
    4: "TORSO",
    5: "SHIELD",
    6: "WEAPON",
    7: "LEGS",
    8: "FEET",
}
VOCATIONS = ("DRUID", "KNIGHT", "MONK", "PALADIN", "SORCERER")
WEAPONS = {
    "ammo": "AMMUNITION",
    "ammunition": "AMMUNITION",
    "axe": "AXE",
    "club": "CLUB",
    "distance": "DISTANCE",
    "missile": "DISTANCE",
    "shield": "SHIELD",
    "spellbook": "SPELLBOOK",
    "sword": "SWORD",
    "wand": "WAND",
}
REGENERATION = {
    "healthgain": ("HEALTH_GAIN", "SIGNED_POINTS"),
    "healthticks": ("HEALTH_TICKS", "MILLISECONDS"),
    "managain": ("MANA_GAIN", "SIGNED_POINTS"),
    "manaticks": ("MANA_TICKS", "MILLISECONDS"),
}
TRANSFORMS = (
    "ROTATE",
    "WRAP",
    "USE",
    "EQUIP",
    "DEEQUIP",
    "MALE",
    "FEMALE",
    "DESTROY",
    "DECAY",
    "WRITE_ONCE",
)


def known(value):
    return {"state": "KNOWN", "value": value}


NA = {"state": "NOT_APPLICABLE"}
UNKNOWN = {"state": "UNKNOWN"}


def item_key(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def target(item_id):
    return {"key": item_key(item_id), "revision": "definition-r1"}


def transforms(kind, item_id):
    return known(
        {
            "targets": [
                {
                    "kind": value,
                    "target": known(target(item_id)) if value == kind else NA,
                }
                for value in TRANSFORMS
            ]
        }
    )


def point_ratio(value):
    return {"numerator": int(value), "denominator": 1}


def weapon_facts(row, attrs, registration):
    item_id = row["id"]
    # Crystal registers the nested weapon type after the outer attributes.
    kind = WEAPONS.get(registration.get("weapontype", attrs.get("weapontype")))
    if item_id == 20341:
        kind = "SWORD"  # Light Rapier, TibiaWiki revision 1147851.
    ammo = WEAK_AMMO.get(item_id, attrs.get("ammotype", "").upper())
    if item_id in WEAK_AMMO:
        kind = "AMMUNITION"
    if kind is None:
        if row.get("clothes") == 5:
            kind = "SHIELD"
        else:
            return NA, None
    attack = int(attrs.get("attack", 0))
    range_cells = int(attrs.get("range", 1))
    hit_chance = int(attrs.get("hitchance", 0))
    max_hit_chance = int(attrs.get("maxhitchance", 90 if ammo else 75))
    elements = {
        key[7:].upper(): int(value)
        for key, value in attrs.items()
        if key.startswith("element")
    }
    semantics = {
        "weapon_type": known(kind),
        "attack": known(attack),
        "defense": known(int(attrs.get("defense", 0))),
        "extra_defense": known(int(attrs.get("extradef", 0))),
        "range": known(range_cells),
        "hit_chance": known(point_ratio(hit_chance)) if "hitchance" in attrs else NA,
        "max_hit_chance": known(point_ratio(max_hit_chance))
        if "maxhitchance" in attrs
        else NA,
        "ammunition": known(ammo) if ammo else NA,
        "elemental": known(
            [
                {"element": element, "points": known(points)}
                for element, points in elements.items()
            ]
        ),
    }
    ranged = None
    if kind == "WAND":
        ranged = {
            "kind": "ROD" if attrs.get("primarytype") == "rods" else "WAND",
            "range_cells": range_cells,
            "element": registration["wandtype"].upper(),
            "mana_per_shot": int(registration["mana"]),
            "damage_min": int(registration["fromdamage"]),
            "damage_max": int(registration["todamage"]),
        }
    elif kind in ("DISTANCE", "AMMUNITION"):
        ranged = {
            "kind": kind if ammo else "THROWING",
            "attack": attack,
            "hit_chance": hit_chance,
            "max_hit_chance": max_hit_chance,
        }
        if ammo:
            ranged["ammunition"] = ammo
        if kind == "DISTANCE":
            ranged["range_cells"] = range_cells
        if ranged["kind"] != "DISTANCE":
            ranged["element"], ranged["elemental_attack"] = next(
                iter(elements.items()), ("PHYSICAL", 0)
            )
    return known(semantics), ranged


def profile(native, lowering, row, bindings):
    item_id = row["id"]
    attrs = row["xml"]
    registration = row.get("registration", {})
    binding = bindings[str(item_id)]
    identity = {"family": "Item", **target(item_id)}
    if binding["disposition"] != "EXACT" or binding["target"] != identity:
        raise ValueError(f"equipment source binding {item_id}")
    node = ET.Element("item", {"name": row["name"]})
    for key, value in attrs.items():
        ET.SubElement(node, "attribute", {"key": key, "value": value})
    record = native.profile(
        item_id,
        node,
        {"flags": {"flags.cumulative": row["stackable"]}},
        {
            "movable": True,
            "pickupable": True,
            "block_solid": False,
            "block_projectile": False,
            "has_height": False,
        },
        bindings,
        None,
    )
    record["production_binding"] = binding
    record["production_binding_qualification"] = None
    record["production_definition"] = {
        "family": "Item",
        "production_key": identity["key"],
        "revision_ref": identity["revision"],
    }
    record["authoring"]["item"] = identity
    record["attributes"].update(always_on_top=False, always_on_top_order=0)
    semantics = record["semantics"]
    semantics["weapon"], ranged = weapon_facts(row, attrs, registration)
    if ranged is not None:
        record["authoring"]["ranged"] = ranged
    # 19400 has a registered club weapon and the ItemType default hand slot.
    if item_id == 19400:
        slot = "WEAPON"
    elif item_id in WEAK_AMMO:
        slot = "EXTRA"
    else:
        slot = SLOTS[row["clothes"]]
    source_vocations = registration.get("vocation", "").upper()
    vocations = [vocation for vocation in VOCATIONS if vocation in source_vocations]
    if source_vocations and not vocations:
        raise ValueError(f"equipment source vocations {item_id}")
    two_handed = attrs.get("slottype") == "two-handed"
    launcher = ranged is not None and ranged["kind"] == "DISTANCE"
    semantics["equipment"] = known(
        {
            "patterns": known(
                [
                    {
                        "pattern_id": 1,
                        "primary_slot": known(slot),
                        "compatibility_rule": NA,
                        "additional_reserved_slots": known(
                            ["SHIELD"] if two_handed and not launcher else []
                        ),
                        "mutually_exclusive_groups": known(
                            [NON_QUIVER_HAND] if launcher or slot == "SHIELD" else []
                        ),
                        "level": known(int(registration.get("level", 0))),
                        "vocations": known(vocations),
                    }
                ]
            )
        }
    )
    _, resistance = lowering.canary_resistances(attrs)
    if isinstance(resistance, str):
        raise TypeError(f"equipment source resistance {item_id}: {resistance}")
    semantics["protection"] = known(
        {
            "armor": known(int(attrs.get("armor", 0))),
            "resistances": known(resistance or []),
        }
    )
    modifier_types = {
        key: (value, "SIGNED_POINTS")
        for key, value in lowering.CANARY_MODIFIER_POINTS.items()
    }
    modifier_types.update(REGENERATION)
    modifiers = [
        {
            "kind": kind,
            "parameter": known({"kind": type_name, "value": int(attrs[key])}),
            "target_domain": UNKNOWN,
            "evaluation_phase": UNKNOWN,
            "priority": UNKNOWN,
        }
        for key, (kind, type_name) in modifier_types.items()
        if key in attrs
    ]
    semantics["skill_modifiers"] = known(
        {"modifiers": known(sorted(modifiers, key=lambda m: m["kind"]))}
    )
    if imbue := row.get("imbuement"):
        families = imbue["families"]
        semantics["imbuement"] = known(
            {
                "slot_count": known(imbue["slot_count"]),
                "allowed_family_tiers": known(
                    [
                        {"family": family, "tier": TIERS[families[family]]}
                        for family in ORDER
                        if family in families
                    ]
                ),
                "excluded_families": known([]),
            }
        )
    if "transformdeequipto" in attrs:
        semantics["use_transform"] = transforms(
            "DEEQUIP", int(attrs["transformdeequipto"])
        )
        semantics["temporal"]["value"]["consumption_mode"] = known(
            "AUTHORITATIVE_ACTIVE_TIME_BUDGET"
        )
    semantics["trade_restrictions"] = known(
        {
            "marketable": known(row["marketable"]),
            "tradeable": UNKNOWN,
            "vocations": UNKNOWN,
            "account_binding_policy": UNKNOWN,
            "character_binding_policy": UNKNOWN,
        }
    )
    return record


def source_rows(root):
    source = json.loads((root / FACTS).read_bytes())
    if (
        source["schema"] != "OTERYN_CRYSTAL_EQUIPMENT_SLICE/v1"
        or source["revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        or source["sources"] != SOURCES
    ):
        raise ValueError("equipment slice source")
    rows = {row["id"]: row for row in source["records"]}
    if (
        len(rows) != 183
        or len(rows) != len(source["records"])
        or not HELD <= rows.keys()
        or {i for i, row in rows.items() if row.get("paired_dependency")}
        != {22060, 22061, 23474, 23477}
    ):
        raise ValueError("equipment slice cohort")
    return rows


def complete_paired_base(record, item_id, rows, lowering):
    active = int(rows[item_id]["xml"]["transformequipto"])
    if int(rows[active]["xml"]["transformdeequipto"]) != item_id:
        raise ValueError(f"equipment pair source {item_id}")
    # Regular combat reads armor and resistance from the durable base definition.
    _, resistances = lowering.canary_resistances(rows[active]["xml"])
    if isinstance(resistances, str):
        raise TypeError(f"equipment pair resistance {item_id}: {resistances}")
    record["semantics"]["use_transform"] = transforms("EQUIP", active)
    record["semantics"]["protection"] = known(
        {
            "armor": known(int(rows[active]["xml"]["armor"])),
            "resistances": known(resistances or []),
        }
    )


def complete_boss_yield_pairs(root, items):
    import lower_equip_abilities_packet as lowering

    rows = source_rows(root)
    by_key = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    for item_id in (23474, 23477):
        current = by_key[item_key(item_id)]
        if (
            current["semantics"]["use_transform"] != UNKNOWN
            or current["semantics"]["protection"] != UNKNOWN
        ):
            raise ValueError(f"equipment paired source predecessor {item_id}")
        complete_paired_base(current, item_id, rows, lowering)


def admit(root, items, order):
    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles as native
    import lower_equip_abilities_packet as lowering
    from grizzly_shop_profiles import accepted_first, known_conflict

    rows = source_rows(root)
    bindings = native.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    existing = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    complete_boss_yield_pairs(root, items)
    admitted = []
    for item_id, row in rows.items():
        if item_id in HELD | {23474, 23477}:
            continue
        record = profile(native, lowering, row, bindings)
        if "transformequipto" in row["xml"]:
            complete_paired_base(record, item_id, rows, lowering)
        if (prior := existing.get(item_key(item_id))) is not None:
            if item_id not in QUEST_REWARDS | {17810} or any(
                prior[field] != record[field]
                for field in (
                    "production_binding",
                    "production_definition",
                    "admission",
                )
            ):
                raise ValueError(f"equipment source identity collision {item_id}")
            if item_id in QUEST_REWARDS:
                # The quest row owns custody, authoring identity and already
                # qualified values. Complete its equipment facts from Crystal.
                if prior.get("production_binding_qualification") != record.get(
                    "production_binding_qualification"
                ):
                    raise ValueError(f"equipment source qualification collision {item_id}")
                for field in ("equipment", "weapon", "protection", "skill_modifiers", "imbuement"):
                    prior["semantics"][field] = accepted_first(
                        prior["semantics"].get(field, UNKNOWN), record["semantics"][field]
                    )
                if "ranged" in record["authoring"]:
                    prior["authoring"].setdefault("ranged", record["authoring"]["ranged"])
                continue
            # Both audit slices include the spike shield. Keep the first slice's
            # shield exclusion group and complete the other facts from slice two.
            pattern = prior["semantics"]["equipment"]["value"]["patterns"]["value"][0]
            selected = record["semantics"]["equipment"]["value"]["patterns"]["value"][0]
            selected["mutually_exclusive_groups"] = pattern["mutually_exclusive_groups"]
            if known_conflict(prior["semantics"], record["semantics"]):
                raise ValueError(f"equipment source semantics collision {item_id}")
            prior["semantics"] = accepted_first(prior["semantics"], record["semantics"])
            prior["attributes"] = record["attributes"] | prior["attributes"]
            continue
        admitted.append(record)
    apply_profile_weights(admitted, root)
    items["records"].extend(admitted)
    items["records"].sort(key=order)
