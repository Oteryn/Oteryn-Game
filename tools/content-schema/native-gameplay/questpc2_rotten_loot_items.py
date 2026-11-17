"""Six exact Rotten Blood Item admissions from the independently replayed fact packet."""
import copy, hashlib, json, sys
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
    # Global client weights describe the canonical reference. These two admitted
    # native leaves retain the independently pinned runtime loader-default zero.
    # Accept only that exact client-weight overlay over the complete source body.
    from import_cyclopedia_global import load_weights
    weights = load_weights(root)
    planned = dict(generated)
    shared.admit_native(root, planned, profile_path, manifest_path, order, canonical,
                        item_ids=source.IDS, qualified_profiles=profiles)
    document = json.loads(planned[profile_path])
    for record in document['records']:
        key = record['production_definition']['production_key']
        if key not in {'oteryn:item.tibia.i43899', 'oteryn:item.tibia.i43900'}:
            continue
        physical = profiles[key]['semantics']['physical']
        if physical['value']['weight'] != {'state': 'KNOWN', 'value': 0}:
            raise ValueError('ROTTEN_ITEM_NATIVE_SOURCE_WEIGHT_CHANGED:' + key)
        client = copy.deepcopy(physical)
        client['value']['weight'] = {'state': 'KNOWN', 'value': weights[int(key.rsplit('.i', 1)[1])]}
        if record['semantics']['physical'] not in (physical, client):
            raise ValueError('ROTTEN_ITEM_NATIVE_PHYSICAL_SOURCE_CONFLICT:' + key)
        record['semantics']['physical'] = copy.deepcopy(physical)
    planned[profile_path] = canonical(document)
    manifest = json.loads(planned[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(planned[profile_path]).hexdigest()
    planned[manifest_path] = canonical(manifest)
    generated.update(planned)
