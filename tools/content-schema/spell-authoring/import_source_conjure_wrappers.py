"""Recover four exact current conjure wrappers shadowed by name-only rune dispatch.

Source-only r34 companions; existing schemas/conjure converter, no Lua execution.
Item references deliberately retain source-player-r28 provider identities.
"""
import argparse
import copy
import gzip
import json
import os
import re
from pathlib import Path

import import_source_player_bundles as base

REVISION = 'source-player-r34'
PINS = {'canary-main-current':'04b83b512114bfd888000d6e1433ed8ecaec7c5b',
        'crystal-summer-current':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
WRAPPERS = {'chameleon_rune':(3147,3178,1), 'destroy_field_rune':(3147,3148,3)}


def qualify(record, data):
    key = record['registration_key']; snapshot = key.split('/')[0]
    raw = record['source_callback_facts']; stem = Path(raw['file']).stem
    if snapshot not in PINS or record['source_revision'] != PINS[snapshot]:
        raise ValueError('wrapper source revision mismatch')
    if raw['file'] != 'data/scripts/spells/conjuring/' + stem + '.lua' or stem not in WRAPPERS:
        raise ValueError('not an allowlisted conjure wrapper')
    if raw['spell_type'] != 'instant' or raw['cast']['tier'] != 'conjure':
        raise ValueError('wrapper carrier/tier mismatch; rune behavior cannot be imported as conjure')
    if base.sha(data) != record['source_sha256']:
        raise ValueError('wrapper source digest mismatch')
    if raw['blob'] != base.hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest():
        raise ValueError('wrapper Git blob mismatch')
    names=re.findall(r'spell:name\("([^"]+)"\)',data.decode())
    if names != [raw['name']] or raw['registrar']['name'] != raw['name']:
        raise ValueError('wrapper registration name mismatch')
    matches = list(re.finditer(r'function\s+spell\.onCastSpell\(creature,\s*variant\)\s*return\s+creature:conjureItem\((\d+),\s*(\d+),\s*(\d+)\)\s*end',data.decode()))
    if len(matches) != 1 or len(re.findall(r'\bonCastSpell\b',data.decode())) != 1:
        raise ValueError('wrapper callback is not the exact literal conjure template')
    expected = WRAPPERS[stem]
    actual = tuple(map(int,matches[0].groups()))
    facts = raw['cast']['conjure']
    if actual != expected or actual != (facts['reagent_item_id'],facts['result_item_id'],facts['count']):
        raise ValueError('wrapper conjure arguments differ from source facts')
    return {'start_line':data[:matches[0].start()].count(b'\n')+1,
            'end_line':data[:matches[0].end()].count(b'\n')+1,
            'callback_sha256':base.sha(matches[0].group().encode()),
            'reagent_item_id':actual[0],'result_item_id':actual[1],'count':actual[2],
            'argument_count':3,'missing_fourth_argument':'nil',
            'source_effect_branch':'result_item_is_rune ? CONST_ME_MAGIC_RED : fourth_argument',
            'result_item_type_qualified':False,'runtime_activation':False}


def generate(out, source_root, r28):
    os.environ['GIT_NO_LAZY_FETCH'] = '1'
    if out.exists():
        raise ValueError('new r34 output required; frozen output cannot be overwritten')
    manifest_path=r28/'package-manifest.json';manifest=json.loads(manifest_path.read_text())['files']
    callback=r28/'source-callback-facts.jsonl.gz'
    if manifest.get(callback.name)!=base.sha(callback.read_bytes()):
        raise ValueError('r28 callback input pin mismatch')
    rows=[r for r in map(json.loads,gzip.decompress(callback.read_bytes()).splitlines())
          if any(r['registration_key']==snapshot+'/data/scripts/spells/conjuring/'+stem+'.lua#1'
                 for snapshot in PINS for stem in WRAPPERS)]
    expected={snapshot+'/data/scripts/spells/conjuring/'+stem+'.lua#1' for snapshot in PINS for stem in WRAPPERS}
    if len(rows)!=4 or {r['registration_key'] for r in rows}!=expected:
        raise ValueError('exact four source wrapper registrations required')
    out.mkdir(parents=True)
    schema=base.receipt_schema();base.write(out/'receipt.schema.json',schema)
    validator=base.validate_spell.Draft202012Validator(schema,registry=base.validate_spell.REGISTRY)
    index=[];proofs=[]
    for record in sorted(rows,key=lambda r:r['registration_key']):
        reg=record['registration_key'];snapshot=reg.split('/')[0];raw=record['source_callback_facts']
        repo=source_root/snapshot.split('-')[0];data=base.source_file(repo,PINS[snapshot],raw['file'])
        proof=qualify(record,data)
        helper_path='data/scripts/lib/register_spells.lua'
        helper=base.source_file(repo,PINS[snapshot],helper_path)
        helper_text=helper.decode();start=helper_text.index('function Player:conjureItem(reagentId, conjureId, conjureCount, effect)')
        branch='self:getPosition():sendMagicEffect(item:getType():isRune() and CONST_ME_MAGIC_RED or effect)'
        branch_pos=helper_text.index(branch,start)
        proof['helper_effect_branch_source']={'path':helper_path,'source_revision':PINS[snapshot],
             'file_sha256':base.sha(helper),'function_start_line':helper_text[:start].count('\n')+1,
             'branch_line':helper_text[:branch_pos].count('\n')+1,'branch_sha256':base.sha(branch.encode())}
        proofs.append({'registration_key':reg,**proof})
        row_id=base.sha(reg.encode())[:16];prefix=snapshot+'/'+row_id
        for filename in ['receipt.json','source-header.json']:
            path=r28/prefix/filename
            if manifest.get(prefix+'/'+filename)!=base.sha(path.read_bytes()):
                raise ValueError('r28 wrapper bundle pin mismatch')
        receipt=copy.deepcopy(json.loads((r28/prefix/'receipt.json').read_text()))
        projection=json.loads((r28/prefix/'source-header.json').read_text())['spell']
        defaults,default_proofs=base.default_fields(repo,PINS[snapshot])
        key=receipt['candidate_key']
        spell=base.fill_header(projection,defaults,{'key':key,'revision':REVISION},raw['registrar'])
        execution,deps,notes,gaps=base.convert_conjure(None,raw,spell,snapshot)
        gaps=[{'source_field':'conjure.source_effect_branch',
               'reason':'Owning helper selects rune RED versus fourth argument (nil here); result Item type and visual owner binding remain unqualified. No unconditional default effect asserted.'}]
        spell['execution']=execution
        catalog={'definitions':base.item_references({'spell':spell,'dependencies':deps})}
        errors=base.validate_spell.validate({'spell':spell},deps,catalog)
        if errors:raise ValueError('r34 candidate validation: '+' | '.join(errors))
        receipt.update(status='CANDIDATE_SCHEMA_VALID',blockers=[],remaining_mechanics=gaps,
                       native_execution_qualified=False,dependencies={k:len(v) for k,v in deps.items()},
                       schema_and_semantic_validation_errors=[],item_owner_bindings_required=catalog['definitions'],
                       engine_default_proofs=default_proofs,conversion_notes=notes+['Exact instant conjure wrapper; name-only rune native dispatch bypassed after full source/template qualification.'])
        validator.validate(receipt)
        folder=out/prefix;folder.mkdir(parents=True)
        for filename,value in [('source-header.json',{'spell':projection}),('spell.json',{'spell':spell}),
                               ('dependencies.json',deps),('catalog.json',catalog),('receipt.json',receipt)]:
            base.write(folder/filename,value)
        index.append({'registration_key':reg,'candidate_key':key,'candidate_revision':REVISION,
                      'status':'CANDIDATE_SCHEMA_VALID','blockers':[]})
    payload=b''.join((json.dumps(r,sort_keys=True,separators=(',',':'))+'\n').encode() for r in sorted(rows,key=lambda r:r['registration_key']))
    (out/callback.name).write_bytes(gzip.compress(payload,mtime=0))
    base.write(out/'wrapper-source-proof.json',{'schema':'OTERYN_SOURCE_CONJURE_WRAPPER_QUALIFICATION/v1',
               'records':proofs,'source_pins':PINS,'item_provider_revision':base.REVISION,
               'runtime_activation':False,'external_sources_used':False})
    summary={'schema':'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1','records':4,'revision':REVISION,
             'source_populations':{s:2 for s in PINS},'status_counts':{'CANDIDATE_SCHEMA_VALID':4},
             'native_descriptor_count':0,'all_receipts_schema_valid':True,'runtime_activation':False,
             'external_sources_used':False,'full_source_mechanics_1_to_1_complete':False,
             'source_callback_payload_sha256':base.sha(payload),'records_index':index,
             'input_proofs':{'r28/package-manifest.json':base.sha(manifest_path.read_bytes())},
             'converter_proofs':{p.name:base.sha(p.read_bytes()) for p in [Path(__file__),Path(base.__file__),
                               base.HERE/'spell.schema.json',base.HERE/'spell-dependencies.schema.json']}}
    base.write(out/'import-summary.json',summary)
    base.write(out/'package-manifest.json',{'schema':'OTERYN_SOURCE_PLAYER_PACKAGE/v1',
               'files':{p.relative_to(out).as_posix():base.sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file()}})
    return summary


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--source-root',type=Path,required=True)
    parser.add_argument('--r28',type=Path,required=True)
    args=parser.parse_args();print(json.dumps(generate(args.out,args.source_root,args.r28)))
