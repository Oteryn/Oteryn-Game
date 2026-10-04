"""Independent immutable-source formulas/branches and closed candidate contracts."""
import copy
import json
import math
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

from lupa import LuaRuntime
import project_source_world_control_candidates as p

ROOT=p.HERE.parents[2]


def flat(model,level):
 if model['kind']=='crystal_base_damage_healing':
  env={'level':level};env['step']=evaluate(model['step'],env,{})
  return evaluate(model['result'],env,{})
 aggregate=model['aggregated_baseline_initial'];baseline=model['current_level_baseline_initial'];denominator=model['factor_denominator_initial'];threshold=model['threshold_initial'];step=model['threshold_step_initial'];tier=model['tier_initial'];factor=1/denominator
 while level>=threshold:
  baseline=threshold;factor=1/(denominator+tier);aggregate+=threshold/(denominator+tier-1);tier+=1;threshold+=step;step+=model['next_threshold_step_add']
 return min(math.ceil(aggregate+(level-baseline)*factor),model['result_ceiling'])


def evaluate(expr,env,helpers):
 if 'const' in expr:return float(expr['const'])
 if 'helper' in expr:return flat(helpers[expr['helper']],env[expr['input']])
 if 'input' in expr:return env[expr['input']]
 args=[evaluate(a,env,helpers) for a in expr['arguments']];op=expr['operator']
 if op=='add':return sum(args)
 if op=='multiply':
  value=args[0]
  for a in args[1:]:value*=a
  return value
 if op=='divide':return args[0]/args[1]
 if op=='subtract':return args[0]-args[1]
 if op=='floor':return math.floor(args[0])
 if op=='sqrt':return math.sqrt(args[0])
 if op=='negate':return -args[0]
 if op=='maximum':return max(args)
 raise ValueError(op)


class SourceWorldControl(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.rows=p.build(ROOT);cls.active=[r for r in cls.rows if 'spell' in r];cls.by={(r['registration_key'].split('-')[0],Path(r['registration_key'].split('#')[0]).stem):r['spell']['spell']['execution']['native_behavior']['parameters'] for r in cls.active}
  cls.temp=tempfile.TemporaryDirectory();base=Path(cls.temp.name)
  raw=p.src('canary','src/creatures/players/player.cpp')
  defs='\n'.join(p.base.source_cpp_function(raw,name).decode() for name in ['Player::calculateFlatDamageHealing','Player::getHarmonyBonus','Player::getHarmonyDamage'])
  source='''#include <cmath>\n#include <cstdint>\n#include <limits>\n#include <iostream>\n#include <algorithm>\n#include <utility>\nenum class Virtue_t {None,Harmony};\nconstexpr int CONDITION_SERENE=1, BUFF_HARMONYBONUS=2;\nenum class WheelStage_t{ASCETIC};\nstruct Wheel{int stage;int getStage(WheelStage_t){return stage;}};\nstruct Player{uint32_t level;uint8_t harmony;Virtue_t virtue;bool serene;int buff;Wheel m_wheelPlayer;bool hasCondition(int){return serene;}int getBuff(int){return buff;}uint16_t calculateFlatDamageHealing()const;double_t getHarmonyBonus();std::pair<uint64_t,uint64_t> getHarmonyDamage(double,double);};\n'''+defs+'''\nint main(){uint32_t level;int harmony,virtue,serene,wheel,buff;double lo,hi;while(std::cin>>level>>harmony>>virtue>>serene>>wheel>>buff>>lo>>hi){Player p{level,(uint8_t)harmony,virtue?Virtue_t::Harmony:Virtue_t::None,(bool)serene,buff,Wheel{wheel}};auto pair=p.getHarmonyDamage(lo,hi);std::cout<<p.calculateFlatDamageHealing()<<" "<<pair.first<<" "<<pair.second<<"\\n";}}\n'''
  (base/'actual.cpp').write_text(source);subprocess.run(['g++','-std=c++20',str(base/'actual.cpp'),'-o',str(base/'actual')],check=True,capture_output=True);cls.actual=base/'actual'
 @classmethod
 def tearDownClass(cls):cls.temp.cleanup()
 def test_exact_population_and_headers(self):
  self.assertEqual((len(self.rows),len(self.active)),(37,31))
  for row in self.rows:
   raw=(ROOT/p.old.BASE/p.folder(row['registration_key'])/'source-header.json').read_bytes();self.assertEqual(row['source_header_bytes'],raw)
   self.assertFalse(row['runtime_activation']);self.assertFalse(row['native_execution_qualified'])
   if 'spell' in row:self.assertTrue(row['authoring_contract_extension_pending']);self.assertEqual(row['spell']['spell']['costs']['mana'],json.loads(raw)['spell'].get('costs',{}).get('mana',0))
 def test_closed_models_and_mutation(self):
  schema=p.extension(self.rows);p.Draft202012Validator.check_schema(schema)
  with tempfile.TemporaryDirectory() as d:
   path=Path(d)/'extension.json';p.put(path,schema);validator,_,_=p.compose.validator([path],list(p.validate_spell.SCHEMAS.values()))
   for row in self.active:self.assertEqual(list(validator.iter_errors(row['spell'])),[])
   sample=copy.deepcopy(self.active[0]['spell']);sample['spell']['execution']['native_behavior']['parameters']['invented']=True;self.assertTrue(list(validator.iter_errors(sample)))
   sample=copy.deepcopy(self.active[0]['spell']);sample['spell']['execution']['native_behavior']['parameters']['source_model']='r62/pretend';self.assertTrue(list(validator.iter_errors(sample)))
 def test_barrier_donor_branches_and_shared_selector(self):
  for stem in ['magic_wall','wild_growth']:
   c=self.by['canary',stem];r=self.by['crystal',stem];self.assertTrue(c['expert_pvp_context_from_caster']);self.assertFalse(r['expert_pvp_context_from_caster']);self.assertEqual(r['safe_item_when'],['WORLDTYPE_OPTIONAL']);self.assertEqual(r['creation_position'],'target_position');self.assertEqual(c['insert_flag'],'FLAG_NOLIMIT')
  for stem in ['heal_friend',"nature's_embrace"]:
   model=self.by['crystal',stem];self.assertEqual(model['party_selector']['distance'],'Chebyshev_XY_to_caster');self.assertFalse(model['party_selector']['viewport_filter']);self.assertTrue(model['secondary_attempted_even_primary_false'])
 def test_world_literals_and_sequence(self):
  for donor in ['canary','crystal']:
   self.assertEqual(self.by[donor,'find_person']['geometry']['far_distance_exclusive'],275)
   self.assertEqual(self.by[donor,'desintegrate_rune']['removal']['unique_id_strictly_greater_than'],65535)
  self.assertEqual(self.by['canary','sweeping_takedown']['execution_order'],['inner_execute_ignore_result','outer_execute_ignore_result','clear_cache','return_true'])
  self.assertEqual(self.by['crystal','divine_grenade']['schedule_order'],['explode_at_3000','remove_indicator_at_3000'])
  self.assertEqual(self.by['crystal','balanced_brawl']['failure_return'],False)
  self.assertEqual(self.by['crystal',"nature's_embrace"]['self_refusal_message'],"You can't cast this spell to yourself.")
 def test_actual_lua_mass_mend_draw_order_and_sign(self):
  params=self.by['crystal','mass_spirit_mend'];lua=LuaRuntime(unpack_returned_tuples=True)
  source=p.src('crystal','data/scripts/spells/healing/mass_spirit_mend.lua').decode();helper=p.src('crystal','data/scripts/lib/register_spells.lua').decode()
  lua.execute(re.search(r'(?ms)^function calculateBaseDamageHealing\([^\n]*\)\n.*?^end[ \t]*$',helper).group(0))
  lua.execute(re.search(r'(?ms)^local function targetFunction\([^\n]*\)\n.*?^end[ \t]*$',source).group(0).replace('local function ','function ',1))
  lua.execute('CONST_ME_MAGIC_BLUE=1;CONST_ME_MAGIC_RED=2; function table.contains(t,v) for _,x in ipairs(t) do if x==v then return true end end return false end')
  for is_player,is_monster,name,targetid,draws,heal,effect in [(True,False,'Caster',1,2,True,1),(True,False,'Friend',2,1,True,1),(False,True,'Leiden',3,1,True,2),(False,True,'RIFT FRAGMENT',4,1,True,1),(False,True,'Rat',5,1,False,None),(True,False,'specific_creature_name',6,1,False,None)]:
   sampled=[];health=[];effects=[];lua.globals().math.random=lambda lo,hi:sampled.append((lo,hi)) or lo
   pos=lua.table_from({'sendMagicEffect':lambda self,value:effects.append(value)});player=lua.table_from({'getLevel':lambda *a:500,'getMagicLevel':lambda *a:100})
   caster=lua.table_from({'getPlayer':lambda *a:player,'getId':lambda *a:1});target=lua.table_from({'getId':lambda *a:targetid,'isPlayer':lambda *a:is_player,'isMonster':lambda *a:is_monster,'getName':lambda *a:name,'addHealth':lambda self,value:health.append(value),'getPosition':lambda *a:pos})
   lua.globals().targetFunction(caster,target);self.assertEqual(len(sampled),draws);self.assertEqual(bool(health),heal)
   helpers={'crystal_base_damage_healing':params['helper_definitions'][0]};env={'level':500,'magic_level':100}
   self.assertEqual(sampled[0],tuple(evaluate(params['formula'][k],env,helpers) for k in ['minimum','maximum']))
   if targetid==1:self.assertEqual(sampled[1],tuple(evaluate(params['self_formula'][k],env,helpers) for k in ['minimum','maximum']))
   if heal:self.assertGreater(health[0],0);self.assertEqual(effects,[effect])

 def test_actual_cpp_flat_and_harmony_grid(self):
  levels=[0,1,499,500,501,1099,1100,1101,1799,1800,1801,2600,10000,100000,1000000]
  cases=[(level,h,v,s,w,b) for level in levels for h,v,s,w,b in [(0,0,0,0,0),(1,0,0,0,0),(5,1,1,3,102),(5,1,0,2,100),(3,0,0,0,50)]]
  data=''.join(' '.join(map(str,case))+' 125.5 242.75\n' for case in cases);lines=subprocess.check_output([str(self.actual)],input=data.encode()).decode().splitlines();helper=p.flat_helper('canary');hmodel=p.harmony_helper()
  for case,line in zip(cases,lines):
   level,h,v,s,w,b=case;got=list(map(int,line.split()));self.assertEqual(got[0],flat(helper,level));percent=hmodel['base_percent']+(hmodel['harmony_virtue_bonus_serene'] if s else hmodel['harmony_virtue_bonus_other']) * v+w+(b-100 if b else 0);multiplier=1 if h==0 or percent<=0 else 1+percent*2**(h-1)/100;self.assertEqual(got[1:],[int(125.5*multiplier),int(242.75*multiplier)])
 def test_actual_lua_formula_grid(self):
  # Execute the immutable donor callback functions; mocks bind supplied live inputs only.
  for row in self.active:
   reg=row['registration_key'];donor=reg.split('-')[0];stem=Path(reg.split('#')[0]).stem;params=row['spell']['spell']['execution']['native_behavior']['parameters']
   if not any(k in params for k in ['formula','primary_formula']):continue
   if stem=='mass_spirit_mend':continue # inline random draws checked separately by source eligibility review
   raw=p.src(donor,reg.split('/',1)[1].split('#')[0]).decode();lua=LuaRuntime(unpack_returned_tuples=True)
   if donor=='crystal':
    helper=p.lua_proof(donor,'data/scripts/lib/register_spells.lua','calculateBaseDamageHealing');text=p.src(donor,helper['path']).decode();chunk=re.search(r'(?ms)^function calculateBaseDamageHealing\([^\n]*\)\n.*?^end[ \t]*$',text).group(0);lua.execute(chunk)
   for match in re.finditer(r'(?m)^local (SPELL_BASE_POWER(?:_CENTER)?|HEAL_SCALE|SHARED_CONSERVATION_RATIO) = ([^\n]+)',raw):lua.execute(match.group(1)+' = '+match.group(2))
   chunks=re.findall(r'(?ms)^(?:local )?function (?:calculateSweepingDamage|onGetDeathEchoValues|onGetDeathEchoEchoValues|onGetFormulaValues[^\s(]*)\([^\n]*\)\n.*?^end[ \t]*$',raw)
   lua.execute('sweepingTakedownCache={}; WHEEL_GRADE_NONE=0; '+ '\n'.join(c.replace('local function ','function ',1) for c in chunks))
   for level,ml,skill,attack in [(0,0,0,7),(1,1,110,7),(499,80,111,40),(500,120,121,50),(1100,100,161,65),(2600,150,251,70)]:
    helpers={h['kind']:h for h in params['helper_definitions'] if 'kind' in h};flat_value=flat(p.flat_helper(donor),level);player=lua.table_from({'calculateFlatDamageHealing':lambda *a:flat_value,'getHarmonyDamage':lambda self,lo,hi:(int(lo),int(hi)),'getId':lambda *a:1,'upgradeSpellsWOD':lambda *a:0})
    callback=lua.globals().onGetFormulaValuesInner if stem=='sweeping_takedown' else lua.globals().onGetDeathEchoValues if stem=='death_echo' else lua.globals().onGetFormulaValues
    got=callback(player,skill,attack,0) if donor=='canary' else callback(player,level,ml,0)
    formula=params.get('formula',params.get('primary_formula'));env={'level':level,'magic_level':ml,'attack_skill':skill,'attack_value':attack}
    if stem=='sweeping_takedown':
     coef=next((c for threshold,c in params['skill_bonus']['strict_thresholds_descending'] if skill>threshold),0);env['skill_quadratic_bonus']=(skill-110)**2*coef
    wanted=[evaluate(formula[k],env,helpers) for k in ['minimum','maximum']]
    if params.get('harmony_pair_multiplier'):wanted=list(map(int,wanted))
    for a,b in zip(got,wanted):self.assertAlmostEqual(a,b,places=8,msg=reg)
    if 'target_damage_formula' in params:
     # Accepted target normalization runs original Lua math with independently
     # executed Crystal helper and negates the completed optional Harmony pair.
     actual_crystal=LuaRuntime(unpack_returned_tuples=True);helper_text=p.src('crystal','data/scripts/lib/register_spells.lua').decode();actual_crystal.execute(re.search(r'(?ms)^function calculateBaseDamageHealing\([^\n]*\)\n.*?^end[ \t]*$',helper_text).group(0))
     canonical_flat=actual_crystal.globals().calculateBaseDamageHealing(level)
     target_player=lua.table_from({'calculateFlatDamageHealing':lambda *a:canonical_flat,'getHarmonyDamage':lambda self,lo,hi:(int(lo),int(hi)),'getId':lambda *a:1})
     actual_target=callback(target_player,skill,attack,0)
     target_helpers={'crystal_base_damage_healing':params['target_level_helper']}
     target_bounds=[evaluate(params['target_damage_formula'][k],env,target_helpers) for k in ['minimum','maximum']]
     if params.get('harmony_pair_multiplier'):target_bounds=list(map(int,target_bounds))
     for a,b in zip(actual_target,target_bounds):self.assertAlmostEqual(-a,b,places=8,msg='normalized '+reg)


if __name__=='__main__':unittest.main()
