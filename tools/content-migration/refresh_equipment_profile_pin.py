"""Regenerate the equipment cohort pin without deriving its expectation from actual data."""

from __future__ import annotations

import argparse
import copy
import importlib
import first_dragon_item_profiles
import json
import os
import re
import tempfile
from pathlib import Path

import qready1_clock_equipment_pin
import qready1_cradle_equipment_pin
import qready1_craft_equipment_pin
import questpc4_treasure_reward_equipment_pin

ROOT = Path(__file__).resolve().parents[2]
TEST = (
    ROOT
    / "crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs"
)


# Intersections with the independently committed pre-SRX3 source provider,
# origin/main 98d6ae8ce5e12d60916532c72353126f534fa20a. These are identities,
# not counts sampled from the generated final provider.
SRX3_SOURCES = (
    ("SRX3-PRIMAL-POD", "primal_pod_profiles", ()),
    ("SRX3-SOUL-WAR-ITEMS", "soul_war_source_profiles", ()),
    ("SRX3-BOSS-MOVEMENT", "boss_movement_item_profiles", ()),
    ("SRX3-BOSS-CORPSES", "boss_corpse_profiles",
     (4026, 4098, 4240, 4241, 4242, 4246, 4247, 4248, 5973, 5995)),
    ("SRX3-BOSS-YIELDS", "boss_reward_yield_profiles",
     (239, 3028, 3081, 3097, 3384, 3583, 3723, 7428, 7642, 7643, 23373, 23374, 23375, 23535)),
    ("M25-GALTHEN-CORPSES", "m25_corpse_items", ()),
    ("M25-GALTHEN-THRONE", "galthen_boss_furniture", ()),
    ("M25-GALTHEN-YIELDS", "galthen_boss_loot", ()),
)


def registered_source_document(root, document):
    """Replay the later guarded House stage, retaining only this owner's keys."""
    proven = keys(document)
    generated = {"profiles": json.dumps(document).encode(),
                 "manifest": b'{"item_profiles":{}}'}
    owner = importlib.import_module("house_decoration_actions")
    owner.admit(root, generated, "profiles", "manifest",
                lambda row: row["production_definition"]["production_key"],
                lambda value: json.dumps(value, sort_keys=True).encode())
    # House's own additions belong to its already declared cohort. Only its
    # authoritative enrichment of these independent source rows composes here.
    result = {"records": [row for row in json.loads(generated["profiles"])["records"]
                          if row["production_definition"]["production_key"] in proven]}
    if keys(result) != proven:
        raise ValueError("source registration changed the qualified identity set")
    return result


def source_cohorts(root):
    """Re-use each closed source authority before counting its independent keys."""
    taxonomy = {
        row["target"]["key"]: row["source_taxonomy"]
        for row in json.loads((root / "content/items/taxonomy/items.json").read_bytes())["records"]
    }
    result = []
    for name, module_name, previous_ids in SRX3_SOURCES:
        generated = {"profiles": b'{"records":[]}', "manifest": b'{"item_profiles":{}}'}
        owner = importlib.import_module(module_name)
        owner.admit(root, generated, "profiles", "manifest",
                    lambda row: row["production_definition"]["production_key"],
                    lambda value: json.dumps(value, sort_keys=True).encode())
        document = json.loads(generated["profiles"])
        for row in document["records"]:
            key = row["production_definition"]["production_key"]
            # Registration's Hireling stage attaches canonical taxonomy to
            # earlier profiles. Yield admission runs after that stage. Never
            # copy a later generated profile's authoring to manufacture parity.
            if module_name != "boss_reward_yield_profiles" and key in taxonomy and "taxonomy" not in row["authoring"]:
                row["authoring"]["taxonomy"] = copy.deepcopy(taxonomy[key])
        previous = {f"oteryn:item.tibia.i{item}" for item in previous_ids}
        result.append((name, registered_source_document(root, document), previous))
    return result


def source_coowners(root):
    """Main's pinned Primal packet already contributes these five identities."""
    owner = importlib.import_module("primal_item_profiles")
    document = owner.load_capture(root)
    expected = {f"oteryn:item.tibia.i{item}" for item in owner.IDS}
    if keys(document) != expected:
        raise ValueError("Primal coowner source identity set differs")
    return [(owner.COHORT_NAME, registered_source_document(root, document), expected)]


def merged_source_duplicates(root):
    """Closed source intersections introduced by the mechanic integration."""
    import elemental_spheres_profiles as elemental
    import floor_assembly_profiles as floor
    import qengine_reward_item_profiles as engine
    import quest_callback_profiles as callbacks
    floor_keys = {f'oteryn:item.tibia.i{i}' for i in floor.IDS}
    elemental_keys = {f'oteryn:item.tibia.i{i}' for i in elemental.IDS}
    expected_floor = {f'oteryn:item.tibia.i{i}' for i in
                      (7738,7739,7740,7741,7742,7743,7744,7745,7746,7748,8106,8139,8140,8141)}
    if floor_keys & elemental_keys != expected_floor:
        raise ValueError('merged Elemental/Floor source intersection differs')
    # These named cohorts also occur in QENGINE's exact IDS witness. M05's jar
    # is counted here for its separate ENGNPC-M05 row, in addition to the
    # independently checked QENGINE/QNPCFX pair below.
    engine_shared = {f'oteryn:item.tibia.i{i}' for i in
                     (5467,21389,21466,22008,27654,12526,12533,12534,12535,21506,21507)}
    if not engine_shared <= {f'oteryn:item.tibia.i{i}' for i in engine.IDS}:
        raise ValueError('merged QENGINE source intersection differs')
    # The frozen pre-pass3 provider a2e337c72e already admits these four
    # M35 claim items and the access key. Certificate6498 is new and is
    # deliberately NOT subtracted. Retain both source witnesses.
    claim_shared = {f'oteryn:item.tibia.i{i}' for i in (645,3014,3210,3602,2973)}
    if not claim_shared <= callbacks.requested_keys(root):
        raise ValueError('merged claim/callback source intersection differs')
    if expected_floor & engine_shared or expected_floor & claim_shared or engine_shared & claim_shared:
        raise ValueError('ambiguous merged source intersection')
    return expected_floor | engine_shared | claim_shared


def coowner_equivalent(source, qualified):
    """Two guarded captures may name their own revision of the SAME candidate."""
    normalized = copy.deepcopy(source)
    original = normalized.get("authoring", {}).get("item")
    accepted = qualified.get("authoring", {}).get("item")
    if original != accepted:
        if (not isinstance(original, dict) or not isinstance(accepted, dict)
                or set(original) != {"family", "key", "revision"}
                or set(accepted) != set(original)
                or original["family"] != accepted["family"]
                or original["key"] != accepted["key"]):
            return False
        # Neither revision is inferred from actual output: both are independently
        # source-guarded input rows, and the SRX3 row is also compared in full below.
        aliases = {f"oteryn:item.tibia.i{item}": f"candidate:item/{item}"
                   for item in (39527, 39528, 39529, 39530)}
        key = source["production_definition"]["production_key"]
        if (key not in aliases or original["family"] != "Item"
                or original["key"] != aliases[key]
                or (original["revision"], accepted["revision"]) != (
                    "spell-p2-r21", "srx3-boss-corpses-crystal-201189f")):
            return False
        normalized["authoring"]["item"] = copy.deepcopy(accepted)
    return normalized == qualified


def keys(document):
    rows = [
        row["production_definition"]["production_key"] for row in document["records"]
    ]
    if len(rows) != len(set(rows)):
        raise ValueError("duplicate production key within a profile owner")
    return set(rows)


def regenerate(source, base, updates, qengine, npc, starting=None, srx3=(), coowners=(), engine_npc_shared=(), duplicate_keys=()):
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
    engine_keys, npc_keys = keys(qengine), keys(npc)
    shared = engine_keys & npc_keys
    # M05's source jar is independently captured by both Item producers.
    # Count it once only when the complete source-owned records agree.
    engine_rows = {r['production_definition']['production_key']: r for r in qengine['records']}
    npc_rows = {r['production_definition']['production_key']: r for r in npc['records']}
    if shared != set(engine_npc_shared) or any(engine_rows[k] != npc_rows[k] for k in shared):
        raise ValueError("QENGINE/NPC shared source identity or facts differ")
    registered_engine = counts.get("QENGINE-REWARDS", 0)
    if registered_engine:
        subtraction = re.search(r"additions\.len\(\)\s*-\s*([0-9_]+)", source)
        if (subtraction is None
                or int(subtraction[1].replace("_", "")) not in (registered_engine, registered_engine + len(shared))
                or registered_engine > len(qengine["records"])):
            raise ValueError("registered QENGINE cohort differs from its explicit subtraction")
    declared = (
        original
        + sum(counts.values())
        + len(qengine["records"]) - registered_engine - len(shared)
        + sum(map(int, extra.groups()))
    )
    actual = keys(base) | keys(updates) | keys(starting or {"records": []})
    engine_keys, npc_keys = keys(qengine), keys(npc)
    if not (engine_keys | npc_keys) <= actual:
        raise ValueError("source proof keys are overlapping or absent")
    duplicate_keys = set(duplicate_keys)
    if not duplicate_keys <= actual:
        raise ValueError('merged source duplicate identity absent')
    declared -= len(duplicate_keys)
    actual_rows = {
        row["production_definition"]["production_key"]: row
        for document in (base, updates, starting or {"records": []})
        for row in document["records"]
    }
    coowner_rows = {}
    coowner_keys = set()
    for name, document, expected in coowners:
        proven = keys(document)
        if proven != expected or counts.get(name) != len(proven):
            raise ValueError("already-counted coowner declaration differs")
        if coowner_keys & proven or not proven <= actual:
            raise ValueError("already-counted coowner identity set differs")
        coowner_keys.update(proven)
        coowner_rows.update({row["production_definition"]["production_key"]: row
                             for row in document["records"]})
    additions = []
    added_keys = set()
    cohort_names = set()
    for name, document, previous in srx3:
        if name in cohort_names or name in counts:
            raise ValueError("duplicate SRX3 profile cohort")
        cohort_names.add(name)
        proven = keys(document)
        if not previous <= proven or not previous <= actual:
            raise ValueError("SRX3 source-base intersection differs")
        new = proven - previous
        if added_keys & new:
            raise ValueError("SRX3 source cohorts overlap")
        added_keys.update(new)
        for row in document["records"]:
            key = row["production_definition"]["production_key"]
            if key in new and actual_rows.get(key) != row:
                raise ValueError(f"SRX3 source profile differs: {name}/{key}")
        for key in new & coowner_keys:
            if not coowner_equivalent(coowner_rows[key], actual_rows[key]):
                raise ValueError(f"already-counted coowner profile differs: {key}")
        additions.append((name, len(new - coowner_keys)))
    if coowner_keys != added_keys & coowner_keys:
        raise ValueError("already-counted coowner source overlap differs")
    declared += sum(count for _, count in additions)
    if declared != len(actual):
        raise ValueError(
            f"declared cohorts={declared}, generated profile union={len(actual)}"
        )
    if duplicate_keys:
        block = '    // Closed intersections of the independently captured mechanic cohorts.\n'
        block += '    const MERGED_PROFILE_DUPLICATE_KEYS: &[&str] = &[\n'
        block += ''.join(f'        "{key}",\n' for key in sorted(duplicate_keys))
        block += '    ];\n'
        marker = r'    // Closed intersections of the independently captured mechanic cohorts\.\n    const MERGED_PROFILE_DUPLICATE_KEYS:.*?    ];\n'
        if 'const MERGED_PROFILE_DUPLICATE_KEYS:' in source:
            source, count = re.subn(marker, lambda _: block, source, flags=re.S)
            if count != 1:
                raise ValueError('ambiguous merged source duplicate witness')
        else:
            position = source.index('    assert_eq!(\n        profiles.records.len(),')
            source = source[:position] + block + source[position:]
        term = '            - MERGED_PROFILE_DUPLICATE_KEYS.len()\n'
        if term not in source:
            position = source.index('    assert_eq!(\n        profiles.records.len(),')
            position = source.index('\n    );', position)
            source = source[:position] + '\n' + term.rstrip('\n') + source[position:]
    if registered_engine:
        source = re.sub(r"additions\.len\(\)\s*-\s*[0-9_]+",
                        f"additions.len() - {registered_engine + len(shared)}", source)
    # Pin the independent admission packet, never the generated runtime union.
    # New shared quest inputs must retain the exact-key/raw-speed checks below.
    engine_pin = r"assert_eq!\(added_keys\.len\(\),\s*[0-9_]+\);"
    if len(re.findall(engine_pin, source)) > 1:
        raise ValueError("ambiguous shared engine exact-key pin")
    source = re.sub(
        engine_pin, f"assert_eq!(added_keys.len(), {len(engine_keys)});", source
    )
    source = re.sub(
        r'\("qengine-reward-item-profiles\.json",\s*[0-9_]+\)',
        f'("qengine-reward-item-profiles.json", {len(engine_keys)})',
        source,
    )
    source = re.sub(
        r"// QENGINE2 captures \d+ shared Items plus \w+ portable QUESTPC4 Items\.",
        "// QENGINE2 independently captures shared engine and portable quest Items.",
        source,
    )
    if additions:
        cohort_block = "    // Generated from closed source packets minus pinned pre-SRX3 identities.\n"
        if coowners:
            cohort_block += "    // W10-PRIMAL's independently pinned shared keys are already counted above.\n"
        cohort_block += "    const SRX3_SOURCE_COHORTS: &[(&str, usize)] = &[\n"
        cohort_block += "".join(f'        ("{name}", {count}),\n' for name, count in additions)
        cohort_block += "    ];\n"
        pattern = r"    // Generated from closed source packets minus pinned pre-SRX3 identities\.\n(?:    // W10-PRIMAL's independently pinned shared keys are already counted above\.\n)?    const SRX3_SOURCE_COHORTS:.*?    ];\n"
        if "const SRX3_SOURCE_COHORTS:" in source:
            source, replaced = re.subn(pattern, lambda _: cohort_block, source, flags=re.S)
            if replaced != 1:
                raise ValueError("ambiguous generated SRX3 cohort block")
        else:
            position = source.index("    assert_eq!(\n        profiles.records.len(),")
            source = source[:position] + cohort_block + source[position:]
        term = "            + SRX3_SOURCE_COHORTS.iter().map(|(_, n)| n).sum::<usize>()\n"
        if term not in source:
            engine_terms = list(re.finditer(
                r"^            \+ (?:additions\.len\(\)|\(additions\.len\(\) - [0-9_]+\))\n",
                source, re.M,
            ))
            if len(engine_terms) != 1:
                raise ValueError("ambiguous registered engine contribution")
            position = engine_terms[0].end()
            source = source[:position] + term + source[position:]
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
    source = first_dragon_item_profiles.cohort_receipt(original)
    source = questpc4_treasure_reward_equipment_pin.refresh(
        source,
        json.loads((ROOT / "tools/content-schema/native-gameplay/questpc4-treasure-reward-profiles.json").read_bytes()),
    )
    import qengine_equipment_cohort
    source = qengine_equipment_cohort.refresh(
        source, json.loads((ROOT / paths[2]).read_bytes())
    )
    import qengine_reward_equipment_pin
    source = qengine_reward_equipment_pin.refresh(source, json.loads((ROOT / paths[2]).read_bytes()))
    from m41_equipment_cohort import register
    # A stale preceding owner already determines --check; refuse without touching
    # its file or requiring unrelated owner inputs to be admitted first.
    if args.check and source != original:
        raise SystemExit("equipment cohort pin is stale")
    from qnpcfx_item_profiles import cohort_receipt, profile_identity_receipt
    packet = json.loads((ROOT / paths[3]).read_bytes())
    source = profile_identity_receipt(cohort_receipt(source, packet), packet)
    source = qready1_clock_equipment_pin.refresh(
        source,
        json.loads((ROOT / "tools/content-schema/native-gameplay/qready1-quest-item-profiles.json").read_bytes()),
    )
    source = qready1_craft_equipment_pin.refresh(
        source,
        json.loads((ROOT / "tools/content-schema/native-gameplay/qready1-craft-item-profiles.json").read_bytes()),
    )
    source = qready1_cradle_equipment_pin.refresh(
        source,
        json.loads((ROOT / "tools/content-schema/native-gameplay/qready1-cradle-item-profiles.json").read_bytes()),
    )
    import engnpc2_slug_corpse_profiles
    source = engnpc2_slug_corpse_profiles.refresh_cohort(
        source,
        json.loads((ROOT / engnpc2_slug_corpse_profiles.PATH).read_bytes()),
    )
    generated, count = regenerate(
        register(source), *(json.loads((ROOT / p).read_text()) for p in paths), srx3=source_cohorts(ROOT),
        coowners=source_coowners(ROOT), engine_npc_shared=("oteryn:item.tibia.i27654",),
        duplicate_keys=merged_source_duplicates(ROOT),
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
