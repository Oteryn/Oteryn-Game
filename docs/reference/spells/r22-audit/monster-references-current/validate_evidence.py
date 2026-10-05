import collections,hashlib,json,pathlib,re,unicodedata
root=pathlib.Path(__file__).resolve().parent
facts=[json.loads(s) for s in (root/'tibiopedia-monster-facts.jsonl').read_text().splitlines()]
comparisons=json.loads((root/'monster-slot-field-comparisons.json').read_text())
actual=json.load(open('/workspace/spells-r22-monster-import-current/monster-spell-slots.json'))
assert len(facts)==len({x['monster_name'] for x in facts})
assert len(comparisons)==len({x['slot_key'] for x in comparisons})
index={x['slot_key']:x for x in comparisons}
for source in actual:
 key=f"{source['source']}/{source['monster_source']['path']}/{source['group']}/{source['source_slot_index']}"
 c=index[key]
 assert c['source_sha256']==source['monster_source']['sha256'],key
 assert c['monster_name']==source['monster'],key
 assert {k:v['source_value'] for k,v in c['field_comparisons'].items()}==source['source_parameters'],key
for row in comparisons:assert row['all_fields_verified'] is False
aliases=[]
for fact in facts:
 assert fact['read_method']=='normal_http'
 assert fact['url'].startswith('https://tibiopedia.pl/monsters/')
 if fact['status']=='attack_facts_read':
  assert fact['http_status']==200
  assert re.fullmatch('[0-9a-f]{64}',fact['page_sha256'])
  title=fact['title'].split(' - Tibia')[0].removeprefix('Monsters: ')
  if title.casefold()!=fact['monster_name'].casefold():
   strip=lambda s:''.join(c for c in unicodedata.normalize('NFKD',s.casefold()) if not unicodedata.combining(c))
   assert strip(title)==strip(fact['monster_name']),fact
   aliases.append({'source_name':fact['monster_name'],'wiki_title':title,'url':fact['url']})
assert len(actual)>0
report={'actual_import_slots_sha256':hashlib.sha256(pathlib.Path('/workspace/spells-r22-monster-import-current/monster-spell-slots.json').read_bytes()).hexdigest(),'actual_import_profiles_sha256':hashlib.sha256(pathlib.Path('/workspace/spells-r22-monster-import-current/monster-profiles.json').read_bytes()).hexdigest(),'field_comparisons_sha256':hashlib.sha256((root/'monster-slot-field-comparisons.json').read_bytes()).hexdigest(),'public_facts_sha256':hashlib.sha256((root/'tibiopedia-monster-facts.jsonl').read_bytes()).hexdigest(),'status':'PASS','actual_import_slots_verified':len(actual),'reference_union_slots':len(comparisons),'unique_monster_page_records':len(facts),'page_statuses':dict(collections.Counter(f['status'] for f in facts)),'source_parameters_and_hash_mismatches':0,'missing_actual_slots':0,'incorrect_complete_verification_claims':0,'canonical_spelling_aliases':aliases,'checks':['All actual import slots joined','Every source parameter preserved exactly','Every source whole-file SHA matched','No complete-all-field claim while external values unknown','Unique slot keys and unique fact species','All successful pages normal HTTP200 with SHA256','Page title identity equals source identity modulo diacritics']}
(root/'evidence-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False))
