"""Reproduce bounded name-candidate comparisons against an immutable source pin.

A name match, including a unique match, is not a qualified client/server ID.
No source field is promoted to runtime admission. Trade observations are retained
separately until service/currency/residence and direction mappings are qualified.
"""
import argparse,collections,decimal,hashlib,json,os,pathlib,re,subprocess

HERE=pathlib.Path(__file__).resolve().parent
REPO=next(p for p in HERE.parents if (p/'.git').exists())
PIN='5efd67da164ac9dc33c8c066434b228cebb86c2b'


def blob(path,receipts):
    raw=subprocess.check_output(['git','show',PIN+':'+path],cwd=REPO)
    receipts[path]={'repository_path':path,'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),
                    'git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()}
    return json.loads(raw)


def field(semantics,path):
    node=semantics;visited=[]
    for part in path:
        if isinstance(node,dict)and 'state'in node:
            if node['state']!='KNOWN':return {'resolution':'ancestor_'+node['state'],'ancestor_path':'.'.join(visited),'source':node}
            node=node['value']
        if not isinstance(node,dict)or part not in node:
            return {'resolution':'ABSENT','missing_path':'.'.join(visited+[part])}
        node=node[part];visited.append(part)
    return {'resolution':'PRESENT','source':node}


def known(snapshot):
    source=snapshot.get('source',{})
    return source.get('value')if snapshot['resolution']=='PRESENT'and source.get('state')=='KNOWN'else None


def compare(observed,snapshot,kind):
    actual=known(snapshot)
    if actual is None:return 'SOURCE_UNQUALIFIED_OR_UNKNOWN'
    if kind=='weight':
        match=re.fullmatch(r'(\d+\.\d{2}) oz',observed)
        if not match:return 'CLIENT_FORMAT_UNQUALIFIED'
        expected=decimal.Decimal(match[1])*100
        return 'CANDIDATE_FIELD_EQUAL'if expected==actual else 'CANDIDATE_FIELD_DIFFERENT'
    if observed not in ('yes','no'):return 'CLIENT_FORMAT_UNQUALIFIED'
    return 'CANDIDATE_FIELD_EQUAL'if (observed=='yes')==actual else 'CANDIDATE_FIELD_DIFFERENT'


# These are raw source-semantic comparisons, not an emulation of renderer omission.
ROUTES={
 'Weight':(['physical','weight'],'weight'),
 'Tradeable In Market':(['trade_restrictions','marketable'],'market'),
 'Armor':(['protection','armor'],'integer'),
 'Capacity':(['container','capacity'],'integer'),
 'Imbuement Slots':(['imbuement','slot_count'],'integer'),
 'Protection':(['protection','resistances'],'protection'),
 'Required Level':(['equipment','patterns'],'required_level'),
 'Vocations':(['equipment','patterns'],'vocations'),
 'Body Position':(['equipment','patterns'],'body_position'),
 'Skill Boost':(['skill_modifiers','modifiers'],'skill_boost'),
 'Mantra':(['skill_modifiers','modifiers'],'mantra'),
 'Damage Reflection':(['skill_modifiers','modifiers'],'reflection'),
 'Classification':(None,'forge'),
 'Required Magic Level':(None,'required_magic_level'),
}
def record_field(record,label,native):
    if ROUTES[label][0] is not None:
        semantics=record.get('semantics',{}) if native else record.get('definition',{}).get('semantics',{})
        return field(semantics,ROUTES[label][0])
    value=record.get('authoring',{}).get('forge' if label=='Classification' else 'required_magic_level')
    if value is None:return {'resolution':'ABSENT','missing_path':'authoring.'+label}
    return {'resolution':'PRESENT','source':{'state':'KNOWN','value':value['classification'] if label=='Classification' else value}}
def leaf(value):
    return value.get('value') if isinstance(value,dict) and value.get('state')=='KNOWN' else None
def projected(snapshot,kind):
    value=known(snapshot)
    if value is None:return None
    if kind in ('weight','market','integer','forge','required_magic_level'):return value
    if kind=='protection':
        if not isinstance(value,list):return None
        result=[]
        for resistance in value:
            v=leaf(resistance.get('percent'))
            if not isinstance(v,dict) or not v.get('denominator'):return None
            if resistance.get('kind')=='FIRE_FIELD':continue
            n=decimal.Decimal(v['numerator'])/v['denominator']
            label={'LIFE_DRAIN':'life drain','MANA_DRAIN':'mana drain'}.get(resistance['kind'],resistance['kind'].lower())
            result.append(label+' '+format(n,'+f').rstrip('0').rstrip('.')+'%' if n%1 else label+' '+format(int(n),'+d')+'%')
        return ', '.join(sorted(result))
    if kind in ('required_level','vocations','body_position'):
        if not value:return None
        key={'required_level':'level','vocations':'vocations','body_position':'primary_slot'}[kind]
        vals=[leaf(p.get(key)) for p in value]
        if any(v is None for v in vals) or any(v!=vals[0] for v in vals):return None
        if kind=='required_level':return vals[0]
        if kind=='vocations':return ', '.join(sorted(v.lower()+'s' for v in vals[0]))
        if vals[0]=='WEAPON':
            extra=[leaf(p.get('additional_reserved_slots')) for p in value]
            if any(v is None for v in extra):return None
            if all('SHIELD' in v for v in extra):return 'both hands'
        return {'HEAD':'head','TORSO':'body','LEGS':'legs','FEET':'feet','WEAPON':'weapon hand','SHIELD':'shield hand','AMULET':'neck','RING':'finger','CONTAINER':'container','EXTRA':'extra slot'}.get(vals[0])
    labels={'SKILL_DISTANCE':'distance fighting','SKILL_SWORD':'sword fighting','SKILL_AXE':'axe fighting','SKILL_CLUB':'club fighting','SKILL_FIST':'fist fighting','SKILL_SHIELD':'shielding','MAGIC_LEVEL_POINTS':'magic level','ENERGY_MAGIC_LEVEL_POINTS':'energy magic level','EARTH_MAGIC_LEVEL_POINTS':'earth magic level','FIRE_MAGIC_LEVEL_POINTS':'fire magic level','HOLY_MAGIC_LEVEL_POINTS':'holy magic level','ICE_MAGIC_LEVEL_POINTS':'ice magic level','DEATH_MAGIC_LEVEL_POINTS':'death magic level','HEALING_MAGIC_LEVEL_POINTS':'healing magic level','SPEED':'speed'}
    if kind in ('mantra','reflection'):
        desired='MANTRA' if kind=='mantra' else 'REFLECT_DAMAGE'
        found=[leaf(m.get('parameter')) for m in value if m['kind']==desired]
        return found[0].get('value') if len(found)==1 and isinstance(found[0],dict) else None
    result=[]
    for modifier in value:
        k=modifier['kind'];parameter=leaf(modifier.get('parameter'))
        if k=='MANTRA':continue
        if k not in labels or not isinstance(parameter,dict) or parameter.get('kind')!='SIGNED_POINTS' or not isinstance(parameter.get('value'),int):return None
        result.append(labels[k]+' '+format(parameter['value'],'+d'))
    return ', '.join(sorted(result)) if result else None
def compare_extended(observed,snapshot,kind):
    if kind in ('weight','market'):return compare(observed,snapshot,kind)
    value=projected(snapshot,kind)
    if value is None:return 'SOURCE_UNQUALIFIED_OR_UNKNOWN'
    if isinstance(value,int):
        if not re.fullmatch(r'[+-]?\d+',observed):return 'CLIENT_FORMAT_UNQUALIFIED'
        equal=int(observed)==value
    else:
        norm=lambda s:', '.join(sorted(v.strip().lower() for v in str(s).split(',')))
        equal=norm(observed)==norm(value)
    return 'CANDIDATE_FIELD_EQUAL' if equal else 'CANDIDATE_FIELD_DIFFERENT'

def generate():
    receipts={};paths=subprocess.check_output(['git','ls-tree','-r','--name-only',PIN,'content/items/definitions'],cwd=REPO,text=True).splitlines()
    canonical=[]
    for path in paths:
        if re.fullmatch(r'content/items/definitions/items-\d+-\d+\.json',path):
            for record in blob(path,receipts)['records']:canonical.append((record['definition'],path))
    manifest=blob('content/spells.manifest.json',receipts);selected={}
    for key in ('item_profiles','item_profile_updates','starting_loadout_item_profiles'):
        path=(pathlib.PurePosixPath('content')/manifest[key]['path']).as_posix()
        records=blob(path,receipts)['records'];assert receipts[path]['sha256']==manifest[key]['sha256']
        for record in records:selected[record['production_definition']['production_key']]=(record,path)
    names=collections.defaultdict(list)
    for definition,path in canonical:
        name=known(field(definition.get('semantics',{}),['presentation','name']))
        if isinstance(name,str):names[name.casefold()].append((definition,path,name))
    observations=[];input_receipts=[]
    for path in sorted(HERE.glob('observations-*.json')):
        raw=path.read_bytes();packet=json.loads(raw)
        input_receipts.append({'path':path.name,'sha256':hashlib.sha256(raw).hexdigest()})
        observations += [dict(item,category=packet['category'])for item in packet['items']]
    observations.sort(key=lambda item:item['ui_entry_ordinal'])
    assert observations and len({item['ui_entry_ordinal']for item in observations})==len(observations)
    results=[];classes=collections.Counter();field_classes=collections.Counter()
    for item in observations:
        candidates=names[item['fullName'].casefold()];classes['no_name_candidate'if not candidates else 'one_name_candidate'if len(candidates)==1 else 'multiple_name_candidates']+=1
        row={'capture_key':item['capture_key'],'category':item['category'],'ui_entry_ordinal':item.get('ui_entry_ordinal'),
             'observed_name':item['fullName'],'binding_status':'UNQUALIFIED_NAME_CORRESPONDENCE',
             'candidate_count':len(candidates),'candidates':[]}
        for definition,path,name in candidates:
            key=definition['identity']['key'];semantics=definition.get('semantics',{})
            candidate={'repository_key':key,'source_name':name,'match':'exact_name'if name==item['fullName']else'casefold_name',
                       'canonical_path':path,'canonical_materializable_literal':definition.get('materializable'),
                       'selected_membership':key in selected,'fields':{}}
            for label,(route,kind) in ROUTES.items():
                if label not in item['properties']:continue
                snapshot=record_field({'definition':definition},label,False);classification=compare_extended(item['properties'][label],snapshot,kind)
                detail={'client_literal':item['properties'][label],'canonical':snapshot,'comparison':classification,'comparison_layer':'RAW_SOURCE_SEMANTICS_NOT_RENDERER_OR_RUNTIME'}
                field_classes[classification]+=1
                if key in selected:
                    profile,profile_path=selected[key];selected_snapshot=record_field(profile,label,True)
                    detail['selected_profile']={'path':profile_path,'field':selected_snapshot,
                        'comparison':compare_extended(item['properties'][label],selected_snapshot,kind)}
                candidate['fields'][label]=detail
            if key in selected:candidate['selected_admission_literal']=selected[key][0].get('admission')
            row['candidates'].append(candidate)
        results.append(row)
    return {'schema':'OTERYN_PARTIAL_ITEMS_SOURCE_CANDIDATES/v1','classification':'DERIVED','source_commit':PIN,
        'input_observation_receipts':input_receipts,
        'scope':{'category':'Helmets and Hats','ordinals':[observations[0]['ui_entry_ordinal'],observations[-1]['ui_entry_ordinal']],'records':len(observations),'category_semantic_parity_complete':False,'observed_category_end_receipt':'category-end-receipt.json'if(HERE/'category-end-receipt.json').exists()else None,
                 'compared_fields':list(ROUTES),'non_narrative_fields_retained_but_not_source_compared':True,'narrative_text_retained_privately_with_hash_receipts':True},
        'identity_boundary':'Every repository key is an unqualified name candidate, including unique names; no UI ID or appearance mapping was read. Equal name/icon never collapses rows.',
        'weight_conversion_source':{'commit':PIN,'path':'crates/gameplay/dispatch/src/gameplay_transport/inspection.rs'},
        'weight_conversion':'Only two-decimal client oz converted to hundredths-of-ounce source integer; raw literals preserved.',
        'counts':{'canonical_records':len(canonical),'selected_membership':len(selected),'name_candidate_classes':dict(classes),'candidate_field_classes':dict(field_classes)},
        'not_claimed':['Runtime admission','Executable trade offers','Market value currentness','Complete source semantic parity','Source difference as gameplay bug','Raw Speed equality as formatter parity: pinned inspection.rs shifts speed right by1','Omitted aggregate component as proven retail absence','Old15.30 market metadata as forge classification'],
        'items':results,'source_receipts':list(receipts.values())}


if __name__=='__main__':
    p=argparse.ArgumentParser(__doc__);p.add_argument('--check',action='store_true');args=p.parse_args()
    result=generate();path=HERE/'source-candidate-comparison.json'
    if args.check:assert json.loads(path.read_text())==result,'Source candidate comparison drift'
    else:
        path.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
        packets=[json.loads(p.read_text())for p in sorted(HERE.glob('observations-*.json'))]
        field_rows=collections.defaultdict(list)
        for packet in packets:
            for item in packet['items']:
                for field in item['properties']:
                    if field not in ROUTES:field_rows[field].append(item['ui_entry_ordinal'])
        backlog={'classification':'UNCOMPARED_OBSERVED_FIELDS','fields':[{'field':field,'observations':len(ordinals),'physical_ordinals':ordinals,'source_route_status':'NOT_YET_QUALIFIED_FOR_COMPARISON'}for field,ordinals in sorted(field_rows.items())],
            'private_narrative_receipts':sum('description_receipt'in item for packet in packets for item in packet['items']),
            'boundary':'These fields are retained literally but have not been source-compared. Private Description values and display layouts are hash/length receipts only, not semantic comparisons. No full Basic semantic parity claim. Raw source semantics and renderer projection are separate.'}
        (HERE/'uncompared-field-backlog.json').write_text(json.dumps(backlog,indent=2)+'\n',encoding='utf-8',newline='\n')
        paths=sorted(HERE.glob('observations-*.json'))+[HERE/'uncompared-field-backlog.json',path]
        paths += [HERE/name for name in ('category-end-receipt.json','reader-coverage-union.json','tail-journal-receipt.json','field-presence-and-display-coverage.json','reader-correction-receipt.json','client-appearance-receipt.json','compare_sources.py','validate.py','README.md')if(HERE/name).exists()]
        integrity={'files':{p.name:{'bytes':len(p.read_bytes()),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}for p in paths}}
        (HERE/'packet-integrity.json').write_text(json.dumps(integrity,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps(result['counts']))
