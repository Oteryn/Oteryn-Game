"""M26: source-backed native Item closure for captured endgame rewards and corpses."""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from grizzly_shop_profiles import accepted_first,known_conflict

REVISION='ff7ede593c69d4c658b382c97443e8155926924a'
PATH='tools/content-schema/native-gameplay/m26-endgame-item-profiles.json'
CAPTURE_SHA256='8f43529e14f808791e9d84cff2ad4488922e4c38e8ad05878f65b228eaa75326'
PROFILE='content/items/definitions/spell-native-profiles.json'
BOSSES=('oteryn:creature.goshnar_s_spite',)


def canonical(value):
    return (json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode()


def required_ids(root,xml):
    candidates=json.loads((root/'content/encounters/bosses/crystal-lever-candidates.json').read_bytes())
    captured={row['creature_key'] for row in candidates['records'] if row['path']=='data-global/lib/quests/soul_war.lua'
              or '/rotten_blood_quest/rotten_' in row['path']}
    bosses=set(BOSSES)
    if not bosses<=captured:
        raise ValueError('M26_ITEM_BOSS_SET')
    profiles=json.loads((root/'content/creatures/definitions/spell-native-profiles.json').read_bytes())
    selected={r['profile']['target']['key']:r for r in profiles['records'] if r['profile']['target']['key'] in bosses}
    if selected.keys()!=bosses:
        raise ValueError('M26_ITEM_CREATURE_CLOSURE')
    ids={34111,33876}
    for row in selected.values():
        key=row['profile']['data']['profile']['details']['corpse_item']['key']
        ids.add(int(key.rsplit('.i',1)[1]))
    expected_loot={'oteryn:loot.creature.'+key.removeprefix('oteryn:creature.') for key in bosses}
    found=set()
    for path in sorted((root/'content/loot').glob('loot-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            table=row['definition']
            if table['identity']['key'] not in expected_loot:
                continue
            if table['identity']['key'] in found:
                raise ValueError('M26_DUPLICATE_LOOT')
            found.add(table['identity']['key'])
            ids.update(int(entry['item']['key'].rsplit('.i',1)[1]) for entry in table['entries'])
    if found!=expected_loot:
        raise ValueError('M26_MISSING_LOOT:'+str(sorted(expected_loot-found)))
    # Include every literal decay descendant, stopping at the source's terminal0.
    for _ in range(32):
        more={int(source.attrs(xml[item]).get('decayto','0')) for item in ids if item in xml}-{0}
        if more<=ids:
            return sorted(ids),sorted(bosses)
        ids.update(more)
    raise ValueError('M26_ITEM_DECAY_CLOSURE_TOO_DEEP')


def capture(crystal,root=ROOT):
    pins=source.engine_items.ENGINE_ARTIFACT_DIGESTS[source.engine_items.CRYSTAL_PROFILE]
    raw={path:(crystal/path).read_bytes() for path in ('data/items/items.xml','data/items/appearances.dat')}
    if any(hashlib.sha256(value).hexdigest()!=pins[path] for path,value in raw.items()):
        raise ValueError('M26_ITEM_SOURCE_HASH')
    xml=source.xml_index(raw['data/items/items.xml'])
    needed,bosses=required_ids(root,xml)
    active={r['production_definition']['production_key'] for r in json.loads((root/PROFILE).read_bytes())['records']}
    previous=set(load_capture(root)['item_ids']) if (root/PATH).exists() else set()
    ids=sorted(({item for item in needed if f'oteryn:item.tibia.i{item}' not in active}|previous)&set(needed))
    objects=source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    physical={r['server_item_id']:r for r in source.policies(raw['data/items/appearances.dat'],raw['data/items/items.xml'],set(ids))}
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    definitions={r['definition']['identity']['key']:r['definition'] for p in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(p.read_bytes())['records']}
    records=[]
    for item in ids:
        key=f'oteryn:item.tibia.i{item}'
        binding=bindings.get(str(item))
        expected={'family':'Item','key':key,'revision':'definition-r1'}
        if (binding is None or binding['target']!=expected or binding['source_revision']!=REVISION
                or binding['source_key']!='oteryn:source.crystalserver' or binding['disposition']!='EXACT'):
            raise ValueError('M26_ITEM_BINDING:'+str(item))
        definition=definitions.get(key)
        if definition is None or definition['identity']!=expected or item not in objects:
            raise ValueError('M26_ITEM_CANONICAL_IDENTITY:'+str(item))
        row=source.profile(item,xml.get(item),objects[item],physical[item],bindings,None)
        if not row['admission']['materializable'] or 'Ground' not in row['admission']['legal_destinations']:
            raise ValueError('M26_ITEM_CUSTODY:'+str(item))
        accepted=definition.get('semantics',{})
        for group in ('physical','stack','container','temporal'):
            if group in accepted and known_conflict(accepted[group],row['semantics'][group]):
                raise ValueError('M26_ITEM_ACCEPTED_CONFLICT:'+str(item)+':'+group)
        row['semantics']=accepted_first(accepted,row['semantics'])
        row.update(production_binding=copy.deepcopy(binding),production_binding_qualification=None,
                   production_definition={'family':'Item','production_key':key,'revision_ref':'definition-r1'})
        records.append(row)
    return {'schema':'OTERYN_M26_ENDGAME_ITEM_PROFILES/v1','source_revision':REVISION,
            'sources':[{'path':p,'sha256':pins[p]} for p in raw], 'bosses':bosses,
            'required_item_ids':needed,'item_ids':ids,'records':records}


def load_capture(root=ROOT):
    raw=(root/PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest()!=CAPTURE_SHA256:
        raise ValueError('M26_ITEM_CAPTURE_DRIFT')
    data=json.loads(raw)
    if (data['schema']!='OTERYN_M26_ENDGAME_ITEM_PROFILES/v1' or data['source_revision']!=REVISION
            or data['item_ids']!=sorted(set(data['item_ids'])) or data['bosses']!=sorted(BOSSES)
            or [int(r['production_binding']['external_id']) for r in data['records']]!=data['item_ids']):
        raise ValueError('M26_ITEM_CAPTURE_SHAPE')
    return data


def admit(root,generated,profile_path,manifest_path,order,encode):
    overlay=load_capture(root)
    document=json.loads(generated[profile_path])
    present={r['production_definition']['production_key']:r for r in document['records']}
    for row in overlay['records']:
        key=row['production_definition']['production_key']
        if key in present:
            current=present[key]
            if current['production_binding']!=row['production_binding'] or current['admission']!=row['admission']:
                raise ValueError('M26_ITEM_EXISTING_CUSTODY_CONFLICT:'+key)
            for group in ('physical','stack','container','temporal'):
                if known_conflict(current['semantics'][group],row['semantics'][group]):
                    raise ValueError('M26_ITEM_EXISTING_SEMANTIC_CONFLICT:'+key+':'+group)
            current['semantics']=accepted_first(current['semantics'],row['semantics'])
        else:
            document['records'].append(copy.deepcopy(row))
    document['records'].sort(key=order)
    raw=encode(document)
    generated[profile_path]=raw
    manifest=json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256']=hashlib.sha256(raw).hexdigest()
    generated[manifest_path]=encode(manifest)


COHORT_LEDGER='crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'


def cohort_receipt(text,capture):
    records=capture['records']
    if (len(records)!=len(capture['item_ids'])
            or any(row['attributes']['speed_bonus']!=0 for row in records)):
        raise ValueError('M26_COHORT_SOURCE_SHAPE')
    match=re.search(r'(const PROFILE_ADDITIONS: &\[\(&str, usize\)\] = &\[\n)(.*?)(^\];)',text,re.M|re.S)
    if match is None:
        raise ValueError('M26_COHORT_LEDGER_SHAPE')
    body=match[2]
    entries=list(re.finditer(r'^    \("([^"\n]+)", ([0-9]+)\),\n',body,re.M))
    names=[entry[1] for entry in entries]
    if len(names)!=len(set(names)):
        raise ValueError('M26_COHORT_DUPLICATE')
    marker='    // M26-ENDGAME: generated by m26_endgame_item_profiles.py --refresh-cohort.\n'
    evidence='    // Crystal ff7ede59 Items '+','.join(map(str,capture['item_ids']))+'; source speed_bonus=0.\n'
    block=marker+evidence+'    ("M26-ENDGAME", '+str(len(records))+'),\n'
    if 'M26-ENDGAME' in names:
        pattern=re.escape(marker)+r'    // [^\n]+\n    \("M26-ENDGAME", [0-9]+\),\n'
        if len(re.findall(pattern,body))!=1:
            raise ValueError('M26_COHORT_RECEIPT_OWNERSHIP')
        body=re.sub(pattern,lambda _:block,body,count=1)
    else:
        successor=next((index for index,entry in enumerate(entries) if entry[1]>'M26-ENDGAME'),len(entries))
        offset=entries[successor-1].end() if successor else 0
        body=body[:offset]+block+body[offset:]
    return text[:match.start(2)]+body+text[match.end(2):]


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal',type=Path)
    parser.add_argument('--refresh-cohort',action='store_true')
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    if args.refresh_cohort:
        path=ROOT/COHORT_LEDGER
        expected=cohort_receipt(path.read_text(),load_capture())
        if args.check:
            if path.read_text()!=expected:raise SystemExit('M26_COHORT_RECEIPT_DRIFT')
        else:path.write_text(expected)
        print('M26 source cohort receipt verified' if args.check else 'M26 source cohort receipt regenerated')
        raise SystemExit(0)
    if args.crystal is None:parser.error('--crystal is required for capture')
    raw=canonical(capture(args.crystal))
    sha=hashlib.sha256(raw).hexdigest()
    if args.check:
        if (ROOT/PATH).read_bytes()!=raw or sha!=CAPTURE_SHA256:
            raise SystemExit('M26_ITEM_CAPTURE_DRIFT')
    else:
        (ROOT/PATH).write_bytes(raw)
        this=Path(__file__)
        this.write_text(re.sub(r"(?m)^CAPTURE_SHA256\s*=\s*'[^']+'$",
                              "CAPTURE_SHA256='"+sha+"'",this.read_text(),count=1))
        Path(importlib.util.cache_from_source(str(this))).unlink(missing_ok=True)
    print(json.dumps({'new_profiles':len(json.loads(raw)['records']),'needed_items':len(json.loads(raw)['required_item_ids']),'sha256':sha}))
