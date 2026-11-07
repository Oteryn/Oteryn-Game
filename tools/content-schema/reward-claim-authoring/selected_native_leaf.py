"""Bounded native leaf selections reconcile a chosen recipe with pinned Wiki identity.

The donor imports and all other reward formats remain unchanged. The existing
plain once owner mints the selected leaf; this producer never executes Lua.
"""
import copy
import hashlib
import json
import re
from pathlib import Path

PROFILE = 'native_selected_leaf_v1'
SCHEMA = 'OTERYN_NATIVE_SELECTED_LEAF/v1'
INPUT = 'tools/content-schema/reward-claim-authoring/native-selected-leaves.json'
TOOL = 'tools/content-schema/reward-claim-authoring/selected_native_leaf.py'


def require(ok, message):
    if not ok:
        raise ValueError('native selected leaf: ' + message)


def encoded(value):
    return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()


def choices(root):
    path = root / INPUT
    if not path.exists():
        return []
    doc = json.loads(path.read_bytes())
    require(set(doc) == {'schema','selections'} and type(doc['schema']) is int and doc['schema']==1,
            'input shape')
    rows = doc['selections']
    require(isinstance(rows,list) and len(rows)<=64,'selection bound')
    require(len({r['claim'] for r in rows})==len(rows),'duplicate claim')
    return rows


def apply(records, root, items):
    for selection in choices(root):
        require(set(selection)=={'claim','quest','unique_id','expected_reward','reward','wiki','rule'},'selection fields')
        require(isinstance(selection['rule'],str) and 0<len(selection['rule'])<=1024,'explicit rule')
        matches=[r['definition'] for r in records if r['definition']['identity']['key']==selection['claim']]
        require(len(matches)==1,'one existing source claim')
        d=matches[0]
        require(set(d)=={'identity','claim','quest','placements','readiness','provenance'} and d['claim']=={'per':'character','repeat':{'kind':'once'}}
                and len(d['placements'])==1,'only plain singleton once leaves')
        p=d['placements'][0]
        require(set(p['reward'])=={'items'} and len(p['reward']['items'])==1
                and p['reward']['items'][0]==selection['expected_reward'],'exact original donor reward')
        ids=[i['unique_id'] for i in p['source_binding']['legacy_unique_ids'] if i['server']=='crystalserver']
        require(ids==[selection['unique_id']] and type(selection['unique_id']) is int
                and 1<=selection['unique_id']<=65535,'exact source selector')
        reward=selection['reward']
        require(set(reward)=={'count','item'} and type(reward['count']) is int and 1<=reward['count']<=100
                and set(reward['item'])=={'family','key','revision'}
                and reward['item']['family']=='Item' and reward['item']['revision']=='definition-r1','selected amount')
        key=reward['item']['key'];item=items.get(key)
        require(item is not None and item['materializable'] is True,'canonical Item admission')
        import reward_claim_authoring as author
        require(author.stack_problem(item,reward['count']) is None,'selected stack facts')
        wiki=selection['wiki']
        require(set(wiki)=={'path','title','pageid','revid','sha256'}
                and wiki['path'].startswith('imports/tibiawiki/') and wiki['path'].endswith('.json')
                and '..' not in Path(wiki['path']).parts,'Wiki identity')
        snapshot=root/wiki['path'];doc=json.loads(snapshot.read_bytes())
        pages=[p for p in doc['pages'] if p.get('title')==wiki['title'] and p.get('pageid')==wiki['pageid']]
        require(len(pages)==1,'exact Wiki object page')
        revisions=[r for r in pages[0]['revisions'] if r['revid']==wiki['revid']]
        require(len(revisions)==1,'exact Wiki revision')
        text=revisions[0]['slots']['main']['*']
        require(hashlib.sha256(text.encode()).hexdigest()==wiki['sha256'],'Wiki content digest')
        match=re.search(r'(?mi)^\|\s*itemid\s*=\s*([0-9]+)\s*$',text)
        require(match is not None and key=='oteryn:item.tibia.i'+match[1],'Wiki exact item identity')
        quests=[]
        for path in (root/'content/quests/definitions').glob('*.json'):
            quests.extend(r['definition'] for r in json.loads(path.read_bytes()).get('records',[])
                          if r.get('definition',{}).get('identity',{}).get('key')==selection['quest'])
        require(len(quests)==1,'canonical quest')
        quest=quests[0]; recipe=quest['oteryn_recipe']['payload']['recipe']
        require(any(c['key']==selection['claim'] for c in quest['claims'])
                and recipe['repeat']['kind']=='once','chosen quest owns this once claim')
        require(any(r.get('kind')=='item' and r.get('name','').casefold()==wiki['title'].casefold()
                    and r.get('count')==reward['count'] and r.get('item',reward['item'])==reward['item']
                    for r in recipe['reward_intents']),'chosen recipe selects this exact reward')
        require(any(title in text for title in quest['oteryn_recipe']['payload']['covered_wiki_titles']),
                'Wiki confirms acquisition in the selected quest')
        old=copy.deepcopy(p['reward']['items'][0])
        d['definition_profile']=PROFILE
        d['native_selection']={'schema':SCHEMA,'rule':selection['rule'],
            'generator':TOOL,'quest':selection['quest'],'source_claim':d['provenance']['pilot_key'],
            'source_revision':d['provenance']['pilot_revision'],'unique_id':selection['unique_id'],
            'position':copy.deepcopy(p['source_binding']['project_position']),
            'original_reward':old,'original_reward_sha256':hashlib.sha256(encoded(old)).hexdigest(),
            'reward':copy.deepcopy(reward),
            'wiki':{**wiki,'item_id':int(match[1]),'snapshot_sha256':hashlib.sha256(snapshot.read_bytes()).hexdigest()}}
        p['reward']['items']=[copy.deepcopy(reward)]
        d['readiness']='ready'
    return records


def validate(rows, root):
    import reward_claim_authoring as author
    source=json.loads((root/author.CLAIMS_REL).read_bytes())['claims']
    items={r['definition']['identity']['key']:r['definition']
           for path in (root/'content/items/definitions').glob('items-*.json')
           for r in json.loads(path.read_bytes())['records']}
    selected={row['claim'] for row in choices(root)}
    source=[c for c in source if 'oteryn:reward-claim.'+c['identity']['key'].split('reward-claim/',1)[1].replace('/','.') in selected]
    state=json.loads((root/author.QUEST_STATE_REL).read_bytes())
    transitions={t['key'] for quest in state['quests'] for t in quest['transitions'] if t['source']['owner']=='chest'}
    original,_=author.build_records(source,json.loads((root/author.MANIFEST_REL).read_bytes()),items,transitions=transitions)
    apply(original,root,items)
    expected=[r for r in original if r['definition'].get('definition_profile')==PROFILE]
    return [] if rows==expected else ['native selected leaf output differs from its exact source/Wiki/recipe projection']
