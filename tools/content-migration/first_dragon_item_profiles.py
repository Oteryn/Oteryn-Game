"""M18: native collision for the donor exit carpet, through the normal Item importer.

Crystal201189f data/items/items.xml:35502-35504 names ornate carpet17400;
its appearances.dat clip/unmove flags and items.cpp:233-244 establish the
nonblocking, immovable Ground-only object. No identity or inventory capability
is invented. The already accepted canonical presentation wins.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from grizzly_shop_profiles import accepted_first, known_conflict, SOURCE_HASHES, REVISION

PATH = 'tools/content-schema/native-gameplay/first-dragon-item-profiles.json'
KEY = 'oteryn:item.tibia.i17400'
COHORT = 'M18-FIRST-DRAGON'


def encode(value):
    return (json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode()


def capture(crystal, root=ROOT):
    blobs = {name:(crystal/'data/items'/name).read_bytes() for name in SOURCE_HASHES}
    for name, raw in blobs.items():
        if hashlib.sha256(raw).hexdigest() != SOURCE_HASHES[name]:
            raise ValueError('FIRST_DRAGON_ITEM_SOURCE_DRIFT:'+name)
    xml = source.xml_index(blobs['items.xml'])
    objects = source.engine_items.load_appearance_objects(blobs['appearances.dat'])
    physical, = source.policies(blobs['appearances.dat'],blobs['items.xml'],{17400})
    if (source.attrs(xml[17400]) != {'primarytype':'floor decorations'}
            or objects[17400]['flags'] != {'flags.clip':True,'flags.unmove':True}
            or physical['block_solid'] or physical['block_projectile']):
        raise ValueError('FIRST_DRAGON_EXIT_CARPET_PHYSICAL_DRIFT')
    bindings = source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    binding = bindings['17400']
    if binding['target'] != {'family':'Item','key':KEY,'revision':'definition-r1'}:
        raise ValueError('FIRST_DRAGON_EXIT_CARPET_IDENTITY_DRIFT')
    definitions = [r['definition'] for p in (root/'content/items/definitions').glob('items-*.json')
                   for r in json.loads(p.read_bytes())['records'] if r['definition']['identity']['key']==KEY]
    definition, = definitions
    row = source.profile(17400,xml[17400],objects[17400],physical,bindings,None)
    for group in ('physical','stack','container','temporal'):
        if known_conflict(definition['semantics'][group],row['semantics'][group]):
            raise ValueError('FIRST_DRAGON_ACCEPTED_ITEM_FACT_CONFLICT:'+group)
    row['semantics'] = accepted_first(definition['semantics'],row['semantics'])
    row.update(production_binding=copy.deepcopy(binding),production_binding_qualification=None,
               production_definition={'family':'Item','production_key':KEY,'revision_ref':'definition-r1'})
    row['attributes'].update(always_on_top=True,always_on_top_order=1)
    records = [row]
    # Crystal201189f data/items/items.xml:48202-48217 supplies the exact
    # First Dragon corpse chain; items.cpp owns the physical/temporal defaults.
    corpse_ids = {25065, 25066, 25067}
    corpse_physical = {r['server_item_id']:r for r in source.policies(blobs['appearances.dat'],blobs['items.xml'],corpse_ids)}
    all_definitions = {r['definition']['identity']['key']:r['definition']
                       for p in (root/'content/items/definitions').glob('items-*.json')
                       for r in json.loads(p.read_bytes())['records']}
    for item in sorted(corpse_ids):
        binding = bindings[str(item)]
        key = f'oteryn:item.tibia.i{item}'
        if binding['target']['key'] != key:
            raise ValueError('FIRST_DRAGON_CORPSE_IDENTITY')
        corpse = source.profile(item,xml[item],objects[item],corpse_physical[item],bindings,None)
        if corpse['admission']['legal_destinations'] != ['Ground']:
            raise ValueError('FIRST_DRAGON_CORPSE_CUSTODY')
        definition = all_definitions[key]
        for group in ('physical','stack','container','temporal'):
            if group in definition.get('semantics', {}) and known_conflict(definition['semantics'][group],corpse['semantics'][group]):
                raise ValueError('FIRST_DRAGON_CORPSE_ACCEPTED_CONFLICT:'+group)
        corpse['semantics'] = accepted_first(definition.get('semantics', {}),corpse['semantics'])
        corpse.update(production_binding=copy.deepcopy(binding),production_binding_qualification=None,
                      production_definition={'family':'Item','production_key':key,'revision_ref':binding['target']['revision']})
        records.append(corpse)
    return {'schema':'OTERYN_FIRST_DRAGON_ITEM_PROFILES/v1' ,'source_revision':REVISION,
            'sources': [{'path':'data/items/'+p,'sha256':SOURCE_HASHES[p]} for p in sorted(blobs)],
            'item_ids':[17400,25065,25066,25067], 'source_flags':objects[17400]['flags'],'records':records}


def load(root=ROOT):
    doc = json.loads((root/PATH).read_bytes())
    if (doc['schema']!='OTERYN_FIRST_DRAGON_ITEM_PROFILES/v1' or doc['source_revision']!=REVISION
            or doc['item_ids'] != [17400,25065,25066,25067] or len(doc['records'])!=4
            or doc['records'][0]['production_definition']['production_key']!=KEY):
        raise ValueError('FIRST_DRAGON_ITEM_CAPTURE_SET')
    return doc


def admit(root, generated, profile_path, manifest_path, order, canonical):
    document = json.loads(generated[profile_path])
    for row in load(root)['records']:
        key = row['production_definition']['production_key']
        matches = [r for r in document['records'] if r['production_definition']['production_key']==key]
        if matches:
            current, = matches
            if current != row:
                raise ValueError('FIRST_DRAGON_EXISTING_NATIVE_ITEM_CONFLICT:'+key)
        else:
            document['records'].append(copy.deepcopy(row))
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def cohort_receipt(text, root=ROOT):
    doc = load(root)
    if len(doc['records']) != 4: raise ValueError('FIRST_DRAGON_COHORT_SOURCE_SET')
    entry = f'    ("{COHORT}", 4),\n'
    pattern = r'(const PROFILE_ADDITIONS: &\[\(&str, usize\)\] = &\[\n)(.*?)(^\];)'
    match = re.search(pattern,text,re.M|re.S)
    if not match: raise ValueError('FIRST_DRAGON_COHORT_LEDGER')
    if COHORT in match[2]:
        return re.sub(r'    \("M18-FIRST-DRAGON", [0-9]+\),\n', lambda _: entry, text)
    return text[:match.start(3)] + entry + text[match.start(3):]


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--crystal',type=Path,required=True)
    p.add_argument('--check',action='store_true')
    a=p.parse_args();raw=encode(capture(a.crystal))
    if a.check:
        if (ROOT/PATH).read_bytes()!=raw: raise SystemExit('FIRST_DRAGON_ITEM_CAPTURE_DRIFT')
    else: (ROOT/PATH).write_bytes(raw)
