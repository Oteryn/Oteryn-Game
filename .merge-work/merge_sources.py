import json,re,difflib
from pathlib import Path
cs=json.loads(Path('.merge-work/allchunks.json').read_text()); overrides={};notes={}
def put(suffix,i,value):
 p=next(p for p in cs if p.endswith(suffix));a,b,c=cs[p][i];overrides[p,i]=value(a,b,c) if callable(value) else (a if value=='ours' else c if value=='theirs' else value)
def choose(suffix,indices,side):
 for i in indices:put(suffix,i,side)
def union(a,b,c):
 if not b:return a+c
 out=a
 # Apply incoming changes only where the exact base fragment remains.
 for tag,i,j,k,l in reversed(list(difflib.SequenceMatcher(None,b.splitlines(True),c.splitlines(True),autojunk=False).get_opcodes())):
  if tag=='equal':continue
  old=''.join(b.splitlines(True)[i:j]);new=''.join(c.splitlines(True)[k:l])
  if old and out.count(old)==1:out=out.replace(old,new,1)
  elif not old:
   before=''.join(b.splitlines(True)[max(0,i-2):i]);after=''.join(b.splitlines(True)[i:i+2])
   if new in out:continue
   if after and out.count(after)==1:out=out.replace(after,new+after,1)
   elif before and out.count(before)==1:out=out.replace(before,before+new,1)
 return out
for i in (0,2,3):put('examples/materialize_content_world_project_v2.rs',i,lambda a,b,c:a.replace('25 + 4 +','30 +').replace('29 + cake','30 + cake'))
put('examples/materialize_content_world_project_v2.rs',1,lambda a,b,c:a.replace('68 +','68 + 3 +'))
choose('activation_fixture.rs',[0],'ours');choose('starting_loadout_input.rs',[0],'ours');choose('npc_catalogue.rs',[0],'ours')
choose('project/bestiary.rs',[0],'ours')
choose('ability_tests.rs',[0],'theirs');put('ability_tests.rs',1,lambda a,b,c:a.replace('71','122'));choose('ability_tests.rs',[2],'ours')
put('grizzly_kills.rs',0,lambda a,b,c:a.replace('    objective_keys:', '    active_goals: Vec<String>,\n    objective_keys:',1))
choose('grizzly_kills.rs',[1,2],'theirs')
put('grizzly_kills.rs',3,lambda a,b,c:'                        active_goals: native_goals::active(copy.tracks()),\n'+a)
put('kill_reward.rs',0,lambda a,b,c:c.replace('oteryn_gameplay_quests::quest::native_kills::tracks_creature(&creature)','super::grizzly_kills::tracks_native_creature(sink.queue, &creature)'))
put('npc_command.rs',0,'mod cake;\nmod catalogue;\nmod catalogue_track_mutation;\n')
choose('quest_dialogue.rs',[0],'ours')
put('special_movement.rs',0,lambda a,b,c:c.replace(b,a))
choose('newhaven_corruptor.rs',[0],'ours')
put('qengine_access.rs',0,lambda a,b,c:a.replace('            actor_placement:', '            party: Vec::new(),\n            actor_placement:',1))
put('qengine_rewards.rs',0,lambda a,b,c:a.replace('            } else if name.starts_with("qengine-rewards-questpc4-paradox-upper-") {',c.replace('            if','            } else if',1)))
choose('rotten_blood_progression.rs',[0],'theirs');choose('soul_war.rs',[1],'ours');choose('viola_book.rs',[0],'ours')
# Keep fields from both callback constructor revisions; compiler verifies every constructor.
for suffix in ['carried_step.rs','fire_feathered_native.rs','hive_access.rs','rotten_blood.rs','schrodinger_island.rs','shared_switches.rs','shared_world_progress.rs']:
 p=next(p for p in cs if p.endswith(suffix))
 for i,(a,b,c) in enumerate(cs[p]):
  if 'position_aliases:' in c: put(suffix,i,lambda a,b,c:a.replace('Selector {','Selector { position_aliases: Vec::new(),').replace('Selector{','Selector{ position_aliases: Vec::new(),'))
  elif 'set_if: None' in c:put(suffix,i,'theirs')
  elif 'Callback' in c and 'failure_teleport:' in c:put(suffix,i,lambda a,b,c:a.replace('Callback {','Callback { failure_teleport: None,').replace('Callback{','Callback{ failure_teleport: None,'))
choose('quest/mod.rs',[2],'ours')
put('quest/mod.rs',3,lambda a,b,c:a+c.replace(b,'') if b in c else union(a,b,c))
choose('npc_rules.rs',[1],'ours')
for i in [0,1,2,3,4,5]:
 if i<len(cs[next(p for p in cs if p.endswith('npc_rules/tests.rs'))]):choose('npc_rules/tests.rs',[i],'ours')
put('npc_rules/validate.rs',0,lambda a,b,c:c.replace(b,a))
# Registry composition and independent native goal owners precede world lowering.
put('quest/loader.rs',0,lambda a,b,c:c.replace(b,a))
put('quest/loader.rs',1,lambda a,b,c:a+c)
choose('quest/loader.rs',[2],'ours')
choose('quest/reward_callbacks.rs',[0,1,2],'ours')
put('quest/reward_callbacks.rs',3,lambda a,b,c:c+a)
put('quest/reward_callbacks.rs',4,lambda a,b,c:c+a)
# World callback definition fields: completes already belongs to main.
put('quest/world_callbacks.rs',0,lambda a,b,c:c.replace('    #[serde(default, skip_serializing_if = "is_false")]\n    pub completes: bool,\n',''))
put('quest/world_callbacks.rs',1,lambda a,b,c:a.split('    callbacks.retain')[0]+c)
put('quest/world_callbacks.rs',2,lambda a,b,c:a+'        let alias_positions: BTreeSet<_> = selector.position_aliases.iter().copied().collect();\n')
put('quest/world_callbacks.rs',3,lambda a,b,c:a+c)
put('quest/world_callbacks.rs',4,lambda a,b,c:c.replace(b,a))
choose('quest/world_callbacks.rs',[5],'theirs')
put('quest/world_callbacks.rs',6,lambda a,b,c:c.replace('storage.preserve)))','storage.preserve || storage.increment || storage.at_least)))'))
put('quest/world_callbacks.rs',7,lambda a,b,c:'                (authored && source.starts_with("oteryn-authored:")) ||\n'+c)
choose('quest/world_callbacks.rs',[8,9],'theirs')
# Generator steps are independent and all must survive.
for i in range(4):put('regenerate_content.py',i,lambda a,b,c:a+c)
for i in [0,1]:put('register_spell_families.py',i,lambda a,b,c:a+c)
# Retain each branch's tests rather than choosing one test body.
for p,rows in cs.items():
 if p.endswith('.py') and '/test_' in p:
  for i,(a,b,c) in enumerate(rows):
   if b is None and a and c and ('def test_' in a or 'class ' in a) and ('def test_' in c or 'class ' in c):overrides[p,i]=a+'\n'+c
Path('.merge-work/overrides.json').write_text(json.dumps({f'{p}#{i}':v for (p,i),v in overrides.items()}))
pat=re.compile(r'^<<<<<<< [^\n]*\n(.*?)^=======\n(.*?)^>>>>>>> [^\n]*\n',re.M|re.S)
for p,rows in cs.items():
 s=(Path('.merge-work/originals')/p).read_text();index=0
 def resolve(m):
  global index
  i=index;index+=1;a,b,c=rows[i]
  if (p,i) in overrides:return overrides[p,i]
  if a==c:return a
  if b is not None:
   if a==b:return c
   if c==b:return a
  if not a:return c
  if not c:return a
  if b is not None:return union(a,b,c)
  # Adjacent additive registrations compose; alternatives retain current main.
  if a.lstrip().startswith(('mod ','use ','    ["python3"')) and c.lstrip().startswith(('mod ','use ','    ["python3"')):return a+c
  return a
 Path(p).write_text(pat.sub(resolve,s))
print('resolved source files',len(cs))
