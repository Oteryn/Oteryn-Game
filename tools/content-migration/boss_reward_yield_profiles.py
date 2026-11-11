#!/usr/bin/env python3
"""SRX-3: exact source custody for required encounter yields and an explicit corpse bag.

This adds no loot entries/chances, equipment fallback, or canonical materialization.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import boss_corpse_profiles as corpses

ROOT = corpses.ROOT
PIN = corpses.PIN
PATH = 'tools/content-schema/native-gameplay/boss-reward-yield-profiles.json'
PIN_PATH = 'tools/content-schema/native-gameplay/boss-reward-yield-profiles-pin.json'
SCHEMA = 'OTERYN_NATIVE_BOSS_REWARD_YIELD_PROFILES/v1'
BINDING_PATH = 'tools/content-schema/native-gameplay/candidate-bindings/boss-reward-items-candidate.json'
RUST_PIN = 'crates/gameplay/content/src/content/native_gameplay/boss_reward_bindings_pin.rs'
EXTRA_IDS = frozenset({43860,43946,43947,44048,43778,43779,43780,43781,43782})
EXTRA_SOURCES = {
    'src/items/item.cpp':'e27e622bf4fd9a94d7fffb3a818fd0c5f911e6fbf4b92844320154ded326950b',
    'data-global/monster/quests/rotten_blood_quest/bakragore.lua':'974df9389e734ca947111cd41af9adec44f531adf0cc53233d5a7a94ab190178',
    'data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua':'21ba33eb58d1b5a4a88ca0c950d49a8369b34f3a11fe39a7e549a8b3ab154b46',
}
IDS = frozenset({3125,3130,23474,7428,23477,239,3028,3042,3081,3097,3384,3583,3723,3728,7385,7416,7642,7643,23373,23374,23375,23535,23545,33778,39037,43501,43502,43503,43504,43778,43779,43780,43781,43782,43847,43860,43946,43947,43961,43962,43966,43967,43968,44048,50150,50152})
CLASSIFICATION_ENRICHMENT_IDS = frozenset({239,3028,3081,3097,3384,7385,7642,7643,23373,23374,23375,23535})
BOSSES = ('foam_stalker','outburst','grand_mother_foulscale', 'bakragore', 'chagorz', 'ichgahal', 'murcion', 'vemiath', 'bloated_man_maggot', 'converter', 'darklight_construct', 'darklight_emitter', 'darklight_matter', 'darklight_source', 'darklight_striker', 'meandering_mushroom', 'mycobiontic_beetle', 'oozing_carcass', 'rotten_man_maggot', 'sopping_carcass', 'sopping_corpus', 'walking_pillar', 'wandering_pillar', 'goshnar_s_greed', 'goshnar_s_malice', 'monk_s_apparition', 'paladin_s_apparition', 'sorcerer_s_apparition', 'capricious_phantom', 'distorted_phantom', 'infernal_demon', 'rotten_golem', 'goshnar_s_spite', 'goshnar_s_hatred', 'goshnar_s_cruelty', 'goshnar_s_megalomania_green', 'goshnar_s_megalomania_blue', 'dreadful_disruptor')


HAM_FOOD_PATH = 'data/scripts/actions/items/foods.lua'
HAM_FOOD_SHA256 = '6023845b3f982b4a7cc9cf3e43bb8d33f467e3e10f9bdf3f0713aeae88de902f'
HAM_FOOD_PROOF = {'repository':'zimbadev/crystalserver','revision':PIN,
                  'path':HAM_FOOD_PATH,'sha256':HAM_FOOD_SHA256,'lines':[15,138,152]}


def ham_preserved_semantics(definition, appearance, food):
    """Keep actual existing Food/market semantics; custody adds no replacement use rule."""
    if (hashlib.sha256(food).hexdigest() != HAM_FOOD_SHA256
            or b'[3583] = { 60, "Chomp." }' not in food
            or b'player:feed(itemFood[1] * 12)' not in food):
        raise ValueError('BOSS_REWARD_HAM_FOOD_SOURCE')
    known=lambda value:{'state':'KNOWN','value':value}
    unknown={'state':'UNKNOWN'}
    consumption=known({'kind':'FOOD','value':{'regeneration_seconds':720}})
    trade=known({'account_binding_policy':unknown,'character_binding_policy':unknown,
                 'marketable':known(True),'tradeable':unknown,'vocations':unknown})
    flags=appearance['flags']
    if (definition['identity'] != {'family':'Item','key':'oteryn:item.tibia.i3583','revision':'definition-r1'}
            or definition['semantics']['consumption'] != consumption
            or definition['semantics']['trade_restrictions'] != trade
            or flags.get('flags.market') is not True
            or flags.get('market.trade_as_object_id') != 3583
            or flags.get('market.show_as_object_id') != 3583):
        raise ValueError('BOSS_REWARD_HAM_KNOWN_SEMANTICS')
    return {'consumption':copy.deepcopy(consumption),'trade_restrictions':copy.deepcopy(trade)}


def source_selection(root):
    selected = {}
    for path in sorted((root/'content/loot').glob('loot-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in {'oteryn:loot.creature.'+key for key in BOSSES}:
                selected[definition['identity']['key']] = definition
    if len(selected) != len(BOSSES):
        raise ValueError('BOSS_REWARD_YIELD_LOOT_SET')
    def walk(entries):
        for entry in entries:
            yield entry['item']['key']
            yield from walk(entry.get('children', []))
    references = {key for definition in selected.values() for key in walk(definition['entries'])}
    if not {'oteryn:item.tibia.i'+str(item) for item in IDS-{43860}} <= references:
        raise ValueError('BOSS_REWARD_YIELD_UNREFERENCED_ITEM')
    tables = [{'identity':value['identity'], 'sha256':hashlib.sha256(corpses.encode(value)).hexdigest()}
              for _,value in sorted(selected.items())]
    # This is source evidence, not an invented ordinary Loot-table identity.
    # The death callback adds one physical-corpse child independently of the
    # rewardBoss table's personal entitlements.
    callback='data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua'
    return tables+[{'source_callback':callback,'source_revision':PIN,
                    'sha256':EXTRA_SOURCES[callback], 'lines':[21,43],
                    'yield_ids':[43860,43895], 'destination':'PhysicalCorpseChild'}]


def binding_source_selection(root):
    return [row for row in source_selection(root)
            if row.get('identity',{}).get('key') not in {'oteryn:loot.creature.grand_mother_foulscale','oteryn:loot.creature.outburst','oteryn:loot.creature.foam_stalker'}]

def existing_targets(root):
    wanted = {'oteryn:item.tibia.i'+str(item) for item in EXTRA_IDS}
    found = {}
    for path in sorted((root/'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in wanted:
                if definition['identity']['key'] in found or definition['client_projection'] != 'ClientSafe':
                    raise ValueError('BOSS_REWARD_EXISTING_REFERENCE_AMBIGUOUS')
                found[definition['identity']['key']] = definition['identity']
    if set(found) != wanted:raise ValueError('BOSS_REWARD_MISSING_EXISTING_CANONICAL_REFERENCE')
    return found


def qualification_packet(cache, root):
    blobs = corpses.load_source_blobs(cache)
    for path, digest in EXTRA_SOURCES.items():
        raw = (cache/Path(path).name).read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:raise ValueError('BOSS_REWARD_BINDING_FACTORY_SOURCE:'+path)
        blobs[path] = raw
    if (b'iType.id = static_cast<uint16_t>(object.id());' not in blobs['src/items/items.cpp']
            or b'if (it.id != 0)' not in blobs['src/items/item.cpp']
            or b'{ id = 43946, chance = 180' not in blobs['data-global/monster/quests/rotten_blood_quest/bakragore.lua']
            or b'{ id = 43947, chance = 180' not in blobs['data-global/monster/quests/rotten_blood_quest/bakragore.lua']):
        raise ValueError('BOSS_REWARD_BINDING_FACTORY_CLOSURE')
    objects = corpses.source.engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    targets = existing_targets(root)
    if not EXTRA_IDS <= objects.keys():raise ValueError('BOSS_REWARD_MISSING_APPEARANCE')
    bindings = [{'source_key':'oteryn:source.crystalserver','source_revision':PIN,
                 'identity_namespace':'ots/item_server_id','external_id':str(item),
                 'target':targets['oteryn:item.tibia.i'+str(item)],'disposition':'EXACT'} for item in sorted(EXTRA_IDS)]
    return {'schema':'OTERYN_NATIVE_BOSS_REWARD_BINDING_EXTENSION/v1','source_revision':PIN,
            'sources':[{'repository':'zimbadev/crystalserver','revision':PIN,'path':path,'sha256':digest}
                       for path,digest in {**corpses.SOURCE_HASHES,**EXTRA_SOURCES}.items()],
            'source_tables':binding_source_selection(root),'bindings':bindings,
            'scope':'Nine exact source bindings.43860 has a separately source-qualified reference-only declaration at ordinal34111; the other eight references already existed. No alias, Item use or custody admission follows from binding alone.'}


def binding_extension(root):
    packet = json.loads((root/BINDING_PATH).read_bytes())
    if (packet['schema'] != 'OTERYN_NATIVE_BOSS_REWARD_BINDING_EXTENSION/v1'
            or packet['source_revision'] != PIN or packet['source_tables'] != binding_source_selection(root)
            or packet['sources'] != [{'repository':'zimbadev/crystalserver','revision':PIN,'path':path,'sha256':digest}
                for path,digest in {**corpses.SOURCE_HASHES,**EXTRA_SOURCES}.items()]):
        raise ValueError('BOSS_REWARD_BINDING_PACKET_AUTHORITY')
    targets = existing_targets(root)
    expected = [{'source_key':'oteryn:source.crystalserver','source_revision':PIN,
                 'identity_namespace':'ots/item_server_id','external_id':str(item),
                 'target':targets['oteryn:item.tibia.i'+str(item)],'disposition':'EXACT'} for item in sorted(EXTRA_IDS)]
    if packet['bindings'] != expected:raise ValueError('BOSS_REWARD_BINDING_PACKET_EXACT_SET')
    return {row['external_id']:row for row in expected}, packet


def capture(cache: Path, root: Path = ROOT):
    blobs = corpses.load_source_blobs(cache)
    source = corpses.source
    xml = source.xml_index(blobs['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    physics = {row['server_item_id']:row for row in source.policies(blobs['data/items/appearances.dat'],blobs['data/items/items.xml'],IDS)}
    bindings = corpses.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    packet = qualification_packet(cache,root)
    additions = {row['external_id']:row for row in packet['bindings']}
    if set(additions) & bindings.keys():raise ValueError('BOSS_REWARD_BINDING_ALREADY_PROTECTED')
    bindings.update(additions)
    # Later registered navigation enrichment preserves this admitted canonical
    # taxonomy on every native Item. Retain it in the exact expected source body
    # as well; it grants no physics, use or loot semantics.
    taxonomy = {row['target']['key']: row['source_taxonomy'] for row in
        json.loads((root/'content/items/taxonomy/items.json').read_bytes())['records']}
    food=(cache/Path(HAM_FOOD_PATH).name).read_bytes()
    ham=[row['definition'] for path in sorted((root/'content/items/definitions').glob('items-*.json'))
         for row in json.loads(path.read_bytes())['records']
         if row['definition']['identity']['key']=='oteryn:item.tibia.i3583']
    if len(ham)!=1:raise ValueError('BOSS_REWARD_HAM_CANONICAL_REFERENCE')
    preserved_ham=ham_preserved_semantics(ham[0],objects[3583],food)
    records = []
    for item in sorted(IDS):
        row = source.profile(item,xml.get(item),objects[item],physics[item],bindings,None)
        corpses.complete_source_top_order(row, objects[item])
        if item == 3583:
            row['semantics'].update(copy.deepcopy(preserved_ham))
        if item == 43860:
            # The script comment is not an authoritative item name or use rule.
            row['semantics']['presentation'] = {'state':'UNKNOWN'}
        if (not row['admission']['materializable']
                or row['admission']['legal_destinations'] != ['CharacterInventory','Ground']
                or not physics[item]['movable'] or not physics[item]['pickupable']):
            raise ValueError(f'BOSS_REWARD_YIELD_CARRIED_PHYSICS:{item}')
        binding = bindings[str(item)]
        if binding['disposition'] != 'EXACT':
            raise ValueError(f'BOSS_REWARD_YIELD_EXACT_BINDING:{item}')
        row['authoring']['item']['revision'] = 'srx3-boss-reward-yields-crystal-201189f'
        row['production_binding'] = copy.deepcopy(binding)
        row['production_binding_qualification'] = copy.deepcopy(packet) if item in EXTRA_IDS else None
        row['production_definition'] = {'family':'Item','production_key':binding['target']['key'],'revision_ref':binding['target']['revision']}
        if (item in EXTRA_IDS or item==3583) and binding['target']['key'] in taxonomy:
            row['authoring']['taxonomy'] = copy.deepcopy(taxonomy[binding['target']['key']])
        records.append(row)
    return {'schema':SCHEMA,'source_revision':PIN,'producer':'tools/content-migration/boss_reward_yield_profiles.py',
            'sources':[{'repository':'zimbadev/crystalserver','revision':PIN,'path':path,'sha256':digest} for path,digest in corpses.SOURCE_HASHES.items()],
            'yield_specific_sources':[HAM_FOOD_PROOF],'source_tables':source_selection(root),'yield_ids':sorted(IDS),'without_xml_override':sorted(IDS-set(xml)),
            'xml_source_lines':corpses.xml_source_lines(blobs['data/items/items.xml'],IDS),'records':records}


def admit(root, generated, destination, manifest_path, order, canonical):
    raw = (root/PATH).read_bytes()
    if json.loads((root/PIN_PATH).read_bytes()) != {'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()}:
        raise ValueError('BOSS_REWARD_YIELD_CAPTURE_DRIFT')
    data = json.loads(raw)
    if (data['schema'] != SCHEMA or data['source_revision'] != PIN or data['yield_ids'] != sorted(IDS)
            or data.get('yield_specific_sources') != [HAM_FOOD_PROOF]
            or data['source_tables'] != source_selection(root) or len(data['records']) != len(IDS)
            or data['sources'] != [{'repository':'zimbadev/crystalserver','revision':PIN,'path':path,'sha256':digest} for path,digest in corpses.SOURCE_HASHES.items()]):
        raise ValueError('BOSS_REWARD_YIELD_SOURCE_AUTHORITY')
    bindings = corpses.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    additions, packet = binding_extension(root)
    if set(additions) & bindings.keys():raise ValueError('BOSS_REWARD_BINDING_ALREADY_PROTECTED')
    bindings.update(additions)
    document = json.loads(generated[destination])
    existing = {row['production_definition']['production_key']:row for row in document['records']}
    from build_full_spell_artifact import native_loot_yield_facts
    seen = set()
    for row in data['records']:
        binding = row['production_binding']; item = int(binding['external_id']); key = row['production_definition']['production_key']
        if (item not in IDS or item in seen or binding != bindings.get(str(item)) or binding['disposition'] != 'EXACT'
                or row['production_definition'] != {'family':'Item','production_key':binding['target']['key'],'revision_ref':binding['target']['revision']}
                or row['production_binding_qualification'] != (packet if item in EXTRA_IDS else None)
                or not row['admission']['materializable']
                or row['admission']['legal_destinations'] != ['CharacterInventory','Ground']):
            raise ValueError(f'BOSS_REWARD_YIELD_NATIVE_AUTHORITY:{key}')
        if native_loot_yield_facts(binding['target'], row) is None:
            raise ValueError(f'BOSS_REWARD_YIELD_EXACT_CUSTODY_FACTS:{key}')
        seen.add(item)
        if key in existing:
            previous = existing[key]
            if item == 3583:
                for part in ('consumption', 'trade_restrictions'):
                    known = previous['semantics'].get(part, {'state': 'UNKNOWN'})
                    if known != {'state': 'UNKNOWN'} and known != row['semantics'][part]:
                        raise ValueError('BOSS_REWARD_HAM_KNOWN_CUSTODY_CONFLICT')
                    previous['semantics'][part] = copy.deepcopy(row['semantics'][part])
            if item in CLASSIFICATION_ENRICHMENT_IDS:
                # Only source-proved previously UNKNOWN custody fields may be
                # completed. Known equipment/rune/fluid bodies remain intact.
                def enrich(old, proven):
                    if old == {'state':'UNKNOWN'}:
                        return copy.deepcopy(proven)
                    if isinstance(old,dict) and isinstance(proven,dict) and old.keys()==proven.keys():
                        return {part:enrich(old[part],proven[part]) for part in old}
                    if old != proven:
                        raise ValueError(f'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT:{key}')
                    return copy.deepcopy(old)
                for part in ('classification','stack','container','charges','temporal'):
                    previous['semantics'][part]=enrich(previous['semantics'][part],row['semantics'][part])
            if (previous['production_definition'] != row['production_definition'] or previous['production_binding'] != binding
                    or previous['admission'] != row['admission'] or any(previous['semantics'][part] != row['semantics'][part]
                        for part in ('physical','stack','container','charges','temporal'))):
                raise ValueError(f'BOSS_REWARD_YIELD_PROFILE_CONFLICT:{key}')
        else:
            document['records'].append(row)
    document['records'].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path]); manifest['item_profiles']['sha256'] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = canonical(manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-cache',type=Path,required=True)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    import boss_conditional_item_reference
    boss_conditional_item_reference.publish(args.source_cache,ROOT,args.check)
    raw = corpses.encode(capture(args.source_cache))
    packet = corpses.encode(qualification_packet(args.source_cache,ROOT))
    pin = corpses.encode({'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()})
    rust = ('// Generated only by tools/content-migration/boss_reward_yield_profiles.py.\n'
            'pub(super) const CAPTURE_SHA256: &str =\n    "'+hashlib.sha256(raw).hexdigest()+'";\n'
            'pub(super) const PACKET_SHA256: &str =\n    "'+hashlib.sha256(packet).hexdigest()+'";\n').encode()
    for path, content in ((PATH,raw),(PIN_PATH,pin),(BINDING_PATH,packet),(RUST_PIN,rust)):
        if args.check:
            if (ROOT/path).read_bytes() != content:raise SystemExit(f'BOSS_REWARD_YIELD_GENERATED_DRIFT:{path}')
        else:(ROOT/path).write_bytes(content)
    print('BOSS_REWARD_YIELD_SOURCE_PROFILES',len(IDS),'CHECK' if args.check else 'GENERATED')

if __name__=='__main__':main()
