"""I19 Item closure: Crystal 201189f enchanting.lua:13-41,126-133.

The existing admitted Item profiles own names, physical and weapon facts. This
overlay adds only the enchanting outputs' materialization, stack shape and the
1000-charge budget declared by the action. The engine stack ceiling is 100.
"""

import argparse
import hashlib
import json
from collections import Counter
from copy import deepcopy
from pathlib import Path

from import_cyclopedia_global import apply_stats, load_capture
from native_profile_composition import encode_native_profile

PATH = "tools/content-schema/native-gameplay/enchanting-item-profiles.json"
WEAPON_EXPECTATIONS = (
    "tools/content-schema/native-gameplay/enchanting-global-weapon-facts.json"
)

GEMS = {675, 676, 677, 678}
AMMO = {761, 762, 763, 774}
# Crystal items.xml:22117+: 8077 and its outputs are nonstacking shields; Lua preserves subtype.
SHIELDS = {8078, 8079, 8080, 8081}
WEAPONS = (
    set(range(660, 675))
    | set(range(679, 694))
    | set(range(779, 794))
    | {794, 795, 796, 797, 798, 801, 802, 803, 804, 805, 806, 807, 808, 809, 810}
)


def known(value):
    return {"state": "KNOWN", "value": value}


def apply_admissions(definitions):
    found = set()
    for definition in definitions:
        item = int(definition["identity"]["key"].rsplit("i", 1)[1])
        if item not in GEMS | AMMO | SHIELDS | WEAPONS:
            continue
        found.add(item)
        semantics = definition["semantics"]
        physical = semantics.get("physical", {})
        if (
            physical.get("state") != "KNOWN"
            or physical["value"]["weight"].get("state") != "KNOWN"
        ):
            raise ValueError(f"enchanting output lacks existing physical facts: {item}")
        if (
            item in WEAPONS | AMMO | SHIELDS
            and semantics.get("weapon", {}).get("state") != "KNOWN"
        ):
            raise ValueError(f"enchanting output lacks existing weapon facts: {item}")
        stackable = item in GEMS | AMMO
        definition["materializable"] = True
        definition["stack_class"] = "StackCapable" if stackable else "NonStackable"
        semantics["stack"] = known(
            {
                "stackable": known(stackable),
                "stack_max": known(100) if stackable else {"state": "NOT_APPLICABLE"},
            }
        )
        if item in WEAPONS:
            semantics["charges"] = known({"count": known(1000)})
    if found != GEMS | AMMO | SHIELDS | WEAPONS:
        raise ValueError("incomplete enchanting Item closure")


def load_overlay(root):
    overlay = json.loads((root / PATH).read_bytes())
    if (
        overlay["schema"] != "OTERYN_NATIVE_ENCHANTING_ITEM_PROFILES/v1"
        or overlay["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
    ):
        raise ValueError("ENCHANTING_PROFILE_SOURCE")
    return overlay


def weapon_expectations(root):
    """Publish only qualified Global replacements for the enchanting weapon closure."""
    capture = load_capture(root)
    expected = {}
    for record in load_overlay(root)["records"]:
        binding = record["production_binding"]
        identifier = int(binding["external_id"])
        if identifier not in WEAPONS | AMMO | SHIELDS or identifier not in capture:
            continue
        key = binding["target"]["key"]
        if key != record["production_definition"]["production_key"]:
            raise ValueError("ENCHANTING_WEAPON_IDENTITY:" + key)
        donor = record["semantics"]["weapon"]
        projected = {"semantics": {"weapon": deepcopy(donor)}}
        basic = capture[identifier]["basic"]
        weapon_facts = {
            field: basic[field] for field in ("Attack", "Defence") if field in basic
        }
        apply_stats(projected, weapon_facts, Counter())
        if projected["semantics"]["weapon"] != donor:
            expected[key] = projected["semantics"]["weapon"]
    return expected


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = load_overlay(root)
    document = json.loads(generated[profile_path])
    existing = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    for record in overlay["records"]:
        key = record["production_definition"]["production_key"]
        if key in existing:
            if (
                existing[key]["production_binding"] != record["production_binding"]
                or existing[key]["admission"] != record["admission"]
            ):
                raise ValueError(f"ENCHANTING_PROFILE_CONFLICT:{key}")
        else:
            document["records"].append(record)
    document["records"].sort(key=order)
    raw = encode_native_profile(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    output = root / WEAPON_EXPECTATIONS
    expected = weapon_expectations(root)
    raw = (json.dumps(expected, sort_keys=True, indent=2) + "\n").encode()
    if args.check:
        if output.read_bytes() != raw:
            raise SystemExit("Enchanting Global weapon expectations drift")
    else:
        output.write_bytes(raw)
    print(f"PASS enchanting Global weapon expectations ({len(expected)} items)")


if __name__ == "__main__":
    main()
