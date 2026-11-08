"""Regenerate the equipment cohort pin without deriving its expectation from actual data."""

from __future__ import annotations

import argparse
import questpc4_treasure_reward_equipment_pin
import qready1_clock_equipment_pin
import qready1_craft_equipment_pin
import json
import os
import re
import tempfile
from pathlib import Path

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


def regenerate(source, base, updates, qengine, npc, starting=None):
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
    extra = re.search(r"cohorts add (\d+) \+ (\d+) keys", source)
    if extra is None:
        raise ValueError("missing independently declared floor/callback cohorts")
    registered_engine = counts.get("QENGINE-REWARDS", 0)
    if registered_engine:
        subtraction = re.search(r"additions\.len\(\)\s*-\s*([0-9_]+)", source)
        if (subtraction is None
                or int(subtraction[1].replace("_", "")) != registered_engine
                or registered_engine > len(qengine["records"])):
            raise ValueError("registered QENGINE cohort differs from its explicit subtraction")
    declared = (
        original
        + sum(counts.values())
        + len(qengine["records"]) - registered_engine
        + sum(map(int, extra.groups()))
    )
    actual = keys(base) | keys(updates) | keys(starting or {"records": []})
    engine_keys, npc_keys = keys(qengine), keys(npc)
    if engine_keys & npc_keys or not (engine_keys | npc_keys) <= actual:
        raise ValueError("source proof keys are overlapping or absent")
    if declared != len(actual):
        raise ValueError(
            f"declared cohorts={declared}, generated profile union={len(actual)}"
        )
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
    source = questpc4_treasure_reward_equipment_pin.refresh(
        original,
        json.loads((ROOT / "tools/content-schema/native-gameplay/questpc4-treasure-reward-profiles.json").read_bytes()),
    )
    source = qready1_clock_equipment_pin.refresh(
        source,
        json.loads((ROOT / "tools/content-schema/native-gameplay/qready1-quest-item-profiles.json").read_bytes()),
    )
    source = qready1_craft_equipment_pin.refresh(
        source,
        json.loads((ROOT / "tools/content-schema/native-gameplay/qready1-craft-item-profiles.json").read_bytes()),
    )
    generated, count = regenerate(
        source, *(json.loads((ROOT / p).read_text()) for p in paths)
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
