"""Capture and register the fourth quarter of the Crystal equipment audit."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from collections import Counter
from decimal import Decimal
from pathlib import Path

import quest_batch3_item_profiles as crystal
import ranged_item_profiles as ranged
from build_activity_source_world import source_top_orders
from grizzly_shop_profiles import accepted_first
from imbuement_item_profiles import apply_equipment_audit
from import_cyclopedia_global import apply_profile_weights, apply_stats, load_capture
from item_spell_augments import KINDS, parse_item_spell_augments
from lower_elemental_magic_modifier_packet import native_order
from nrwicked_profiles import REWARD_IDS
from protected_loot_yields import ARBAZILOTH_HARD_ITEMS

ROOT = crystal.ROOT
PACKET = "tools/content-schema/native-gameplay/crystal-equipment-slice-4.json"
SCHEMA = "OTERYN_CRYSTAL_EQUIPMENT_SLICE_4/v1"
BASE = "3e09916423181c795c829eca5e5414dde3f29a3e"
ARBAZILOTH_BASE = "b878e74655af20a0883cc91169e344f9724e2fc7"
ARBAZILOTH_KEYS = ARBAZILOTH_HARD_ITEMS | {
    f"oteryn:item.tibia.i{i}" for i in REWARD_IDS
}
BANK_SHA = "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
EXISTING = {43877, 43878, 43879, 43880, 50254, 50255}
DONOR_TEST_ITEMS = {28464, 28465, 28466, 28478, 28479}
HELD = {
    50192
}  # Worn protection and the cross-vocation decay target need a lifecycle owner.
AREA_ARROWS = {53168, 53169, 53170, 53172}
SLOTS = {
    "head": "HEAD",
    "armor": "TORSO",
    "legs": "LEGS",
    "feet": "FEET",
    "hand": "WEAPON",
    "shield": "SHIELD",
    "ammo": "EXTRA",
}
HAND_GROUP = "oteryn:equipment-group.non_quiver_left_hand"
GLOBAL = "docs/reference/content-prep/item-cyclopedia-capture-1/items.jsonl"
SOURCE_FILES = {
    "data/items/items.xml": crystal.XML_SHA256,
    "data/items/appearances.dat": BANK_SHA,
}
GLOBAL_FALLBACK = {
    50259: ["mantra"],
    51268: ["speed"],
    51666: ["weapon_type", "defense", "extra_defense"],
    52964: ["fire_resistance"],
    53233: ["mantra"],
}


def item_key(identifier):
    return f"oteryn:item.tibia.i{identifier}"


def global_augments(text):
    """The Global dialog reports percentages and signed cooldown seconds."""
    augments = []
    for entry in text.split(", "):
        spell, effect = entry.lower().split(" -> ")
        match = re.fullmatch(r"([+-]\d+(?:\.\d+)?)(%|s) (.+)", effect)
        if match is None:
            raise ValueError(f"Unsupported Global spell augment: {entry}")
        number, unit, label = match.groups()
        kind = KINDS[label]
        cooldown = kind == "cooldown"
        value = abs(Decimal(number)) * (1000 if cooldown else 100)
        if (
            unit != ("s" if cooldown else "%")
            or not number.startswith("-" if cooldown else "+")
            or value != int(value)
            or not 0 < value <= (86400000 if cooldown else 100000)
        ):
            raise ValueError(f"Invalid Global spell augment: {entry}")
        augments.append({"spell": spell, "kind": kind, "value": int(value)})
    return augments


def complete_arbaziloth_policy(prior, source):
    """Fill donor facts without replacing the qualified Arbaziloth equipment."""
    key = source["production_definition"]["production_key"]
    if any(
        prior[field] != source[field]
        for field in ("production_binding", "production_definition", "admission")
    ):
        raise ValueError(f"Crystal equipment Arbaziloth custody conflict: {key}")
    semantics = accepted_first(prior["semantics"], source["semantics"])
    authoring = source["authoring"] | prior["authoring"]
    if key == item_key(49520):
        # The Global Cyclopedia measures attack +6. Use the qualified launcher
        # value in the Item weapon facts consumed by Inspection and LOOK.
        semantics["weapon"]["value"]["attack"] = crystal.grizzly.source.K(
            authoring["ranged"]["attack"]
        )
    if key == item_key(49529):
        # The Global capture has no mana value. Crystal registers 21 per shot.
        authoring["ranged"] = source["authoring"]["ranged"]
    return {
        **prior,
        "semantics": semantics,
        "authoring": authoring,
        "attributes": source["attributes"] | prior["attributes"],
    }


def audit_slice(root, xml, appearances):
    """Reproduce the base audit against the three actual manifest providers."""

    def base_json(path):
        return json.loads(
            subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=root)
        )

    manifest = base_json("content/spells.manifest.json")
    profiles = {}
    for field in (
        "item_profiles",
        "item_profile_updates",
        "starting_loadout_item_profiles",
    ):
        path = Path("content") / manifest[field]["path"]
        for row in base_json(path.as_posix())["records"]:
            profiles[row["production_definition"]["production_key"]] = row
    cohort = sorted(
        identifier
        for identifier, row in xml.items()
        if identifier not in DONOR_TEST_ITEMS
        and appearances.get(identifier, {}).get("flags", {}).get("flags.take") is True
        and row["attrs"].keys()
        & {"attack", "defense", "armor", "slottype", "weapontype"}
    )
    materializable, gaps = [], []
    for identifier in cohort:
        profile = profiles.get(item_key(identifier), {})
        if profile.get("admission", {}).get("materializable"):
            materializable.append(identifier)
        equipment = profile.get("semantics", {}).get("equipment", {})
        if not equipment.get("value", {}).get("patterns", {}).get("value"):
            gaps.append(identifier)
    if (len(cohort), len(materializable), len(gaps)) != (1290, 603, 715):
        raise ValueError("Crystal equipment base audit changed")
    # Allocate the remainder to the first three quarters.
    ids = gaps[-(len(gaps) // 4) :]
    if ids[0] != 40588 or set(ids) & set(materializable) != EXISTING:
        raise ValueError("Crystal equipment slice membership changed")
    return ids


def equipment_pattern(attrs):
    source = crystal.grizzly.source
    known = source.K
    weapon = attrs.get("weapontype")
    slot = attrs.get("slot") or attrs.get("slottype")
    if weapon == "ammunition":
        slot = "ammo"
    elif weapon == "distance" or slot == "two-handed":
        slot = "hand"
    primary = SLOTS[slot]
    two_handed = attrs.get("slottype") == "two-handed"
    # A bow shares the off hand with a quiver, while a melee weapon reserves it.
    groups = (
        [HAND_GROUP]
        if primary == "SHIELD" or two_handed and weapon == "distance"
        else []
    )
    vocation = sorted(
        value.upper()
        for value in source.engine_items.parse_vocation_attribute(
            attrs.get("vocation", "")
        )
    )
    if "vocation" in attrs and not vocation:
        raise ValueError(f"Unknown Crystal vocation: {attrs['vocation']}")
    return {
        "pattern_id": 1,
        "primary_slot": known(primary),
        "additional_reserved_slots": known(
            ["SHIELD"] if two_handed and weapon != "distance" else []
        ),
        "mutually_exclusive_groups": known(groups),
        "level": known(int(attrs.get("level", 0))),
        "vocations": known(vocation),
        "compatibility_rule": source.NA,
    }


def modifier_facts(attrs, engine, order, known, unknown):
    skills = {
        "skillsword": "SKILL_SWORD",
        "skillaxe": "SKILL_AXE",
        "skillclub": "SKILL_CLUB",
        "skilldist": "SKILL_DISTANCE",
        "skillfist": "SKILL_FIST",
        "skillshield": "SKILL_SHIELD",
    }
    points = (
        skills
        | {
            "mantra": "MANTRA",
            "reflectdamage": "REFLECT_DAMAGE",
            "magiclevelpoints": "MAGIC_LEVEL_POINTS",
            "speed": "SPEED",
        }
        | {
            field: value.upper() + "_MAGIC_LEVEL_POINTS"
            for field, value in engine.MAGIC_LEVEL_SUFFIX_MAP.items()
        }
    )
    percents = {
        "criticalhitchance": "CRITICAL_HIT_CHANCE",
        "criticalhitdamage": "CRITICAL_HIT_DAMAGE",
        "lifeleechamount": "LIFE_LEECH_AMOUNT",
        "lifeleechchance": "LIFE_LEECH_CHANCE",
        "manaleechamount": "MANA_LEECH_AMOUNT",
        "manaleechchance": "MANA_LEECH_CHANCE",
    }
    parameters = {
        kind: ("SIGNED_POINTS", int(attrs[field]))
        for field, kind in points.items()
        if field in attrs
    }
    parameters.update(
        {
            kind: ("RATIONAL_PERCENT", engine.ratio_value(int(attrs[field]), 100))
            for field, kind in percents.items()
            if field in attrs
        }
    )
    # Physical is the unconverted attack. The typed elemental values only name nonphysical bonds.
    if attrs.get("elementalbond") in ("earth", "energy"):
        parameters["ELEMENTAL_BOND"] = ("ELEMENT", attrs["elementalbond"].upper())
    return [
        {
            "kind": kind,
            "parameter": known(
                {"kind": parameters[kind][0], "value": parameters[kind][1]}
            ),
            "evaluation_phase": unknown,
            "priority": unknown,
            "target_domain": unknown,
        }
        for kind in sorted(parameters, key=order.__getitem__)
    ]


def capture(root, directory):
    source = crystal.grizzly.source
    engine = source.engine_items
    xml = (directory / "items.xml").read_bytes()
    bank = (directory / "appearances.dat").read_bytes()
    if any(
        hashlib.sha256(raw).hexdigest() != SOURCE_FILES[path]
        for path, raw in (
            ("data/items/items.xml", xml),
            ("data/items/appearances.dat", bank),
        )
    ):
        raise ValueError("Crystal equipment source pin mismatch")

    objects = engine.load_appearance_objects(bank)
    item_facts = engine.load_items_xml(xml)
    ids = audit_slice(root, item_facts, objects)
    nodes = source.xml_index(xml)
    portable_ids = set(ids) - HELD
    physical = {
        row["server_item_id"]: row for row in source.policies(bank, xml, portable_ids)
    }
    top_orders = source_top_orders(bank, portable_ids)
    lines = crystal.xml_lines(xml)
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    taxonomy = {
        row["target"]["key"]: row["source_taxonomy"]
        for row in json.loads(
            (root / "content/items/taxonomy/items.json").read_bytes()
        )["records"]
    }
    details = load_capture(root)
    fallback = {
        identifier: details[identifier]["basic"] for identifier in GLOBAL_FALLBACK
    }
    selected_xml = ET.Element("items")
    selected_xml.extend(
        nodes[i]
        for i in ids
        if i not in AREA_ARROWS
        and item_facts[i]["attrs"].get("weapontype") in ("distance", "wand")
    )
    ranged_profiles, held_ranged, _ = ranged.build(ET.tostring(selected_xml))
    if held_ranged:
        raise ValueError(
            f"Unexpected unsupported Crystal ranged profiles: {held_ranged}"
        )

    prior_profiles = json.loads(
        subprocess.check_output(
            [
                "git",
                "show",
                f"{ARBAZILOTH_BASE}:content/items/definitions/spell-native-profiles.json",
            ],
            cwd=root,
        )
    )
    arbaziloth = {
        key: row
        for row in prior_profiles["records"]
        if (key := row["production_definition"]["production_key"]) in ARBAZILOTH_KEYS
    }
    known, unknown, na = source.K, source.U, source.NA
    order = native_order(root)
    rows = []
    for identifier in ids:
        if identifier in HELD:
            continue
        attrs = dict(item_facts[identifier]["attrs"])
        if identifier == 51666:
            ink = fallback[identifier]
            if (
                ink["Body Position"] != "weapon hand"
                or ink["Weapon Type"] != "stabbing weapon"
            ):
                raise ValueError("Ink Sword Global fallback drift")
            defense, extra = ink["Defence"].split(" +")
            attrs.update(
                weapontype="sword", slot="hand", defense=defense, extradef=extra
            )
        if identifier == 52964:
            element, amount = fallback[identifier]["Protection"].split(" +")
            attrs["absorbpercent" + element] = amount.removesuffix("%")

        if identifier in (50259, 53233):
            attrs.setdefault("mantra", fallback[identifier]["Mantra"])
        if identifier == 51268:
            label, points = fallback[identifier]["Skill Boost"].split(" +")
            if label != "speed":
                raise ValueError("Norcferatu goretrampers speed fallback drift")
            attrs.setdefault("speed", points)

        record = source.profile(
            identifier,
            nodes[identifier],
            objects[identifier],
            physical[identifier],
            bindings,
            None,
        )
        if record["admission"]["legal_destinations"] != [
            "CharacterInventory",
            "Ground",
        ] or any(
            record["semantics"][field] != na
            for field in ("temporal", "charges", "container")
        ):
            raise ValueError(
                f"Unexpected nonportable or stateful equipment {identifier}"
            )
        semantics = record["semantics"]
        # Retain identity and numeric policy only. Existing descriptions remain with their owner.
        semantics["presentation"]["value"]["description"] = unknown
        record["attributes"].update(
            speed_bonus=int(attrs.get("speed", 0)),
            always_on_top=top_orders[identifier] != 0,
            always_on_top_order=top_orders[identifier],
        )
        if identifier not in EXISTING:
            text = details.get(identifier, {}).get("basic", {}).get("Augments")
            augments = (
                global_augments(text)
                if text
                else parse_item_spell_augments(nodes[identifier], "crystal")
            )
            record["attributes"]["spell_augments"] = augments
        weapon = attrs.get("weapontype")
        pattern = equipment_pattern(attrs)
        semantics["equipment"] = known({"patterns": known([pattern])})
        vocation = pattern["vocations"]["value"]
        if weapon:
            semantics["weapon"] = known(
                {
                    "weapon_type": known(weapon.upper()),
                    "attack": known(int(attrs.get("attack", 0))),
                    "defense": known(int(attrs.get("defense", 0))),
                    "extra_defense": known(int(attrs.get("extradef", 0))),
                    "range": known(int(attrs.get("range", 1))),
                    "ammunition": known(attrs["ammotype"].upper())
                    if "ammotype" in attrs
                    else na,
                    "hit_chance": known(engine.ratio_value(int(attrs["hitchance"])))
                    if "hitchance" in attrs
                    else na,
                    "max_hit_chance": known(
                        engine.ratio_value(int(attrs["maxhitchance"]))
                    )
                    if "maxhitchance" in attrs
                    else na,
                    "elemental": known(
                        [
                            {
                                "element": element.upper(),
                                "points": known(int(attrs["element" + element])),
                            }
                            for element in ("death", "earth", "energy", "fire", "ice")
                            if "element" + element in attrs
                        ]
                    ),
                }
            )
        semantics["protection"] = known(
            {
                "armor": known(int(attrs.get("armor", 0))),
                "resistances": known(
                    sorted(
                        [
                            {
                                "kind": entry["damage_type"].upper(),
                                "percent": known(entry["reduction_percent"]),
                            }
                            for entry in engine.build_resistances(attrs)
                        ],
                        key=lambda entry: entry["kind"],
                    )
                ),
            }
        )
        semantics["skill_modifiers"] = known(
            {"modifiers": known(modifier_facts(attrs, engine, order, known, unknown))}
        )
        binding = bindings[str(identifier)]
        record["production_binding"] = binding
        record["production_binding_qualification"] = None
        record["production_definition"] = {
            "family": "Item",
            "production_key": binding["target"]["key"],
            "revision_ref": binding["target"]["revision"],
        }
        if binding["target"]["key"] in taxonomy:
            record["authoring"]["taxonomy"] = taxonomy[binding["target"]["key"]]
        if identifier in ranged_profiles:
            profile = ranged_profiles[identifier]
            if profile["kind"] == "WAND" and "DRUID" in vocation:
                profile["kind"] = "ROD"
            record["authoring"]["ranged"] = profile
        if (key := item_key(identifier)) in ARBAZILOTH_KEYS:
            record = complete_arbaziloth_policy(arbaziloth[key], record)
        rows.append(record)
    apply_profile_weights(rows, root)
    return {
        "schema": SCHEMA,
        "base": BASE,
        "arbaziloth_base": ARBAZILOTH_BASE,
        "source_revision": crystal.REVISION,
        "source_files": SOURCE_FILES,
        "source_lines": {str(i): lines[i] for i in ids},
        "global_fallback": {"source": GLOBAL, "fields": GLOBAL_FALLBACK},
        "existing": sorted(EXISTING),
        "held": sorted(HELD),
        "held_ranged": sorted(AREA_ARROWS),
        "records": rows,
    }


def complete_monk_bonds(root, *providers):
    """Retain the Crystal bonds on Monk weapons admitted before the gap audit."""
    from build_source_item_profiles import K, U

    # Crystal 201189f8 data/items/items.xml:84524,84585,84745,84858,86058.
    bonds = {
        50157: "earth",
        50159: "energy",
        50166: "energy",
        50171: "energy",
        51119: "earth",
    }
    order = native_order(root)
    records = {}
    for provider in providers:
        for row in provider["records"]:
            key = row["production_definition"]["production_key"]
            if key in records:
                raise ValueError(f"Duplicate equipment provider record: {key}")
            records[key] = row
    for identifier, bond in bonds.items():
        record = records[item_key(identifier)]
        semantics = record["semantics"]
        if semantics["weapon"]["value"]["weapon_type"] != K("FIST"):
            raise ValueError(f"Crystal bond requires a fist weapon: {identifier}")
        group = semantics["skill_modifiers"].get("value", {})
        modifiers = group.get("modifiers", {}).get("value", []).copy()
        [modifier] = modifier_facts(
            {"elementalbond": bond}, crystal.grizzly.source.engine_items, order, K, U
        )
        existing = [row for row in modifiers if row["kind"] == "ELEMENTAL_BOND"]
        if existing and existing != [modifier]:
            raise ValueError(f"Conflicting Crystal bond: {identifier}")
        if not existing:
            modifiers.append(modifier)
        modifiers.sort(key=lambda row: order[row["kind"]])
        semantics["skill_modifiers"] = K({**group, "modifiers": K(modifiers)})


def apply(root, document, order):
    packet = json.loads((root / PACKET).read_bytes())
    records = packet["records"]
    if (
        packet["schema"] != SCHEMA
        or packet["source_revision"] != crystal.REVISION
        or packet["source_files"] != SOURCE_FILES
        or packet["base"] != BASE
        or packet["arbaziloth_base"] != ARBAZILOTH_BASE
        or packet["existing"] != sorted(EXISTING)
        or packet["held"] != sorted(HELD)
        or packet["held_ranged"] != sorted(AREA_ARROWS)
    ):
        raise ValueError("Crystal equipment receipt pin mismatch")
    ids = [int(row["production_binding"]["external_id"]) for row in records]
    if (
        len(ids) != 177
        or ids != sorted(set(ids))
        or set(map(int, packet["source_lines"])) != set(ids) | HELD
    ):
        raise ValueError("Crystal equipment receipt membership drift")
    by_key = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    details = load_capture(root)
    catalog = json.loads(
        (root / "content/abilities/definitions/player-spells.json").read_bytes()
    )
    spell_names = {
        entry["bundle"]["spell"]["name"].lower() for entry in catalog["bundles"]
    }
    for identifier, record in zip(ids, records, strict=True):
        key = item_key(identifier)
        prior = by_key.get(key)
        if identifier in AREA_ARROWS:
            # The native shot owner cannot settle the donor area attack. Keep
            # the captured policy and trade rows, but refuse materialization.
            if prior is not None or record["authoring"].get("ranged") is not None:
                raise ValueError(f"Unexpected area ammunition policy: {identifier}")
            continue
        for augment in record["attributes"]["spell_augments"]:
            if augment["spell"] not in spell_names:
                raise ValueError(f"Unknown spell augment on {identifier}: {augment}")
        if key in ARBAZILOTH_KEYS:
            if prior is None:
                raise ValueError(f"Missing prior Arbaziloth equipment: {identifier}")
            composed = complete_arbaziloth_policy(prior, record)
            expected = copy.deepcopy(record)
            presentation = expected["semantics"]["presentation"]["value"]
            # A canonical blank description adds no detail to the captured equipment.
            blank = {"state": "KNOWN", "value": ""}
            if (
                presentation["description"] == {"state": "UNKNOWN"}
                and prior["semantics"]["presentation"]["value"]["description"] == blank
            ):
                presentation["description"] = blank
            if identifier in (49524, 49526, 49530):
                # The canonical Global defence supersedes the donor's bonus.
                apply_stats(
                    expected,
                    {"Defence": details[identifier]["basic"]["Defence"]},
                    Counter(),
                )
            if composed != expected:
                raise ValueError(
                    f"Crystal equipment Arbaziloth policy drift: {identifier}"
                )
            prior.update(composed)
            continue
        if prior is None and identifier not in EXISTING:
            document["records"].append(record)
            continue
        if (
            (identifier not in EXISTING and identifier != 50260)
            or prior is None
            or prior["production_binding"] != record["production_binding"]
            or prior["admission"] != record["admission"]
        ):
            raise ValueError(
                f"Crystal equipment overlaps established policy: {identifier}"
            )
        for field in ("equipment", "weapon", "protection", "skill_modifiers"):
            selected = record["semantics"][field]
            previous = prior["semantics"][field]
            agrees = previous in ({"state": "UNKNOWN"}, selected)
            if not agrees and identifier == 50260 and field == "equipment":
                # Outfit boss loot already admits the Monk torso. Retain its
                # known requirements while completing the donor pattern.
                [old_pattern] = previous["value"]["patterns"]["value"]
                [new_pattern] = selected["value"]["patterns"]["value"]
                agrees = all(
                    value == {"state": "UNKNOWN"} or new_pattern.get(key) == value
                    for key, value in old_pattern.items()
                )
            elif not agrees and identifier == 50260 and field == "skill_modifiers":
                agrees = all(
                    value in selected["value"]["modifiers"]["value"]
                    for value in previous["value"]["modifiers"]["value"]
                )
            if not agrees:
                raise ValueError(
                    f"Crystal equipment conflicts with native {field}: {identifier}"
                )
            prior["semantics"][field] = selected
        prior["semantics"]["physical"]["value"]["weight"] = record["semantics"][
            "physical"
        ]["value"]["weight"]
        captured_ranged = record["authoring"].get("ranged")
        if (
            captured_ranged is not None
            and prior["authoring"].get("ranged") != captured_ranged
        ):
            raise ValueError(
                f"Crystal equipment conflicts with native ranged policy: {identifier}"
            )
    apply_equipment_audit(root, document)
    document["records"].sort(key=order)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source_directory", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = capture(ROOT, args.source_directory)
    records = packet.pop("records")
    header = json.dumps(packet, indent=2, ensure_ascii=False)
    rows = ",\n".join(
        "    " + json.dumps(row, ensure_ascii=False, separators=(",", ":"))
        for row in records
    )
    output = header[:-2] + ',\n  "records": [\n' + rows + "\n  ]\n}\n"
    path = ROOT / PACKET
    if args.check:
        if path.read_text() != output:
            raise SystemExit("Crystal equipment source receipt is stale")
    else:
        path.write_text(output)
