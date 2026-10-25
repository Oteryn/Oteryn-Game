#!/usr/bin/env python3
"""RANGED-CONTENT-1: deterministic, offline Item authoring from pinned Canary facts.

The imported items.xml is byte-identical at the NIGHT-W1 revision and its older
D384 import pin. Weapon registration (mana and damage) is nested in this XML at
that revision, rather than in wands.lua. No identities or admission are created.
Source defaults are explicit here, never runtime fallbacks for incomplete JSON.
"""
from __future__ import annotations

import argparse
from collections import Counter
import copy
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "imports/canary/items-xml/items.xml"
REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
SOURCE_SHA256 = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
PROFILES = "content/items/definitions/spell-native-profiles.json"
MANIFEST = "content/spells.manifest.json"
REPORT = "tools/content-migration/samples/ranged-content-source.json"
ELEMENTS = {"PHYSICAL", "DEATH", "EARTH", "ENERGY", "FIRE", "ICE"}


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def integer(values, field, default=None, low=0, high=65535):
    raw = values.get(field, default)
    if raw is None or not str(raw).lstrip("-").isdigit():
        raise ValueError(f"RANGED_REQUIRED_INTEGER:{field}")
    value = int(raw)
    if not low <= value <= high:
        raise ValueError(f"RANGED_INTEGER_RANGE:{field}")
    return value


def attributes(nodes, allowed=None):
    result = {}
    for node in nodes:
        name = node.attrib["key"].lower()
        if allowed is not None and name not in allowed and not name.startswith("element"):
            continue
        if name in result:
            raise ValueError(f"RANGED_DUPLICATE_ATTRIBUTE:{name}")
        result[name] = node.get("value")
    return result


def build(raw):
    profiles, held, counts = {}, [], Counter()
    seen = set()
    for item in ET.fromstring(raw).findall("item"):
        nodes = item.findall("attribute")
        if not any(node.get("key", "").lower() == "weapontype" and node.get("value") in {"distance", "ammunition", "wand"} for node in nodes):
            continue
        top = attributes(nodes, {"weapontype", "primarytype", "ammotype", "attack", "range", "hitchance", "maxhitchance"})
        weapon = top.get("weapontype")
        if weapon not in {"distance", "ammunition", "wand"}:
            continue
        nested = attributes(item.findall("attribute[@key='script']/attribute"))
        ammo = top.get("ammotype")
        kind = ("ROD" if top.get("primarytype") == "rods" or "rod" in item.get("name", "").split()
                else "WAND") if weapon == "wand" else (
                    "AMMUNITION" if weapon == "ammunition" else "DISTANCE" if ammo else "THROWING")
        if "id" in item.attrib:
            ids = [integer(item.attrib, "id", low=1)]
        else:
            ids = range(integer(item.attrib, "fromid", low=1), integer(item.attrib, "toid", low=1) + 1)
            if not ids:
                raise ValueError("RANGED_INVERTED_ID_RANGE")
        profile = {"kind": kind}
        if kind in {"WAND", "ROD"}:
            element = nested.get("wandtype", "").upper()
            if element not in ELEMENTS - {"PHYSICAL"}:
                raise ValueError("RANGED_REQUIRED_MAGIC_ELEMENT")
            profile.update(
                damage_min=integer(nested, "fromdamage", low=1),
                damage_max=integer(nested, "todamage", low=1),
                mana_per_shot=integer(nested, "mana", low=1), element=element)
            if profile["damage_min"] > profile["damage_max"]:
                raise ValueError("RANGED_INVERTED_DAMAGE")
        else:
            profile.update(attack=integer(top, "attack", 0),
                           hit_chance=integer(top, "hitchance", 0, low=-100 if kind == "DISTANCE" else 0, high=100),
                           max_hit_chance=integer(top, "maxhitchance", 90 if ammo else 75, low=1, high=100))
            if kind in {"DISTANCE", "AMMUNITION"}:
                if ammo not in {"arrow", "bolt"}:
                    raise ValueError("RANGED_REQUIRED_AMMUNITION")
                profile["ammunition"] = ammo.upper()
            if kind != "DISTANCE":
                elemental = [(name[7:].upper(), integer(top, name, low=1)) for name in top if name.startswith("element")]
                if len(elemental) > 1 or any(element not in ELEMENTS - {"PHYSICAL"} for element, _ in elemental):
                    raise ValueError("RANGED_ELEMENTAL_ATTACK")
                profile["element"], profile["elemental_attack"] = elemental[0] if elemental else ("PHYSICAL", 0)
        if kind != "AMMUNITION":
            profile["range_cells"] = integer(top, "range", 1, low=1, high=255)
        for item_id in ids:
            if item_id in seen:
                raise ValueError("RANGED_DUPLICATE_ITEM")
            seen.add(item_id)
            counts[kind] += 1
            # RANGED-0 §4 accepts only 1..7 cells. Retain the source fact, never clamp it.
            if profile.get("range_cells", 1) > 7:
                held.append({"item_id": item_id, "reason": "range_exceeds_RANGED_0_bound", "profile": profile})
            else:
                profiles[item_id] = copy.deepcopy(profile)
    return profiles, held, dict(sorted(counts.items()))


def source_profiles(root):
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("RANGED_SOURCE_DIGEST")
    return build(raw)


def tree_profile(row, profiles):
    key = row["definition"]["identity"]["key"]
    profile = profiles.get(int(key.rsplit(".i", 1)[1]))
    if profile is not None:
        row.setdefault("authoring", {})["ranged"] = copy.deepcopy(profile)


def apply_native(root, generated):
    """Annotate existing admitted policies; never expand native admission."""
    profiles, _, _ = source_profiles(root)
    document = json.loads(generated[PROFILES])
    for record in document["records"]:
        item_id = int(record["production_binding"]["external_id"])
        if item_id in profiles:
            record["authoring"]["ranged"] = copy.deepcopy(profiles[item_id])
    # Preserve the established native profile file's pretty-print convention.
    generated[PROFILES] = (json.dumps(document, ensure_ascii=False, indent=2) + "\n").encode()
    manifest = json.loads(generated[MANIFEST])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[PROFILES]).hexdigest()
    generated[MANIFEST] = encode(manifest)


def outputs(root=ROOT):
    profiles, held, counts = source_profiles(root)
    result, defined = {}, set()
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        document = json.loads(path.read_bytes())
        for row in document["records"]:
            defined.add(int(row["definition"]["identity"]["key"].rsplit(".i", 1)[1]))
            tree_profile(row, profiles)
        result[path.relative_to(root).as_posix()] = encode(document)
    result[REPORT] = encode({
        "schema": "OTERYN_RANGED_CONTENT_SOURCE/v1",
        "source": {"repository": "opentibiabr/canary", "revision": REVISION,
                   "path": "data/items/items.xml", "sha256": SOURCE_SHA256},
        "counts": counts, "held": held,
        "undefined_items": sorted(set(profiles) - defined),
        "profiles": {str(key): value for key, value in sorted(profiles.items())},
    })
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    drift = [path for path, data in generated.items() if not (ROOT / path).exists() or (ROOT / path).read_bytes() != data]
    if args.check:
        if drift:
            raise SystemExit("RANGED_GENERATED_DRIFT:" + ",".join(drift))
    else:
        for path in drift:
            (ROOT / path).write_bytes(generated[path])


if __name__ == "__main__":
    main()
