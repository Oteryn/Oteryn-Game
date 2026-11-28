"""Regenerate the equipment cohort pin without deriving its expectation from actual data."""

from __future__ import annotations

import argparse
import copy
import importlib
import imbuement_item_profiles
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import first_dragon_item_profiles
import m01c_key_door_equipment_pin
import qengine_reward_equipment_pin
from native_profile_composition import compact_item_profile
import qready1_clock_equipment_pin
import qready1_cradle_equipment_pin
import qready1_craft_equipment_pin
import questpc4_treasure_reward_equipment_pin
from srx1_profile_updates import compact_item_record

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
    (
        "SRX3-BOSS-CORPSES",
        "boss_corpse_profiles",
        (4026, 4098, 4240, 4241, 4242, 4246, 4247, 4248, 5973, 5995),
    ),
    (
        "SRX3-BOSS-YIELDS",
        "boss_reward_yield_profiles",
        (
            239,
            3028,
            3081,
            3097,
            3384,
            3583,
            3723,
            7428,
            7642,
            7643,
            23373,
            23374,
            23375,
            23535,
        ),
    ),
    ("M25-GALTHEN-CORPSES", "m25_corpse_items", ()),
    ("M25-GALTHEN-THRONE", "galthen_boss_furniture", ()),
    ("M25-GALTHEN-YIELDS", "galthen_boss_loot", ()),
)


def registered_source_document(root, document):
    """Replay later source policies, retaining only this owner's keys."""
    proven = keys(document)
    generated = {
        "profiles": json.dumps(document).encode(),
        "manifest": b'{"item_profiles":{}}',
    }
    owner = importlib.import_module("house_decoration_actions")
    owner.admit(
        root,
        generated,
        "profiles",
        "manifest",
        lambda row: row["production_definition"]["production_key"],
        lambda value: json.dumps(value, sort_keys=True).encode(),
    )
    # House's own additions belong to its already declared cohort. Only its
    # authoritative enrichment of these independent source rows composes here.
    result = {
        "records": [
            row
            for row in json.loads(generated["profiles"])["records"]
            if row["production_definition"]["production_key"] in proven
        ]
    }
    if keys(result) != proven:
        raise ValueError("source registration changed the qualified identity set")
    # For example, boss loot already admits 23545. The food action adds only
    # consumption, and the cohort still checks the complete composed row.
    import crystal_ordinary_items
    crystal_ordinary_items.apply_food_policies(
        {row['production_definition']['production_key']: row for row in result['records']},
        crystal_ordinary_items.source_records(root),
    )
    # Replay the late equipped facts on the independent source rows so the
    # census still compares complete profiles, including combat and Forge.
    import build_source_item_profiles as source
    import equipped_combat_forge_profiles as equipped
    from forge_item_profiles import project

    packet = equipped.load_facts(root)
    capture = equipped.load_capture(root)
    rows = equipped.records(result)
    for fact in packet['combat']:
        key = f'{equipped.PREFIX}{fact["id"]}'
        if key in rows:
            equipped.apply_combat(rows[key], fact, source, capture)
    forge = [fact for fact in packet['forge'] if f'{equipped.PREFIX}{fact["id"]}' in rows]
    if forge:
        admitted = equipped.forge_owners(root, forge)
        project({'records': [rows[key] for key in admitted]}, admitted)
    imbuement_item_profiles.apply_crystal_audit(root, result, require_all=False)
    return result


def _jar_source_taxonomy(root):
    jar = "oteryn:item.tibia.i27654"
    rows = [
        row
        for row in json.loads(
            (root / "content/items/taxonomy/items.json").read_bytes()
        )["records"]
        if row.get("target", {}).get("key") == jar
    ]
    identity = {"family": "Item", "key": jar, "revision": "definition-r1"}
    if len(rows) != 1 or rows[0].get("target") != identity:
        raise ValueError("NPC_JAR_TAXONOMY_SOURCE_IDENTITY")
    taxonomy = rows[0].get("source_taxonomy")
    if (
        type(taxonomy) is not dict
        or type(taxonomy.get("primary")) is not str
        or not taxonomy["primary"]
    ):
        raise ValueError("NPC_JAR_TAXONOMY_SOURCE_SHAPE")
    return copy.deepcopy(taxonomy)


def _jar_taxonomy_present(row, taxonomy):
    authoring = row.get("authoring")
    if isinstance(authoring, dict) and "taxonomy" not in authoring:
        return False
    if (
        not isinstance(authoring, dict)
        or type(authoring["taxonomy"]) is not dict
        or authoring["taxonomy"] != taxonomy
    ):
        raise ValueError("NPC_JAR_TAXONOMY_PREDECESSOR")
    return True


def registered_jar_taxonomy_document(root, document):
    """Replay Quick Loot's guarded canonical taxonomy contribution to the Jar."""
    jar = "oteryn:item.tibia.i27654"
    if keys(document) != {jar}:
        raise ValueError("NPC_JAR_TAXONOMY_IDENTITY_SET")
    taxonomy = _jar_source_taxonomy(root)
    _jar_taxonomy_present(document["records"][0], taxonomy)
    expected = copy.deepcopy(document["records"][0])
    expected["authoring"]["taxonomy"] = copy.deepcopy(taxonomy)
    generated = {
        "profiles": json.dumps(document).encode(),
        "manifest": b'{"item_profiles":{}}',
    }
    owner = importlib.import_module("quick_loot_item_profiles")
    owner.admit(
        root,
        generated,
        "profiles",
        "manifest",
        lambda row: row["production_definition"]["production_key"],
        lambda value: json.dumps(value, sort_keys=True).encode(),
    )
    projected = {
        "records": [
            row
            for row in json.loads(generated["profiles"])["records"]
            if row["production_definition"]["production_key"] == jar
        ]
    }
    if projected != {"records": [expected]} or not _jar_taxonomy_present(
        projected["records"][0], taxonomy
    ):
        raise ValueError("NPC_JAR_TAXONOMY_REGISTRATION_DELTA")
    return projected


def registered_engine_npc_documents(root, qengine, npc):
    """Compose only the shared Jar's qualified registration successors."""
    from qnpcfx_item_profiles import KEYS

    jar = "oteryn:item.tibia.i27654"
    npc_keys = keys(npc)
    if npc_keys != KEYS or keys(qengine) & npc_keys != {jar}:
        raise ValueError(
            "QENGINE/NPC shared source identity or facts differ: NPC_HOUSE_SHARED_IDENTITY_SET"
        )
    engine_row = next(
        row
        for row in qengine["records"]
        if row["production_definition"]["production_key"] == jar
    )
    index = next(
        i
        for i, row in enumerate(npc["records"])
        if row["production_definition"]["production_key"] == jar
    )
    npc_row = npc["records"][index]

    def registered_flags(row):
        attributes = row.get("attributes")
        fields = {"always_on_top", "always_on_top_order"}
        if isinstance(attributes, dict) and fields.isdisjoint(attributes):
            return False
        if (
            not isinstance(attributes, dict)
            or not fields.issubset(attributes)
            or attributes["always_on_top"] is not False
            or type(attributes["always_on_top_order"]) is not int
            or attributes["always_on_top_order"] != 0
        ):
            raise ValueError(
                "QENGINE/NPC shared House facts differ: NPC_HOUSE_TOP_ORDER_PREDECESSOR"
            )
        return True

    engine_registered = registered_flags(engine_row)
    registered_flags(npc_row)
    result = copy.deepcopy(npc)
    if any(not isinstance(row.get("authoring"), dict) for row in (engine_row, npc_row)):
        raise ValueError("NPC_JAR_TAXONOMY_PREDECESSOR")
    if any("taxonomy" in row.get("authoring", {}) for row in (engine_row, npc_row)):
        taxonomy = _jar_source_taxonomy(root)
        engine_taxonomy = _jar_taxonomy_present(engine_row, taxonomy)
        _jar_taxonomy_present(npc_row, taxonomy)
        if engine_taxonomy:
            expected = copy.deepcopy(npc_row)
            expected["authoring"]["taxonomy"] = copy.deepcopy(taxonomy)
            projected = registered_jar_taxonomy_document(root, {"records": [npc_row]})
            if projected != {"records": [expected]} or not _jar_taxonomy_present(
                projected["records"][0], taxonomy
            ):
                raise ValueError("NPC_JAR_TAXONOMY_REGISTRATION_DELTA")
            result["records"][index] = projected["records"][0]
    # The first registration may capture the raw/raw pair. Never introduce a
    # successor or erase a known field merely to make that pair compare equal.
    if not engine_registered:
        return copy.deepcopy(qengine), result
    expected = copy.deepcopy(result["records"][index])
    expected["attributes"].update(always_on_top=False, always_on_top_order=0)
    projected = registered_source_document(
        root, {"records": [result["records"][index]]}
    )
    if projected != {"records": [expected]} or not registered_flags(
        projected["records"][0]
    ):
        raise ValueError(
            "QENGINE/NPC shared House changed other facts: NPC_HOUSE_REGISTRATION_DELTA"
        )
    result["records"][index] = projected["records"][0]
    return copy.deepcopy(qengine), result


def boss_yield_prior_record(root):
    """Reproduce the earlier, requested Crimson Sword owner before yield custody."""
    import authored_static_item_constructors as authored
    import quest_callback_profiles as callbacks

    key = "oteryn:item.tibia.i7385"
    requested, authored_keys, _ = callbacks._requested_inputs(root)
    if key not in requested or key not in authored_keys:
        raise ValueError("SRX3_BOSS_YIELD_TRADER_NOT_REQUESTED")
    definitions = [
        row["definition"]
        for path in sorted((root / "content/items/definitions").glob("items-*.json"))
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] == key
    ]
    if len(definitions) != 1 or definitions[0]["identity"] != {
        "family": "Item",
        "key": key,
        "revision": "definition-r1",
    }:
        raise ValueError("SRX3_BOSS_YIELD_TRADER_CANONICAL_IDENTITY")
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )["bindings"]
    static = json.loads((root / authored.ADMISSIONS).read_bytes())
    bindings.extend(row["binding"] for row in static["records"])
    matches = [
        binding
        for binding in bindings
        if binding["target"] == definitions[0]["identity"]
    ]
    _, source_items = callbacks.canonical_source_inputs(root)
    return callbacks.canonical_native_profile(
        root, definitions[0], matches, source_items
    )


def source_cohorts(root):
    """Re-use each closed source authority before counting its independent keys."""
    taxonomy = {
        row["target"]["key"]: row["source_taxonomy"]
        for row in json.loads(
            (root / "content/items/taxonomy/items.json").read_bytes()
        )["records"]
    }
    result = []
    for name, module_name, previous_ids in SRX3_SOURCES:
        generated = {"profiles": b'{"records":[]}', "manifest": b'{"item_profiles":{}}'}
        owner = importlib.import_module(module_name)
        if module_name == "boss_reward_yield_profiles":
            generated["profiles"] = json.dumps(
                {"records": [boss_yield_prior_record(root)]}
            ).encode()
        owner.admit(
            root,
            generated,
            "profiles",
            "manifest",
            lambda row: row["production_definition"]["production_key"],
            lambda value: json.dumps(value, sort_keys=True).encode(),
        )
        document = json.loads(generated["profiles"])
        for row in document["records"]:
            key = row["production_definition"]["production_key"]
            # Registration's Quick Loot stage attaches canonical taxonomy to
            # earlier profiles. Yield admission runs after that stage. Never
            # copy a later generated profile's authoring to manufacture parity.
            if (
                module_name != "boss_reward_yield_profiles"
                and key in taxonomy
                and "taxonomy" not in row["authoring"]
            ):
                row["authoring"]["taxonomy"] = copy.deepcopy(taxonomy[key])
        previous = {f"oteryn:item.tibia.i{item}" for item in previous_ids}
        if module_name == "boss_reward_yield_profiles":
            from crystal_equipment_slice import complete_boss_yield_pairs

            complete_boss_yield_pairs(root, document)
        document = registered_source_document(root, document)
        result.append((name, document, previous))
    return result


def source_coowners(root):
    """Main's pinned Primal packet already contributes these five identities."""
    owner = importlib.import_module("primal_item_profiles")
    document = owner.load_capture(root)
    expected = {f"oteryn:item.tibia.i{item}" for item in owner.IDS}
    if keys(document) != expected:
        raise ValueError("Primal coowner source identity set differs")
    return [(owner.COHORT_NAME, registered_source_document(root, document), expected)]


def merged_source_duplicates(root, source):
    """Count only intersections not already removed by source-cohort owners."""
    import engnpc2_slug_corpse_profiles as slugs
    import qengine_reward_item_profiles as engine

    # equipment_source_overlaps already lowers the Floor, craft, M05 and M22
    # rows. M26's five shared boss identities are likewise removed there, so
    # no compensating subtraction of unrelated M35 claim identities belongs here.
    cohorts = (
        ("M07-FIRE-BUG", {5467}),
        ("M07-LION-TRIALS", {21389, 21466}),
        ("ENGNPC2-SLUG-CORPSES", set(slugs.IDS)),
    )
    engine_shared = set()
    for name, ids in cohorts:
        matches = re.findall(rf'\("{re.escape(name)}",\s*([\d_]+)\)', source)
        if len(matches) != 1 or int(matches[0].replace("_", "")) != len(ids):
            raise ValueError("merged source cohort declaration differs: " + name)
        identities = {f"oteryn:item.tibia.i{i}" for i in ids}
        if engine_shared & identities:
            raise ValueError("ambiguous merged source intersection")
        engine_shared.update(identities)
    engine_keys = keys(json.loads((root / engine.PROOF).read_bytes()))
    if (
        engine_keys != {f"oteryn:item.tibia.i{i}" for i in engine.IDS}
        or not engine_shared <= engine_keys
    ):
        raise ValueError("merged QENGINE source intersection differs")
    return engine_shared


def profile_equivalent(source, qualified):
    if source == qualified:
        return True
    fields = {"attributes", "semantics", "production_binding"}
    if qualified is None or not fields <= (source.keys() & qualified.keys()):
        return False
    return compact_item_record(copy.deepcopy(source)) == compact_item_record(copy.deepcopy(qualified))


def coowner_equivalent(source, qualified):
    """Two guarded captures may name their own revision of the SAME candidate."""
    normalized = copy.deepcopy(source)
    original = normalized.get("authoring", {}).get("item")
    accepted = qualified.get("authoring", {}).get("item")
    if original != accepted:
        if (
            not isinstance(original, dict)
            or not isinstance(accepted, dict)
            or set(original) != {"family", "key", "revision"}
            or set(accepted) != set(original)
            or original["family"] != accepted["family"]
            or original["key"] != accepted["key"]
        ):
            return False
        # Neither revision is inferred from actual output: both are independently
        # source-guarded input rows, and the SRX3 row is also compared in full below.
        aliases = {
            f"oteryn:item.tibia.i{item}": f"candidate:item/{item}"
            for item in (39527, 39528, 39529, 39530)
        }
        key = source["production_definition"]["production_key"]
        if (
            key not in aliases
            or original["family"] != "Item"
            or original["key"] != aliases[key]
            or (original["revision"], accepted["revision"])
            != ("spell-p2-r21", "srx3-boss-corpses-crystal-201189f")
        ):
            return False
        normalized["authoring"]["item"] = copy.deepcopy(accepted)
    return profile_equivalent(normalized, qualified)


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
    counts = {
        name: int(n.replace("_", ""))
        for name, n in re.findall(r'\(\s*"([^\"]+)"\s*,\s*([\d_]+)\s*,?\s*\)', table)
    }
    if "QUESTPC2-SOURCE-CLOSURE" in names and "QUESTPC2-SOURCE-CLOSURE" not in counts:
        counts["QUESTPC2-SOURCE-CLOSURE"] = questpc2_additions(source)
    if len(names) != len(set(names)) or set(names) != counts.keys():
        raise ValueError("duplicate or unresolved declared profile cohort")
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
    # Preserve the three prior unknowns. The pinned Item XML gives the
    # Shrunken Head Necklace speed 10 and the Prismatic Armor and Boots 15.
    expected = {
        f"oteryn:item.tibia.i{item}": speed
        for item, speed in {
            3456: None,
            16110: 15,
            16112: 15,
            19357: 10,
            31266: None,
            31268: None,
        }.items()
    }
    speeds = {
        row["production_definition"]["production_key"]: row["attributes"].get("speed_bonus")
        for row in qengine["records"]
    }
    observed = {key: speed for key, speed in speeds.items() if speed != 0}
    if observed != expected or any(
        speed is not None and type(speed) is not int for speed in speeds.values()
    ):
        raise ValueError(
            f"QENGINE exact source speed facts differ: expected {expected}, observed {observed}"
        )
    source = re.sub(
        r'\("qengine-reward-item-profiles.json", [\d_]+\)',
        f'("qengine-reward-item-profiles.json", {len(keys(qengine))})',
        source,
    )
    start = source.index("        for source in records {", source.index(marker))
    end = source.index("\n    for id in [2949,", start)
    source = (
        source[:start]
        + """        for source in records {
            let key = source["production_definition"]["production_key"]
                .as_str()
                .ok_or("source Item key")?;
            let actual = profiles
                .records
                .iter()
                .find(|row| row.production_definition.production_key == key)
                .ok_or_else(|| format!("missing source-qualified SEASON profile {key}"))?;
            let expected = if path == "qengine-reward-item-profiles.json" {
                expected_qengine_reward_speed(key)
            } else {
                Some(0)
            };
            assert_eq!(source["attributes"]["speed_bonus"], serde_json::json!(expected), "source speed {path}:{key}");
            assert_eq!(actual.attributes.speed_bonus, expected, "{path}:{key}");
            assert!(actual.admission.materializable, "{path}:{key}");
        }
    }"""
        + source[end:]
    )
    return source


def regenerate(
    source,
    base,
    updates,
    qengine,
    npc,
    starting=None,
    srx3=(),
    coowners=(),
    engine_npc_shared=(),
    duplicate_keys=(),
    *,
    custody=None,
    frozen=None,
    overlap_keys=frozenset(),
    baseline_cohorts=None,
    baseline_qengine_count=None,
):
    source = source_speed_pin(source, qengine)
    if frozen is not None:
        expected, own_only = frozen
        row = f'    ("QUESTPC8-MERGED", {own_only}),'
        if '("QUESTPC8-MERGED",' in source:
            source = re.sub(r'    \("QUESTPC8-MERGED", [\d_]+\),', row, source)
        else:
            source = source.replace(
                '    ("QUICKLOOT-1",', row + '\n    ("QUICKLOOT-1",', 1
            )

    if "    let later_main_profiles =" in source:
        # QREADY registrations overlap the old lane-only keys, so their ownership
        # shifts out of that cohort while the immutable baseline stays unchanged.
        start = source.index("    let later_main_profiles =")
        end = source.index("    let baseline =", start)
        later = source[start:end]
        for name in ("QREADY1", "QREADY2"):
            if f'"{name}"' not in later:
                later = later.replace(
                    '"QUESTPC8-MERGED"', f'"QUESTPC8-MERGED" | "{name}"'
                )
        later = re.sub(
            r"Frozen own-only cohort\d+ plus\d+ post-baseline main registrations\.",
            "Frozen published union partitions the same205 post-baseline registrations.",
            later,
        )
        if custody is not None and baseline_cohorts is not None:
            if baseline_qengine_count is None:
                raise ValueError("missing immutable QENGINE baseline cohort")
            cohorts = declared_cohorts(source)
            if isinstance(baseline_cohorts, dict):
                changed = {
                    name
                    for name in cohorts.keys() & baseline_cohorts.keys()
                    if name != "QNPCFX" and cohorts[name] != baseline_cohorts[name]
                }
                if changed:
                    raise ValueError(
                        "baseline cohort count changed without an identity proof: "
                        + ",".join(sorted(changed))
                    )
            # The NPC row is independently deducted below. Every other row
            # absent from the immutable published baseline is a later cohort,
            # including the package's corpse/profile successors.
            later_names = sorted(cohorts.keys() - set(baseline_cohorts) - {"QNPCFX"})
            later_count = sum(cohorts[name] for name in later_names)
            later = (
                "    let later_main_profiles = PROFILE_ADDITIONS\n"
                "        .iter()\n        .filter(|(name, _)| matches!(*name, "
                + " | ".join(json.dumps(name) for name in later_names)
                + "))\n        .map(|(_, count)| count)\n        .sum::<usize>();\n"
                + f"    assert_eq!(later_main_profiles, {later_count});\n"
                + f"    const QENGINE2_BASE_PROFILES: usize = {baseline_qengine_count};\n"
                + f"    assert_eq!(additions.len(), QENGINE2_BASE_PROFILES + {len(keys(qengine)) - baseline_qengine_count});\n"
            )
        elif frozen is not None:
            cohort_names = {
                "WORLDCHG",
                "SRV9-MARRIAGE",
                "QNPCFX",
                "QUESTPC4-STAG-ITEMS",
                "QUESTPC8-MERGED",
                "QREADY1",
                "QREADY2",
            }
            table = source.split("const PROFILE_ADDITIONS:", 1)[1].split("];", 1)[0]
            later_count = sum(
                int(n.replace("_", ""))
                for name, n in re.findall(r'\("([^\"]+)",\s*([\d_]+)\)', table)
                if name in cohort_names
            )
            later = re.sub(
                r"assert_eq!\(later_main_profiles, [\d_]+\);",
                f"assert_eq!(later_main_profiles, {later_count});",
                later,
            )
            later = re.sub(
                r"// Frozen published union partitions.*",
                "// Published owner cohorts plus independently declared recipe tools.",
                later,
            )
        source = source[:start] + later + source[end:]

    original = int(
        re.search(r"const ORIGINAL_PROFILES: usize = ([\d_]+);", source)[1].replace(
            "_", ""
        )
    )
    counts = declared_cohorts(source)
    if counts.get("QNPCFX") != len(npc["records"]):
        raise ValueError("NPC source proof differs from the declared cohort")
    engine_keys, npc_keys = keys(qengine), keys(npc)
    shared = engine_keys & npc_keys
    # M05's source jar is independently captured by both Item producers.
    # Count it once only when the complete source-owned records agree.
    engine_rows = {
        r["production_definition"]["production_key"]: r for r in qengine["records"]
    }
    npc_rows = {r["production_definition"]["production_key"]: r for r in npc["records"]}
    if shared != set(engine_npc_shared) or any(
        engine_rows[k] != npc_rows[k] for k in shared
    ):
        diagnostic = {
            "expected_shared": sorted(set(engine_npc_shared)),
            "actual_shared": sorted(shared),
            "mismatching_records": {
                key: {"qengine": engine_rows[key], "npc": npc_rows[key]}
                for key in sorted(shared)
                if engine_rows[key] != npc_rows[key]
            },
        }
        raise ValueError(
            "QENGINE/NPC shared source identity or facts differ: "
            + json.dumps(diagnostic, sort_keys=True)
        )
    legacy_sum = custody is None and frozen is None
    registered_engine = counts.get("QENGINE-REWARDS", 0)
    if frozen is not None and custody is None:
        declared = len(expected)
    elif custody is not None:
        if (
            custody.get("schema") != 1
            or custody.get("base") != "1ae39c7fb9bef09c3cd50d16410374f4134b1676"
            or custody.get("base_count") != 3663
        ):
            raise ValueError("invalid pinned custody baseline")
        custody_additions = custody["added_keys"]
        if len(custody_additions) != len(set(custody_additions)):
            raise ValueError("duplicate custody addition")
        declared = custody["base_count"] + len(custody_additions)
    else:
        extra = re.search(r"cohorts add (\d+) \+ (\d+) keys", source)
        if extra is None:
            raise ValueError("missing independently declared floor/callback cohorts")
        registered_engine = counts.get("QENGINE-REWARDS", 0)
        if registered_engine:
            subtraction = re.search(r"additions\.len\(\)\s*-\s*([0-9_]+)", source)
            if (
                subtraction is None
                or int(subtraction[1].replace("_", ""))
                not in (registered_engine, registered_engine + len(shared))
                or registered_engine > len(qengine["records"])
            ):
                raise ValueError(
                    "registered QENGINE cohort differs from its explicit subtraction"
                )
        declared = (
            original
            + sum(counts.values())
            + len(qengine["records"])
            - registered_engine
            - len(shared)
            + sum(map(int, extra.groups()))
        )
    overlap = "- 1 // the pick already belongs to the base provider" in source
    if overlap:
        pick = "oteryn:item.tibia.i3456"
        if pick not in keys(base) or pick not in keys(qengine):
            raise ValueError("declared pick overlap lacks both source owners")
    actual = keys(base) | keys(updates) | keys(starting or {"records": []})
    if frozen is not None and actual != expected:
        raise ValueError("generated profile keys differ from frozen published union")
    engine_keys, npc_keys = keys(qengine), keys(npc)
    if not (engine_keys | npc_keys) <= actual:
        raise ValueError("source proof keys are overlapping or absent")
    duplicate_keys = set(duplicate_keys)
    if not duplicate_keys <= actual:
        raise ValueError("merged source duplicate identity absent")
    if legacy_sum:
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
        coowner_rows.update(
            {
                row["production_definition"]["production_key"]: row
                for row in document["records"]
            }
        )
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
            if key in new and not profile_equivalent(row, actual_rows.get(key)):
                diagnostic = {
                    "cohort": name,
                    "key": key,
                    "expected_source_row": row,
                    "actual_row": actual_rows.get(key),
                }
                raise ValueError(
                    f"SRX3 source profile differs: {name}/{key}: "
                    + json.dumps(diagnostic, sort_keys=True)
                )
        for key in new & coowner_keys:
            if not coowner_equivalent(coowner_rows[key], actual_rows[key]):
                raise ValueError(f"already-counted coowner profile differs: {key}")
        additions.append((name, len(new - coowner_keys)))
    if coowner_keys != added_keys & coowner_keys:
        raise ValueError("already-counted coowner source overlap differs")
    if legacy_sum:
        declared += sum(count for _, count in additions)
    if not set(overlap_keys) <= (engine_keys & actual):
        raise ValueError("unproven source cohort overlap")
    if legacy_sum:
        declared -= len(overlap_keys)
    if overlap and legacy_sum:
        declared -= 1
    if declared != len(actual):
        raise ValueError(
            f"declared cohorts={declared}, generated profile union={len(actual)}"
        )
    if duplicate_keys and legacy_sum:
        block = "    // Closed intersections of the independently captured mechanic cohorts.\n"
        block += "    const MERGED_PROFILE_DUPLICATE_KEYS: &[&str] = &[\n"
        block += "".join(f'        "{key}",\n' for key in sorted(duplicate_keys))
        block += "    ];\n"
        marker = r"    // Closed intersections of the independently captured mechanic cohorts\.\n    const MERGED_PROFILE_DUPLICATE_KEYS:.*?    ];\n"
        if "const MERGED_PROFILE_DUPLICATE_KEYS:" in source:
            source, count = re.subn(marker, lambda _: block, source, flags=re.DOTALL)
            if count != 1:
                raise ValueError("ambiguous merged source duplicate witness")
        else:
            position = source.index("    assert_eq!(\n        profiles.records.len(),")
            source = source[:position] + block + source[position:]
        term = "            - MERGED_PROFILE_DUPLICATE_KEYS.len()\n"
        if term not in source:
            position = source.index("    assert_eq!(\n        profiles.records.len(),")
            position = source.index("\n    );", position)
            source = source[:position] + "\n" + term.rstrip("\n") + source[position:]
    if registered_engine and legacy_sum:
        source = re.sub(
            r"additions\.len\(\)\s*-\s*[0-9_]+",
            f"additions.len() - {registered_engine + len(shared)}",
            source,
        )
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
    if additions and legacy_sum:
        cohort_block = "    // Generated from closed source packets minus pinned pre-SRX3 identities.\n"
        if coowners:
            cohort_block += "    // W10-PRIMAL's independently pinned shared keys are already counted above.\n"
        cohort_block += "    const SRX3_SOURCE_COHORTS: &[(&str, usize)] = &[\n"
        cohort_block += "".join(
            f'        ("{name}", {count}),\n' for name, count in additions
        )
        cohort_block += "    ];\n"
        pattern = r"    // Generated from closed source packets minus pinned pre-SRX3 identities\.\n(?:    // W10-PRIMAL's independently pinned shared keys are already counted above\.\n)?    const SRX3_SOURCE_COHORTS:.*?    ];\n"
        if "const SRX3_SOURCE_COHORTS:" in source:
            source, replaced = re.subn(
                pattern, lambda _: cohort_block, source, flags=re.DOTALL
            )
            if replaced != 1:
                raise ValueError("ambiguous generated SRX3 cohort block")
        else:
            position = source.index("    assert_eq!(\n        profiles.records.len(),")
            source = source[:position] + cohort_block + source[position:]
        term = (
            "            + SRX3_SOURCE_COHORTS.iter().map(|(_, n)| n).sum::<usize>()\n"
        )
        if term not in source:
            engine_terms = list(
                re.finditer(
                    r"^            \+ (?:additions\.len\(\)|\(additions\.len\(\) - [0-9_]+\))\n",
                    source,
                    re.MULTILINE,
                )
            )
            if len(engine_terms) != 1:
                raise ValueError("ambiguous registered engine contribution")
            position = engine_terms[0].end()
            source = source[:position] + term + source[position:]
    if custody is not None:
        if not set(custody["added_keys"]) <= actual:
            raise ValueError("custody source identity absent")
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
    original = TEST.read_text()
    source = first_dragon_item_profiles.cohort_receipt(original)
    source = m01c_key_door_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/key-door-item-profiles.json"
            ).read_bytes()
        ),
    )
    source = questpc4_treasure_reward_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/questpc4-treasure-reward-profiles.json"
            ).read_bytes()
        ),
    )
    import qengine_equipment_cohort

    source = qengine_equipment_cohort.refresh(
        source, json.loads((ROOT / paths[2]).read_bytes())
    )
    source = qengine_reward_equipment_pin.refresh(
        source, json.loads((ROOT / paths[2]).read_bytes())
    )
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
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/qready1-quest-item-profiles.json"
            ).read_bytes()
        ),
    )
    source = qready1_craft_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/qready1-craft-item-profiles.json"
            ).read_bytes()
        ),
    )
    source = qready1_cradle_equipment_pin.refresh(
        source,
        json.loads(
            (
                ROOT
                / "tools/content-schema/native-gameplay/qready1-cradle-item-profiles.json"
            ).read_bytes()
        ),
    )
    import engnpc2_slug_corpse_profiles

    source = engnpc2_slug_corpse_profiles.refresh_cohort(
        source,
        json.loads((ROOT / engnpc2_slug_corpse_profiles.PATH).read_bytes()),
    )
    import equipment_source_overlaps

    srx3 = source_cohorts(ROOT)
    boss_corpses = keys(
        next(document for name, document, _ in srx3 if name == "SRX3-BOSS-CORPSES")
    )
    source = equipment_source_overlaps.refresh(source, ROOT, boss_corpses)
    import equipment_known_speed_pin

    source = equipment_known_speed_pin.refresh(
        source,
        equipment_known_speed_pin.producer.capture(
            ROOT, ids=equipment_known_speed_pin.IDS
        ),
    )
    census_path = (
        ROOT
        / "tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json"
    )
    custody = (
        json.loads(census_path.read_text())
        if "questpc8-equipment-custody-census.json" in source
        else None
    )
    baseline_cohorts = None
    baseline_qengine_count = None
    if custody is not None:
        published = subprocess.check_output(
            ["git", "show", custody["base"] + ":" + TEST.relative_to(ROOT).as_posix()],
            cwd=ROOT,
            text=True,
        )
        baseline_cohorts = declared_cohorts(published)
        baseline_qengine = subprocess.check_output(
            [
                "git",
                "show",
                custody["base"]
                + ":tools/content-schema/native-gameplay/qengine-reward-item-profiles.json",
            ],
            cwd=ROOT,
        )
        baseline_qengine_count = len(keys(json.loads(baseline_qengine)))
    documents = [json.loads((ROOT / p).read_text()) for p in paths]
    documents[2], documents[3] = registered_engine_npc_documents(
        ROOT, documents[2], documents[3]
    )
    # Compare the independently generated source rows after replaying the same
    # qualified Global facts that supersede them in the final native provider.
    from forge_item_profiles import owners, project
    from import_cyclopedia_global import load_capture
    from item_detail_source_facts import apply_capture_stats
    from ranged_item_profiles import apply_authoring
    from srx1_profile_updates import exact_source_keys, omit_unknown_attributes

    capture = load_capture(ROOT)
    forge_owners = owners(ROOT)
    for name, document, _ in srx3:
        apply_capture_stats(document, capture)
        project(document, forge_owners)
        if name == "M25-GALTHEN-YIELDS":
            apply_authoring(ROOT, document)
    coowners = source_coowners(ROOT)
    exact_keys = exact_source_keys(json.loads((ROOT / paths[2]).read_bytes()))
    for _, document, _ in (*srx3, *coowners):
        for row in document["records"]:
            if row["production_definition"]["production_key"] not in exact_keys:
                omit_unknown_attributes(row)
    generated, count = regenerate(
        register(source),
        *documents,
        srx3=srx3,
        coowners=coowners,
        engine_npc_shared=("oteryn:item.tibia.i27654",),
        duplicate_keys=merged_source_duplicates(ROOT, source),
        custody=custody,
        baseline_cohorts=baseline_cohorts,
        baseline_qengine_count=baseline_qengine_count,
        overlap_keys=(
            set()
            if custody is not None
            else {
                f"oteryn:item.tibia.i{i}"
                for i in __import__(
                    "qengine_reward_item_profiles"
                ).PREVIOUSLY_COUNTED_IDS
            }
        ),
        frozen=None if custody is not None else frozen_union(),
    )
    generated = subprocess.check_output(
        ["rustfmt", "--edition", "2024", "--config", "skip_children=true"],
        input=generated,
        text=True,
        cwd=ROOT,
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
