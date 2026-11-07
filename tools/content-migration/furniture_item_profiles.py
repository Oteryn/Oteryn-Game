"""Capture SRV7 furniture rules from the pinned Crystal source, without fetching.

The input directory contains the read-only raw files at PIN. Existing identity
bindings are reused; this generator never invents an item or collision policy.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
PROFILE = 'tools/content-schema/native-gameplay/furniture-item-profiles.json'
RULES = 'tools/content-schema/native-gameplay/furniture-rules.json'
SOURCES = ('data/items/items.xml', 'data/items/appearances.dat',
           'data-crystal/scripts/actions/other/construction_kits.lua',
           'data-global/scripts/actions/other/kits.lua',
           'data/scripts/actions/items/decay_to.lua')


def pairs(raw):
    return [(int(a), int(b)) for a, b in re.findall(r'\[(\d+)\]\s*=\s*(\d+)', raw.decode())]


def capture(donor, canary=None):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
    raw = {p: (donor / p).read_bytes() for p in SOURCES}
    import furniture_side_actions
    sides=furniture_side_actions.capture(donor)
    xml = source.xml_index(raw[SOURCES[0]])
    attributes = {i: source.attrs(e) for i, e in xml.items()}
    objects = source.engine_items.load_appearance_objects(raw[SOURCES[1]])
    wrapping = []
    for e in ET.fromstring(raw[SOURCES[0]]):
        attrs = source.attrs(e)
        if 'wrapableto' in attrs:
            wrapping.append([int(e.get('id', e.get('fromid'))),
                             int(e.get('id', e.get('toid'))), int(attrs['wrapableto'])])
    # Crystal items.cpp237-242 seeds23398 from official wrap/unwrap flags;
    # explicit XML ranges loaded afterwards retain their source override.
    covered={i for lo,hi,_ in wrapping for i in range(lo,hi+1)}
    wrapping += [[i,i,23398] for i,obj in sorted(objects.items()) if i!=23398 and i in xml and i not in covered
                 and (obj['flags'].get('flags.wrap') or obj['flags'].get('flags.unwrap'))]
    wrapping.sort()
    # All three festive-table states share retained Wiki page96628/rev1020057.
    # 35027 has no object in either pinned appearance bank; the two other states
    # use generic wrapper23398. Keep30231 as the receipt's exact unwrap target.
    if [30231, 30231, 35027] not in wrapping:
        raise ValueError('FESTIVE_TABLE_SOURCE_DRIFT')
    wrapping[wrapping.index([30231, 30231, 35027])] = [30231, 30231, 23398]
    # Modern Game::wrapItem always emits ITEM_DECORATION_KIT23398.
    # XML wrapableto qualifies wrapping, but does not select its output.
    # Crystal201189 src/game/game.cpp4899-4901; Canary47dfd game.cpp5485.
    # Keep legacy destination profiles admitted for existing items/receipts.
    legacy_wrap_profiles = {target for _, _, target in wrapping}
    wrapping = [[first, last, 23398] for first, last, _ in wrapping]
    construction = sorted(pairs(raw[SOURCES[2]]) + pairs(raw[SOURCES[3]]))
    # Native transformOnUse participates even when no Lua action overrides it.
    # The fixed floor trapdoor and podium state retain their dedicated owners.
    use = {i: int(a['transformonuse']) for i, a in attributes.items()
           if 'transformonuse' in a and a.get('type') not in ('door', 'bed')
           and i not in (3481, 3482, 8708, 35973, 35974)}
    use.update({i: target for i, target in pairs(raw[SOURCES[4]])
                if attributes.get(i, {}).get('type') not in ('door', 'bed') and i not in (3481,3482)})
    golden_path='data-global/scripts/actions/object/golden_outfit_display.lua'
    golden_raw=(donor/golden_path).read_bytes()
    golden=pairs(golden_raw)
    if len(golden)!=16 or {i for i,_ in golden}!={31500,*range(31502,31517)}:
        raise ValueError('GOLDEN_DISPLAY_SOURCE_DRIFT')
    use.update(golden)
    sides['guarded_use']=sorted(set(sides['guarded_use'])|{i for i,_ in golden})
    sides['sources'].append({'path':golden_path,'sha256':hashlib.sha256(golden_raw).hexdigest()})
    normal_shop=[]
    for record in json.loads((ROOT/'content/services/trade/trade-00000-00323.json').read_bytes())['records']:
        service=record['declaration']
        npc=service['identity']['key'].replace('oteryn:service.trade.','oteryn:npc.')
        for offer in service.get('offers',[]):
            item=offer['item']; number=int(item['key'].split('.i')[-1])
            flags=objects.get(number,{}).get('flags',{})
            if (offer['direction']=='SellToPlayer' and offer['unit_price']>0
                and (flags.get('flags.wrap') or flags.get('flags.unwrap')
                     or int(attributes.get(number,{}).get('wrapableto','0'))>0)):
                normal_shop.append((npc,number,offer['unit_price']))
    normal_shop=[{'source_row':1_000_000+at,'npc_key':npc,'item_id':number,'price':price}
                 for at,(npc,number,price) in enumerate(sorted(set(normal_shop)))]
    use.update(sides['extra_use'])
    portable_use=sorted([[a,b] for a,b in use.items() if a not in sides['guarded_use']
        and all(objects.get(i,{}).get('flags',{}).get('flags.take') for i in (a,b))])
    portable_side=sorted(r['id'] for r in sides['side_actions'] if objects.get(r['id'],{}).get('flags',{}).get('flags.take'))
    voice_block=raw[SOURCES[4]].decode().split('local transformItems',1)[0]
    use_speech=sorted([[int(i),json.loads(text)] for i,text in re.findall(r'\[(\d+)\]\s*=\s*("(?:[^"\\]|\\.)*")',voice_block)])
    if len(portable_use)!=80 or len(portable_side)!=15 or len(use_speech)!=2:
        raise ValueError('PORTABLE_FURNITURE_SOURCE_DRIFT')
    carpet_placement=[[a,b] for a,b in sides['carpets'] if objects[a]['flags'].get('flags.take') and not objects[b]['flags'].get('flags.take')]
    rotate = {i: int(a['rotateto']) for i, a in attributes.items()
              if 'rotateto' in a and a.get('type') not in ('bed', 'mailbox', 'dummy')}
    # Charged and inactive orientations have distinct retained Wiki identities.
    # XML9039/9040 crosses those families; rotation must never add/remove a charge.
    if rotate.get(9039)!=9035 or rotate.get(9040)!=9034: raise ValueError('DRACOYLE_SOURCE_ROTATE_DRIFT')
    rotate[9039]=9034;rotate[9040]=9035
    # Current independent Wiki articles have exactly two orientation IDs each.
    # The official appearance Rotate flag exists; both donors omit these targets.
    easel_pairs=[(a,a+1) for a in (14687,14689,14691,14693,14695,14697)]
    for a,b in easel_pairs:
        if a in rotate or b in rotate or not all(objects[i]['flags'].get('flags.rotate') for i in (a,b)):
            raise ValueError('ARTIST_EASEL_SOURCE_DRIFT')
        rotate[a]=b;rotate[b]=a
    # W18 actual retailer floors: Fayla's ornate carpet and Hofech's red
    # carpet are existing B3 fixtures, not new player-acquisition items.
    # Crystal201189f8 data/items/items.xml:5336-5338,35502-35504;
    # the pinned official flags independently prove nonblocking/nonpickup.
    # S06 actual56 retail grant NPCs: source center fixtures retain their
    # original B3 geometry. Seven missing native nonblocking profiles caused
    # the actual postpopulation realizer to hold11 original placements.
    # Gnomilly support center15846 is likewise source-nonblocking.
    # All ten fixtures are nonpickupable; no acquisition is introduced.
    npc_floor_physics = {2355, 2576, 2579, 2584, 5581, 8427, 15846, 15854, 17396, 21935}
    ids = {i for lo, hi, target in wrapping for i in range(lo, hi + 1)}
    ids.update(npc_floor_physics)
    ids.update(legacy_wrap_profiles)
    ids.update(target for _, _, target in wrapping)
    ids.update(i for row in construction for i in row)
    ids.update(i for row in list(use.items()) + list(rotate.items()) for i in row)
    ids.update(rule['id'] for rule in sides['side_actions'])
    ids.update(rule['output'] for rule in sides['side_actions'] if rule['output'] is not None)
    # A timed source's decay target must also have an explicit native policy.
    while True:
        extra = {int(attributes.get(i, {}).get('decayto', '0')) for i in ids} - {0} - ids
        if not extra:
            break
        ids.update(extra)
    bindings = binding_index(BINDINGS.read_bytes())
    objects = source.engine_items.load_appearance_objects(raw[SOURCES[1]])
    canary = canary or donor / 'canary'
    canary_bank = (canary / 'data/items/appearances.dat').read_bytes()
    other_objects = source.engine_items.load_appearance_objects(canary_bank)
    if 35027 in objects or 35027 in other_objects:
        raise ValueError('FESTIVE_TABLE_RETIRED_TARGET_REQUIRES_REVIEW')
    wiki = json.loads((ROOT / 'imports/tibiawiki/facts/items-stats.json').read_bytes())['records']
    for item in (30229, 30230, 30231):
        observation, = wiki[f'oteryn:item.tibia.i{item}']['observations']
        if (observation['page_id'], observation['revision_id'], observation['content_sha256']) != (
                96628, 1020057, '686a45779cd467f4f85791f2eb3c6bb2f356d222896d1328fe570139877323a3'):
            raise ValueError('FESTIVE_TABLE_STRUCTURED_SOURCE_DRIFT')

    unavailable = sorted(ids - objects.keys())
    ids.intersection_update(objects)
    physical = {r['server_item_id']: r for r in source.policies(raw[SOURCES[1]], raw[SOURCES[0]], ids)}
    records = []
    for i in sorted(ids):
        node = xml.get(i)
        signed_weight = int(attributes.get(i, {}).get('weight', '0')) < 0
        if signed_weight:
            # Christmas-tree XML declares -100. Preserve that unsupported signed
            # weight as UNKNOWN rather than inventing an unsigned weight.
            node = copy.deepcopy(node)
            for child in list(node):
                if child.get('key', '').lower() == 'weight':
                    node.remove(child)
        record = source.profile(i, node, objects[i], physical[i], bindings, None)
        if i in npc_floor_physics:
            if record['attributes']['blocks_movement'] is not False or record['semantics']['physical']['value']['pickupable'] != source.K(False):
                raise ValueError('FURNITURE_NPC_FLOOR_PHYSICS_SOURCE_DRIFT')
            record['admission']['materializable'] = False
            record['admission']['legal_destinations'] = ['Ground']
        if signed_weight:
            record['semantics']['physical']['value']['weight'] = source.U
        records.append(record)
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                          bindings, ids, independent_crystal_source=(PIN, raw[SOURCES[0]]))
    evidence = [{'path': p, 'sha256': hashlib.sha256(raw[p]).hexdigest()} for p in SOURCES]
    evidence+=sides['sources']
    evidence.append({'reference':'current-world-source-trade','path':'content/services/trade/trade-00000-00323.json','sha256':hashlib.sha256((ROOT/'content/services/trade/trade-00000-00323.json').read_bytes()).hexdigest()})
    evidence.append({'reference': 'canary-47dfd51f', 'path': 'data/items/appearances.dat',
                     'sha256': hashlib.sha256(canary_bank).hexdigest()})
    # Official native appearance flags are the current-client affordance source.
    # Keep complete Item profiles for other owners while refusing legacy XML-only rotations.
    nonretail_rotate=[{'item_id':i,'donor_target':t,'official_rotate':False,
                      'reason':'Pinned official appearance has no Rotate affordance; XML-only behavior is not admitted.'}
                     for i,t in sorted(rotate.items()) if not objects[i]['flags'].get('flags.rotate')]
    rotate={i:t for i,t in rotate.items() if objects[i]['flags'].get('flags.rotate')}
    if len(nonretail_rotate)!=18: raise ValueError('NONRETAIL_ROTATE_SOURCE_DRIFT')
    rules = {'schema': 'OTERYN_FURNITURE_RULES/v1', 'source_revision': PIN,
             'sources': evidence, 'unavailable_appearance_ids': unavailable,
             'accepted_overrides': [{'item_id': 30231, 'field': 'wrapableto', 'donor_value': 35027,
                 'qualified_value': 23398, 'evidence': 'imports/tibiawiki/facts/items-stats.json',
                 'page_id': 96628, 'revision_id': 1020057,
                 'content_sha256': '686a45779cd467f4f85791f2eb3c6bb2f356d222896d1328fe570139877323a3',
                 'reason': 'Same Festive Table identity as30229/30230; absent35027 in both donor banks; use their23398 wrapper.'}],
             'wrap': wrapping, 'construction': construction, 'golden_display':sorted(golden),
             'accepted_rotation_overrides':[{'item_id':9039,'donor_value':9035,'qualified_value':9034},{'item_id':9040,'donor_value':9034,'qualified_value':9035}],
             'artist_easel_correction': {'evidence':'docs/architecture/reviews/SRV7-W18-ARTIST-EASEL-SOURCE-2026-11-06.json','pairs':easel_pairs,'source_revision':'31748fd6abc1a18378ecade240c805a7aab6bf20'},
             'npc_wrapped_offers': normal_shop, 'nonretail_rotate':nonretail_rotate,
             'side_actions':sides['side_actions'],'side_pairs':sides['side_pairs'],
             'carpets':sides['carpets'],'carpet_placement':carpet_placement,'guarded_use':sides['guarded_use'],
             'excluded_non_furniture_music':sides['excluded_non_furniture_music'],
             'portable_use':portable_use,'portable_side':portable_side,'use_speech':use_speech,
             'use': sorted([i, t] for i, t in use.items()),
             'rotate': sorted([i, t] for i, t in rotate.items()),
             'capacities': [[int(r['production_definition']['production_key'].split('.i')[-1]),
                             r['semantics']['container']['value']['capacity']['value']]
                            for r in bound['records'] if r['semantics']['container']['state'] == 'KNOWN'],
             'lifetimes': [[i, int(attributes[i]['duration']) * 1000 if 'duration' in attributes[i] else None,
                            int(attributes[i].get('decayto', '0')) or None,
                            attributes[i].get('stopduration', '0') in ('1', 'true')]
                           for i in sorted(ids) if 'duration' in attributes.get(i, {})
                           or attributes.get(i, {}).get('stopduration', '0') in ('1', 'true')]}
    profiles = {'schema': 'OTERYN_NATIVE_FURNITURE_ITEM_PROFILES/v1',
                'source_revision': PIN, 'sources': evidence, 'records': bound['records']}
    return rules, profiles


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PROFILE).read_bytes())
    if overlay['schema'] != 'OTERYN_NATIVE_FURNITURE_ITEM_PROFILES/v1' or overlay['source_revision'] != PIN:
        raise ValueError('FURNITURE_PROFILE_SOURCE')
    document = json.loads(generated[profile_path])
    captured = {r['production_definition']['production_key']: r for r in overlay['records']}
    # Several previously admitted quest rewards have only weight/stack facts.
    # Their source-qualified collision facts are required when a decoration or
    # its decay target appears in the House map; preserve every known owner fact.
    for record in document['records']:
        source = captured.get(record['production_definition']['production_key'])
        if source is not None:
            # The shared Wiki page once applied the active wand's50-minute
            # duration to off3046 while leaving all owner fields unknown.
            # Crystal6886–6890 gives off3046 no active clock; its stopped
            # remaining budget is owned separately by the furniture timer.
            legacy_off_wand={'state':'KNOWN','value':{
                'consumption_mode':{'state':'UNKNOWN'},'decay_target':{'state':'UNKNOWN'},
                'duration':{'state':'KNOWN','value':3000000},'stop_duration':{'state':'UNKNOWN'}}}
            if record['production_definition']['production_key']=='oteryn:item.tibia.i3046' and record['semantics']['temporal']==legacy_off_wand:
                if source['semantics']['temporal']!={'state':'NOT_APPLICABLE'}:
                    raise ValueError('FURNITURE_OFF_WAND_SOURCE_DRIFT')
                record['semantics']['temporal']=copy.deepcopy(source['semantics']['temporal'])
            for field in ('temporal','container'):
                if record['semantics'][field]['state']=='UNKNOWN' and source['semantics'][field]['state']!='UNKNOWN':
                    record['semantics'][field]=source['semantics'][field]
            for field in ('blocks_movement', 'blocks_projectile', 'has_height', 'immovable_block_solid'):
                if record['attributes'].get(field) is None:
                    record['attributes'][field] = source['attributes'][field]
    existing = {r['production_definition']['production_key'] for r in document['records']}
    document['records'] = sorted(document['records'] + [r for r in overlay['records']
        if r['production_definition']['production_key'] not in existing], key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def generated_catalogues(rules):
    migration = (ROOT / 'tools/content-schema/native-gameplay/furniture-migration.sql').read_text()
    rows = [[1, *row] for row in rules['wrap']]
    rows += [[2, a, a, b] for a, b in rules['construction']]
    rows += [[3, a, a, b] for a, b in rules['use']]
    rows += [[4, a, a, b] for a, b in rules['rotate']]
    rows += [[3,a,a,b] for a,b in rules['side_pairs']]
    rows += [[2,a,a,b] for a,b in rules['carpet_placement']]
    sql = migration.replace('__RULE_ROWS__', ',\n'.join(' (' + ','.join(map(str, row)) + ')' for row in rows))
    lifetime = (ROOT / 'tools/content-schema/native-gameplay/furniture-lifetimes.sql').read_text()
    values = ',\n'.join(' (' + ','.join('NULL' if v is None else str(v).lower() for v in row) + ')'
                       for row in rules['lifetimes'])
    sql += '\n' + lifetime.replace('__LIFETIME_ROWS__', values)
    sql += '\n' + (ROOT / 'tools/content-schema/native-gameplay/furniture-expiry.sql').read_text()
    capacities = (ROOT / 'tools/content-schema/native-gameplay/furniture-containers.sql').read_text()
    values = ',\n'.join(f' ({i},{capacity})' for i, capacity in rules['capacities'])
    sql += '\n' + capacities.replace('__CAPACITY_ROWS__', values)
    sql += '\n' + (ROOT / 'tools/content-schema/native-gameplay/furniture-absolute.sql').read_text()
    sql += '\n' + (ROOT / 'tools/content-schema/native-gameplay/furniture-carried-rotate.sql').read_text().replace('__PORTABLE_USE_ROWS__', ',\n'.join(' (' + ','.join(map(str,row)) + ')' for row in rules['portable_use']))
    sql += '\n' + (ROOT / 'tools/content-schema/native-gameplay/furniture-construction.sql').read_text()
    for name in ('furniture-movement-start.sql','furniture-transfer-start.sql','furniture-house-absolute-expiry.sql'):
        sql += '\n' + (ROOT / 'tools/content-schema/native-gameplay' / name).read_text()
    sql += '\n' + (ROOT/'tools/content-schema/native-gameplay/furniture-dracoyle-in-place.sql').read_text()
    sql += '\n' + (ROOT/'tools/content-schema/native-gameplay/furniture-ground-rotate.sql').read_text()
    sql += '\n' + (ROOT/'tools/content-schema/native-gameplay/furniture-channel-expiry.sql').read_text()
    guarded=(ROOT/'tools/content-schema/native-gameplay/furniture-guarded-use.sql').read_text()
    guarded_rows=',\n'.join(f'({i},{3 if i in {pair[0] for pair in rules["golden_display"]} else 2 if i==26077 else 1})' for i in rules['guarded_use'])
    sql+='\n'+guarded.replace('__GUARDED_USE_ROWS__',guarded_rows)
    sql += '\n' + (ROOT / 'tools/content-schema/native-gameplay/furniture-golden-display.sql').read_text()
    origins=(ROOT/'tools/content-schema/native-gameplay/furniture-npc-origins.sql').read_text()
    rows=',\n'.join(f"({offer['source_row']},'oteryn:item.tibia.i{offer['item_id']}',{offer['price']},0,'{offer['npc_key']}')" for offer in rules['npc_wrapped_offers'])
    sql+='\n'+origins.replace('__NPC_FURNITURE_ROWS__',rows).replace('__NPC_FURNITURE_LAST__',str(rules['npc_wrapped_offers'][-1]['source_row']))


    path = 'crates/gameplay/dispatch/src/gameplay_transport/furniture_catalog.rs'
    rust = (ROOT / path).read_text()
    rust = rust.replace('//! `data-crystal/scripts/actions/other/construction_kits.lua`.',
                        '//! `data-crystal/scripts/actions/other/construction_kits.lua` and\n//! `data-global/scripts/actions/other/kits.lua`.')
    for constant, field in [('WRAP_TRANSFORMS', 'wrap'), ('CONSTRUCTION_KITS', 'construction')]:
        body = '\n'.join('    (' + ', '.join(map(str, row)) + '),' for row in rules[field])
        rust, count = re.subn(r'(pub const ' + constant + r':[^=]+ = &\[).*?(\n\];)',
                              lambda m: m[1] + '\n' + body + m[2], rust, count=1, flags=re.S)
        if count != 1:
            raise ValueError('FURNITURE_RUST_CATALOGUE_LAYOUT')
    rust = rust.replace('assert_eq!(CONSTRUCTION_KITS.len(), 56);', 'assert_eq!(CONSTRUCTION_KITS.len(), 68);')
    rust=re.sub(r'assert_eq!\(WRAP_TRANSFORMS.len\(\), [0-9]+\);',f'assert_eq!(WRAP_TRANSFORMS.len(), {len(rules["wrap"])});',rust)
    if '//! Festive-table state30231' not in rust:
        rust = rust.replace('//! Crystal furniture catalogue', '//! Festive-table state30231 uses the accepted shared23398 wrapper (SRV7 source correction).\n//! Crystal furniture catalogue',1)
    return [('apps/game-server/migrations/0992_house_furniture_population.sql', sql.encode()),
            (path, rust.encode())]


def generated_wrap_admission(rules, profiles):
    """Closed policy population for native13-byte Wrap; no per-Item state."""
    eligible = {item for first, last, _ in rules['wrap'] for item in range(first, last + 1)}
    rows = []
    for record in profiles['records']:
        definition = record['production_definition']
        match = re.fullmatch(r'oteryn:item[.]tibia[.]i([1-9][0-9]*)', definition['production_key'])
        if not match or int(match[1]) not in eligible:
            continue
        physical = record['semantics']['physical']
        if physical.get('state') != 'KNOWN' or physical['value']['pickupable'] != {'state': 'KNOWN', 'value': True}:
            continue
        if not record['admission']['materializable'] or 'CharacterInventory' not in record['admission']['legal_destinations']:
            raise ValueError('PORTABLE_WRAP_SOURCE_ADMISSION_DRIFT')
        if definition['family'] != 'Item' or definition['revision_ref'] != 'definition-r1':
            raise ValueError('PORTABLE_WRAP_SOURCE_IDENTITY_DRIFT')
        stack = record['semantics']['stack']
        if stack.get('state') != 'KNOWN':
            raise ValueError('PORTABLE_WRAP_SOURCE_STACK_DRIFT')
        stack = stack['value']
        if stack['stackable'] == {'state': 'KNOWN', 'value': False}:
            maximum = 1
        elif stack['stackable'] == {'state': 'KNOWN', 'value': True} and stack['stack_max'].get('state') == 'KNOWN':
            maximum = stack['stack_max']['value']
            if type(maximum) is not int or not 2 <= maximum <= 100:
                raise ValueError('PORTABLE_WRAP_SOURCE_STACK_DRIFT')
        else:
            raise ValueError('PORTABLE_WRAP_SOURCE_STACK_DRIFT')
        rows.append((int(match[1]), maximum))
    rows.sort()
    if len(rows) != 88 or len({item for item, _ in rows}) != 88 or sum(maximum > 1 for _, maximum in rows) != 70:
        raise ValueError('PORTABLE_WRAP_SOURCE_ROSTER_DRIFT')
    directory = ROOT / 'tools/content-schema/native-gameplay'
    source = (directory / 'furniture-native-wrap-source.sql').read_text()
    if source.count('__PORTABLE_WRAP_ROWS__') != 1:
        raise ValueError('PORTABLE_WRAP_SOURCE_TEMPLATE_DRIFT')
    source = source.replace('__PORTABLE_WRAP_ROWS__', ',\n'.join(f'({item},{maximum})' for item, maximum in rows))
    for name in ('furniture-native-wrap-quantity.sql', 'furniture-native-wrap-writer.sql', 'furniture-floor-construction.sql'):
        source += '\n' + (directory / name).read_text()
    return [('tools/content-schema/native-gameplay/furniture-native-wrap.generated.sql', source.encode())]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--canary', type=Path, help='pinned Canary source; defaults to <crystal>/canary')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    rules, profiles = capture(args.crystal, args.canary)
    outputs = [(path, (json.dumps(doc, indent=2, ensure_ascii=False) + '\n').encode())
               for path, doc in [(RULES, rules), (PROFILE, profiles)]] + generated_catalogues(rules) + generated_wrap_admission(rules, profiles)
    for path, raw in outputs:
        target = ROOT / path
        if args.check:
            if target.read_bytes() != raw:
                raise ValueError(f'FURNITURE_SOURCE_DRIFT {path}')
        else:
            target.write_bytes(raw)
    print(f"furniture: {len(rules['wrap'])} wrap ranges, {len(rules['construction'])} kits, "
          f"{len(rules['use'])} use, {len(rules['rotate'])} rotate, {len(profiles['records'])} profiles")


if __name__ == '__main__':
    main()
