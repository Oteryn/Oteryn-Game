"""Replay accepted authoring deltas over the immutable r25 spell import baseline.

These are qualified project snapshots, not new Global facts or runtime grants.
"""
from __future__ import annotations
import copy
import hashlib
import json
from pathlib import Path

PATCH_SHA256 = '4aa9ae460c13fcaee1f95b6ea9c71df3e8ca3db03c98d9cf739682755d187517'
PATCH_PATH = 'imports/spells/monster-seven/accepted-authoring-overlay.json'
ALLOWED_PATHS = frozenset({
    'content/creatures/definitions/spell-native-profiles.json',
    'content/presentations/definitions/spell-native-profiles.json',
    'content/presentations/bindings/spell-appearances.json',
    'content/spells.manifest.json',
})
def digest(raw):
    return hashlib.sha256(raw).hexdigest()
def value_digest(value):
    return digest(json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode())
def parts(pointer):
    if not isinstance(pointer,str) or not pointer.startswith('/'):
        raise ValueError('OVERLAY_INVALID_POINTER')
    raw=pointer[1:].split('/')
    if any('~' in x.replace('~0','').replace('~1','') for x in raw):
        raise ValueError('OVERLAY_INVALID_POINTER_ESCAPE')
    return [x.replace('~1','/').replace('~0','~') for x in raw]
def parent(document,pointer):
    keys=parts(pointer);node=document
    for key in keys[:-1]:
        if isinstance(node,list):
            if not key.isdigit() or str(int(key))!=key:raise ValueError('OVERLAY_ARRAY_INDEX')
            node=node[int(key)]
        elif isinstance(node,dict):node=node[key]
        else:raise ValueError('OVERLAY_POINTER_PARENT')
    return node,keys[-1]
def encode(value,style):
    if style=='COMPACT':options={'separators':(',',':')}
    elif style=='PRETTY2':options={'indent':2}
    elif style=='COMPACT_SORTED':options={'separators':(',',':'),'sort_keys':True}
    elif style=='PRETTY2_SORTED':options={'indent':2,'sort_keys':True}
    else:raise ValueError('OVERLAY_ENCODING')
    return (json.dumps(value,ensure_ascii=False,**options)+'\n').encode()
def apply_overlay(root:Path,generated:dict[str,bytes],patch=None):
    raw=(root/PATCH_PATH).read_bytes() if patch is None else encode(patch,'PRETTY2')
    if digest(raw)!=PATCH_SHA256:raise ValueError('OVERLAY_RECEIPT_DIGEST')
    receipt=json.loads(raw)
    if receipt.get('schema')!='OTERYN_ACCEPTED_SPELL_AUTHORING_OVERLAY/v1' or receipt.get('runtime_activation') is not False:
        raise ValueError('OVERLAY_SCHEMA')
    if receipt.get('qualification')!='ADOPTED_SOURCE_WIKI_PROJECT_AUTHORING_SNAPSHOT':
        raise ValueError('OVERLAY_QUALIFICATION')
    files=receipt.get('files',[])
    if len(files)!=len(ALLOWED_PATHS) or {f.get('path') for f in files}!=ALLOWED_PATHS:
        raise ValueError('OVERLAY_PATH_SET')
    qualifications=receipt.get('qualification_receipts',[])
    if len(qualifications)!=1 or qualifications[0].get('path')!='imports/spells/monster-seven/adopted-qualification.json':
        raise ValueError('OVERLAY_QUALIFICATION_PATH')
    if digest((root/qualifications[0]['path']).read_bytes())!=qualifications[0]['sha256']:
        raise ValueError('OVERLAY_QUALIFICATION_DIGEST')
    result=dict(generated)
    for f in files:
        path=f['path'];raw=result[path]
        if digest(raw)!=f['base_sha256']:raise ValueError('OVERLAY_BASE_DIGEST:'+path)
        value=json.loads(raw);seen=set()
        for op in f['operations']:
            pointer=op['pointer']
            if pointer in seen:raise ValueError('OVERLAY_DUPLICATE_POINTER')
            seen.add(pointer);node,key=parent(value,pointer)
            index=None
            if isinstance(node,list):
                if not key.isdigit() or str(int(key))!=key:raise ValueError('OVERLAY_ARRAY_INDEX')
                index=int(key);exists=index<len(node)
            elif isinstance(node,dict):exists=key in node
            else:raise ValueError('OVERLAY_POINTER_PARENT')
            action=op['op']
            if action=='add':
                if exists:raise ValueError('OVERLAY_ADD_EXISTS')
                if isinstance(node,list):
                    if index!=len(node):raise ValueError('OVERLAY_ARRAY_APPEND')
                    node.append(copy.deepcopy(op['value']))
                else:node[key]=copy.deepcopy(op['value'])
            elif action in {'replace','remove'}:
                if not exists or value_digest(node[index] if isinstance(node,list) else node[key])!=op['old_value_sha256']:
                    raise ValueError('OVERLAY_OLD_VALUE_DIGEST')
                if action=='replace':node[index if isinstance(node,list) else key]=copy.deepcopy(op['value'])
                elif isinstance(node,list):raise ValueError('OVERLAY_ARRAY_REMOVE_UNSUPPORTED')
                else:del node[key]
            else:raise ValueError('OVERLAY_UNKNOWN_OPERATION')
        order_seen=set()
        for order in f.get('object_key_order',[]):
            pointer=order['pointer']
            if pointer in order_seen:raise ValueError('OVERLAY_DUPLICATE_KEY_ORDER')
            order_seen.add(pointer)
            if pointer=='':node=value
            else:
                container,key=parent(value,pointer)
                node=container[int(key)] if isinstance(container,list) else container[key]
            keys=order['keys']
            if not isinstance(node,dict) or len(keys)!=len(set(keys)) or set(keys)!=set(node):raise ValueError('OVERLAY_KEY_ORDER')
            saved=dict(node);node.clear();node.update((k,saved[k]) for k in keys)
        output=encode(value,f['encoding'])
        if digest(output)!=f['output_sha256']:raise ValueError('OVERLAY_OUTPUT_DIGEST:'+path)
        result[path]=output
    return result
