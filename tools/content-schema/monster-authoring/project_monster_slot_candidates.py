"""Actual v1 target definitions for donor Combat pieces; exact full-slot holds retained."""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace
import tarfile
import canary_batch as cb
import spell_scripts as ss
from source_monster_inline_semantics import read

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
ARCHIVE = ROOT / 'docs/reference/spells/r28-source-closure/monster-source-package.tar.gz'
R51 = ROOT / 'docs/reference/spells/r51-source-closure/source-monster-slot-semantics.json.gz'
SCHEMA = HERE / 'monster.schema.json'


def converter(donor):
    conv = cb.Converter.__new__(cb.Converter)
    definitions = read(donor, 'src/creatures/creatures_definitions.hpp').decode()
    enums = {enum: ss.enum_values(definitions, enum) for enum in ss.ENUMS}
    effect_text = read(donor, cb.EFFECT_CONSTANTS).decode()
    conv.magic_effects = ss.enum_values(effect_text, 'MagicEffectClasses')
    conv.missiles = ss.enum_values(effect_text, 'ShootType_t')
    conv.item_ids = ss.enum_values(effect_text, 'ItemID_t')
    conv.magic_effect_names = {v:k for k,v in conv.magic_effects.items()}
    conv.missile_names = {v:k for k,v in conv.missiles.items()}
    conv.spell_scripts = SimpleNamespace(enums=enums)
    conv.pending_definitions = set()
    # Species facts are common across exact donor variants; Crystal supplements the common cache.
    common=json.loads((HERE/'samples/wiki-population-2026-09-27.json').read_text())
    extra=json.loads((HERE/'samples/wiki-population-crystal-00ce02a5-2026-09-27.json').read_text())
    conv.wiki={r['monster']:r for r in common['monsters']}
    if donor=='crystal':conv.wiki.update({r['monster']:r for r in extra['monsters']})
    return conv


def load():
    if hashlib.sha256(ARCHIVE.read_bytes()).hexdigest() != '2bad4b500349bf6578a2df1b191420ff1efef62fafe1f822f9b26f5a9bb1c75a':
        raise ValueError('immutable R28 source archive differs')
    packet = json.loads(gzip.decompress(R51.read_bytes()))
    registrations = {}
    with tarfile.open(ARCHIVE) as archive:
        for donor in ['canary','crystal']:
            for record in json.load(archive.extractfile('registered-spells/'+donor+'.json')):
                key = donor, record['provenance']['path'], record['name']
                if key in registrations:
                    raise ValueError('duplicate exact donor registration')
                registrations[key] = record
        ally_refs={}
        for slug in ('elite_pirat','1st_mate_ratticus','mister_catkiller'):
            member='converted-bundles/crystal/data-global/monster/quests/a_pirates_tail_quest/'+slug+'/monster.json'
            identity=json.load(archive.extractfile(member))['creature']['identity']
            ally_refs[slug]={'family':'Creature',**identity}
        registrations['ratmiral_ally_refs']=ally_refs
    return packet, registrations


def target_fragment(conv, combat, key, flags, donor):
    original = copy.deepcopy(combat)
    source_callbacks = copy.deepcopy(original.get('callbacks', {}))
    subids=[args[1] for c in original.get('conditions',[]) for method,args in c.get('calls',[]) if method=='setParameter' and args and args[0]=='CONDITION_PARAM_SUBID']
    # Only the declared base Combat is projected here; caller retains callback/controller holds.
    original['callbacks'] = {k:v for k,v in original.get('callbacks', {}).items()
                             if k in ('CALLBACK_PARAM_LEVELMAGICVALUE','CALLBACK_PARAM_SKILLVALUE')}
    formula = original.get('formula')
    if formula:
        if len(formula) != 5 or formula[0] != 'COMBAT_FORMULA_DAMAGE':
            raise cb.SpellUnresolved('nonliteral source Combat formula requires separate target operator')
        low, high = int(formula[1]), int(formula[3])
        healing = conv.engine_params(original.get('param_calls',[]),[]).get('COMBAT_PARAM_TYPE') == 'COMBAT_HEALING'
        if (healing and (low < 0 or high < 0)) or (not healing and (low > 0 or high > 0)):
            raise cb.SpellUnresolved('positive/mixed signed COMBAT_FORMULA_DAMAGE requires sign-aware health operator')
        original['formula'] = None
    else:
        low = high = None
    deps = {'abilities':[],'effects':[],'formulas':[]}
    notes = []
    geometry = {'needs_target':bool(flags.get('needTarget',[False])[0] or flags.get('needCasterTargetOrDirection',[False])[0]),
                'needs_direction':bool(flags.get('needDirection',[False])[0])}
    conv.combat_ability(key, original, geometry, int(flags.get('range',[0])[0] or 0), deps, lambda binding:binding, notes)
    if formula:
        formula_key = key+'/static-source-formula'
        deps['formulas'] = [f for f in deps['formulas'] if f['identity']['key'] != cb.CASTER_MAGNITUDE]
        deps['formulas'].append({'identity':cb.ident(formula_key),'kind':'range',
                                 'magnitude':{'minimum':min(abs(low),abs(high)),'maximum':max(abs(low),abs(high))}})
        for effect in deps['effects']:
            if effect.get('formula',{}).get('key') == cb.CASTER_MAGNITUDE:
                effect['formula'] = cb.ref('Formula',formula_key)
    # Converter's provisional Canary presentation namespace is made donor scoped.
    def namespace(value):
        if isinstance(value,dict):return {k:namespace(v) for k,v in value.items()}
        if isinstance(value,list):return [namespace(v) for v in value]
        if isinstance(value,str) and value.startswith('canary.appearance:') and donor=='crystal':
            return 'crystal.appearance:'+value[len('canary.appearance:'):]
        return value
    return namespace(deps), {'source_callbacks_not_projected':source_callbacks,
                            'source_formula':formula,'conversion_notes':notes,
                            'projection_scope':'declared_base_combat_only','source_condition_subids_not_projected':subids}


def classify_slot(slot, source_packet, registrations, converters):
    identity = slot['slot_identity']
    suffix = hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest()[:16]
    out = {'slot_identity':identity,'source':slot['source'],'monster':slot['monster'],
           'source_parameters':slot['source_parameters'],'original_slot_sha256':slot['original_slot_sha256'],
           'status':'BLOCKED','full_slot_projection_complete':False,'runtime_activation':False,
           'native_execution_qualified':False,'source_numeric_equivalence':False,'source_type_equivalence':False,
           'canonical_normalization':None,'target_schedule':None,'target_fragments':[],'blockers':[]}
    if slot['source_program_index'] is None:
        inline=slot['inline_semantics']
        out['normalized_mechanics']=inline['normalized_combat']
        conv=converters[slot['source']];conv.current_slug=cb.slug(slot['monster'])
        damage,wiki_note=conv.undefined_damage_from_wiki(slot['source_parameters'])
        if damage is not None:
            deps={'abilities':[],'effects':[],'formulas':[]};target_base=slot['source']+':{}/source-slot/'+suffix
            result=conv.spell(slot['source_parameters'],target_base,deps,lambda binding:binding)
            if result[0] in ('UNRESOLVED','OMIT'):
                raise ValueError('accepted D25 match failed actual target conversion')
            if slot['source']=='crystal':
                encoded=json.dumps(deps).replace('canary.appearance:','crystal.appearance:');deps=json.loads(encoded)
            validate_definitions(deps)
            out['target_fragments']=[{'source_combat_index':0,'definitions':deps,'source_callbacks_not_projected':{},
                                      'source_formula':None,'conversion_notes':[wiki_note,'Explicit D25 Game normalization; raw undefined donor type retained in R51 and source_parameters; source_numeric/type equivalence false.'],
                                      'projection_scope':'accepted_d25_canonical_normalization','source_condition_subids_not_projected':[],'full_slot_candidate':True,'runtime_activation':False}]
            wiki_record=conv.wiki[conv.current_slug]
            policy=ROOT/'docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md'
            out['canonical_normalization']={'decision':'D25','damage_type':damage,'policy_path':str(policy.relative_to(ROOT)),
                                            'policy_sha256':hashlib.sha256(policy.read_bytes()).hexdigest(),
                                            'wiki_page_id':wiki_record['page_id'],'wiki_revision_id':wiki_record['cut_revision_id'],
                                            'wiki_record_sha256':hashlib.sha256(json.dumps(wiki_record,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                                            'read_method':'existing_local_cached_wiki_facts'}
            out['status']='SOURCE_SCHEMA_VALID';out['full_slot_projection_complete']=True
            out['blockers']=[{'kind':'canonical_normalization_runtime_unqualified',
                             'detail':'D25 projects wiki element '+damage+' instead of donor undefined type. Native typed damage/drain operator and signed source equivalence remain unqualified.',
                             'source_values':[]}]
            return out
        out['blockers']=[{'kind':'undefined_signed_health_operator_missing',
                         'detail':'Existing target damage/heal operators cannot preserve donor COMBAT_UNDEFINEDDAMAGE signed health path. D25: '+wiki_note,
                         'source_values':[inline['normalized_combat']['min_combat_value'],inline['normalized_combat']['max_combat_value']]}]
        return out
    program=source_packet['source_programs'][slot['source_program_index']]
    ast=program['source_syntax']; key=slot['source'],ast['path'],str(slot['source_parameters']['name']).lower()
    record=registrations.get(key)
    if record is None:raise ValueError('missing exact registered source capture')
    if record['provenance']['sha256'] != ast['source_sha256']:raise ValueError('source capture/AST pin mismatch')
    info=record['conversion'];combats=info.get('reference_combats',{})
    out['normalized_mechanics']={'registered_name':record['name'],'source_path':ast['path'],
                                  'source_sha256':ast['source_sha256'],'combat_count':len(combats),
                                  'registration_flags':info.get('spell_calls',{})}
    if (record['name'] == 'ratmiral ball' and ast['source_sha256'] == '098f7d48e31bea3f38b3a8089432bcc9529b1449a60e182d1a3274eed658cd7c'):
        combat = combats['0']
        flags = info.get('spell_calls', {})
        target_key = slot['source']+':ability/source-slot/'+suffix+'/named-top-tile-heal'
        deps,scope = target_fragment(converters[slot['source']],combat,target_key,flags,slot['source'])
        if slot['source_parameters'].get('minDamage',0) != 0 or slot['source_parameters'].get('maxDamage',0) != 0:
            raise ValueError('Ratmiral base heal is no longer zero')
        effect = deps['effects'][0]
        # Zero base health still presents MAGIC_RED, then source TARGETTILE callback heals named allies.
        visual = effect.get('presentation',{})
        effect.clear();effect.update(identity=cb.ident(target_key+'/effect'),operation='presentation_only',presentation=visual)
        formula_key = target_key+'/ally-heal-range'
        deps['formulas'] = [{'identity':cb.ident(formula_key),'kind':'range','magnitude':{'minimum':0,'maximum':1000}}]
        callback_key = target_key+'/ally-heal'
        named = ['elite_pirat','1st_mate_ratticus','mister_catkiller']
        deps['effects'].append({'identity':cb.ident(callback_key),'operation':'heal','damage_type':'healing',
                               'formula':cb.ref('Formula',formula_key),
                               'affects':{'kind':'named_creatures','creatures':[registrations['ratmiral_ally_refs'][name] for name in named],
                                          'top_creature_only':True,'excludes_caster_name':False,'includes_caster':True}})
        deps['abilities'][0]['effects'].append(cb.ref('Effect',callback_key))
        validate_definitions(deps)
        out['target_fragments'] = [{'source_combat_index':0,'definitions':deps,
                                    'source_callbacks_not_projected':{},'source_formula':None,
                                    'conversion_notes':['Exact pinned RatmiralBall source: top creature, monster/name filter, independent math.random(0,1000); param.removeCaster is never consumed.'],
                                    'projection_scope':'complete_named_top_tile_heal','source_condition_subids_not_projected':[],
                                    'full_slot_candidate':True,'runtime_activation':False}]
        out['status']='SOURCE_SCHEMA_VALID';out['full_slot_projection_complete']=True
        out['blockers']=[{'kind':'named_top_tile_healing_consumer_unqualified',
                          'detail':'Source data is projected into existing named_creatures/top_creature_only authoring fields; native filter, per-target random provider and caster self-area anchoring remain unqualified.',
                          'source_values':[0,1000]}]
        return out
    for index,combat in sorted(combats.items(),key=lambda kv:int(kv[0])):
        target_key=slot['source']+':ability/source-slot/'+suffix+'/combat-'+index
        try:
            deps,scope=target_fragment(converters[slot['source']],combat,target_key,
                                       info.get('spell_calls',{}),slot['source'])
            validate_definitions(deps)
            out['target_fragments'].append({'source_combat_index':int(index),'definitions':deps,**scope,
                                            'full_slot_candidate':False,'runtime_activation':False})
        except (cb.SpellUnresolved,ValueError) as error:
            out['blockers'].append({'kind':'base_combat_target_field_missing','detail':'Combat '+index+': '+str(error),
                                    'source_values':[]})
    if not combats:
        out['blockers'].append({'kind':'world_or_caster_state_controller_missing',
                                'detail':'No declared Combat; cast mutates world/caster state through external helpers.',
                                'source_values':[]})
    funcs={f['node_ref']:f['name'] for f in program['function_programs']}
    cast_refs={r for f in program['function_programs'] if f['name']=='spell.onCastSpell' for r in f['statement_refs']}
    callback_calls=[op['call_identity'] for op in program['external_operations']
                    if op['context_function_ref'] in funcs and funcs[op['context_function_ref']]!='spell.onCastSpell']
    out['blockers'].append({'kind':'whole_cast_controller_not_projected',
                            'detail':'Exact cast statements '+','.join(map(str,sorted(cast_refs)))+'; target fragments do not project guards, delayed stages, return semantics or world mutations.',
                            'source_values':[]})
    for operation in program['mechanic_operations']:
        if operation['category'] not in ('scheduled_callback','world_teleport','world_transform','world_remove',
                                         'monster_spawn','zone_combat_helper','probability_draw','target_assignment'):
            continue
        values = [a['symbol'] or a['numeric_literal'] or a['string_literal']
                  for a in operation['arguments']]
        out['blockers'].append({'kind':'source_controller_operator_not_projected',
                                'detail':operation['category']+': '+str(operation['call_identity'])+' arguments '+str(values),
                                'source_values':[]})
    if callback_calls:
        out['blockers'].append({'kind':'source_callback_behavior_not_projected',
                               'detail':', '.join(sorted(set(str(c) for c in callback_calls))), 'source_values':[]})
    if any(c.get('source_callbacks_not_projected') for c in out['target_fragments']):
        out['blockers'].append({'kind':'registered_target_or_chain_callback_not_projected',
                               'detail':'Combat callbacks require their target/filter/native behavior before the fragment is a complete slot.',
                               'source_values':[]})
    subids=[v for f in out['target_fragments'] for v in f['source_condition_subids_not_projected']]
    if subids:out['blockers'].append({'kind':'condition_subid_target_field_missing','detail':'Existing condition target schema does not preserve source CONDITION_PARAM_SUBID group keys.','source_values':subids})
    if out['target_fragments']:out['status']='PARTIAL_TARGET_DEFINITIONS'
    return out


def validate_definitions(deps):
    import jsonschema
    schema=json.loads(SCHEMA.read_text())
    for family,definition in [('abilities','ability'),('effects','effect'),('formulas','formula')]:
        for value in deps[family]:
            jsonschema.Draft202012Validator({'$ref':'#/$defs/'+definition,'$defs':schema['$defs']}).validate(value)


def build():
    source_packet, registrations=load()
    converters={d:converter(d) for d in ('canary','crystal')}
    slots=[classify_slot(slot,source_packet,registrations,converters) for slot in source_packet['slots']]
    def retag(value, donor):
        if isinstance(value, dict):
            if value.get('family')=='Creature':return value
            return {k:('source-r54-'+donor+'-'+('04b83b51' if donor=='canary' else '00ce02a5')
                       if k=='revision' else retag(v,donor)) for k,v in value.items()}
        if isinstance(value, list):return [retag(v,donor) for v in value]
        if value==cb.CASTER_MAGNITUDE and donor=='crystal':return 'crystal:formula/caster-magnitude'
        return value
    for slot in slots:
        for fragment in slot['target_fragments']:
            fragment['definitions']=retag(fragment['definitions'],slot['source'])
        if slot['full_slot_projection_complete']:
            ability=slot['target_fragments'][0]['definitions']['abilities'][0]['identity']
            slot['target_schedule']={'ability':{'family':'Ability',**ability},
                                     'interval_ms':slot['source_parameters'].get('interval',2000),
                                     'chance_percent':min(slot['source_parameters'].get('chance',100),100)}

    return {'schema':'OTERYN_MONSTER_TARGET_FRAGMENT_PROJECTION/v1','slot_count':175,'slots':slots,
            'partial_target_slot_count':sum(bool(s['target_fragments']) and not s['full_slot_projection_complete'] for s in slots),
            'target_definition_slot_count':sum(bool(s['target_fragments']) for s in slots),
            'blocked_without_target_count':sum(not s['target_fragments'] for s in slots),
            'full_slot_candidate_count':sum(s['full_slot_projection_complete'] for s in slots),'runtime_activation':False,'native_admission':False,
            'canonical_selection_changed':False,'external_sources_used':True,'fresh_external_reads':False}


def validate(packet):
    import jsonschema
    schema=json.loads((HERE/'project-monster-slot-candidates.schema.json').read_text())
    from referencing import Registry, Resource
    target_schema=json.loads(SCHEMA.read_text())
    registry=Registry().with_resource(target_schema['$id'],Resource.from_contents(target_schema))
    jsonschema.Draft202012Validator(schema,registry=registry).validate(packet)
    source,_=load(); originals={json.dumps(s['slot_identity'],sort_keys=True):s for s in source['slots']}
    seen=set()
    for slot in packet['slots']:
        key=json.dumps(slot['slot_identity'],sort_keys=True)
        if key in seen or key not in originals:raise ValueError('slot identity population changed')
        seen.add(key)
        if slot['source_parameters']!=originals[key]['source_parameters']:raise ValueError('raw donor parameters changed')
        for fragment in slot['target_fragments']:validate_definitions(fragment['definitions'])
    if len(seen)!=175:raise ValueError('all175 required')
    if packet != build():raise ValueError('actual target definitions/statuses differ from pinned source projection')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True);args=parser.parse_args()
    packet=build();validate(packet);args.out.parent.mkdir(parents=True,exist_ok=True)
    args.out.write_bytes(gzip.compress((json.dumps(packet,sort_keys=True,separators=(',',':'))+'\n').encode(),mtime=0))


if __name__=='__main__':main()
