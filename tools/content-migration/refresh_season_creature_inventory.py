"""Refresh strict inventory pins for source-qualified Creature and raid additions."""

import argparse
from collections import Counter
import json
from pathlib import Path
import re

import cake_native_successor as cake
import crystal_creature_parity as crystal
from cyclopedia_creatures import inventory as cyclopedia_inventory
import raid_catalogue
from crystal_spawn_creatures import KEYS as CRYSTAL_SPAWN_KEYS
from world_project_v2_to_tree import CREATURE_FAMILIES, git_blob_sha

ROOT = Path(__file__).resolve().parents[2]


def validate_inventory(root, rows, native, schedules):
    """Validate each source cohort before repinning the combined inventory."""
    from questpc7_thaian_authoring import PACKET

    packet, _ = cake.read_canonical(root / PACKET)
    if packet["source"]["page_id"] != 48123 or packet["source"]["revision"] != 445240:
        raise ValueError("Thaian source authoring revision")
    profiles = {row["target"]["family"]: row for row in packet["profiles"]}
    records = {row["identity"]["family"]: row for row in packet["records"]}
    authoring = profiles["Creature"]["data"]
    canonical_authoring = {**authoring, "profile": dict(authoring["profile"])}
    # The native World writer omits an empty immunity list. The successor still
    # requires its explicit reference_level and every other pinned source field.
    if canonical_authoring["profile"].get("immunities") == []:
        del canonical_authoring["profile"]["immunities"]
    key = "oteryn:creature.thaian"
    canonical = [row for row in rows if row["definition"]["identity"]["key"] == key]
    active = [row for row in native if row["profile"]["target"]["key"] == key]
    # The typed World writer omits empty immunities and a null reference level.
    # world_project_v2_to_tree restores reference_level=None in Creature shards.
    # Derive that complete tree form from source; native rows remain raw-exact.
    canonical_authoring = cake.source_profile(profiles["Creature"])["data"]
    canonical_authoring["profile"]["details"].setdefault("reference_level", None)
    if (
        len(canonical) != 1
        or len(active) != 1
        or canonical[0]["definition"] != records["Creature"]
        or canonical[0]["authoring"] not in (authoring, canonical_authoring)
        or active[0]["profile"] != profiles["Creature"]
        or active[0]["behavior"] != profiles["Behavior"]
    ):
        raise ValueError(
            "Thaian source authoring closure: "
            + json.dumps(
                {
                    "source": packet["source"],
                    "counts": {"canonical": len(canonical), "native": len(active)},
                    "expected": {
                        "definition": records["Creature"],
                        "canonical_authoring_forms": [
                            profiles["Creature"]["data"],
                            canonical_authoring,
                        ],
                        "native_profile": profiles["Creature"],
                        "native_behavior": profiles["Behavior"],
                    },
                    "observed": {"canonical": canonical, "native": active},
                },
                sort_keys=True,
                separators=(",", ":"),
            )
        )
    behavior = active[0]["behavior"]["data"]["profile"]
    attacks = behavior.get("attacks", [])
    if len(attacks) != 1 or behavior.get("defenses", []):
        raise ValueError("Thaian exact melee schedule")
    ability = [
        row
        for row in active[0]["attack_definitions"]["profiles"]
        if row["target"] == attacks[0]["ability"]
    ]
    if len(ability) != 1 or ability[0]["data"]["profile"]["details"]["kind"] != "Melee":
        raise ValueError("Thaian exact melee ability")
    # Thaian extends the canonical Cake cohort; independently qualified
    # native-only Fugue and Elemental bodies retain their exact schedule proofs.
    from engenc2_fugue_profile import inventory_extension

    extra_native, extra_schedules = inventory_extension(root, native)
    from engenc_elemental_profiles import inventory_extension as elemental_inventory

    elemental_native, elemental_schedules = elemental_inventory(native)
    extra_native += elemental_native
    extra_schedules += elemental_schedules
    from nrwicked_profiles import inventory_extension as nrwicked_inventory

    _, nrwicked_schedules = nrwicked_inventory(root, native)
    extra_schedules += nrwicked_schedules
    additions, added_schedules, _, _ = cyclopedia_inventory(root, rows, native)
    from crystal_spawn_creatures import qualify

    reference = cake.read_project_document(root, "definitions/reference.json")
    _, _, _, delta = qualify(root, reference["records"])
    crystal_keys = CRYSTAL_SPAWN_KEYS
    crystal_rows = {
        row["definition"]["identity"]["key"]: row
        for row in rows
        if row["definition"]["identity"]["key"] in crystal_keys
    }
    crystal_native = {
        row["profile"]["target"]["key"]: row
        for row in native
        if row["profile"]["target"]["key"] in crystal_keys
    }
    if crystal_rows.keys() != crystal_keys or crystal_native.keys() != crystal_keys:
        raise ValueError("Crystal placement Creature inventory is incomplete")
    crystal_schedules = 0
    crystal_attacks = Counter()
    for creature in crystal_keys:
        definition = delta.records["Creature", creature]
        authoring = cake.source_profile(delta.profiles["Creature", creature])["data"]
        for field in ("abilities", "resistances"):
            if authoring["profile"].get(field) == []:
                del authoring["profile"][field]
        authoring["profile"]["details"].setdefault("reference_level", None)
        behavior = delta.profiles["Behavior", definition["behavior"]["key"]]
        if (
            crystal_rows[creature]["definition"] != definition
            or crystal_rows[creature]["authoring"] != authoring
            or crystal_native[creature]["profile"]["data"] != authoring
            or crystal_native[creature]["behavior"] != behavior
        ):
            raise ValueError("Crystal placement Creature inventory changed: " + creature)
        schedule = behavior["data"]["profile"]
        crystal_schedules += sum(len(schedule.get(key, [])) for key in ("attacks", "defenses"))
        for attack in schedule.get("attacks", []):
            ability = delta.profiles["Ability", attack["ability"]["key"]]
            crystal_attacks[ability["data"]["profile"]["details"]["kind"]] += 1
    if crystal_schedules != 10:
        raise ValueError("Crystal spawn schedule inventory")
    packet, _ = crystal.read_packet(root)
    canonical = {crystal.identity(row["definition"]["identity"]): row for row in rows}
    active = {crystal.identity(row["profile"]["target"]): row for row in native}
    prior = crystal.prior_cohort_keys(root)
    parity_schedules = parity_creatures = 0
    for entry in packet["definitions"]:
        key = crystal.identity(entry["identity"])
        definition = next(row for row in entry["records"] if row["identity"] == entry["identity"])
        authored = next(row for row in entry["profiles"] if row["target"] == definition["behavior"])
        if (
            key not in active
            or key not in canonical
            or canonical[key]["definition"] != definition
            or canonical[key]["source_bindings"].count(entry["binding"]) != 1
        ):
            raise ValueError(f"Crystal Creature inventory mismatch: {key}")
        parity_creatures += key[1] not in prior
        if key[1] in crystal.ACTIVE_ENCOUNTER_KEYS:
            continue
        if active[key]["behavior"] != authored:
            raise ValueError(f"Crystal Creature Behavior inventory mismatch: {key}")
        if key[1] not in prior:
            parity_schedules += sum(
                len(authored["data"]["profile"].get(group, [])) for group in ("attacks", "defenses")
            )
    for entry in packet["schedules"]:
        key = crystal.identity(entry["creature"])
        behavior = active[key]["behavior"]["data"]["profile"]
        if behavior.get(entry["group"], []).count(entry["schedule"]) != 1:
            raise ValueError(f"Crystal schedule inventory mismatch: {key}")
        parity_schedules += 1
    if (
        len(rows) - 1 - additions - len(crystal_keys) != 1863 + 1 + parity_creatures
        or len(native) != len(rows) + extra_native
        or schedules - 1 - added_schedules - crystal_schedules
        != 7201 + 4 + extra_schedules + parity_schedules
    ):
        raise ValueError("Non-seasonal canonical inventory changed; review its source first")
    return 1, crystal_attacks


def generated(root):
    read = lambda path: json.loads((root / path).read_bytes())
    index = read("content/creatures/definitions/index.json")
    rows = [row for path in index["shards"] for row in read(path)["records"]]
    packet, _ = cake.read_canonical(root / cake.PACKET)
    declarations = cake.read_project_document(root, "definitions/declarations.json")
    reference = cake.read_project_document(root, "definitions/reference.json")
    # Crystal data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:79,118:
    # replay main's hash-checked tree-first empty-loot admission before exact family equality.
    from primal_reference_overlay import apply_reference

    apply_reference(reference["records"], root)
    admitted = cake.qualify(
        rows, declarations, reference, packet, read("imports/crystalserver/bindings/items.json")
    )
    native = read("content/creatures/definitions/spell-native-profiles.json")["records"]
    native = [
        row for row in native if row["profile"]["target"]["key"].startswith("oteryn:creature.")
    ]
    (selected,) = [row for row in native if row["profile"]["target"]["key"] == cake.CAKE[1]]
    if selected["profile"]["data"] != admitted["authoring"]:
        raise ValueError("Cake native profile differs from its qualified source")
    behavior = selected["behavior"]["data"]["profile"]
    # Crystal201189f cake_golem.lua:76-95: three attack slots, one defence slot.
    if (len(behavior["attacks"]), len(behavior["defenses"])) != (3, 1):
        raise ValueError("Cake source schedule inventory")
    schedules = sum(
        len(row["behavior"]["data"]["profile"].get(key, []))
        for row in native
        for key in ("attacks", "defenses")
    )
    thaian_melee, crystal_attacks = validate_inventory(root, rows, native, schedules)
    _, _, cyclopedia_melee, cyclopedia_spells = cyclopedia_inventory(root, rows, native)
    from engenc_elemental_profiles import inventory_extension as elemental_inventory

    elemental_native, _ = elemental_inventory(native)
    result = {}

    def replace(path, pattern, replacement):
        text = result.get(path, (root / path).read_text())
        text, count = re.subn(pattern, replacement, text)
        if count != 1:
            raise ValueError("Missing or ambiguous inventory seam: " + path)
        result[path] = text

    replace(
        "crates/gameplay/monster-ai/src/ai_think/profile_schedule/tests.rs",
        r'assert_eq!\(creatures.len\(\), [0-9_]+, "complete canonical Creature corpus"\);',
        f'assert_eq!(creatures.len(), {len(rows)}, "complete canonical Creature corpus");',
    )
    replace(
        "crates/gameplay/monster-ai/src/ai_think/native_profile_tests.rs",
        r"\([0-9_]+, [0-9_]+, 15, 8\)",
        f"({len(native)}, {schedules}, 15, 8)",
    )
    # Greater Fire adds two CreateItem occurrences; Greater Energy adds none.
    # Their exact, source-qualified two-profile cohort adds two occurrences.
    # Keep the source-census assertion exact for that complete cohort.
    source_item_additions = sum(
        entry.get("owner") == "source_item" for entry in crystal.read_packet(root)[0]["schedules"]
    )
    replace(
        "crates/gameplay/content/src/content/native_gameplay.rs",
        r"assert_eq!\(\(create, remove\), \([0-9_]+, [0-9_]+\)\);",
        f"assert_eq!((create, remove), ({62 + elemental_native}, {20 + source_item_additions}));",
    )
    parity, _ = crystal.read_packet(root)
    counts = Counter()
    abilities = {
        crystal.identity(row["target"]): row["data"]["profile"].get("details")
        for row in declarations["authoring_profiles"]
        if row["target"]["family"] == "Ability"
    }
    prior = crystal.prior_cohort_keys(root)
    for entry in parity["definitions"]:
        if entry["identity"]["key"] in prior:
            continue
        body = next(
            row["data"]["profile"]
            for row in entry["profiles"]
            if row["target"]["family"] == "Behavior"
        )
        for attack in body.get("attacks", []):
            details = abilities[crystal.identity(attack["ability"])]
            counts[details["kind"]] += 1
    for entry in parity["schedules"]:
        if entry["group"] != "attacks":
            continue
        if entry.get("owner") == "source_item":
            counts["SourceItem"] += 1
        else:
            counts["Spell"] += 1

    # The generic spell constructor intentionally leaves source Item casts to
    # their existing Item owner. The parser census still counts those slots.
    path = "crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane.rs"
    replace(
        path,
        r'\("melee:Ok\(\(\)\)", [0-9_]+\)',
        f'("melee:Ok(())", {1689 + 1 + thaian_melee + counts["Melee"] + cyclopedia_melee + crystal_attacks["Melee"]})',
    )
    replace(
        path,
        r'\("spell:Ok\(\(\)\)", [0-9_]+\)',
        f'("spell:Ok(())", {4198 + len(behavior["attacks"]) - 1 + counts["Spell"] + cyclopedia_spells + crystal_attacks["Spell"]})',
    )
    replace(
        path,
        r'\("spell:Err\(UnsupportedShape\)", [0-9_]+\)',
        f'("spell:Err(UnsupportedShape)", {116 + counts["SourceItem"]})',
    )

    # Crystal cake_golem.lua:11-19: [2,3,5],30 => easy / very rare.
    histogram = Counter()
    for row in rows:
        bestiary = row["authoring"]["profile"].get("bestiary")
        if bestiary:
            histogram[
                bestiary["difficulty"]
                + (" very rare" if bestiary["occurrence"] == "very_rare" else "")
            ] += 1
    path = "crates/gameplay/progression/src/bestiary_charm_crystal_parity_tests.rs"
    text = (root / path).read_text()
    start = text.index("    let expected: BTreeMap<&str, usize> = [")
    end = text.index("    ]", start)
    block = text[start:end]
    if set(re.findall(r'\("([^"]+)", [0-9_]+\)', block)) != set(histogram):
        raise ValueError("Bestiary difficulty categories changed")
    block = re.sub(r'\("([^"]+)", [0-9_]+\)', lambda m: f'("{m[1]}", {histogram[m[1]]})', block)
    result[path] = text[:start] + block + text[end:]
    replace(
        path,
        r"assert_eq!\(rows.len\(\), [0-9_]+\);",
        f"assert_eq!(rows.len(), {sum(histogram.values())});",
    )

    outputs = raid_catalogue.outputs(root, register=False)
    if outputs[raid_catalogue.CATALOGUE] != (root / raid_catalogue.CATALOGUE).read_bytes():
        raise ValueError("Regenerate the source-qualified raid catalogue first")
    if outputs[raid_catalogue.INDEX] != (root / raid_catalogue.INDEX).read_bytes():
        raise ValueError("Regenerate the source-qualified raid index first")
    raids = json.loads(outputs[raid_catalogue.CATALOGUE])["records"]
    active = sum(row["active"] for row in raids)
    hare = raid_catalogue.HALLOWEEN_HARE_KEY
    held = raid_catalogue.GLOBAL_RAID_HOLDS.keys() | (raid_catalogue.HALLOWEEN.keys() - {hare})
    inactive = {row["key"] for row in raids if not row["active"]}
    backpack_raid = [row for row in raids if row["key"] == "raid:venore.orcsambackpack"]
    if (
        len(raids) != 239 + 1
        or active != 136 + 1 + 1 + 1
        or not held <= inactive
        or hare in inactive
        or len(backpack_raid) != 1
        or not backpack_raid[0]["active"]
    ):
        raise ValueError("Non-seasonal raid inventory changed; review its source first")
    path = "crates/gameplay/encounter-rules/src/raid/catalogue.rs"
    replace(
        path,
        r"assert_eq!\(catalogue.records\(\).len\(\), [0-9_]+\);",
        f"assert_eq!(catalogue.records().len(), {len(raids)});",
    )
    replace(
        path,
        r"assert_eq!\(catalogue.active\(\).count\(\), [0-9_]+\);",
        f"assert_eq!(catalogue.active().count(), {active});",
    )
    # A merge can retain the fresh shards but take the other side's old central
    # summaries. Verify exact canonical membership before republishing summaries.
    manifest, lock = read("content/manifest.json"), read("content/content.lock.json")
    managed = {row["path"] for row in manifest["managed_files"]}
    for family, (node, stem) in CREATURE_FAMILIES.items():
        registration = manifest["families"][family]
        family_index = read(registration["index"])
        actual = [
            row["definition"]["identity"]
            for shard in family_index["shards"]
            for row in read(shard)["records"]
        ]
        expected = [
            row["identity"] for row in reference["records"] if row["identity"]["family"] == family
        ]
        identities = lambda values: {cake.identity(value) for value in values}
        if (
            len(actual) != family_index["record_count"]
            or len(identities(actual)) != len(actual)
            or identities(actual) != identities(expected)
        ):
            raise ValueError("Canonical family summary mismatch: " + family)
        shard_name = re.compile(re.escape(node + stem) + r"-[0-9]{5}-[0-9]{5}\.json$")
        if any(not shard_name.fullmatch(path) for path in family_index["shards"]):
            raise ValueError("Canonical family shard path mismatch: " + family)
        managed = {path for path in managed if not shard_name.fullmatch(path)}
        managed.update(family_index["shards"])
        registration["records"] = len(actual)
        lock["family_counts"][family] = len(actual)
    manifest["families"]["Encounter.Raid"]["records"] = len(raids)
    manifest["managed_files"] = [{"path": path} for path in sorted(managed)]
    lock["family_counts"]["Encounter.Raid"] = len(raids)
    bindings = sum(len(row["source_bindings"]) for row in rows)
    if bindings != index["attached_source_bindings"]:
        raise ValueError("Canonical Creature binding summary mismatch")
    lock["source_binding_counts"]["Creature"] = bindings
    lock["authoring_profile_counts"] = dict(
        sorted(
            Counter(row["target"]["family"] for row in declarations["authoring_profiles"]).items()
        )
    )
    for name, locator in {
        "reference": "definitions/reference.json",
        "declarations": "definitions/declarations.json",
        "editor": "editor/author.json",
        "sources": "provenance/sources.json",
        "imports": "provenance/imports.json",
    }.items():
        lock["legacy_blobs"][name] = git_blob_sha(root / "content/world" / locator)
    for path, value in [("content/manifest.json", manifest), ("content/content.lock.json", lock)]:
        result[path] = json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, text in generated(ROOT).items():
        if args.check:
            if (ROOT / path).read_text() != text:
                raise SystemExit("Stale seasonal inventory: " + path)
        elif (ROOT / path).read_text() != text:
            (ROOT / path).write_text(text)
    print("PASS source-qualified seasonal Creature, schedule, Bestiary and raid pins")
