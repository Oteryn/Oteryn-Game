#!/usr/bin/env python3
"""Capture the ten pinned ambient XML spawners of Soul War and Ragiaz."""
import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

SOURCE = "data-global/world/world-monster.xml"
SHA256 = "f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e"
OUTPUT = Path(__file__).with_name("srx3-spite-spawns.json")


def build(path):
    raw = Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError("pinned Crystal monster XML changed")
    text = raw.decode()
    expected = {
        "Bone Capsule": ("oteryn:creature.bone_capsule", [(33485, 32333, 14)]),
        "Poor Soul": ("oteryn:creature.poor_soul", [(33851, 31870, 7)]),
        "Necromantic Focus": ("oteryn:creature.necromantic_focus", [(33704, 31634, 14), (33716, 31634, 14)]),
        "Lesser Splinter of Madness": ("oteryn:creature.lesser_splinter_of_madness", [(33712, 31635, 14)]),
        "Weeping Soul": ("oteryn:creature.weeping_soul", [(33738, 31632, 14), (33748, 31634, 14)]),
        "Spiteful Spitter": ("oteryn:creature.spiteful_spitter", [(33744, 31636, 14)]),
        "Symbol of Hatred": ("oteryn:creature.symbol_of_hatred", [(33738, 31599, 14), (33748, 31599, 14)]),
    }
    source_lines = {}
    for line, value in enumerate(text.splitlines(), 1):
        for name in expected:
            if '<monster name="' + name + '"' in value:
                source_lines.setdefault(name, []).append(line)
    result = []
    observed = {}
    for source in ET.fromstring(raw):
        for monster in source:
            name = monster.get("name")
            if name not in expected:
                continue
            key, positions = expected[name]
            at = (int(source.get("centerx")) + int(monster.get("x")),
                  int(source.get("centery")) + int(monster.get("y")),
                  int(monster.get("z")))
            if at not in positions:
                raise ValueError("unreviewed source ambient spawn: " + name)
            index = observed.get(name, 0)
            observed[name] = index + 1
            result.append(dict(position=list(at), radius=int(source.get("radius")),
                               respawn_seconds=int(monster.get("spawntime")),
                               creature_key=key, source_path=SOURCE, source_sha256=SHA256,
                               line=source_lines[name][index]))
    if (len(result) != 10
            or any(observed.get(name) != len(positions) for name, (_, positions) in expected.items())
            or any(row["radius"] != 1 or row["respawn_seconds"] != (60 if row["creature_key"] == "oteryn:creature.bone_capsule" else 10) for row in result)):
        raise ValueError("Soul War XML source set changed")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    args = parser.parse_args()
    OUTPUT.write_text(json.dumps(build(args.source), indent=2) + "\n")
