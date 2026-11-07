"""Native World-change operands, admitted by the existing source Item importer.

Sandfish13992: Crystal201189f items.xml:32101-32104 weight660 and food;
foods.lua:97,150 gives55*12=660 regenerationseconds. All portability, stack,
container and collision facts come from the same pinned appearances.dat.
The canonical Item already admits its physical/consumption facts; this module
closes the missing NativeGameplay Item provider membership only.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/content-schema/native-gameplay'))
import build_source_item_profiles as native

REVISION='201189f871d23597b82caf88fd611454a2ddefb8'
PATH='tools/content-schema/native-gameplay/wvar-item-profiles.json'
SOURCES=(('data/items/items.xml','8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'),
         ('data/items/appearances.dat','17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'),
         ('data/scripts/actions/items/foods.lua','6023845b3f982b4a7cc9cf3e43bb8d33f467e3e10f9bdf3f0713aeae88de902f'))
KEY='oteryn:item.tibia.i13992'
CANONICAL='content/items/definitions/items-03500-03999.json'


def read_archive(archive):
    with tarfile.open(archive) as donor:
        rows=[]
        for path,_ in SOURCES:
            matches=[m for m in donor.getmembers() if m.name.endswith('/'+path)]
            if len(matches)!=1 or REVISION not in matches[0].name:
                raise ValueError('WVAR_ITEM_SOURCE_REVISION')
            rows.append((path,donor.extractfile(matches[0]).read()))
        return rows


def build_sources(sources):
    if [(path,hashlib.sha256(raw).hexdigest()) for path,raw in sources]!=list(SOURCES):
        raise ValueError('WVAR_ITEM_SOURCE_HASH')
    xml,appearances,foods=[raw for _,raw in sources]
    index=native.xml_index(xml)
    row=index[13992]
    if native.attrs(row)!={'primarytype':'food','weight':'660'} or row.get('name')!='sandfish':
        raise ValueError('WVAR_SANDFISH_XML_FACTS')
    if '[13992] = { 55, "Munch." }' not in foods.decode() or 'player:feed(itemFood[1] * 12)' not in foods.decode():
        raise ValueError('WVAR_SANDFISH_FOOD_FACTS')
    bindings=native.binding_index(native.BINDINGS.read_bytes())
    objects=native.engine_items.load_appearance_objects(appearances)
    physical={r['server_item_id']:r for r in native.policies(appearances,xml,{13992})}
    record=native.profile(13992,row,objects[13992],physical[13992],bindings,None)
    record['semantics']['consumption']={'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':660}}}
    bound=native.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[record]},bindings,set(index))
    return {'schema':'OTERYN_NATIVE_WVAR_ITEM_PROFILES/v1','producer':'tools/content-migration/wvar_item_profiles.py','source_revision':REVISION,
            'source_files':[{'path':p,'sha256':h} for p,h in SOURCES],
            'binding_table_sha256':hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),'records':bound['records']}


def build_archive(archive): return build_sources(read_archive(archive))


def admit(root,generated,profile_path,manifest_path,order,canonical):
    overlay=json.loads((root/PATH).read_bytes())
    if overlay['schema']!='OTERYN_NATIVE_WVAR_ITEM_PROFILES/v1' or overlay['source_revision']!=REVISION or overlay['source_files']!=[{'path':p,'sha256':h} for p,h in SOURCES] or overlay['binding_table_sha256']!=hashlib.sha256((root/'imports/crystalserver/bindings/items.json').read_bytes()).hexdigest():
        raise ValueError('WVAR_ITEM_PROFILE_PROVENANCE')
    if len(overlay['records'])!=1 or overlay['records'][0]['production_definition']['production_key']!=KEY:
        raise ValueError('WVAR_ITEM_PROFILE_SET')
    row=overlay['records'][0]
    definition,=[r['definition'] for r in json.loads((root/CANONICAL).read_bytes())['records'] if r['definition']['identity']['key']==KEY]
    for field in ('physical','stack','consumption'):
        if definition['semantics'][field]!=row['semantics'][field]:
            raise ValueError('WVAR_ITEM_CANONICAL_FACTS')
    if (definition['identity']!=row['production_binding']['target'] or not definition['materializable']
            or definition['stack_class']!=row['admission']['stack_class']
            or row['authoring']['item']!={'family':'Item','key':'candidate:item/13992','revision':'spell-p2-r21'}):
        raise ValueError('WVAR_ITEM_CANONICAL_IDENTITY')
    document=json.loads(generated[profile_path])
    if any(r['production_definition']['production_key']==KEY for r in document['records']):
        raise ValueError('WVAR_ITEM_PROFILE_DUPLICATE')
    document['records']=sorted(document['records']+copy.deepcopy(overlay['records']),key=order)
    data=(json.dumps(document,indent=2,ensure_ascii=False)+'\n').encode()
    generated[profile_path]=data
    manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=hashlib.sha256(data).hexdigest();generated[manifest_path]=canonical(manifest)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--crystal-archive',type=Path,required=True);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    raw=(json.dumps(build_archive(args.crystal_archive),indent=2,ensure_ascii=False)+'\n').encode()
    if args.check:
        if (ROOT/PATH).read_bytes()!=raw: raise SystemExit('WVAR_ITEM_PROFILE_DRIFT')
    else: (ROOT/PATH).write_bytes(raw)
    print('WVAR_ITEM_SOURCE_VERIFIED item=13992 weight=660 stack=100 food_seconds=660')
