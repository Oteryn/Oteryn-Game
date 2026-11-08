"""Exact native-goal corpse closure, reusing the existing Ground-only corpse owner.

Pinned XML supplies capacities, durations, decay edges and explicit late-stage
weights. Missing weights use the existing non-pickupable Game corpse policy0.
WorldObject movement/corpse identity and canonical known facts remain binding.
"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
GOALS = 'tools/content-schema/quest-authoring/samples/native-kill-goals.json'
CREATURES = 'content/creatures/definitions/spell-native-profiles.json'
GROUPS = ('physical', 'stack', 'container', 'temporal')


def module(name, root):
    spec = importlib.util.spec_from_file_location(name, root / 'tools/content-schema/native-gameplay' / (name + '.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def policies(root=ROOT):
    creatures = {o['creature'] for g in json.loads((root / GOALS).read_bytes())['goals'] for o in g['origins']}
    raw = (root / CREATURES).read_bytes()
    manifest = json.loads((root / 'content/spells.manifest.json').read_bytes())
    if hashlib.sha256(raw).hexdigest() != manifest['creature_profiles']['sha256']:
        raise ValueError('NATIVE_GOAL_CREATURE_PROFILE_PIN')
    roots = set()
    for creature in creatures:
        rows = [r['profile'] for r in json.loads(raw)['records'] if r['profile']['target']['key'] == creature]
        if len(rows) != 1:
            raise ValueError('NATIVE_GOAL_CREATURE_IDENTITY')
        corpse = rows[0]['data']['profile']['details']['corpse_item']
        if corpse['family'] != 'Item' or corpse['revision'] != 'definition-r1' or not corpse['key'].startswith('oteryn:item.tibia.i'):
            raise ValueError('NATIVE_GOAL_CORPSE_IDENTITY')
        roots.add(int(corpse['key'].rsplit('.i', 1)[1]))
    if not 1 <= len(roots) <= 16:
        raise ValueError('NATIVE_GOAL_CORPSE_BOUND')
    source = module('encounter_corpse_items', root)
    result = source.source_policies(root, tuple(sorted(roots)), explicit_weights=True)
    nodes = {int(n.get('id')): n for n in ET.fromstring((root / source.XML).read_bytes()) if n.get('id')}
    weights = {}
    for item in result:
        values = [a.get('value') for a in nodes[item] if a.get('key', '').lower() == 'weight']
        if len(values) > 1:
            raise ValueError('NATIVE_GOAL_CORPSE_WEIGHT_DUPLICATE')
        weight = int(values[0]) if values else 0
        if not 0 <= weight <= 2**63 - 1:
            raise ValueError('NATIVE_GOAL_CORPSE_WEIGHT_BOUND')
        weights[item] = weight
    return result, weights


def compatible(accepted, observed):
    if isinstance(accepted, dict):
        return accepted.get('state') == 'UNKNOWN' or isinstance(observed, dict) and all(
            k in observed and compatible(v, observed[k]) for k, v in accepted.items())
    return accepted == observed


def apply_admissions(definitions, root=ROOT):
    portable = module('questpc2_portable_corpse_facts', root).profiles(root)
    definitions = list(definitions)
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('NATIVE_GOAL_CORPSE_DUPLICATE_ITEM')
    values, weights = policies(root)
    wanted = {f'oteryn:item.tibia.i{i}' for i in values}
    objects = {}
    for path in (root / 'content/world/objects').glob('objects-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            pointer = row.get('provenance', {}).get('item_pointer')
            if pointer and pointer['key'] in wanted:
                if pointer['key'] in objects:
                    raise ValueError('NATIVE_GOAL_CORPSE_DUPLICATE_OBJECT')
                objects[pointer['key']] = row
    builder = module('primal_corpse_items', root)
    replacements = []
    for item, (capacity, duration, target) in values.items():
        key = f'oteryn:item.tibia.i{item}'
        definition, world = indexed[key], objects[key]
        movable = world['movable']
        if (world['provenance']['item_pointer'] != definition['identity']
                or world['kind'] != {'state': 'KNOWN', 'value': 'corpse'}
                or world['corpse']['corpse'] != {'state': 'KNOWN', 'value': True}
                or world['corpse']['player_corpse'] != {'state': 'KNOWN', 'value': False}
                or movable.get('state') != 'KNOWN' or type(movable.get('value')) is not bool):
            raise ValueError('NATIVE_GOAL_CORPSE_WORLDOBJECT_PROOF')
        observed = builder.corpse_definition(definition, capacity, duration, target, movable['value'], weights[item])
        if key in portable:
            for field in ('movable', 'weight'):
                if (observed['semantics']['physical']['value'][field]
                        != portable[key]['semantics']['physical']['value'][field]):
                    raise ValueError('NATIVE_GOAL_CORPSE_PORTABLE_PHYSICAL_CONFLICT')
            for group in ('physical', 'container'):
                observed['semantics'][group] = copy.deepcopy(portable[key]['semantics'][group])
        replacement = copy.deepcopy(definition)
        accepted_semantics = definition.get('semantics', {})
        replacement.setdefault('semantics', copy.deepcopy(observed['semantics']))
        for group in GROUPS:
            if not compatible(accepted_semantics.get(group, {'state': 'UNKNOWN'}), observed['semantics'][group]):
                raise ValueError('NATIVE_GOAL_CORPSE_KNOWN_FACT_CONFLICT')
            replacement['semantics'][group] = observed['semantics'][group]
        replacement['materializable'] = True
        replacement['stack_class'] = 'NonStackable'
        replacements.append((definition, replacement))
    for definition, replacement in replacements:
        definition.clear()
        definition.update(replacement)


def admit_native(root, generated, profile_path, manifest_path, order, canonical):
    portable = module('questpc2_portable_corpse_facts', root).profiles(root)
    values, _ = policies(root)
    wanted = {f'oteryn:item.tibia.i{i}' for i in values}
    definitions = {}
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in wanted:
                if definition['identity']['key'] in definitions:
                    raise ValueError('NATIVE_GOAL_CORPSE_DUPLICATE_ITEM')
                definitions[definition['identity']['key']] = definition
    expected = copy.deepcopy(list(definitions.values()))
    apply_admissions(expected, root)
    if {d['identity']['key']: d for d in expected} != definitions:
        raise ValueError('NATIVE_GOAL_CORPSE_CANONICAL_ADMISSION_REQUIRED')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    document = json.loads(generated[profile_path])
    for key in sorted(wanted):
        definition = definitions[key]
        exact = [b for b in bindings if b['target'] == definition['identity'] and b['disposition'] == 'EXACT']
        if len(exact) != 1:
            raise ValueError('NATIVE_GOAL_CORPSE_BINDING_PROOF')
        present = [r for r in document['records'] if r['production_definition']['production_key'] == key]
        if len(present) > 1:
            raise ValueError('NATIVE_GOAL_CORPSE_DUPLICATE_PROFILE')
        if key in portable and not present:
            raise ValueError('NATIVE_GOAL_CORPSE_PORTABLE_PROFILE_REQUIRED')
        if present:
            record = present[0]
            if record['production_binding'] != exact[0]:
                raise ValueError('NATIVE_GOAL_CORPSE_PROFILE_BINDING')
            if key in portable:
                source = portable[key]
                if (record['admission'] != source['admission']
                        or any(not compatible(record['semantics'][group], source['semantics'][group])
                               for group in GROUPS)):
                    raise ValueError('NATIVE_GOAL_CORPSE_PORTABLE_SOURCE_CONFLICT')
                continue
            for group in GROUPS:
                if not compatible(record['semantics'][group], definition['semantics'][group]):
                    raise ValueError('NATIVE_GOAL_CORPSE_NATIVE_FACT_CONFLICT')
        else:
            record = dict(authoring=dict(item=definition['identity']), semantics=copy.deepcopy(definition['semantics']),
                          attributes=dict(speed_bonus=0), production_definition=dict(family='Item', production_key=key, revision_ref=definition['identity']['revision']),
                          production_binding=exact[0], production_binding_qualification=None)
            document['records'].append(record)
        for group in GROUPS:
            record['semantics'][group] = copy.deepcopy(definition['semantics'][group])
        record['admission'] = dict(materializable=True, stack_class='NonStackable', legal_destinations=['Ground'])
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
