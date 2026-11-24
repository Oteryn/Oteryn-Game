"""Import the missing Crystal taming targets through the native Creature converter."""

import argparse
import hashlib
import json
from pathlib import Path
import sys

from creature_admission_stage import Mapper, Stage

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "tools/content-schema/native-gameplay/taming-creature-profiles.json"
CREATURES = "content/creatures/definitions/spell-native-profiles.json"
PRESENTATIONS = "content/presentations/definitions/spell-native-profiles.json"
MANIFEST = "content/spells.manifest.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
TARGETS = (
    ("quests/rotten_blood_quest/foxmouse", "foxmouse", 73,
     "623d5a807ff90c4f9663f2c142e9e57d9775f8e82f4c556dfcc6281be5d53c5e"),
    ("winter_update_2025/quests/tamed_frazzlemaw", "battlefrazzle", 8,
     "070c5547ee7d1ddf953f2af3d9318694101a09c46fd35b783eae71c77e88d29c"),
)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class TamingMapper(Mapper):
    def key(self, family, key):
        # The action's Battlefrazzle key names the mount. Its success and failure
        # text identify the registered Tamed Frazzlemaw creature.
        if (family, key) == ("Creature", "canary:creature/tamed_frazzlemaw"):
            return "oteryn:creature.battlefrazzle"
        return super().key(family, key)


def capture(canary, crystal):
    sys.path.insert(0, str(ROOT / "tools/content-schema/monster-authoring"))
    import canary_batch
    import spell_scripts

    objects = canary_batch.load_appearance_objects(crystal / "data/items/appearances.dat")
    items = canary_batch.load_items_xml(crystal / "data/items/items.xml")
    names, index = canary_batch.name_index(objects, items)
    converter = canary_batch.Converter(canary, objects, items, names, index)
    converter.monster_root, converter.monster_dir = crystal, "data-global/monster"
    converter.source = {"repository": "zimbadev/crystalserver", "revision": REVISION}
    converter.spell_scripts = spell_scripts.SpellScripts(canary, extra_roots=(crystal,))
    converted = []
    for relative, name, action_line, expected_sha in TARGETS:
        path = f"{converter.monster_dir}/{relative}.lua"
        if digest((crystal / path).read_bytes()) != expected_sha:
            raise ValueError(f"Crystal creature source drift: {path}")
        _, monster, dependencies, _, readiness, _ = converter.convert(relative)
        if any(row["status"].startswith("unresolved") for row in readiness["entries"]):
            raise ValueError(f"Unresolved Crystal creature semantics: {path}")
        stage = Stage(TamingMapper({2886: "oteryn:item.tibia.i2886"}))
        key = stage.stage_monster(monster, path)
        stage.stage_dependencies(dependencies, key)
        if key != f"oteryn:creature.{name}":
            raise ValueError(f"Unexpected taming creature identity: {key}")
        owner = stage.records["Creature", key]
        creature = stage.profiles["Creature", key]
        creature["data"]["profile"]["details"].setdefault("reference_level", None)
        behavior = stage.profiles["Behavior", owner["behavior"]["key"]]
        closure = sorted(k for k in stage.records.keys() & stage.profiles.keys() if k[0] in
                         {"Creature", "Behavior", "Ability", "Effect", "Formula"})
        converted.append({
            "source": {"path": path, "revision": REVISION, "sha256": expected_sha,
                       "taming_action": f"data-global/scripts/actions/mounts/mounts.lua:{action_line}"},
            "creature": {"profile": creature, "presentation": owner["presentation"], "behavior": behavior,
                         "attack_definitions": {"records": [stage.records[k] for k in closure],
                                                "profiles": [stage.profiles[k] for k in closure]}},
            "presentation": stage.profiles["Presentation", owner["presentation"]["key"]],
        })
    return {"records": converted}


def apply_native(root, generated, encode):
    source = json.loads((root / SOURCE).read_bytes())
    profiles = ((CREATURES, "creature_profiles", "creature"),
                (PRESENTATIONS, "presentation_profiles", "presentation"))
    manifest = json.loads(generated[MANIFEST])
    for path, pin, field in profiles:
        document = json.loads(generated[path])
        additions = [row[field] for row in source["records"]]
        identity = lambda row: (row["profile"] if field == "creature" else row)["target"]["key"]
        keys = {identity(row) for row in additions}
        if len(keys) != len(TARGETS):
            raise ValueError(f"Duplicate taming source identity in {path}")
        existing = {identity(row): row for row in document["records"]}
        for row in additions:
            key = identity(row)
            if key not in existing:
                document["records"].append(row)
            elif field != "presentation" or row != existing[key]:
                raise ValueError(f"Conflicting taming definition in {path}: {key}")
        generated[path] = encode(document)
        manifest[pin]["sha256"] = digest(generated[path])
    generated[MANIFEST] = encode(manifest)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary", type=Path, required=True)
    parser.add_argument("--crystal", type=Path, required=True)
    args = parser.parse_args()
    (ROOT / SOURCE).write_text(json.dumps(capture(args.canary, args.crystal), indent=2) + "\n")
