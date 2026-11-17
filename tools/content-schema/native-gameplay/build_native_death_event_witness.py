"""Capture exact ordered W01 onDeath registrations from pinned donor Lua.

All parent profiles, actual canonical identity bindings, registrations and Lua
body digests are frozen by EXPECTED_PARENT_ROWS_SHA256. A new canonical root
alone cannot bless changed source semantics. Existing source caches are read
only; no engine or tool installation is required.
"""
from pathlib import Path
import argparse, hashlib, json, re
import death_event_source as p
ROOT=Path(__file__).resolve().parents[3]
EXPECTED_PARENT_ROWS_SHA256='537937e6c0322cf066a127887995c8692e2bd0bb328e8af967b34c81d20be625'
def canonical(value):return json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
def sha(raw):return hashlib.sha256(raw).hexdigest()
def source_bytes(cache,blob):
 raw=(cache/(blob+'.lua')).read_bytes()
 if hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=blob:raise ValueError('source Git blob mismatch')
 return raw

def build(cache:Path,tree_path:Path):
 audit=json.loads((ROOT/'docs/agents/evidence/srx3-creature-event-audit.json').read_bytes())
 provenance=json.loads((ROOT/'content/world/provenance/sources.json').read_bytes())
 native=json.loads((ROOT/'content/creatures/definitions/spell-native-profiles.json').read_bytes())
 main=json.loads((ROOT/'crates/gameplay/content/src/content/native_monster_callback_witness.json').read_bytes())
 if native['source_definitions_sha256']!=main['source_definitions_sha256']:raise ValueError('source root mismatch')
 records={}
 for row in native['records']:
  target=row['profile']['target']
  if target['key'] in records:raise ValueError('duplicate native target')
  records[target['key']]=row
 sources_by_donor={};creatures=[];seen_names=set()
 for donor in audit['donors']:
  sources={}
  for event in donor['events']:
   if len(event['sources'])!=1:raise ValueError('ambiguous source event')
   source=event['sources'][0];raw=source_bytes(cache,source['git_blob'])
   if sha(raw)!=source['sha256']:raise ValueError('event source SHA mismatch')
   sources[event['name']]=(source['path'],raw)
  sources_by_donor[donor['repository']]=sources
  for creature in donor['creatures']:
   if creature['name'].lower() in seen_names:continue
   seen_names.add(creature['name'].lower());creatures.append((donor,creature))
 # Literal named guard/STEP context extends actual room consequences, not the
 # generic progression-table-name set. No registered source is inferred.
 tree=json.loads(tree_path.read_bytes())['tree']
 wanted={'shadow_tentacle','possessed_tree','unbound_blightwalker','unbound_demon','unbound_demon_outcast','unbound_defiler','dragonking_zyrtarch','soul_of_dragonking_zyrtarch'}
 canary=audit['donors'][0]
 for node in tree:
  if not node['path'].startswith('data-otservbr-global/monster/quests/forgotten_knowledge/'):continue
  if Path(node['path']).stem not in wanted:continue
  raw=source_bytes(cache,node['sha']);text=raw.decode()
  match=re.search(r'Game\.createMonsterType\("([^"\n]+)"\)',text)
  if not match:raise ValueError('missing literal context type')
  name=match[1]
  if name.lower() in seen_names:continue
  seen_names.add(name.lower());creatures.append((canary,{'name':name,'path':node['path'],'git_blob':node['sha'],'sha256':sha(raw)}))
 extra={'name':'PossessedTree','path':'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_possessed_tree.lua','git_blob':'708e04d878e5e9a995261a852066779f64f6adfe'}
 sources_by_donor['opentibiabr/canary'][extra['name']]=(extra['path'],source_bytes(cache,extra['git_blob']))
 rows=[];guards={}
 for donor,creature in creatures:
  raw=source_bytes(cache,creature['git_blob'])
  if sha(raw)!=creature['sha256']:raise ValueError('monster SHA mismatch')
  # Source registration must actually execute one explicit type register.
  p.monster_registrations(raw.decode(),require_registration=True)
  parsed=p.ordered_death_events(raw,sources_by_donor[donor['repository']])
  if donor['repository']=='opentibiabr/canary':
   namespace='canary/monster-file';external=creature['path'].removeprefix('data-otservbr-global/monster/').removesuffix('.lua')
  else:namespace='crystalserver/monster-file';external=creature['path']
  bindings=[binding for binding in provenance['source_identity_bindings']
      if binding['identity_namespace']==namespace and binding['external_id']==external
      and binding['target']['family']=='Creature' and binding['target']['key'].startswith('oteryn:')]
  if len(bindings)!=1:raise ValueError(('missing/ambiguous ACTUAL canonical source binding',creature['name'],bindings))
  binding=bindings[0];record=records.get(binding['target']['key'])
  if record is None or record['profile']['target']!=binding['target']:raise ValueError('missing actual bound native profile')
  if binding['disposition']!='EXACT':raise ValueError('source identity mapping is not exact')
  key=binding['target']['key']
  if key in guards:raise ValueError('duplicate covered identity')
  # These fingerprints must be frozen in the reviewed official producer;
  # regeneration cannot silently bless changed native Creature semantics.
  guards[key]=sha(canonical({'profile':record['profile'],'source_identity_binding':binding}))
  rows.append({'native_profile':record['profile'],'source_identity_binding':binding,
       'monster_source':{'repository':donor['repository'],'revision':donor['revision'],
         'path':creature['path'],'git_blob':creature['git_blob'],'sha256':sha(raw)},
       'registered_events':parsed['registered_events'],'on_death':parsed['on_death']})
 rows.sort(key=lambda row:row['native_profile']['target']['key'])
 result={'schema':'OTERYN_W01_ORDERED_NATIVE_DEATH_EVENTS/v1',
    'source_definitions_sha256':native['source_definitions_sha256'],'parents':rows}
 if sha(canonical(rows))!=EXPECTED_PARENT_ROWS_SHA256:
  raise ValueError('ordered source registration/native profile semantics changed; review required before guard update')
 return result

def outputs(result:dict,output:Path):
 raw=(json.dumps(result,ensure_ascii=False,sort_keys=True,indent=2)+'\n').encode()
 keys=[row['native_profile']['target']['key'] for row in result['parents']]
 pin=('// Generated by build_native_death_event_witness.py; do not edit.\n'
      +'pub(super) const WITNESS_SHA256: [u8;32] = '+str(list(hashlib.sha256(raw).digest()))+';\n'
      +'pub(super) const COVERED_KEYS: &[&str] = &[\n'
      +''.join('    '+json.dumps(key)+',\n' for key in keys)+'];\n').encode()
 return {output:raw,output.with_name('native_w01_death_events_pin.rs'):pin}

def refresh_native_pin(path:Path):
 """Rebind the frozen source capture only when all selected native bodies match."""
 result=json.loads(path.read_bytes())
 if result.get('schema')!='OTERYN_W01_ORDERED_NATIVE_DEATH_EVENTS/v1' or sha(canonical(result['parents']))!=EXPECTED_PARENT_ROWS_SHA256:
  raise ValueError('ordered source registration/native profile semantics changed')
 native=json.loads((ROOT/'content/creatures/definitions/spell-native-profiles.json').read_bytes())
 main=json.loads((ROOT/'crates/gameplay/content/src/content/native_monster_callback_witness.json').read_bytes())
 if main['source_definitions_sha256']!=native['source_definitions_sha256']:
  raise ValueError('source root mismatch')
 profiles={row['profile']['target']['key']:row['profile'] for row in native['records']}
 bindings={row['definition']['identity']['key']:row['source_bindings']
           for shard in (ROOT/'content/creatures/definitions').glob('creatures-*.json')
           for row in json.loads(shard.read_bytes())['records']}
 for row in result['parents']:
  key=row['native_profile']['target']['key']
  if profiles.get(key)!=row['native_profile'] or row['source_identity_binding'] not in bindings.get(key,[]):
   raise ValueError('ordered death native body or source binding changed: '+key)
 result['source_definitions_sha256']=native['source_definitions_sha256']
 return result

def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--source-cache',type=Path,default=Path('/tmp/srx3-lua-audit/blobs'))
 parser.add_argument('--canary-tree',type=Path,default=Path('/tmp/srx-canary-tree.json'))
 parser.add_argument('--out',type=Path,default=ROOT/'crates/gameplay/content/src/content/native_w01_death_events_witness.json')
 parser.add_argument('--check',action='store_true')
 parser.add_argument('--refresh-native-pin',action='store_true')
 args=parser.parse_args();result=refresh_native_pin(args.out) if args.refresh_native_pin else build(args.source_cache,args.canary_tree)
 for path,raw in outputs(result,args.out).items():
  if args.check:
   if not path.exists() or path.read_bytes()!=raw:raise SystemExit('stale source capture: '+str(path))
  else:path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
 print('ordered W01 source parents='+str(len(result['parents']))+' onDeath='+str(sum(bool(row['on_death']) for row in result['parents'])))
if __name__=='__main__':main()
