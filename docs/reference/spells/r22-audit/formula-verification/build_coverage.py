"""Evaluate exact current ASTs against external recorded/live neutral caster facts.
No authored RNG means are invented. Native stages and Harmony stay incomparable.
"""
import argparse,collections,copy,hashlib,json,math,pathlib,sys
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--workspace',type=pathlib.Path,default=pathlib.Path('/workspace'))
parser.add_argument('--out',type=pathlib.Path,default=pathlib.Path(__file__).parent)
parser.add_argument('--repo',type=pathlib.Path)
parser.add_argument('--bundles',type=pathlib.Path)
parser.add_argument('--baseline',type=pathlib.Path)
parser.add_argument('--live',type=pathlib.Path)
parser.add_argument('--comparison',type=pathlib.Path)
args=parser.parse_args()
ROOT=args.workspace;OUT=args.out;OUT.mkdir(parents=True,exist_ok=True)
REPO=args.repo or ROOT/'Oteryn-Game'
sys.path.insert(0,str(REPO/'tools/content-schema/spell-authoring'))
from validate_spell import evaluate,expression_shape
sys.path.insert(0,str(pathlib.Path(__file__).parent));from compare_current import nominal_center_ast
load=lambda p:json.loads(pathlib.Path(p).read_text())
save=lambda p,x:pathlib.Path(p).write_text(json.dumps(x,indent=2,ensure_ascii=False)+'\n')
comparison=load(args.comparison or OUT/'current-comparison/comparison.json');reference=load(args.baseline or ROOT/'spells-r18/implementation-handoff-r18/calculator-vectors/fixtures/baseline.json');live=load(args.live or OUT/'live-reference.json')
metadata={s['id']:s for s in reference['metadata']['spells']}
carrier=args.bundles or ROOT/'spells-r21-implemented/bundles'

def candidates(spell,deps):
 out=[]
 for f in deps['formulas']:
  if 'minimum' in f and 'maximum' in f:out.append(('dependencies.formulas/'+f['identity']['key'],f))
 def walk(value,path):
  if isinstance(value,dict):
   if isinstance(value.get('minimum'),dict) and isinstance(value.get('maximum'),dict):out.append((path,value))
   else:
    for k,v in value.items():walk(v,path+'/'+k)
  elif isinstance(value,list):
   for k,v in enumerate(value):walk(v,path+'/'+str(k))
 walk(spell.get('execution',{}),'spell.execution')
 return out

def env_for(formula,spell,case):
 used=set().union(*(expression_shape(formula[b])[2] for b in ('minimum','maximum')))
 mapping={'level':'level','magic_level':'magic_level','attack_skill':'attack_skill','attack_value':'attack_value','attack_factor':'attack_factor','shielding_skill':'shielding','shield_defense':'shield_defense'}
 env={k:spell.get('base_power') if k=='base_power' else (case.get('shieldMetadata') or {}).get('defense') if k=='shield_defense' else case['formulaInputs'].get(mapping.get(k,'')) for k in used}
 return env if all(x is not None for x in env.values()) else None

entries=[];rows=[]
for record in comparison['catalog']:
 if record['disposition']=='removed':continue
 bundle=record['bundle'];sp=load(carrier/bundle/'spell.json')['spell'];dep=load(carrier/bundle/'dependencies.json');fs=candidates(sp,dep)
 entry={'bundle':bundle,'name':sp['name'],'carrier':sp['carrier'],'identity':sp['identity'],'base_power':sp.get('base_power'),'native_behavior':sp.get('execution',{}).get('native_behavior',{}).get('key'),'effect_operations':record['effect_operations'],'formula_paths':[p for p,f in fs],'matched_calculator_ids':record['matched_calculator_ids'],'original_bounds_cases':record['compared_cases'],'original_nominal_cases':record['nominal_centers_compared'],'mechanics_verification':'source-imported; this arithmetic lane does not certify cast/target/world execution','gameplay_e2e_verified':False}
 for ref_name,ref in [('recorded',reference),('live',live)]:
  for case in ref['cases']:
   for result in case['response']['spells']:
    if result['id'] not in record['matched_calculator_ids']:continue
    card=metadata[result['id']]
    r={'bundle':bundle,'reference':ref_name,'case_id':case['caseId'],'calculator_id':result['id'],'formula_input':case['formulaInputs'],'oracle_raw':result['raw']}
    reason=None
    if not fs:reason='no_damage_ast_in_extracted_formula_paths'
    elif len(fs)!=1:reason='multiple_formula_components_require_exact_component_owner'
    elif card.get('stage',0)>0:reason='stage_specific_reference_requires_current_wheel_stage'
    elif card.get('additionalDamageMultiplier',1)!=1:reason='component_multiplier_reference_requires_exact_component_owner'
    elif sp.get('harmony_role')=='spender' and card.get('isSpender'):reason='pre_harmony_vs_full_five_harmony_different_states'
    else:
     path,f=fs[0];env=env_for(f,sp,case)
     if env is None:reason='unsupported_formula_input'
     elif result['raw'].get('min') is not None and result['raw'].get('max') is not None:
      got=[math.trunc(evaluate(f[b],env)) for b in ('minimum','maximum')];want=[result['raw']['min'],result['raw']['max']]
      r.update(status='bounds_equal' if got==want else 'bounds_differ',candidate=got,oracle=want,formula_path=path,input=env)
      if 'shield_defense' in env:r['source_model_conflict_note']='Oteryn source requires actual shield_item defense; calculator includes shield defense plus weapon defenseMod, BP/4 offset and buckets4. This is retained external-model divergence, not a same-input proof of runtime defect.'
     else:
      center,scope=nominal_center_ast(f)
      if center is None:reason='no_published_bounds_and_no_explicit_nominal_component'
      else:
       got=evaluate(center,env);want=result['raw']['avg'];r.update(status='nominal_equal' if got==want else 'nominal_differ',candidate=got,oracle=want,formula_path=path,input=env,scope=scope)
    if reason:r.update(status='not_compared',reason=reason)
    rows.append(r)
 rel=[r for r in rows if r['bundle']==bundle]
 counts=collections.Counter(r['status'] for r in rel);entry['comparison_counts']=dict(counts);entry['unverified_reasons']=sorted({r['reason'] for r in rel if 'reason' in r})
 if counts['bounds_differ'] or counts['nominal_differ']:status='independent_numeric_conflict'
 elif counts['bounds_equal']:status='verified_neutral_bounds_only'
 elif counts['nominal_equal']:status='verified_nominal_component_only_bounds_rng_unknown'
 elif fs:status='no_independent_formula'
 else:status='not_applicable_to_hp_formula' if not record['matched_calculator_ids'] else 'no_independent_formula_extracted'
 if status=='not_applicable_to_hp_formula' and any(e.get('condition',{}).get('damage_over_time') or e.get('condition',{}).get('regeneration') for e in dep['effects']):status='no_independent_damage_or_regeneration_schedule'
 if status=='not_applicable_to_hp_formula' and sp['carrier']=='rune' and any(e['operation']=='create_item' and any(t in sp['name'].casefold() for t in ('fire','poison','energy')) for e in dep['effects']):status='no_independent_field_damage_schedule'
 entry['formula_verification']=status
 entry['input_files']={n:hashlib.sha256((carrier/bundle/n).read_bytes()).hexdigest() for n in ('spell.json','dependencies.json')}
 entries.append(entry)
assert len(entries)==246 and len({e['bundle'] for e in entries})==246
conflicts=[r for r in rows if r['status'] in ('bounds_differ','nominal_differ')]
summary={'active_entries':246,'removed_entries_excluded':6,'classifications':dict(collections.Counter(e['formula_verification'] for e in entries)),'comparison_statuses':dict(collections.Counter(r['status'] for r in rows)),'numeric_conflict_bundles':sorted({r['bundle'] for r in conflicts}),'all_mechanics_externally_verified':False}
save(OUT/'per-spell-formula-coverage.json',{'schema':'OTERYN_R22_ALL_ACTIVE_FORMULA_COVERAGE/v1','summary':summary,'sources':[{'url':'https://github.com/kik-tibia/tibiatools','commit':'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1','method':'pinned upstream calculator recorded actual executions'},{'url':'https://tibiatools.io/api/v1/damage','method':'live normal HTTP POST','receipts':'live-probe-receipts.json','revision':'not published by API'}],'limitations':['Equal bounds certify only neutral arithmetic under supplied inputs, not RNG or full execution.','Nominal value is explicit common authored component; never inferred by averaging endpoints.','N/A HP formula does not mean all support mechanics verified.','No independent data is explicitly unverified; no fabricated formula bounds.','Harmony/stage-specific source states remain distinct.','Live API metadata identity joined exact pinned metadata and unchanged responses; current backend revision unpublished.'],'entries':entries})
save(OUT/'formula-comparisons.json',{'summary':summary,'rows':rows});save(OUT/'formula-conflicts.json',{'summary':summary,'conflicts':conflicts})
print(json.dumps(summary))
