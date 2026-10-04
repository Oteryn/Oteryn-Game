"""Immutable Lua observations, exact raw identities and closed r64 controller data."""
import copy
import json
import re
import unittest
from lupa import LuaRuntime
import project_monster_world_controllers as p


class WorldControllers(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.rows=p.build();cls.by={r['controller']['parameters']['source_model']:r for r in cls.rows if r['source']=='canary'}
 def source(self,stem):
  row=self.by['r64/'+stem];root=row['source_proofs'][0];return p.read(root['source'],root['path']).decode(),row['controller']['parameters']
 def lua(self,source):
  lua=LuaRuntime(unpack_returned_tuples=True);lua.execute('function Spell(kind) return setmetatable({}, {__index=function(t,k) return function(...) return t end end}) end');lua.globals().CONST_ANI_HOLY='CONST_ANI_HOLY';lua.globals().CONST_ME_MAGIC_BLUE='CONST_ME_MAGIC_BLUE';lua.globals().CONST_ME_SOUND_RED='CONST_ME_SOUND_RED';lua.globals().TALKTYPE_MONSTER_YELL='TALKTYPE_MONSTER_YELL';lua.globals().TALKTYPE_MONSTER_SAY='TALKTYPE_MONSTER_SAY';return lua
 def position(self,lua,x,y=None,z=None,effects=None):
  if y is None:return x
  value=lua.table_from({'x':x,'y':y,'z':z});value['sendMagicEffect']=lambda self,effect:(effects.append(('magic',effect)) if effects is not None else None);value['sendDistanceEffect']=lambda self,target,effect:(effects.append(('distance',effect)) if effects is not None else None);return value
 def test_exact_population_and_identity(self):
  _,old=p.load_inputs();self.assertEqual(len(self.rows),21);index={p.identity(r):r for r in old}
  for row in self.rows:
   before=index[p.identity(row)]
   for field in ['source','monster','original_slot_sha256','source_parameters']:self.assertEqual(row[field],before[field])
   self.assertTrue(row['full_slot_projection_complete']);self.assertTrue(row['authoring_contract_extension_pending'])
   for field in p.FLAGS:self.assertFalse(row[field])
 def test_strict_schema_and_no_shell(self):
  schema=p.schema(self.rows);p.Draft202012Validator.check_schema(schema);validator=p.Draft202012Validator({'$ref':'#/$defs/controller',**schema})
  for row in self.rows:self.assertEqual(list(validator.iter_errors(row['controller'])),[])
  controller=copy.deepcopy(self.rows[0]['controller']);controller['parameters']['opaque_lua']='return true';self.assertTrue(list(validator.iter_errors(controller)))
  controller=copy.deepcopy(self.rows[0]['controller']);controller['parameters']['source_model']='invented';self.assertTrue(list(validator.iter_errors(controller)))
 def test_original_lua_mazoran_literal_inventory(self):
  source,model=self.source('mazoran_fire');prefix=source[:source.index('local function revertLava')];lua=self.lua(source);lua.globals().Position=lambda x,y,z:self.position(lua,x,y,z)
  protected,grounds,items=lua.execute(prefix+'\nreturn Montains,tiles,itemsRoom')
  def arr(t):return [t[i] for i in range(1,len(t)+1)]
  self.assertEqual(arr(protected),model['ground_scan']['protected_item_ids']);self.assertEqual(arr(grounds),model['restore']['ground_item_ids'])
  literal=[{'item_id':v['itemid'],'position':{k:v['position'][k] for k in ['x','y','z']}} for v in arr(items)];self.assertEqual(literal,model['restore']['ordered_entries']);self.assertEqual(len(literal),887);self.assertLess(len(set(p.canonical(x) for x in literal)),len(literal))
 def test_original_lua_fixed_heal_order(self):
  source,model=self.source('heal_brain_head')
  for tile_present,target_match in [(False,False),(True,False),(True,True)]:
   lua=self.lua(source);events=[];lua.globals().Position=lambda x,y,z:self.position(lua,x,y,z,events);origin=self.position(lua,1,2,3,events)
   monster=lua.table_from({'isMonster':lambda *a:True,'getName':lambda *a:'Brain Head' if target_match else 'Rat','addHealth':lambda self,value:events.append(('heal',value))});tile=lua.table_from({'getTopCreature':lambda *a:monster});lua.globals().Tile=lambda *a:tile if tile_present else None;lua.globals().math.random=lambda lo,hi:lo
   caster=lua.table_from({'getPosition':lambda *a:origin});spell=lua.execute(source+'\nreturn spell');result=spell['onCastSpell'](caster,None)
   self.assertEqual(result,tile_present)
   if tile_present:self.assertEqual(events[:2],[('distance',model['projectile']),('magic',model['impact_effect'])]);self.assertEqual(events[2:],[('heal',300)] if target_match else [])
   else:self.assertEqual(events,[])
 def test_original_lua_time_form_rng_floor_order(self):
  for stem in ['time_guardian','time_guardiann']:
   source,model=self.source(stem);lua=self.lua(source);events=[];lua.globals().Position=lambda x,y,z:self.position(lua,x,y,z);lua.globals().math.random=lambda *a:events.append('draw_form_index') or 1
   caster=lua.table_from({'getPosition':lambda *a:events.append('capture_caster_position') or self.position(lua,1,2,13)});spell=lua.execute(source+'\nreturn spell');self.assertTrue(spell['onCastSpell'](caster,None));expected=model['initial_order'][:model['initial_order'].index('floor_guard_return_true')];self.assertEqual(events,expected)
 def test_original_lua_vortex_and_generator(self):
  source,model=self.source('charge_vortex');lua=self.lua(source);events=[];ground=lua.table_from({'transform':lambda self,value:events.append(('transform',value))});pos=self.position(lua,32264,31253,14);tile=lua.table_from({'getGround':lambda *a:ground,'getPosition':lambda *a:pos});lua.globals().Position=lambda x,y,z:self.position(lua,x,y,z);lua.globals().Tile=lambda *a:tile;lua.globals().math.random=lambda *a:1;scheduled=[];lua.globals().addEvent=lambda fn,delay,*args:scheduled.append((fn,delay,args));spell=lua.execute(source+'\nreturn spell');self.assertIsNone(spell['onCastSpell'](None,None));self.assertEqual(events,[('transform',model['ground_id'])]);fn,delay,args=scheduled[0];self.assertEqual(delay,model['revert_delay_ms']);fn(*args);self.assertEqual(events[-1],('transform',model['revert_id']))
  source,model=self.source('generator');lua=self.lua(source);voices=[];monster=lua.table_from({'say':lambda self,text,kind:voices.append((text,kind))});spawn=[];lua.globals().Position=lambda x,y,z:self.position(lua,x,y,z);lua.globals().math.random=lambda *a:4;lua.globals().Game=lua.table_from({'createMonster':lambda name,pos,extended,force:spawn.append((name,{k:pos[k] for k in ['x','y','z']},extended,force)) or monster});spell=lua.execute(source+'\nreturn spell');self.assertIsNone(spell['onCastSpell'](None,None));self.assertEqual(spawn,[(model['monster_name'],model['positions'][3],True,True)]);self.assertEqual(voices,[(model['created_monster_voice'],model['voice_type'])])
 def test_original_lua_gaz_state_helper_and_rng(self):
  source,model=self.source("gaz'haragoth_summon");helper=p.read('canary','data/libs/functions/creature.lua').decode();helper=re.search(r'(?ms)^function Creature:setSummon\([^\n]*\)\n.*?^end[ \t]*$',helper).group(0)
  for count,draw,fail,foreign,expected_attempts,desired,result in [(0,0,False,False,2,2,True),(0,0,True,False,2,2,True),(2,24,True,False,1,3,True),(2,25,False,False,0,2,True),(7,0,False,False,0,2,False),(0,0,False,True,2,2,True)]:
   lua=self.lua(source);events=[];spawned=[];lua.execute('Creature={}; function table.contains(t,v) for _,x in ipairs(t) do if x==v then return true end end return false end');lua.execute(helper);lua.globals().Monster=lambda value:value;foreign_sum=lua.table_from({'setMaster':lambda self,master,flag:events.append(('master',master,flag)),'setTarget':lambda self,target:events.append(('target',target))}) if foreign else None;lua.globals().sum=foreign_sum;lua.globals().DATA_DIRECTORY='data-otservbr-global';lua.globals().dofile=lambda *a:lua.execute('GazVariables={MinionsNow=2,MaxSummons=7}');lua.globals().math.random=lambda *a:draw
   pos=self.position(lua,1,2,3,events);caster=lua.table_from({'getPosition':lambda *a:pos,'say':lambda self,text,kind:events.append(('say',text))});spectators=lua.table_from([lua.table_from({'getName':lambda *a:"Minion of Gaz'haragoth"}) for i in range(count)])
   def create(name,pos,extended,force):
    spawned.append((name,extended,force));return None if fail else lua.eval('setmetatable({}, {__index=Creature})')
   lua.globals().Game=lua.table_from({'getSpectators':lambda *a:spectators,'createMonster':create});spell=lua.execute(source+'\nreturn spell');self.assertEqual(spell['onCastSpell'](caster,None),result);self.assertEqual(len(spawned),expected_attempts);self.assertEqual(lua.globals().GazVariables['MinionsNow'],desired)
   if foreign:self.assertEqual(len([x for x in events if x[0]=='master']),expected_attempts);self.assertTrue(all(x[2] is True for x in events if x[0]=='master'))
   else:self.assertEqual([x for x in events if x[0]=='master'],[])
   self.assertEqual(len([x for x in events if x[0]=='say']),expected_attempts)

if __name__=='__main__':unittest.main()
