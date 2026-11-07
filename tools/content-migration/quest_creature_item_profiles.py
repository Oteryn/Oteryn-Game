"""Source-qualified corpse custody for creatures bound to native quest death callbacks."""
import copy
import hashlib
import json
import qengine_reward_item_profiles as shared

PROOF = 'tools/content-schema/native-gameplay/quest-creature-item-profiles.json'
ADMISSIONS = 'content/items/definitions/quest-loot-custody.json'


def admit(root, generated, profile_path, order):
    source = shared.source
    xml_raw=(root/shared.XML).read_bytes()
    client_raw=(root/shared.CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=shared.XML_SHA or hashlib.sha256(client_raw).hexdigest()!=shared.CLIENT_SHA:
        raise ValueError('QUEST_CREATURE_CUSTODY_SOURCE_DRIFT')
    xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client_raw)
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    callbacks=json.loads(generated['content/quests/shared-kill-callbacks.json'])['callbacks']
    creatures={c['creature'] for c in callbacks}
    creature_path='content/creatures/definitions/spell-native-profiles.json'
    wanted=set()
    for record in json.loads((root/creature_path).read_bytes())['records']:
        profile=record['profile']
        if profile['target']['key'] in creatures:
            corpse=profile['data']['profile']['details'].get('corpse_item')
            if corpse is None: raise ValueError('QUEST_CREATURE_CORPSE_ABSENT')
            wanted.add(int(corpse['key'].rsplit('i',1)[1]))
    if len(wanted)>256: raise ValueError('QUEST_CREATURE_CORPSE_LIMIT')
    # A corpse's source decay successors retain their own exact container/physical facts.
    pending=list(wanted)
    while pending:
        item=pending.pop()
        target=int(source.attrs(xml[item]).get('decayto','0'))
        if target and target not in wanted:
            wanted.add(target);pending.append(target)
        if len(wanted)>256: raise ValueError('QUEST_CREATURE_DECAY_LIMIT')
    physical={r['server_item_id']:r for r in source.policies(client_raw,xml_raw,wanted)}
    records=[source.profile(i,xml[i],objects[i],physical[i],bindings,None) for i in sorted(wanted)]
    packet=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,wanted)
    document=json.loads(generated[profile_path])
    current={r['production_definition']['production_key']:r for r in document['records']}
    for record in packet['records']:
        key=record['production_definition']['production_key']
        if not record['admission']['materializable']:
            raise ValueError('QUEST_CREATURE_CORPSE_NOT_MATERIALIZABLE:'+key)
        prior=current.get(key)
        if prior is not None:
            if prior != record:
                # Existing accepted native providers remain authoritative.
                if not prior['admission']['materializable'] or shared.accepted.known_conflict(prior['semantics'],record['semantics']):
                    raise ValueError('QUEST_CREATURE_CUSTODY_NATIVE_CONFLICT:'+key)
            continue
        document['records'].append(copy.deepcopy(record))
    document['records'].sort(key=order)
    generated[profile_path]=(json.dumps(document,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n').encode()
    loot=json.loads((root/'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
    loot_keys={r['loot']['key'] for r in loot['creature_loot'] if r['creature']['key'] in creatures and r['loot'] is not None}
    selected={r['production_definition']['production_key'] for r in packet['records']}
    def include(entries):
        for entry in entries:
            selected.add(entry['item']['key']);include(entry.get('children',[]))
    for table in loot['tables']:
        if table['identity']['key'] in loot_keys:include(table['entries'])
    declarations=[]
    for record in document['records']:
        ref=record['production_definition']
        if ref['production_key'] in selected:
            declarations.append({'family':ref['family'],'key':ref['production_key'],'revision':ref['revision_ref']})
    generated[ADMISSIONS]=(json.dumps({'schema':'OTERYN_NATIVE_LOOT_CUSTODY_ADMISSIONS/v1','items':sorted(declarations,key=lambda r:r['key'])},indent=2)+'\n').encode()
    generated[PROOF]=(json.dumps({'schema':'OTERYN_QUEST_CREATURE_CUSTODY_PROFILES/v1',
        'sources':[{'path':shared.XML,'sha256':shared.XML_SHA},{'path':shared.CLIENT,'sha256':shared.CLIENT_SHA},
                   {'path':creature_path,'sha256':hashlib.sha256((root/creature_path).read_bytes()).hexdigest()}],
        'creatures':sorted(creatures),'records':packet['records']},ensure_ascii=False,indent=2)+'\n').encode()
