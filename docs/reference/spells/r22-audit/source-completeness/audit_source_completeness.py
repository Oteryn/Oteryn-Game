"""Read-only exhaustive Git-tree spell registration/reference audit; never executes OTS engine."""
import argparse, collections, copy, hashlib, importlib.util, json, re, subprocess, sys
from pathlib import Path
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repository',type=Path,required=True)
parser.add_argument('--source-root',type=Path,required=True)
parser.add_argument('--out',type=Path,required=True)
parser.add_argument('--snapshot',action='append',help='LABEL=canary|crystal@REVISION; repeat for each exact source tree')
args=parser.parse_args()
BASE=args.repository/'tools/content-schema'
OUT=args.out
OUT.mkdir(parents=True,exist_ok=True)
sys.path.insert(0,str(BASE/'monster-authoring'))
import spell_census as monster_census
spec=importlib.util.spec_from_file_location('player_census',BASE/'spell-authoring/spell_census.py')
player_census=importlib.util.module_from_spec(spec); spec.loader.exec_module(player_census)
def git(root,*args): return subprocess.check_output(['git','-C',str(root),*args])
def write(path,value): path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
old=json.loads((BASE/'spell-authoring/samples/executable-spell-catalog.json').read_text())
active={(b['bundle']['spell']['carrier'],b['bundle']['spell']['name'].casefold()) for b in old['bundles']}
removed={(b['carrier'],b['name'].casefold()) for b in old['removed']}
output={'schema':'OTERYN_EXHAUSTIVE_UPSTREAM_SPELL_AUDIT/v1','scope':'Every tracked Lua file in exact Git trees; engine not executed; source registrations are evidence, not runtime completion or official Tibia authority','sources':{}}
snapshots=[]
for value in args.snapshot or ['canary=canary@99902524e052f37574194466c2949c576e4ab269','crystal=crystal@ff7ede593c69d4c658b382c97443e8155926924a']:
    label,target=value.split('=',1); upstream,ref=target.split('@',1)
    assert re.fullmatch(r'[a-z0-9-]+',label) and upstream in ('canary','crystal') and re.fullmatch(r'[0-9a-f]{40}',ref)
    snapshots.append((label,upstream,ref))
for source, upstream, ref in snapshots:
    checkout=args.source_root/upstream
    revision=git(checkout,'rev-parse',ref).decode().strip()
    tree=[]
    for line in git(checkout,'ls-tree','-rz','--full-tree',revision).split(b'\0'):
        if not line: continue
        meta,path=line.split(b'\t'); mode,kind,blob=meta.decode().split(); path=path.decode()
        if kind=='blob' and (path.endswith('.lua') or path.endswith('.xml') or path in player_census.ENUM_HEADERS): tree.append((path,blob))
    proc=subprocess.Popen(['git','-C',str(checkout),'cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
    payload,_=proc.communicate(('\n'.join(blob for path,blob in tree)+'\n').encode())
    assert proc.returncode==0
    pos=0; files=[]; snapshot=OUT/'upstream-local-only'/source
    for path,blob in tree:
        end=payload.index(b'\n',pos); header=payload[pos:end].decode().split(); size=int(header[2]); data=payload[end+1:end+1+size];pos=end+size+2
        assert header[0]==blob and hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==blob
        dst=snapshot/path;dst.parent.mkdir(parents=True,exist_ok=True);dst.write_bytes(data)
        if not path.endswith('.lua'): continue
        text=data.decode('utf8',errors='replace'); files.append((path,blob,data,text))
    census=player_census.Census(snapshot,upstream)
    registrations=[]; failures=[]; lexical_candidates=[]; all_files=[]; compatibility_recoveries=[]
    for path,blob,data,text in files:
        all_files.append({'file':path,'git_blob':blob,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'present_in_original_worktree':(checkout/path).is_file()})
        if not re.search(r'\bSpell\s*\(',text): continue
        constructs=re.findall(r'(\w+)\s*=\s*Spell\s*\(\s*["\'](\w+)["\']',text)
        lexical=[]
        for variable,kind in constructs:
            for quote, name in re.findall(re.escape(variable)+r'\s*:\s*name\s*\(\s*(["\'])(.*?)\1',text):
                lexical.append({'carrier':kind.lower(),'name':name,'register_call_present':bool(re.search(re.escape(variable)+r'\s*:\s*register\s*\(',text))})
        lexical_candidates.append({'file':path,'git_blob':blob,'sha256':hashlib.sha256(data).hexdigest(),'lexical_candidates':lexical})
        result=census.evaluate(path,data)
        extraction='original_player_census_sandbox'
        if 'error' in result and lexical:
            original_error=result['error']
            retry=copy.copy(census); retry.helpers=list(census.helpers)
            compatibility_path='data/libs/compat/compat.lua'
            compatibility_bytes=(snapshot/compatibility_path).read_bytes()
            shims='setCombatCallback = function(c, ...) return c:setCallback(...) end\ncreateConditionObject = Condition\nsetConditionParam = function(c, ...) return c:setParameter(...) end\naddOutfitCondition = function(c, ...) return c:setOutfit(...) end'
            vocation_path='data/libs/functions/vocation.lua'
            vocation_bytes=(snapshot/vocation_path).read_bytes()
            vocation_table=vocation_bytes.decode().split('function Vocation.',1)[0]
            retry.helpers.extend([vocation_table,shims])
            harness_sources=[{'file':compatibility_path,'sha256':hashlib.sha256(compatibility_bytes).hexdigest(),'aliases':['setCombatCallback = Combat.setCallback','createConditionObject = Condition','setConditionParam = Condition.setParameter','addOutfitCondition = Condition.setOutfit']},{'file':vocation_path,'sha256':hashlib.sha256(vocation_bytes).hexdigest(),'used':'exact VOCATION table preceding method definitions'}]
            if 'gaz_functions.lua' in text:
                dependency=path.rsplit('/',1)[0]+'/gaz_functions.lua'
                dependency_bytes=(snapshot/dependency).read_bytes()
                exact_path=path.split('/')[0]+'/scripts/spells/monster/gaz_functions.lua'
                retry.helpers.append('DATA_DIRECTORY = '+json.dumps(path.split('/')[0])+'\ndofile = function(path) if path ~= '+json.dumps(exact_path)+' then error("unqualified dofile") end '+dependency_bytes.decode()+' end')
                harness_sources.append({'file':dependency,'sha256':hashlib.sha256(dependency_bytes).hexdigest(),'used':'only exact source-qualified dofile dependency permitted'})
            recovered=retry.evaluate(path,data)
            if 'error' not in recovered:
                result=recovered; extraction='bounded_source_compatibility_alias_sandbox'
                compatibility_recoveries.append({'file':path,'sha256':hashlib.sha256(data).hexdigest(),'original_error':original_error,'harness_sources':harness_sources,'limitation':'Registration extraction only; no engine, world or custom callback execution is proven.'})
        if 'error' in result:
            failures.append({**result,'sha256':hashlib.sha256(data).hexdigest(),'lexical_candidates':lexical})
            continue
        for index,record in enumerate(result['spells']):
            key=(record['spell_type'],str(record['name']).casefold())
            status='active_catalog' if key in active else 'source_proven_removed' if key in removed else 'missing_from_player_catalog'
            registrations.append({'registration_key':source+'/'+path+'#'+str(index+1),'logical_key':list(key),'catalog_match':status,'sha256':hashlib.sha256(data).hexdigest(),'engine_enabled_path':not any(part.startswith('#') for part in Path(path).parts),'extraction':extraction,'record':record})
    print(source,'registrations',len(registrations),'failures',len(failures),flush=True)
    monster_slots=[];monster_errors=[];monster_files=[]
    for path,blob,data,text in files:
        if not re.search(r'\bcreateMonsterType\s*\(',text):continue
        monster_files.append(path)
        try: record,error=monster_census.load_monster(snapshot/path)
        except Exception as exc:
            monster_errors.append({'file':path,'git_blob':blob,'sha256':hashlib.sha256(data).hexdigest(),'error':str(exc)[:300]});continue
        if error:monster_errors.append({'file':path,'git_blob':blob,'sha256':hashlib.sha256(data).hexdigest(),'error_after_registered':error})
        for block in ('attacks','defenses'):
            values=record.get(block) or []
            sequence=values.get('_list',[]) if isinstance(values,dict) else values
            for index,entry in enumerate(sequence):
                if not isinstance(entry,dict):continue
                monster_slots.append({'slot_key':source+'/'+path+'/'+block+'/'+str(index+1),'file':path,'git_blob':blob,'sha256':hashlib.sha256(data).hexdigest(),'block':block,'index':index+1,'monster_name':record.get('name'), 'name':str(entry.get('name','')).casefold(),'entry':entry})
    registered_names=collections.defaultdict(list)
    for r in registrations: registered_names[r['logical_key'][1]].append(r['registration_key'])
    for slot in monster_slots:
        slot['registered_spell_candidates']=registered_names.get(slot['name'],[])
        slot['resolution']='registered_source_spell' if slot['name'] in registered_names else 'inline_engine_kind' if slot['name'] in monster_census.INLINE else 'unresolved_source_reference'
    count=collections.Counter(r['catalog_match'] for r in registrations)
    logical=collections.defaultdict(list)
    for r in registrations:logical[tuple(r['logical_key'])].append(r['registration_key'])
    summary={'tracked_lua_files':len(files),'original_worktree_missing_lua_files':sum(not r['present_in_original_worktree'] for r in all_files),'spell_constructor_files':len(lexical_candidates),'sandbox_registered_rows':len(registrations),'enabled_path_rows':sum(r['engine_enabled_path'] for r in registrations),'distinct_registered_logical_keys':len(logical),'registration_files_unreadable':len(failures),'compatibility_recovered_registration_files':len(compatibility_recoveries),'catalog_match_rows':dict(count),'monster_constructor_files':len(monster_files),'monster_slots':len(monster_slots),'monster_distinct_slot_names':len({s['name'] for s in monster_slots}),'monster_load_findings':len(monster_errors),'slot_resolution':dict(collections.Counter(s['resolution'] for s in monster_slots))}
    write(OUT/(source+'-tracked-lua-files.json'),all_files)
    write(OUT/(source+'-registered-spells.json'),registrations)
    write(OUT/(source+'-unreadable-registration-files.json'),failures)
    write(OUT/(source+'-compatibility-recovered-registration-files.json'),compatibility_recoveries)
    write(OUT/(source+'-lexical-spell-constructor-files.json'),lexical_candidates)
    write(OUT/(source+'-monster-attack-defense-slots.json'),monster_slots)
    write(OUT/(source+'-monster-load-findings.json'),monster_errors)
    write(OUT/(source+'-duplicate-logical-registrations.json'),[{'logical_key':list(k),'registrations':v} for k,v in sorted(logical.items()) if len(v)>1])
    output['sources'][source]={'revision':revision,'summary':summary,'missing_player_catalog_logical_keys':[list(k) for k in sorted(logical) if k not in active and k not in removed]}
    print(source,json.dumps(summary),flush=True)
write(OUT/'source-completeness-summary.json',output)
