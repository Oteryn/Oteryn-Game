from pathlib import Path
import json,hashlib,collections,re,gzip
out=Path('/tmp/monster-wiki-priority-20261004/core');p=out/'proposal.json';d=json.loads(p.read_text());rows=d['rows'];pop=Path('/tmp/monster-final-fill-20261004/population/bundles');sample=Path('/workspace/monster-completion/tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json');sam=json.loads(sample.read_text());mon={r['monster']:r for r in sam['monsters']};br=json.load(gzip.open('/workspace/monster-field-audit-20261002/wiki/pages.json.gz','rt'))['pages'];brby={p['page_title']:p for p in br}
# Attach actual active-field receipt lineage to every row, retaining preexisting donor identities.
for r in rows:
 actor=r['target']['key'].split('.')[-1];path=pop/actor/'manifest.json';manifest=json.loads(path.read_text())if path.exists()else{};dest={'health':'stats/max_health','experience':'stats/experience','armor':'stats/armor','speed':'stats/speed','immunities':'immunities/damage_types'}[r['field']]
 active=[]
 for e in manifest.get('entries',[]):
  if e.get('status')=='mapped'and e.get('destination','').endswith('/'+dest):active.append({'entry':e,'source':manifest['sources'][e['source_index']]})
 r['active_normalized_lineage']={'manifest_path':str(path),'manifest_sha256':hashlib.sha256(path.read_bytes()).hexdigest()if path.exists()else None,'receipts':active}
 if r['field']=='speed'and r['provenance']:
  r.update(status='WIKI_CONFIRMED',apply=True,new=r['canonical'],new_present=r['canonical_present'],reason='Exact cached wiki observed speed equals accepted canonical. Root accepts this value as PROJECT_NATIVE_BASE_SPEED_FROM_WIKI; no engine-unit conversion or Global parity claimed.',qualification='PROJECT_NATIVE_BASE_SPEED_FROM_WIKI')
 if not r['apply']and r['field']=='armor':
  source=mon.get(actor);proof=[]
  if source:
   for rr in source.get('rows',[]):
    if rr['field']=='armor'and rr.get('wiki')==r['canonical']and rr.get('status')=='DIFF':proof.append({'url':source.get('page_url'),'wiki_title':source.get('wiki_title'),'page_id':source.get('page_id'),'revision_id':source.get('cut_revision_id'),'content_sha256':source.get('cut_content_sha256'),'field':'armor','raw':rr.get('wiki_raw'),'normalized':rr['wiki'],'line':rr.get('wiki_line'),'capture_path':str(sample),'capture_sha256':hashlib.sha256(sample.read_bytes()).hexdigest(),'identity_binding':'Cached monster-qualified sample names exact source actor key; variant title retained.'})
  if proof:r.update(status='WIKI_CONFIRMED',apply=True,new=r['canonical'],new_present=r['canonical_present'],provenance=proof,reason='Cached source-actor-qualified Fandom numeric armor row explicitly equals accepted canonical, including named variant.')
  elif actor=='pink_butterfly'and active:
   r.update(status='WIKI_NORMALIZED_LINEAGE',apply=True,new=r['canonical'],new_present=r['canonical_present'],provenance=active,reason='Accepted active Fandom Butterfly(Purple) armor0 normalized into PinkButterfly source variant. Retain honest existing variant-lineage qualification, not exact same-title numeric confirmation.',qualification='PROJECT_ACCEPTED_VARIANT_WIKI_LINEAGE_GLOBAL_IDENTITY_UNVERIFIED')
 if actor=='magma_bubble'and r['field']=='experience':
  page=brby['Magma Bubble'];raw=page['fields']['exp'];assert raw=='80.000';r.update(status='WIKI_CONFIRMED',apply=False,new=80000,new_present=True,reason='Portuguese wiki thousands separator80.000 means80000. Native80000 alreadycorrect; canonical80 is locale parser error. Repair canonical only.',provenance=[{'url':page['url'],'revision_id':page['revision_id'],'content_sha256':page['content_sha256'],'field':'exp','raw':raw,'normalized':80000,'normalization':'Portuguese dot thousands grouping'}],canonical_correction={'old_present':r['canonical_present'],'old':r['canonical'],'new_present':True,'new':80000})
 if actor=='dragon_hoard':
  src=mon[actor];pro=[]
  for rr in src['rows']:
   if r['field']=='experience'and rr['field']=='experience'or r['field']=='immunities'and rr['field'].startswith('resistance.'):
    pro.append({'url':src['page_url'],'wiki_title':src['wiki_title'],'revision_id':src['cut_revision_id'],'content_sha256':src['cut_content_sha256'],'field':rr['field'],'raw':rr.get('wiki_raw'),'normalized':rr.get('wiki'),'note':rr.get('note'),'line':rr.get('wiki_line'),'capture_path':str(sample),'capture_sha256':hashlib.sha256(sample.read_bytes()).hexdigest()})
  desired=600000 if r['field']=='experience'else r['old'];r.update(status='WIKI_CONFIRMED',apply=False,new=desired,new_present=True,provenance=pro,reason='Exact Fandom DragonHoard outranks misbound DragonPack lineage (different actor). Native alreadycorrect; canonical repair only.0% receiveddamage means intrinsic damageimmunity, not neutral resistance.',canonical_correction={'old_present':r['canonical_present'],'old':r['canonical'],'new_present':True,'new':desired})
 if actor=='egg'and r['field']=='immunities':
  r.update(status='UNKNOWN',apply=False,reason='Accepted canonicaldeath has only broad donor immunity receipt, no activeWiki field receipt. CachedSourceEgg has neutraldeathpercent0 and only condition immunities; no source-exact damageproof supportsaddingdeath. Identity review required; do not invent immunity.')
summary={'rows':len(rows),'native_apply':sum(r['apply']for r in rows),'canonical_corrections':sum('canonical_correction'in r for r in rows),'statuses':dict(collections.Counter(r['status']for r in rows)),'native_by_field':dict(collections.Counter(r['field']for r in rows if r['apply'])),'unknown':[r['target']['key']+':'+r['field']for r in rows if r['status']=='UNKNOWN']}
d['summary']=summary;d['normalization_policy']='Root approved PROJECT_NATIVE_BASE_SPEED_FROM_WIKI; no guessedfactor2. Typed mapping naturally consumes acceptedvalue unchanged. DRAGON_PACK is rejectedwrongidentity; MAGMA_EXP thousandsfixed.'
p.write_text(json.dumps(d,indent=2)+'\n');(out/'fallback-proposal.json').write_text(json.dumps({'status':'PREPARED_ACCEPTED_ACTIVE_LINEAGE_NOT_NEW_NUMERIC_CONFIRMATION','rows':[r for r in rows if r['field']=='speed'or r['status']in ['WIKI_NORMALIZED_LINEAGE','UNKNOWN']or'canonical_correction'in r]},indent=2)+'\n');print(summary)
