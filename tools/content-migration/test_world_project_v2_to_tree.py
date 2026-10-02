#!/usr/bin/env python3
import json
import copy
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
project = json.loads((ROOT / "content/project.json").read_text(encoding="utf-8"))
manifest = json.loads((ROOT / "content/manifest.json").read_text(encoding="utf-8"))
lock = json.loads((ROOT / "content/content.lock.json").read_text(encoding="utf-8"))

assert project["runtime_source"] == "legacy_until_separately_qualified"
assert manifest["compatibility"] == {
    "legacy_mutated": False,
    "legacy_root": "content/world",
    "runtime_switch_authorized": False,
}
reference = json.loads((ROOT / 'content/world/definitions/reference.json').read_text())
creature_families = ['Creature', 'Presentation', 'Behavior', 'Loot', 'Ability', 'Effect', 'Formula']
family_counts = {family: sum(row['identity']['family'] == family for row in reference['records']) for family in creature_families}
assert family_counts['Creature'] == 1763
assert lock["family_counts"] == {
    "Item": 34031, "Mount": 252,
    **family_counts, "Document": 1609,
    "NPC": 1110, "Dialogue": 694, "Service.Trade": 324, "Service.Travel": 56, "Encounter": 104, "Charm": 25,
    "Proficiency": 443, "RewardClaim": 231, "StarterKit": 1,
}
assert lock["source_binding_counts"] == {"Item": 165, "Mount": 252, "Creature": 1763, "Encounter": 104, "NPC": 2376}
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
assert manifest["families"]["RewardClaim"] == {"records": 231, "index": "content/interactions/reward_claims/index.json"}
assert "RewardClaim" in project["migrated_families"]
assert sum(path.startswith("content/interactions/reward_claims/reward-claims-") for path in paths) == 3
assert "content/interactions/reward_claims/index.json" in paths
assert manifest["families"]["StarterKit"] == {"records": 1, "index": "content/starter/index.json"}
assert "StarterKit" in project["migrated_families"]
assert "content/starter/starter-kits-00000-00000.json" in paths and "content/starter/index.json" in paths
assert "NPC" in project["migrated_families"] and "Dialogue" in project["migrated_families"] and "Service" in project["migrated_families"]
assert "NPC" not in project["next_population_families"] and "Dialogue" not in project["next_population_families"] and "Service" not in project["next_population_families"]

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
for script in ("test_reward_claim_authoring.py",):
    subprocess.run([sys.executable, script], cwd=reward_claim_tool, check=True)
# StarterKit likewise.
starter_kit_tool = ROOT / "tools" / "content-schema" / "starter-kit-authoring"
subprocess.run([sys.executable, "test_starter_kit_authoring.py"], cwd=starter_kit_tool, check=True)

print(f"PASS managed_files={len(paths)} item_shards=69")
