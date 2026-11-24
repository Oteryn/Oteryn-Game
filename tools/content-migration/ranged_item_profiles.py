#!/usr/bin/env python3
"""RANGED-CONTENT-1: deterministic Item authoring from pinned Canary and Crystal facts.

The imported items.xml is byte-identical at the NIGHT-W1 revision and its older
D384 import pin. Weapon registration (mana and damage) is nested in this XML at
that revision, rather than in wands.lua. No identities or admission are created.
Source defaults are explicit here, never runtime fallbacks for incomplete JSON.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "imports/canary/items-xml/items.xml"
REVISION = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
SOURCE_SHA256 = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
CRYSTAL_SOURCE = "content/items/ranged-crystal-source.xml"
CRYSTAL_REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CRYSTAL_XML_SHA256 = "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
CRYSTAL_EXCERPT_SHA256 = "14759602d92ad18193a7038de58d1023866c6da7d4f08db26f2a2ef9abaef2ed"
PROFILES = "content/items/definitions/spell-native-profiles.json"
RANGED_PROFILES = "content/items/ranged-profiles.json"
MANIFEST = "content/spells.manifest.json"
REPORT = "tools/content-migration/samples/ranged-content-source.json"
ELEMENTS = {"PHYSICAL", "DEATH", "EARTH", "ENERGY", "FIRE", "ICE"}


def encode(value):
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode()


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
        if (
            allowed is not None
            and name not in allowed
            and not name.startswith("element")
        ):
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
        script = item.findall("attribute[@key='script']/attribute")
        # XML weapon registration declares some thrown weapons solely as missiles.
        missile = any(
            node.get("key", "").lower() == "weapontype"
            and node.get("value") == "missile"
            for node in script
        )
        if not any(
            node.get("key", "").lower() == "weapontype"
            and node.get("value") in {"distance", "ammunition", "wand"}
            for node in nodes
        ) and not missile:
            continue
        top = attributes(
            nodes,
            {
                "weapontype",
                "primarytype",
                "ammotype",
                "attack",
                "range",
                "hitchance",
                "maxhitchance",
            },
        )
        weapon = top.get("weapontype", "distance" if missile else None)
        if weapon not in {"distance", "ammunition", "wand"}:
            continue
        nested = attributes(script)
        ammo = top.get("ammotype")
        kind = (
            (
                "ROD"
                if top.get("primarytype") == "rods"
                or "rod" in item.get("name", "").split()
                else "WAND"
            )
            if weapon == "wand"
            else (
                "AMMUNITION"
                if weapon == "ammunition"
                else "DISTANCE"
                if ammo
                else "THROWING"
            )
        )
        if "id" in item.attrib:
            ids = [integer(item.attrib, "id", low=1)]
        else:
            ids = range(
                integer(item.attrib, "fromid", low=1),
                integer(item.attrib, "toid", low=1) + 1,
            )
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
                mana_per_shot=integer(nested, "mana", low=1),
                element=element,
            )
            if profile["damage_min"] > profile["damage_max"]:
                raise ValueError("RANGED_INVERTED_DAMAGE")
        else:
            profile.update(
                attack=integer(top, "attack", 0),
                hit_chance=integer(
                    top, "hitchance", 0, low=-100 if kind == "DISTANCE" else 0, high=100
                ),
                max_hit_chance=integer(
                    top, "maxhitchance", 90 if ammo else 75, low=1, high=100
                ),
            )
            if kind in {"DISTANCE", "AMMUNITION"}:
                if ammo not in {"arrow", "bolt"}:
                    raise ValueError("RANGED_REQUIRED_AMMUNITION")
                profile["ammunition"] = ammo.upper()
            if kind != "DISTANCE":
                elemental = [
                    (name[7:].upper(), integer(top, name, low=1))
                    for name in top
                    if name.startswith("element")
                ]
                if len(elemental) > 1 or any(
                    element not in ELEMENTS - {"PHYSICAL"} for element, _ in elemental
                ):
                    raise ValueError("RANGED_ELEMENTAL_ATTACK")
                profile["element"], profile["elemental_attack"] = (
                    elemental[0] if elemental else ("PHYSICAL", 0)
                )
        if kind != "AMMUNITION":
            profile["range_cells"] = integer(top, "range", 1, low=1, high=255)
        for item_id in ids:
            if item_id in seen:
                raise ValueError("RANGED_DUPLICATE_ITEM")
            seen.add(item_id)
            counts[kind] += 1
            # RANGED-0 §4 accepts only 1..7 cells. Retain the source fact, never clamp it.
            if profile.get("range_cells", 1) > 7:
                held.append(
                    {
                        "item_id": item_id,
                        "reason": "range_exceeds_RANGED_0_bound",
                        "profile": profile,
                    }
                )
            else:
                profiles[item_id] = copy.deepcopy(profile)
    return profiles, held, dict(sorted(counts.items()))


def source_profiles(root):
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("RANGED_SOURCE_DIGEST")
    profiles, held, counts = build(raw)
    crystal_raw = (root / CRYSTAL_SOURCE).read_bytes()
    if hashlib.sha256(crystal_raw).hexdigest() != CRYSTAL_EXCERPT_SHA256:
        raise ValueError("RANGED_CRYSTAL_SOURCE_DIGEST")
    crystal, crystal_held, crystal_counts = build(crystal_raw)
    if crystal_held or profiles.keys() & crystal.keys():
        raise ValueError("RANGED_CRYSTAL_SOURCE_OVERLAP_OR_RANGE")
    retained_held = [row for row in held if row["item_id"] not in crystal]
    total_counts = Counter(counts) + Counter(crystal_counts)
    for row in held:
        if row["item_id"] in crystal:
            total_counts[row["profile"]["kind"]] -= 1
    return profiles | crystal, retained_held, dict(sorted(total_counts.items()))


def item_rows(root):
    return [
        row
        for path in sorted((root / "content/items/definitions").glob("items-*.json"))
        for row in json.loads(path.read_bytes())["records"]
    ]


def profile_record(row, profile, captured=None):
    """Keep sealed authoring owners intact; the separate profile only owns new facts."""
    profile = copy.deepcopy(profile)
    if captured is not None and profile["kind"] in {"DISTANCE", "THROWING"}:
        profile.update(captured)
    authoring = row.get("authoring", {})
    if "weapon_attack_modifier_points" in authoring:
        del profile["attack"]
    if "weapon_absolute_hit_chance_percent" in authoring:
        del profile["hit_chance"]
    return {"item": row["definition"]["identity"], "profile": profile}


def resolved_profile(profile, authoring):
    """Compile the native value from the existing owners, never a duplicated default."""
    profile = copy.deepcopy(profile)
    for field, owner in (
        ("attack", "weapon_attack_modifier_points"),
        ("hit_chance", "weapon_absolute_hit_chance_percent"),
    ):
        if owner not in authoring:
            continue
        if field in profile:
            raise ValueError(f"RANGED_DUPLICATE_OWNER:{field}")
        value = authoring[owner]
        if field == "hit_chance":
            numerator, denominator = value["numerator"], value["denominator"]
            if denominator <= 0 or numerator % denominator:
                raise ValueError("RANGED_HIT_CHANCE_NOT_INTEGER")
            value = numerator // denominator
        profile[field] = value
    return profile


def apply_authoring(root, *documents):
    """Annotate existing admitted policies; never expand native admission."""
    raw = (root / RANGED_PROFILES).read_bytes()
    if raw != outputs(root)[RANGED_PROFILES]:
        raise ValueError("RANGED_PROFILE_SOURCE_DRIFT")
    rows = {row["definition"]["identity"]["key"]: row for row in item_rows(root)}
    profiles = json.loads(raw)
    if profiles["schema"] != "OTERYN_RANGED_ITEM_PROFILES/v1":
        raise ValueError("RANGED_PROFILE_SCHEMA")
    records = profiles["records"]
    profiles = {record["item"]["key"]: record for record in records}
    if len(profiles) != len(records):
        raise ValueError("RANGED_DUPLICATE_ITEM")
    for key, record in profiles.items():
        if key not in rows or record["item"] != rows[key]["definition"]["identity"]:
            raise ValueError("RANGED_PROFILE_TARGET")
    for document in documents:
        for record in document["records"]:
            key = record["production_binding"]["target"]["key"]
            if key in profiles:
                owner = rows[key].get("authoring", {})
                record["authoring"]["ranged"] = resolved_profile(
                    profiles[key]["profile"], owner
                )


def apply_native(root, generated, starter_path):
    from newhaven_starter_profiles import encode_fragment

    document = json.loads(generated[PROFILES])
    starter = json.loads(generated[starter_path])
    apply_authoring(root, document, starter)
    # Match the active provider's compact serialization within its existing 8 MiB bound.
    generated[PROFILES] = encode(document)
    generated[starter_path] = encode_fragment(starter["records"])
    manifest = json.loads(generated[MANIFEST])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[PROFILES]
    ).hexdigest()
    manifest["starting_loadout_item_profiles"]["sha256"] = hashlib.sha256(
        generated[starter_path]
    ).hexdigest()
    generated[MANIFEST] = encode(manifest)


def outputs(root=ROOT):
    from import_cyclopedia_global import attack_fields, launcher_fields, load_capture

    profiles, held, counts = source_profiles(root)
    capture = load_capture(root)
    result, defined, records = {}, set(), []
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        document = json.loads(path.read_bytes())
        for row in document["records"]:
            item_id = int(row["definition"]["identity"]["key"].rsplit(".i", 1)[1])
            defined.add(item_id)
            # Remove the former overlay, restoring the sealed authoring siblings exactly.
            authoring = row.get("authoring", {})
            authoring.pop("ranged", None)
            if "authoring" in row and not authoring:
                del row["authoring"]
            if item_id in profiles:
                attack = capture.get(item_id, {}).get("basic", {}).get("Attack")
                captured = (
                    launcher_fields(attack)
                    if attack and not attack_fields(attack)
                    else None
                )
                records.append(profile_record(row, profiles[item_id], captured))
        result[path.relative_to(root).as_posix()] = encode(document)
    result[RANGED_PROFILES] = encode(
        {
            "schema": "OTERYN_RANGED_ITEM_PROFILES/v1",
            "records": sorted(records, key=lambda record: record["item"]["key"]),
        }
    )
    result[REPORT] = encode(
        {
            "schema": "OTERYN_RANGED_CONTENT_SOURCE/v1",
            "source": {
                "repository": "opentibiabr/canary",
                "revision": REVISION,
                "path": "data/items/items.xml",
                "sha256": SOURCE_SHA256,
            },
            "crystal_supplement": {
                "repository": "zimbadev/crystalserver",
                "revision": CRYSTAL_REVISION,
                "path": "data/items/items.xml",
                "sha256": CRYSTAL_XML_SHA256,
                "excerpt": CRYSTAL_SOURCE,
                "excerpt_sha256": CRYSTAL_EXCERPT_SHA256,
            },
            "counts": counts,
            "held": held,
            "undefined_items": sorted(set(profiles) - defined),
            "profiles": {str(key): value for key, value in sorted(profiles.items())},
        }
    )
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    drift = [
        path
        for path, data in generated.items()
        if not (ROOT / path).exists() or (ROOT / path).read_bytes() != data
    ]
    if args.check:
        if drift:
            raise SystemExit("RANGED_GENERATED_DRIFT:" + ",".join(drift))
    else:
        for path in drift:
            (ROOT / path).write_bytes(generated[path])


if __name__ == "__main__":
    main()
