#!/usr/bin/env python3
import copy
import importlib.util
import json
import copy
import re
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def test_relation_rows_route_by_owner():
    """D322: PR Use validation applies to PR Use rows only; main-only rows keep main's checks."""
    spec = importlib.util.spec_from_file_location("migration_d322", ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py")
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    targets = migration.use_relation_targets()
    relations = {migration.target_id(row["source"]) for row in migration.load(ROOT / "content/items/relations/items.json")["records"]}
    real_use = migration.validate_use_relation
    routed = []
    migration.validate_use_relation = lambda row, definition: (routed.append(migration.target_id(row["source"])), real_use(row, definition))
    migration.main()
    migration.validate_use_relation = real_use
    # Main-only rows exist and none of them reaches the PR Use validator.
    assert len(routed) == 55 and set(routed) <= targets and relations - targets, "main-only relation row routed to Use"
    # A PR Use row whose sealed Use owner is lost falls out of the closed55 and fails closed.
    dropped = sorted(set(routed))[0]
    real_targets = migration.use_relation_targets
    migration.use_relation_targets = lambda: real_targets() - {dropped}
    try:
        migration.main()
    except migration.ValidationError as error:
        assert str(error) == "RELATION_COVERAGE", error
    else:
        raise AssertionError("PR Use row without its sealed owner accepted")
    finally:
        migration.use_relation_targets = real_targets
    print("ok test_relation_rows_route_by_owner")


def test_closed_weapon_metadata_admission():
    spec = importlib.util.spec_from_file_location("migration", ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py")
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    rows, receipt = migration.closed_weapon_metadata()
    owners = {migration.target_id(row["item"]): copy.deepcopy(row) for row in receipt["parent_authoring"]}
    for key, row in rows.items():
        owners.setdefault(key, {"item": row["target"]}).update(copy.deepcopy(row["facts"]))
    migration.extend_forge289_owners(owners)
    aliases = migration.item_alias_targets()
    staged = {migration.staged_item_target(row["target"], aliases): row for row in migration.load(
        ROOT / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json")["items"]}
    migration.validate_item_authoring_targets(owners, staged)
    key = list(rows)[-1]
    field = next(iter(rows[key]["facts"]))
    variants = []
    for mutation in (lambda row: row.pop(field), lambda row: row.update(use_ability="invented"),
                     lambda row: row["item"].update(revision="wrong"), lambda row: row.update({field: False})):
        bad = copy.deepcopy(owners)
        mutation(bad[key])
        variants.append(bad)
    wrong = copy.deepcopy(owners)
    wrong[key][field] = {"numerator": 1, "denominator": 100} if isinstance(wrong[key][field], dict) else wrong[key][field] + 1
    variants += [wrong, {k: v for k, v in owners.items() if k != key}, owners | {("Item", "unlisted", "definition-r1"): {}}]
    for bad in variants:
        try:
            migration.validate_item_authoring_targets(bad, staged)
        except migration.ValidationError:
            pass
        else:
            raise AssertionError("unqualified weapon owner accepted")
    admitted = {migration.target_id(row["source"]): row for row in receipt["new_derived_relations"]}
    definitions = {migration.target_id(row["identity"]): row for row in migration.load(
        ROOT / "content/world/definitions/reference.json")["records"]}
    relation_key = next(iter(admitted))
    row, definition = admitted[relation_key], definitions[relation_key]
    migration.validate_weapon_relation(row, definition, rows, admitted)
    for value in (None, 0, True):
        bad = copy.deepcopy(definition)
        bad["semantics"]["imbuement"] = {"state": "UNKNOWN"} if value is None else {
            "state": "KNOWN", "value": {"slot_count": {"state": "KNOWN", "value": value}}}
        try:
            migration.validate_weapon_relation(row, bad, rows, admitted)
        except migration.ValidationError:
            pass
        else:
            raise AssertionError("unknown/non-positive slot accepted")
    for mutation in (lambda r: r["relations"][0].update(basis="weapon_attack_modifier_points"),
                     lambda r: r["relations"][0].update(ruleset="rulesets/items/exaltation-forge/"),
                     lambda r: r["relations"].append(copy.deepcopy(r["relations"][0])),
                     lambda r: r["source"].update(key="unlisted"), lambda r: r.update(extra="unqualified")):
        bad = copy.deepcopy(row)
        mutation(bad)
        try:
            migration.validate_weapon_relation(bad, definition, rows, admitted)
        except migration.ValidationError:
            pass
        else:
            raise AssertionError("unqualified weapon relation accepted")
    read_bytes = Path.read_bytes
    with patch("pathlib.Path.read_bytes", lambda p: b"forged" if p.name == "OTV2-20261002-item-weapon-metadata-promotion-v1.json" else read_bytes(p)):
        try:
            migration.closed_weapon_metadata()
        except migration.ValidationError as error:
            assert str(error) == "WEAPON_METADATA_PACKET_DIGEST"
        else:
            raise AssertionError("forged weapon packet accepted")
    print("ok test_closed_weapon_metadata_admission")
source_claim_count = len(json.loads((ROOT / "tools/content-schema/quest-authoring/samples/chests/claims.json").read_text())["claims"])
project = json.loads((ROOT / "content/project.json").read_text(encoding="utf-8"))
manifest = json.loads((ROOT / "content/manifest.json").read_text(encoding="utf-8"))
lock = json.loads((ROOT / "content/content.lock.json").read_text(encoding="utf-8"))

assert project["runtime_source"] == "legacy_until_separately_qualified"
assert manifest["compatibility"] == {
    "legacy_mutated": False,
    "legacy_root": "content/world",
    "runtime_switch_authorized": False,
}
from world_project_v2_to_tree import retained_quest_registration
quest_families, quest_paths = retained_quest_registration(ROOT)
if quest_families:
    assert "Quest" in project["migrated_families"] and "Quest" not in project["next_population_families"]
    assert set(quest_paths).issubset({row["path"] for row in manifest["managed_files"]})

reference = json.loads((ROOT / 'content/world/definitions/reference.json').read_text())
creature_families = ['Creature', 'Presentation', 'Behavior', 'Loot', 'Ability', 'Effect', 'Formula']
family_counts = {family: sum(row['identity']['family'] == family for row in reference['records']) for family in creature_families}
assert family_counts['Creature'] == 1863
assert lock["family_counts"] == {
    "Item": 34043, "Mount": 252,
    **family_counts, "Document": 1609,
    "NPC": 1110, "Dialogue": 694, "Service.Trade": 324, "Service.Travel": 56, "Encounter": 108, "Charm": 25,
    "Proficiency": 443, "RewardClaim": source_claim_count, "StarterKit": 1,
    **{family: value["records"] for family, value in quest_families.items()},
}
assert lock["source_binding_counts"] == {"Item": 476, "Mount": 252, "Creature": 1863, "Encounter": 108, "NPC": 2376}
assert lock["editor_entry_counts"] == {"Item": 165, "Mount": 252}

paths = [row["path"] for row in manifest["managed_files"]]
assert len(paths) == len(set(paths))
shard_name = re.compile(r"-\d{5}-\d{5}\.json$")
for directory in {(ROOT / path).parent for path in paths if shard_name.search(path)}:
    stale = sorted(file.relative_to(ROOT).as_posix() for file in directory.glob("*.json")
                   if shard_name.search(file.name) and file.relative_to(ROOT).as_posix() not in paths)
    assert not stale, stale
assert sum(path.startswith("content/items/definitions/items-") for path in paths) == 69
assert any(path.startswith("content/cosmetics/mounts/mounts-") for path in paths)
assert any(path.startswith("content/creatures/definitions/creatures-") for path in paths)
assert any(path.startswith("content/npcs/definitions/npcs-") for path in paths)
assert any(path.startswith("content/dialogues/definitions/dialogues-") for path in paths)
assert any(path.startswith("content/encounters/definitions/encounters-") for path in paths)
assert any(path.startswith("content/services/trade/trade-") for path in paths)
assert any(path.startswith("content/services/travel/travel-") for path in paths)
assert "imports/canary/bindings/creatures.json" in paths
assert "imports/tibiawiki/bindings/creatures.json" in paths
assert all(not path.startswith("content/world/") for path in paths)

assert manifest["families"]["NPC"] == {"records": 1110, "index": "content/npcs/definitions/index.json"}
assert manifest["families"]["Dialogue"] == {"records": 694, "index": "content/dialogues/definitions/index.json"}
assert manifest["families"]["Service.Trade"] == {"records": 324, "index": "content/services/trade/index.json"}
assert manifest["families"]["Service.Travel"] == {"records": 56, "index": "content/services/travel/index.json"}
assert manifest["families"]["Charm"] == {"records": 25, "index": "content/charms/index.json"}
assert "Charm" in project["migrated_families"] and "Charm" not in project["next_population_families"]
assert any(path.startswith("content/charms/charms-") for path in paths) and "content/charms/index.json" in paths
assert manifest["families"]["Proficiency"] == {"records": 443, "index": "content/proficiencies/index.json"}
assert "Proficiency" in project["migrated_families"] and "Proficiency" not in project["next_population_families"]
assert sum(path.startswith("content/proficiencies/proficiencies-") for path in paths) == 3
assert "content/proficiencies/index.json" in paths and "content/proficiencies/bindings.json" in paths
assert manifest["families"]["RewardClaim"] == {"records": source_claim_count, "index": "content/interactions/reward_claims/index.json"}
assert "RewardClaim" in project["migrated_families"]
assert sum(path.startswith("content/interactions/reward_claims/reward-claims-") for path in paths) == 3
assert "content/interactions/reward_claims/index.json" in paths
assert manifest["families"]["StarterKit"] == {"records": 1, "index": "content/starter/index.json"}
assert "StarterKit" in project["migrated_families"]
assert "content/starter/starter-kits-00000-00000.json" in paths and "content/starter/index.json" in paths
assert "NPC" in project["migrated_families"] and "Dialogue" in project["migrated_families"] and "Service" in project["migrated_families"]
assert "NPC" not in project["next_population_families"] and "Dialogue" not in project["next_population_families"] and "Service" not in project["next_population_families"]

# Keep the standalone Quest preservation guards on the migration workflow's test path.
subprocess.run([sys.executable, str(ROOT / "tools/content-migration/test_quest_registration_preservation.py")], check=True)

# Core Item admission is a separate accepted data overlay, retained by every regeneration.
subprocess.run([sys.executable, str(ROOT / "tools/content-migration/test_quest_reward_item_semantics.py")], check=True)
subprocess.run([sys.executable, str(ROOT / "tools/content-migration/quest_reward_item_semantics.py"), "--check"], check=True)

assert manifest['families']['Document'] == {'records': 1609, 'index': 'content/documents/index.json'}
assert 'Document' in project['migrated_families']
assert sum(path.startswith('content/documents/documents-') for path in paths) == 4
assert 'content/documents/index.json' in paths and 'Document' not in lock['source_binding_counts']


def test_document_roundtrip_negatives(declarations, sources):
    import validate_world_project_v2_to_tree as validator
    docs = [row for row in declarations['records'] if row.get('kind') == 'Document']
    rows = [{'declaration': row} for row in docs]
    assert validator.validate_document_rows(rows, declarations, sources) == 1609
    bad_rows = copy.deepcopy(rows); bad_rows[0]['declaration']['content'][0] += ' altered'
    bad_reference = copy.deepcopy(declarations)
    creature = next(row for row in bad_reference['authoring_profiles'] if row['data']['kind'] == 'Creature')
    creature['data']['profile'].setdefault('details', {})['encyclopedia_document'] = {'family': 'Document', 'key': 'oteryn:document/missing', 'revision': 'definition-r1'}
    bad_binding = copy.deepcopy(sources)
    bad_binding['source_identity_bindings'].append({'target': {'family': 'Document', **docs[0]['identity']}})
    for args in [(rows[:-1], declarations, sources), (bad_rows, declarations, sources),
                 (rows, bad_reference, sources), (rows, declarations, bad_binding)]:
        try:
            validator.validate_document_rows(*args)
        except validator.ValidationError:
            pass
        else:
            raise AssertionError('Dropped/rewritten Document, dangling ref or fabricated binding was accepted')


test_document_roundtrip_negatives(json.loads((ROOT / 'content/world/definitions/declarations.json').read_text()),
                                  json.loads((ROOT / 'content/world/provenance/sources.json').read_text()))

# The RewardClaim family has no legacy source: its own authoring tool must reproduce it exactly.
reward_claim_tool = ROOT / "tools" / "content-schema" / "reward-claim-authoring"
for script in ("test_reward_claim_authoring.py", "test_reward_claim_variant_authoring.py"):
    subprocess.run([sys.executable, script], cwd=reward_claim_tool, check=True)
# StarterKit likewise.
starter_kit_tool = ROOT / "tools" / "content-schema" / "starter-kit-authoring"
subprocess.run([sys.executable, "test_starter_kit_authoring.py"], cwd=starter_kit_tool, check=True)

test_closed_weapon_metadata_admission()
test_relation_rows_route_by_owner()
subprocess.run([sys.executable, str(ROOT / "tools/content-migration/test_item_taxonomy.py")], check=True)
subprocess.run([sys.executable, str(ROOT / "tools/content-migration/test_item_official_navigation.py")], check=True)
print(f"PASS managed_files={len(paths)} item_shards=69")
