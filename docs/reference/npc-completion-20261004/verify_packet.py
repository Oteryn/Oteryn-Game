"""Verify the complete review packet without extraction, build, network or donor caches."""
from pathlib import Path
import hashlib,json,tarfile
ROOT=Path(__file__).resolve().parent
manifest=json.loads((ROOT/'manifest.json').read_text())
archive=ROOT/manifest['archive']
def digest_file(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
assert digest_file(archive)==manifest['archive_sha256'], 'archive hash'
expected={r['path']:r for r in manifest['entries']};seen=set();worlds={k:[] for k in manifest['world_trees']}
with tarfile.open(archive,'r:*') as tar:
 for member in tar:
  name=member.name
  assert member.isfile() and name in expected and name not in seen, 'unexpected/duplicate/non-file member'
  assert not Path(name).is_absolute() and '..' not in Path(name).parts, 'unsafe member path'
  seen.add(name);record=expected[name];stream=tar.extractfile(member);h=hashlib.sha256();size=0
  parts=[] if name.split('/',1)[0] in worlds else None
  for block in iter(lambda:stream.read(1024*1024),b''):
   h.update(block);size+=len(block)
   if parts is not None:parts.append(block)
  assert size==record['bytes']==member.size and h.hexdigest()==record['sha256'], name
  if parts is not None:worlds[name.split('/',1)[0]].append((name.split('/',1)[1],b''.join(parts)))
assert seen==set(expected), 'missing member'
for world,records in worlds.items():
 h=hashlib.sha256()
 for locator,raw in sorted(records):
  encoded=locator.encode();h.update(len(encoded).to_bytes(8,'big'));h.update(encoded);h.update(len(raw).to_bytes(8,'big'));h.update(raw)
 assert h.hexdigest()==manifest['world_trees'][world], world
print(json.dumps({'status':'PASS_PACKET_HASHES_AND_WORLD_TREES','files':len(seen),'runtime_activated':False}))
