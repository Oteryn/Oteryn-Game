"""Closed map-only physical qualification for the33 actual Spike source corridor objects.

Reuses the pinned source importer. It never creates an Item alias, changes a known
collision flag, or replaces an existing materializable/inventory admission.
"""
from pathlib import Path
import argparse,copy,hashlib,importlib.util,json,sys
ROOT=Path(__file__).resolve().parents[2]
PROFILE='tools/content-schema/world-authoring/samples/spike-corridor-source-physics.json'
IDS={1854,1855,1856,1857,1858,1865,1867,12083,12084,12345,12347,12349,12350,12351,12352,12353,12354,15466,15847,15849,15850,15852,15853,15855,15856,15859,15860,18368,19350,19375,19376,19377,19378}
CPP_EVIDENCE=[{'repository': 'opentibiabr/canary', 'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446', 'path': 'src/items/items.hpp', 'sha256': 'b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6', 'line': 322}, {'repository': 'opentibiabr/canary', 'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446', 'path': 'src/items/functions/item/item_parse.cpp', 'sha256': '4abe1c35c621595472bc6b30d39e10d7db115dd569d9e3334b64b600b9141477', 'line': 235}, {'repository': 'zimbadev/crystalserver', 'revision': '201189f871d23597b82caf88fd611454a2ddefb8', 'path': 'src/items/items.hpp', 'sha256': '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca', 'line': 339}, {'repository': 'zimbadev/crystalserver', 'revision': '201189f871d23597b82caf88fd611454a2ddefb8', 'path': 'src/items/functions/item/item_parse.cpp', 'sha256': '08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7', 'line': 233}]
COLLISION=('blocks_movement','blocks_projectile','immovable_block_solid','has_height')

def capture(canary,crystal):
    # A private module instance supplies a closed authoring roster to the same
    # SHA-qualified importer; the normal seasonal producer's module is untouched.
    spec=importlib.util.spec_from_file_location('_spike_source_capture',ROOT/'tools/content-migration/seasonal_item_profiles.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    module.IDS=IDS
    packet=module.capture(canary,crystal)
    sys.path.insert(0,str(ROOT/'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    dat=(canary/'data/items/appearances.dat').read_bytes();xml=(canary/'data/items/items.xml').read_bytes()
    actual=source.policies(dat,xml,IDS)
    other_dat=(crystal/'data/items/appearances.dat').read_bytes()
    if hashlib.sha256(other_dat).hexdigest()!='17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1':raise ValueError('SPIKE_PINNED_CRYSTAL_APPEARANCES')
    other_xml=(crystal/'data/items/items.xml').read_bytes()
    cross={p['server_item_id']:p for p in source.policies(other_dat,other_xml,IDS)}
    fields=['block_solid','block_projectile','has_height','ground','floor_change','floor_change_kind']
    if any(p[f]!=cross[p['server_item_id']][f] for p in actual for f in fields):raise ValueError('SPIKE_CROSS_DONOR_WORLD_PHYSICS')
    if any(p['floor_change'] or p['floor_change_kind'] is not None for p in actual):raise ValueError('SPIKE_SOURCE_FLOOR_CHANGE_CHANGED')
    packet['schema']='OTERYN_NATIVE_SPIKE_CORRIDOR_SOURCE_PHYSICS/v1'
    packet['withheld']=[{'id':15012,'reason':'No XML Item identity and absent actual compiled palette; dropped provisional appearance. No Item alias or admission invented.'}]
    for evidence in CPP_EVIDENCE:
        donor=canary if evidence['repository']=='opentibiabr/canary' else crystal
        raw=(donor/evidence['path']).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=evidence['sha256']:raise ValueError('SPIKE_PINNED_FLOOR_DEFAULT_SOURCE')
    packet['floor_change_default_evidence']=CPP_EVIDENCE
    packet['source_world_physics']=actual
    packet['crosscheck_world_physics']={'appearances_sha256':hashlib.sha256(other_dat).hexdigest(),'items_xml_sha256':hashlib.sha256(other_xml).hexdigest(),'fields':fields,'differences':[]}
    return packet

def admit(root,generated,profile_path,manifest_path,order,canonical):
    document=json.loads((root/PROFILE).read_bytes())
    if document['schema']!='OTERYN_NATIVE_SPIKE_CORRIDOR_SOURCE_PHYSICS/v1':raise ValueError('SPIKE_PHYSICAL_SOURCE_SCHEMA')
    if document['source_revision']!='47dfd51f45280a59a1d3e50ba7edd573d7234446' or document['crosscheck_revision']!='201189f871d23597b82caf88fd611454a2ddefb8':raise ValueError('SPIKE_PHYSICAL_SOURCE_REVISION')
    if document['floor_change_default_evidence']!=CPP_EVIDENCE:raise ValueError('SPIKE_KNOWN_FLOOR_DEFAULT_SOURCE')
    rows=document['records']
    if len(rows)!=len(IDS) or {int(r['production_binding']['external_id']) for r in rows}!=IDS:raise ValueError('SPIKE_PHYSICAL_EXACT_SOURCE_COVERAGE')
    if document['crosscheck_world_physics']['appearances_sha256']!='17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1' or document['crosscheck_world_physics']['differences']:raise ValueError('SPIKE_PHYSICAL_CROSSCHECK')
    facts={p['server_item_id']:p for p in document['source_world_physics']}
    if len(document['source_world_physics'])!=len(IDS) or set(facts)!=IDS or any(p['floor_change'] or p['floor_change_kind'] is not None for p in facts.values()):raise ValueError('SPIKE_PHYSICAL_KNOWN_FLOOR_NONE')
    native=json.loads(generated[profile_path]);by={r['production_definition']['production_key']:r for r in native['records']}
    for source in rows:
        id_=int(source['production_binding']['external_id']);definition=source['production_definition']
        if definition!={'family':'Item','production_key':f'oteryn:item.tibia.i{id_}','revision_ref':'definition-r1'}:raise ValueError('SPIKE_PHYSICAL_NATIVE_IDENTITY')
        target={'family':'Item','key':definition['production_key'],'revision':'definition-r1'}
        if source['production_binding']['disposition']!='EXACT' or source['production_binding']['target']!=target:raise ValueError('SPIKE_PHYSICAL_EXACT_BINDING')
        physical=facts[id_]
        expected={'blocks_movement':physical['block_solid'],'blocks_projectile':physical['block_projectile'],'immovable_block_solid':physical['block_solid'] and not physical['movable'],'has_height':physical['has_height']}
        if any(source['attributes'].get(f) is not expected[f] for f in COLLISION):raise ValueError('SPIKE_PHYSICAL_SOURCE_FLAGS')
        if source['admission']['legal_destinations']!=['Ground']:raise ValueError('SPIKE_STATIC_SOURCE_DESTINATION')
        projected=copy.deepcopy(source);projected['admission']['materializable']=False
        current=by.get(definition['production_key'])
        if current is None:
            native['records'].append(projected);continue
        if current['admission']!=projected['admission']:raise ValueError('SPIKE_EXISTING_ITEM_AUTHORITY_MUST_STAY')
        if any(current[f]!=projected[f] for f in ('production_definition','production_binding','semantics')):raise ValueError('SPIKE_EXISTING_SOURCE_CONFLICT')
        for f in COLLISION:
            if current['attributes'].get(f) is not None and current['attributes'][f] is not expected[f]:raise ValueError('SPIKE_EXISTING_KNOWN_COLLISION_CONFLICT')
            current['attributes'][f]=expected[f]
    native['records'].sort(key=order)
    raw=(json.dumps(native,indent=2,ensure_ascii=False)+'\n').encode();generated[profile_path]=raw
    manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=hashlib.sha256(raw).hexdigest();generated[manifest_path]=canonical(manifest)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--canary',type=Path,required=True);p.add_argument('--crystal',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    a.output.write_text(json.dumps(capture(a.canary,a.crystal),indent=2)+'\n')
