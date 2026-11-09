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
import taming_cosmetics
import wizard_cosmetics
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
    paths = ['data/XML/mounts.xml', 'data/XML/outfits.xml', 'data/npclib/npc_system/npc_handler.lua'] + [f'data-global/npc/{n}.lua' for n in (
        'morgan', 'navigator', 'costello', 'budrik', 'sam', 'cael', 'cranky_lizard_crone', 'palimuth', 'yalahari',
        *wizard_cosmetics.DONOR_NPCS, 'dedoras', 'muriel', 'the_first_dragon', 'broken_servant_sentry', 'avar_tar', 'ezebeth', 'irmana', 'percybald', 'raymond_striker', 'duncan', 'ariella', 'ishina', 'habdel', 'miraia', 'razan', 'hjaern', 'tom', 'amber', 'norma', 'hugo', 'simon_the_beggar', 'gregor', 'ustan', 'ceiron', 'a_majestic_warwolf', 'vescu', 'erayo', 'atrad', 'elane', 'legola', 'zoltan', 'lynda', 'sandra', 'myra', 'angelina', 'eruaran', 'gnomally', 'ottokar', 'king_tibianus', 'the_bone_master', 'the_dream_master', 'vanys', 'eshaya', 'gnomadness', 'a_sweaty_cyclops', 'gnommander')]
    paths += ['data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua', 'data-global/scripts/actions/valuables/random_items.lua',
              'data-global/scripts/quests/the_new_frontier/action_outfit.lua',
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
        # Explicit owner-selected quest rewards may use admitted looks labelled store by the donor.
        if row.get('enabled') == 'yes' and (row.get('from') != 'store' or name in ('Royal Costume', 'Dragon Slayer')):
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
    ram_look_hash = looks[1934]
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
    # Reuse the existing source item-action owner. Matching both source look
    # identities is required; a source label alone never admits a grant.
    item_packet=json.loads((ROOT/'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
    scoped={'oteryn:outfit.afflicted','oteryn:outfit.soil_guardian','oteryn:outfit.crystal_warlord'}
    for outfit in outfits:
        if outfit['key'] not in scoped:continue
        outfit_looks=sorted([outfit['female']['id'],outfit['male']['id']])
        for grant in item_packet['grants']:
            if grant['kind'] not in ('outfit','addon') or grant.get('look_types')!=outfit_looks:continue
            target={'kind':'Outfit','outfit_key':outfit['key']} if grant['kind']=='outfit' else {
                'kind':'Addon','outfit_key':outfit['key'],'addon':grant['addon']}
            mappings.append({'grant_key':grant['key'],'target':target,'compatible_provenance':[item_packet['provenance']]})
    mappings.append({'grant_key': 'oteryn:appearance-grant/quest/soul-war/revenant-base',
        'target': {'kind': 'Outfit', 'outfit_key': 'oteryn:outfit.revenant'},
        'compatible_provenance': ['crystal-201189f-soul-war-revenant-v1']})
    # Keep the accepted Rotten Blood base receipt usable after catalogue promotion.
    # This owner deliberately has no admitted addon grant.
    mappings.append({'grant_key': 'oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base',
        'target': {'kind': 'Outfit', 'outfit_key': 'oteryn:outfit.decaying_defender'},
        'compatible_provenance': ['owner-pr2784-decaying-defender-base-v1']})
    for row in outfits:
        if row['key'] in ('oteryn:outfit.hunter','oteryn:outfit.mage','oteryn:outfit.summoner'):
            for sex in ('female','male'):
                for addon in range(row['addon_count']+1):
                    mappings.append({'grant_key':row['key']+f'/quest-grant/{sex}/{addon}',
                        'target':{'kind':'Outfit','outfit_key':row['key']} if addon==0 else {'kind':'Addon','outfit_key':row['key'],'addon':addon},
                        'sex':sex,'compatible_provenance':[PROVENANCE]})
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
    # NX-5's promoted item grants and mounts remain admitted alongside quest rewards.
    nx5 = json.loads((ROOT / 'tools/content-schema/cosmetics-authoring/nx5-authoring.json').read_bytes())
    cosmetics.validate(nx5)
    nx5_doc = nx5['document']
    doc['outfits'].extend(nx5_doc['outfits'])
    # Reliable Ram uses the named look, not a mutable donor mount ordinal.
    # Crystal 201189f actions_reliable_ram.lua still says 250, but its mounts.xml
    # assigns Reliable Ram/look1934 to 252 (250 is Jaracal/look1962). The retained
    # 9f5a72c corpus agrees on the semantic Ram identity/look and source speed.
    ram = next(row for row in ET.fromstring(inputs['data/XML/mounts.xml'])
               if row.get('name') == 'Reliable Ram')
    if ram.get('clientid') != '1934' or ram.get('speed') != '10' or ram.get('premium') != 'no':
        raise ValueError('Reliable Ram source identity drift')
    ram_source = {'reference': 'crystal:data/XML/mounts.xml', 'revision': PIN,
                  'sha256': sha(inputs['data/XML/mounts.xml'])}
    ram_key = 'oteryn:content.mount.reliable_ram'
    doc['mounts'] = [*nx5_doc['mounts'], {'key': ram_key, 'revision': 'srv7-reliable-ram-r1',
        'look': {'id':1934,'source_record_sha256':ram_look_hash},
        'speed_bonus':10,'premium':False,'source':ram_source}]
    mappings.append({'grant_key':ram_key+'/quest-grant',
                     'target':{'kind':'Mount','mount_key':ram_key},
                     'compatible_provenance':[PROVENANCE]})
    grant_keys = {row['grant_key'] for row in mappings}
    for row in nx5_doc['grant_mappings']:
        if row['grant_key'] not in grant_keys:
            mappings.append(row)
        else:
            existing = next(m for m in mappings if m['grant_key'] == row['grant_key'])
            if existing['target'] != row['target']:
                existing['outfit_aliases'] = [row['target']['outfit_key']]
    # Taming uses the same production cosmetics owner as quest/item grants.
    taming_source = taming_cosmetics.append(ROOT, doc, looks)
    nx5_promotions = {row['path']: row for row in nx5['promotions']}
    quest_outfits = len(outfits) - len(nx5_doc['outfits'])
    for path in sorted(cosmetics.promotion_paths(doc)):
        parts = path.split('/')
        nx5_path = path if parts[1] == 'mounts' and int(parts[2]) < len(nx5_doc['mounts']) else None
        if parts[1] == 'outfits' and int(parts[2]) >= quest_outfits:
            nx5_path = '/outfits/' + str(int(parts[2]) - quest_outfits) + '/' + parts[3]
        if nx5_path is not None:
            author['promotions'].append({**nx5_promotions[nx5_path], 'path': path})
            continue
        citation=palette_source if path == '/palette' else source
        if parts[1] == 'mounts':
            citation = ram_source if doc['mounts'][int(parts[2])]['key'] == ram_key else taming_source
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
            requires=(), present=(), achievement=None, fee=None):
        path = f'data-global/npc/{npc}.lua'
        callbacks.append({'key': 'oteryn:quest-reward/' + name, 'quest': 'oteryn:quest.' + quest,
            'npc': 'oteryn:npc.' + npc, 'keywords': keywords, 'topic': topic, 'next_topic': next_topic,
            'storage': states, 'take': [{'item': f'oteryn:item.tibia.i{i}', 'count': n} for i,n in take],
            'give': None if give is None else {'item': f'oteryn:item.tibia.i{give[0]}', 'count': give[1], 'contents': []},
            'outfit': None if outfit is None else {'outfit': 'oteryn:outfit.' + outfit[0], 'addons': outfit[1]},
            'achievement': achievement or ('oteryn:achievement/backpack_tourist' if name == 'sam-backpack' else None),
            'reply': ['Thank you for your help.'],
            'sources': [f'crystal:{PIN}:{path}:sha256:{sha(inputs[path])}']})
        if fee is not None:
            callbacks[-1]['fee']=fee
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
    def requirement(track,value,maximum=None):
        return {'track':track,'minimum':value,'maximum':value if maximum is None else maximum}
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
    annihilator=prefix+'u7_24/the_annihilator/reward'
    add('l3-demon-base','the_annihilator_quest','avar_tar',['base'],
        [storage(annihilator,1,2)],outfit=('demon',0),topic=2)
    for name,quest,track,before,after,bit in [
        ('shield','demon_helmet_quest',prefix+'u6_4/demon_helmet/rewards/demon_helmet',1,2,1),
        ('helmet','demon_oak',prefix+'u8_2/the_demon_oak/done',3,4,2)]:
        add('l3-demon-'+name,quest,'avar_tar',[name],[storage(track,before,after)],
            outfit=('demon',bit),topic=2,requires=[requirement(annihilator,2)])
    for word,topic,next_topic in [('outfit',0,1),('yes',1,2)]:
        offers.append({'npc':'oteryn:npc.avar_tar','keywords':[word],'topic':topic,
            'next_topic':next_topic,'storage':[],'reply':['Choose the base, shield or helmet.']})
    dark=prefix+'u10_50/dark_trails/'
    votes=prefix+'u10_50/oramond_quest/voting_points'
    glooth=prefix+'u10_50/glooth_engineer_outfits/'
    add('l3-glooth-engineer-base','dark_trails_quest','ezebeth',['outfit'],
        [storage(dark+'outfit',-1,1,0)],outfit=('glooth_engineer',0),
        requires=[requirement(dark+'mission18',1)])
    for suffix,bit,minimum,maximum,finished_second in [
        ('second',2,6,2147483647,False),('first-low',1,3,5,False),('first-high',1,6,2147483647,True)]:
        required=[requirement(dark+'outfit',1),{'track':votes,'minimum':minimum,'maximum':maximum}]
        if finished_second:required.append(requirement(glooth+'addon2',1))
        add('l3-glooth-engineer-'+suffix,'glooth_engineer_outfits_quest','ezebeth',['addon'],
            [storage(glooth+'addon'+str(bit),-1,1,0)],outfit=('glooth_engineer',bit),requires=required)
    declarations=[{'key':votes,'quest':'oteryn:quest.oramond_quest','initial':-1,'min':-1,'max':2147483647,
        'sources':[f'crystal:{PIN}:data-global/npc/ezebeth.lua:93-103:sha256:{sha(inputs["data-global/npc/ezebeth.lua"])}']}]
    noble=prefix+'u7_8/nobleman_outfits/'
    first=noble+'nobleman_first_addon'
    second=noble+'nobleman_second_addon'
    for name,track,topic,pay_topic,bit,words in [('first',first,23,24,1,['dress','coat']),('second',second,29,30,2,['hat'])]:
        parent=[] if bit==1 else [storage(first,2,2,preserve=True)]
        add('l4-nobleman-'+name+'-start','nobleman_outfits_quest','irmana',['150000'],
            parent+[storage(track,-1,1,0)],topic=topic)
        add('l4-nobleman-'+name,'nobleman_outfits_quest','irmana',['yes'],
            parent+[storage(track,1,2)],outfit=('nobleman',bit),topic=pay_topic,
            fee={'gold':150000,'include_bank':False},achievement='oteryn:achievement/aristocrat' if bit==2 else None)
        start_topic=19 if bit==1 else 25
        guard=parent+[storage(track,-1,0,0,preserve=True)]
        for keywords,before,after in [(['addon'],0,start_topic),(['yes'],start_topic,start_topic+1),
            (words,start_topic+1,start_topic+2),(['yes'],start_topic+2,start_topic+3),(['yes'],start_topic+3,topic)]:
            offers.append({'npc':'oteryn:npc.irmana','keywords':keywords,'topic':before,'next_topic':after,
                'storage':guard,'reply':['The accessory costs 150000 gold carried with you.']})
        offers.append({'npc':'oteryn:npc.irmana','keywords':['addon'],'topic':0,'next_topic':pay_topic,
            'storage':parent+[storage(track,1,1,preserve=True)],'reply':['Are you here to collect your accessory for 150000 gold?']})
    royal='oteryn:quest-progress/outfit_quest/royal_costume_outfit'
    for name,before,after,silver,gold,bit,topic,word in [
        ('base',-1,1,15000,12500,0,16,'armor'),('shield',1,2,7500,6250,1,17,'shield'),('crown',2,3,7500,6250,2,18,'crown')]:
        guard=[storage(royal,before,after,0 if bit==0 else before)]
        add('l4-royal-costume-'+name,'royal_costume_outfits_quest','percybald',['yes'],guard,
            take=[(22516,silver),(22721,gold)],outfit=('royal_costume',bit),topic=topic,next_topic=15)
        if name == 'crown':
            callbacks[-1]['completes'] = True
        offers.append({'npc':'oteryn:npc.percybald','keywords':[word],'topic':15,'next_topic':topic,
            'storage':guard,'reply':[f'The donation is {silver} silver tokens and {gold} gold tokens.']})
    for words,before,after in [(['outfit','addon','royal'],0,14),(['yes'],14,15)]:
        offers.append({'npc':'oteryn:npc.percybald','keywords':words,'topic':before,'next_topic':after,
            'storage':[],'reply':['Choose armor, shield or crown for a token donation.']})
    pirate=prefix+'u7_8/pirate_outfits/'
    shattered=prefix+'u7_8/the_shattered_isles/'
    pirate_owner='outfit_and_addon_quests'  # retain the admitted shared source track owner
    add('l4-pirate-base',pirate_owner,'raymond_striker',['pirate outfit'],
        [storage(pirate+'pirate_base_outfit',-1,1,0)],outfit=('pirate',0),requires=[
            requirement(shattered+'access_to_laguna_island',1),requirement(shattered+'rays_mission4',5)])
    sabre=pirate+'pirate_sabre_addon'
    start=[storage(sabre,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',-1,1,1)]
    add('l4-pirate-sabre-start',pirate_owner,'duncan',['yes'],start,topic=2)
    for words,before,after in [(['task'],0,1),(['yes'],1,2)]:
        offers.append({'npc':'oteryn:npc.duncan','keywords':words,'topic':before,'next_topic':after,
            'storage':[storage(sabre,-1,0,0,preserve=True)],'reply':['Bring 100 eye patches, 100 peg legs and 100 hooks in that order.']})
    for name,before,after,item,topic,word in [
        ('eye-patches',1,2,6098,3,'eye patches'),('peg-legs',2,3,6126,4,'peg legs'),('hooks',3,4,6097,5,'hooks')]:
        states=[storage(sabre,before,after)]
        add('l4-pirate-'+name,pirate_owner,'duncan',['yes'],states,take=[(item,100)],topic=topic)
        offers.append({'npc':'oteryn:npc.duncan','keywords':[word],'topic':0,'next_topic':topic,
            'storage':states,'reply':['Have you gathered 100 '+word+'?']})
    # Morgan's existing firebird callback owns the only sabre grant; do not duplicate it.
    hat=pirate+'pirate_hat_addon'
    hat_start=[storage(pirate+'pirate_base_outfit',1,1,preserve=True),storage(hat,-1,1,0)]
    add('l4-pirate-hat-start',pirate_owner,'ariella',['yes'],hat_start,topic=4)
    for words,before,after in [(['addon'],0,2),(['task'],2,3),(['yes'],3,4)]:
        offers.append({'npc':'oteryn:npc.ariella','keywords':words,'topic':before,'next_topic':after,
            'storage':hat_start,'reply':['Bring proof from the four pirate leaders.']})
    add('l4-pirate-hat',pirate_owner,'ariella',['yes'],[storage(hat,1,2)],
        take=[(6101,1),(6102,1),(6100,1),(6099,1)],outfit=('pirate',2),topic=5)
    offers.append({'npc':'oteryn:npc.ariella','keywords':['task'],'topic':0,'next_topic':5,
        'storage':[storage(hat,1,2)],'reply':['Did you bring the shirt, sabre, hat and eye patch?']})
    oriental=prefix+'u7_8/oriental_outfits/'
    orc_powder=prefix+'u7_6/explorer_society/the_orc_powder'
    # The door belongs to Oriental; the source addon/global-start tracks retain their shared owner.
    for sex,first_npc,second_npc in [('female','ishina','miraia'),('male','habdel','razan')]:
        first=oriental+'first_oriental_addon'
        second=oriental+'second_oriental_addon'
        common='oteryn:quest-progress/outfit_quest/default_start'
        states=[storage(first,-1,1,0),storage(common,-1,1,1)]
        add('l5-oriental-'+sex+'-first-start','outfit_and_addon_quests',first_npc,['yes'],states,topic=2,
            requires=[requirement(orc_powder,35,2147483647)])
        callbacks[-1]['creation_sex']=sex
        callbacks[-1]['participants']=[{'quest':'oteryn:quest.oriental_outfits_quest',
            'storage':[storage(oriental+'oriental_door',0,1,1)]}]
        for words,before,after in [(['outfit'],0,1),(['yes'],1,2)]:
            offers.append({'npc':'oteryn:npc.'+first_npc,'keywords':words,'topic':before,'next_topic':after,
                'storage':states,'requires':[requirement(orc_powder,35,2147483647)],'reply':['Could you bring a mermaid comb?']})
        add('l5-oriental-'+sex+'-comb','outfit_and_addon_quests',first_npc,['yes'],[storage(first,1,2)],
            take=[(5945,1)],outfit=('oriental',1),topic=3)
        callbacks[-1]['creation_sex']=sex
        offers.append({'npc':'oteryn:npc.'+first_npc,'keywords':['comb'],'topic':0,'next_topic':3,
            'storage':[storage(first,1,2)],'reply':['Have you brought the mermaid comb?']})
        states=[storage(second,-1,1,0),storage(common,-1,1,1)]
        add('l5-oriental-'+sex+'-second-start','outfit_and_addon_quests',second_npc,['yes'],states,topic=2)
        callbacks[-1]['creation_sex']=sex
        for words,before,after in [(['task'],0,1),(['yes'],1,2)]:
            offers.append({'npc':'oteryn:npc.'+second_npc,'keywords':words,'topic':before,'next_topic':after,
                'storage':states,'reply':['Bring ape fur, fish fins, enchanted chicken wings and blue cloth in order.']})
        for name,word,item,count,before in [('ape-fur','ape fur',5883,100,1),('fish-fins','fish fins',5895,100,2),
            ('wings','enchanted chicken wings',5891,2,3),('blue-cloth','blue cloth',5912,100,4)]:
            states=[storage(second,before,before+1)]
            add('l5-oriental-'+sex+'-'+name,'outfit_and_addon_quests',second_npc,['yes'],states,
                take=[(item,count)],outfit=('oriental',2) if before==4 else None,topic=3)
            callbacks[-1]['creation_sex']=sex
            offers.append({'npc':'oteryn:npc.'+second_npc,'keywords':[word],'topic':0,'next_topic':3,
                'storage':states,'reply':['Did you bring the '+word+'?']})
    ice=prefix+'u8_0/the_ice_islands/'
    norseman=ice+'norseman_outfit'
    shared_start='oteryn:quest-progress/outfit_quest/default_start'
    add('l5-norseman-base','the_ice_islands_quest','hjaern',['mission'],
        [storage(ice+'questline',39,40),storage(ice+'mission12',0,6,2147483647),storage(ice+'yakchal_door',0,1,1)],
        take=[(7289,1)],outfit=('norseman',0),requires=[requirement(ice+'obelisk0'+str(n),5) for n in range(1,5)])
    callbacks[-1]['participants']=[{'quest':'oteryn:quest.outfit_and_addon_quests',
        'storage':[storage(norseman,0,1,3),storage(shared_start,-1,1,1)]}]
    for name,count,before,topic in [('first',5,40,3),('second',10,42,4)]:
        states=[storage(ice+'questline',before,before+1)]
        add('l5-norseman-'+name+'-shards','the_ice_islands_quest','hjaern',['yes'],states,take=[(7290,count)],topic=topic)
        # The donor's shard keyword is a root branch, including after reward topic3/4.
        for current_topic in (0,3,4):
            offers.append({'npc':'oteryn:npc.hjaern','keywords':['shard'],'topic':current_topic,'next_topic':topic,
                'storage':states,'reply':['Have you brought '+str(count)+' frostheart shards?']})
        bit=1 if name=='first' else 2
        add('l5-norseman-'+name+'-reward','the_ice_islands_quest','hjaern',['reward'],
            [storage(ice+'questline',before+1,before+2)],outfit=('norseman',bit),next_topic=topic)
        callbacks[-1]['participants']=[{'quest':'oteryn:quest.outfit_and_addon_quests',
            'storage':[storage(norseman,0,bit+1,3)]}]
    rook_body=prefix+'u7_8/druid_outfits_rook/druid_body_addon_rook'
    add('l6-druid-rook-start','druid_outfits_quest_on_rook','tom',['addon'],[storage(rook_body,-1,1,0)])
    add('l6-druid-rook-body','druid_outfits_quest_on_rook','tom',['yes'],[storage(rook_body,1,2)],
        take=[(5896,50),(5897,50)],outfit=('druid',1),topic=1)
    offers.append({'npc':'oteryn:npc.tom','keywords':['paws','bear paws'],'topic':0,'next_topic':1,
        'storage':[storage(rook_body,1,2)],'reply':['Have you brought 50 bear paws and 50 wolf paws?']})
    citizen_rook=prefix+'u7_8/citizen_outfits_rook/'
    rook_ref='oteryn:quest-progress/outfit_quest/ref'
    ref_source=f'crystal:{PIN}:data-global/npc/amber.lua:103-106:sha256:{sha(inputs["data-global/npc/amber.lua"])}'
    declarations.append({'key':rook_ref,'quest':'oteryn:quest.outfit_and_addon_quests','initial':0,
        'min':0,'max':2147483647,'writable':True,'sources':[ref_source]})
    backpack=citizen_rook+'addon_backpack_rook'
    mission=citizen_rook+'mission_backpack_rook'
    timer=citizen_rook+'addon_backpack_rook_timer'
    states=[storage(backpack,-1,1,0),storage(mission,0,1,4)]
    add('l6-citizen-rook-backpack-start','citizen_outfits_quest_on_rook','amber',['yes'],states,topic=2)
    callbacks[-1]['participants']=[{'quest':'oteryn:quest.outfit_and_addon_quests','storage':[],
        'state_ops':[{'track':rook_ref,'from':{'kind':'between','minimum':0,'maximum':2147483646},
            'effect':{'kind':'add','value':1}}]}]
    for words,before,after in [(['addon','outfit'],0,1),(['backpack','minotaur','leather'],1,2)]:
        offers.append({'npc':'oteryn:npc.amber','keywords':words,'topic':before,'next_topic':after,
            'storage':states,'reply':['Would you bring 100 minotaur leathers for a handmade backpack?']})
    states=[storage(backpack,1,2),storage(mission,1,2)]
    add('l6-citizen-rook-backpack-materials','citizen_outfits_quest_on_rook','amber',['yes'],states,take=[(5878,100)],topic=3)
    callbacks[-1]['state_ops']=[{'track':timer,'from':{'kind':'any'},'effect':{'kind':'set_now'}}]
    offers.append({'npc':'oteryn:npc.amber','keywords':['addon','outfit','backpack','leather'],'topic':0,'next_topic':3,
        'storage':states,'reply':['Did you bring 100 minotaur leathers? Your backpack takes two hours to make.']})
    add('l6-citizen-rook-backpack','citizen_outfits_quest_on_rook','amber',['addon','outfit','backpack'],
        [storage(backpack,2,3),storage(mission,2,4)],outfit=('citizen',1))
    callbacks[-1]['state_ops']=[{'track':timer,'from':{'kind':'elapsed_at_least','seconds':7201},'effect':{'kind':'add','value':0}}]
    callbacks[-1]['participants']=[{'quest':'oteryn:quest.outfit_and_addon_quests','storage':[storage(rook_ref,0,0,2147483647)]}]
    hat=citizen_rook+'addon_hat_rook'
    hat_mission=citizen_rook+'mission_hat_rook'
    states=[storage(hat,-1,1,0),storage(hat_mission,0,1)]
    add('l6-citizen-rook-hat-start','citizen_outfits_quest_on_rook','norma',['yes'],states,topic=1)
    offers.append({'npc':'oteryn:npc.norma','keywords':['addon','outfit','hat'],'topic':0,'next_topic':1,
        'storage':states,'reply':['Shall I teach you how to make a feather hat?']})
    states=[storage(hat,1,2),storage(hat_mission,1,0)]
    add('l6-citizen-rook-hat','citizen_outfits_quest_on_rook','norma',['yes'],states,
        take=[(3374,1),(5902,50),(5890,100)],outfit=('citizen',2),topic=2)
    offers.append({'npc':'oteryn:npc.norma','keywords':['addon','outfit','hat'],'topic':0,'next_topic':2,
        'storage':states,'reply':['Have you brought a legion helmet, 50 honeycombs and 100 chicken feathers?']})
    wizard_cosmetics.append(add, storage, offers, prefix)
    beggar=prefix+'u7_8/beggar_outfits/'
    beggar_stage=beggar+'beggar_outfit'
    beggar_timer=beggar+'beggar_outfit_timer'
    beggar_addon_timer=beggar+'beggar_outfit_timer_addon'
    beggar_door=beggar+'beggar_first_addon_door'
    general='outfit_and_addon_quests'
    def beggar_source(npc,lines):
        path='data-global/npc/'+npc+'.lua'
        return f'crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}'
    declarations.append({'key':beggar_door,'quest':'oteryn:quest.'+general,'initial':0,
        'min':-1,'max':1,'writable':True,'sources':[beggar_source('simon_the_beggar','158-166')]})
    def beggar_offer(npc,words,topic,next_topic,states,reply):
        offers.append({'npc':'oteryn:npc.'+npc,'keywords':words,'topic':topic,
            'next_topic':next_topic,'storage':states,'reply':[reply]})
    states=[storage(beggar_stage,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l6-beggar-start',general,'hugo',['yes'],states,topic=3)
    callbacks[-1]['requires_premium']=True
    for word,topic,next_topic in [('outfit',0,2),('yes',2,3)]:
        beggar_offer('hugo',[word],topic,next_topic,states,'Will you help gather the materials for my creation?')
    for suffix,word,item,count,stage in [('cloth','20 brown pieces of cloth',5913,20,1),
            ('leather','50 minotaur leathers',5878,50,2),('wings','10 bat wings',5894,10,3),
            ('blossoms','30 heaven blossoms',5921,30,4)]:
        states=[storage(beggar_stage,stage,stage+1)]
        add('l6-beggar-'+suffix,general,'hugo',['yes'],states,take=[(item,count)],topic=4)
        if suffix=='blossoms':
            callbacks[-1]['state_ops']=[{'track':beggar_timer,'from':{'kind':'any'},'effect':{'kind':'set_now'}}]
        beggar_offer('hugo',[word],0,4,states,f'Have you brought {count} of the requested materials?')
    states=[storage(beggar_stage,5,6)]
    add('l6-beggar-base',general,'hugo',['yes'],states,outfit=('beggar',0),topic=5)
    callbacks[-1]['requires_premium']=True
    callbacks[-1]['state_ops']=[{'track':beggar_timer,'from':{'kind':'elapsed_at_least','seconds':86401},'effect':{'kind':'add','value':0}}]
    beggar_offer('hugo',['outfit'],0,5,states,'Have you come to collect your new outfit?')
    states=[storage(beggar_stage,6,7)]
    add('l6-beggar-first-start',general,'simon_the_beggar',['yes'],states,topic=2)
    callbacks[-1]['requires_outfits']=['oteryn:outfit.beggar']
    for word,topic,next_topic in [('outfit',0,1),('yes',1,2)]:
        beggar_offer('simon_the_beggar',[word],topic,next_topic,states,'Will you bring 100 ape fur and pay 20000 gold for our deal?')
    states=[storage(beggar_stage,7,8),storage(beggar_door,-1,0,0,preserve=True)]
    add('l6-beggar-first-materials',general,'simon_the_beggar',['yes'],states,
        take=[(5883,100)],fee={'gold':20000,'include_bank':True},topic=3)
    callbacks[-1]['requires_premium']=True
    callbacks[-1]['state_ops']=[{'track':beggar_addon_timer,'from':{'kind':'any'},'effect':{'kind':'set_now'}}]
    beggar_offer('simon_the_beggar',['100 ape fur'],0,3,states,'Did you bring the fur and the 20000 gold fee?')
    add('l6-beggar-dress',general,'simon_the_beggar',['gypsy dress'],
        [storage(beggar_stage,8,10),storage(beggar_door,-1,1,0)],outfit=('beggar',1))
    callbacks[-1]['creation_sex']='female'
    add('l6-beggar-beard-start',general,'simon_the_beggar',['beard'],[storage(beggar_stage,8,9)])
    callbacks[-1]['creation_sex']='male'
    callbacks[-1]['state_ops']=[{'track':beggar_addon_timer,'from':{'kind':'any'},'effect':{'kind':'set_now'}}]
    add('l6-beggar-beard',general,'simon_the_beggar',['beard','addon'],
        [storage(beggar_stage,9,10),storage(beggar_door,-1,1,0)],outfit=('beggar',1))
    callbacks[-1]['creation_sex']='male'
    callbacks[-1]['state_ops']=[{'track':beggar_addon_timer,'from':{'kind':'elapsed_at_least','seconds':432000},'effect':{'kind':'add','value':0}}]
    states=[storage(beggar_stage,10,11),storage(beggar_door,1,1)]
    add('l6-beggar-staff-start',general,'simon_the_beggar',['yes'],states,topic=6)
    for word,topic,next_topic in [('addon',0,4),('staff',4,5),('yes',5,6)]:
        beggar_offer('simon_the_beggar',[word],topic,next_topic,states,'Will you retrieve my favourite staff?')
    states=[storage(beggar_stage,11,11,preserve=True),storage(beggar+'beggar_second_addon',0,2,1)]
    add('l6-beggar-staff',general,'simon_the_beggar',['yes'],states,take=[(6107,1)],outfit=('beggar',2),topic=7)
    callbacks[-1]['requires_premium']=True
    beggar_offer('simon_the_beggar',['staff'],0,7,states,'Did you bring my favourite staff?')
    for callback in callbacks:
        if callback['key'].startswith('oteryn:quest-reward/l6-beggar-'):
            npc=callback['npc'].split('.')[-1]
            callback['sources']=[beggar_source(npc,'55-199' if npc=='hugo' else '84-222')]
    def source_offer(npc,words,topic,next_topic,states):
        offers.append({'npc':'oteryn:npc.'+npc,'keywords':words,'topic':topic,'next_topic':next_topic,
            'storage':states,'reply':['Will you help me with this task?']})
    knight=prefix+'u7_8/knight_outfits/'
    ape=prefix+'u7_6/the_ape_city/questline'
    knight_helmet=knight+'addon_helmet'
    knight_mission=knight+'mission_helmet'
    for key,maximum,npc,lines in [(knight_helmet,7,'gregor','63-211'),
            (knight+'ramsays_helmet_door',1,'gregor','149-160'),
            (prefix+'u7_8/druid_outfits/druid_amulet_door',1,'ceiron','174-180'),
            ('oteryn:quest-progress/atrad',2,'atrad','75-104')]:
        path='data-global/npc/'+npc+'.lua'
        declarations.append({'key':key,'quest':'oteryn:quest.'+general,'initial':0,'min':-1,
            'max':maximum,'writable':True,'sources':[f'crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}']})
    def stamp(track):
        return {'track':track,'from':{'kind':'any'},'effect':{'kind':'set_now'}}
    def elapsed(track,seconds):
        return {'track':track,'from':{'kind':'elapsed_at_least','seconds':seconds},'effect':{'kind':'add','value':0}}
    add('l7-knight-sword-start',general,'morgan',["knight's sword"],[storage(knight+'addon_sword',-1,1,0)])
    callbacks[-1]['forbids_addons']=[{'outfit':'oteryn:outfit.knight','addons':1}]
    next(c for c in callbacks if c['key']=='oteryn:quest-reward/knight-sword')['forbids_addons']=[{'outfit':'oteryn:outfit.knight','addons':1}]
    states=[storage(knight_helmet,-1,1,0),storage(knight_mission,0,1,6)]
    add('l7-knight-helmet-start',general,'gregor',['yes'],states,topic=2,
        requires=[{'track':ape,'minimum':16,'maximum':18}])
    callbacks[-1]['requires_premium']=True
    callbacks[-1]['state_ops']=[{'track':rook_ref,'from':{'kind':'between','minimum':0,'maximum':2147483646},'effect':{'kind':'add','value':1}}]
    source_offer('gregor',['task'],0,1,states)
    source_offer('gregor',['yes'],1,2,states)
    for name,word,item,count,before,topic in [('fangs','behemoth fang',5893,100,1,3),
            ('helmet','ramsay the reckless helmet',5924,1,2,4),('sweat','sweat',5885,1,3,5),('steel','royal steel',5887,1,4,6)]:
        states=[storage(knight_helmet,before,before+1),storage(knight_mission,-1,before+1,6)]
        if name=='fangs':states.append(storage(knight+'ramsays_helmet_door',-1,1,1))
        add('l7-knight-'+name,general,'gregor',['yes'],states,take=[(item,count)],topic=topic,
            requires=[{'track':ape,'minimum':16,'maximum':18}])
        source_offer('gregor',[word],0,topic,states)
    add('l7-knight-helmet-work',general,'sam',['adorn','outfit','addon'],
        [storage(knight_helmet,5,6),storage(knight_mission,-1,6,6)])
    callbacks[-1]['state_ops']=[stamp(knight+'addon_helmet_timer')]
    add('l7-knight-helmet-reward',general,'sam',['adorn','outfit','addon'],
        [storage(knight_helmet,6,7),storage(knight_mission,-1,0,6),storage(rook_ref,0,0,2147483647)],outfit=('knight',2))
    callbacks[-1]['state_ops']=[elapsed(knight+'addon_helmet_timer',7201)]
    druid=prefix+'u7_8/druid_outfits/'
    body=druid+'druid_body_addon'
    hat=druid+'druid_hat_addon'
    add('l7-druid-body-start',general,'ustan',['addon'],[storage(body,-1,1,0)])
    states=[storage(body,1,2)]
    add('l7-druid-body',general,'ustan',['yes'],states,take=[(5896,50),(5897,50)],outfit=('druid',1),topic=1)
    source_offer('ustan',['paws','bear paws'],0,1,states)
    states=[storage(hat,-1,1,0),storage(druid+'default_start',0,1,1)]
    add('l7-druid-hat-start',general,'ceiron',['yes'],states,give=(4867,1),topic=4)
    for words,topic,next_topic in [(['addon','outfit'],0,2),(['faolan'],2,3),(['yes'],3,4)]:
        source_offer('ceiron',words,topic,next_topic,states)
    states=[storage(hat,1,2)]
    add('l7-druid-griffinclaw',general,'ceiron',['yes'],states,take=[(5937,1)],topic=5)
    source_offer('ceiron',['griffinclaw','container'],0,5,states)
    for name,before,topic,give in [('water-start',2,6,(5938,1)),('dust-start',4,8,None),('chain-start',6,10,None)]:
        states=[storage(hat,before,before+1)]
        if name=='chain-start':states.append(storage(druid+'druid_amulet_door',-1,1,1))
        add('l7-druid-'+name,general,'ceiron',['yes'],states,give=give,topic=topic)
        source_offer('ceiron',['task'],0,topic,states)
    for name,words,item,count,before,topic in [('water',['waterskin','water skin'],5939,1,3,7),
            ('dust',['dust','demon dust'],5906,100,5,9),('chain',['chain','wolf tooth chain'],5940,1,7,11)]:
        states=[storage(hat,before,before+1)]
        add('l7-druid-'+name,general,'ceiron',['yes'],states,take=[(item,count)],topic=topic)
        source_offer('ceiron',words,0,topic,states)
    add('l7-druid-language',general,'ceiron',['faolan'],[storage(hat,8,9)])
    states=[storage(hat,9,10)]
    add('l7-druid-hat',general,'a_majestic_warwolf',['yes'],states,outfit=('druid',2),topic=1)
    source_offer('a_majestic_warwolf',['addon','outfit'],0,1,states)
    assassin=prefix+'u7_8/assassin_outfits/'
    base=assassin+'assassin_base_outfit'
    first=assassin+'assassin_first_addon'
    second=assassin+'assassin_second_addon'
    states=[storage(base,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l7-assassin-base-start',general,'vescu',['yes'],states,topic=3)
    callbacks[-1]['requires_actor_conditions']=['drunk']
    for words,topic,next_topic in [(['sober'],0,1),(['potion'],1,2),(['yes'],2,3)]:
        source_offer('vescu',words,topic,next_topic,states)
    for name,word,item,count,before in [('eyes','30 bonelord eyes',5898,30,1),('scales','10 red dragon scales',5882,10,2),
            ('lizard','30 lizard scales',5881,30,3),('fins','20 fish fins',5895,20,4),('vampire','20 vampire dust',5905,20,5),
            ('demon','10 demon dust',5906,10,6),('sweat',"warrior's sweat",5885,1,7)]:
        states=[storage(base,before,before+1)]
        add('l7-assassin-'+name,general,'vescu',['yes'],states,take=[(item,count)],topic=4)
        source_offer('vescu',[word],0,4,states)
    add('l7-assassin-base',general,'vescu',['secret'],[storage(base,8,9)],outfit=('assassin',0))
    states=[storage(first,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l7-assassin-first-start',general,'erayo',['yes'],states,topic=2,
        requires=[{'track':prefix+'u7_8/the_shattered_isles/shipwrecked','minimum':2,'maximum':2}])
    callbacks[-1]['requires_outfits']=['oteryn:outfit.assassin']
    source_offer('erayo',['addon'],0,1,states)
    source_offer('erayo',['yes'],1,2,states)
    for index,(name,item) in enumerate([('blue',5912),('green',5910),('red',5911),('brown',5913),('yellow',5914),('white',5909),('yarn',5886)],1):
        count=10 if name=='yarn' else 50
        word='10 spools of yarn' if name=='yarn' else f'50 {name} cloth'
        states=[storage(first,index,index+1)]
        add('l7-assassin-'+name,general,'erayo',['yes'],states,take=[(item,count)],
            outfit=('assassin',1) if name=='yarn' else None,topic=3)
        source_offer('erayo',[word],0,3,states)
    atrad='oteryn:quest-progress/atrad'
    states=[storage(atrad,-1,1,0),storage(second,0,1)]
    add('l7-assassin-second-start',general,'atrad',['yes'],states,topic=3)
    callbacks[-1]['requires_actor_conditions']=['fire']
    source_offer('atrad',['addon','outfit'],0,2,states)
    source_offer('atrad',['yes'],2,3,states)
    add('l7-assassin-second',general,'atrad',['nose ring','ring'],[storage(atrad,1,2),storage(second,1,2)],
        take=[(5804,1),(5930,1)],outfit=('assassin',2))
    callbacks[-1]['requires_actor_conditions']=['fire']
    for callback in callbacks:
        if callback['key'].startswith('oteryn:quest-reward/l7-'):
            npc=callback['npc'].split('.')[-1];path='data-global/npc/'+npc+'.lua'
            lines={'morgan':'115-148','gregor':'63-211','sam':'67-88','ustan':'63-85','ceiron':'60-187','a_majestic_warwolf':'64-75','vescu':'56-189','erayo':'57-114','atrad':'60-105'}[npc]
            callback['sources']=[f'crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}']
    # L8: source looktypes are explicit sex-qualified grants, including cross-family rewards.
    for key,maximum,npc in [(prefix+'u7_8/hunter_outfits/hunter/addon_glove',1,'legola'),
            (prefix+'u7_8/mage_and_summoner_outfits/addon_hat_cloak',11,'myra')]:
        path='data-global/npc/'+npc+'.lua'
        declarations.append({'key':key,'quest':'oteryn:quest.'+general,'initial':0,'min':-1,'max':maximum,
            'writable':True,'sources':[f'crystal:{PIN}:{path}:sha256:{sha(inputs[path])}']})
    def targets(*values):
        callbacks[-1]['outfits']=[{'outfit':'oteryn:outfit.'+family,'addons':bit,'sex':sex} for family,bit,sex in values]
    hunter=prefix+'u7_8/hunter_outfits/'
    hat=hunter+'hunter_hat_addon'
    states=[storage(hat,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l8-hunter-start',general,'elane',['yes'],states,topic=3)
    for words,a,b in [(['addon','outfit'],0,1),(['task'],1,2),(['yes'],2,3)]:source_offer('elane',words,a,b,states)
    for name,words,before,topic,items in [('crossbow',['crossbow'],1,4,[(5947,1)]),
            ('leather',['leather'],2,5,[(5876,100),(5948,100)]),('wings',['chicken wing'],3,6,[(5891,5)]),
            ('hat',['steel'],4,7,[(5887,1),(5888,1),(5889,1)])]:
        states=[storage(hat,before,before+1)]
        add('l8-hunter-'+name,general,'elane',['yes'],states,take=items,topic=topic)
        source_offer('elane',words,0,topic,states)
        if name=='hat':targets(('hunter',1,'male'),('hunter',2,'female'))
    glove=hunter+'hunter/addon_glove'
    states=[storage(glove,-1,1,0)]
    add('l8-hunter-gloves',general,'legola',['yes'],states,take=[(5875,1)],topic=1)
    callbacks[-1]['requires_premium']=True
    targets(('hunter',2,'male'),('hunter',1,'female'))
    source_offer('legola',['sniper gloves'],0,1,states)
    mage=prefix+'u7_8/mage_and_summoner_outfits/'
    wand=mage+'addon_wand'
    states=[storage(wand,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l8-mage-male-start',general,'angelina',['yes'],states)
    states=[storage(wand,1,2)]
    add('l8-mage-male-wand-start',general,'lynda',['yes'],states,topic=2)
    source_offer('lynda',['angelina'],0,1,states);source_offer('lynda',['yes'],1,2,states)
    for name,words,before,topic,items in [('wands',['wand','rod'],2,3,[(i,1) for i in (3065,3066,3067,3069,3070,3071,3072,3073,3074,3075)]),
            ('sulphur',['sulphur'],3,4,[(5904,10)]),('soul',['soul stone'],4,5,[(5809,1)]),('ankhs',['ankh'],5,6,[(3077,20)])]:
        states=[storage(wand,before,before+1)]
        add('l8-mage-male-'+name,general,'lynda',['yes'],states,take=items,topic=topic)
        source_offer('lynda',words,0,topic,states)
        if name=='ankhs':callbacks[-1]['state_ops']=[stamp(mage+'addon_wand_timer')]
    add('l8-mage-male-ritual',general,'lynda',['ritual'],[storage(wand,6,7)])
    callbacks[-1]['state_ops']=[elapsed(mage+'addon_wand_timer',10801)]
    targets(('mage',1,'male'),('summoner',1,'female'))
    add('l8-mage-male-hat',general,'zoltan',['yes'],[],take=[(5903,1)],topic=1)
    callbacks[-1].update(creation_sex='male',exact_take=True,forbids_addons=[{'outfit':'oteryn:outfit.mage','addons':2,'sex':'male'}])
    targets(('mage',2,'male'),('summoner',2,'female'))
    source_offer('zoltan',['proof'],0,1,[])
    belt=mage+'addon_belt'
    states=[storage(belt,-1,1,0),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l8-mage-female-belt-start',general,'sandra',['yes'],states,topic=2)
    source_offer('sandra',['vial','ticket','bonus','deposit'],0,1,states);source_offer('sandra',['yes'],1,2,states)
    states=[storage(belt,1,2)]
    add('l8-mage-female-belt',general,'sandra',['yes'],states,take=[(5958,1)],topic=4)
    targets(('mage',1,'female'),('summoner',1,'male'))
    source_offer('sandra',['prize'],0,4,states)
    hat=mage+'addon_hat_cloak';mission=mage+'mission_hat_cloak'
    states=[storage(hat,-1,1,0),storage(mission,0,1,11)]
    add('l8-mage-female-hat-start',general,'myra',['yes'],states,topic=2)
    for words,a,b in [(['outfit'],0,100),(['tiara'],100,1),(['yes'],1,2)]:source_offer('myra',words,a,b,states)
    for before,(name,item,count,word) in enumerate([('wings',5894,70,'bat wings'),('cloth',5911,20,'red cloth'),
            ('fur',5883,40,'ape fur'),('orchids',5922,35,'holy orchids'),('yarn',5886,10,'yarn'),
            ('lizard',5881,60,'lizard scales'),('scales',5882,40,'red dragon scales'),('sulphur',5904,15,'sulphur'),('vampire',5905,30,'vampire dust')],1):
        states=[storage(hat,before,before+1),storage(mission,0,before+1,11)]
        add('l8-mage-female-'+name,general,'myra',['yes'],states,take=[(item,count)],topic=before+2)
        source_offer('myra',[word],0,before+2,states)
    add('l8-mage-female-hat',general,'zoltan',['myra'],[storage(hat,10,11),storage(mission,0,0,11)])
    callbacks[-1]['creation_sex']='female'
    targets(('mage',2,'female'),('summoner',2,'female'))
    for name,bit,item,track,words in [('mask',1,20276,'dream_outfit_1',['dream warden mask']),('claw',2,20275,'dream_outfit',['dream warden claw'])]:
        key='oteryn:quest-progress/'+track
        path='data-global/npc/eruaran.lua'
        declarations.append({'key':key,'quest':'oteryn:quest.'+general,'initial':0,'min':-1,'max':1,'writable':True,
            'sources':[f'crystal:{PIN}:{path}:153-199:sha256:{sha(inputs[path])}']})
        states=[storage(key,-1,1,0)]
        add('l8-dream-warden-'+name,general,'eruaran',['yes'],states,take=[(item,1)],outfit=('dream_warden',bit),topic=bit)
        callbacks[-1]['requires_premium']=True
        source_offer('eruaran',words,0,bit,states)
    # Gnomally's private config selector is lowered to distinct conversation
    # topics. Source trades are repeatable, with no quest storage side effect.
    for family,base,first,start,base_word,first_word in [
            ('soil-guardian',16252,16253,101,'basic soil guardian outfit','addition to the soil guardian outfit'),
            ('crystal-warlord',16255,16256,103,'basic crystal warlord outfit','addition to the crystal warlord armor outfit')]:
        quest=family.replace('-','_')+'_outfits_quest'
        for suffix,token,count,gift,topic,word in [('base',16129,20,base,start,base_word),('first',16128,70,first,start+1,first_word)]:
            add('l9-'+family+'-'+suffix,quest,'gnomally',['yes'],[],take=[(token,count)],give=(gift,1),topic=topic)
            source_offer('gnomally',[word],0,topic,[])
    add('l9-gnomish-supply','bigfoot_s_burden_quest','gnomally',['yes'],[],take=[(16128,2)],give=(15698,1),topic=105)
    source_offer('gnomally',['supply'],0,105,[])
    add('l9-afflicted-pouch','afflicted_outfits_quest','ottokar',['yes'],[],take=[(12517,1)],give=(12413,1),topic=1)
    callbacks[-1]['achievement_progress']=[{'key':'oteryn:achievement/doctor_doctor','target':100}]
    source_offer('ottokar',['medicine','medicine pouch','belongings of deceasead'],0,1,[])
    for callback in callbacks:
        if callback['key'].startswith('oteryn:quest-reward/l9-'):
            npc=callback['npc'].split('.')[-1];path='data-global/npc/'+npc+'.lua'
            lines='75-169' if npc=='gnomally' else '61-78'
            callback['sources']=[f'crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}']
    # Chosen Golden programme retains its authored prices and boots-first order.
    golden='oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit'
    for suffix,before,after,bit,gold,topic,word in [
            ('base',0,1,0,100000,3,'armor'),
            ('boots',1,2,1,50000,5,'boots'),
            ('helmet',2,3,2,50000,4,'helmet')]:
        guard=[storage(golden,before,after)]
        add('l10-golden-'+suffix,'golden_outfits_quest','king_tibianus',['yes'],guard,
            outfit=('golden',bit),topic=topic,next_topic=2,fee={'gold':gold,'include_bank':True})
        callbacks[-1]['sources'].append('owner:chosen-golden:100000-50000-50000:boots-before-helmet')
        offers.append({'npc':'oteryn:npc.king_tibianus','keywords':[word],'topic':2,'next_topic':topic,
            'storage':guard,'reply':[f'The chosen commission costs {gold} gold. Do you agree?']})
    for words,before,after in [(['outfit','addon'],0,1),(['yes'],1,2)]:
        offers.append({'npc':'oteryn:npc.king_tibianus','keywords':words,'topic':before,'next_topic':after,
            'storage':[],'reply':['Choose armor, boots or helmet for your golden commission.']})
    # Distinct order ranks choose the shared three documents' actual USE branch.
    parent='oteryn:quest-progress/chosen-source/dreamer_s_challenge_quest/s4'
    declarations.append({'key':parent,'quest':'oteryn:quest.dreamer_s_challenge_quest','initial':0,
        'min':0,'max':1,'writable':False,'sources':['owner:chosen-Dreamers-Challenge-terminal-puzzle-s4:read-only']})
    for family,npc,opposite,join_achievement,final_achievement in [
            ('brotherhood','the_bone_master','nightmare','bone_brother','skull_and_bones'),
            ('nightmare','the_dream_master','brotherhood','nightmare_knight','nightmare_walker')]:
        quest=family+'_outfits_quest'
        rank=prefix+'u7_9/'+family+'_outfits/outfits'
        other=prefix+'u7_9/'+opposite+'_outfits/outfits'
        requirements=[requirement(parent,1),requirement(other,0)]
        for stage,before,after,item,count,topic,achievement in [
                ('join',0,1,768,0,2,join_achievement),('officer',1,2,769,500,3,None),('leader',2,3,770,1000,4,final_achievement)]:
            guard=[storage(rank,before,after)]
            add('l10-'+family+'-'+stage,quest,npc,['yes'],guard,take=[] if count==0 else [(6499,count)],
                give=(item,1),topic=topic,requires=requirements,achievement='oteryn:achievement/'+achievement if achievement else None)
            callbacks[-1]['requires_premium']=True
            source_offer(npc,['join'] if stage=='join' else ['advancement','demonic','essence'],0,1 if stage=='join' else topic,
                [storage(rank,before,before,preserve=True)])
            offers[-1]['requires']=requirements
        source_offer(npc,['yes'],1,2,[storage(rank,0,0,preserve=True)])
        offers[-1]['requires']=requirements
        for stage,item,threshold,bit in [('follower',768,1,0),('officer',769,2,1),('leader',770,3,2)]:
            claim=prefix+'u7_9/'+family+'_outfits/document_claim_'+str(bit)
            declarations.append({'key':claim,'quest':'oteryn:quest.'+quest,'initial':0,'min':0,'max':1,'writable':True,
                'sources':[f'crystal:{PIN}:data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua:27-59:durable-one-shot-owned-guard']})
            add('l10-'+family+'-use-'+stage,quest,npc,['document '+stage],
                [storage(rank,threshold,threshold,4,preserve=True),storage(claim,0,1)],take=[(item,1)],outfit=(family,bit),requires=requirements)
            callback=callbacks[-1];callback['carried_item']=f'oteryn:item.tibia.i{item}';callback['requires_premium']=True
            if bit==0:callback['forbids_outfits']=['oteryn:outfit.'+family]
            else:
                callback['requires_outfits']=['oteryn:outfit.'+family]
                callback['forbids_addons']=[{'outfit':'oteryn:outfit.'+family,'addons':bit}]
            document_path='data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua'
            callback['sources'].append(f'crystal:{PIN}:{document_path}:1-69:sha256:{sha(inputs[document_path])}')
    # L11: source task rewards plus explicitly selected Oteryn qualification desks.
    dream_ready=[requirement(prefix+'u12_00/the_dream_courts/ward_stones/questline',3),
        requirement(prefix+'u12_00/the_dream_courts/main/the_summer_court',1)]
    wanted='oteryn:quest-progress/chosen-source/kilmaresh_quest/s21'
    declarations.append({'key':wanted,'quest':'oteryn:quest.kilmaresh_quest','initial':0,'min':0,'max':1,
        'sources':['owner:chosen-Kilmaresh-s21:report-three-Wanted-outlaws:read-only']})
    desks=[('dream-warrior','dream_warrior_outfits_quest','vanys','dream_warrior',250,dream_ready,'dream warrior outfit'),
        ('issavi','authored.citizen_of_issavi_outfits_quest','eshaya','citizen_of_issavi',250,[requirement(wanted,1)],'issavi attire')]
    for label,quest,npc,family,level,ready,word in desks:
        add('l11-'+label+'-base',quest,npc,[word],[],outfit=(family,0),requires=ready)
        callbacks[-1].update(minimum_level=level,requires_premium=True,forbids_outfits=['oteryn:outfit.'+family])
        if label=='issavi':callbacks[-1]['sources'].append('owner:chosen-Issavi-attire-keeper-role:Eshaya:Wanted-report-s21')
    for label,quest,npc,family,level,ready,tokens,carried in [
            ('dream-warrior','dream_warrior_outfits_quest','vanys','dream_warrior',250,dream_ready,[(30169,5),(30168,1)],False),
            ('issavi','authored.citizen_of_issavi_outfits_quest','eshaya','citizen_of_issavi',250,[requirement(wanted,1)],[(37003,1),(37002,1)],True),
            ('makeshift','makeshift_warrior_outfits_quest','gnomadness','makeshift_warrior',100,[],[(27657,1),(27656,1)],True)]:
        for bit,(item,count) in enumerate(tokens,1):
            add('l11-'+label+'-'+('first' if bit==1 else 'second'),quest,npc,
                ['document'] if carried else ['pomegranates' if bit==1 else 'ice shield'],[],
                take=[(item,count)],outfit=(family,bit),topic=0 if carried else 8,requires=ready,
                achievement='oteryn:achievement/cobbled_and_patched' if label=='makeshift' and bit==2 else None)
            callback=callbacks[-1];callback.update(minimum_level=level,requires_premium=True,
                requires_outfits=['oteryn:outfit.'+family],forbids_addons=[{'outfit':'oteryn:outfit.'+family,'addons':bit}])
            if bit==2:callback['requires_addons']=[{'outfit':'oteryn:outfit.'+family,'addons':1}]
            if carried:callback['carried_item']=f'oteryn:item.tibia.i{item}'
            if label=='issavi':callback['sources'].append('owner:chosen-Issavi-veil-addon1-scabbard-addon2:one-token-each')
        if not carried:
            for words,before,after in [(['addon'],0,6),(['yes'],6,7),(['yes'],7,8)]:
                source_offer(npc,words,before,after,[]);offers[-1]['requires']=ready
    qualification=prefix+'makeshift_warrior/qualification'
    declarations.append({'key':qualification,'quest':'oteryn:quest.makeshift_warrior_outfits_quest','initial':0,'min':0,'max':2,
        'writable':True,'sources':['owner:chosen-Gnome-Deep-Hub-salvage-apprenticeship:s1-s2:gnomadness-role']})
    add('l11-makeshift-start','makeshift_warrior_outfits_quest','gnomadness',['salvage'],[storage(qualification,0,1)],next_topic=11)
    callbacks[-1].update(minimum_level=100,requires_premium=True)
    callbacks[-1]['sources'].append('owner:chosen-Gnome-Deep-Hub-apprenticeship:meeting-admitted-Gnomadness-witnesses-exploration')
    add('l11-makeshift-base','makeshift_warrior_outfits_quest','gnomadness',['yes'],[storage(qualification,1,2)],outfit=('makeshift_warrior',0),topic=11)
    callbacks[-1].update(minimum_level=100,requires_premium=True,forbids_outfits=['oteryn:outfit.makeshift_warrior'])
    callbacks[-1]['sources'].append('owner:chosen-Gnome-Deep-Hub-salvage-apprenticeship:base-qualification')
    source_offer('gnomadness',['salvage'],0,11,[storage(qualification,1,1,preserve=True)])
    # L12: owner-selected fixed hunting trials; native deaths are the only counter writer.
    for label,family,quest,npc,level,counts,materials in [
        ('dragon-slayer','dragon_slayer','authored.dragon_slayer_outfits_quest','a_sweaty_cyclops',0,[('dragons',20),('dragon_lords',20)],[(5920,10),(5882,10)]),
        ('cave-explorer','cave_explorer','cave_explorer_outfits_quest','gnommander',80,[('kills',20)],[]),
        ('fire-fighter','fire_fighter','fire_fighter_outfits_quest','gnomadness',250,[('kills',10)],[])]:
        root='oteryn:quest-progress/chosen-trial/'+family+'/'
        provenance='owner:chosen-trial:'+family+':fixed-native-creature-deaths:questpc3-engine-5e72a3f0b'
        for track,maximum in [('phase',2)]+counts:
            declarations.append({'key':root+track,'quest':'oteryn:quest.'+quest,'initial':0,'min':0,'max':maximum,'writable':True,'sources':[provenance]})
        add('l12-'+label+'-start',quest,npc,[family.replace('_',' ')+' trial'],
            [storage(root+'phase',0,1)]+[storage(root+track,0,0,preserve=True) for track,_ in counts])
        callbacks[-1].update(requires_premium=True)
        if level:callbacks[-1]['minimum_level']=level
        callbacks[-1]['sources'].append(provenance)
        add('l12-'+label+'-base',quest,npc,[family.replace('_',' ')+' report'],
            [storage(root+'phase',1,2)]+[storage(root+track,maximum,maximum,preserve=True) for track,maximum in counts],
            take=materials,outfit=(family,0),fee={'gold':1000,'include_bank':False} if label=='cave-explorer' else None)
        callbacks[-1].update(requires_premium=True,forbids_outfits=['oteryn:outfit.'+family])
        if level:callbacks[-1]['minimum_level']=level
        callbacks[-1]['sources'].append(provenance)
        if label=='dragon-slayer':continue
        for bit in (1,2):
            add('l12-'+label+'-'+('first' if bit==1 else 'second'),quest,npc,[family.replace('_',' ')+' addon '+str(bit)],[],
                take=[(39544 if bit==1 else 39545,1)] if label=='fire-fighter' else [],outfit=(family,bit),
                fee={'gold':2000 if bit==1 else 3000,'include_bank':False} if label=='cave-explorer' else None,
                achievement='oteryn:achievement/friendly_fire' if label=='fire-fighter' and bit==2 else None)
            callbacks[-1].update(minimum_level=level,requires_premium=True,requires_outfits=['oteryn:outfit.'+family],
                forbids_addons=[{'outfit':'oteryn:outfit.'+family,'addons':bit}])
            if bit==2:callbacks[-1]['requires_addons']=[{'outfit':'oteryn:outfit.'+family,'addons':1}]
            callbacks[-1]['sources'].append(provenance)
    callbacks.sort(key=lambda c:c['key'])
    offers.sort(key=lambda c:(c['npc'],c['topic'],c['keywords']))
    validate_callbacks({'schema':1,'tracks':declarations,'callbacks':callbacks})
    native_cosmetics = cosmetics.provisioning_input(
        ROOT / cosmetics.AUTHORING, appearance / 'manifest.json', ROOT / cosmetics.OUTPUT,
        authoring_bytes=encoded(author))
    gameplay['schema']='OTERYN_NATIVE_GAMEPLAY_MANIFEST/v6'
    gameplay['cosmetics']={'path':'cosmetics/outfits/native-cosmetics.json','sha256':sha(native_cosmetics)}
    generated = {'content/cosmetics/outfits/authoring.json': encoded(author),
            'content/cosmetics/outfits/native-cosmetics.json': native_cosmetics,
            'content/spells.manifest.json': (json.dumps(gameplay,indent=2,sort_keys=True)+'\n').encode(),
            'content/quests/shared-reward-callbacks.json': encoded({'schema':1,'tracks':declarations,'callbacks':callbacks}),
            'content/quests/shared-reward-offers.json': encoded({'schema':1,'offers':offers})}
    # Preserve the source's complete contiguous 1..10001 supply-package table.
    random_path='data-global/scripts/actions/valuables/random_items.lua'
    parser=load_module('qengine_literal_item_tables','tools/content-migration/event_item_actions.py')
    config,_=parser.table(inputs[random_path].decode(),'config')
    chances=config[15698]['chances'];rewards=[];last=0
    if list(chances)!=list(range(1,26)):raise ValueError('GNOMISH_SOURCE_OUTCOME_COUNT')
    for row in chances.values():
        if row['from']!=last+1 or row['to']<row['from']:raise ValueError('GNOMISH_SOURCE_RANGE_GAP')
        count=row.get('count',1)
        rewards.append({'item':f"oteryn:item.tibia.i{row['itemId']}",'name':'','weight':row['to']-row['from']+1,'minimum':count,'maximum':count})
        last=row['to']
    if last!=10001:raise ValueError('GNOMISH_SOURCE_ROLL_BOUND')
    generated['tools/content-schema/native-gameplay/qengine-reward-bag-profiles.json']=encoded({
        'schema':'OTERYN_QENGINE_REWARD_BAG_PROFILES/v1','sources':[{'path':random_path,'revision':PIN,'sha256':sha(inputs[random_path])}],
        'bags':[{'constant':'QENGINE_GNOMISH_SUPPLY','bag':'oteryn:item.tibia.i15698','playable':True,
            'selection':'weighted','picks':1,'roll_max':10001,'donor':f'crystal:{PIN}:{random_path}:49-80:sha256:{sha(inputs[random_path])}',
            'rewards':rewards}]})
    greetings=[]
    for npc in sorted({c['npc'] for c in callbacks}):
        path='data-global/npc/'+npc.removeprefix('oteryn:npc.')+'.lua'
        messages={}
        citations=[]
        for field,constant in [('greet','GREET'),('farewell','FAREWELL'),('walkaway','WALKAWAY')]:
            matches=list(re.finditer(r'npcHandler:setMessage\(MESSAGE_'+constant+r',\s*"([^"\n]*)"\)',inputs[path].decode()))
            origin=path
            # Conditional NPC messages belong to their functional selectors.
            # Use the source handler's neutral default when no unique static message exists.
            if len(matches)!=1:
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
    reward_items.admit(ROOT,generated,reward_items.PROFILE,'content/spells.manifest.json',native.provider_order,cosmetics.canonical)
    return generated


def validate_callbacks(document):
    schema=json.loads((ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_bytes())
    Draft202012Validator(schema).validate(document)
    declarations=document.get('tracks',[])
    declared=set()
    readonly={r['track'] for c in document['callbacks'] for r in c.get('requires',[])}
    owners={}
    for c in document['callbacks']:
        for owner,states in [(c['quest'],c['storage']+c.get('state_ops',[]))]+[(p['quest'],p['storage']+p.get('state_ops',[])) for p in c.get('participants',[])]:
            for state in states: owners.setdefault(state['track'],set()).add(owner)
    written=set(owners)
    for track in declarations:
        writable=track.get('writable',False)
        if (track['key'] in declared or not track['min']<=track['initial']<=track['max']
            or (writable and owners.get(track['key'])!={track['quest']})
            or (not writable and (track['key'] not in readonly or track['key'] in written))):
            raise ValueError('invalid reward track declaration '+track['key'])
        declared.add(track['key'])
    keys=set()
    for callback in document['callbacks']:
        reward=callback.get('give')
        if reward and reward.get('text') is not None and len(reward['text'].encode('utf-8'))>7996:
            raise ValueError('literal reward text exceeds UTF-8 byte bound')
        storage=callback['storage']
        source, target = callback.get('carried_item'), callback.get('carried_target')
        if target is not None:
            if (source is None or source == target or not callback.get('give')
                    or callback.get('peer_requires') or callback.get('creature_target')
                    or any({'item': item, 'count': 1} not in callback['take'] for item in (source, target))):
                raise ValueError('invalid carried quest recipe '+callback['key'])
        if source is not None and (
                (target is None and (callback['take']!=[{'item':source,'count':1}]
                    or callback.get('give') is not None))
                or callback.get('fee') is not None or callback.get('requires_actor_conditions')):
            raise ValueError('invalid carried quest tool '+callback['key'])
        targets=([callback['outfit']] if callback.get('outfit') is not None else callback.get('outfits',[]))
        value_guard=any(t['outfit']==g['outfit'] and t['addons'] & g['addons']
            and (t.get('sex') is None or t.get('sex')==(g.get('sex') or callback.get('creation_sex')))
            for t in targets for g in callback.get('forbids_addons',[]))
        value_guard = value_guard or any(t['addons']==0 and t.get('sex') is None and t['outfit'] in callback.get('forbids_outfits',[]) for t in targets)
        value_guard = value_guard or (bool(callback['take']) and callback.get('give') is not None)
        if callback.get('outfits') and callback.get('outfit') is not None:
            raise ValueError('mixed legacy and target-set grants '+callback['key'])
        participants=callback.get('participants',[])
        participant_storage=[s for p in participants for s in p['storage']]
        participant_ops=[s for p in participants for s in p.get('state_ops',[])]
        operations=callback.get('state_ops',[])
        all_keys=[s['track'] for s in storage+operations+participant_storage+participant_ops]
        if (len(all_keys)>16 or len(all_keys)!=len(set(all_keys))
            or any(len(p['storage'])+len(p.get('state_ops',[]))>8 for p in participants)
            or len({p['quest'] for p in participants})!=len(participants)
            or any(p['quest']==callback['quest'] for p in participants)
            or any(s['minimum']>s['maximum'] for s in participant_storage)
            or any(s['from']['kind']=='between' and s['from']['minimum']>s['from']['maximum'] for s in operations+participant_ops)
            or {r['track'] for r in callback.get('requires',[])} & set(all_keys)):
            raise ValueError('invalid reward participants '+callback['key'])
        requires=callback.get('requires',[])
        if (callback['key'] in keys
            or len({s['track'] for s in storage})!=len(storage)
            or len({p['key'] for p in callback.get('achievement_progress',[])})!=len(callback.get('achievement_progress',[]))
            or len({i['item'] for i in callback['take']})!=len(callback['take'])
            or len({i['item'] for i in callback.get('present',[])})!=len(callback.get('present',[]))
            or (callback.get('fee') and any(i['item'] in ('oteryn:item.tibia.i3031','oteryn:item.tibia.i3035','oteryn:item.tibia.i3043') for i in callback['take']))
            or len({r['track'] for r in requires})!=len(requires)
            or {r['track'] for r in requires}&{s['track'] for s in storage}
            or any(r['minimum']>r['maximum'] for r in requires)
            or any(s['minimum']>s['maximum'] for s in storage)
            or (not value_guard and all(s.get('preserve',False) or s['minimum']<=s['set']<=s['maximum'] for s in storage))
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
