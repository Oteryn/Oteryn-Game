"""Regenerate the equipment cohort pin without deriving its expectation from actual data."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import subprocess
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


def questpc2_additions(source):
    if "questpc2_source_profile_cohort::IDS.len()" not in source:
        return 0
    from questpc2_equipment_cohort import declared_additions
    return declared_additions(TEST.with_name("questpc2_source_profile_cohort.rs"))


def declared_cohorts(source):
    table = source.split("const PROFILE_ADDITIONS:", 1)[1].split("];", 1)[0]
    names = re.findall(r'\(\s*"([^\"]+)"\s*,', table)
    counts = {name: int(n.replace('_', '')) for name, n in
              re.findall(r'\(\s*"([^\"]+)"\s*,\s*([\d_]+)\s*,?\s*\)', table)}
    if 'QUESTPC2-SOURCE-CLOSURE' in names and 'QUESTPC2-SOURCE-CLOSURE' not in counts:
        counts['QUESTPC2-SOURCE-CLOSURE'] = questpc2_additions(source)
    if len(names) != len(set(names)) or set(names) != counts.keys():
        raise ValueError('duplicate or unresolved declared profile cohort')
    return counts


def frozen_union():
    sys.path.insert(0, str(ROOT / "tools/contracts"))
    from refresh_quest_custody_equipment import composition
    own, _ = composition("534ea6b23de451ae1086e0be585d1f75887aa17a")
    main, _ = composition("c77d436d8be39f348924e66d97928ffd69e61a8c")
    from questpc8_wrath_profile_expectation import declared_keys
    own = own | declared_keys(ROOT)
    return own | main, len(own - main)


def source_speed_pin(source, qengine):
    marker = '("qengine-reward-item-profiles.json",'
    if marker not in source:
        return source
    pick = 'oteryn:item.tibia.i3456'
    unknown = {row['production_definition']['production_key']
               for row in qengine['records']
               if row['attributes'].get('speed_bonus') is None}
    if unknown != {pick} or any(row['attributes'].get('speed_bonus') not in (None, 0)
                               for row in qengine['records']):
        raise ValueError('QENGINE speed proof must retain only the existing unknown-speed pick')
    source = re.sub(r'\("qengine-reward-item-profiles.json", [\d_]+\)',
                    f'("qengine-reward-item-profiles.json", {len(keys(qengine))})', source)
    start = source.index('        for source in records {', source.index(marker))
    end = source.index('\n    for id in [2949,', start)
    source = source[:start] + '''        for source in records {
            let key = source["production_definition"]["production_key"]
                .as_str()
                .ok_or("source Item key")?;
            let actual = profiles
                .records
                .iter()
                .find(|row| row.production_definition.production_key == key)
                .ok_or_else(|| format!("missing source-qualified SEASON profile {key}"))?;
            if path == "qengine-reward-item-profiles.json" && key == "oteryn:item.tibia.i3456" {
                assert!(source["attributes"]["speed_bonus"].is_null(), "source speed {path}:{key}");
                assert_eq!(actual.attributes.speed_bonus, None, "{path}:{key}");
            } else {
                assert_eq!(source["attributes"]["speed_bonus"], serde_json::json!(0), "source speed {path}:{key}");
                assert_eq!(actual.attributes.speed_bonus, Some(0), "{path}:{key}");
            }
            assert!(actual.admission.materializable, "{path}:{key}");
        }
    }''' + source[end:]
    return source


def regenerate(source, base, updates, qengine, npc, starting=None, custody=None, frozen=None, overlap_keys=frozenset(), baseline_cohorts=None, baseline_qengine_count=None):
    source = source_speed_pin(source, qengine)
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
        if custody is not None and baseline_cohorts is not None:
            if baseline_qengine_count is None:
                raise ValueError('missing immutable QENGINE baseline cohort')
            cohorts = declared_cohorts(source)
            # The NPC row is independently deducted below. Every other row
            # absent from the immutable published baseline is a later cohort,
            # including the package's corpse/profile successors.
            later_names = sorted(cohorts.keys() - set(baseline_cohorts) - {'QNPCFX'})
            later_count = sum(cohorts[name] for name in later_names)
            later = ('    let later_main_profiles = PROFILE_ADDITIONS\n'
                     '        .iter()\n        .filter(|(name, _)| matches!(*name, '
                     + ' | '.join(json.dumps(name) for name in later_names)
                     + '))\n        .map(|(_, count)| count)\n        .sum::<usize>();\n'
                     + f'    assert_eq!(later_main_profiles, {later_count});\n'
                     + f'    const QENGINE2_BASE_PROFILES: usize = {baseline_qengine_count};\n'
                     + f'    assert_eq!(additions.len(), QENGINE2_BASE_PROFILES + {len(keys(qengine)) - baseline_qengine_count});\n')
        elif frozen is not None:
            cohort_names = {"WORLDCHG", "SRV9-MARRIAGE", "QNPCFX", "QUESTPC4-STAG-ITEMS", "QUESTPC8-MERGED", "QREADY1", "QREADY2"}
            table = source.split("const PROFILE_ADDITIONS:", 1)[1].split("];", 1)[0]
            later_count = sum(int(n.replace("_", "")) for name, n in re.findall(r'\("([^\"]+)",\s*([\d_]+)\)', table) if name in cohort_names)
            later = re.sub(r"assert_eq!\(later_main_profiles, [\d_]+\);", f"assert_eq!(later_main_profiles, {later_count});", later)
            later = re.sub(r"// Frozen published union partitions.*", "// Published owner cohorts plus independently declared recipe tools.", later)
        source = source[:start] + later + source[end:]

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
            + questpc2_additions(source)
        )
    overlap = '- 1 // the pick already belongs to the base provider' in source
    if overlap:
        pick = 'oteryn:item.tibia.i3456'
        if pick not in keys(base) or pick not in keys(qengine):
            raise ValueError('declared pick overlap lacks both source owners')
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
    if overlap:
        declared -= 1
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
    census_path = ROOT / "tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json"
    custody = json.loads(census_path.read_text()) if "questpc8-equipment-custody-census.json" in source else None
    baseline_cohorts = None
    baseline_qengine_count = None
    if custody is not None:
        published = subprocess.check_output(['git', 'show', custody['base'] + ':' + TEST.relative_to(ROOT).as_posix()], cwd=ROOT, text=True)
        baseline_cohorts = set(declared_cohorts(published))
        baseline_qengine = subprocess.check_output(['git', 'show', custody['base'] + ':tools/content-schema/native-gameplay/qengine-reward-item-profiles.json'], cwd=ROOT)
        baseline_qengine_count = len(keys(json.loads(baseline_qengine)))
    generated, count = regenerate(
        source, *(json.loads((ROOT / p).read_text()) for p in paths),
        custody=custody,
        baseline_cohorts=baseline_cohorts,
        baseline_qengine_count=baseline_qengine_count,
        overlap_keys=(set() if "questpc8-equipment-custody-census.json" in source else
            {f"oteryn:item.tibia.i{i}" for i in
             __import__('qengine_reward_item_profiles').PREVIOUSLY_COUNTED_IDS}),
        frozen=None if "questpc8-equipment-custody-census.json" in source else frozen_union(),
    )
    generated = subprocess.check_output(
        ['rustfmt', '--edition', '2024', '--config', 'skip_children=true'],
        input=generated, text=True, cwd=ROOT,
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
