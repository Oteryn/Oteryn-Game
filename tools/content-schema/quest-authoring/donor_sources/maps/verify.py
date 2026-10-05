#!/usr/bin/env python3
"""Portable exact map capture qualification. Cache verification is explicit."""
import argparse,gzip,hashlib,json,struct
from pathlib import Path
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
HERE=Path(__file__).resolve().parent

def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        while chunk:=f.read(1024*1024):h.update(chunk)
    return h.hexdigest()
def local_file(root,relative):
    p=Path(relative)
    if p.is_absolute() or '..' in p.parts or not p.parts:raise ValueError('unsafe relative path')
    target=(root/p).resolve()
    if not target.is_relative_to(root.resolve()):raise ValueError('path escapes root')
    return target

def cache_file(relative,cache_root,blob_root,index_root):
    p=Path(relative)
    if p.is_absolute() or '..' in p.parts:raise ValueError('unsafe cache path')
    if p.parts[:1]==('source-blobs',) and len(p.parts)==2:
        return local_file(blob_root,p.name) if blob_root else local_file(cache_root,relative)
    if p.parts[:2]==('complete-source-maps','node-indexes') and len(p.parts)==3:
        return local_file(index_root,p.name) if index_root else local_file(cache_root,relative)
    raise ValueError('unrecognized cache layout')

def qualify(manifest_path,cache_root=None,blob_root=None,index_root=None,structural_only=False):
    root=manifest_path.resolve().parent;m=json.loads(manifest_path.read_text())
    if m.get('schema')!='OTERYN_PORTABLE_SOURCE_MAP_QUALIFICATION/v1':raise ValueError('wrong qualification namespace')
    if set(m)!={'schema','files','receipt','archives','archive_members','acquisitions','summary','semantics_complete','native_promotion','runtime_enabled'}:raise ValueError('unexpected qualification fields')
    if any(m[k] is not False for k in ('semantics_complete','native_promotion','runtime_enabled')):raise ValueError('unsafe promotion')
    for r in m['files']:
        if digest(local_file(root,r['path']))!=r['sha256']:raise ValueError('thin input digest mismatch')
    receipt_path=local_file(root,m['receipt']['path'])
    if digest(receipt_path)!=m['receipt']['sha256']:raise ValueError('receipt digest mismatch')
    receipt=json.loads(receipt_path.read_text());schema=json.loads((root/'source_map_structural.schema.json').read_text())
    Draft202012Validator(schema).validate(receipt)
    for r in receipt['records']:
        if r['artifact_path']!='source-blobs/'+r['artifact_sha256']:raise ValueError('wrong artifact path identity')
        if r['status']!='STRUCTURAL_AST_COMPLETE':raise ValueError('structural hold remains')
        if r['index_path']!='complete-source-maps/node-indexes/'+r['artifact_sha256']+'.nodes.gz':raise ValueError('wrong index path identity')
        if r['decoded_stream_bytes']!=r['decoded_bytes'] or sum(r['types'].values())!=r['node_count']:raise ValueError('wrong structural counts')
    if len({r['artifact_sha256'] for r in receipt['records']})!=len(receipt['records']):raise ValueError('duplicate map stream')
    if len(receipt['records'])!=m['summary']['unique_otbm_streams'] or sum(r['node_count'] for r in receipt['records'])!=m['summary']['unique_stream_nodes']:raise ValueError('summary mismatch')
    normalized=json.loads((root/'inputs.normalized.json').read_text())
    if any(m[k]!=normalized[k] for k in ('archives','archive_members','acquisitions')):raise ValueError('normalized input projection mismatch')
    if structural_only:return {'valid':True,'mode':'STRUCTURAL_ONLY','cache_provenance':'NOT_VERIFIED','runtime_enabled':False}
    if not cache_root and not (blob_root and index_root):raise ValueError('supply --cache-root or both explicit cache directories')
    checked={}
    def check_blob(relative,expected):
        p=cache_file(relative,cache_root,blob_root,index_root)
        if relative not in checked:
            if digest(p)!=expected:raise ValueError('source bytes mismatch')
            checked[relative]=p
        return p
    for r in receipt['records']:
        p=check_blob(r['artifact_path'],r['artifact_sha256'])
        with p.open('rb') as f:magic=f.read(2)
        h=hashlib.sha256();count=0
        with (gzip.open if magic==b'\x1f\x8b' else open)(p,'rb') as f:
            while chunk:=f.read(1024*1024):h.update(chunk);count+=len(chunk)
        if h.hexdigest()!=r['decoded_stream_sha256'] or count!=r['decoded_stream_bytes']:raise ValueError('decoded source mismatch')
        ip=cache_file(r['index_path'],cache_root,blob_root,index_root)
        if digest(ip)!=r['index_sha256'] or ip.stat().st_size!=r['index_byte_count']:raise ValueError('index mismatch')
        count=0
        with gzip.open(ip,'rb') as f:
            while chunk:=f.read(1024*1024):count+=len(chunk)
        if count!=r['node_count']*24:raise ValueError('index record count mismatch')
        for ref in r['source_refs']:
            if 'git_blob_sha1' not in ref or 'member_path' in ref:continue
            gh=hashlib.sha1(b'blob '+str(p.stat().st_size).encode()+b'\0')
            with p.open('rb') as f:
                while chunk:=f.read(1024*1024):gh.update(chunk)
            if gh.hexdigest()!=ref['git_blob_sha1']:raise ValueError('Git map identity mismatch')
    for r in m['archives']+m['archive_members']:
        p=check_blob(r['cache_relative_path'],r['sha256'])
        if p.stat().st_size!=r['bytes']:raise ValueError('archive/member size mismatch')
        if 'source_ref' in r:
            gh=hashlib.sha1(b'blob '+str(p.stat().st_size).encode()+b'\0')
            with p.open('rb') as f:
                while chunk:=f.read(1024*1024):gh.update(chunk)
            if gh.hexdigest()!=r['source_ref']['git_blob_sha1']:raise ValueError('Git archive identity mismatch')
    for r in m['acquisitions']:
        p=check_blob(r['cache_relative_path'],r['sha256'])
        if p.stat().st_size!=r['byte_count']:raise ValueError('acquisition size mismatch')
        if r.get('expected_sha256',r['sha256'])!=r['sha256']:raise ValueError('release map hash mismatch')
        if r.get('status')!='VERIFIED_BYTES':raise ValueError('unqualified acquisition')
        if 'git_blob_sha1' in r:
            gh=hashlib.sha1(b'blob '+str(p.stat().st_size).encode()+b'\0')
            with p.open('rb') as f:
                while chunk:=f.read(1024*1024):gh.update(chunk)
            if gh.hexdigest()!=r['git_blob_sha1']:raise ValueError('acquisition Git identity mismatch')
    return {'valid':True,'mode':'SOURCE_BACKED','cache_provenance':'VERIFIED_EXACT_LOCAL_BYTES','maps':len(receipt['records']),'nodes':m['summary']['unique_stream_nodes'],'runtime_enabled':False}

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--manifest',type=Path,default=HERE/'qualification.json');ap.add_argument('--cache-root',type=Path);ap.add_argument('--blob-root',type=Path);ap.add_argument('--index-root',type=Path);ap.add_argument('--structural-only',action='store_true');a=ap.parse_args()
    try:print(json.dumps(qualify(a.manifest,a.cache_root,a.blob_root,a.index_root,a.structural_only)))
    except (ValueError,OSError,KeyError,ValidationError,gzip.BadGzipFile) as ex:ap.exit(1,str(ex)+'\n')
if __name__=='__main__':main()
