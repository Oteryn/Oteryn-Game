"""Audit all candidate Spell records against genuine all-vocation TibiaPal probes.
Authoring formula evaluation only; no runtime activation or mutation of either repo.
"""
import argparse, collections, copy, hashlib, json, math, subprocess, sys
from pathlib import Path

DEFAULT_DELIVERY = Path('/workspace/scratch/oteryn-spell-delivery')
DEFAULT_CANDIDATE = Path('/workspace/scratch/oteryn-spell-candidate')

def load(p): return json.loads(Path(p).read_text())
def norm(s): return ' '.join(s.casefold().split())
def identity(d): return (d['key'], d['revision'])
def save(p,d): p.write_text(json.dumps(d,indent=2,ensure_ascii=False)+'\n')

def nominal_center_ast(formula):
    """Extract explicit common nominal component, never average the endpoints."""
    low, high=formula['minimum'],formula['maximum']
    la, ha=low.get('args',[]),high.get('args',[])
    if low.get('op')=='mul' and high.get('op')=='mul' and len(la)==len(ha)==2:
        if la[0]==ha[0] and la[1]=={'const':'0.9'} and ha[1]=={'const':'1.1'}:
            return copy.deepcopy(la[0]), 'Retained OTS0.9/1.1 callback interval around an explicit nominal builder component; RNG mean unqualified.'
    if low.get('op')=='add' and high.get('op')=='add' and len(la)==len(ha)==2 and la[0]==ha[0]:
        lf,hf=la[1],ha[1]
        if lf.get('op')=='floor' and hf.get('op')=='floor' and len(lf.get('args',[]))==len(hf.get('args',[]))==1:
            lterm,hterm=lf['args'][0],hf['args'][0]
            lt,ht=lterm.get('args',[]),hterm.get('args',[])
            if lterm.get('op')=='sub' and hterm.get('op')=='add' and len(lt)==len(ht)==2 and lt==ht:
                return {'op':'add','args':[copy.deepcopy(la[0]),{'op':'floor','args':[copy.deepcopy(lt[0])]}]}, 'Explicit proposed nominal component with original source interval half-width retained; RNG mean and endpoints unqualified.'
    return None,None

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--delivery',type=Path,default=DEFAULT_DELIVERY)
    parser.add_argument('--candidate',type=Path,default=DEFAULT_CANDIDATE)
    parser.add_argument('--reference',type=Path)
    parser.add_argument('--bundles',type=Path,default=DEFAULT_DELIVERY/'formula-repair-r18/candidate/bundles')
    parser.add_argument('--readiness',type=Path,default=DEFAULT_DELIVERY/'formula-repair-r18/candidate/readiness.json')
    parser.add_argument('--out',type=Path)
    a=parser.parse_args()
    reference_path=a.reference or a.delivery/'all-professions/reference/cases.json'
    out=a.out or a.delivery/'formula-repair-r18/validation/comparison'
    out.mkdir(parents=True,exist_ok=True)
    sys.path.insert(0,str(a.candidate/'tools/content-schema/spell-authoring'))
    from validate_spell import evaluate, expression_shape
    readiness=load(a.readiness)
    readiness_index={(norm(s['name']),s['spell_type']):s for s in readiness['spells']}
    ref=load(reference_path)
    definitions=ref['metadata']['spells']
    by_id={s['id']:s for s in definitions}
    # A scope binds stage/component records to an upstream-provided displayName.
    # No hand-written synonym table or suffix-stripping joins are allowed.
    scope_names=collections.defaultdict(set)
    for s in ref['metadata']['rawSpellDefinitions']:
        if 'displayName' in s or (s.get('stage',0)==0 and '(' not in s['name']):
            scope_names[s['scope']].add(norm(s.get('displayName',s['name'])))
    # Only a unique anchor spelling is accepted; scope conflicts are explicit.
    records=[]; index=collections.defaultdict(list)
    for p in sorted(a.bundles.glob('*/spell.json')):
        spell=load(p)['spell']; deps=load(p.parent/'dependencies.json')
        r=readiness_index[(norm(spell['name']),spell['carrier'])]
        record={'bundle':p.parent.name,'name':spell['name'],'carrier':spell['carrier'],
          'identity':spell['identity'],'authoring_status':r['status'],'blockers':r['blockers'],
          'requirements':spell['requirements'],'base_power':spell.get('base_power'),
          'harmony_role':spell.get('harmony_role'),'formula_count':len(deps['formulas']),
          'effect_operations':[e['operation'] for e in deps['effects']],
          'matched_calculator_ids':[],'_spell':spell,'_deps':deps}
        records.append(record);index[(norm(spell['name']),spell['carrier'])].append(record)
    assert len(records)==252, f'Expected 252 inventory entries, got {len(records)}'
    joins={}; joins_report=[]
    for s in definitions:
        carrier={'spell':'instant','rune':'rune'}.get(s['spellType'])
        if carrier is None:
            joins_report.append({'calculator_id':s['id'],'name':s['name'],'status':'outside_spell_catalog','reason':s['spellType']});continue
        names={norm(s['name'])}
        direct=index.get((norm(s['name']),carrier),[])
        proof='exact_name_and_carrier'
        matches=direct
        if not matches:
            names=scope_names[s['scope']]
            if len(names)==1:
                matches=index.get((next(iter(names)),carrier),[])
                proof='upstream_explicit_display_name_and_scope_component'
        if len(matches)!=1:
            joins_report.append({'calculator_id':s['id'],'name':s['name'],'status':'ambiguous_identity' if matches else 'absent_from_candidate_catalog','scope_names':sorted(names)});continue
        r=matches[0];joins[s['id']]=r;r['matched_calculator_ids'].append(s['id'])
        joins_report.append({'calculator_id':s['id'],'name':s['name'],'status':'matched','bundle':r['bundle'],'proof':proof,'scope':s['scope'],'stage':s.get('stage')})
    rows=[]; pending=collections.Counter(); matrix=collections.defaultdict(collections.Counter)
    for case in ref['cases']:
        vocation=case['normalizedRequest']['vocation'] if 'vocation' in case.get('normalizedRequest',{}) else case['request']['stats']['vocation']
        formula_inputs=case['formulaInputs']
        for result in case['response']['spells']:
            s=by_id[result['id']]; r=joins.get(s['id']);matrix[vocation]['calculator_rows']+=1
            base={'case_id':case['caseId'],'family':case['family'],'vocation':vocation,
              'calculator_id':s['id'],'calculator_name':s['name'],'calculator_raw':result['raw'],
              'calculator_stage':s.get('stage'),'bundle':r['bundle'] if r else None}
            reason=None;limitations=[];formula=None;env={}
            if r is None:reason='no_spell_identity_match'
            elif r['authoring_status']!='ready':reason='blocked_authoring'
            elif s.get('stage',0)>0:reason='calculator_stage_selection_not_authored'
            elif s.get('additionalDamageMultiplier',1)!=1:reason='calculator_component_multiplier_not_authored'
            else:
                deps=r['_deps'];sp=r['_spell']
                if len(deps['effects'])!=1 or deps['effects'][0]['operation']!='damage':reason='not_single_damage_effect'
                elif len(deps['formulas'])!=1:reason='not_single_formula'
                else:
                    effect=deps['effects'][0]; fref=effect.get('formula')
                    formula=deps['formulas'][0]
                    if not fref or identity(fref)!=identity(formula['identity']):reason='formula_reference_not_exact'
                    elif formula['kind']!='player_expression':reason='not_player_expression'
                    else:
                        used=set().union(*(expression_shape(formula[b])[2] for b in ('minimum','maximum')))
                        mapping={'level':'level','magic_level':'magic_level','attack_skill':'attack_skill','attack_value':'attack_value','attack_factor':'attack_factor','shielding_skill':'shielding','shield_defense':'shield_defense'}
                        missing=[]
                        for variable in sorted(used):
                            if variable=='base_power':value=sp.get('base_power')
                            else:value=formula_inputs.get(mapping.get(variable,''))
                            if value is None:missing.append(variable)
                            else:env[variable]=value
                        if missing:reason='unsupported_formula_inputs:'+','.join(sorted(missing))
                        elif result['raw'].get('min') is None or result['raw'].get('max') is None:reason='calculator_has_no_bounds'
                        else:
                            for ab in deps['abilities']:
                                for field in ('chain','variants','windup_ms','encounter'):
                                    if field in ab:limitations.append('ability_'+field+'_execution_untested')
                            if sp.get('harmony_role'):limitations.append('harmony_state_and_cast_not_tested')
                            if s.get('stage')==0:limitations.append('calculator_central_component_only')
            if reason:
                base.update(verdict='not_compared',reason=reason)
                if reason=='calculator_has_no_bounds':
                    diagnostic=[math.trunc(evaluate(formula[b],env)) for b in ('minimum','maximum')]
                    average=result['raw'].get('avg')
                    if average is not None:
                        if r['harmony_role']=='spender' and s.get('isSpender'):
                            base['state_conflict']='Candidate source pre-Harmony bounds versus calculator fixed full5Harmony power; base bounds and upstream mean describe different states.'
                            base['qualification']='pre_harmony_vs_full_harmony_not_comparable'
                            matrix[vocation]['pre_harmony_vs_full_harmony_cases']+=1
                        base.update(input=env,candidate_truncated_bounds=diagnostic,
                          average_interval_consistency='inside_interval' if diagnostic[0]<=average<=diagnostic[1] else 'outside_interval',
                          average_interval_scope='Necessary compatibility only, not equality of expected values or RNG qualification.')
                        center_ast,center_scope=nominal_center_ast(formula)
                        if center_ast is not None:
                            nominal=evaluate(center_ast,env)
                            base.update(nominal_center_value=nominal,nominal_center_matches_reference=nominal==average,nominal_center_scope=center_scope)
                            matrix[vocation]['nominal_centers_compared']+=1
                            matrix[vocation]['nominal_centers_same']+=int(nominal==average)
                            matrix[vocation]['nominal_centers_different']+=int(nominal!=average)
                        matrix[vocation]['average_interval_diagnostics']+=1
                        matrix[vocation]['average_outside_authored_interval']+=int(not diagnostic[0]<=average<=diagnostic[1])
                matrix[vocation]['not_compared']+=1;pending[reason]+=1
            else:
                raw=[evaluate(formula[b],env) for b in ('minimum','maximum')]
                # This is the existing converter/authoring truncation convention.
                # Do not claim it proves runtime rounding or RNG distribution.
                rounded=[math.trunc(x) for x in raw]
                expected=[result['raw']['min'],result['raw']['max']]
                verdict='same_bounds' if rounded==expected else 'different_bounds'
                base.update(verdict=verdict,input=env,candidate_expression_bounds=raw,
                   candidate_truncated_bounds=rounded,delta=[rounded[i]-expected[i] for i in (0,1)],
                   formula_identity=formula['identity'],limitations=sorted(set(limitations)),
                   float_bounds_near_integer=[abs(x-round(x))<1e-9 and x!=round(x) for x in raw],
                   comparison_scope='limited_base_component' if limitations else 'pure_single_damage_expression',
                   rng_distribution_tested=False,runtime_rounding_tested=False,cast_eligibility_tested=False)
                if norm(r['name'])=='strong ethereal spear' and sp.get('base_power')==38 and s['power']==25:
                    base['source_conflict']='Wiki/declared BP38 versus calculator BP25; retained explicitly, not fitted away.'
                    base['qualification']='explicit_retained_source_conflict'
                    matrix[vocation]['explicit_retained_source_conflict']+=1
                matrix[vocation]['compared']+=1;matrix[vocation][verdict]+=1
                matrix[vocation][base['comparison_scope']]+=1
            rows.append(base)
    record_summary=[]
    for r in records:
        relevant=[x for x in rows if x['bundle']==r['bundle']]
        numerical=[x for x in relevant if x['verdict']!='not_compared']
        removed=any('removed from the game' in b or 'S25' in b for b in r['blockers'])
        if removed:disposition='removed'
        elif r['authoring_status']!='ready':disposition='blocked_authoring'
        elif numerical:disposition='numeric_difference_observed' if any(x['verdict']=='different_bounds' for x in numerical) else 'same_bounds_in_tested_cases'
        elif not r['matched_calculator_ids']:disposition='not_supported_by_calculator'
        else:disposition='matched_but_not_numerically_qualified'
        cleaned={k:v for k,v in r.items() if not k.startswith('_')}
        cleaned.update(disposition=disposition,compared_cases=len(numerical),
            different_bound_cases=sum(x['verdict']=='different_bounds' for x in numerical),
            source_conflicts=sorted({x['source_conflict'] for x in numerical if x.get('source_conflict')}),
            state_conflicts=sorted({x['state_conflict'] for x in relevant if x.get('state_conflict')}),
            average_interval_diagnostics=sum('average_interval_consistency' in x for x in relevant),
            nominal_centers_compared=sum('nominal_center_value' in x for x in relevant),
            nominal_centers_same=sum(x.get('nominal_center_matches_reference') is True for x in relevant),
            nominal_centers_different=sum(x.get('nominal_center_matches_reference') is False for x in relevant),
            average_outside_interval_cases=sum(x.get('average_interval_consistency')=='outside_interval' for x in relevant),
            same_bound_cases=sum(x['verdict']=='same_bounds' for x in numerical),
            untested_reasons=sorted(set(x.get('reason') for x in relevant if x.get('reason'))),
            limitations=sorted(set(t for x in numerical for t in x.get('limitations',[]))),
            qualified_for_runtime=False)
        record_summary.append(cleaned)
    summary={'catalog_total':len(records),'catalog_dispositions':dict(collections.Counter(r['disposition'] for r in record_summary)),
      'calculator_definitions':len(definitions),'calculator_identity_status':dict(collections.Counter(j['status'] for j in joins_report)),
      'reference_cases':len(ref['cases']),'reference_spell_rows':len(rows),
      'numeric_cases':sum(r['verdict']!='not_compared' for r in rows),
      'same_bound_cases':sum(r['verdict']=='same_bounds' for r in rows),
      'different_bound_cases':sum(r['verdict']=='different_bounds' for r in rows),
      'different_bound_cases_within_one_point':sum(r['verdict']=='different_bounds' and max(map(abs,r['delta']))<=1 for r in rows),
      'different_bound_cases_more_than_one_point':sum(r['verdict']=='different_bounds' and max(map(abs,r['delta']))>1 for r in rows),
      'numeric_catalog_records':sum(r['compared_cases']>0 for r in record_summary),
      'pre_harmony_vs_full_harmony_cases':sum(r.get('qualification')=='pre_harmony_vs_full_harmony_not_comparable' for r in rows),
      'nominal_centers_compared':sum('nominal_center_value' in r for r in rows),
      'nominal_centers_same':sum(r.get('nominal_center_matches_reference') is True for r in rows),
      'nominal_centers_different':sum(r.get('nominal_center_matches_reference') is False for r in rows),
      'explicit_retained_source_conflict_cases':sum(r.get('qualification')=='explicit_retained_source_conflict' for r in rows),
      'average_interval_diagnostics':sum('average_interval_consistency' in r for r in rows),
      'average_outside_interval_cases':sum(r.get('average_interval_consistency')=='outside_interval' for r in rows),
      'average_outside_interval_catalog_records':sum(r['average_outside_interval_cases']>0 for r in record_summary),
      'pure_numeric_catalog_records':sum(r['compared_cases']>0 and not r['limitations'] for r in record_summary),
      'not_compared_reasons':dict(pending)}
    document={'schema':'OTERYN_ALL_VOCATION_CALCULATOR_AUDIT/v1','source':ref['source'],
      'reference_sha256':hashlib.sha256(reference_path.read_bytes()).hexdigest(),
      'authoring_revision':readiness['revision'],'candidate_base_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=a.candidate,text=True).strip(),
      'readiness_sha256':hashlib.sha256(a.readiness.read_bytes()).hexdigest(),
      'comparison_scope':'Actual repaired candidate AST; proposals with source conflicts explicit. Raw damage bounds only.',
      'rounding_policy':'math.trunc of existing authored AST bounds; runtime rounding unqualified',
      'limitations':['All 252 catalog records have an explicit disposition; absent healing/support/movement remain untested.',
       'Calculator baseline excludes proficiency, wheel, stances, charms, criticals, target resistance and full shared-build modifiers.',
       'Source is locally executed upstream calculator code at pinned commit, not an official Tibia runtime oracle.',
       'Equal min/max do not establish average, bucket distribution, RNG, cast eligibility or gameplay correctness.',
       'Chain, variants, stage components and Harmony are explicitly limited base-expression comparisons.',
       'No synonym guessing; matches use exact name/carrier or upstream explicit displayName+scope.',
       'This audit makes no replacements or repo mutations; repaired models are explicitly proposed, source conflicts remain visible.'],
      'summary':summary,'profession_matrix':{k:dict(v) for k,v in sorted(matrix.items())},
      'catalog':record_summary,'calculator_coverage':joins_report,'cases':rows}
    save(out/'comparison.json',document)
    save(out/'catalog-dispositions.json',{'summary':summary,'catalog':record_summary})
    save(out/'profession-matrix.json',document['profession_matrix'])
    findings=[]
    for r in record_summary:
        if r['different_bound_cases']:
            differing=[x for x in rows if x['bundle']==r['bundle'] and x['verdict']=='different_bounds']
            findings.append({'bundle':r['bundle'],'spell':r['name'],'different_cases':len(differing),
              'matched_cases':r['same_bound_cases'],'min_delta':min(x['delta'][0] for x in differing),
              'max_delta':max(x['delta'][1] for x in differing),
              'largest_absolute_delta':max(abs(v) for x in differing for v in x['delta']),
              'within_one_point_only':all(max(map(abs,x['delta']))<=1 for x in differing),
              'limitations':r['limitations'],
              'first_difference':differing[0]})
    save(out/'findings.json',{'different_spell_records':len(findings),'findings':findings})
    avg_findings=[]
    for r in record_summary:
        outside=[x for x in rows if x['bundle']==r['bundle'] and x.get('average_interval_consistency')=='outside_interval']
        if outside:
            avg_findings.append({'spell':r['name'],'bundle':r['bundle'],'outside_interval_cases':len(outside),'first_case':outside[0]})
    save(out/'nominal-center-comparison.json',{'scope':'Explicit nominal component compared only; buckets0 does not prove deterministic output or RNG mean. Source dispersion retained.','cases':[r for r in rows if 'nominal_center_value' in r]})
    save(out/'average-interval-findings.json',{'scope':'Necessary consistency only; buckets0 does not prove deterministic damage; does not qualify RNG or full execution','findings':avg_findings})
    lines=['# Wszystkie profesje: porównanie aktualnych formuł Oteryn', '',
      'Porównano rzeczywiste wygenerowane AST z naprawionego kandydata poza repo. Korekty są propozycjami modeli, nie zaakceptowaną prawdą kanoniczną. '
      'Równość granic nie kwalifikuje rozkładu losowego ani wykonania zaklęcia.', '',
      '| Profesja | Wiersze kalkulatora | Porównane granice | Zgodne | Różne | Pominięte |',
      '|---|---:|---:|---:|---:|---:|']
    for v,c in sorted(matrix.items()):lines.append(f"| {v} | {c['calculator_rows']} | {c['compared']} | {c['same_bounds']} | {c['different_bounds']} | {c['not_compared']} |")
    lines+=['',f"Katalog: {len(records)} rekordów. Liczbowo porównano {summary['numeric_catalog_records']}; "
      f"{summary['pure_numeric_catalog_records']} ma pojedynczą formułę bez wykrytych ograniczeń stanu/komponentu. "
      f"Różnice wykryto dla {len(findings)} rekordów.", '',
      'Każdy rekord ma status w catalog-dispositions.json. Healing/support/ruch i inne możliwości '
      'bez odpowiednika kalkulatora są jawnie nieprzetestowane. Blokowane rekordy i nieodwzorowane '
      'stage/wheel/shield/Harmony nie dostały fikcyjnych formuł.', '',
      'Granice istniejących wyrażeń są obcinane math.trunc zgodnie z wcześniejszym probe authoringu. '
      'Średnie kalkulatora zapisano jako dane referencyjne; nie porównano ich z połową przedziału. '
      'RNG, rounding silnika, eligibility, odporności, crit/fatal/charm i pełne modyfikatory buildu '
      'nie są w tym raporcie zakwalifikowane.', '',
      f"{summary['different_bound_cases_more_than_one_point']} przypadków różni się o więcej niż 1 punkt; {summary['different_bound_cases_within_one_point']} o najwyżej 1 punkt. Małe różnice wymagają sprawdzenia precyzji i zaokrągleń, nie świadczą same o złym modelu.", '', '## Różnice', '', '| Zaklęcie | Różne przypadki | Zgodne przypadki | Ograniczenia |', '|---|---:|---:|---|']
    for f in findings:lines.append(f"| {f['spell']} | {f['different_cases']} | {f['matched_cases']} | {', '.join(f['limitations']) or 'surowe granice'} |")
    lines+=['', f"Dla {summary['average_interval_diagnostics']} wierszy bez min/max dodatkowo sprawdzono, czy podana średnia mieści się w przedziale istniejącej formuły. {summary['average_outside_interval_cases']} średnich leży poza nim ({summary['average_outside_interval_catalog_records']} rekordów). To test koniecznej zgodności, nie próba wyliczania średniej jako środka przedziału."]
    lines+=['',f"{summary['pre_harmony_vs_full_harmony_cases']} dodatkowe porównania pre-Harmony spenderów do pełnych5Harmony w kalkulatorze mają różny stan i nie stanowią kwalifikacji ani dowodu błędu bazowej formuły."]
    lines+=['',f"Jawne nominalne komponenty wyodrębnione z rzeczywistych AST: {summary['nominal_centers_compared']} porównań; {summary['nominal_centers_same']} zgodnych, {summary['nominal_centers_different']} różnych. Nie wyliczano średniej z połowy przedziału; buckets=0 nie dowodzi deterministycznych obrażeń."]
    (out/'SUMMARY.md').write_text('\n'.join(lines)+'\n')
    checks={
      'all_252_catalog_entries_classified':len(record_summary)==252,
      'baseline_reference_excludes_context_modifiers':all(not c['normalizedRequest'].get('perks') and not c['normalizedRequest'].get('targets') and not c['normalizedRequest']['stats'].get('stanceIds') and not c['normalizedRequest']['stats'].get('bonus') and not c['normalizedRequest']['stats'].get('critChance') and not c['normalizedRequest']['stats'].get('fatalChance') for c in ref['cases']),
      'each_bundle_unique':len({r['bundle'] for r in record_summary})==252,
      'all_calculator_damage_definitions_identity_matched':all(j['status']=='matched' for j in joins_report if by_id[j['calculator_id']]['spellType'] in ('spell','rune')),
      'all_reference_response_rows_disposed':len(rows)==sum(len(c['response']['spells']) for c in ref['cases']),
      'all_five_professions_present':set(matrix)=={'druid','knight','monk','paladin','sorcerer'},
      'matrix_partition':all(c['calculator_rows']==c['compared']+c['not_compared'] for c in matrix.values()),
      'numerical_partition':summary['numeric_cases']==summary['same_bound_cases']+summary['different_bound_cases'],
      'no_runtime_qualified_records':all(r['qualified_for_runtime'] is False for r in record_summary),
      'no_stage_above_zero_claimed_as_pure_comparison':all(r.get('calculator_stage',0) in (None,0) for r in rows if r['verdict']!='not_compared'),
    }
    save(out/'CHECKS.json',{'passed':all(checks.values()),'checks':checks})
    assert all(checks.values()),checks
    print(json.dumps(summary,ensure_ascii=False))

if __name__=='__main__':main()
