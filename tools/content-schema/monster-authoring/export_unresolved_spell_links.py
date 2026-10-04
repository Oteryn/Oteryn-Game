"""Join immutable unresolved source slots to exact source facts, never to executable Abilities."""
import argparse
import gzip
import hashlib
import json
import tarfile
from collections import Counter
from pathlib import Path

SCHEMA='OTERYN_UNRESOLVED_MONSTER_SPELL_LINKS/v1'


def sha(data):return hashlib.sha256(data).hexdigest()
def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':')).encode()
def identity(source,record):
    return (source,record.get('revision'),record.get('path'),record.get('sha256'),record.get('git_blob'))


def index_records(records,custom=False):
    result={}
    for n,record in enumerate(records):
        value=record['source_identity'] if custom else record
        key=identity(value['source'],value)
        if key in result:raise ValueError('duplicate source fact identity')
        result[key]=(n,record)
    return result


def join(slots,inventory,custom,inventory_path,custom_path):
    facts=index_records(inventory);descriptors=index_records(custom,True)
    out=[];seen=set()
    for slot in slots:
        if slot['conversion_status']!='unresolved_semantics':continue
        slot_key=(slot['candidate_id'],slot['group'],slot['source_slot_index'])
        if slot_key in seen:raise ValueError('duplicate unresolved slot identity')
        seen.add(slot_key)
        source=slot['source'];registered=slot['registered_source'];reference=None;descriptor=None
        if registered is not None:
            key=identity(source,registered)
            match=facts.get(key)
            if match is None:
                raise ValueError('registered source lacks exact revision/path/SHA/blob inventory identity: '+str(slot_key))
            n,fact=match
            reference={'artifact':inventory_path,'record_index':n,'record_sha256':sha(canonical(fact)),
                       'source_identity':{'source':source,**{k:registered[k] for k in ['revision','path','sha256','git_blob']}},
                       'call_count':len(fact['calls']),'scope':'source_file_lexical_evidence_not_executable_cast'}
            if key in descriptors:
                n,data=descriptors[key]
                descriptor={'artifact':custom_path,'record_index':n,'record_sha256':sha(canonical(data)),
                            'mechanic_category':data['mechanic_category'],'scope':'exact_source_file_descriptor_not_native_admission'}
        params=slot['source_parameters']
        reasons=[r.get('resolution','') for r in slot['conversion_manifest_rows'] if r.get('status')=='unresolved_semantics']
        row={'schema':SCHEMA,'slot_identity':{'candidate_id':slot_key[0],'group':slot_key[1],'source_slot_index':slot_key[2]},
             'source':source,'monster':slot['monster'],'monster_source':{k:slot['monster_source'][k] for k in ['revision','path','sha256','git_blob']},
             'original_slot_sha256':sha(canonical(slot)),'source_parameters':params,
             'conversion_status':'unresolved_semantics','resolution':slot['resolution'],'script_tier':slot['script_tier'],
             'reasons':reasons,'registered_source':None if registered is None else {k:registered[k] for k in ['revision','path','sha256','git_blob']},
             'registered_fact_link':reference,'custom_descriptor_link':descriptor,
             'inline_type_present':registered is None and 'type' in params,
             'inline_raw_type':params.get('type') if registered is None else None,
             'runtime_activation':False,'native_ability_admission':False,
             'external_verification':slot['external_verification']}
        if not reasons:raise ValueError('unresolved source slot has no recorded source reason')
        out.append(row)
    return sorted(out,key=lambda r:tuple(r['slot_identity'].values()))


def export(root,out):
    import jsonschema
    if out.exists():raise ValueError('new output required; frozen datasets cannot be overwritten')
    receipt_path=root/'monster-source-package-manifest.json';receipt=json.loads(receipt_path.read_text())
    archive=root/receipt['archive']['path']
    if sha(archive.read_bytes())!=receipt['archive']['sha256']:raise ValueError('immutable monster archive SHA mismatch')
    slots=[];slot_proofs=[]
    with tarfile.open(archive) as tar:
        manifest_data=tar.extractfile('monster-package-manifest.json').read()
        manifest=json.loads(manifest_data);entries={e['path']:e for e in manifest['entries']}
        for source in ['canary','crystal']:
            name=source+'-monster-spell-slots.json';data=tar.extractfile(name).read()
            if sha(data)!=entries[name]['sha256'] or len(data)!=entries[name]['bytes']:
                raise ValueError('archive slot member SHA/bytes mismatch')
            rows=json.loads(data);slots.extend(rows)
            slot_proofs.append({'member':name,'sha256':sha(data),'bytes':len(data),'records':len(rows)})
    inventory_path=root/'source-mechanics-inventory.json.gz';custom_path=root/'source-custom-mechanics.json.gz'
    inventory_receipt=json.loads((root/'source-mechanics-receipt.json').read_text())
    pins=[r for r in inventory_receipt['artifacts'] if Path(r['path']).name==inventory_path.name]
    if len(pins)!=1 or sha(inventory_path.read_bytes())!=pins[0]['sha256']:
        raise ValueError('mechanics inventory receipt pin mismatch')
    custom_receipt=json.loads((root/'source-custom-mechanics-qualification.json').read_text())
    custom_payload=gzip.decompress(custom_path.read_bytes())
    if sha(custom_path.read_bytes())!=custom_receipt['gzip_sha256'] or sha(custom_payload)!=custom_receipt['payload_sha256']:
        raise ValueError('custom descriptor receipt pin mismatch')
    inventory=json.loads(gzip.decompress(inventory_path.read_bytes()))
    custom=json.loads(custom_payload)
    if custom['record_count']!=len(custom['records']) or custom['record_count']!=21:
        raise ValueError('exact 21 custom descriptor population required')
    rows=join(slots,inventory['files'],custom['records'],inventory_path.as_posix(),custom_path.as_posix())
    if len(slots)!=20742 or len(rows)!=175:raise ValueError('immutable r28 source population changed')
    schema_path=Path(__file__).with_name('unresolved-monster-spell-links.schema.json');schema=json.loads(schema_path.read_text())
    validator=jsonschema.Draft202012Validator(schema)
    for row in rows:validator.validate(row)
    payload=b''.join(canonical(row)+b'\n' for row in rows)
    out.mkdir(parents=True)
    data_path=out/'unresolved-monster-spell-links.jsonl.gz';data_path.write_bytes(gzip.compress(payload,mtime=0))
    registered=sum(row['registered_fact_link'] is not None for row in rows)
    proof={'schema':SCHEMA+'/proof','record_count':len(rows),'all_source_slots':len(slots),
           'status_counts':dict(Counter(row['conversion_status'] for row in rows)),
           'registered_fact_links':registered,'inline_raw_slots':len(rows)-registered,
           'custom_descriptor_links':sum(row['custom_descriptor_link'] is not None for row in rows),
           'source_counts':dict(Counter(row['source'] for row in rows)),
           'gzip_path':data_path.as_posix(),'gzip_sha256':sha(data_path.read_bytes()),'payload_sha256':sha(payload),
           'schema_path':schema_path.relative_to(Path(__file__).resolve().parents[3]).as_posix(),'schema_sha256':sha(schema_path.read_bytes()),
           'format':'jsonl_gzip','source_archive_sha256':sha(archive.read_bytes()),'source_receipt_sha256':sha(receipt_path.read_bytes()),
           'internal_manifest_sha256':sha(manifest_data),'source_slot_members':slot_proofs,
           'inventory_sha256':sha(inventory_path.read_bytes()),'custom_descriptors_sha256':sha(custom_path.read_bytes()),
           'source_revisions':{s['source']:s['revision'] for s in manifest['sources']},
           'producer_sha256':sha(Path(__file__).read_bytes()),
           'all_records_schema_valid':True,'original_parameters_and_slot_status_preserved':True,
           'runtime_activation':False,'native_ability_admission':False,'external_sources_used':False}
    (out/'unresolved-monster-spell-links-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
    return proof


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--r28',type=Path,required=True);parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args();print(json.dumps(export(args.r28,args.out)))
