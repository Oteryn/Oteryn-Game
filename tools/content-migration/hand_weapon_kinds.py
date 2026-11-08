"""Complete unknown native hand kinds from agreeing taxonomy and pinned XML.

No identity, admission, numeric combat stat or known field is replaced. In
particular current dwarven-shield defence34 is retained over donor defence26.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
XML = 'imports/canary/items-xml/items.xml'
XML_SHA = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
PROOF = 'tools/content-schema/native-gameplay/hand-weapon-kind-facts.json'
BINDINGS = 'imports/crystalserver/bindings/items.json'
BINDINGS_SHA = '1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a'
FIELDS = ('weapon_type','attack','defense','extra_defense','elemental','range','hit_chance','max_hit_chance','ammunition')


def capture(root, profiles):
    raw = (root / XML).read_bytes()
    bound = (root / BINDINGS).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA or hashlib.sha256(bound).hexdigest() != BINDINGS_SHA:
        raise ValueError('HAND_KIND_SOURCE_PIN')
    bindings = {r['external_id']:r for r in json.loads(bound)['bindings']}
    nodes = {int(n.get('id')):n for n in ET.fromstring(raw) if n.get('id')}
    lines = {}
    for line_number, line in enumerate(raw.splitlines(), 1):
        match = re.search(rb'<item\b[^>]*\bid="([0-9]+)"', line)
        if match:
            lines[int(match.group(1))] = line_number
    records = []
    for record in profiles['records']:
        if record.get('authoring',{}).get('taxonomy',{}).get('primary') != 'Shields':
            continue
        key = record['production_definition']['production_key']
        number = key.removeprefix('oteryn:item.tibia.i')
        binding = bindings.get(number)
        if not binding or binding['disposition'] != 'EXACT' or binding['target'] != {'family':'Item','key':key,'revision':'definition-r1'}:
            raise ValueError('HAND_KIND_IDENTITY_BINDING')
        node = nodes.get(int(number))
        attrs = {} if node is None else {a.get('key','').lower():a.get('value') for a in node.findall('attribute')}
        if attrs.get('weapontype') != 'shield':
            raise ValueError('HAND_KIND_TAXONOMY_XML_DISAGREEMENT:'+key)
        records.append({'key':key,'source_id':int(number),'source_line':lines[int(number)],
                        'taxonomy':'Shields','weapon_type':'SHIELD'})
    return {'schema':'OTERYN_NATIVE_HAND_KIND_FACTS/v1','source':XML,'source_sha256':XML_SHA,
            'binding_sha256':BINDINGS_SHA,'records':sorted(records,key=lambda r:r['source_id'])}


def complete(profiles, facts):
    result = copy.deepcopy(profiles)
    by_key = {r['production_definition']['production_key']:r for r in result['records']}
    for fact in facts['records']:
        record = by_key[fact['key']]
        weapon = record['semantics']['weapon']
        if weapon['state'] == 'UNKNOWN':
            weapon = {'state':'KNOWN','value':{field:{'state':'UNKNOWN'} for field in FIELDS}}
            record['semantics']['weapon'] = weapon
        if weapon['state'] != 'KNOWN':
            raise ValueError('HAND_KIND_CONTRADICTORY_WEAPON_STATE')
        kind = weapon['value']['weapon_type']
        wanted = {'state':'KNOWN','value':fact['weapon_type']}
        if kind['state'] == 'UNKNOWN':
            weapon['value']['weapon_type'] = wanted
        elif kind != wanted:
            raise ValueError('HAND_KIND_KNOWN_CONFLICT')
    return result


def apply_native(root, generated, canonical):
    profiles = json.loads(generated[PROFILE])
    facts = capture(root,profiles)
    if json.loads((root / PROOF).read_bytes()) != facts:
        raise ValueError('HAND_KIND_CAPTURE_DRIFT')
    raw = canonical(complete(profiles,facts))
    generated[PROFILE] = raw
    manifest = json.loads(generated[MANIFEST])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[MANIFEST] = canonical(manifest)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture',action='store_true')
    args=parser.parse_args()
    facts=capture(ROOT,json.loads((ROOT / PROFILE).read_bytes()))
    raw=(json.dumps(facts,indent=2)+'\n').encode()
    if args.capture:
        (ROOT / PROOF).write_bytes(raw)
    elif (ROOT / PROOF).read_bytes() != raw:
        raise SystemExit('HAND_KIND_CAPTURE_DRIFT')
    print(f"hand kinds: {len(facts['records'])} pinned source/taxonomy agreements")
