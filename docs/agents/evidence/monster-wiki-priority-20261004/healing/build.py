import pathlib,json,gzip,hashlib,collections
ROOT=pathlib.Path('/workspace/monster-main-reconciliation-20261004');OUT=pathlib.Path(__file__).resolve().parent;inputs={};sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p,gz=False):
 p=pathlib.Path(p);b=p.read_bytes();inputs[str(p)]={'sha256':sha(b),'bytes':len(b)};return json.loads(gzip.decompress(b)if gz else b)
findings=load(OUT.parent/'resistance_b/proposal.json')['non_resistance_wiki_findings'];assert len(findings)==5
cache=pathlib.Path('/workspace/monster-field-audit-20261002/wiki/pages.json.gz');wiki={p['page_title']:p for p in load(cache,True)['pages']};native_rel='content/creatures/definitions/spell-native-profiles.json';native=load(ROOT/native_rel);world_rel='content/world/definitions/declarations.json';world=load(ROOT/world_rel);group=collections.defaultdict(list)
for f in findings:group[f['actor']].append(f)
rows=[];evidence=[]
for actor,fs in sorted(group.items()):
 key='oteryn:creature.'+actor;family_rel=fs[0]['canonical_path'];family=load(ROOT/family_rel);fi,fr=next((i,r)for i,r in enumerate(family['records'])if r['definition']['identity']['key']==key);ni,nr=next((i,r)for i,r in enumerate(native['records'])if r['profile']['target']['key']==key);wi,wr=next((i,r)for i,r in enumerate(world['authoring_profiles'])if r['target']['family']=='Creature'and r['target']['key']==key)
 name=fr['authoring']['profile']['details']['display_name'];page=wiki[name]
 # Explicit accepted historical wiki mapping establishes actor association, not a global alias guess.
 manifestpath=pathlib.Path('/workspace/monster-completion-output/bundles')/actor/'manifest.json';historical=load(manifestpath);titles=[s.get('title')for s in historical['sources']if s.get('kind')=='mediawiki'];assert name in titles,(actor,titles)
 entryproof=[]
 for f in fs:
  field={'earth':'earthDmgMod','death':'deathDmgMod','energy':'energyDmgMod','ice':'iceDmgMod'}[f['damage_type']];raw=page['fields'][field];assert raw==f['wiki_damage_taken'];assert page['revision_id']==f['revision_id']and page['content_sha256']==f['content_sha256'];percent=-int(raw.rstrip('%'));assert percent==f['wiki_healing_percent']['numerator']and percent>0
  ep={'damage_type':f['damage_type'],'percent':{'numerator':percent,'denominator':1},'provenance':'WIKI_CONFIRMED','wiki_damage_taken':raw,'wiki_field':field,'wiki_line':page['field_lines'][field],'wiki_title':name,'wiki_url':page['url'],'wiki_revision_id':page['revision_id'],'wiki_content_sha256':page['content_sha256'],'wiki_cache':str(cache),'identity_binding_manifest':str(manifestpath),'retrieval_method':page['method']};entryproof.append(ep);evidence.append(ep)
 locations=[]
 for rel,idx,details,pointer in [(family_rel,fi,fr['authoring']['profile']['details'],f'/records/{fi}/authoring/profile/details/healing_from_damage'),(native_rel,ni,nr['profile']['data']['profile']['details'],f'/records/{ni}/profile/data/profile/details/healing_from_damage'),(world_rel,wi,wr['data']['profile']['details'],f'/authoring_profiles/{wi}/data/profile/details/healing_from_damage')]:
  old=details.get('healing_from_damage');merged={r['damage_type']:r for r in(old or[])}
  for ep in entryproof:merged[ep['damage_type']]={'damage_type':ep['damage_type'],'percent':ep['percent']}
  new=[merged[k]for k in sorted(merged)];locations.append({'path':rel,'expected_file_sha256':inputs[str(ROOT/rel)]['sha256'],'pointer':pointer,'old_present':'healing_from_damage'in details,'old':old,'new_present':True,'new':new})
 rows.append({'target':nr['profile']['target'],'field':'details.healing_from_damage','provenance':'WIKI_CONFIRMED','locations':locations,'evidence':entryproof,'current_native_resistances':nr['profile']['data']['profile'].get('resistances'),'current_native_immunities':nr['profile']['data']['profile'].get('immunities'),'other_fields_unchanged':True})
assert len(rows)==4 and len(evidence)==5
for r in rows:
 for l in r['locations']:
  assert len({e['damage_type']for e in l['new']})==len(l['new'])
  assert all(e['percent']['numerator']>=0 and e['percent']['denominator']==1 for e in l['new'])
code={}
for rel in ['apps/game-server/src/content/project/v2/creature.rs','apps/game-server/src/content/native_gameplay.rs','apps/game-server/src/foundation/runtime_actor_companion.rs']:
 b=(ROOT/rel).read_bytes();code[rel]=sha(b)
proof={'schema':'OTERYN_WIKI_PRIORITY_HEALING_FROM_DAMAGE_CORRECTION/v1','rows':rows,'counts':{'actors':4,'healing_entries':5,'guarded_locations':12,'wiki_confirmed_entries':5},'inputs':inputs,'code_sha256':code,'schema_support':{'percent_above100_allowed':True,'witness':'apps/game-server/src/content/project/v2/creature.rs:814-840 damage_responses: sortedunique snake damage_type; exact ratio; nonnegative; no100 ceiling'},'runtime_qualification':{'native_authoring_retention':'SUPPORTED_BY_EXISTING_TYPED_PROFILE_DECODER','elemental_heal_consumer':'NOT_FOUND_IN_BOUNDED_RUST_APPS_CRATES_VENDOR_SWEEP','healing_vs_immunity_resistance_order':'UNCONFIRMED_EXISTING_OWNER_CONSUMER_DEPENDENCY','physical_300_percent_heal_executed':False},'policy':'Negative wiki damage-taken cells become positive healing_from_damage; never resistance above100. Preserve source/current immunity/resistance pending owner ordering qualification. Source-field data import is independent from runtime activation.','product_writes':False,'network_used':False,'native_tests':'PENDING_ROOT'}
(OUT/'proposal.json').write_text(json.dumps(proof,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'counts':proof['counts'],'sha256':sha((OUT/'proposal.json').read_bytes())}))
