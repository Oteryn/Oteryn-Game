"""SRX-1: physical admission for Decision 25's already qualified writing cohort.

Text rules remain owned by writable_item_profiles. Current Crystal XML and the
retained official appearance bank fill physical facts, never resolve pending text rules.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import dispenser_item_profiles as capture_source
import writable_item_profiles as writing

ROOT=Path(__file__).resolve().parents[2]
PATH='tools/content-schema/native-gameplay/writing-item-profiles.json'
QUALIFIED={'WRITE_ONCE_TEXT_SEMANTICS_ONLY','FIXED_TEXT_SEMANTICS_ONLY','REWRITE_TEXT_SEMANTICS_ONLY'}


def ids(root=ROOT):
    document=json.loads(writing.outputs(root)[writing.PATH])
    return tuple(sorted(int(row['item']['key'].rsplit('i',1)[1]) for row in document['records'] if row['qualification'] in QUALIFIED))


def merge_known(old,new,path=()):
    if new=={'state':'UNKNOWN'}:
        return copy.deepcopy(old)
    if old=={'state':'UNKNOWN'}:
        return copy.deepcopy(new)
    if isinstance(old,dict) and isinstance(new,dict):
        return {key:merge_known(old[key],new[key],(*path,key)) if key in old and key in new else copy.deepcopy(old.get(key,new.get(key))) for key in old.keys()|new.keys()}
    if old!=new and (path[-4:]==('physical','value','weight','value') or path[-4:]==('presentation','value','description','value')):
        # Preserve the base's accepted Item statistics and editorial punctuation.
        return copy.deepcopy(old)
    if old!=new:
        raise ValueError(f'WRITING_KNOWN_FACT_CONFLICT:{path}:{old!r}:{new!r}')
    return copy.deepcopy(old)


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=json.loads((root/PATH).read_bytes())
    expected_ids=ids(root)
    if (packet['schema']!='OTERYN_NATIVE_WRITING_ADMISSION/v1'
            or packet['source_revision']!=capture_source.REVISION
            or packet['source_xml_sha256']!=capture_source.XML_SHA
            or packet['records']!=capture_source.records(packet['source_rows'],root,expected_ids)):
        raise ValueError('WRITING_ADMISSION_SOURCE_DRIFT')
    document=json.loads(generated[profile_path])
    old={row['production_definition']['production_key']:row for row in document['records']}
    for row in packet['records']:
        key=row['production_definition']['production_key']
        existing=old[key]
        if existing['admission']['materializable']:
            continue # Existing accepted owners retain their physical admission.
        if existing['production_definition']!=row['production_definition']:
            raise ValueError('WRITING_IDENTITY_CONFLICT')
        existing['semantics']=merge_known(existing['semantics'],row['semantics'],(key,))
        existing['attributes']=row['attributes']
        existing['admission']=row['admission']
    document['records'].sort(key=order)
    raw=(json.dumps(document,indent=2,ensure_ascii=False)+'\n').encode()
    generated[profile_path]=raw
    manifest=json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256']=hashlib.sha256(raw).hexdigest()
    generated[manifest_path]=canonical(manifest)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--xml',required=True,type=Path)
    args=parser.parse_args()
    packet=capture_source.capture(args.xml,ids())
    packet['schema']='OTERYN_NATIVE_WRITING_ADMISSION/v1'
    (ROOT/PATH).write_text(capture_source.capture_json(packet))
