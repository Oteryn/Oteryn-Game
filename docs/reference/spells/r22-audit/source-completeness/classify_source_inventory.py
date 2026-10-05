"""Classify every audit Lua file and map registrations to catalogue provenance without claiming runtime execution."""
import argparse,collections,hashlib,json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--audit',type=Path,required=True);p.add_argument('--repository',type=Path,required=True);a=p.parse_args()
def read(name):return json.loads((a.audit/name).read_text())
def write(name,value):(a.audit/name).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
cat=json.loads((a.repository/'tools/content-schema/spell-authoring/samples/executable-spell-catalog.json').read_text())
active=collections.defaultdict(list)
for b in cat['bundles']:
 s=b['bundle']['spell'];active[(s['carrier'],s['name'].casefold())].append({'identity':s['identity'],'source_identities':b['source_identities']})
removed={(b['carrier'],b['name'].casefold()):b for b in cat['removed']}
summ=read('source-completeness-summary.json');all_summary={};all_keys=set(); missing_keys=set()
monster_core={'candy horror wave','nibblemaw wave','heal malice','heal monster','heal monster 9x9'}
for label,metadata in summ['sources'].items():
 regs=read(label+'-registered-spells.json');files=read(label+'-tracked-lua-files.json');errors=read(label+'-unreadable-registration-files.json');lex=read(label+'-lexical-spell-constructor-files.json');slots=read(label+'-monster-attack-defense-slots.json')
 byfile=collections.defaultdict(list); names=collections.defaultdict(list);regkeys=collections.defaultdict(list)
 for r in regs:
  byfile[r['record']['file']].append(r);names[tuple(r['logical_key'])].append(r)
  rg=r['record']['registrar'];identity=rg.get('runeId') if r['logical_key'][0]=='rune' else rg.get('words')
  if identity is not None:regkeys[(r['logical_key'][0],str(identity))].append(r)
 refs=collections.Counter(s['name'] for s in slots)
 efile={e['file']:e for e in errors};lfile={e['file']:e for e in lex}
 pack='data-otservbr-global' if label.startswith('canary') else 'data-global'
 rows=[]
 for f in files:
  path=f['file'];has=byfile[path];error=efile.get(path);is_scope=path.startswith(('data/scripts/',pack+'/scripts/'));disabled=Path(path).name.startswith('#');fixture=path.startswith(('tests/','docs/')) or disabled
  classes=[]
  if has:
   for r in has:
    key=tuple(r['logical_key']);mapped=active.get(key,[]);removal=removed.get(key)
    if fixture: role='disabled_example_or_test_fixture'
    elif key[1] in monster_core or '/spells/monster/' in path or '/quests/' in path:role='monster_or_encounter_registered_spell'
    elif path.startswith(('data/scripts/spells/','data/scripts/runes/')):role='registered_player_spell_or_rune'
    else:role='registered_other_source_spell'
    r['source_classification']=role;r['default_config_scope']=is_scope;r['candidate_identity_matches']=mapped;r['removed_decision']=removal
    r['catalog_match_basis']='case-insensitive name plus carrier; recorded provenance alone does not prove every source variant or every parameter was imported'
    r['referenced_monster_slot_count']=refs.get(key[1],0)
    r['same_logical_key_other_registrations']=[o['registration_key'] for o in names[key] if o['registration_key']!=r['registration_key']]
    enginekey=r['record']['registrar'].get('runeId') if key[0]=='rune' else r['record']['registrar'].get('words')
    r['same_engine_key_other_registrations']=[o['registration_key'] for o in regkeys[(key[0],str(enginekey))] if o['registration_key']!=r['registration_key']] if enginekey is not None else []
    r['duplicate_policy']='preserved all variants; alternate datapack copies are not loaded together by default; same-scope duplicate words/runeId need loader-order qualification, no fabricated winner'
    classes.append(role)
    if not fixture:all_keys.add(key)
    if not fixture and not mapped and not removal:missing_keys.add(key)
  elif error and error.get('lexical_candidates'):
   classes=['source_only_unsupported_registration'];error['declarations_captured']=True;error['runtime_composition']='unimplemented'
  elif path in lfile:classes=['unregistered_spell_constructor_helper']
  elif '/spells/' in path or '/runes/' in path:classes=['unregistered_spell_helper']
  else:classes=['non_spell_lua_source']
  rows.append({**f,'default_config_scope':is_scope,'disabled_filename':disabled,'source_classifications':sorted(set(classes)),'registration_keys':[r['registration_key'] for r in has],'unresolved_registration':error if error and error.get('lexical_candidates') else None})
 conflicts=[]
 for key,rr in regkeys.items():
  if len(rr)>1:
   scope=[r for r in rr if r['default_config_scope'] and r['engine_enabled_path']]
   conflicts.append({'engine_key':list(key),'registrations':[r['registration_key'] for r in rr],'distinct_source_sha256':len({r['sha256'] for r in rr}),'default_config_conflict':len(scope)>1,'default_config_registrations':[r['registration_key'] for r in scope],'winner':'not_assumed'})
 compact=[]
 for r in regs:
  compact.append({k:r[k] for k in ('registration_key','logical_key','catalog_match','sha256','engine_enabled_path','extraction','source_classification','default_config_scope','referenced_monster_slot_count','same_logical_key_other_registrations','same_engine_key_other_registrations')})
  compact[-1].update(file=r['record']['file'],git_blob=r['record']['blob'],registrar=r['record']['registrar'],cast_tier=r['record']['cast']['tier'],catalog_identities=[m['identity'] for m in r['candidate_identity_matches']])
 write(label+'-registration-inventory.json',compact)
 write(label+'-registered-spells.json',regs);write(label+'-file-classification.json',rows);write(label+'-engine-key-conflicts.json',conflicts);write(label+'-unreadable-registration-files.json',errors)
 enabled=[r for r in regs if r['engine_enabled_path']]
 all_summary[label]={'revision':metadata['revision'],'classification_file_rows':len(rows),'classification_counts':dict(collections.Counter(c for r in rows for c in r['source_classifications'])),'enabled_registered_rows_all_datapacks':len(enabled),'default_config_registered_rows':sum(r['default_config_scope'] for r in enabled),'distinct_enabled_logical_keys':len({tuple(r['logical_key']) for r in enabled}),'unresolved_declared_registration_files':sum(bool(e['lexical_candidates']) for e in errors),'opaque_helper_load_errors':sum(not e['lexical_candidates'] for e in errors),'default_config_engine_key_conflicts':sum(c['default_config_conflict'] for c in conflicts)}
write('source-classification-summary.json',{'schema':'OTERYN_SOURCE_CLASSIFICATION/v1','catalog_active_definitions':len(cat['bundles']),'catalog_removed_decisions':len(cat['removed']),'sources':all_summary,'all_snapshot_distinct_enabled_source_logical_keys':len(all_keys),'all_snapshot_distinct_without_player_catalog_or_removal':len(missing_keys),'note':'Missing from player catalogue does not imply missing from source-only monster dependency carrier or runtime implementation. Neither source census nor provenance is gameplay qualification.'})
for old,new in [('canary','canary-main-current'),('crystal','crystal-summer-current')]:
 if old not in summ['sources'] or new not in summ['sources']:continue
 before={r['file']:r for r in read(old+'-tracked-lua-files.json')};after={r['file']:r for r in read(new+'-tracked-lua-files.json')}
 changed=[]
 for path in sorted(before.keys()|after.keys()):
  l=before.get(path);r=after.get(path)
  if l and r and l['sha256']==r['sha256']:continue
  changed.append({'file':path,'change':'added' if l is None else 'deleted' if r is None else 'modified','before':l,'after':r})
 write(new+'-delta-from-approved-pin.json',{'approved':summ['sources'][old]['revision'],'observed_current':summ['sources'][new]['revision'],'changed_lua_files':changed})

write('existing-catalog-provenance-index.json',{'entries':[{'logical_key':list(k),'matches':v} for k,v in sorted(active.items())],'removed':cat['removed'],'match_basis':'Name plus carrier only. Full source-specific semantics remain separately auditable; no assertion that every variant was imported.'})
