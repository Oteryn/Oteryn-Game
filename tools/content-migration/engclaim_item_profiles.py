"""Finite M34 physical item admissions, retaining exact canonical identities."""
import copy
import hashlib
import json
from decimal import Decimal
from pathlib import Path
import quest_reward_item_semantics as reward

ROOT = Path(__file__).resolve().parents[2]
IDS = {3359: 8000, 3276: 3500, 3389: 7000, 50335: 900, 50336: 300, 50337: 300, 50338: 300}
WIKI = {50335: 1143035, 50336: 1143038, 50337: 1143037, 50338: 1143036}
NA = {'state': 'NOT_APPLICABLE'}

def binding(item):
    return {'source_key': 'oteryn:source.tibiawiki', 'source_revision': f'fandom-r{WIKI[item]}',
            'identity_namespace': 'tibiawiki/item_id', 'external_id': str(item),
            'target': {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}', 'revision': 'definition-r1'},
            'disposition': 'EXACT'}

def apply_admissions(definitions, root=ROOT):
    # Wiki weight observations and official client flags are independently checked.
    import sys
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    raw = (root / reward.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != reward.CLIENT_SHA:
        raise ValueError('ENGCLAIM_CLIENT_PIN')
    objects = source.engine_items.load_appearance_objects(raw)
    wiki = json.loads((root / 'imports/tibiawiki/facts/items-stats.json').read_bytes())['records']
    for item, revision in WIKI.items():
        row = wiki[f'oteryn:item.tibia.i{item}']
        if row['item_id'] != item or not any(o['revision_id'] == revision and Decimal(o['fields'].get('weight', '-1')) * 100 == IDS[item] for o in row['observations']):
            raise ValueError('ENGCLAIM_WIKI_WEIGHT_PIN:' + str(item))
    for definition in definitions:
        key = definition['identity']['key']
        if key not in {f'oteryn:item.tibia.i{i}' for i in IDS}:
            continue
        item = int(key.rsplit('.i', 1)[1])
        flags = objects[item]['flags']
        if not flags.get('flags.take') or any(flags.get(k) for k in ('flags.cumulative','flags.container','flags.unmove','flags.liquidcontainer','flags.liquidpool')):
            raise ValueError('ENGCLAIM_PHYSICAL_SOURCE_CONFLICT:' + str(item))
        reward.put_group(definition, 'physical', {'movable': reward.known(True), 'pickupable': reward.known(True), 'weight': reward.known(IDS[item])})
        reward.put_group(definition, 'stack', {'stackable': reward.known(False), 'stack_max': NA})
        for group in ('container','temporal','charges','fluid'):
            if definition['semantics'].get(group, reward.UNKNOWN) not in (reward.UNKNOWN, NA):
                raise ValueError('ENGCLAIM_EXISTING_INSTANCE_OWNER:' + group)
            definition['semantics'][group] = copy.deepcopy(NA)
        definition['stack_class'] = 'NonStackable'
        definition['materializable'] = True

def admit(root, generated, profile_path, manifest_path, order, canonical):
    document = json.loads(generated[profile_path])
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    present = {r['production_definition']['production_key'] for r in document['records']}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key not in {f'oteryn:item.tibia.i{i}' for i in IDS} or key in present:
                continue
            item = int(key.rsplit('.i',1)[1])
            identity = definition['identity']
            selected = binding(item) if item in WIKI else next(b for b in bindings if b['target'] == identity)
            if not definition['materializable'] or definition['stack_class'] != 'NonStackable':
                raise ValueError('ENGCLAIM_CANONICAL_NOT_ADMITTED')
            document['records'].append({'authoring': {'item': identity}, 'semantics': copy.deepcopy(definition['semantics']),
                'attributes': {'speed_bonus': 0, 'blocks_movement': False, 'blocks_projectile': False,
                    'immovable_block_solid': False, 'has_height': False},
                'admission': {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['CharacterInventory','Ground']},
                'production_definition': {'family':'Item','production_key':key,'revision_ref':identity['revision']},
                'production_binding': selected, 'production_binding_qualification': None})
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
