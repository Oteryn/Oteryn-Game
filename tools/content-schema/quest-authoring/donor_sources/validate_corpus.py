"""Offline independent donor corpus byte/inventory validation; never semantic admission."""
import argparse,hashlib,json,pathlib
from jsonschema import Draft202012Validator
D=pathlib.Path(__file__).resolve().parent
TEXT={'.lua','.xml','.json','.toml','.cpp','.hpp','.h','.c','.sql','.conf','.cfg','.ini','.txt','.yaml','.yml','.cmake','.proto','.inc'}
PINS={'canary':('opentibiabr/canary','04b83b512114bfd888000d6e1433ed8ecaec7c5b'),'crystalserver':('zimbadev/crystalserver','9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'),'crystal-summer':('zimbadev/crystalserver','00ce02a57ca5a12e48f32a3476e37471167e4c3f')}
def sha(raw):return hashlib.sha256(raw).hexdigest()
def relevant(path):
 parts=path.split('/');p=pathlib.PurePosixPath(path);return (parts[0].startswith('data') or parts[0] in ('src','scripts','config') or len(parts)==1) and (p.suffix.lower() in TEXT or p.name.endswith(('.lua.dist','.xml.dist')))

def safe_path(value):
 p=pathlib.PurePosixPath(value)
 if p.is_absolute() or '..' in p.parts or str(p)!=value or '\\' in value:raise ValueError('unsafe donor path: '+value)
 return p

def validate(manifest, base_dir=None):
 base_dir=pathlib.Path(base_dir or pathlib.Path.cwd()).resolve()
 def local(value):
  path=pathlib.Path(value)
  return path.resolve() if path.is_absolute() else (base_dir/path).resolve()
 Draft202012Validator(json.loads((D/'donor-corpus.schema.json').read_bytes())).validate(manifest)
 sources={s['source']:s for s in manifest['sources']}
 if len(sources)!=3 or set(sources)!=set(PINS):raise ValueError('duplicate or missing donor source')
 expected={};excluded={};trees=[];full_inventory={}
 for name,s in sources.items():
  if (s['repository'],s['revision'])!=PINS[name]:raise ValueError('pinned donor revision substitution')
  raw=local(s['git_tree_path']).read_bytes()
  if sha(raw)!=s['git_tree_sha256']:raise ValueError('stale Git tree bytes')
  tree=json.loads(raw)
  if tree.get('truncated') is not False:raise ValueError('Git tree is not proven non-truncated')
  blobs=[v for v in tree['tree'] if v['type']=='blob'];full_inventory.update({(name,v['path']):v for v in blobs});selected=[v for v in blobs if relevant(v['path'])]
  if len(selected)!=s['git_tree_selected_files']:raise ValueError('selected tree count differs')
  if len(blobs)!=s['git_tree_all_blobs']:raise ValueError('full tree blob count differs')
  if len({v['path'] for v in blobs})!=len(blobs):raise ValueError('duplicate tree paths')
  for v in selected:safe_path(v['path']);expected[(name,v['path'])]=v
  excluded[name]={'tree_blobs':len(blobs),'outside_selected_scope':len(blobs)-len(selected)}
  trees.append({'source':name,'git_tree_sha256':sha(raw),'tree_sha':tree.get('sha'),'selected_files':len(selected)})
 seen=set();read_cache={};captured_by_source={k:0 for k in sources}
 for f in manifest['files']:
  name=f['source'];s=sources[name];key=(name,f['path'])
  if f.get('file_id') and f['file_id']!=name+':'+s['revision']+':'+f['path']:raise ValueError('file identity differs from source/revision/path')
  if key in seen:raise ValueError('duplicate corpus file')
  seen.add(key)
  if key not in expected:raise ValueError('file outside scoped Git tree')
  if (f['repository'],f['revision'])!=(s['repository'],s['revision']):raise ValueError('cross-donor revision substitution')
  safe_path(f['path']);path=local(f['cache_path'])
  if not path.is_relative_to(local(s['corpus_root'])):raise ValueError('corpus path escapes root')
  # Read each actual destination. CAS metadata cannot prove existing destinations contain identical bytes.
  raw=path.read_bytes();gitsha=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
  if gitsha!=expected[key]['sha'] or gitsha!=f['git_blob_sha1']:raise ValueError('Git blob byte mismatch: '+str(key))
  if len(raw)!=f['byte_count'] or expected[key].get('size',len(raw))!=len(raw):raise ValueError('byte count mismatch')
  if sha(raw)!=f['sha256']:raise ValueError('raw SHA256 mismatch')
  captured_by_source[name]+=1
 failures={(v['source'],v['path']) for v in manifest['failures']}
 if len(failures)!=len(manifest['failures']) or failures&seen or failures!=set(expected)-seen:raise ValueError('missing file accounting differs')
 summary={'selected_git_tree_files':len(expected),'captured_files':len(seen),'missing_files':len(failures),'unique_git_blobs':len({v['git_blob_sha1'] for v in manifest['files']}),'excluded_git_blobs':sum(v['outside_selected_scope'] for v in excluded.values())}
 if summary!=manifest['summary']:raise ValueError('corpus summary differs from verified membership')
 license_keys=set()
 for license in manifest.get('licenses',[]):
  name=license['source'];entry=full_inventory.get((name,license['source_path']));path=local(license['archive_path'])
  if not path.is_relative_to(base_dir):raise ValueError('license path escapes artifact')
  if name in license_keys or entry is None:raise ValueError('license identity missing or duplicated')
  license_keys.add(name);raw=path.read_bytes();gitsha=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
  if (license['repository'],license['revision'])!=PINS[name] or gitsha!=entry['sha'] or gitsha!=license['git_blob_sha1'] or sha(raw)!=license['sha256']:raise ValueError('license identity/bytes differ')
 if manifest.get('portable_layout') and license_keys!=set(PINS):raise ValueError('portable corpus lacks source licenses')
 return {'result':'PASS_LOSSLESS_SELECTED_SCOPE' if not failures else 'PASS_EXPLICIT_INCOMPLETE_CAPTURE','summary':summary,'captured_by_source':captured_by_source,'excluded_scope':excluded,'trees':trees,'raw_bytes_verified':True,'semantic_transcription':'NOT_ASSESSED','dependency_closure':'NOT_ASSESSED','canonical_admission':'NOT_ASSESSED','summer_source_role':'COMPARISON_ONLY_NOT_CANONICAL_REVISION_SUBSTITUTION','source_1_to_1_quest_completion':False}

def main():
 p=argparse.ArgumentParser();p.add_argument('manifest',type=pathlib.Path);p.add_argument('--output',type=pathlib.Path);a=p.parse_args();raw=a.manifest.read_bytes();r=validate(json.loads(raw),a.manifest.parent);r['manifest_sha256']=sha(raw)
 if a.output:a.output.write_text(json.dumps(r,indent=2)+'\n')
 print(json.dumps(r,indent=2))
if __name__=='__main__':main()
