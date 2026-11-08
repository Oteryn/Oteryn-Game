"""Exact Targuna turnips custody; quest-guarded Ram taming remains a separate operation."""
import copy
import hashlib
import json
import qengine_reward_item_profiles as shared

KEY = 'oteryn:item.tibia.i53073'
PROOF = 'tools/content-schema/native-gameplay/questpc5-targuna-turnips.json'


def capture(root=shared.ROOT):
    raw = (root / shared.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != shared.CLIENT_SHA:
        raise ValueError('TARGUNA_TURNIP_CLIENT_PIN')
    obj = shared.source.engine_items.load_appearance_objects(raw)[53073]
    flags = obj['flags']
    if (obj.get('name') != 'bunch of turnips' or flags.get('flags.take') is not True
            or any(flags.get(f, False) for f in ('flags.container', 'flags.cumulative',
                'flags.unmove', 'flags.liquidcontainer', 'flags.liquidpool'))):
        raise ValueError('TARGUNA_TURNIP_CUSTODY')
    definition, = [r['definition'] for path in (root / 'content/items/definitions').glob('items-*.json')
                   for r in json.loads(path.read_bytes())['records'] if r['definition']['identity']['key'] == KEY]
    semantics = copy.deepcopy(definition['semantics'])
    if semantics['physical']['value']['weight'] != dict(state='KNOWN', value=200):
        raise ValueError('TARGUNA_CANONICAL_WEIGHT')
    binding = shared.source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())['53073']
    if binding['target'] != definition['identity'] or binding['disposition'] != 'EXACT':
        raise ValueError('TARGUNA_EXACT_BINDING')
    observed = dict(stack=dict(state='KNOWN', value=dict(stackable=dict(state='KNOWN', value=False), stack_max=dict(state='NOT_APPLICABLE'))), container=dict(state='NOT_APPLICABLE'))
    for group, value in observed.items():
        if shared.accepted.known_conflict(semantics[group], value):
            raise ValueError('TARGUNA_CANONICAL_CONFLICT:' + group)
        semantics[group] = value
    return dict(authoring=dict(item=definition['identity']), semantics=semantics,
        attributes=dict(speed_bonus=0, blocks_movement=False, blocks_projectile=False,
            immovable_block_solid=False, field_condition=None, rune_consumption=None,
            field_replaceable=None, has_height=False),
        admission=dict(materializable=True, stack_class='NonStackable', legal_destinations=['CharacterInventory', 'Ground']),
        production_definition=dict(family='Item', production_key=KEY, revision_ref='definition-r1'),
        production_binding=binding, production_binding_qualification=None)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    row = capture(root)
    generated[PROOF] = (json.dumps(dict(schema=1, records=[row], client_sha256=shared.CLIENT_SHA,
        scope='Physical custody only; canonical Known weight/EXACT binding retained, charges/temporal Unknown preserved; no mount or quest progress.'), indent=2)+'\n').encode()
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, [row])
