import importlib.util,json,sys,tempfile
import time
started=time.perf_counter()
from copy import deepcopy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/native-gameplay'))
import build_full_spell_artifact as build

def load(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
import srv7_seasonal_corpse_loot as corpse
import srv7_acquisition_loot_facts as acq
actual=build.item_facts
with tempfile.TemporaryDirectory(prefix='srv7-acq-current-source-') as path:
 definitions=Path(path)
 for p in build.ITEM_DEFINITIONS.glob('items-*.json'):(definitions/p.name).symlink_to(p)
 native=json.loads((build.ITEM_DEFINITIONS/'spell-native-profiles.json').read_bytes());rows={r['production_definition']['production_key']:r for r in native['records']}
 for p in [ROOT/'tools/content-schema/native-gameplay/seasonal-corpse-item-profiles.json',ROOT/'tools/content-schema/native-gameplay/acquisition-loot-item-profiles.json']:
  for r in json.loads(Path(p).read_bytes())['records']:rows[r['production_definition']['production_key']]=r
 native['records']=list(rows.values());(definitions/'spell-native-profiles.json').write_text(json.dumps(native))
 keys={'oteryn:creature.'+n for n in ('bear','chicken','minotaur','rotworm','wasp','wolf','hellspawn')}
 creatures=json.loads((build.CREATURE_DEFINITIONS/'spell-native-profiles.json').read_bytes());creatures['records']=[r for r in creatures['records'] if r['profile']['target']['key'] in keys]
 section=json.loads(build.loot_tables(creatures,item_definitions=definitions));facts={f['item']['key']:f for f in section['items']}
 f=actual(definitions)
 for key,capacity in corpse.CAPACITIES.items():assert (f[key]['materializable'],f[key]['container_capacity'])==(True,capacity),key
 count=0
 for table in section['tables']:
  for entry in table['entries']:
   key=entry['item']['key'];assert facts[key]['materializable'],key;count+=1
 for i in acq.STACKED:assert f[f'oteryn:item.tibia.i{i}'].get('stackable') is True,i
 # Ground-only source refuses promotion and no invented stack classification.
 for identifier in acq.PROMOTED:
  key=f'oteryn:item.tibia.i{identifier}';r=deepcopy(rows[key]);r['admission']['legal_destinations']=['Ground'];(definitions/'spell-native-profiles.json').write_text(json.dumps({'schema':native['schema'],'records':[r]}))
  one={key:{'item':r['production_binding']['target'],'materializable':False}};acq.admit(one,definitions);assert not one[key]['materializable'],key
 print(f'SOURCE_CURRENT_BUILDER_GREEN bodies=9 physical_hellspawn_capacity=44 tables=7 operands={count} promoted=14 native_stack_facts=19 ground_only_negatives=14 elapsed={time.perf_counter()-started:.3f}s')
