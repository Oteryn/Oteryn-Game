"""QENGINE-2 offline donor catalogue and bounded NPC reward compositions.

Only committed corpus blobs and the existing appearance source are read. This
producer promotes the requested quest catalogue; it never contacts a donor host.
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import re
from pathlib import Path
import sys
import tarfile
import xml.etree.ElementTree as ET
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[3]
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
PROVENANCE = 'crystal-9f5a72c-quest-rewards-v1'
DECISION = 'owner:QENGINE-2:canonical-donor-outfit-and-atomic-quest-grants:2026-11-06'


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def donor_inputs():
    paths = ['data/XML/outfits.xml', 'data/npclib/npc_system/npc_handler.lua'] + [f'data-global/npc/{n}.lua' for n in (
        'morgan', 'navigator', 'costello', 'budrik', 'sam', 'cael', 'cranky_lizard_crone', 'palimuth', 'yalahari',
        'dedoras', 'muriel', 'the_first_dragon', 'broken_servant_sentry')]
    paths += ['data-global/scripts/quests/the_new_frontier/action_outfit.lua',
              'data-global/scripts/quests/wrath_of_the_emperor/actions_mission12_just_rewards.lua',
              'data-global/startup/tables/item.lua']
    result = {}
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        for path in paths:
            row = next(r for r in manifest['files'] if r['source'] == 'crystalserver' and r['revision'] == PIN and r['path'] == path)
            raw = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
            if sha(raw) != row['sha256']:
                raise ValueError('donor blob changed: ' + path)
            result[path] = raw
    return result


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def build():
    inputs = donor_inputs()
    gameplay=json.loads((ROOT/'content/spells.manifest.json').read_bytes())
    source = {'reference': 'crystal:data/XML/outfits.xml', 'revision': PIN,
              'sha256': sha(inputs['data/XML/outfits.xml'])}
    xml = ET.fromstring(inputs['data/XML/outfits.xml'])
    # Explicit identities for the two sex-specific donor display names.
    aliases = {'Noblewoman': 'Nobleman', 'Norsewoman': 'Norseman'}
    groups = {}
    for row in xml:
        name = aliases.get(row.get('name'), row.get('name'))
        if row.get('enabled') == 'yes' and row.get('from') != 'store':
            groups.setdefault(name, {})[row.get('type')] = row.attrib
    appearance = ROOT / 'imports/cipsoft-appearances/outfits'
    manifest_raw = (appearance / 'manifest.json').read_bytes()
    manifest = json.loads(manifest_raw)
    looks = {}
    addon_patterns = {}
    for shard in manifest['files']:
        raw = (appearance / shard['path']).read_bytes()
        if sha(raw) != shard['sha256']:
            raise ValueError('appearance source drift')
        looks.update({r['source_id']: r['source_record_sha256'] for r in json.loads(raw)['records']})
        for record in json.loads(raw)['records']:
            addon_patterns[record['source_id']]=min(next((f.get('value',1) for f in group['sprite_info_fields'] if f['field']==2),1) for group in record['frame_groups'])-1
    outfits, mappings = [], []
    for name, sexes in sorted(groups.items()):
        if set(sexes) != {'0', '1'}:
            raise ValueError('unpaired donor outfit: ' + name)
        female, male = sexes['0'], sexes['1']
        if any(int(s['looktype']) not in looks for s in (female, male)):
            continue  # Newer looks need a later admitted appearance corpus; never fabricate one.
        key = 'oteryn:outfit.' + name.lower().replace(' ', '_').replace('-', '_')
        # Monk is explicitly addon-unavailable; all other admitted quest looks use bits 1/2.
        addons = 0 if name == 'Monk' else min(2,addon_patterns[int(female['looktype'])],addon_patterns[int(male['looktype'])])
        outfits.append({'key': key, 'revision': 'crystal-9f5a72c-r1',
            'female': {'id': int(female['looktype']), 'source_record_sha256': looks[int(female['looktype'])]},
            'male': {'id': int(male['looktype']), 'source_record_sha256': looks[int(male['looktype'])]},
            'addon_count': addons, 'starter_female': female['unlocked'] == 'yes',
            'starter_male': male['unlocked'] == 'yes', 'starter_addons_female': 0,
            'starter_addons_male': 0, 'premium': female['premium'] == 'yes',
            'unlock_sources': [source], 'source': source})
        for addon in range(addons + 1):
            target = {'kind': 'Outfit', 'outfit_key': key} if addon == 0 else {
                'kind': 'Addon', 'outfit_key': key, 'addon': addon}
            mappings.append({'grant_key': f'{key}/quest-grant/{addon}', 'target': target,
                             'compatible_provenance': [PROVENANCE]})
    mappings.append({'grant_key': 'oteryn:appearance-grant/quest/soul-war/revenant-base',
        'target': {'kind': 'Outfit', 'outfit_key': 'oteryn:outfit.revenant'},
        'compatible_provenance': ['crystal-201189f-soul-war-revenant-v1']})
    # Keep the accepted Rotten Blood base receipt usable after catalogue promotion.
    # This owner deliberately has no admitted addon grant.
    mappings.append({'grant_key': 'oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base',
        'target': {'kind': 'Outfit', 'outfit_key': 'oteryn:outfit.decaying_defender'},
        'compatible_provenance': ['owner-pr2784-decaying-defender-base-v1']})
    atlas = load_module('qengine_atlas_palette', 'tools/game-atlas-appearances/export.py')
    palette_path = 'tools/game-atlas-appearances/export.py'
    palette_source = {'reference': palette_path + '#outfit_color', 'revision': 'rounded-hsi-133-v1',
                      'sha256': sha((ROOT / palette_path).read_bytes())}
    doc = {'schema': 'OTERYN_CHARACTER_COSMETICS/v1',
        'content_revision': gameplay['build_training']['content_revision'],
        'source_manifest_sha256': sha(manifest_raw),
        'source_appearances_sha256': manifest['inputs']['appearances']['sha256'],
        'palette': {'key': 'oteryn:palette.outfit', 'revision': 'rounded-hsi-133-v1',
                    'colours': [list(atlas.outfit_color(i)) for i in range(133)], 'source': palette_source},
        'outfits': outfits, 'mounts': [], 'grant_mappings': mappings,
        'fallbacks': {sex: {'outfit_key': 'oteryn:outfit.citizen', 'colours': [0, 0, 0, 0]}
                      for sex in ('female', 'male')}}
    cosmetics = load_module('qengine_cosmetics_authoring', 'tools/content-schema/cosmetics-authoring/cosmetics_authoring.py')
    author = {'schema': 'OTERYN_CHARACTER_COSMETICS_AUTHORING/v1',
        'profile': {'kind': 'reference', 'revision': 'quest-cosmetics-r1', 'mount_speed_bonus': 10},
        'client_version': manifest['client_version'], 'document': doc, 'promotions': []}
    for path in sorted(cosmetics.promotion_paths(doc)):
        citation=palette_source if path == '/palette' else source
        if path in ('/source_manifest_sha256','/source_appearances_sha256'):
            citation={'reference': 'imports/cipsoft-appearances/outfits/manifest.json',
                      'revision':manifest['client_version'],'sha256':sha(manifest_raw)}
        author['promotions'].append({'path': path, 'value': cosmetics.pointer(doc, path),
            'state': 'PROMOTED', 'decision': DECISION,
            'source_kind': 'OWNER_DECISION' if path.startswith('/fallbacks/') or path=='/grant_mappings' else 'STRUCTURED_REFERENCE_DATA',
            'source': citation})
    callbacks = []
    def storage(track, before, after, maximum=None, preserve=False):
        return {'track': track, 'minimum': before, 'maximum': before if maximum is None else maximum,
                'set': after, **({'preserve': True} if preserve else {})}
    def add(name, quest, npc, keywords, states, take=(), give=None, outfit=None, topic=0, next_topic=0,
            requires=(), present=(), achievement=None):
        path = f'data-global/npc/{npc}.lua'
        callbacks.append({'key': 'oteryn:quest-reward/' + name, 'quest': 'oteryn:quest.' + quest,
            'npc': 'oteryn:npc.' + npc, 'keywords': keywords, 'topic': topic, 'next_topic': next_topic,
            'storage': states, 'take': [{'item': f'oteryn:item.tibia.i{i}', 'count': n} for i,n in take],
            'give': None if give is None else {'item': f'oteryn:item.tibia.i{give[0]}', 'count': give[1], 'contents': []},
            'outfit': None if outfit is None else {'outfit': 'oteryn:outfit.' + outfit[0], 'addons': outfit[1]},
            'achievement': achievement or ('oteryn:achievement/backpack_tourist' if name == 'sam-backpack' else None),
            'reply': ['Thank you for your help.'],
            'sources': [f'crystal:{PIN}:{path}:sha256:{sha(inputs[path])}']})
        if requires:
            callbacks[-1]['requires']=list(requires)
        if present:
            callbacks[-1]['present']=[{'item':f'oteryn:item.tibia.i{i}','count':n} for i,n in present]
    prefix = 'oteryn:quest-progress/quest/'
    add('pirate-sabre', 'outfit_and_addon_quests', 'morgan', ['firebird'],
        [storage(prefix+'u7_8/pirate_outfits/pirate_sabre_addon',4,5)], outfit=('pirate',1))
    add('knight-sword', 'outfit_and_addon_quests', 'morgan', ["knight's sword"],
        [storage(prefix+'u7_8/knight_outfits/addon_sword',1,2)],take=[(5892,1),(5880,100)],outfit=('knight',1),topic=1)
    add('deepling-helmet', 'outfit_and_addon_quests', 'navigator', ['yes'],
        [storage('oteryn:quest-progress/navigator',-1,4,0)],outfit=('deepling',2),topic=8)
    add('hallowed-holy-water','tibia_tales','costello',['amanda'],
        [storage(prefix+'u8_1/rest_in_hallowed_ground/questline',1,2)],give=(133,1),topic=3)
    add('hallowed-water-offer','tibia_tales','costello',['holy water'],
        [storage(prefix+'u8_1/rest_in_hallowed_ground/questline',1,1,preserve=True)],next_topic=3)
    # The offer is conversation-only and deliberately excluded from the durable callback list.
    offer=callbacks.pop()
    add('outfox-helmet','tibia_tales','budrik',['mission','quest'],
        [storage(prefix+'u8_1/to_outfox_afox_quest',1,2)],take=[(139,1)],give=(875,1))
    add('sam-backpack','sam_s_old_backpack_quest','sam',['yes'],
        [storage(prefix+'u7_5/sams_old_backpack/sams_old_backpack_npc',-1,1,0)],take=[(3244,1)],topic=1)
    # Offers bind confirmation to this NPC and the common conversation topic.
    offers=[{'npc':'oteryn:npc.morgan','keywords':['forge'],'topic':0,'next_topic':1,
             'reply':['What would you like me to forge?'],
             'storage':[storage(prefix+'u7_8/knight_outfits/addon_sword',1,1,preserve=True)]},
            {'npc':offer['npc'],'keywords':offer['keywords'],'topic':0,'next_topic':3,
             'reply':['Who sent you?'],'storage':offer['storage']},
            {'npc':'oteryn:npc.sam','keywords':['old backpack','backpack'],'topic':0,'next_topic':1,
             'reply':['Did you find my old backpack?'],'storage':callbacks[-1]['storage']}]
    for word,topic,next_topic in [('flou',0,1),('explain',1,2),('yes',2,3),('no',3,4),('yes',4,5),('yes',5,6),('yes',6,7),('helmet',7,8)]:
        offers.append({'npc':'oteryn:npc.navigator','keywords':[word],'topic':topic,'next_topic':next_topic,
            'reply':['Listen carefully.'],'storage':[storage('oteryn:quest-progress/navigator',-1,4,0)]})
    # L1: keep parent QuestState ownership for shared prerequisite tracks.
    frontier = prefix + 'u8_54/the_new_frontier/'
    yalahar = prefix + 'u8_4/in_service_of_yalahar/'
    for name, bit, item, word in [('crest',1,10199,'crest'), ('tribal',2,10198,'tribal crest')]:
        add('l1-warmaster-'+name, 'the_new_frontier_quest', 'cael', ['yes'],
            [storage(frontier+'questline',27,27,preserve=True),
             storage(frontier+'warmaster_addon_'+str(bit),-1,1,0)],
            take=[(item,1)],outfit=('warmaster',bit),topic=60+bit-1)
        offers.append({'npc':'oteryn:npc.cael','keywords':[word], 'topic':0,
            'next_topic':60+bit-1,'storage':callbacks[-1]['storage'],
            'reply':['Can I have that '+('serpent' if bit==1 else 'tribal')+' crest?']})
    for name, bit, item, words in [('cape',1,11701,['cape','first addon']), ('hat',2,11700,['hat','second addon'])]:
        track='oteryn:quest-progress/wayfarer_outfit'+('' if bit==1 else '_1')
        add('l1-wayfarer-'+name,'wayfarer_outfits_quest','cranky_lizard_crone',['yes'],
            [storage(track,-1,1,0)],take=[(item,1)],outfit=('wayfarer',bit),topic=bit)
        offers.append({'npc':'oteryn:npc.cranky_lizard_crone','keywords':words,
            'topic':0,'next_topic':bit,'storage':callbacks[-1]['storage'],
            'reply':['Do you have the '+('old cape' if bit==1 else 'sedge hat')+' with you?']})
    for npc,side in [('palimuth',1),('yalahari',2)]:
        states=[storage(yalahar+'questline',52,53),
                storage(yalahar+'door_to_reward',-1,1,1),
                storage(yalahar+'mission10',-1,4,4)]
        # Palimuth's completion checks questline only; Yalahari also requires
        # his side. The remaining tracks are assignments, not prerequisites.
        if npc=='yalahari':
            states.insert(1,storage(yalahar+'side_decision',side,side,preserve=True))
        add('l1-yalaharian-base-'+npc,'in_service_of_yalahar_quest',npc,['mission'],
            states,outfit=('yalaharian',0))
        path='data-global/npc/'+npc+'.lua'
        lines='244-259' if npc=='palimuth' else '246-252'
        callbacks[-1]['sources']=[f'crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}']
    for name,side,bit in [('sage',1,1),('villain',2,2)]:
        add('l1-yalaharian-'+name,'in_service_of_yalahar_quest','palimuth',['yes'],
            [storage(yalahar+'questline',54,55),
             storage(yalahar+'side_decision',side,side,preserve=True)],
            outfit=('yalaharian',bit),topic=6)
        # The captured NPC checks possession and retains the original crest.
        callbacks[-1]['present']=[{'item':'oteryn:item.tibia.i9041','count':1}]
        offers.append({'npc':'oteryn:npc.palimuth','keywords':['job','addon'],
            'topic':0,'next_topic':6,'storage':callbacks[-1]['storage'],
            'reply':['Did you bring me the vampiric crest?']})
    # Parent prerequisites stay read-only and retain their original quest owner.
    def requirement(track,value):
        return {'track':track,'minimum':value,'maximum':value}
    library=prefix+'u11_80/the_secret_library/'
    library_ready=[storage(library+'library_permission',6,7),storage(library+'questlog',-1,1,1)]
    for name,value in [('small_islands',4),('liquid_death',8),('asuras',7),('falcon_bastion',3),('darashia',9),('mo_ta',8)]:
        library_ready.append(storage(library+name+'/questline',value,value,preserve=True))
    add('l2-battle-mage-base','the_secret_library_quest','dedoras',['check'],library_ready,
        outfit=('battle_mage',0),achievement='oteryn:achievement/battle_mage')
    library_done=[requirement(library+'library_permission',7)]
    for name,bit,item,count,words in [('book',1,28792,5,['book','books','sturdy books']),('epaulettes',2,28793,20,['epaulettes'])]:
        add('l2-battle-mage-'+name,'battle_mage_outfits_quest','dedoras',words,
            [storage(prefix+'u11_80/battle_mage_outfits/addon'+str(bit),-1,1,0)],
            take=[(item,count)],outfit=('battle_mage',bit),topic=3,requires=library_done)
    for word,topic,next_topic in [('addon',0,1),('yes',1,2),('yes',2,3)]:
        offers.append({'npc':'oteryn:npc.dedoras','keywords':[word],'topic':topic,
            'next_topic':next_topic,'storage':[],'requires':library_done,
            'reply':['I need five sturdy books for the first addon or twenty epaulettes for the second.']})
    feathers=prefix+'u11_02/the_first_dragon/feathers'
    add('l2-festive-base','the_first_dragon_quest','the_first_dragon',['reward'],
        [storage(feathers,1,2)],outfit=('festive',0))
    festive_ready=[requirement(feathers,2)]
    # Muriel's captured offers require the stage that only those offers initialize.
    # Bootstrap with parent completion and retained matching evidence; only the
    # subsequent exact hand-in consumes items and earns the addon.
    offers.append({'npc':'oteryn:npc.muriel','keywords':['addons'],'topic':0,'next_topic':1,
        'storage':[],'requires':festive_ready,'reply':['Choose the first or second addon.']})
    for name,bit,item,count,choice,word in [('mask',1,25088,3,'first','mask'),('feather',2,25089,50,'second','feather')]:
        track=prefix+'u11_02/festive_outfits/addon'+str(bit)
        add('l2-festive-'+name+'-start','festive_outfits_quest','muriel',[choice],
            [storage(track,-1,1,0)],topic=1,requires=festive_ready,present=[(item,count)])
        add('l2-festive-'+name,'festive_outfits_quest','muriel',[word,word+'s'],
            [storage(track,1,2)],take=[(item,count)],outfit=('festive',bit),requires=festive_ready)
    elementalist=prefix+'u9_1/elementalist_outfits/'
    add('l2-elementalist-start','elementalist_outfits_quest','broken_servant_sentry',['yes'],
        [storage(elementalist+'outfit',-1,1,0),storage(elementalist+'questline',-1,1,1)],topic=2)
    add('l2-elementalist-base','elementalist_outfits_quest','broken_servant_sentry',['yes'],
        [storage(elementalist+'outfit',1,2)],take=[(12601,20)],outfit=('elementalist',0),topic=4)
    for name,bit,item,words in [('cap',1,12599,['cap','mage']),('staff',2,12803,['staff','spike'])]:
        add('l2-elementalist-'+name,'elementalist_outfits_quest','broken_servant_sentry',words,
            [storage(elementalist+'outfit',2,2,preserve=True),
             storage(elementalist+'addon'+str(bit),-1,1,0)],take=[(item,1)],outfit=('elementalist',bit))
    for before,maximum,first,second in [(-1,0,1,2),(1,1,3,4)]:
        guard=[storage(elementalist+'outfit',before,before,maximum,preserve=True)]
        offers.append({'npc':'oteryn:npc.broken_servant_sentry','keywords':['slime','mould','fungus','sample'],
            'topic':0,'next_topic':first,'storage':guard,'reply':['Did you bring samples for me to analyse?']})
        offers.append({'npc':'oteryn:npc.broken_servant_sentry','keywords':['yes'],
            'topic':first,'next_topic':second,'storage':guard,'reply':['Let me analyse the samples.']})
    callbacks.sort(key=lambda c:c['key'])
    offers.sort(key=lambda c:(c['npc'],c['topic'],c['keywords']))
    validate_callbacks({'schema':1,'callbacks':callbacks})
    native_cosmetics=cosmetics.build(author, appearance/'manifest.json')
    gameplay['schema']='OTERYN_NATIVE_GAMEPLAY_MANIFEST/v6'
    gameplay['cosmetics']={'path':'cosmetics/outfits/native-cosmetics.json','sha256':sha(native_cosmetics)}
    generated = {'content/cosmetics/outfits/authoring.json': encoded(author),
            'content/cosmetics/outfits/native-cosmetics.json': native_cosmetics,
            'content/spells.manifest.json': (json.dumps(gameplay,indent=2,sort_keys=True)+'\n').encode(),
            'content/quests/shared-reward-callbacks.json': encoded({'schema':1,'callbacks':callbacks}),
            'content/quests/shared-reward-offers.json': encoded({'schema':1,'offers':offers})}
    greetings=[]
    for npc in sorted({c['npc'] for c in callbacks}):
        path='data-global/npc/'+npc.removeprefix('oteryn:npc.')+'.lua'
        messages={}
        citations=[]
        for field,constant in [('greet','GREET'),('farewell','FAREWELL'),('walkaway','WALKAWAY')]:
            matches=list(re.finditer(r'npcHandler:setMessage\(MESSAGE_'+constant+r',\s*"([^"\n]*)"\)',inputs[path].decode()))
            origin=path
            if not matches:
                origin='data/npclib/npc_system/npc_handler.lua'
                matches=list(re.finditer(r'\[MESSAGE_'+constant+r'\]\s*=\s*"([^"\n]*)"',inputs[origin].decode()))
            if len(matches)!=1: raise ValueError('ambiguous NPC greeting '+npc+field)
            messages[field]=matches[0].group(1)
            line=inputs[origin][:matches[0].start()].count(b'\n')+1
            citations.append(f'crystal:{PIN}:{origin}:{line}:sha256:{sha(inputs[origin])}')
        greetings.append({'npc':npc,**messages,'sources':citations})
    generated['content/dialogues/quests/shared-reward-greetings.json']=encoded({'schema':1,'npcs':greetings})
    access = load_module('qengine_l1_access', 'tools/content-schema/quest-authoring/qengine_access.py')
    worlds=[]
    for name,quest,path,at,appearance,states,outfit in [
        ('l1-warmaster-wardrobe','the_new_frontier_quest',
         'data-global/scripts/quests/the_new_frontier/action_outfit.lua',(33053,31020,7),5862,
         [storage(frontier+'questline',26,27),storage(frontier+'mission09/reward_door',0,1)],'warmaster'),
        ('l1-wayfarer-wardrobe','wrath_of_the_emperor_quest',
         'data-global/scripts/quests/wrath_of_the_emperor/actions_mission12_just_rewards.lua',(33073,31173,8),11431,
         [storage(prefix+'u8_6/wrath_of_the_emperor/mission12',0,1),
          storage('oteryn:quest-progress/1150',-1,1,1)],'wayfarer')]:
        world=access.callback(name,quest,'use',path,at=access.position(*at),appearance=appearance,
            outfit_reward={'outfit':'oteryn:outfit.'+outfit,'addons':0})
        world['storage']=states
        world['sources']=[f'crystal:{PIN}:{path}:sha256:{sha(inputs[path])}']
        if outfit=='wayfarer':
            table='data-global/startup/tables/item.lua'
            if not re.search(r'\[3200\]\s*=\s*\{\s*itemId\s*=\s*11431,\s*itemPos\s*=\s*\{\s*x\s*=\s*33073,\s*y\s*=\s*31173,\s*z\s*=\s*8', inputs[table].decode()):
                raise ValueError('Wayfarer source wardrobe binding drift')
            world['sources'].append(f'crystal:{PIN}:{table}:sha256:{sha(inputs[table])}')
        worlds.append(world)
    generated['content/quests/shared-outfit-callbacks.json']=encoded({'schema':1,'callbacks':worlds})

    sys.path.insert(0,str(ROOT/'tools/content-migration'))
    import qengine_reward_item_profiles as reward_items
    import register_spell_families as native
    generated[reward_items.PROFILE]=(ROOT/reward_items.PROFILE).read_bytes()
    reward_items.admit(ROOT,generated,reward_items.PROFILE,'content/spells.manifest.json',native.provider_order,encoded)
    return generated


def validate_callbacks(document):
    schema=json.loads((ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_bytes())
    Draft202012Validator(schema).validate(document)
    keys=set()
    for callback in document['callbacks']:
        storage=callback['storage']
        requires=callback.get('requires',[])
        if (callback['key'] in keys
            or len({s['track'] for s in storage})!=len(storage)
            or len({i['item'] for i in callback['take']})!=len(callback['take'])
            or len({i['item'] for i in callback.get('present',[])})!=len(callback.get('present',[]))
            or len({r['track'] for r in requires})!=len(requires)
            or {r['track'] for r in requires}&{s['track'] for s in storage}
            or any(r['minimum']>r['maximum'] for r in requires)
            or any(s['minimum']>s['maximum'] for s in storage)
            or all(s.get('preserve',False) or s['minimum']<=s['set']<=s['maximum'] for s in storage)
            or any(k!=k.lower() or not k.isascii() for k in callback['keywords'])
            or any(len(line.encode('utf-8'))>512 or any(ord(c)<32 or 127<=ord(c)<=159 for c in line) for line in callback['reply'])
            or any(len(source.encode('utf-8'))>1024 for source in callback['sources'])):
            raise ValueError('invalid reward callback '+callback['key'])
        keys.add(callback['key'])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args=parser.parse_args()
    for path,raw in build().items():
        target=ROOT/path
        if args.check:
            if not target.is_file() or target.read_bytes()!=raw:
                raise ValueError('stale QENGINE reward output: '+path)
        elif not target.is_file() or target.read_bytes()!=raw:
            target.write_bytes(raw)

if __name__=='__main__':
    main()
