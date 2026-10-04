"""Source-backed positive and mutation qualification; reports paths/counts only."""
import argparse, copy, gzip, json
from pathlib import Path
from jsonschema import Draft202012Validator
try:
    from . import dialogue_supplement as d
except ImportError:
    import dialogue_supplement as d
p=argparse.ArgumentParser()
for key in ('corpus-manifest','ast-root','repo-root','packet','out'):p.add_argument('--'+key,type=Path,required=True)
a=p.parse_args(); raw=a.packet.read_bytes(); packet=json.loads(gzip.decompress(raw) if a.packet.suffix=='.gz' else raw)
schema=Draft202012Validator(json.loads(Path(__file__).with_name('dialogue_supplement.schema.json').read_text()))
errors=list(schema.iter_errors(packet))
if errors:raise ValueError('Schema paths '+str([list(e.path) for e in errors]))
summary=d.qualify(packet,a.corpus_manifest,a.ast_root,a.repo_root)
mutations=[]
for label in ['false_factory_alias','false_helper_actor_binding','changed_handin_order','source_hash_tamper','native_promotion']:
    bad=copy.deepcopy(packet)
    if label=='false_factory_alias':
        r=next(r for r in bad['records'] if r['path'].endswith('/captain_haba_open_sea.lua'));r['factory_name_declarations'][0]['literal']='Captain Haba'
    elif label=='false_helper_actor_binding':bad['helper_links'][0]['native_actor_binding']=True
    elif label=='changed_handin_order':
        r=next(r for r in bad['records'] if r['path'].endswith('/tereban_functions.lua'));r['events'].reverse()
    elif label=='source_hash_tamper':bad['records'][0]['source_sha256']='0'*64
    else:bad['native_admission']=True
    try:d.qualify(bad,a.corpus_manifest,a.ast_root,a.repo_root)
    except ValueError:mutations.append({'control':label,'result':'REJECTED'})
    else:raise ValueError('Accepted mutated packet '+label)
receipt={'schema':'OTERYN_DONOR_DIALOGUE_SUPPLEMENT_QUALIFICATION/v1','packet_sha256':d.digest(raw),'source_backed_rebuild':'PASS','schema':'OTERYN_DONOR_DIALOGUE_SUPPLEMENT_QUALIFICATION/v1','schema_valid':'PASS','summary':summary,'negative_controls':mutations,'native_admission':False}
a.out.write_bytes(d.stable(receipt)+b'\n');print(json.dumps(summary));print('5 Source-backed mutation controls REJECTED')
