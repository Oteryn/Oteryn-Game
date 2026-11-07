#!/usr/bin/env python3
"""Capture missing Demon and Devil Helmet encounter Items with the existing pinned native importer."""
import argparse
import hashlib
import json
import sys
from pathlib import Path
from build_tool_item_profiles import build as source_profiles
import build_source_item_profiles as native
from build_spell_item_identities import bind_profiles, binding_index, BINDINGS, INDEPENDENT_CRYSTAL_PIN, INDEPENDENT_CRYSTAL_XML_SHA
ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'tools/content-schema/native-gameplay/quest-encounter-item-profiles.json'
IDS = {
    3060, 3061, 3063, 3071, 3275, 3306, 3322, 3359, 3364, 3366, 3413, 3414, 3416,
    4013, 4025, 4026, 4027, 4057, 4058, 4097, 4098, 4099, 5877, 5920, 5954,
    5973, 5983, 5995, 7368, 7382, 7393, 7401, 7430, 11457, 11472, 11482, 44158,
}

CURRENT_CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'

def current_trophy(directory):
    """Exact newer XML membership plus current client and accepted Wiki facts."""
    raw = (directory / 'crystal-current-items.xml').read_bytes()
    if hashlib.sha256(raw).hexdigest() != INDEPENDENT_CRYSTAL_XML_SHA:
        raise ValueError('QUEST_ENCOUNTER_CURRENT_XML_PIN')
    appearance = ROOT / f'content/assets/files/appearances-{CURRENT_CLIENT_SHA}.dat'
    client = appearance.read_bytes()
    if hashlib.sha256(client).hexdigest() != CURRENT_CLIENT_SHA:
        raise ValueError('QUEST_ENCOUNTER_CURRENT_CLIENT_PIN')
    index = native.xml_index(raw)
    objects = native.engine_items.load_appearance_objects(client)
    node, obj = index[44158], objects[44158]
    flags = obj['flags']
    if (node.get('name') != 'dragon trophy' or obj.get('name') != 'dragon trophy'
            or node.findall('attribute') or flags.get('flags.take') is not True
            or flags.get('flags.hang') is not True
            or any(flags.get(key,False) for key in ('flags.unmove','flags.container','flags.cumulative','flags.clothes'))):
        raise ValueError('QUEST_ENCOUNTER_CURRENT_TROPHY_FACTS')
    physical = next(r for r in native.policies(client,raw,{44158}) if r['server_item_id']==44158)
    bindings = binding_index(BINDINGS.read_bytes())
    row = native.profile(44158,node,obj,physical,bindings,None)
    legacy = next(d for d in json.loads((ROOT/'content/world/definitions/reference.json').read_bytes())['records']
                  if d['identity']['key']=='oteryn:item.tibia.i44158')
    if legacy['semantics']['physical']['value']['weight'] != {'state':'KNOWN','value':500}:
        raise ValueError('QUEST_ENCOUNTER_CURRENT_TROPHY_WEIGHT')
    sys.path.insert(0,str(ROOT/'tools/content-migration'))
    from task_board_item_profiles import retain_known
    # Explicit accepted Wiki facts win over an older loader's absent-weight default.
    row['semantics'] = retain_known(row['semantics'],legacy['semantics'])
    bound = bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[row]},bindings,{44158},
                          independent_crystal_source=(INDEPENDENT_CRYSTAL_PIN,raw))
    sources = [{'path':'crystal-current/data/items/items.xml','revision':INDEPENDENT_CRYSTAL_PIN,'sha256':INDEPENDENT_CRYSTAL_XML_SHA},
               {'path':'official/client-assets/15.30/'+appearance.name,'sha256':CURRENT_CLIENT_SHA}]
    evidence = {'item':44158,'xml_revision':INDEPENDENT_CRYSTAL_PIN,'client_sha256':CURRENT_CLIENT_SHA,
                'identity':'Exact existing binding; XML and current client both name dragon trophy; take/hang true, non-stackable, no container or decay.',
                'weight_precedence':'Accepted canonical Known500 over absent XML loader default; all previous Known semantic facts retained.'}
    return bound['records'][0],sources,evidence

def build(directory):
    document = source_profiles(directory, IDS - {44158})
    trophy,sources,evidence = current_trophy(directory)
    document['records'].append(trophy)
    document['records'].sort(key=lambda r:int(r['production_binding']['external_id']))
    document['source_files'].extend(sources)
    document['source_overrides'] = [evidence]
    document['schema'] = 'OTERYN_NATIVE_QUEST_ENCOUNTER_ITEM_PROFILES/v1'
    document['producer'] = 'tools/content-schema/native-gameplay/build_quest_encounter_profiles.py'
    return document

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_directory', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if OUTPUT.read_text() != raw:
            raise SystemExit('QUEST_ENCOUNTER_PROFILE_DRIFT')
    else:
        OUTPUT.write_text(raw)
