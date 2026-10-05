from pathlib import Path
import json,gzip,re,hashlib,collections
base=Path('/workspace/monster-main-reconciliation-20261004');out=Path('/tmp/monster-wiki-priority-20261004/core');inp=json.load(open(out.parent/'core.json'))
cache=Path('/workspace/monster-field-audit-20261002/wiki/pages.json.gz');br=json.load(gzip.open(cache,'rt'))['pages'];enp=Path('/workspace/monster-field-audit-20261002/wiki/en-pages.json.gz');en=json.load(gzip.open(enp,'rt'));en=en.get('pages',[])if isinstance(en,dict)else en
norm=lambda s:re.sub(r'[^a-z0-9]','',s.lower())
pages=collections.defaultdict(list)
for p in br+en:
 title=p.get('page_title',p.get('title',''));pages[norm(title)].append(p)
can={};filehash={}
for path in {r['canonical_path']for r in inp['rows']}:
 raw=(base/path).read_bytes();filehash[path]=hashlib.sha256(raw).hexdigest();j=json.loads(raw)
 for r in j['records']:can[r.get('identity',{}).get('key')]=r
num=lambda x:int(str(x))if re.fullmatch(r'\d+',str(x).strip())else None
emap={'physical':'physicalDmgMod','earth':'earthDmgMod','fire':'fireDmgMod','death':'deathDmgMod','energy':'energyDmgMod','holy':'holyDmgMod','ice':'iceDmgMod'}
rows=[]
for r in inp['rows']:
 key=r['target']['key'];cr=can.get(key,{});profile=cr.get('authoring',{}).get('profile',{});title=profile.get('details',{}).get('display_name',key.removeprefix('oteryn:creature.').replace('_',' ').title());ps=pages[norm(title)];field=r['field'];proof=[];status='UNKNOWN';apply=False;new=r['old'];present=r['old_present'];reason='No matching explicit numeric wiki cell proves accepted canonical value.'
 for p in ps:
  fs=p.get('fields',{});wc={'health':'hp','experience':'exp','armor':'defense','speed':'speed'}.get(field)
  if p in en:wc={'health':'hp','experience':'exp','armor':'armor','speed':'speed'}.get(field)
  if wc and num(fs.get(wc))==r['canonical'] and r['canonical_present']:
   proof.append(dict(url=p.get('url',p.get('page_url')),page_title=p.get('page_title'),revision_id=p.get('revision_id'),revision_timestamp=p.get('revision_timestamp'),content_sha256=p.get('content_sha256'),method=p.get('method','CACHED_CAPTURE'),field=wc,raw=fs[wc],normalized=num(fs[wc])))
 if field in ['health','experience','armor']and proof:
  status='WIKI_CONFIRMED';apply=True;new=r['canonical'];present=True;reason='Exact display-title cached wiki plain integer equals accepted canonical field. Wiki BR defense is visible Armor, not donor defense skill.'
 elif field=='speed':
  status='SOURCE_ONLY'if r['source_bindings'] else'UNKNOWN';reason='Native donor monster.speed is raw engine speed; Wiki observed speed is distinct. Canonical observed value retained in comparison, no raw native replacement without explicit unit contract.'
  if proof:reason+=' Cached wiki confirms canonical observed speed, not raw engine equality.'
 elif field=='immunities':
  old=set(r['old']or[]);newset=set(r['canonical']or[]);changes=old.symmetric_difference(newset);covered={};evidence=[]
  for p in ps:
   fs=p.get('fields',{})
   for el in changes:
    wc=emap.get(el)
    if not wc:continue
    raw=fs.get(wc)
    match=re.fullmatch(r'\s*(\d+(?:[.,]\d+)?)%\s*',str(raw))
    if match:
     received=float(match.group(1).replace(',','.'));expected=el in newset
     if (received==0)==expected:
      covered[el]=True;evidence.append(dict(url=p.get('url'),page_title=p.get('page_title'),revision_id=p.get('revision_id'),revision_timestamp=p.get('revision_timestamp'),content_sha256=p.get('content_sha256'),method=p.get('method','CACHED_CAPTURE'),field=wc,raw=raw,normalized_received_damage_percent=received,immunity=expected))
  if changes and all(covered.get(x)for x in changes):
   status='WIKI_CONFIRMED';apply=True;new=r['canonical'];present=r['canonical_present'];proof=evidence;reason='Only intrinsic DAMAGE immunity changes checked against explicit received-damage modifier. Condition Invisibility/Paralysis data is separate and preserved; 0% remains represented by canonical resistance data.'
  else:
   status='SOURCE_ONLY'if r['source_bindings']else'UNKNOWN';reason='Damage immunity change lacks explicit same-identity percentage for '+','.join(sorted(changes-set(covered)))+'; textual Invisibility/Paralysis is not elemental damage immunity proof.'
 rows.append(dict(target=r['target'],field=field,old_present=r['old_present'],old=r['old'],new_present=present,new=new,canonical_present=r['canonical_present'],canonical=r['canonical'],canonical_path=r['canonical_path'],canonical_file_sha256=filehash[r['canonical_path']],apply=apply,status=status,reason=reason,provenance=proof,source_bindings=r['source_bindings']))
summary={'rows':len(rows),'apply':sum(r['apply']for r in rows),'statuses':dict(collections.Counter(r['status']for r in rows)),'by_field':dict(collections.Counter(r['field']for r in rows if r['apply']))}
(out/'proposal.json').write_text(json.dumps({'status':'PREPARED_NO_PRODUCT_WRITES','native_sha256':inp['native_sha256'],'source_cache':str(cache),'source_cache_sha256':hashlib.sha256(cache.read_bytes()).hexdigest(),'summary':summary,'rows':rows},indent=2)+'\n')
print(summary)
print('numeric unproven',[(r['target']['key'],r['field'],r['canonical'])for r in rows if not r['apply']and r['field']not in ['speed','immunities']])
