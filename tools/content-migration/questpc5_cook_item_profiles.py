"""Three source-qualified Cook Item identities; physical custody only, no scripted use."""
import copy
import hashlib
import json
import sys
from decimal import Decimal

CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
FACTS = 'imports/tibiawiki/facts/items-stats.json'
OWN = 'imports/tibiawiki/source-evidence/stack-default-successor8/global-own-itemid-index.json'
SPEC = {
    44525: ('fryclops axe', 1800, 104249, 1047591, '5f58bda6575ee8208129ce38d20c7f670f2b6c5a9e62af579a762f78da8f8a66'),
    44527: ('spirit container', 600, 104250, 1047595, '635adc918782aa053c00be4992948637048a50f19a302453fa0a365c8994293e'),
    44528: ('spirit container', 600, 104252, 1046764, '87d1ac9c6799d5d3606c86c971fc8f6008b09086c856eb83eab5260319c2ec9f'),
}
GROUPS = ('charges classification container equipment fluid imbuement physical presentation '
          'protection readable_writeable skill_modifiers stack temporal trade_restrictions use_transform weapon').split()


def known(value):
    return dict(state='KNOWN', value=value)


def capture(root):
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    from engine_items import load_appearance_objects
    raw = (root / f'content/assets/files/appearances-{CLIENT_SHA}.dat').read_bytes()
    if hashlib.sha256(raw).hexdigest() != CLIENT_SHA:
        raise ValueError('COOK_ITEM_CLIENT_PIN')
    objects = load_appearance_objects(raw)
    facts = json.loads((root / FACTS).read_bytes())['records']
    mentions = json.loads((root / OWN).read_bytes())['by_own_itemid_integer_mention']
    result = []
    for item, (name, weight, page, revision, digest) in SPEC.items():
        observation, = facts[str(item)]['observations']
        if (facts[str(item)]['item_id'] != item or observation['page_id'] != page
                or observation['revision_id'] != revision or observation['content_sha256'] != digest
                or Decimal(observation['fields']['weight']) * 100 != weight
                or observation['fields']['primarytype'] != 'Quest Items'
                or not any(m['page_id'] == page and m['revision_id'] == revision
                           and m['content_sha256'] == digest and m['raw_itemid'] == str(item)
                           and m['strict_numeric_itemid'] and m['balanced'] and not m['inside_comment']
                           and not m['duplicate_itemid_parameters'] for m in mentions[str(item)])):
            raise ValueError(f'COOK_ITEM_WIKI_IDENTITY:{item}')
        obj = objects[item]
        flags = obj['flags']
        if (obj['name'] != name or not flags.get('flags.take')
                or any(flags.get(f, False) for f in ['flags.cumulative', 'flags.container',
                    'flags.unmove', 'flags.unpass', 'flags.liquidcontainer', 'flags.liquidpool'])):
            raise ValueError(f'COOK_ITEM_PHYSICAL_FLAGS:{item}')
        identity = dict(family='Item', key=f'oteryn:item.tibia.i{item}', revision='definition-r1')
        semantics = {group: dict(state='UNKNOWN') for group in GROUPS}
        semantics['physical'] = known(dict(weight=known(weight), movable=known(True), pickupable=known(True)))
        semantics['stack'] = known(dict(stackable=known(False), stack_max=dict(state='NOT_APPLICABLE')))
        semantics['container'] = dict(state='NOT_APPLICABLE')
        semantics['presentation'] = known(dict(name=known(name), description=dict(state='UNKNOWN')))
        definition = dict(identity=identity, kind='Item', materializable=True,
                          client_projection='ClientSafe', stack_class='NonStackable', semantics=semantics)
        binding = dict(source_key='oteryn:source.tibiawiki', source_revision=f'fandom-r{revision}',
                       identity_namespace='tibiawiki/item_id', external_id=str(item),
                       target=identity, disposition='EXACT')
        profile = dict(authoring=dict(item=identity), semantics=copy.deepcopy(semantics),
            attributes=dict(speed_bonus=0, blocks_movement=False, blocks_projectile=False,
                immovable_block_solid=False, field_condition=None, rune_consumption=None,
                field_replaceable=None, has_height=False),
            admission=dict(materializable=True, stack_class='NonStackable',
                legal_destinations=['CharacterInventory', 'Ground']),
            production_definition=dict(family='Item', production_key=identity['key'], revision_ref='definition-r1'),
            production_binding=binding, production_binding_qualification=None)
        result.append(dict(definition=definition, binding=binding, profile=profile,
            witness=dict(page_id=page, revision_id=revision, content_sha256=digest, weight=weight),
            assumption='Selected Oteryn portable quest material has zero intrinsic speed bonus; no scripted use, charge or lifetime is activated by this custody admission.'))
    return result

PROOF = 'tools/content-schema/native-gameplay/questpc5-cook-material-bindings.json'
PIN = 'crates/gameplay/content/src/content/native_gameplay/questpc5_cook_material_pin.rs'


def extend_reference(reference, sources, root):
    """Add only the three explicitly captured tree-first Items; retain every legacy row."""
    by_key = {row['identity']['key']: row for row in reference['records']}
    for row in capture(root):
        definition = row['definition']
        key = definition['identity']['key']
        if key in by_key:
            if by_key[key] != definition:
                raise ValueError('COOK_EXISTING_DEFINITION_CONFLICT:' + key)
        else:
            reference['records'].append(definition)
        present = [b for b in sources['source_identity_bindings'] if b['target'] == definition['identity']]
        if present and present != [row['binding']]:
            raise ValueError('COOK_EXISTING_BINDING_CONFLICT:' + key)
        if not present:
            sources['source_identity_bindings'].append(row['binding'])


def proof_outputs(root):
    rows = capture(root)
    packet = dict(schema=1,
        sources=[dict(path=path, sha256=hashlib.sha256((root / path).read_bytes()).hexdigest())
                 for path in [FACTS, OWN, f'content/assets/files/appearances-{CLIENT_SHA}.dat']],
        bindings=[row['binding'] for row in rows],
        items=[dict(identity=row['definition']['identity'], witness=row['witness'],
                    assumption=row['assumption']) for row in rows])
    raw = (json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode()
    pin = ('// Generated by tools/content-migration/questpc5_cook_item_profiles.py.\n'
           f'const SOURCE_SHA256: &str = "{hashlib.sha256(raw).hexdigest()}";\n').encode()
    return {PROOF: raw, PIN: pin}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    import qengine_reward_item_profiles as shared
    generated.update(proof_outputs(root))
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical,
                         [row['profile'] for row in capture(root)])


if __name__ == '__main__':
    import argparse
    from pathlib import Path
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    for path, raw in proof_outputs(root).items():
        target = root / path
        if args.check:
            if not target.is_file() or target.read_bytes() != raw:
                raise ValueError('COOK_MATERIAL_PROOF_DRIFT:' + path)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
