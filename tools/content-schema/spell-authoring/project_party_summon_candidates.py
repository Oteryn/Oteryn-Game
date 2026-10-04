"""R57: source-correct party/summon proposals, with strict native-reader holds."""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
import re
from types import SimpleNamespace
import import_source_player_bundles as base
import native_companions
import validate_spell

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
R28=ROOT/'docs/reference/spells/r28-source-closure/player-source-bundles'
WORKLIST=ROOT/'docs/reference/spells/blocked-173-worklist.json'
PINS={'canary':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystal':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
NAMES={'summon creature','animate dead rune','convince creature rune'}


def sha(raw):return hashlib.sha256(raw).hexdigest()


def records():
    work=json.loads(WORKLIST.read_text())
    rows=next(value for value in work.values() if isinstance(value,list))
    rows=[r for r in rows if r.get('lane') in ('party','summons')]
    facts={r['registration_key']:r for r in map(json.loads,gzip.decompress((R28/'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    if len(rows)!=28:raise ValueError('exact11party17summon population required')
    return rows,facts


def condition_target(raw, donor, key):
    import canary_batch, spell_scripts
    source=base.source_file(Path('/workspace/spell-sources')/donor,PINS[donor],'src/creatures/creatures_definitions.hpp').decode()
    enums={name:spell_scripts.enum_values(source,name) for name in spell_scripts.ENUMS}
    converter=canary_batch.Converter.__new__(canary_batch.Converter)
    converter.spell_scripts=SimpleNamespace(enums=enums)
    deps={'abilities':[],'effects':[],'formulas':[]};notes=[]
    for index,condition in enumerate(raw.get('conditions',[])):
        body=converter.script_condition(copy.deepcopy(condition),deps,key+'/formula-'+str(index),notes)
        if body:
            effect={'identity':{'key':key+'/effect-'+str(index),'revision':'source-player-r57'},**body}
            validate_spell.Draft202012Validator({'$ref':'#/$defs/effect','$defs':validate_spell.SCHEMAS['monster.schema.json']['$defs']}).validate(effect)
            deps['effects'].append(effect)
    return deps,notes


CPP_ROOTS={
 'src/lua/functions/core/game/game_functions.cpp':['GameFunctions::luaGameCreateMonster'],
 'src/lua/functions/creatures/creature_functions.cpp':['CreatureFunctions::luaCreatureSetMaster'],
 'src/creatures/creature.cpp':['Creature::setMaster'],
 'src/lua/functions/creatures/monster/monster_type_functions.cpp':['MonsterTypeFunctions::luaMonsterTypeManaCost','MonsterTypeFunctions::luaMonsterTypeIsSummonable','MonsterTypeFunctions::luaMonsterTypeIsConvinceable']}
POLICY='docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md'


def closure_proofs(donor):
    repo=Path('/workspace/spell-sources')/donor;old=native_companions.PINS[donor];proofs=[]
    for path,symbols in CPP_ROOTS.items():
        previous=base.source_file(repo,old,path);current=base.source_file(repo,PINS[donor],path)
        for symbol in symbols:
            before=base.source_cpp_function(previous,symbol);now=base.source_cpp_function(current,symbol)
            if before!=now:raise ValueError('accepted acquisition root changed: '+symbol)
            proofs.append({'path':path,'symbol':symbol,'old_revision':old,'current_revision':PINS[donor],
                           'old_file_sha256':sha(previous),'current_file_sha256':sha(current),
                           'old_function_sha256':sha(before),'current_function_sha256':sha(now),'function_equal':True})
    path='data/libs/functions/creature.lua'
    previous=base.source_file(repo,old,path);current=base.source_file(repo,PINS[donor],path)
    def helper(data):
        matches=re.findall(rb'function Creature:setSummon\(monster\).*?\nend',data,re.S)
        if len(matches)!=1:raise ValueError('unique source setSummon helper required')
        return matches[0]
    before=helper(previous);now=helper(current)
    proofs.append({'path':path,'symbol':'Creature:setSummon','old_revision':old,'current_revision':PINS[donor],
                   'old_file_sha256':sha(previous),'current_file_sha256':sha(current),
                   'old_function_sha256':sha(before),'current_function_sha256':sha(now),'function_equal':before==now,
                   'source_operations':['Monster(monster)','reject_missing_monster','setMaster(self,true)','setTarget(self.attackedCreature)','return_true']})
    return proofs


def native_proposal(fact, source_text, header, donor, key):
    raw=fact['source_callback_facts'];name=raw['name'].casefold()
    spec=native_companions.SPECS[name]['sources'][donor]
    if raw['file']!=spec['path'] or sha(source_text)!=spec['sha256'] or raw['blob']!=spec['blob']:
        raise ValueError('acquire wrapper differs from exact accepted source recipe')
    descriptor=native_companions._recipe(name,donor)
    defaults,proofs=base.default_fields(Path('/workspace/spell-sources')/donor,PINS[donor])
    spell=base.fill_header(header,defaults,{'key':key,'revision':'source-player-r57'},raw['registrar'])
    spell['execution']={'native_behavior':descriptor}
    if spell['carrier']=='rune':spell['targeting'].setdefault('range_tiles',0)
    bundle={'spell':spell};deps={'abilities':[],'effects':[],'formulas':[]};catalog={'definitions':base.item_references(bundle)}
    errors=validate_spell.validate(bundle,deps,catalog)
    if errors:raise ValueError('actual native proposal schema: '+str(errors))
    helper_audit=[]
    for path in native_companions._helper_paths(name):
        previous=base.source_file(Path('/workspace/spell-sources')/donor,native_companions.PINS[donor],path)
        current=base.source_file(Path('/workspace/spell-sources')/donor,PINS[donor],path)
        helper_audit.append({'path':path,'old_sha256':sha(previous),'current_sha256':sha(current),
                             'whole_file_equal':previous==current,'relevant_function_closure_qualified':False})
    return {'status':'SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED','bundle':bundle,'dependencies':deps,'catalog':catalog,
            'exact_cast_matches_accepted_recipe':True,'native_profile_alias_used':False,
            'old_template_revision':native_companions.PINS[donor],'source_revision':PINS[donor],
            'engine_default_proofs':proofs,'helper_audit':helper_audit,'bounded_function_proofs':closure_proofs(donor),
            'source_api_operations_declared':True,'raw_getter_binding_qualified':False,
            'mana_normalization_policy':{'path':POLICY,'sha256':sha((ROOT/POLICY).read_bytes()),'decision':'C.2 creature summoning.mana_cost'},
            'source_cpp_helper_equivalence':False,'native_reader_admitted':False,'runtime_activation':False}


def build_row(plan,fact):
    donor=fact['source_callback_facts']['source'];raw=fact['source_callback_facts']
    key=plan['registration_key'];row_id=sha(key.encode())[:16]
    folder=R28/key.split('/')[0]/row_id
    original_header_bytes=(folder/'source-header.json').read_bytes()
    header=json.loads(original_header_bytes)['spell']
    source=base.source_file(Path('/workspace/spell-sources')/donor,PINS[donor],raw['file'])
    if sha(source)!=fact['source_sha256'] or fact['source_revision']!=PINS[donor]:
        raise ValueError('immutable source/header identity mismatch')
    candidate_key='candidate:spell/source/'+key.split('/')[0]+'/'+row_id
    out={'registration_key':key,'lane':plan['lane'],'name':raw['name'],
         'source_revision':PINS[donor],'source_sha256':sha(source),'source_header':header,
         'source_header_sha256':sha(original_header_bytes),'source_header_path':str((folder/'source-header.json').relative_to(ROOT)),
         'status':'BLOCKED','proposed_native_binding':None,'partial_target_dependencies':None,
         'source_parameters':None,'full_source_registration_candidate':False,'native_reader_admitted':False,'native_data_model_complete':False,
         'source_alias_to_existing_native_profile':False,'reader_acceptance_qualified':False,'runtime_capability_blocked':True,
         'required_operations_unrepresented':[],
         'runtime_activation':False,'native_identity_allocation':False,'canonical_selection_changed':False,
         'source_numeric_equivalence':False,'input_provider_equivalence':False,'native_execution_qualified':False,'blockers':[]}
    name=raw['name'].casefold()
    if name in NAMES:
        out['proposed_native_binding']=native_proposal(fact,source,header,donor,candidate_key)
        out['status']='CANDIDATE_SCHEMA_VALID'
        out['full_source_registration_candidate']=True;out['native_data_model_complete']=True
        out['blockers']=[{'kind':'native_profile_67_exact_header_gate','detail':'Existing reader accepts its exact pinned native profile, not this new source-registration identity/header. No alias or header rewrite is applied.'},
                         {'kind':'world_provider_execution_unqualified','detail':'Six bounded C++ acquisition roots equal accepted old functions. World provider execution and complete native reader admission remain unqualified.'}]
        if name!='summon creature':
            out['status']='PARTIAL_NATIVE_SOURCE_TARGET';out['full_source_registration_candidate']=False;out['native_data_model_complete']=False
            out['blockers'].append({'kind':'set_summon_attacked_target_copy','detail':'Source Creature:setSummon also copies owner attackedCreature to summon target; current acquire_summon descriptor has no explicit target inheritance parameter.'})
    elif plan['lane']=='party' and '/party/' in raw['file']:
        text=source.decode();distance=re.search(r':getDistance\(position\)\s*<=\s*(\d+)',text)
        mana=re.search(r'local baseMana\s*=\s*(\d+)',text)
        deps,notes=condition_target(raw,donor,candidate_key)
        out['partial_target_dependencies']=deps
        out['status']='PARTIAL_SOURCE_CONDITION_TARGET'
        out['source_parameters']={'party_distance_bound':int(distance[1]) if distance else None,
                                  'base_mana':int(mana[1]) if mana else raw['registrar'].get('mana'),
                                  'mana_mode':'scaled_ceil_0.9_power_count' if mana else 'fixed_registrar_mana',
                                  'condition_declarations':raw.get('conditions',[]),'conversion_notes':notes,
                                  'whole_cast_complete':False}
        out['blockers']=[{'kind':'party_member_selection_shape','detail':'Source getDistance <=36 selects members independently of Combat area; current party_buff params use fixed same_floor + area membership.'},
                         {'kind':'condition_subid_and_full_cast_effects','detail':'Condition SUBID, member presentation, failure returns and mana/spent commit order are retained as source declarations; partial Effect does not qualify the full cast.'}]
    elif '/familiar/' in raw['file']:
        text=source.decode();spellid=re.search(r'local spellId\s*=\s*(\d+)',text)
        helper='data/libs/functions/player.lua';helper_bytes=base.source_file(Path('/workspace/spell-sources')/donor,PINS[donor],helper)
        out['source_parameters']={'spell_id':int(spellid[1]) if spellid else raw['registrar'].get('id'),
                                  'helper':'Player:CreateFamiliarSpell','helper_path':helper,'helper_sha256':sha(helper_bytes),
                                  'existing_descriptor_key':'familiar_summon','native_profile_reuse_qualified':False}
        out['blockers']=[{'kind':'familiar_current_helper_contract','detail':'Current CreateFamiliarSpell spell-ID/config/permission/duration and inherited-condition closure require exact current donor recipe; no generic old familiar profile is substituted.'},
                         {'kind':'native_profile_67_exact_header_gate','detail':'Current source registration cannot be admitted by relabelling a pinned old native profile.'}]
    elif name=='swift foot':
        out['blockers']=[{'kind':'serene_harmony_shared_condition_branch','detail':'Current Canary Swift Foot modifies Serene/harmony and shares conditions with summons; the preserved R32 source program is not a qualified native swift_foot/Speed profile.'}]
    else:
        out['blockers']=[{'kind':'party_dependent_monk_damage_callback','detail':'Crystal Devastating Knockout depends on party members/virtue/harmony/weapon and damage callbacks; party_buff cannot encode this offensive callback.'}]
    if not out['native_data_model_complete']:
        out['required_operations_unrepresented']=[b['kind'] for b in out['blockers']]
    return out


def build():
    plans,facts=records();rows=[build_row(p,facts[p['registration_key']]) for p in plans]
    return {'schema':'OTERYN_PARTY_SUMMON_PROJECTION_AUDIT/v1','records':28,'party_records':11,'summon_records':17,
            'rows':rows,'full_source_registration_candidates':sum(r['full_source_registration_candidate'] for r in rows),
            'proposed_native_bindings':sum(r['proposed_native_binding'] is not None for r in rows),
            'partial_condition_targets':sum(r['partial_target_dependencies'] is not None for r in rows),
            'remaining_without_target':sum(r['proposed_native_binding'] is None and r['partial_target_dependencies'] is None for r in rows),
            'runtime_activation':False,'native_identity_allocation':False,'canonical_selection_changed':False,
            'external_sources_used':False,'input_provider_equivalence':False}


def validate(packet):
    if packet!=build():raise ValueError('lane data/flags differ from exact immutable source projection')
    if len({r['registration_key'] for r in packet['rows']})!=28:raise ValueError('exact28 lane identities required')
    for row in packet['rows']:
        proposal=row['proposed_native_binding']
        if proposal:
            errors=validate_spell.validate(proposal['bundle'],proposal['dependencies'],proposal['catalog'])
            if errors:raise ValueError(str(errors))
        if row['native_reader_admitted'] or row['reader_acceptance_qualified'] or row['source_alias_to_existing_native_profile']:
            raise ValueError('exact native-profile guard has not been qualified')


def write_json(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(value,sort_keys=True,indent=2)+'\n')


def write_packet(out):
    packet=build();validate(packet);out.mkdir(parents=True,exist_ok=True)
    legacy=out/'partial-data.json.gz'
    if legacy.exists():legacy.unlink()
    write_json(out/'partial-data.json',packet)
    write_json(out/'lane-audit.json',{'records':packet['rows']})
    index=[]
    for row in packet['rows']:
        folder=out/'snapshot'/sha(row['registration_key'].encode())[:16];folder.mkdir(parents=True,exist_ok=True)
        header=(ROOT/row['source_header_path']).read_bytes()
        (folder/'source-header.json').write_bytes(header)
        complete=row['full_source_registration_candidate'];proposal=row['proposed_native_binding']
        if complete:
            write_json(folder/'spell.json',proposal['bundle'])
            write_json(folder/'dependencies.json',proposal['dependencies'])
            write_json(folder/'catalog.json',proposal['catalog'])
        receipt={k:row[k] for k in ('registration_key','source_revision','source_sha256','source_header_sha256','status',
                                  'native_data_model_complete','full_source_registration_candidate','source_alias_to_existing_native_profile',
                                  'reader_acceptance_qualified','runtime_capability_blocked','required_operations_unrepresented',
                                  'runtime_activation','native_identity_allocation','canonical_selection_changed','input_provider_equivalence','native_execution_qualified')}
        write_json(folder/'receipt.json',receipt)
        index.append({**receipt,'snapshot':str(folder.relative_to(out))})
    summary={k:v for k,v in packet.items() if k!='rows'};summary['records_index']=index
    write_json(out/'import-summary.json',summary)
    lines=(ROOT/POLICY).read_text().splitlines()
    proof={'schema':'OTERYN_PARTY_SUMMON_PROJECTION_PROOF/v1','records':28,
           'policy_proofs':[{'path':POLICY,'sha256':sha((ROOT/POLICY).read_bytes()),
                            'exact_rows':[line for line in lines if "creature's `summoning.mana_cost`" in line or "target's `summoning.mana_cost`" in line]}],
           'input_proofs':{str(WORKLIST.relative_to(ROOT)):sha(WORKLIST.read_bytes()),
                           str((R28/'source-callback-facts.jsonl.gz').relative_to(ROOT)):sha((R28/'source-callback-facts.jsonl.gz').read_bytes())},
           'runtime_activation':False,'native_execution_qualified':False,'native_identity_allocation':False,
           'canonical_selection_changed':False,'input_provider_equivalence':False,
           'source_cpp_helper_equivalence':False,'source_full_mechanics_1_to_1':False}
    write_json(out/'projection-proof.json',proof)
    # Strict receipt schemas prohibit extra keys and require each exact observed row.
    schema={'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'urn:oteryn:party-summon-receipt:1',
            'oneOf':[{'const':{k:v for k,v in row.items() if k!='snapshot'}} for row in index]}
    write_json(out/'receipt.schema.json',schema)
    (out/'README.md').write_text('''# R57 party and summon source projection

Exact population: 28 registrations (11 party, 17 summons), pinned Canary 04b83b51 and Crystal 00ce02a5. No fresh downloads.

Two Summon Creature registrations have complete actual existing-schema DATA bundles; four Animate Dead/Convince registrations have partial native target definitions because Creature:setSummon additionally copies attackedCreature and the existing descriptor does not expose that operation. Ten party registrations have partial typed condition Effects; twelve remain exact named blockers. Full DATA does not imply native execution.

The six bounded C++ acquisition functions and Lua Creature:setSummon are compared at exact function bytes against accepted descriptor revisions. The accepted C.2 summoning.mana_cost normalization is explicit; the raw getManaCost binding is unqualified. Current party distance selection, source SUBID and full cast order remain blocked. Familiar current helper contracts, Swift Foot Serene/harmony branch and Devastating Knockout callbacks remain blocked.

Every source-header.json is byte-exact R28. Only CANDIDATE_SCHEMA_VALID snapshots contain spell/dependencies/catalog bundles. Source-only partial-data.json retains six proposed models without aliasing old native profiles. Native reader admission, runtime activation, identity allocation, canonical selection and provider equivalence all remain false.

Producer: tools/content-schema/spell-authoring/project_party_summon_candidates.py. Six focused unittest regressions validate real schemas, headers, helper proofs, partition, manifest and refused promotion.
''')
    manifest={'files':{str(p.relative_to(out)):sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file() and p.name!='package-manifest.json'}}
    write_json(out/'package-manifest.json',manifest)
    return packet


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True);args=parser.parse_args()
    write_packet(args.out)


if __name__=='__main__':main()
