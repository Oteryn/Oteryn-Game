#!/usr/bin/env python3
"""Admit appearance and movement from pinned Global-map Crystal NPC registrations."""

import argparse
import hashlib
import json
from pathlib import Path
import re

import import_crystal_npc_placements as source
from npc_admission_stage import behavior_profile, presentation_profile, ref
from questpc7_world_seal import refresh

FACTS = source.placements.ROOT / "imports/crystalserver/facts/crystal-global-npcs-201189f8.json"

PINS = {
    "brom": ("Brom", "f3fa08318f891a2f3431d347bede4eaafdda2299747c13e0ee32d234ed5122e3"),
    "brutus": ("Brutus", "a4dd011a5812877dc822968a95d414305ae5b7ffb4795cf70cf3b3894f76ae93"),
    "doctor_marrow": ("Doctor Marrow", "af1ff19264525f2b0726211c05f54ec4e244eee797d4fa919ed8dfd82a874135"),
    "ellipsis": ("...", "6a167beaed12476682af973eeb1d72a480753aa56d63d688244c49ebb75a130a"),
    "eustacios_spy_rat": ("Eustacios Spy Rat", "8ab58c88432b7c4b76947fa1c95233dcb43940c711cc904d25aba9864129d1f7"),
    "harlow": ("Harlow", "a7b39edd4830c79d07e02ca715260a48577184cea2463002506b62796f6109e3"),
    "roughington": ("Roughington", "7354649a4e88dd4344a6fc74a4a94539a003c50d6c9d30b9b0a934bfaec40f7f"),
    "shadowpunch": ("Shadowpunch", "ed0cf26d620cc9228b1afea41ddc6e4859fdb1b42da2da443885187a71ccfcb9"),
    "victor": ("Victor", "f3f20421193d046e6034d8e021ae6b8b26f199e244ffd9db4fd219d4b1303a0a"),
}


def pinned_facts(checkout, key, name, sha256):
    path = f"data-global/npc/{name if key == 'ellipsis' else key}.lua"
    raw = (checkout / path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != sha256:
        raise ValueError(f"{path}: unexpected donor script bytes")
    script = raw.decode()
    literal = json.dumps(name)
    if (f"Game.createNpcType({literal})" not in script
            and not (f"local internalNpcName = {literal}" in script
                     and "Game.createNpcType(internalNpcName)" in script)):
        raise ValueError(f"{path}: unexpected donor registration")
    outfit = re.search(r"npcConfig\.outfit\s*=\s*\{([^}]+)\}", script)[1]
    numbers = {k: int(v) for k, v in re.findall(r"(\w+)\s*=\s*(\d+)", outfit)}
    appearance = {key: numbers.get(field, 0) for key, field in {
        "look_type": "lookType", "head": "lookHead", "body": "lookBody",
        "legs": "lookLegs", "feet": "lookFeet", "addons": "lookAddons",
    }.items()}
    if appearance["look_type"] == 0:
        raise ValueError(f"{path}: donor outfit has no creature look type")
    motion = {
        "walk_interval_ms": int(re.search(r"npcConfig\.walkInterval\s*=\s*(\d+)", script)[1]),
        "walk_radius": int(re.search(r"npcConfig\.walkRadius\s*=\s*(\d+)", script)[1]),
        "floor_change": re.search(r"floorchange\s*=\s*(false|true)", script)[1] == "true",
    }
    return presentation_profile(appearance), behavior_profile(motion), {
        "evidence": "OtsHypothesisOnly",
        "repository": "zimbadev/crystalserver", "revision": source.REVISION,
        "path": path, "sha256": sha256,
    }


def capture(checkout):
    eligible = {row["name"] for row in source.read_sources(checkout)[source.GLOBAL] if row["engine_spawns"]}
    facts = {}
    for key, (name, digest) in PINS.items():
        if name not in eligible:
            raise ValueError(f"{name}: no Global map spawn")
        appearance, motion, evidence = pinned_facts(checkout, key, name, digest)
        facts[key] = {"name": name, "appearance": appearance, "motion": motion, "evidence": evidence}
    FACTS.write_text(json.dumps(facts, ensure_ascii=False, indent=2) + "\n")


def add_records():
    root = source.placements.ROOT
    world = root / "content/world/definitions"
    reference_path, declaration_path = (world / name for name in ("reference.json", "declarations.json"))
    reference, declarations = (json.loads(p.read_bytes()) for p in (reference_path, declaration_path))
    existing = {row["identity"]["key"] for row in declarations["records"] if row["kind"] == "NPC"}
    facts = json.loads(FACTS.read_bytes())
    if set(facts) != set(PINS):
        raise ValueError("incomplete pinned NPC facts")
    order = {
        ("records", "identity", "family"): list(dict.fromkeys(row["identity"]["family"] for row in reference["records"])),
        ("records", "identity", "kind"): list(dict.fromkeys(row["kind"] for row in declarations["records"])),
        ("authoring_profiles", "target", "family"): list(dict.fromkeys(row["target"]["family"] for row in declarations["authoring_profiles"])),
    }
    for key, (name, digest) in PINS.items():
        npc = f"oteryn:npc.{key}"
        fact = facts[key]
        evidence = fact["evidence"]
        if fact["name"] != name or evidence["sha256"] != digest or evidence["revision"] != source.REVISION:
            raise ValueError(f"{key}: retained NPC source pin mismatch")
        if npc in existing:
            continue
        appearance, motion = fact["appearance"], fact["motion"]
        presentation = ref("Presentation", f"oteryn:presentation.npc.{key}")
        behavior = ref("Behavior", f"oteryn:behavior.npc.{key}")
        for identity, profile, projection in ((presentation, appearance, "ClientSafe"),
                                               (behavior, motion, "ServerOnly")):
            reference["records"].append({"kind": "Generic", "identity": identity,
                                         "client_projection": projection})
            declarations["authoring_profiles"].append({
                "target": identity, "data": {"kind": identity["family"], "profile": profile},
            })
        declarations["records"].append({
            "kind": "NPC", "identity": {"key": npc, "revision": "definition-r1"},
            "presentation": presentation, "behavior": behavior, "dialogue": None, "services": [],
            "fields": [
                {"field_path": "oteryn:source.npc.display_name", "value": {"type": "Text", "value": name}},
                {"field_path": "oteryn:source.npc.floor_change", "value": {"type": "Boolean", "value": False}},
                {"field_path": "oteryn:source.npc.global_map_script", "value": {
                    "type": "Text", "value": source.placements.dump(evidence),
                }},
            ],
        })
    for (records, identity, group), groups in order.items():
        doc = reference if records == "records" and group == "family" else declarations
        index = {name: i for i, name in enumerate(groups)}
        doc[records].sort(key=lambda row: (
            index[row[identity][group] if group == "family" else row[group]],
            row[identity]["key"], row[identity]["revision"],
        ))
    reference_path.write_text(json.dumps(reference, ensure_ascii=False, separators=(",", ":")) + "\n")
    declaration_path.write_text(json.dumps(declarations, ensure_ascii=False, separators=(",", ":")) + "\n")
    refresh(root)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, help="retain facts from the pinned donor checkout")
    args = parser.parse_args()
    if args.capture:
        capture(args.capture)
    add_records()
