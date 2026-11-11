"""W02 Sugar Daddy: exact source Item custody and callback closure.

Canary47df data-otservbr-global/monster/bosses/sugar_daddy.lua:21-25,76-132
is the existing canonical native Creature's source. It has four attacks, two
native defenses, no summons and no event scripts. Crystal201189f supplies the
lever geometry and implicit BossLeverOnDeath (boss_lever.lua:241 and
scripts/creaturescripts/monster/boss_lever_death.lua:1-39).

Canary items.hpp:282,307,369 supplies weight0/container8/non-pausing defaults;
items.cpp and Crystal items.cpp:163-174 bind appearance.id to that exact ItemType.
Corpse48416 has container+corpse flags and no XML overrides. No other corpse,
identity or unknown gameplay callback is admitted by this closed capture.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import importlib.util
import json

from native_profile_composition import encode_native_profile
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from grizzly_shop_profiles import accepted_first

PATH = 'tools/content-schema/native-gameplay/boss-reward-item-profiles.json'
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
CRYSTAL = '201189f871d23597b82caf88fd611454a2ddefb8'
BOSS = 'oteryn:creature.sugar_daddy'
BODY = 48416
CANARY_HASHES = {
 'data/items/appearances.dat': 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50',
 'data/items/items.xml': '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2',
 'src/items/items.cpp': 'f695ed647e111923cf8e4d487c4a0706f19e7edbfb7896655c20494d6d927fd7',
 'src/items/items.hpp': 'b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6',
 'data-otservbr-global/monster/bosses/sugar_daddy.lua': 'fc600b57ec2ce2464e0e0d5aae0be63068ea151d02079fbfccf75abd812fb585',
}
CRYSTAL_HASHES = {
 'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
 'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
 'data/libs/functions/boss_lever.lua': '0a91d34c17c8fb2a20cdfddd3bb57df23d52ddafbc2053b03c938ee9162edd03',
 'data/scripts/creaturescripts/monster/boss_lever_death.lua': '25155bf01a3b717408842c593af196bd19011ba5012d6e23a3380c3534ffbcf7',
}
CAPTURE_SHA256 = '037cf9c089dcdd19783fad8215f15c8739b02dcdb76bff0d1459473e0f675618'


def canonical(value):
 return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False)+'\n').encode()


def body_binding():
 return {'source_key': 'oteryn:source.crystalserver', 'source_revision': CRYSTAL,
  'identity_namespace': 'ots/item_server_id', 'external_id': str(BODY),
  'target': {'family': 'Item', 'key': f'oteryn:item.tibia.i{BODY}', 'revision': 'definition-r1'},
  'disposition': 'EXACT'}


def selected(root):
 table = None
 for path in sorted((root / 'content/loot').glob('loot-*.json')):
  for row in json.loads(path.read_bytes())['records']:
   if row['definition']['identity']['key'] == 'oteryn:loot.creature.sugar_daddy':
    if table is not None: raise ValueError('BOSS_DUPLICATE_LOOT')
    table = row['definition']
 if table is None: raise ValueError('BOSS_MISSING_LOOT')
 return table, {int(entry['item']['key'].rsplit('.i',1)[1]) for entry in table['entries']} | {BODY}


def capture(canary, crystal, root=ROOT):
 captured = {path: (canary/path).read_bytes() for path in CANARY_HASHES}
 if any(hashlib.sha256(raw).hexdigest()!=CANARY_HASHES[path] for path,raw in captured.items()):
  raise ValueError('BOSS_DONOR_HASH')
 monster = captured['data-otservbr-global/monster/bosses/sugar_daddy.lua'].decode()
 if any(token in monster for token in ('monster.events', 'monster.summon', 'registerEvent')):
  raise ValueError('BOSS_UNSUPPORTED_CALLBACK')
 crystal_raw = {path:(crystal/path).read_bytes() for path in CRYSTAL_HASHES}
 if any(hashlib.sha256(raw).hexdigest()!=CRYSTAL_HASHES[path] for path,raw in crystal_raw.items()): raise ValueError('BOSS_CRYSTAL_SOURCE_HASH')
 crystal_dat = crystal_raw['data/items/appearances.dat']
 if hashlib.sha256(crystal_dat).hexdigest() != '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1':
  raise ValueError('BOSS_CRYSTAL_IDENTITY_SOURCE')
 crystal_objects = source.engine_items.load_appearance_objects(crystal_dat)
 if BODY not in crystal_objects or crystal_objects[BODY]['flags'].get('flags.container') is not True:
  raise ValueError('BOSS_CORPSE_IDENTITY')
 table, ids = selected(root)
 xml = source.xml_index(captured['data/items/items.xml'])
 objects = source.engine_items.load_appearance_objects(captured['data/items/appearances.dat'])
 physical = {r['server_item_id']:r for r in source.policies(captured['data/items/appearances.dat'],captured['data/items/items.xml'],ids)}
 bindings = source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
 if str(BODY) in bindings: raise ValueError('BOSS_CORPSE_BINDING_COLLISION')
 bindings[str(BODY)] = body_binding()
 definitions = {r['definition']['identity']['key']:r['definition'] for p in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(p.read_bytes())['records']}
 records = []
 for item in sorted(ids):
  if item not in objects or str(item) not in bindings: raise ValueError(f'BOSS_ITEM_SOURCE:{item}')
  row = source.profile(item,xml.get(item),objects[item],physical[item],bindings,None)
  key = bindings[str(item)]['target']['key']
  definition = definitions.get(key)
  if definition is None or definition['identity'] != bindings[str(item)]['target']: raise ValueError('BOSS_CANONICAL_IDENTITY')
  row['semantics'] = accepted_first(definition.get('semantics', {}),row['semantics'])
  if item == BODY:
   if item in xml or row['semantics']['container'] != source.K({'capacity':source.K(8)}) or row['admission']['legal_destinations'] != ['Ground']:
    raise ValueError('BOSS_CORPSE_POLICY')
  elif row['admission']['legal_destinations'] != ['CharacterInventory','Ground']:
   raise ValueError(f'BOSS_YIELD_CUSTODY:{item}')
  binding = bindings[str(item)]
  row.update(production_binding=copy.deepcopy(binding),production_binding_qualification=None,
   production_definition={'family':'Item','production_key':key,'revision_ref':binding['target']['revision']})
  records.append(row)
 profile, = [r for r in json.loads((root/'content/creatures/definitions/spell-native-profiles.json').read_bytes())['records'] if r['profile']['target']['key']==BOSS]
 raw_profile = json.dumps(profile,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
 return {'schema':'OTERYN_NATIVE_BOSS_REWARD_ITEM_PROFILES/v1','source_revision':CANARY,
  'sources':[{'path':p,'sha256':v} for p,v in CANARY_HASHES.items()],
  'crystal_sources':[{'path':p,'sha256':v} for p,v in CRYSTAL_HASHES.items()],
  'loot_sha256':hashlib.sha256(canonical(table)).hexdigest(),'item_ids':sorted(ids),'records':records,
  'callback_closure':{'creature_key':BOSS,'native_profile_sha256':hashlib.sha256(raw_profile).hexdigest(),
   'source_path':'data-otservbr-global/monster/bosses/sugar_daddy.lua','source_sha256':CANARY_HASHES['data-otservbr-global/monster/bosses/sugar_daddy.lua'],
   'source_events':[],'source_summons':False,'required_callbacks':['BossLeverOnDeath']}}


def admit(root,generated,profile_path,manifest_path,order,encode):
 raw=(root/PATH).read_bytes()
 if hashlib.sha256(raw).hexdigest()!=CAPTURE_SHA256: raise ValueError('BOSS_CAPTURE_DRIFT')
 overlay=json.loads(raw);table,ids=selected(root)
 if overlay['item_ids']!=sorted(ids) or overlay['loot_sha256']!=hashlib.sha256(canonical(table)).hexdigest(): raise ValueError('BOSS_LOOT_CLOSURE_DRIFT')
 document=json.loads(generated[profile_path]);present={r['production_definition']['production_key']:r for r in document['records']}
 for row in overlay['records']:
  key=row['production_definition']['production_key']
  if key in present:
   # Fill only unknown source facts; accepted values and equipment pattern
   # identities retain their existing owner's precedence.
   current=present[key]
   if current['production_binding']!=row['production_binding']: raise ValueError('BOSS_EXISTING_IDENTITY_CONFLICT')
   current['semantics']=accepted_first(current['semantics'],row['semantics'])
  else: document['records'].append(row)
 document['records'].sort(key=order)
 data=encode_native_profile(document);generated[profile_path]=data
 manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=hashlib.sha256(data).hexdigest();generated[manifest_path]=encode(manifest)


if __name__=='__main__':
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--canary',type=Path,required=True);parser.add_argument('--crystal',type=Path,required=True);parser.add_argument('--check',action='store_true');args=parser.parse_args()
 raw=canonical(capture(args.canary,args.crystal))
 if args.check:
  if (ROOT/PATH).read_bytes()!=raw or hashlib.sha256(raw).hexdigest()!=CAPTURE_SHA256: raise SystemExit('BOSS_CAPTURE_DRIFT')
 else:
  (ROOT/PATH).write_bytes(raw)
  this=Path(__file__);this.write_text(re.sub(r"(?m)^CAPTURE_SHA256 = '[0-9a-z]+'$", "CAPTURE_SHA256 = '"+hashlib.sha256(raw).hexdigest()+"'", this.read_text(), count=1))
  # A same-second regenerated digest has the same source length; invalidate
  # this generator's cached module before the immediate importer/check pass.
  Path(importlib.util.cache_from_source(str(this))).unlink(missing_ok=True)
 print(json.dumps({'records':len(json.loads(raw)['records']),'sha256':hashlib.sha256(raw).hexdigest()}))
