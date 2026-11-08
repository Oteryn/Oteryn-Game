"""Regenerate the equipment cohort pin without deriving its expectation from actual data."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
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


<<<<<<< HEAD
def frozen_union():
    sys.path.insert(0, str(ROOT / "tools/contracts"))
    from refresh_quest_custody_equipment import composition
    own, _ = composition("534ea6b23de451ae1086e0be585d1f75887aa17a")
    main, _ = composition("c77d436d8be39f348924e66d97928ffd69e61a8c")
    from questpc8_wrath_profile_expectation import declared_keys
    own = own | declared_keys(ROOT)
    return own | main, len(own - main)


def regenerate(source, base, updates, qengine, npc, starting=None, custody=None, frozen=None, overlap_keys=frozenset()):
    if frozen is not None:
        expected, own_only = frozen
        row = f'    ("QUESTPC8-MERGED", {own_only}),'
        if '("QUESTPC8-MERGED",' in source:
            source = re.sub(r'    \("QUESTPC8-MERGED", [\d_]+\),', row, source)
        else:
            source = source.replace('    ("QUICKLOOT-1",', row + '\n    ("QUICKLOOT-1",', 1)

    if "    let later_main_profiles =" in source:
        # QREADY registrations overlap the old lane-only keys, so their ownership
        # shifts out of that cohort while the immutable baseline stays unchanged.
        start = source.index("    let later_main_profiles =")
        end = source.index("    let baseline =", start)
        later = source[start:end]
        for name in ("QREADY1", "QREADY2"):
            if f'"{name}"' not in later:
                later = later.replace('"QUESTPC8-MERGED"', f'"QUESTPC8-MERGED" | "{name}"')
        later = re.sub(r"Frozen own-only cohort\d+ plus\d+ post-baseline main registrations\.",
                       "Frozen published union partitions the same205 post-baseline registrations.", later)
        if frozen is not None:
            cohort_names = {"WORLDCHG", "SRV9-MARRIAGE", "QNPCFX", "QUESTPC4-STAG-ITEMS", "QUESTPC8-MERGED", "QREADY1", "QREADY2"}
            table = source.split("const PROFILE_ADDITIONS:", 1)[1].split("];", 1)[0]
            later_count = sum(int(n.replace("_", "")) for name, n in re.findall(r'\("([^\"]+)",\s*([\d_]+)\)', table) if name in cohort_names)
            later = re.sub(r"assert_eq!\(later_main_profiles, [\d_]+\);", f"assert_eq!(later_main_profiles, {later_count});", later)
            later = re.sub(r"// Frozen published union partitions.*", "// Published owner cohorts plus independently declared recipe tools.", later)
        source = source[:start] + later + source[end:]

=======
def questpc2_additions(source):
    if "questpc2_source_profile_cohort::IDS.len()" not in source:
        return 0
    from questpc2_equipment_cohort import declared_additions
    return declared_additions(TEST.with_name("questpc2_source_profile_cohort.rs"))


def regenerate(source, base, updates, qengine, npc, starting=None):
>>>>>>> refs/qp/sideC
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
<<<<<<< HEAD
    if frozen is not None and custody is None:
        declared = len(expected)
    elif custody is not None:
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
=======
    extra = re.search(r"cohorts add (\d+) \+ (\d+) keys", source)
    if extra is None:
        raise ValueError("missing independently declared floor/callback cohorts")
    declared = (
        original
        + sum(counts.values())
        + len(qengine["records"])
        + sum(map(int, extra.groups()))
        + questpc2_additions(source)
    )
    # The source-qualified pick is shared with the original native provider.
    overlap = '- 1 // the pick already belongs to the base provider' in source
    if overlap:
        pick = 'oteryn:item.tibia.i3456'
        if pick not in keys(base) or pick not in keys(qengine):
            raise ValueError('declared pick overlap lacks both source owners')
        declared -= 1
>>>>>>> refs/qp/sideC
    actual = keys(base) | keys(updates) | keys(starting or {"records": []})
    if frozen is not None and actual != expected:
        raise ValueError("generated profile keys differ from frozen published union")
    engine_keys, npc_keys = keys(qengine), keys(npc)
    if engine_keys & npc_keys or not (engine_keys | npc_keys) <= actual:
        raise ValueError("source proof keys are overlapping or absent")
    if not set(overlap_keys) <= (engine_keys & actual):
        raise ValueError("unproven source cohort overlap")
    if custody is None and frozen is None:
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
    start = source.find("    assert_eq!(\n        profiles.records.len(),")
    if start < 0:
        return source, declared
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
    source = TEST.read_text()
    generated, count = regenerate(
        source, *(json.loads((ROOT / p).read_text()) for p in paths),
        custody=(json.loads((ROOT / "tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json").read_text())
                 if "questpc8-equipment-custody-census.json" in source else None),
        overlap_keys=(set() if "questpc8-equipment-custody-census.json" in source else
            {f"oteryn:item.tibia.i{i}" for i in
             __import__('qengine_reward_item_profiles').PREVIOUSLY_COUNTED_IDS}),
        frozen=None if "questpc8-equipment-custody-census.json" in source else frozen_union(),
    )
    if args.check and generated != source:
        raise SystemExit("equipment cohort pin is stale")
    if not args.check and generated != source:
        with tempfile.NamedTemporaryFile("w", dir=TEST.parent, delete=False) as output:
            output.write(generated)
            temporary = output.name
        os.replace(temporary, TEST)
    print(
        f"PASS equipment profiles={count}; independent cohort sum and generated union agree"
    )


if __name__ == "__main__":
    main()
