"""Complete canonical custody for source encounter loot with known physical facts.

Canary04b83 angry_demon.lua names this closure. The pinned client and existing
source importer own stack/container facts; canonical physical/combat facts win.
This does not change loot probabilities, instance charges or use effects.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted
from build_spell_item_identities import historical_binding_bytes

IDS = {3060, 3063, 3284, 3306, 3364, 3366, 3414, 5954, 7368, 7382, 7393}
XML = 'imports/canary/items-xml/items.xml'
XML_SHA = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
CLIENT = f'content/assets/files/appearances-{CLIENT_SHA}.dat'
BINDINGS = 'imports/crystalserver/bindings/items.json'
BINDINGS_SHA = '1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a'



def client_only_custody_supported(obj):
    flags = obj['flags']
    return (flags.get('flags.take') is True
            and not flags.get('flags.container', False)
            and not flags.get('flags.cumulative', False))


def facts(root=ROOT, ids=None, *, allow_containers=False, xml_override=None):
    ids = IDS if ids is None else ids
    raw = {}
    for path, digest in [(XML, XML_SHA), (CLIENT, CLIENT_SHA), (BINDINGS, BINDINGS_SHA)]:
        raw[path] = (root / path).read_bytes()
        if path == BINDINGS:
            raw[path] = historical_binding_bytes(raw[path], BINDINGS_SHA)
        if hashlib.sha256(raw[path]).hexdigest() != digest:
            raise ValueError('ENCOUNTER_LOOT_SOURCE_PIN')
    if xml_override is not None:
        # Exact retained Crystal summer source for the newer portable trophy.
        # The ordinary Canary pin and all other source guards above stay mandatory.
        if set(ids) != {44158} or hashlib.sha256(xml_override).hexdigest() != '13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece':
            raise ValueError('ENCOUNTER_LOOT_ALTERNATE_XML_SCOPE_OR_PIN')
        raw[XML] = xml_override
    xml = source.xml_index(raw[XML])
    objects = source.engine_items.load_appearance_objects(raw[CLIENT])
    physical = {r['server_item_id']: r for r in source.policies(raw[CLIENT], raw[XML], ids)}
    bindings = source.binding_index(raw[BINDINGS], expected_sha=BINDINGS_SHA)
    result = {}
    for item in sorted(ids):
        profile = source.profile(item, xml[item], objects[item], physical[item], bindings, None)
        bound = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                      'records': [profile]}, bindings, {item})['records'][0]
        if bound['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']:
            raise ValueError('ENCOUNTER_LOOT_NOT_PORTABLE')
        if not allow_containers and bound['semantics']['container'] != {'state': 'NOT_APPLICABLE'}:
            raise ValueError('ENCOUNTER_LOOT_CONTAINER_OWNER_REQUIRED')
        result[bound['production_definition']['production_key']] = bound
    return result


def apply_admissions(definitions, root=ROOT, ids=None, *, source_core=False, xml_override=None):
    definitions = list(definitions)
    indexed = {d['identity']['key']: d for d in definitions}
    if len(indexed) != len(definitions):
        raise ValueError('ENCOUNTER_LOOT_DUPLICATE_IDENTITY')
    planned = []
    for key, profile in facts(root, ids, allow_containers=source_core, xml_override=xml_override).items():
        definition = indexed[key]
        physical = (accepted.accepted_first(definition['semantics']['physical'], profile['semantics']['physical'])
                    if source_core else definition['semantics']['physical'])
        if (physical.get('state') != 'KNOWN'
                or physical['value']['weight'].get('state') != 'KNOWN'
                or any(physical['value'][field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))):
            raise ValueError('ENCOUNTER_LOOT_EXISTING_PHYSICAL_REQUIRED')
        updated = copy.deepcopy(definition)
        if source_core:
            updated['semantics']['physical'] = physical
        groups = ('stack', 'container', 'charges', 'temporal') if source_core else ('stack', 'container')
        for group in groups:
            if accepted.known_conflict(definition['semantics'][group], profile['semantics'][group]):
                raise ValueError('ENCOUNTER_LOOT_CORE_CONFLICT')
            updated['semantics'][group] = accepted.accepted_first(
                definition['semantics'][group], profile['semantics'][group])
        stack_class = profile['admission']['stack_class']
        if definition['stack_class'] not in ('Unknown', stack_class):
            raise ValueError('ENCOUNTER_LOOT_STACK_CONFLICT')
        updated['stack_class'] = stack_class
        updated['materializable'] = True
        planned.append((definition, updated))
    for definition, updated in planned:
        definition.clear()
        definition.update(updated)


def outputs(root=ROOT):
    shards = {p: json.loads(p.read_bytes())
              for p in (root / 'content/items/definitions').glob('items-*.json')}
    owned = {f'oteryn:item.tibia.i{i}' for i in IDS}
    apply_admissions([r['definition'] for shard in shards.values() for r in shard['records']], root)
    return {p: (json.dumps(shard, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            for p, shard in shards.items()
            if any(r['definition']['identity']['key'] in owned for r in shard['records'])}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, raw in outputs().items():
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f'stale encounter loot Item: {path.relative_to(ROOT)}')
        else:
            path.write_bytes(raw)
