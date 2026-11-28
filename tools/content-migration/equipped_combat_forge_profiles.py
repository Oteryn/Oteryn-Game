"""Reconcile the audited combat and Forge cohorts in the active Item provider.

Crystal main 201189f8 items.xml supplies combat literals and weapon equip scripts.
Its appearances.dat supplies portable, nonstacking Forge membership. The class
caps from protocolgame.cpp:6781-6795 and configmanager.cpp:314 agree with every
retained TibiaWiki Forge289 owner in the Item tree.
"""

import hashlib
import json
import re
import xml.etree.ElementTree as ET

import house_decoration_actions
from forge_item_profiles import owners, project
from grizzly_shop_profiles import accepted_first, known_conflict
from import_cyclopedia_global import load_capture
from starting_loadout_profiles import FRAGMENT

FACTS = "imports/crystalserver/facts/equipped-combat-forge.json"
PREFIX = "oteryn:item.tibia.i"
XML_SHA = "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
APPEARANCE_SHA = "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
NATIVE_CUSTODY = {
    "materializable": True,
    "stack_class": "NonStackable",
    "legal_destinations": ["CharacterInventory", "Ground"],
}
# Crystal's Forge cohort also supplies the loot classification for these weapons.
FORGE_LOOT_OVERLAPS = {3309, 7423, 7453, 8096, 8100, 11692, 11693}


def records(document):
    return {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }


def global_combat(fact, capture):
    stats = fact["stats"].copy()
    basic = capture.get(fact["id"], {}).get("basic", {})
    for name, label in [
        ("attack", "Attack"),
        ("defense", "Defence"),
        ("armor", "Armour"),
    ]:
        if name in stats:
            observed = re.fullmatch(r"(\d+)(?: \+ ?\d+)?", basic.get(label, ""))
            if observed:
                stats[name] = int(observed[1])
    if "range" in stats:
        observed = re.search(r"(\d+) fields?$", basic.get("Attack", ""))
        if observed:
            stats["range"] = int(observed[1])
    return stats


def apply_combat(record, fact, source, capture):
    semantics = record["semantics"]
    stats = global_combat(fact, capture)
    shield_type = fact.get("shield_type")
    if "armor" in stats:
        protection = semantics["protection"]
        if protection["state"] != "KNOWN":
            protection = source.K({"resistances": source.U})
            semantics["protection"] = protection
        protection["value"]["armor"] = source.K(stats["armor"])
        return

    weapon = semantics["weapon"]
    if weapon["state"] != "KNOWN":
        weapon = source.K(
            dict.fromkeys(
                (
                    "weapon_type",
                    "attack",
                    "defense",
                    "extra_defense",
                    "range",
                    "hit_chance",
                    "max_hit_chance",
                    "ammunition",
                    "elemental",
                ),
                source.U,
            )
        )
        semantics["weapon"] = weapon
    for name, value in stats.items():
        weapon["value"][name] = source.K(value)
    if shield_type:
        # The hands' defence reader also requires the weapon kind and extra
        # defence. Crystal's ItemType defaults extra defence to zero.
        for name, value in [("weapon_type", shield_type), ("extra_defense", 0)]:
            if weapon["value"][name] == source.U:
                weapon["value"][name] = source.K(value)
    if "range" in stats:
        ranged = record["authoring"]["ranged"]
        if ranged["range_cells"] not in (fact["stats"]["range"], stats["range"]):
            raise ValueError(f"CRYSTAL_RANGED_PROFILE_DRIFT: {fact['id']}")
        ranged["range_cells"] = stats["range"]


def portable_profile(fact, owner, source, bindings, packet):
    definition = owner["definition"]
    identifier = fact["id"]
    name = definition["semantics"]["presentation"]["value"]["name"]["value"]
    node = ET.Element("item", name=name)
    for key, value in fact["attributes"].items():
        ET.SubElement(node, "attribute", key=key, value=value)
    if "script" in fact:
        script = ET.SubElement(node, "attribute", key="script")
        for key, value in fact["script"].items():
            ET.SubElement(script, "attribute", key=key, value=value)
    # The pinned capture establishes ordinary portable, nonstacking,
    # nonblocking equipment with no timed or container state for this cohort.
    record = source.profile(
        identifier,
        node,
        {"flags": packet["forge_flags"]},
        packet["forge_physical"],
        bindings,
        None,
    )
    record["semantics"] = accepted_first(definition["semantics"], record["semantics"])
    return record


def load_facts(root):
    packet = json.loads((root / FACTS).read_bytes())
    combat, forge = packet["combat"], packet["forge"]
    if (
        packet["schema"] != "OTERYN_CRYSTAL_COMBAT_FORGE_FACTS/v1"
        or packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        or packet["items_xml_sha256"] != XML_SHA
        or packet["appearances_sha256"] != APPEARANCE_SHA
        or len(combat) != 88
        or len({fact["id"] for fact in combat}) != 88
        or len(forge) != 215
        or len({fact["id"] for fact in forge}) != 215
    ):
        raise ValueError("CRYSTAL_COMBAT_FORGE_FACTS")
    return packet


def forge_owners(root, facts):
    admitted = owners(root)
    result = {}
    for fact in facts:
        identifier = fact["id"]
        key = f"{PREFIX}{identifier}"
        owner = admitted[key]
        classification = fact["classification"]
        max_tier = {1: 1, 2: 2, 3: 3, 4: 10}[classification]
        if owner["authoring"]["forge"] != {
            "classification": classification,
            "max_tier": max_tier,
        }:
            raise ValueError(f"CRYSTAL_FORGE_OWNER_CONFLICT:{identifier}")
        result[key] = owner
    return result


def apply(root, items, generated, manifest_path, order, canonical):
    import build_source_item_profiles as source

    packet = load_facts(root)
    combat, forge = packet["combat"], packet["forge"]

    native = records(items)
    starters = json.loads(generated[FRAGMENT])
    starter_records = records(starters)
    active = native | starter_records
    capture = load_capture(root)
    for fact in combat:
        key = f"{PREFIX}{fact['id']}"
        apply_combat(active[key], fact, source, capture)

    admitted = forge_owners(root, forge)
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    new, cohort = [], []
    for fact in forge:
        key = f"{PREFIX}{fact['id']}"
        if key not in active:
            new.append(portable_profile(fact, admitted[key], source, bindings, packet))
        else:
            record = active[key]
            if fact["id"] in FORGE_LOOT_OVERLAPS:
                qualified = portable_profile(fact, admitted[key], source, bindings, packet)
                observed = qualified["semantics"]["classification"]
                existing = record["semantics"]["classification"]
                if (
                    record["production_binding"] != bindings[str(fact["id"])]
                    or observed["state"] != "KNOWN"
                    or known_conflict(existing, observed)
                ):
                    raise ValueError(f"CRYSTAL_FORGE_LOOT_CLASSIFICATION:{key}")
                record["semantics"]["classification"] = accepted_first(existing, observed)
            cohort.append(record)

    bound = source.bind_profiles(
        {"schema": items["schema"], "records": new},
        bindings,
        {fact["id"] for fact in forge},
    )["records"]
    cohort.extend(bound)
    project({"records": cohort}, admitted)
    for record in cohort:
        patterns = record["semantics"]["equipment"]["value"]["patterns"]["value"]
        slots = {pattern["primary_slot"]["value"] for pattern in patterns}
        if slots != {"WEAPON"} or record["admission"] != NATIVE_CUSTODY:
            raise ValueError(
                f"CRYSTAL_FORGE_PORTABLE_WEAPON:{record['production_definition']}"
            )

    # The House owner also applies appearance ordering during SRX-1 composition.
    # Lower it here for these late additions so SRX-1 stays a writing/potion delta.
    house = json.loads((root / house_decoration_actions.PROFILES).read_bytes())
    house_decoration_actions.apply_top_order(bound, house)
    items["records"].extend(bound)
    items["records"].sort(key=order)

    # Preserve the starter supplement's existing record-by-record encoding.
    lines = [
        "    " + json.dumps(row, sort_keys=True, separators=(",", ":"))
        for row in starters["records"]
    ]
    generated[FRAGMENT] = (
        '{\n  "schema": "OTERYN_NATIVE_ITEM_PROFILES/v1",\n  "records": [\n'
        + ",\n".join(lines)
        + "\n  ]\n}\n"
    ).encode()
    manifest = json.loads(generated[manifest_path])
    manifest["starting_loadout_item_profiles"]["sha256"] = hashlib.sha256(
        generated[FRAGMENT]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)
