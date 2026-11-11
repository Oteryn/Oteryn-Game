"""Six exact Rotten Blood Item admissions from the independently replayed fact packet."""
import copy, json, sys
from pathlib import Path
import native_goal_loot_items as shared
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'quest-authoring'))
import questpc2_rotten_item_facts as source


def selected():
    return source.IDS


def apply_admissions(definitions, root):
    facts = source.capture(root)
    if facts != json.loads((root / source.OUTPUT).read_bytes()):
        raise ValueError('ROTTEN_ITEM_SOURCE_FACTS_STALE')
    accepted = shared.importer(root).accepted
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('ROTTEN_ITEM_DUPLICATE_CANONICAL_IDENTITY')
    planned = []
    for row in facts['items']:
        key = row['production_definition']['production_key']
        definition = indexed[key]
        if definition['materializable']:
            continue
        updated = copy.deepcopy(definition)
        for group in source.GROUPS:
            old, observed = definition['semantics'][group], row['semantics'][group]
            if group != 'physical' and accepted.known_conflict(old, observed):
                raise ValueError('ROTTEN_ITEM_ACCEPTED_CORE_CONFLICT:' + key)
            updated['semantics'][group] = accepted.accepted_first(old, observed)
        physical = updated['semantics']['physical']
        if (physical['state'] != 'KNOWN' or physical['value']['weight']['state'] != 'KNOWN'
                or any(physical['value'][f] != {'state': 'KNOWN', 'value': True} for f in ('movable', 'pickupable'))):
            raise ValueError('ROTTEN_ITEM_PORTABLE_PHYSICAL_REQUIRED:' + key)
        stack = row['candidate_admission']['stack_class']
        if definition['stack_class'] not in ('Unknown', stack):
            raise ValueError('ROTTEN_ITEM_STACK_CONFLICT:' + key)
        updated.update(materializable=True, stack_class=stack)
        planned.append((definition, updated))
    for definition, updated in planned:
        definition.clear()
        definition.update(updated)


def admit_native(root, generated, profile_path, manifest_path, order, canonical):
    profiles = {r['production_definition']['production_key']: r for r in source.capture(root, profiles_only=True)}
    shared.admit_native(root, generated, profile_path, manifest_path, order, canonical,
                        item_ids=source.IDS, qualified_profiles=profiles)
