import sys
from pathlib import Path
sys.path.insert(0, str(Path('tools/content-migration').resolve()))
import world_project_v2_to_tree as tool
reference=tool.load(tool.LEGACY/'definitions'/'reference.json')
declarations=tool.load(tool.LEGACY/'definitions'/'declarations.json')
editor=tool.load(tool.LEGACY/'editor'/'author.json')
sources=tool.load(tool.LEGACY/'provenance'/'sources.json')
authoring={tool.target_id(row['item']):row for row in declarations.get('item_authoring',[])}
editors={tool.target_id(row['target']):row for row in editor['entries']}
bindings={}
for row in sources['source_identity_bindings']:
 bindings.setdefault(tool.target_id(row['target']),[]).append(row)
for rows in bindings.values(): rows.sort(key=tool.canonical_bytes)
items=[row for row in reference['records'] if row['identity']['family']=='Item']
from quest_reward_item_semantics import apply_admissions
apply_admissions(items,tool.ROOT)
tool.apply_summer_item_admissions(items)
tool.apply_primal_item_admissions(items)
from amber_item_semantics import apply_container_fact
apply_container_fact(items,tool.ROOT)
from enchanting_item_profiles import apply_admissions as enchanting
enchanting(items)
for start in range(0,len(items),tool.ITEM_SHARD_SIZE):
 rows=[]
 for definition in items[start:start+tool.ITEM_SHARD_SIZE]:
  key=tool.target_id(definition['identity']);row={'definition':definition}
  if key in editors:row['editor']=editors[key]
  if key in bindings:row['source_bindings']=bindings[key]
  extra={k:v for k,v in authoring.get(key,{}).items() if k not in {'item','taxonomy'}}
  if extra:row['authoring']=extra
  rows.append(row)
 end=start+len(rows)-1
 tool.write(f'content/items/definitions/items-{start:05d}-{end:05d}.json',{'schema':'OTERYN_ITEM_AUTHORING_SHARD/v1','family':'Item','source_legacy_role':'content/world/definitions/reference.json','shard':{'index':start//tool.ITEM_SHARD_SIZE,'start':start,'end':end,'count':len(rows)},'records':rows})
print(f'Generated {len(items)} canonical Items using existing source materializers')
