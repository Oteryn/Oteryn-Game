"""Validate a self-contained source research archive offline without execution."""
import argparse,hashlib,json,pathlib,tarfile,tempfile
try:
 from .validate_corpus import validate,relevant,PINS,safe_path
except ImportError:
 from validate_corpus import validate,relevant,PINS,safe_path

def verify_archive(archive,expected_sha=None):
 digest=hashlib.sha256(archive.read_bytes()).hexdigest()
 if expected_sha and digest!=expected_sha:raise ValueError('archive digest differs')
 with tempfile.TemporaryDirectory(prefix='donor-source-verify-') as tmp:
  base=pathlib.Path(tmp)
  with tarfile.open(archive,'r:gz') as tar:
   members=tar.getmembers();names=[m.name for m in members]
   if len(set(names))!=len(names):raise ValueError('duplicate archive member')
   if len(members)>50000 or sum(m.size for m in members)>512*1024*1024:raise ValueError('archive outside finite source scope')
   for m in members:
    safe_path(m.name)
    if not m.isfile():raise ValueError('archive member is not ordinary source file')
    dest=base/m.name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(tar.extractfile(m).read())
  manifest_raw=(base/'corpus-manifest.json').read_bytes();manifest=json.loads(manifest_raw)
  if not manifest.get('portable_layout'):raise ValueError('archive manifest is not portable')
  for source in manifest['sources']:
   safe_path(source['corpus_root']);safe_path(source['git_tree_path'])
  for value in [f['cache_path'] for f in manifest['files']]+[l['archive_path'] for l in manifest['licenses']]:safe_path(value)
  report=validate(manifest,base)
  allowed={'README.txt','corpus-manifest.json','excluded-git-blobs.json','corpus-summary.json'}|{f['cache_path'] for f in manifest['files']}|{s['git_tree_path'] for s in manifest['sources']}|{l['archive_path'] for l in manifest['licenses']}
  if set(names)!=allowed:raise ValueError('archive member inventory differs')
  expected_excluded={}
  for s in manifest['sources']:
   tree=json.loads((base/s['git_tree_path']).read_bytes())
   for f in tree['tree']:
    if f['type']=='blob' and not relevant(f['path']):expected_excluded[(s['source'],f['path'])]=(s['repository'],s['revision'],f['sha'],f.get('size'))
  excluded=json.loads((base/'excluded-git-blobs.json').read_bytes());got={}
  for f in excluded['files']:
   key=(f['source'],f['path'])
   if key in got:raise ValueError('duplicate excluded member')
   got[key]=(f['repository'],f['revision'],f['git_blob_sha1'],f['byte_count'])
  if got!=expected_excluded:raise ValueError('excluded tree complement differs')
  report.update(archive_sha256=digest,archive_members=len(members),archive_bytes=archive.stat().st_size,manifest_sha256=hashlib.sha256(manifest_raw).hexdigest(),source_licenses_verified=len(manifest['licenses']),portable_standalone=True)
  return report

def main():
 p=argparse.ArgumentParser();p.add_argument('archive',type=pathlib.Path);p.add_argument('--expected-sha256');p.add_argument('--output',type=pathlib.Path);a=p.parse_args();r=verify_archive(a.archive,a.expected_sha256)
 if a.output:a.output.write_text(json.dumps(r,indent=2)+'\n')
 print(json.dumps(r,indent=2))
if __name__=='__main__':main()
