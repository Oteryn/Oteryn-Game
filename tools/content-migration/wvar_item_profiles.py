"""Native World-change operands, admitted by the existing source Item importer.

Sandfish13992: Crystal201189f items.xml:32101-32104 weight660 and food;
foods.lua:97,150 gives55*12=660 regenerationseconds. All portability, stack,
container and collision facts come from the same pinned appearances.dat.
The canonical Item already admits its physical/consumption facts; this module
closes its missing NativeGameplay membership. The same verified source admits
six genuine Bibby/Wyda loot leaves and their exact actor-specific loot tables.
"""
import argparse
import copy
import hashlib
import json
import re
from pathlib import Path
import sys
import tarfile

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/content-schema/native-gameplay'))
import build_source_item_profiles as native
from grizzly_shop_profiles import accepted_first

REVISION='201189f871d23597b82caf88fd611454a2ddefb8'
PATH='tools/content-schema/native-gameplay/wvar-item-profiles.json'
SOURCES=(('data/items/items.xml','8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'),
         ('data/items/appearances.dat','17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1'),
         ('data/scripts/actions/items/foods.lua','6023845b3f982b4a7cc9cf3e43bb8d33f467e3e10f9bdf3f0713aeae88de902f'),
         ('data-crystal/monster/raids/giant_spider_wyda.lua','d64925aa3300e7bb6c8ec8abcfd815500624d344e4e2b411f8559624108993d6'),
         ('data-crystal/monster/bosses/bibby_bloodbath.lua','62bc99a21a214baaf6e1b0a8dba5ee595674c7baf15b0b3f952925ea3b58e5a2'))
KEY='oteryn:item.tibia.i13992'
REWARD_IDS=frozenset((8031,7412,7395,817,3316,3265))
OWNED_IDS=REWARD_IDS|{13992}
KEYS={f'oteryn:item.tibia.i{i}' for i in OWNED_IDS}
CANONICAL='content/items/definitions/items-03500-03999.json'


# A bounded successor of the existing WVAR source Item/corpse/loot producers.
REWARD_PATH='tools/content-schema/native-gameplay/wvar-reward-closure.json'
REWARD_ARCHIVE_SHA='2b734db10c8c00e7a11a59f1cb49a786099959e98a622732b7fb80bcc8345b9d'
REWARD_EXTRA_KEYS={'oteryn:creature.'+name for name in ('elf','elf_scout','elf_arcanist','elf_overseer','thornfire_wolf')}

def recipe_creature_keys(root=ROOT):
    keys=set(REWARD_EXTRA_KEYS)
    for relative in ['content/encounters/world-changes/variants.json','content/encounters/world-changes/variants-extra.json']:
        for definition in json.loads((root/relative).read_bytes()):
            for variant in definition['variants']:
                keys.update(row['creature'] for row in variant['spawns'])
    return keys

def build_recipe_reward_archive(archive,root=ROOT):
    if hashlib.sha256(archive.read_bytes()).hexdigest()!=REWARD_ARCHIVE_SHA:
        raise ValueError('WVAR_REWARD_ARCHIVE_SHA')
    keys=recipe_creature_keys(root)-{'oteryn:creature.mamma_longlegs'}
    wanted={key.rsplit('.',1)[1]:key for key in keys};lua={};captured={}
    source_paths={p:h for p,h in SOURCES[:2]}
    source_paths['src/items/items.hpp']='06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca'
    with tarfile.open(archive) as donor:
        for member in donor:
            path=member.name.split('/',1)[-1]
            if path in source_paths:
                value=donor.extractfile(member).read()
                if hashlib.sha256(value).hexdigest()!=source_paths[path]:raise ValueError('WVAR_REWARD_BASE_SOURCE')
                captured[path]=value
            elif '/monster/' in path and path.endswith('.lua') and Path(path).stem in wanted:
                value=donor.extractfile(member).read()
                lua.setdefault(Path(path).stem,[]).append((path,value))
    if set(captured)!=set(source_paths) or set(lua)!=set(wanted):raise ValueError('WVAR_REWARD_SOURCE_CLOSURE')
    defaults=captured['src/items/items.hpp'].decode()
    if any(value not in defaults for value in ('uint16_t maxItems = 8','int32_t weight = 0','bool stopTime = false')):raise ValueError('WVAR_REWARD_DEFAULTS')
    xml=captured['data/items/items.xml'];index=native.xml_index(xml)
    appearances=captured['data/items/appearances.dat'];objects=native.engine_items.load_appearance_objects(appearances)
    bindings=native.binding_index(native.BINDINGS.read_bytes());lines=xml.decode().splitlines()
    # XML nodes may be ranges. Source line receipts are computed once per original node.
    spans={};stack=[]
    for n,line in enumerate(lines):
        match=re.search(r'<item\b([^>]*)>',line)
        if match:
            attrs=match[1];ident=re.search(r'\bid="(\d+)"',attrs);first=re.search(r'\bfromid="(\d+)"',attrs);last=re.search(r'\btoid="(\d+)"',attrs)
            if ident or first:
                ids=[int(ident[1])] if ident else range(int(first[1]),int(last[1])+1)
                stack=[n,list(ids)]
                if line.rstrip().endswith('/>'):
                    for i in stack[1]:spans[i]=(n,n)
                    stack=[]
        if '</item>' in line and stack:
            for i in stack[1]:spans[i]=(stack[0],n)
            stack=[]
    def xml_receipt(i):
        start,end=spans[i];span=('\n'.join(lines[start:end+1])+'\n').encode()
        return {'path':'data/items/items.xml','line_start':start+1,'line_end':end+1,'sha256':source_paths['data/items/items.xml'],'span_sha256':hashlib.sha256(span).hexdigest()}
    creatures=[];required=set();corpses={}
    for stem,key in sorted(wanted.items()):
        candidates=sorted(lua[stem],key=lambda row:(not row[0].startswith('data-crystal/'),row[0]))
        path,value=candidates[0];text=value.decode();source_lines=text.splitlines()
        corpse_match=re.search(r'monster\.corpse\s*=\s*(\d+)',text)
        if not corpse_match:raise ValueError('WVAR_REWARD_CORPSE_SOURCE:'+key)
        corpse=int(corpse_match[1]);current=corpse;seen=set()
        while current:
            if current in seen:raise ValueError('WVAR_REWARD_CORPSE_CYCLE')
            seen.add(current);required.add(current);attrs=native.attrs(index[current]);obj=objects[current]
            capacity=int(attrs.get('containersize','8')) if obj['flags'].get(5,False) else None
            if 'containersize' in attrs:capacity=int(attrs['containersize'])
            target=int(attrs.get('decayto','0'));duration=int(attrs.get('duration','0'))*1000
            fact={'item':current,'capacity':capacity,'duration_ms':duration,'target':target or None,'source':xml_receipt(current)}
            if current in corpses and corpses[current]!=fact:raise ValueError('WVAR_REWARD_CORPSE_CONFLICT')
            corpses[current]=fact;current=target
        loot=None;loot_start=None;loot_end=None
        if 'monster.loot = {' in text:
            loot_start=next(n for n,line in enumerate(source_lines) if 'monster.loot = {' in line)
            if source_lines[loot_start].strip().endswith('{}'):loot_end=loot_start
            else:loot_end=next(n for n in range(loot_start+1,len(source_lines)) if source_lines[n].strip()=='}')
            entries=[]
            for line in source_lines[loot_start+1:loot_end]:
                line=line.split('--',1)[0]
                body=re.search(r'\{([^{}]+)\}',line)
                if not body:continue
                body=body[1];ident=re.search(r'\bid\s*=\s*(\d+)',body)
                if ident:item=int(ident[1])
                else:
                    name=re.search(r'\bname\s*=\s*"([^"]+)"',body)[1]
                    matches=[i for i,node in index.items() if node.get('name')==name]
                    if len(matches)!=1:raise ValueError('WVAR_REWARD_LOOT_NAME:'+name)
                    item,=matches
                chance=int(re.search(r'\bchance\s*=\s*(\d+)',body)[1]);maximum=re.search(r'\bmaxCount\s*=\s*(\d+)',body)
                if not 0<chance<=100000:raise ValueError('WVAR_REWARD_LOOT_PROBABILITY')
                required.add(item);entries.append({'item':copy.deepcopy(bindings[str(item)]['target']),'min_count':1,'max_count':int(maximum[1]) if maximum else 1,'probability_ppm':chance*10})
            if entries:loot={'identity':{'family':'Loot','key':'oteryn:loot.creature.'+stem,'revision':'definition-r1'},'algorithm':'IndependentBernoulliPpm','entries':entries}
        creatures.append({'creature':key,'corpse':corpse,'loot':loot,'source':{'path':path,'sha256':hashlib.sha256(value).hexdigest(),'corpse_line':text[:corpse_match.start()].count('\n')+1,'loot_line_start':None if loot_start is None else loot_start+1,'loot_line_end':None if loot_end is None else loot_end+1}})
    physical={row['server_item_id']:row for row in native.policies(appearances,xml,required)}
    records=[]
    for i in sorted(required):
        row=native.profile(i,index[i],objects[i],physical[i],bindings,None)
        # This owner qualifies physical rewards, not equipment/combat/effect systems.
        # Keep those existing owners' unknown/known facts untouched.
        supported={'physical','stack','container','temporal'}
        if i not in corpses:supported.add('charges')
        row['semantics']={field:copy.deepcopy(value) if field in supported else {'state':'UNKNOWN'} for field,value in row['semantics'].items()}
        records.append(row)
    records=native.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,set(index))['records']
    return {'schema':'OTERYN_WVAR_SOURCE_REWARD_CLOSURE/v1','source_revision':REVISION,'archive_sha256':REWARD_ARCHIVE_SHA,'sources':[{'path':p,'sha256':h} for p,h in source_paths.items()],'binding_table_sha256':hashlib.sha256((root/'imports/crystalserver/bindings/items.json').read_bytes()).hexdigest(),'recipe_creature_keys':sorted(recipe_creature_keys(root)),'creatures':creatures,'corpses':list(sorted(corpses.values(),key=lambda row:row['item'])),'records':records,'item_sources':[{'item':i,**xml_receipt(i)} for i in sorted(required)]}

def recipe_reward_overlay(root=ROOT):
    value=json.loads((root/REWARD_PATH).read_bytes())
    if value['schema']!='OTERYN_WVAR_SOURCE_REWARD_CLOSURE/v1' or value['archive_sha256']!=REWARD_ARCHIVE_SHA or value['recipe_creature_keys']!=sorted(recipe_creature_keys(root)):
        raise ValueError('WVAR_REWARD_RECIPE_SCOPE')
    if value.get('binding_table_sha256')!=hashlib.sha256((root/'imports/crystalserver/bindings/items.json').read_bytes()).hexdigest():
        raise ValueError('WVAR_REWARD_BINDINGS_SHA')
    return value



def apply_recipe_reward_canonical(definitions,root=ROOT):
    value=recipe_reward_overlay(root);indexed={d['identity']['key']:d for d in definitions};prepared=[]
    legacy={5977:30,4038:30,4039:20}
    for row in value['records']:
        key=row['production_definition']['production_key'];definition=indexed[key]
        if definition['identity']!=row['production_binding']['target']:raise ValueError('WVAR_REWARD_BINDING')
        semantics=copy.deepcopy(definition.get('semantics',{field:{'state':'UNKNOWN'} for field in row['semantics']}))
        # Correct only our earlier16-slot corpse projection; it was not a donor fact.
        item=int(key.rsplit('i',1)[1])
        if item in legacy and semantics['container']=={'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':16}}}:
            semantics['container']=copy.deepcopy(row['semantics']['container'])
        merged=accepted_first(semantics,row['semantics'])
        replacement=copy.deepcopy(definition);replacement.update(materializable=True,stack_class=row['admission']['stack_class'],semantics=merged)
        prepared.append((definition,replacement))
    for definition,replacement in prepared:definition.clear();definition.update(replacement)

def admit_recipe_reward_closure(root,generated):
    value=recipe_reward_overlay(root)
    item_path='content/items/definitions/spell-native-profiles.json';manifest_path='content/spells.manifest.json'
    items=json.loads(generated[item_path]);known={r['production_definition']['production_key']:r for r in items['records']}
    for row in value['records']:
        key=row['production_definition']['production_key']
        if key not in known:items['records'].append(copy.deepcopy(row));known[key]=row
        elif not known[key]['admission']['materializable']:raise ValueError('WVAR_REWARD_EXISTING_ITEM_REFUSED')
    items['records'].sort(key=lambda row:row['production_definition']['production_key'])
    generated[item_path]=(json.dumps(items,indent=2,ensure_ascii=False)+'\n').encode()
    manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=hashlib.sha256(generated[item_path]).hexdigest()
    generated[manifest_path]=(json.dumps(manifest,sort_keys=True,separators=(',',':'))+'\n').encode()
    path='tools/content-schema/native-gameplay/loot-tables.json';document=json.loads(generated.get(path,(root/path).read_bytes()))
    profiles={row['profile']['target']['key']:row for row in json.loads(generated['content/creatures/definitions/spell-native-profiles.json'])['records']}
    bindings={row['creature']['key']:row for row in document['creature_loot']};tables={row['identity']['key']:i for i,row in enumerate(document['tables'])}
    primary=set();leaf=set()
    for row in value['creatures']:
        key=row['creature'];profile=profiles[key]['profile']['data']['profile'];expected=f'oteryn:item.tibia.i{row["corpse"]}'
        if profile['details']['corpse_item']['key']!=expected:raise ValueError('WVAR_REWARD_ACTOR_CORPSE:'+key)
        primary.add(row['corpse']);loot=row['loot'];bindings[key]['loot']=None if loot is None else copy.deepcopy(loot['identity'])
        if loot is not None:
            if loot['identity']['key'] not in tables:raise ValueError('WVAR_REWARD_TABLE_MISSING')
            document['tables'][tables[loot['identity']['key']]]=copy.deepcopy(loot)
            leaf.update(int(e['item']['key'].rsplit('i',1)[1]) for e in loot['entries'])
    source_items={int(row['production_definition']['production_key'].rsplit('i',1)[1]):row for row in value['records']}
    facts={row['item']['key']:row for row in document['items']}
    for item in sorted(primary|leaf):
        row=source_items[item];key=row['production_definition']['production_key'];fact=facts.get(key)
        if fact is None:
            fact={'item':copy.deepcopy(row['production_binding']['target'])};document['items'].append(fact)
        sem=row['semantics'];container=sem['container'];capacity=container.get('value',{}).get('capacity',{}).get('value')
        fact.update(materializable=True,container_capacity=capacity)
        if item in leaf:
            fact['stackable']=sem['stack']['value']['stackable']['value']
            charges=sem['charges'];fact['charges']=charges.get('value',{}).get('count',{}).get('value')
    document['items'].sort(key=lambda row:row['item']['key'])
    # Preserve the established row-oriented JSON representation, keeping review diffs bounded.
    generated[path]=('{"schema":'+json.dumps(document['schema'])+',\n'+',\n'.join(json.dumps(k)+':[\n'+',\n'.join(json.dumps(row,sort_keys=True,separators=(',',':'),ensure_ascii=False) for row in document[k])+'\n]' for k in ['creature_loot','tables','items'])+'}\n').encode()


def read_archive(archive):
    with tarfile.open(archive) as donor:
        rows=[]
        for path,_ in SOURCES:
            matches=[m for m in donor.getmembers() if m.name.endswith('/'+path)]
            if len(matches)!=1 or REVISION not in matches[0].name:
                raise ValueError('WVAR_ITEM_SOURCE_REVISION')
            rows.append((path,donor.extractfile(matches[0]).read()))
        return rows


def build_sources(sources):
    if [(path,hashlib.sha256(raw).hexdigest()) for path,raw in sources]!=list(SOURCES):
        raise ValueError('WVAR_ITEM_SOURCE_HASH')
    captured=dict(sources)
    xml,appearances,foods=[captured[path] for path,_ in SOURCES[:3]]
    index=native.xml_index(xml)
    row=index[13992]
    if native.attrs(row)!={'primarytype':'food','weight':'660'} or row.get('name')!='sandfish':
        raise ValueError('WVAR_SANDFISH_XML_FACTS')
    if '[13992] = { 55, "Munch." }' not in foods.decode() or 'player:feed(itemFood[1] * 12)' not in foods.decode():
        raise ValueError('WVAR_SANDFISH_FOOD_FACTS')
    bindings=native.binding_index(native.BINDINGS.read_bytes())
    objects=native.engine_items.load_appearance_objects(appearances)
    physical={r['server_item_id']:r for r in native.policies(appearances,xml,OWNED_IDS)}
    records=[native.profile(i,index[i],objects[i],physical[i],bindings,None) for i in sorted(OWNED_IDS)]
    record=next(r for r in records if r['authoring']['item']['key']=='candidate:item/13992')
    record['semantics']['consumption']={'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':660}}}
    bound=native.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,set(index))
    return {'schema':'OTERYN_NATIVE_WVAR_ITEM_PROFILES/v1','producer':'tools/content-migration/wvar_item_profiles.py','source_revision':REVISION,
            'source_files':[{'path':p,'sha256':h} for p,h in SOURCES],
            'binding_table_sha256':hashlib.sha256(native.BINDINGS.read_bytes()).hexdigest(),'records':bound['records'],
            'loot_tables':loot_sources(captured,index,bindings)}


def loot_sources(captured,index,bindings):
    result=[]
    for creature,path in [('giant_spider_wyda',SOURCES[3][0]),('bibby_bloodbath',SOURCES[4][0])]:
        text=captured[path].decode().split('monster.loot = {',1)[1].split('\n}',1)[0]
        entries=[]
        for body in re.findall(r'\{([^{}]+)\}',text):
            identifier=re.search(r'\bid\s*=\s*(\d+)',body)
            if identifier: item=int(identifier[1])
            else:
                name=re.search(r'\bname\s*=\s*"([^"]+)"',body)[1]
                matches=[i for i,node in index.items() if node.get('name')==name]
                if len(matches)!=1:raise ValueError('WVAR_LOOT_NAME_BINDING')
                item,=matches
            chance=int(re.search(r'\bchance\s*=\s*(\d+)',body)[1])
            maximum=re.search(r'\bmaxCount\s*=\s*(\d+)',body)
            if not 0<chance<=100000:raise ValueError('WVAR_LOOT_PROBABILITY')
            entries.append({'item':copy.deepcopy(bindings[str(item)]['target']),
                'min_count':1,'max_count':int(maximum[1]) if maximum else 1,'probability_ppm':chance*10})
        expected=2 if creature=='giant_spider_wyda' else 17
        if len(entries)!=expected:raise ValueError('WVAR_LOOT_SOURCE_SET')
        result.append({'identity':{'family':'Loot','key':f'oteryn:loot.creature.{creature}','revision':'definition-r1'},
            'algorithm':'IndependentBernoulliPpm','entries':entries})
    return result

def build_archive(archive): return build_sources(read_archive(archive))


def overlay(root):
    value=json.loads((root/PATH).read_bytes())
    if value['schema']!='OTERYN_NATIVE_WVAR_ITEM_PROFILES/v1' or value['source_revision']!=REVISION or value['source_files']!=[{'path':p,'sha256':h} for p,h in SOURCES] or value['binding_table_sha256']!=hashlib.sha256((root/'imports/crystalserver/bindings/items.json').read_bytes()).hexdigest():
        raise ValueError('WVAR_ITEM_PROFILE_PROVENANCE')
    if {r['production_definition']['production_key'] for r in value['records']}!=KEYS or len(value['records'])!=len(KEYS):
        raise ValueError('WVAR_ITEM_PROFILE_SET')
    return value


def apply_canonical_admissions(definitions,root):
    indexed={d['identity']['key']:d for d in definitions}
    replacements=[]
    for row in overlay(root)['records']:
        key=row['production_definition']['production_key']
        if key==KEY: continue
        definition=indexed[key]
        if definition['identity']!=row['production_binding']['target']:
            raise ValueError('WVAR_REWARD_CANONICAL_IDENTITY')
        replacement={'kind':'Item','identity':copy.deepcopy(definition['identity']),
            'client_projection':copy.deepcopy(definition['client_projection']),
            'materializable':True,'stack_class':row['admission']['stack_class'],
            'semantics':accepted_first(definition['semantics'],row['semantics'])}
        if definition.get('materializable') is True and definition!=replacement:
            raise ValueError('WVAR_REWARD_CANONICAL_POLICY_CONFLICT')
        replacements.append((definition,replacement))
    for definition,replacement in replacements:
        definition.clear();definition.update(replacement)
    if (root/REWARD_PATH).is_file():apply_recipe_reward_canonical(definitions,root)


def canonical_outputs(root):
    shards={p:json.loads(p.read_bytes()) for p in (root/'content/items/definitions').glob('items-*.json')}
    apply_canonical_admissions([r['definition'] for d in shards.values() for r in d['records']],root)
    keys={f'oteryn:item.tibia.i{i}' for i in REWARD_IDS}
    if (root/REWARD_PATH).is_file():keys.update(row['production_definition']['production_key'] for row in recipe_reward_overlay(root)['records'])
    return {p:(json.dumps(d,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode()
        for p,d in shards.items() if any(r['definition']['identity']['key'] in keys for r in d['records'])}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    rows=overlay(root)['records']
    definitions={r['definition']['identity']['key']:r['definition']
        for p in (root/'content/items/definitions').glob('items-*.json')
        for r in json.loads(p.read_bytes())['records']}
    for row in rows:
        key=row['production_definition']['production_key'];definition=definitions[key]
        for field in (('physical','stack','consumption') if key==KEY else ('physical','stack','container','temporal')):
            if definition['semantics'][field]!=row['semantics'][field]:raise ValueError('WVAR_ITEM_CANONICAL_FACTS')
        if (definition['identity']!=row['production_binding']['target'] or not definition['materializable']
                or definition['stack_class']!=row['admission']['stack_class']
                or row['authoring']['item']!={'family':'Item','key':f"candidate:item/{key.rsplit('i',1)[-1]}",'revision':'spell-p2-r21'}):raise ValueError('WVAR_ITEM_CANONICAL_IDENTITY')
    document=json.loads(generated[profile_path])
    if any(r['production_definition']['production_key'] in KEYS for r in document['records']):raise ValueError('WVAR_ITEM_PROFILE_DUPLICATE')
    document['records']=sorted(document['records']+copy.deepcopy(rows),key=order)
    data=(json.dumps(document,indent=2,ensure_ascii=False)+'\n').encode();generated[profile_path]=data
    manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=hashlib.sha256(data).hexdigest();generated[manifest_path]=canonical(manifest)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--crystal-archive',type=Path,required=True);parser.add_argument('--check',action='store_true');parser.add_argument('--canonical',action='store_true');parser.add_argument('--recipe-rewards',action='store_true');args=parser.parse_args()
    raw=(json.dumps(build_archive(args.crystal_archive),indent=2,ensure_ascii=False)+'\n').encode()
    if args.check:
        if (ROOT/PATH).read_bytes()!=raw: raise SystemExit('WVAR_ITEM_PROFILE_DRIFT')
    else: (ROOT/PATH).write_bytes(raw)
    if args.recipe_rewards:
        captured=build_recipe_reward_archive(args.crystal_archive)
        receipt=(json.dumps(captured,indent=2)+'\n').encode()
        if args.check:
            if (ROOT/REWARD_PATH).read_bytes()!=receipt:raise SystemExit('WVAR_REWARD_SOURCE_DRIFT')
        else:(ROOT/REWARD_PATH).write_bytes(receipt)
        print(f'WVAR_RECIPE_REWARD_SOURCE_VERIFIED creatures={len(captured["creatures"])} corpses={len(captured["corpses"])} items={len(captured["records"])}')
    if args.canonical:
        for path,data in canonical_outputs(ROOT).items():
            if args.check:
                if path.read_bytes()!=data:raise SystemExit(f'WVAR_REWARD_CANONICAL_DRIFT:{path}')
            else:path.write_bytes(data)
    print(f'WVAR_ITEM_SOURCE_VERIFIED items={len(OWNED_IDS)} sandfish_food_seconds=660')
