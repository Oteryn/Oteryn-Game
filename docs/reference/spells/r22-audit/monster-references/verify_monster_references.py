import concurrent.futures,datetime,hashlib,html,json,pathlib,re,time,urllib.request,urllib.parse,collections
ROOT=pathlib.Path('/workspace/spells-r22-monster-reference'); AUD=pathlib.Path('/workspace/spells-r22-source-audit'); NOW=datetime.datetime.now(datetime.timezone.utc).isoformat()
def txt(s):return ' '.join(html.unescape(re.sub('<[^>]*>',' ',s)).split())
slots=[];names={}
for source in ('canary','crystal'):
 for row in json.loads((AUD/f'{source}-monster-attack-defense-slots.json').read_text()):
  p=AUD/'upstream-local-only'/source/row['file']; raw=p.read_text(errors='replace'); m=re.search(r'Game\.createMonsterType\s*\(\s*([\"\'])(.*?)\1',raw,re.S)
  name=m.group(2) if m else row.get('monster_name'); row=dict(row,source=source,monster_name=name);slots.append(row)
  if name:names.setdefault(name,[]).append(row['slot_key'])
actual_slots_path=pathlib.Path('/workspace/spells-r22-monster-import/monster-spell-slots.json')
if actual_slots_path.exists():
 existing={row['slot_key']:i for i,row in enumerate(slots)}
 for actual in json.loads(actual_slots_path.read_text()):
  key=f"{actual['source']}/{actual['monster_source']['path']}/{actual['group']}/{actual['source_slot_index']}"
  row={'slot_key':key,'file':actual['monster_source']['path'],'sha256':actual['monster_source']['sha256'],'block':actual['group'],'index':actual['source_slot_index'],'monster_name':actual['monster'],'source':actual['source'],'name':actual['source_parameters'].get('name'),'entry':actual['source_parameters']}
  if key in existing: slots[existing[key]]=row
  else: existing[key]=len(slots);slots.append(row)
  names.setdefault(row['monster_name'],[]).append(key)
profile_path=pathlib.Path('/workspace/spells-r22-monster-import/monster-profiles.json')
if profile_path.exists():
 for profile in json.loads(profile_path.read_text()): names.setdefault(profile['name'],[])
def fetch(name):
 url='https://tibiopedia.pl/monsters/'+urllib.parse.quote(name.replace(' ','_'),safe='')
 out={'monster_name':name,'url':url,'read_method':'normal_http','observed_at':NOW}
 try:
  request=urllib.request.Request(url,headers={'Cookie':'tp_lang=en; tp_layout=library','User-Agent':'Oteryn compatibility research; public creature facts'})
  with urllib.request.urlopen(request,timeout=20) as r:d=r.read();out.update(http_status=r.status,final_url=r.url,page_sha256=hashlib.sha256(d).hexdigest())
  s=d.decode('utf8',errors='replace');out['title']=txt(re.search(r'<title>(.*?)</title>',s,re.S).group(1))
  m=re.search(r'>Attacks:</td>.*?<div style="float:left">(.*?)</div>',s,re.S)
  if not m:out.update(status='page_has_no_attack_table',attacks=[],abilities=[]);return out
  attacks=[]
  for v in re.split(r'<br\s*/?>',m.group(1)):
   label=txt(v); vals=re.findall(r'<span class="att_color_(\w+)">(.*?)</span>',v,re.S)
   attacks.append({'label':label,'damage_ranges':[{'element':e,'text':txt(t),'numeric_range':list(map(int,q.groups())) if (q:=re.fullmatch(r'(\d+)\s*-\s*(\d+)',txt(t))) else None} for e,t in vals],'on_target':True if 'on target' in label.lower() else None})
  am=re.search(r'>Abilities:</td>.*?<td[^>]*class="monsterDetRowC"[^>]*>(.*?)</td>',s,re.S)
  abilities=re.findall(r'<img[^>]*alt="([^"]*)"',am.group(1)) if am else []
  out.update(status='attack_facts_read',attacks=attacks,abilities=[html.unescape(a) for a in abilities if a],abilities_text=txt(am.group(1)) if am else None)
 except Exception as e:out.update(status='unavailable',error=str(e),attacks=[],abilities=[])
 return out
cache=ROOT/'tibiopedia-monster-facts.jsonl';done={}
if cache.exists():
 for line in cache.read_text().splitlines():
  o=json.loads(line);done[o['monster_name']]=o
with cache.open('a') as f,concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
 fs={pool.submit(fetch,n):n for n in names if n not in done}
 for i,future in enumerate(concurrent.futures.as_completed(fs),1):
  o=future.result();done[o['monster_name']]=o;f.write(json.dumps(o,ensure_ascii=False)+'\n');f.flush()
  if i%100==0:print('fetched',len(done),'/',len(names),collections.Counter(o['status'] for o in done.values()),flush=True)
# Explicit comparisons only where element + spell morphology can distinguish candidates; damage estimates are observed ranges, not engine-authoritative constants.
comparisons=[]
for row in slots:
 fact=done.get(row['monster_name']);e=row['entry'];kind=row['name']; elem=e.get('type','').removeprefix('@COMBAT_').removesuffix('DAMAGE').lower();elem={'healing':'heal','earth':'earth'}.get(elem,elem)
 if kind=='melee' and not elem:elem='physical'
 if kind=='healing':elem='heal'
 candidates=[]
 if fact:
  for attack in fact['attacks']:
   if kind=='melee' and not attack['label'].lower().startswith('basic attack'):continue
   if kind!='melee' and attack['label'].lower().startswith('basic attack'):continue
   if any(r['element']==elem for r in attack['damage_ranges']):candidates.append(attack)
 fields={k:{'source_value':v,'status':'unknown_external_not_published'} for k,v in e.items()}
 if fact and fact['status']=='attack_facts_read' and elem:
  fields['type' if 'type' in e else 'name']={'source_value':e.get('type',e.get('name')),'status':'element_or_melee_presence_supported' if candidates else 'no_unambiguous_external_match','external_candidates':candidates}
 if len(candidates)==1:
  ranges=[r['numeric_range'] for r in candidates[0]['damage_ranges'] if r['element']==elem and r['numeric_range'] is not None]
  if len(ranges)==1:
   for key,ix in [('minDamage',0),('maxDamage',1)]:
    if key in e:
     val=abs(e[key]) if isinstance(e[key],(int,float)) else None
     fields[key]={'source_value':e[key],'external_observed_value':ranges[0][ix],'status':'numeric_equal_observed_range' if val==ranges[0][ix] else 'numeric_discrepancy_or_damage_measurement_semantics','note':'Tibiopedia damage ranges are observed; melee skill/attack and armor semantics prevent automatic correction.'}
  if 'target' in e and candidates[0]['on_target'] is True:fields['target']={'source_value':e['target'],'external_value':True,'status':'equal' if e['target'] is True else 'discrepancy_or_slot_mapping_ambiguous'}
 comparisons.append({'slot_key':row['slot_key'],'monster_name':row['monster_name'],'source':row['source'],'file':row['file'],'source_sha256':row['sha256'],'block':row['block'],'spell_name':kind,'external_url':fact['url'] if fact else None,'external_page_sha256':fact.get('page_sha256') if fact else None,'external_status':fact['status'] if fact else 'source_name_unresolved','field_comparisons':fields,'all_fields_verified':False})
(ROOT/'monster-slot-field-comparisons.json').write_text(json.dumps(comparisons,ensure_ascii=False,indent=2)+'\n')
summary={'schema':'OTERYN_MONSTER_EXTERNAL_FIELD_AUDIT/v1','observed_at':NOW,'sources':{s:json.loads((AUD/'source-completeness-summary.json').read_text())['sources'][s]['revision'] for s in ['canary','crystal']},'slot_count':len(slots),'unique_monster_names':len(names),'external_page_statuses':dict(collections.Counter(o['status'] for o in done.values())),'field_statuses':dict(collections.Counter(v['status'] for r in comparisons for v in r['field_comparisons'].values())),'complete_all_field_verified_slots':0,'blocked_other_sources':{'tibiawiki.com.br':'HTTP403; Remote Desktop offline','tibia.fandom.com':'HTTP402; Remote Desktop offline','Tavily':'quota exhausted; no repeated retries'},'limits':['Public wiki attack ranges may be observational or stale. Exact engine chance/interval/effect identifiers generally unpublished.','Element-based candidates are not a proof of slot identity; ambiguous matches are retained as unknown.','No original HTML/Lua/assets redistributed; parsed facts with whole-page hashes and URLs only.','Existing BR normalized creature facts omit attacks; cannot support attack verification.']}
(ROOT/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n');print(json.dumps(summary),flush=True)
