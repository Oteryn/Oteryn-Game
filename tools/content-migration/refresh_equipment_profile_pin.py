"""Regenerate the equipment cohort pin without deriving its expectation from actual data."""

from __future__ import annotations

import argparse
import json
import os
import re
import tempfile
from pathlib import Path

import questpc4_beregar_equipment_pin
import questpc4_recipe_equipment_pin
import questpc4_special_equipment_pin
import questpc4_treasure_equipment_pin
import questpc4_stag_equipment_pin
import questpc4_typed_equipment_pin
import questpc4_yalahar_equipment_pin

ROOT = Path(__file__).resolve().parents[2]
TEST = (
    ROOT
    / "crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs"
)


def keys(document):
    rows = [
        row["production_definition"]["production_key"] for row in document["records"]
    ]
    if len(rows) != len(set(rows)):
        raise ValueError("duplicate production key within a profile owner")
    return set(rows)


def regenerate(source, base, updates, qengine, npc, starting=None, custody=None, overlap_keys=frozenset()):
    original = int(
        re.search(r"const ORIGINAL_PROFILES: usize = ([\d_]+);", source)[1].replace(
            "_", ""
        )
    )
    table = source.split("const PROFILE_ADDITIONS:", 1)[1].split("];", 1)[0]
    cohorts = re.findall(r'\("([^"]+)",\s*([\d_]+)\)', table)
    names = [name for name, _ in cohorts]
    if len(names) != len(set(names)):
        raise ValueError("duplicate declared profile cohort")
    counts = {name: int(count.replace("_", "")) for name, count in cohorts}
    if counts.get("QNPCFX") != len(npc["records"]):
        raise ValueError("NPC source proof differs from the declared cohort")
    if custody is not None:
        if custody.get('schema') != 1 or custody.get('base') != '1ae39c7fb9bef09c3cd50d16410374f4134b1676' or custody.get('base_count') != 3663:
            raise ValueError('invalid pinned custody baseline')
        additions = custody['added_keys']
        if len(additions) != len(set(additions)):
            raise ValueError('duplicate custody addition')
        declared = custody['base_count'] + len(additions)
    else:
        extra = re.search(r"cohorts add (\d+) \+ (\d+) keys", source)
        if extra is None:
            raise ValueError("missing independently declared floor/callback cohorts")
        declared = original + sum(counts.values()) + len(qengine["records"]) + sum(map(int, extra.groups()))
    actual = keys(base) | keys(updates) | keys(starting or {"records": []})
    engine_keys, npc_keys = keys(qengine), keys(npc)
    if engine_keys & npc_keys or not (engine_keys | npc_keys) <= actual:
        raise ValueError("source proof keys are overlapping or absent")
    if not set(overlap_keys) <= (engine_keys & actual):
        raise ValueError("unproven source cohort overlap")
    if custody is None:
        declared -= len(overlap_keys)
    if declared != len(actual):
        raise ValueError(
            f"declared cohorts={declared}, generated profile union={len(actual)}"
        )
    if custody is not None:
        if not set(custody['added_keys']) <= actual:
            raise ValueError('custody source identity absent')
        return source, declared
    # The NPC owner is already counted in PROFILE_ADDITIONS. Keep every strict
    # raw-attribute/identity assertion and remove only its duplicate sum term.
    start = source.index("    assert_eq!(\n        profiles.records.len(),")
    end = source.index("\n    );", start) + len("\n    );")
    block = source[start:end]
    duplicate = "            + npc_additions.len()\n"
    if block.count(duplicate) > 1:
        raise ValueError("ambiguous duplicate NPC contribution")
    return source[:start] + block.replace(duplicate, "") + source[end:], declared


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = (
        "content/items/definitions/spell-native-profiles.json",
        "content/items/definitions/srx1-item-updates.json",
        "tools/content-schema/native-gameplay/qengine-reward-item-profiles.json",
        "tools/content-schema/native-gameplay/qnpcfx-item-profiles.json",
        "content/items/definitions/starting-loadout-profiles.json",
    )
    original = TEST.read_text()
    source = questpc4_stag_equipment_pin.refresh(
        original,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-stag-item-profiles.json"
            ).read_text()
        ),
    )
    source = questpc4_yalahar_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-yalahar-item-profiles.json"
            ).read_text()
        ),
    )
    source = questpc4_beregar_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-beregar-item-profiles.json"
            ).read_text()
        ),
    )
    source = questpc4_recipe_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-recipe-item-profiles.json"
            ).read_text()
        ),
    )
    source = questpc4_typed_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-typed-item-profiles.json"
            ).read_text()
        ),
    )
    source = questpc4_special_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-special-item-profiles.json"
            ).read_text()
        ),
    )
    source = questpc4_treasure_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-treasure-input-profiles.json"
            ).read_text()
        ),
    )
    generated, count = regenerate(
        source, *(json.loads((ROOT / p).read_text()) for p in paths),
        custody=(json.loads((ROOT / "tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json").read_text())
                 if "questpc8-equipment-custody-census.json" in source else None),
        overlap_keys={f"oteryn:item.tibia.i{i}" for i in
            __import__('qengine_reward_item_profiles').PREVIOUSLY_COUNTED_IDS},
    )
    if args.check and generated != original:
        raise SystemExit("equipment cohort pin is stale")
    if not args.check and generated != original:
        with tempfile.NamedTemporaryFile("w", dir=TEST.parent, delete=False) as output:
            output.write(generated)
            temporary = output.name
        os.replace(temporary, TEST)
    print(
        f"PASS equipment profiles={count}; independent cohort sum and generated union agree"
    )


if __name__ == "__main__":
    main()
