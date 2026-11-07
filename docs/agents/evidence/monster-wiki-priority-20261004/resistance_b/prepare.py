from pathlib import Path
from fractions import Fraction
import json,gzip,re,hashlib,subprocess,collections
out=Path('/tmp/monster-wiki-priority-20261004/resistance_b');inp=Path('/tmp/monster-wiki-priority-20261004/resistance_b.json');rows=json.loads(inp.read_text())['rows']
cache=Path('/workspace/monster-field-audit-20261002/wiki');pagespath=cache/'pages.json.gz';covpath=cache/'actor-page-coverage.json.gz';pages={p['page_title']:p for p in json.load(gzip.open(pagespath,'rt'))['pages']};coverage={p['monster']:p for p in json.load(gzip.open(covpath,'rt'))['actors']}
keys={'physical':'physicalDmgMod','earth':'earthDmgMod','fire':'fireDmgMod','death':'deathDmgMod','energy':'energyDmgMod','holy':'holyDmgMod','ice':'iceDmgMod','life_drain':'hpDrainDmgMod','drowning':'drownDmgMod','mana_drain':'manaDrainDmgMod'}
ctypes={'PHYSICALDAMAGE':'physical','EARTHDAMAGE':'earth','FIREDAMAGE':'fire','DEATHDAMAGE':'death','ENERGYDAMAGE':'energy','HOLYDAMAGE':'holy','ICEDAMAGE':'ice','LIFEDRAIN':'life_drain','DROWNDAMAGE':'drowning','MANADRAIN':'mana_drain'}
def rat(x):return Fraction(x['numerator'],x['denominator'])
def enc(x):return {'denominator':x.denominator,'numerator':x.numerator}
def number(s):
 if not isinstance(s,str):return None
 m=re.fullmatch(r'\s*([+-]?\d+(?:[.,]\d+)?)\s*%?\s*',s)
 return Fraction(m[1].replace(',','.')) if m else None
proposals=[];counts=collections.Counter();els=collections.Counter();changedwiki=[]; nonres=[];shards={}
canonroot=Path("/workspace/monster-main-reconciliation-20261004")
for r in rows:
 actor=r['target']['key'].split('.')[-1];cov=coverage.get(actor);page=pages.get(cov.get('resolved_title')) if cov else None
 if page and (cov.get('page_revision_id')!=page['revision_id'] or cov.get('page_content_sha256')!=page['content_sha256']):page=None
 shardpath=canonroot/r['canonical_path']
 if str(shardpath) not in shards: shards[str(shardpath)]=json.loads(shardpath.read_text())
 record=next(z for z in shards[str(shardpath)]['records'] if z['definition']['identity']['key']==r['target']['key'])
 profile=record['authoring']['profile']; immunity=set(profile.get('immunities',[])); healing={x['damage_type']:rat(x['percent']) for x in profile.get('details',{}).get('healing_from_damage',[])}
 dirty_phase=actor=='the_sinister_hermit_dirty'
 current={x['damage_type']:rat(x['percent']) for x in r['canonical'] or []};new=current.copy();proofs=[];sourcevals={};source_receipt=None
 for binding in r.get('source_bindings',[]):
  repo='canary' if binding['source_key'].endswith('canary') else 'crystal';namespace=binding['identity_namespace']
  if 'monster-file' not in namespace:continue
  path='data-otservbr-global/monster/'+binding['external_id']+'.lua';rev=binding['source_revision']
  proc=subprocess.run(['git','show',rev+':'+path],cwd='/workspace/monster-reference-sources/'+repo,env={'PATH':'/usr/bin:/bin','GIT_NO_LAZY_FETCH':'1'},capture_output=True)
  if proc.returncode:continue
  raw=proc.stdout.decode();m=re.search(r'monster\.elements\s*=\s*\{(.*?)\n\}',raw,re.S)
  if m:
   for typ,value in re.findall(r'type\s*=\s*COMBAT_([A-Z]+)\s*,\s*percent\s*=\s*([+-]?\d+(?:\.\d+)?)',m[1]):
    if typ in ctypes:sourcevals[ctypes[typ]]=Fraction(value)
  source_receipt={'repository':repo,'revision':rev,'path':path,'sha256':hashlib.sha256(proc.stdout).hexdigest()};break
 for element,key in keys.items():
  raw=page['fields'].get(key) if page else None;taken=number(raw)
  if dirty_phase: taken=None
  if taken is not None:
   value=100-taken
   if taken < 0:
    nonres.append({'actor':actor,'damage_type':element,'field':'details.healing_from_damage','wiki_damage_taken':raw,'wiki_healing_percent':enc(-taken),'canonical_healing_percent':enc(healing[element]) if element in healing else None,'canonical_path':r['canonical_path'],'revision_id':page['revision_id'],'content_sha256':page['content_sha256'],'field_line':page['field_lines'].get(key),'disposition':'PRESERVE_RESISTANCE_AND_HANDOFF_REAL_HEALING_RESPONSE_NO_PERCENT_ABOVE100'})
   elif value==100 and element in immunity:
    pass # Canonical immunity is already the exact100 response; no duplicate resistance.
   elif value != 0 or element in current: new[element]=value
   status='WIKI_CONFIRMED';proof={'wiki_page':page['page_title'],'url':page['url'],'revision_id':page['revision_id'],'content_sha256':page['content_sha256'],'field':key,'line':page['field_lines'].get(key),'raw_damage_taken':raw,'resistance':enc(value),'method':page['method'],'cache':str(pagespath)}
   if taken < 0:
    status='SOURCE_ONLY' if source_receipt and element in current and current[element]==sourcevals.get(element,Fraction(0)) else 'UNKNOWN'
    proof['wiki_observation_status']='WIKI_CONFIRMED_NEGATIVE_DAMAGE_RESPONSE'
    proof['not_applied_to_resistance']=True
    proof['new_resistance']=enc(new[element]) if element in new else None
    proof['source']=source_receipt if status=='SOURCE_ONLY' else None
   if taken>=0 and (100 if element in immunity else current.get(element,Fraction(0)))!=value:changedwiki.append({'actor':actor,'element':element,'canonical':enc(current.get(element,Fraction(0))),'wiki':enc(value)})
  elif element in current:
   value=current[element];status='SOURCE_ONLY' if source_receipt and value==sourcevals.get(element,Fraction(0)) else 'UNKNOWN';proof={'cache_field':key,'raw':raw,'wiki_not_confirmed_reason':'absent or non-exact/uncertain numeric value','source':source_receipt if status=='SOURCE_ONLY' else None,'source_value':enc(sourcevals.get(element,Fraction(0))) if source_receipt else None}
  else:
   status='UNKNOWN';proof={'cache_field':key,'raw':raw,'wiki_not_confirmed_reason':'absent or non-exact/uncertain value; no new resistance invented'}
  proofs.append({'damage_type':element,'status':status,'present_in_new':element in new,'proof':proof});els[status]+=1
 # Preserve canonical representation for unconfirmed elements. Confirmed neutral values preserve existing explicit0/absent representation.
 newlist=[{'damage_type':k,'percent':enc(v)} for k,v in sorted(new.items())]
 represented=[p['status'] for p in proofs if p['present_in_new']]
 provenance='WIKI_CONFIRMED' if represented and all(p=='WIKI_CONFIRMED' for p in represented) else 'SOURCE_ONLY' if represented and all(p!='UNKNOWN' for p in represented) else 'UNKNOWN'
 counts[provenance]+=1
 proposals.append({'target':r['target'],'field':'resistances','old_present':r['old_present'],'old':r['old'],'new_present':r['canonical_present'] or bool(newlist),'new':newlist,'canonical_present':r['canonical_present'],'canonical':r['canonical'],'canonical_path':r['canonical_path'],'provenance':provenance,'element_provenance':proofs,'wiki_confirmed_elements':sum(p['status']=='WIKI_CONFIRMED' for p in proofs),'source_receipt':source_receipt,'canonical_immunities':sorted(immunity),'canonical_shard_sha256':hashlib.sha256(shardpath.read_bytes()).hexdigest(),'wiki_phase_identity_confirmed':not dirty_phase,'recommendation':'APPLY_WIKI_CONFIRMED_VALUES_AND_PRESERVE_UNCONFIRMED_CANONICAL','neutral_semantics':'Native absent damage-type lookup is0 reduction; explicit0 is equivalent. Only strict wiki damageTaken100 gives confirmed neutral; preserve absent representation if already absent. Absent wiki is UNKNOWN, never confirmation.'})
packet={'schema':'monster-wiki-priority-resistance-b-v1','input_sha256':hashlib.sha256(inp.read_bytes()).hexdigest(),'rows':proposals,'canonical_changes':changedwiki,'non_resistance_wiki_findings':nonres,'semantics':'percent is reduction:100−wiki damageTaken; donor elements percent already reduction. Signed ratios retained exactly. No uncertainty markers stripped.','authority':'Root only product writer; proposal data only.'}
(out/'proposal.json').write_text(json.dumps(packet,indent=2)+'\n');(out/'summary.json').write_text(json.dumps({'actors':len(rows),'list_provenance':counts,'element_provenance':els,'wiki_changes_vs_canonical':changedwiki,'non_resistance_wiki_findings':nonres,'wiki_changes_count':len(changedwiki),'cache_paths':[str(pagespath),str(covpath)],'cache_sha256':hashlib.sha256(pagespath.read_bytes()).hexdigest(),'source_verified':sum(p['source_receipt'] is not None for p in proposals),'neutral_canonical_lists':sum(any(x['percent']['numerator']==0 for x in p['canonical'] or []) for p in proposals),'network':False,'product_edits':False},indent=2)+'\n')
print(json.dumps({'counts':counts,'elements':els,'wiki_changes':len(changedwiki),'source_verified':sum(p['source_receipt'] is not None for p in proposals),'sha256':hashlib.sha256((out/'proposal.json').read_bytes()).hexdigest()}))
