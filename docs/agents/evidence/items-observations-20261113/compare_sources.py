"""Reproduce bounded name-candidate comparisons against an immutable source pin.

A name match, including a unique match, is not a qualified client/server ID.
No source field is promoted to runtime admission. Trade observations are retained
separately until service/currency/residence and direction mappings are qualified.
"""
import argparse,collections,decimal,hashlib,json,pathlib,re,subprocess

HERE=pathlib.Path(__file__).resolve().parent
REPO=HERE.parents[3]
PIN='5efd67da164ac9dc33c8c066434b228cebb86c2b'


def blob(path,receipts):
    raw=subprocess.check_output(['git','show',PIN+':'+path],cwd=REPO)
    receipts[path]={'repository_path':path,'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),
                    'git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()}
    return json.loads(raw)


def field(semantics,path):
    node=semantics;visited=[]
    for part in path:
        if isinstance(node,dict)and node.get('state')in ('KNOWN','UNKNOWN','NOT_APPLICABLE'):
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
    observations=[]
    for name in ('creature-products-basic-1-134-repo-ready.json','armors-basic-1-33-qualified-repo-ready.json'):
        packet=json.loads((HERE/name).read_text())
        observations += [dict(item,category=packet['category'])for item in packet['items']]
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
            for label,route,kind in [('Weight',['physical','weight'],'weight'),('Tradeable In Market',['trade_restrictions','marketable'],'market')]:
                if label not in item['properties']:continue
                snapshot=field(semantics,route);classification=compare(item['properties'][label],snapshot,kind)
                detail={'client_literal':item['properties'][label],'canonical':snapshot,'comparison':classification}
                field_classes[classification]+=1
                if key in selected:
                    profile,profile_path=selected[key];selected_snapshot=field(profile.get('semantics',{}),route)
                    detail['selected_profile']={'path':profile_path,'field':selected_snapshot,
                        'comparison':compare(item['properties'][label],selected_snapshot,kind)}
                candidate['fields'][label]=detail
            if key in selected:candidate['selected_admission_literal']=selected[key][0].get('admission')
            row['candidates'].append(candidate)
        results.append(row)
    return {'schema':'OTERYN_PARTIAL_ITEMS_SOURCE_CANDIDATES/v1','classification':'DERIVED','source_commit':PIN,
        'scope':{'categories':{'Creature Products':{'ordinals':[1,134],'records':134},'Armors':{'capture_ids':[1,33],'records':33,'global_ordinals':'UNKNOWN'}},'records':167,'category_complete':False,
                 'compared_fields':['Weight','Tradeable In Market'],'other_observed_fields_retained_but_not_source_compared':True},
        'identity_boundary':'Every repository key is an unqualified name candidate, including unique names; no UI ID or appearance mapping was read. Equal name/icon never collapses rows.',
        'weight_conversion_source':{'commit':PIN,'path':'crates/gameplay/dispatch/src/gameplay_transport/inspection.rs'},
        'weight_conversion':'Only two-decimal client oz converted to hundredths-of-ounce source integer; raw literals preserved.',
        'counts':{'canonical_records':len(canonical),'selected_membership':len(selected),'name_candidate_classes':dict(classes),'candidate_field_classes':dict(field_classes)},
        'not_claimed':['Runtime admission','Executable trade offers','Market value currentness','Complete source semantic parity','Source difference as gameplay bug'],
        'items':results,'source_receipts':list(receipts.values())}


if __name__=='__main__':
    p=argparse.ArgumentParser(__doc__);p.add_argument('--check',action='store_true');args=p.parse_args()
    result=generate();path=HERE/'source-candidate-comparison.json'
    if args.check:assert json.loads(path.read_text())==result,'Source candidate comparison drift'
    else:path.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(result['counts']))
