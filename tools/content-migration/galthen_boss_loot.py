"""Close the existing Galthen loot tables with exact Canary carried Item policies."""

import argparse
import copy
import hashlib
import json
import subprocess
from pathlib import Path

import boss_corpse_profiles as base
import item_detail_source_facts as item_details
import m25_corpse_items
from import_cyclopedia_global import apply_profile_weights

ROOT = base.ROOT
PATH = "tools/content-schema/native-gameplay/galthen-boss-loot.json"
LEGACY_ROBES = (
    "tools/content-schema/native-gameplay/galthen-boss-loot-legacy-robes.json"
)
LEGACY_REVISIONS = (
    "e2152d050038cecdb3728c222af2ac184e1cac5a",
    "25e8bba17f2077e225aad34186995a5d66a53685",
    "50ef5550ef239478b056a8fa0adb82392ec5f3a1",
)
NATIVE_PROFILES = "content/items/definitions/spell-native-profiles.json"
GLOBAL_DETAILS = "tools/content-schema/native-gameplay/galthen-global-item-details.json"
PIN_PATH = "tools/content-schema/native-gameplay/galthen-boss-loot-pin.json"
RUST_PIN = "crates/gameplay/content/src/content/native_gameplay/galthen_loot_pin.rs"
IDS = (
    40527,
    40528,
    40529,
    40535,
    40578,
    44438,
    44439,
    44619,
    44620,
    44636,
    44637,
    44642,
    44643,
    44648,
    44649,
    44727,
    44728,
    50255,
    50291,
)
CANARY = m25_corpse_items.REVISIONS["canary"]
SOURCES = {
    **m25_corpse_items.SOURCE_HASHES["canary"],
    "data-otservbr-global/monster/undeads/ahau.lua": "9236c2dd65d960e81efc5ed8e9fe251967147856d3fe2e2322013d83ced0d3d8",
}


def complete_source_abilities(row, attributes):
    """Reuse the equipment owner's typed, signed modifier/resistance decoder."""
    import lower_mantra_bond_modifier_packet as mantra
    from lower_equip_abilities_packet import canary_modifiers, canary_resistances

    _, modifiers = canary_modifiers(
        {key: value for key, value in attributes.items() if key != "mantra"}
    )
    _, resistances = canary_resistances(attributes)
    if any(
        value is not None and not isinstance(value, list)
        for value in (modifiers, resistances)
    ):
        raise ValueError("M25_LOOT_EQUIPMENT_SOURCE")
    if "mantra" in attributes:
        typed = mantra.whole_vector(
            {"mantra": [attributes["mantra"]]}, mantra.shared.native_order()
        )
        if typed is None:
            raise ValueError("M25_LOOT_EQUIPMENT_SOURCE")
        modifiers = sorted(
            (modifiers or []) + typed["value"], key=lambda entry: entry["kind"]
        )
    armor = None
    if "armor" in attributes:
        try:
            armor = int(attributes["armor"])
        except (TypeError, ValueError) as error:
            raise ValueError("M25_LOOT_EQUIPMENT_SOURCE") from error
        if not -(2**31) <= armor < 2**31:
            raise ValueError("M25_LOOT_EQUIPMENT_SOURCE")
    # Validate both groups before publishing either one.
    if modifiers is not None:
        row["semantics"]["skill_modifiers"] = base.source.K(
            {"modifiers": base.source.K(modifiers)}
        )
    if resistances is not None or armor is not None:
        row["semantics"]["protection"] = base.source.K(
            {
                "armor": base.source.K(armor) if armor is not None else base.source.U,
                "resistances": base.source.K(resistances)
                if resistances is not None
                else base.source.U,
            }
        )


def table_proofs(root):
    selected = []
    used = set()
    for path in sorted((root / "content/loot").glob("loot-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            table = record["definition"]
            if table["identity"]["key"] in (
                "oteryn:loot.creature.ahau",
                "oteryn:loot.creature.mitmah_vanguard",
            ):
                used.update(entry["item"]["key"] for entry in table["entries"])
                selected.append(
                    {
                        "key": table["identity"]["key"],
                        "sha256": hashlib.sha256(base.encode(table)).hexdigest(),
                    }
                )
    if len(selected) != 2 or not {f"oteryn:item.tibia.i{item}" for item in IDS} <= used:
        raise ValueError("M25_LOOT_EXACT_TABLES")
    return sorted(selected, key=lambda row: row["key"])


def capture(canary, crystal, root=ROOT):
    import imbuement_item_profiles as imbue
    from build_full_spell_artifact import native_loot_yield_facts

    blobs = {path: (canary / path).read_bytes() for path in SOURCES}
    if any(
        hashlib.sha256(raw).hexdigest() != SOURCES[path] for path, raw in blobs.items()
    ):
        raise ValueError("M25_LOOT_SOURCE_HASH")
    source = base.source
    crystal_xml = (crystal / "data/items/items.xml").read_bytes()
    if (
        hashlib.sha256(crystal_xml).hexdigest()
        != base.SOURCE_HASHES["data/items/items.xml"]
    ):
        raise ValueError("M25_LOOT_CRYSTAL_SOURCE_HASH")
    from crystal_equipment_slice_4 import equipment_pattern

    # Crystal's XML carries the robe's Monk equip requirements. Extend this
    # exact co-owner record so every consumer can still compare it in full.
    robe = source.engine_items.load_items_xml(crystal_xml)[50255]["attrs"]
    robe_patterns = source.K({"patterns": source.K([equipment_pattern(robe)])})
    slots = imbue.slots(blobs["data/items/items.xml"], set(IDS))
    crystal_slots = imbue.slots(crystal_xml, set(IDS))
    if {item: row["slot_count"] for item, row in slots.items()} != {
        item: row["slot_count"] for item, row in crystal_slots.items()
    }:
        raise ValueError("M25_LOOT_IMBUEMENT_DONOR_CONFLICT")
    # Existing Imbuement policy makes Canary the value source. Crystal201189f
    # has OT tier10 boots and omits one robe family; retain those differences
    # as evidence rather than importing them into the current Tibia policies.
    differences = {
        item: {"canary": row, "crystal": crystal_slots[item]}
        for item, row in slots.items()
        if row != crystal_slots[item]
    }
    if set(differences) != {44648, 44649, 50255}:
        raise ValueError("M25_LOOT_IMBUEMENT_DIFFERENCE_SET")
    xml = source.xml_index(blobs["data/items/items.xml"])
    objects = source.engine_items.load_appearance_objects(
        blobs["data/items/appearances.dat"]
    )
    physics = {
        row["server_item_id"]: row
        for row in source.policies(
            blobs["data/items/appearances.dat"], blobs["data/items/items.xml"], set(IDS)
        )
    }
    bindings = base.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    taxonomy = {
        row["target"]["key"]: row["source_taxonomy"]
        for row in json.loads(
            (root / "content/items/taxonomy/items.json").read_bytes()
        )["records"]
    }
    records = []
    for item in IDS:
        binding = bindings[str(item)]
        if binding["disposition"] != "EXACT" or binding["target"] != {
            "family": "Item",
            "key": f"oteryn:item.tibia.i{item}",
            "revision": "definition-r1",
        }:
            raise ValueError("M25_LOOT_EXISTING_IDENTITY")
        row = source.profile(
            item, xml.get(item), objects[item], physics[item], bindings, None
        )
        base.complete_source_top_order(row, objects[item])
        complete_source_abilities(row, source.attrs(xml[item]) if item in xml else {})
        if item == 50255:
            row["semantics"]["equipment"] = robe_patterns
        if item in slots:
            fact = slots[item]
            row["semantics"]["imbuement"] = source.K(
                {
                    "slot_count": source.K(fact["slot_count"]),
                    "allowed_family_tiers": source.K(
                        [
                            {
                                "family": family,
                                "tier": imbue.TIERS[fact["families"][family]],
                            }
                            for family in imbue.ORDER
                            if family in fact["families"]
                        ]
                    ),
                    "excluded_families": source.K([]),
                }
            )
        row["production_binding"] = copy.deepcopy(binding)
        row["production_binding_qualification"] = None
        row["production_definition"] = {
            "family": "Item",
            "production_key": binding["target"]["key"],
            "revision_ref": binding["target"]["revision"],
        }
        if binding["target"]["key"] in taxonomy:
            row["authoring"]["taxonomy"] = copy.deepcopy(
                taxonomy[binding["target"]["key"]]
            )
        if item == 50255:
            # The verified Global client shows 128.00 oz for Stoic Iks robe.
            apply_profile_weights([row], root)
        if native_loot_yield_facts(binding["target"], row) is None:
            raise ValueError("M25_LOOT_CARRIED_SOURCE:" + str(item))
        records.append(row)
    return {
        "source_revision": CANARY,
        "tables": table_proofs(root),
        "crystal_cross_check": {
            "revision": base.PIN,
            "path": "data/items/items.xml",
            "sha256": base.SOURCE_HASHES["data/items/items.xml"],
        },
        "imbuement_slots": slots,
        "imbuement_value_source": "Canary47df, matching the existing Imbuement owner and no-OT-features decision",
        "crystal_family_differences": differences,
        "sources": [
            {"repository": "opentibiabr/canary", "path": path, "sha256": sha}
            for path, sha in SOURCES.items()
        ],
        "records": records,
    }


def admit(root, generated, destination, manifest_path, order, canonical):
    raw = (root / PATH).read_bytes()
    if json.loads((root / PIN_PATH).read_bytes()) != {
        "path": PATH,
        "sha256": hashlib.sha256(raw).hexdigest(),
    }:
        raise ValueError("M25_LOOT_CAPTURE_PIN")
    packet = json.loads(raw)
    if (
        packet["source_revision"] != CANARY
        or packet["tables"] != table_proofs(root)
        or [int(row["production_binding"]["external_id"]) for row in packet["records"]]
        != list(IDS)
    ):
        raise ValueError("M25_LOOT_CLOSED_TABLES")
    document = json.loads(generated[destination])
    existing = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    weights = item_details.load_weight_facts(root, generated)
    details = global_details(root, packet["records"])
    for row in packet["records"]:
        key = row["production_definition"]["production_key"]
        identifier = int(row["production_binding"]["external_id"])
        if identifier in weights:
            item_details.apply_weight(row, identifier, weights)
            if key in existing:
                item_details.apply_weight(existing[key], identifier, weights)
        if key in existing and existing[key] != row:
            raise ValueError("M25_LOOT_KNOWN_POLICY_CONFLICT:" + key)
        if key not in existing:
            document["records"].append(row)
    document["records"].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[destination]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)
    generated[GLOBAL_DETAILS] = canonical(details)


def global_details(root, source):
    from forge_item_profiles import owners, project
    from import_cyclopedia_global import load_capture
    from ranged_item_profiles import apply_authoring

    records = copy.deepcopy(source)
    document = {"records": records}
    weights = item_details.qualified_weights(root)
    for row in records:
        identifier = int(row["production_binding"]["external_id"])
        if identifier in weights:
            item_details.apply_weight(row, identifier, weights)
    item_details.apply_capture_stats(document, load_capture(root))
    project(document, owners(root))
    apply_authoring(root, document)
    details = {}
    for donor, active in zip(source, records, strict=True):
        fields = {}
        if donor["semantics"] != active["semantics"]:
            fields["semantics"] = active["semantics"]
        if donor["authoring"].get("forge") != active["authoring"].get("forge"):
            fields["forge"] = active["authoring"]["forge"]
        if donor["authoring"].get("ranged") != active["authoring"].get("ranged"):
            fields["ranged"] = active["authoring"]["ranged"]
        if fields:
            details[donor["production_binding"]["external_id"]] = fields
    return details


def legacy_robes(root):
    rows = []
    for revision in LEGACY_REVISIONS:
        raw = subprocess.check_output(
            ["git", "show", f"{revision}:{NATIVE_PROFILES}"], cwd=root
        )
        [row] = [
            row
            for row in json.loads(raw)["records"]
            if row["production_binding"]["external_id"] == "50255"
        ]
        rows.append(row)
    return base.encode(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary-root", type=Path)
    parser.add_argument("--crystal-root", type=Path)
    parser.add_argument("--legacy-robes", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.legacy_robes:
        evidence = ROOT / LEGACY_ROBES
        raw = legacy_robes(ROOT)
        if args.check:
            if evidence.read_bytes() != raw:
                raise ValueError("M25_LEGACY_ROBE_GENERATED_DRIFT")
        else:
            evidence.write_bytes(raw)
        return
    if args.canary_root is None or args.crystal_root is None:
        parser.error(
            "--canary-root and --crystal-root are required for the current capture"
        )
    raw = base.encode(capture(args.canary_root, args.crystal_root))
    sha = hashlib.sha256(raw).hexdigest()
    rust = (
        "// Generated by tools/content-migration/galthen_boss_loot.py.\n"
        'pub(super) const CAPTURE_SHA256: &str =\n    "' + sha + '";\n'
    ).encode()
    for path, value in {
        PATH: raw,
        PIN_PATH: base.encode({"path": PATH, "sha256": sha}),
        RUST_PIN: rust,
    }.items():
        if args.check:
            if (ROOT / path).read_bytes() != value:
                raise ValueError("M25_LOOT_GENERATED_DRIFT:" + path)
        else:
            (ROOT / path).write_bytes(value)
    print("PASS M25 source-qualified carried yields", len(IDS))


if __name__ == "__main__":
    main()
