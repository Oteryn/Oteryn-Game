import json,gzip,pathlib,hashlib,re,collections,subprocess
from fractions import Fraction
OUT=pathlib.Path(__file__).resolve().parent;ROOT=pathlib.Path('/workspace/monster-main-reconciliation-20261004');IN=OUT.parent/'resistance_a.json';inputs={}
def load(p,gz=False):
 p=pathlib.Path(p);b=p.read_bytes();inputs[str(p)]={'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)};return json.loads(gzip.decompress(b)if gz else b)
data=load(IN);wiki_path=pathlib.Path('/workspace/monster-field-audit-20261002/wiki/pages.json.gz');pages=load(wiki_path,True)['pages'];wiki={p['page_title']:p for p in pages}
samples=pathlib.Path('/workspace/monster-completion/tools/content-schema/monster-authoring/samples');titles={}
for f in ['wiki-population-2026-09-27.json','wiki-population-crystal-00ce02a5-2026-09-27.json','canary-47dfd51f/wiki-2026-09-27.json']:
 for row in load(samples/f).get('monsters',[]):
  if row.get('wiki_title'):titles[row['monster']]=(row['wiki_title'],str(samples/f))
canon={}
for rel in sorted({r['canonical_path']for r in data['rows']}):
 for r in load(ROOT/rel)['records']:canon[r['definition']['identity']['key']]=r
keys={'physical':'physicalDmgMod','earth':'earthDmgMod','fire':'fireDmgMod','death':'deathDmgMod','energy':'energyDmgMod','holy':'holyDmgMod','ice':'iceDmgMod','life_drain':'hpDrainDmgMod','mana_drain':'manaDrainDmgMod','drowning':'drownDmgMod'}
def values(seq):return {r['damage_type']:Fraction(r['percent']['numerator'],r['percent']['denominator']) for r in (seq or [])}
def ratio(v):return {'numerator':v.numerator,'denominator':v.denominator}
def parsed(raw):
 if raw is None:return None
 m=re.fullmatch(r'\s*(-?\d+(?:[.,]\d+)?)\s*%?\s*',str(raw))
 return None if not m else Fraction(100)-Fraction(m[1].replace(',','.'))
def source_proof(row):
 # Exact cached donor evidence only, no lazy fetch. Source-only is not Wiki proof.
 result={}
 for b in row.get('source_bindings',[]):
  repo=pathlib.Path('/workspace/monster-reference-sources')/('crystal'if'crystal'in b['source_key']else'canary')
  pin=b['source_revision'].split(':')[-1];path=b['external_id'];path=path if path.endswith('.lua') else'data-otservbr-global/monster/'+path+'.lua'
  env=__import__('os').environ.copy();env['GIT_NO_LAZY_FETCH']='1'
  p=subprocess.run(['git','show',pin+':'+path],cwd=repo,capture_output=True,env=env)
  if p.returncode:continue
  text=p.stdout.decode();section=re.search(r'monster\.elements\s*=\s*\{(.*?)\n\}',text,re.S)
  if not section:continue
  mapping={'PHYSICAL':'physical','ENERGY':'energy','EARTH':'earth','POISON':'earth','FIRE':'fire','DEATH':'death','HOLY':'holy','ICE':'ice','LIFEDRAIN':'life_drain','MANADRAIN':'mana_drain','DROWN':'drowning'}
  for m in re.finditer(r'type\s*=\s*COMBAT_(\w+)DAMAGE\s*,\s*percent\s*=\s*(-?\d+(?:\.\d+)?)',section[1]):
   damage=mapping.get(m[1]);
   if damage:result[damage]={'value':Fraction(m[2]),'path':path,'revision':pin,'sha256':hashlib.sha256(p.stdout).hexdigest(),'line':text[:section.start(1)+m.start()].count('\n')+1}
  # LIFEDRAIN/MANADRAIN/DROWNDAMAGE suffix irregularities.
  for m in re.finditer(r'type\s*=\s*COMBAT_(LIFEDRAIN|MANADRAIN|DROWNDAMAGE)\s*,\s*percent\s*=\s*(-?\d+(?:\.\d+)?)',section[1]):
   damage={'LIFEDRAIN':'life_drain','MANADRAIN':'mana_drain','DROWNDAMAGE':'drowning'}[m[1]];result[damage]={'value':Fraction(m[2]),'path':path,'revision':pin,'sha256':hashlib.sha256(p.stdout).hexdigest(),'line':text[:section.start(1)+m.start()].count('\n')+1}
 return result
proposals=[];counts=collections.Counter();element_counts=collections.Counter()
for row in data['rows']:
 key=row['target']['key'];slug=key.split('oteryn:creature.',1)[-1];actual=canon[key];name=actual['authoring']['profile']['details']['display_name'];title,titlesrc=titles.get(slug,(name,None));page=wiki.get(title)
 # Explicit cached wiki binding is preferred; exact source/name equality is recorded as corroboration, never inferred variants.
 historical=pathlib.Path('/workspace/monster-completion-output/bundles')/slug/'manifest.json'
 bound_titles=[]
 if historical.exists():
  hm=load(historical);bound_titles=[s.get('title')for s in hm.get('sources',[])if s.get('kind')=='mediawiki'and'Loot Statistics:'not in s.get('title','')]
 binding=(titlesrc is not None or title in bound_titles)
 old=values(row.get('old'));canonical=values(row.get('canonical'));new=dict(old);evidence=[];sp=None
 changed=[t for t in sorted(set(old)|set(canonical))if old.get(t,Fraction(0))!=canonical.get(t,Fraction(0)) or(t in old)!=(t in canonical)]
 for t in changed:
  f=keys.get(t);raw=page.get('fields',{}).get(f)if page and f else None;wv=parsed(raw)if binding else None;cv=canonical.get(t,Fraction(0));ov=old.get(t,Fraction(0));proof={'damage_type':t,'old_present':t in old,'old':ratio(ov),'canonical_present':t in canonical,'canonical':ratio(cv),'neutral_presence_only':ov==cv and((t in old)!=(t in canonical))}
  if wv is not None:
   proof.update(provenance='WIKI_CONFIRMED',wiki_value=ratio(wv),wiki_raw=raw,wiki_field=f,wiki_line=page.get('field_lines',{}).get(f),source={'cache':str(wiki_path),'title':title,'url':page['url'],'revision_id':page['revision_id'],'content_sha256':page['content_sha256'],'method':page.get('method')},identity_binding={'sample':titlesrc,'historical_manifest':str(historical)if title in bound_titles else None},canonical_agrees_wiki=cv==wv)
   # Follow decisive observed value. Preserve canonical presence when it agrees; otherwise non-neutral gets explicit entry.
   if wv==0:
    if t in old and old[t]==0:new[t]=wv
    else:new.pop(t,None)
   else:new[t]=wv
   proof['action']='APPLY_WIKI'if ov!=wv or(t in old)!=(t in new)else'KEEP_WIKI_MATCHING_OLD'
  elif ov==cv:
   # Explicit0 vs omitted neutral changes no numerical policy. Keep serving representation, record absence correctly.
   proof.update(provenance='UNKNOWN',action='KEEP_SEMANTIC_NEUTRAL_REPRESENTATION',reason='Omitted neutral is numerically zero; no new resistance value inferred.')
  else:
   if sp is None:sp=source_proof(row)
   s=sp.get(t)
   if s and s['value']==cv:proof.update(provenance='SOURCE_ONLY',action='RETAIN_OLD_PENDING_WIKI',source={k:v for k,v in s.items()if k!='value'},source_value=ratio(s['value']))
   else:proof.update(provenance='UNKNOWN',action='RETAIN_OLD_PENDING_FIELD_PROOF',wiki_raw=raw,wiki_identity_bound=binding)
  evidence.append(proof);element_counts[proof['provenance']]+=1
 newseq=[]
 for item in (row['old']or[]):
  t=item['damage_type']
  if t in new:newseq.append({'damage_type':t,'percent':ratio(new[t])})
 for t in sorted(set(new)-set(old)):
  assert new[t]!=0
  newseq.append({'damage_type':t,'percent':ratio(new[t])})
 canonical_successor=dict(canonical)
 canonical_conflicts=[]
 for e in evidence:
  if e['provenance']=='WIKI_CONFIRMED' and not e['canonical_agrees_wiki']:
   t=e['damage_type'];v=Fraction(e['wiki_value']['numerator'],e['wiki_value']['denominator'])
   if v==0:canonical_successor.pop(t,None)
   else:canonical_successor[t]=v
   canonical_conflicts.append(e)
 canonical_new=[]
 for item in (row['canonical']or[]):
  t=item['damage_type']
  if t in canonical_successor:canonical_new.append({'damage_type':t,'percent':ratio(canonical_successor[t])})
 for t in sorted(set(canonical_successor)-set(canonical)):
  canonical_new.append({'damage_type':t,'percent':ratio(canonical_successor[t])})
 if not row['old_present'] and not newseq:newseq=None
 if not row['canonical_present'] and not canonical_new:canonical_new=None
 provenance='WIKI_CONFIRMED'if any(e['provenance']=='WIKI_CONFIRMED'for e in evidence)else'SOURCE_ONLY'if any(e['provenance']=='SOURCE_ONLY'for e in evidence)else'UNKNOWN'
 modified=newseq!=row['old'];counts[provenance]+=1;counts['apply_rows'if modified else'keep_rows']+=1
 proposals.append({'target':row['target'],'field':'resistances','old_present':row['old_present'],'old':row['old'],'new_present':row['old_present']or modified,'new':newseq,'canonical_present':row['canonical_present'],'canonical':row['canonical'],'provenance':provenance,'apply':modified,'evidence':evidence,'unresolved_elements':[e['damage_type']for e in evidence if e['provenance']!='WIKI_CONFIRMED'and not e['neutral_presence_only']],'source_bindings':row['source_bindings'],'canonical_path':row['canonical_path'],'canonical_changes':{'required':bool(canonical_conflicts),'old_present':row['canonical_present'],'old':row['canonical'],'new_present':row['canonical_present']or bool(canonical_conflicts),'new':canonical_new,'evidence':canonical_conflicts}})
counts['canonical_conflict_rows']=sum(r['canonical_changes']['required']for r in proposals)
counts['canonical_conflict_elements']=sum(len(r['canonical_changes']['evidence'])for r in proposals)
assert len(proposals)==197 and len({r['target']['key']for r in proposals})==197
for r in proposals:
 for e in r['evidence']:
  if e['provenance']=='WIKI_CONFIRMED':assert Fraction(100)-Fraction(str(e['wiki_raw']).strip().rstrip('%').strip().replace(',','.'))==Fraction(e['wiki_value']['numerator'],e['wiki_value']['denominator'])
result={'schema':'OTERYN_WIKI_PRIORITY_FIELD_PROPOSALS/v1','lane':'resistance_a','expected_native_sha256':data['native_sha256'],'rows':proposals,'counts':dict(counts),'element_provenance_counts':dict(element_counts),'inputs':inputs,'normalization':'Native resistance percent=100-wiki damageTaken percent; omitted neutral0 is semantically distinguished from absent evidence. Unknown elements retained; no SOURCE_ONLY change masquerades aswiki.','validation':{'rows197_unique':True,'wiki_percent_formula_checked':True,'uncertain_or_non_numeric_not_confirmed':True,'sourceonly_does_not_mutate':True},'product_writes':False,'network_used':False}
(OUT/'proposal.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'counts':dict(counts),'element_counts':dict(element_counts),'proposal_sha256':hashlib.sha256((OUT/'proposal.json').read_bytes()).hexdigest()}))
