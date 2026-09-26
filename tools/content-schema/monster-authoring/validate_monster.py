"""Structural/semantic validation of the Monster authoring schema candidate v1, never runtime activation."""
import argparse
import json
import re
from decimal import Decimal
from fractions import Fraction
from math import gcd
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parent
SCHEMA_NAMES = ('monster.schema.json','monster-dependencies.schema.json','monster-import-readiness.schema.json')
SCHEMAS = {name:json.loads((ROOT/name).read_text(encoding='utf-8'),parse_float=Decimal) for name in SCHEMA_NAMES}
REGISTRY = Registry().with_resources((s['$id'],Resource.from_contents(s)) for s in SCHEMAS.values())
FAMILIES = {'creature':'Creature','behavior':'Behavior','presentation':'Presentation','loot':'Loot',
            'abilities':'Ability','effects':'Effect','formulas':'Formula','documents':'Document','items':'Item','loot_tables':'Loot'}

def read(path): return json.loads(Path(path).read_text(encoding='utf-8'),parse_float=Decimal)
def exact_numbers(value):
    if isinstance(value,float):return Decimal(str(value))
    if isinstance(value,dict):return {k:exact_numbers(v) for k,v in value.items()}
    if isinstance(value,list):return [exact_numbers(v) for v in value]
    return value
def percent_to_ppm(value):
    if isinstance(value,bool):raise ValueError('a boolean is not a chance')
    decimal=Decimal(str(value))
    if not decimal.is_finite() or not 0<=decimal<=100:raise ValueError('chance must be between 0 and 100 percent')
    # Integer arithmetic keeps every source digit, independent of Decimal context.
    numerator,denominator=decimal.as_integer_ratio()
    ppm,remainder=divmod(numerator*10000,denominator)
    if remainder:raise ValueError('chance precision is finer than 0.0001 percent')
    return ppm
PERCENT_FIELDS={'critical_chance_percent','chance_percent','static_attack_chance_percent','probability_percent'}
def pointer(path): return '/'+'/'.join(str(k).replace('~','~0').replace('/','~1') for k in path)
def walk(value,path=()):
    yield path,value
    if isinstance(value,dict):
        for k,v in value.items():yield from walk(v,path+(k,))
    elif isinstance(value,list):
        for i,v in enumerate(value):yield from walk(v,path+(i,))
def structural(name,data):
    validator=Draft202012Validator(SCHEMAS[name],registry=REGISTRY)
    return [f'{name}{pointer(e.absolute_path)}: {e.message}' for e in sorted(validator.iter_errors(exact_numbers(data)),key=lambda e:str(list(e.absolute_path)))]
def ident(family,identity):return family,identity['key'],identity['revision']

def validate(monster,deps,catalog=None,manifest=None):
    errors=structural('monster.schema.json',monster)+structural('monster-dependencies.schema.json',deps)
    if manifest is not None:errors+=structural('monster-import-readiness.schema.json',manifest)
    if errors:return errors
    catalog=catalog or {'definitions':[],'assets':[]}
    if set(catalog)-{'definitions','assets'}:
        errors.append('catalog: unknown fields')
    if not isinstance(catalog.get('definitions',[]),list) or not isinstance(catalog.get('assets',[]),list):
        return errors+['catalog: definitions and assets must be arrays']
    records={};local={};assets=set()
    for asset in catalog.get('assets',[]):
        if not isinstance(asset,str): errors.append('catalog: asset binding must be a string');continue
        asset_schema={'$ref':SCHEMAS['monster.schema.json']['$id']+'#/$defs/key'}
        if not Draft202012Validator(asset_schema,registry=REGISTRY).is_valid(asset):errors.append('catalog: invalid asset binding key '+asset)
        if asset in assets:errors.append('catalog: duplicate asset '+asset)
        assets.add(asset)
    for section,family in FAMILIES.items():
        values=[monster[section]] if section in monster else deps.get(section,[])
        for value in values:
            key=ident(family,value['identity'])
            if key in records:errors.append('duplicate local definition '+repr(key))
            records[key]=value;local[key]=value
    allowed_families=set(FAMILIES.values())|{'Interaction','Encounter'}
    for reference in catalog.get('definitions',[]):
        if not isinstance(reference,dict) or set(reference)!={'family','key','revision'}:
            errors.append('catalog: malformed definition reference');continue
        if not isinstance(reference['family'],str) or reference['family'] not in allowed_families:
            errors.append('catalog: unknown family');continue
        errs=structural_ref(reference)
        if errs:errors.extend(errs);continue
        key=ident(reference['family'],reference)
        if key in records:errors.append('duplicate catalog/local definition '+repr(key))
        records[key]=None
    for label,data in (('monster',monster),('dependencies',deps)):
        for path,value in walk(data):
            location=label+pointer(path)
            if isinstance(value,dict) and set(value)=={'family','key','revision'}:
                key=ident(value['family'],value)
                if key not in records:errors.append(location+': unresolved exact definition '+repr(key))
            if path and str(path[-1]).endswith(('asset_binding','palette_binding','color_binding')) and isinstance(value,str):
                if value not in assets:errors.append(location+': unresolved asset '+value)
            if path and path[-1] in PERCENT_FIELDS and not isinstance(value,bool) and isinstance(value,(int,float,Decimal)):
                try:percent_to_ppm(value)
                except ValueError as exc:errors.append(location+': '+str(exc))
            if isinstance(value,dict) and set(value)=={'numerator','denominator'}:
                if gcd(abs(value['numerator']),value['denominator'])!=1:
                    errors.append(location+': ratio must be in lowest terms (zero is 0/1)')
                fraction=Fraction(value['numerator'],value['denominator'])
                parent=str(path[-1]) if path else ''
                if parent in ('mitigation_percent','reduction_percent') and fraction>100:
                    errors.append(location+': reduction/mitigation must not exceed 100 percent')
    c=monster['creature'];b=monster['behavior'];p=monster['presentation']
    if c['stats']['initial_health']>c['stats']['max_health']:errors.append('creature/stats: initial_health exceeds max_health')
    if b['targeting']['flee_health']>c['stats']['max_health']:errors.append('behavior/targeting: flee_health exceeds max_health')
    for group in ('resistances','damage_reflection','healing_from_damage'):
        unique(c[group],'damage_type','creature/'+group,errors)
    if 'bestiary' in c:
        increasing(c['bestiary']['kill_thresholds'],'creature/bestiary/kill_thresholds',errors)
    if 'bosstiary' in c:
        v=c['bosstiary'];increasing([v[k] for k in ('prowess_kills','expertise_kills','mastery_kills')],'creature/bosstiary',errors)
    weights=b['targeting'].get('strategy_weights')
    if weights and sum(weights.values())==0:errors.append('behavior/targeting/strategy_weights: at least one weight must be positive')
    summons=b.get('summons')
    if summons and any(x['count']>summons['max_summons'] for x in summons['entries']):errors.append('behavior/summons: count exceeds max_summons')
    eligibility=c.get('system_eligibility',{})
    if eligibility.get('exclusive_prey') and not eligibility.get('prey'):errors.append('creature/system_eligibility: exclusive_prey requires prey')
    if 'reward_encounter' in c and not eligibility.get('reward_boss'):errors.append('creature/reward_encounter: requires reward_boss')
    for property_name,section in (('presentation',p),('behavior',b)):
        if ident(FAMILIES[property_name],c[property_name])!=ident(FAMILIES[property_name],section['identity']):
            errors.append('creature/'+property_name+': must select this bundle definition exactly')
    if 'loot' in monster:
        if 'loot' not in c or ident('Loot',c['loot'])!=ident('Loot',monster['loot']['identity']):errors.append('creature/loot: must select this bundle Loot exactly')
    cues={}
    for binding in p['audio']['event_bindings']:
        cue=binding['cue_id']
        if cue in cues and cues[cue]!=binding['asset_binding']:errors.append('presentation/audio: cue has conflicting asset bindings '+cue)
        cues[cue]=binding['asset_binding']
    for cue in b.get('periodic_audio',{}).get('cue_ids',[]):
        if not any(x['cue_id']==cue and x['event']=='periodic' for x in p['audio']['event_bindings']):errors.append('behavior/periodic_audio: missing periodic cue binding '+cue)
    for collection in ('palette_bindings','visual_effect_bindings'):
        unique(p['appearance'][collection],'slot','presentation/appearance/'+collection,errors)
    light=p['light']
    if light['level']>0 and 'color_binding' not in light:errors.append('presentation/light: nonzero light requires color_binding')
    for ability in deps['abilities']:
        for cue in ability.get('audio',{}).values():
            if cue not in cues:errors.append('ability/audio: unresolved cue '+cue)
    for formula in deps['formulas']:
        if 'magnitude' in formula and formula['magnitude']['minimum']>formula['magnitude']['maximum']:errors.append('formula/magnitude: minimum exceeds maximum')
        if 'speed' in formula:
            v=formula['speed'];minimum=v['minimum_multiplier'];maximum=v['maximum_multiplier']
            if Fraction(minimum['numerator'],minimum['denominator'])>Fraction(maximum['numerator'],maximum['denominator']):errors.append('formula/speed: minimum multiplier exceeds maximum')
    for effect in deps['effects']:
        damage=effect.get('condition',{}).get('damage_over_time',{}).get('total_damage_range')
        if damage and damage['minimum']>damage['maximum']:errors.append('effect/condition/damage_over_time/total_damage_range: minimum exceeds maximum')
    for key,value in local.items():
        if key[0]=='Loot':
            for entry in value['entries']:
                if entry['min_count']>entry['max_count']:errors.append('loot/entries: min_count exceeds max_count')
                child=entry.get('contents_loot')
                if child:
                    item=local.get(ident('Item',entry['item']))
                    if item is None:errors.append('loot/contents_loot: local Item payload needed to prove container capability')
                    elif 'container' not in item:errors.append('loot/contents_loot: selected Item is not a container')
    for item in deps['items']:
        corpse=ident('Item',item['identity'])==ident('Item',c['corpse_item']) if 'corpse_item' in c else False
        if corpse and not item['classification']['is_corpse']:errors.append('creature/corpse_item: selected Item is not a corpse')
    if 'corpse_item' in c and ident('Item',c['corpse_item']) not in local:errors.append('creature/corpse_item: include local corpse payload to verify its properties')
    cycle_check(local,'Item',lambda v:[v['temporal']['decay_target']] if 'decay_target' in v['temporal'] else [],'corpse decay',errors)
    cycle_check(local,'Loot',lambda v:[e['contents_loot'] for e in v['entries'] if 'contents_loot' in e],'nested loot',errors)
    if manifest is not None:
        for entry in manifest['entries']:
            if entry['source_index']>=len(manifest['sources']):errors.append('manifest: invalid source index')
            status=entry['status'];kind=entry['kind']
            if status in ('unsupported_source_field','unresolved_semantics','unresolved_dependency','partial_text'):
                errors.append('manifest: '+status+' '+entry['source_file']+':'+str(entry['source_line'])+' '+entry['source_field'])
            if kind=='script' and status not in ('resolved_native_behavior','approved_omission'):
                errors.append('manifest: custom script requires explicit native behavior resolution or approved omission')
            if kind=='original_text' and status not in ('mapped','approved_omission'):
                errors.append('manifest: original text completeness has not been resolved')
            if status in ('mapped','resolved_native_behavior'):
                destination=entry['destination']
                try:resolved=resolve_pointer({'monster':monster,'dependencies':deps},destination)
                except (KeyError,IndexError,ValueError,TypeError):errors.append('manifest: missing destination '+destination);continue
                if resolved is None:errors.append('manifest: null destination '+destination)
                if kind=='script' and not destination.startswith(('/monster/behavior/','/dependencies/abilities/','/dependencies/effects/')):
                    errors.append('manifest: script must resolve to a behavior, ability, or effect')
    return errors

def structural_ref(reference):
    schema={'$ref':SCHEMAS['monster.schema.json']['$id']+'#/$defs/'+reference['family']+'Ref'}
    return ['catalog: '+e.message for e in Draft202012Validator(schema,registry=REGISTRY).iter_errors(reference)]
def unique(values,field,label,errors):
    keys=[x[field] for x in values]
    if len(set(keys))!=len(keys):errors.append(label+': duplicate '+field)
def increasing(values,label,errors):
    if any(a>=b for a,b in zip(values,values[1:])):errors.append(label+': thresholds must be strictly increasing')
def cycle_check(local,family,edges,label,errors):
    visiting=set();done=set()
    def visit(key):
        if key in visiting:errors.append(label+': cycle at '+repr(key));return
        if key in done or key not in local:return
        visiting.add(key)
        for reference in edges(local[key]):visit(ident(family,reference))
        visiting.remove(key);done.add(key)
    for key in local:
        if key[0]==family:visit(key)
def resolve_pointer(document,p):
    if not isinstance(p,str):raise ValueError('expected a JSON pointer string')
    if p=='':return document
    if not p.startswith('/'):raise ValueError('expected absolute JSON pointer')
    value=document
    for part in p[1:].split('/'):
        if re.search(r'~(?:[^01]|$)',part):raise ValueError('invalid JSON pointer escape')
        key=part.replace('~1','/').replace('~0','~')
        if isinstance(value,list):
            # RFC 6901: ASCII digits, no sign/leading zeros; '-' has no value.
            if re.fullmatch(r'0|[1-9][0-9]*',key) is None:
                raise ValueError('invalid JSON pointer array index')
            value=value[int(key)]
        elif isinstance(value,dict):
            value=value[key]
        else:
            raise ValueError('JSON pointer cannot traverse a scalar')
    return value

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('monster');parser.add_argument('dependencies');parser.add_argument('--catalog');parser.add_argument('--manifest')
    args=parser.parse_args()
    issues=validate(read(args.monster),read(args.dependencies),read(args.catalog) if args.catalog else None,read(args.manifest) if args.manifest else None)
    print(json.dumps({'valid':not issues,'scope':'local authoring structure and declared dependency closure; no runtime qualification',
        'import_manifest_checked':args.manifest is not None,
        'declared_manifest_resolved':args.manifest is not None and not issues,
        'source_coverage_proven':False,
        'runtime_qualified':False,'errors':issues},ensure_ascii=False,indent=2))
    raise SystemExit(bool(issues))
