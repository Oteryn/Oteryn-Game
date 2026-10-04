"""Bounded secondary cached-manual notes compared with eight immutable house command donors."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
PINS={'canary':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystal':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
NOTES=ROOT/'docs/reference/tibia-manual/houses.md'
COMMANDS={
 'house_guest_list':('guest_access_list','Aleta Sio',['owner','subowner'],'GUEST_LIST'),
 'house_subowner_list':('subowner_access_list','Aleta Som',['owner'],'SUBOWNER_LIST'),
 'house_door_list':('door_access_list','Aleta Grav',['owner'],'doorId'),
 'house_kick':('kick_character','Alana Sio',['owner','subowner'],'GUEST_LIST'),
}


def sha(data):return hashlib.sha256(data).hexdigest()


def source_blob(source,path):
    data=subprocess.check_output(['git','-C','/workspace/spell-sources/'+source,'show',PINS[source]+':'+path])
    return data,{'source':source,'revision':PINS[source],'path':path,'sha256':sha(data),'bytes':len(data),
                 'git_blob':hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()}


def build_records(read_at):
    notes=NOTES.read_bytes();text=notes.decode()
    records=[]
    for source in sorted(PINS):
        cpp,cpp_proof=source_blob(source,'src/map/house/house.cpp')
        if not re.search(r'case HOUSE_SUBOWNER:\s*return listId == GUEST_LIST;',cpp.decode()) or not re.search(r'case HOUSE_OWNER:\s*return true;',cpp.decode()):
            raise ValueError('source access gate changed; manual roles require renewed comparison')
        for filename,(operation,label,roles,list_id) in COMMANDS.items():
            path='data/scripts/spells/house/'+filename+'.lua'
            data,proof=source_blob(source,path);lua=data.decode()
            name=re.search(r'spell:name\("([^"\n]+)"\)',lua)[1]
            words=re.search(r'spell:words\("([^"\n]+)"\)',lua)[1]
            quote=next(line.strip() for line in text.splitlines() if line.lstrip().startswith('- `'+label+'`'))
            quoted_words=re.search(r'`([^`]+)`',quote)[1].lower()
            if words != quoted_words:raise ValueError('current source words do not match cached note')
            if operation != 'kick_character' and ('setEditHouse(house, '+list_id+')' not in lua or 'sendHouseWindow(house, '+list_id+')' not in lua):
                raise ValueError('source list operation changed')
            if operation == 'kick_character' and ('targetPlayer == player' not in lua or 'guestHouse:kickPlayer(player, targetPlayer)' not in lua):
                raise ValueError('source kick operation changed')
            records.append({'schema':'OTERYN_HOUSE_COMMAND_REFERENCE/v1','source_identity':source+'/'+path,
                            'source':proof,'name':name,'words':words,'operation':operation,
                            'external_reference':{'url':'https://www.tibia.com/gameguides/?subtopic=manual&section=houses',
                             'section':'5.7.3 Rights and Duties / House Rights','cache_path':str(NOTES.relative_to(ROOT)),
                             'cache_sha256':sha(notes),'content_sha256':sha(notes),'original_capture_date':'2026-09-28',
                             'read_at_utc':read_at,'read_method':'local_cached_oteryn_manual_notes','quote':quote,
                             'quote_kind':'oteryn_written_secondary_notes','original_manual_text_verified':False},
                            'typed_note_facts':{'authorized_house_roles':roles,'self_kick_available':operation=='kick_character',
                             'list_scope':'guests' if operation=='guest_access_list' else 'subowners' if operation=='subowner_access_list' else 'individual_door' if operation=='door_access_list' else 'not_applicable',
                             'premium_subowner_mentioned':operation=='guest_access_list'},
                            'upstream_comparison':{'words_match':True,'command_operation_match':True,
                             'source_house_gate':list_id,'house_cpp_provenance':cpp_proof,
                             'permission_alignment':'basic_roles_only_premium_not_verified' if operation=='guest_access_list' else 'basic_role_gate_only',
                             'self_kick_branch_present':operation=='kick_character'},
                            'limitations':['Quotes are Oteryn-written notes; original official manual text and original capture provenance are not independently verified.',
                             'No fresh wiki description was obtained: online access and remote browser were unavailable.',
                             'This comparison does not qualify full callback guards, Premium rules, door geometry, access-list syntax, or runtime behavior.'],
                            'fresh_wiki_verified':False,'binding_qualified':False,'runtime_activation':False})
    return records


def generate(out,read_at):
    from jsonschema import Draft202012Validator
    schema_path=HERE/'house-command-reference.schema.json'
    validator=Draft202012Validator(json.loads(schema_path.read_text()))
    records=build_records(read_at)
    for record in records:validator.validate(record)
    out.mkdir(parents=True,exist_ok=True)
    artifact=out/'house-command-references.jsonl.gz'
    payload=b''.join((json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode() for r in sorted(records,key=lambda r:r['source_identity']))
    with artifact.open('wb') as raw,gzip.GzipFile(filename='',fileobj=raw,mode='wb',mtime=0) as stream:stream.write(payload)
    proof={'schema':'OTERYN_HOUSE_COMMAND_REFERENCE_RECEIPT/v1','record_count':len(records),'command_count':len(COMMANDS),
           'gzip_path':str(artifact),'gzip_sha256':sha(artifact.read_bytes()),'payload_sha256':sha(payload),
           'schema_path':str(schema_path.relative_to(ROOT)),'schema_sha256':sha(schema_path.read_bytes()),
           'source_revisions':sorted(PINS.values()),'cache_path':str(NOTES.relative_to(ROOT)),'cache_sha256':sha(NOTES.read_bytes()),
           'read_at_utc':read_at,'exporter_sha256':sha(Path(__file__).read_bytes()),'runtime_activation':False,'fresh_wiki_verified':False,
           'online_attempts':[
             {'url':None,'read_method':'tavily_search','result':'UNAUTHORIZED_REQUIRES_REAUTHENTICATION'},
             {'url':'https://www.tibiawiki.com.br/wiki/Houses','read_method':'normal_http','result':'HTTP_403'},
             {'url':'https://tibia.fandom.com/wiki/House_Management','read_method':'normal_http','result':'HTTP_402'},
             {'url':'https://tibiopedia.pl/poradniki/Domy','read_method':'normal_http','result':'REDIRECTED_TO_SETUP_NO_HOUSE_CONTENT'},
             {'url':'https://tibiawiki.com/wiki/Houses','read_method':'normal_http','result':'HTTP_403'},
             {'url':'https://www.tibia.com/gameguides/?subtopic=manual&section=houses','read_method':'normal_http','result':'HTTP_403'},
             {'url':'https://tibia-stats.com/index.php?akcja=6003','read_method':'normal_http','result':'HTTP_403'},
             {'url':'https://www.tibiaqa.com/2302/what-are-all-the-house-commands','read_method':'normal_http','result':'HTTP_403'},
             {'url':None,'read_method':'remote_desktop_device_status','result':'ALL_DEVICES_OFFLINE_NO_BROWSER_ACTIONS'}],
           'cached_wiki_check':{'path':'docs/reference/spells/r24-candidate/research/browser-read-observations.json',
              'url':'https://www.tibiawiki.com.br/wiki/Aleta_Sio','result':'CACHED_PAGE_EXPLICITLY_NO_ARTICLE_TEXT_NOT_EVIDENCE'},
           'original_manual_provenance_verified':False,
           'online_attempts_observed_at_utc':'2026-10-04T01:19:02Z',
           'online_attempt_timestamp_basis':'Batch observation time; individual request timestamps were not retained.',
           'online_attempt_content_retained':False,
           'tavily_query':'Tibia "aleta sio" "aleta som" "aleta grav" "alana sio" house',
           'tibiopedia_final_url':'https://tibiopedia.pl/setup'}
    (out/'house-command-reference-receipt.json').write_text(json.dumps(proof,indent=2)+'\n')
    return proof


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True);parser.add_argument('--read-at',required=True)
    args=parser.parse_args();print(json.dumps(generate(args.out,args.read_at),sort_keys=True))
