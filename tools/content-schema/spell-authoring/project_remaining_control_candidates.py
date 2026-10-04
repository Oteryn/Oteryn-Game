"""R58 accepted canonical control data; source differences stay explicit.

Only eight plain, policy-covered models qualify as target-data candidates. Native
profile proposals and omitted controllers stay partial. No runtime is activated.
"""
import argparse
from collections import Counter
import copy
import gzip
import hashlib
import json
from pathlib import Path
import tempfile

import import_source_player_bundles as base
import project_player_control_candidates as controls
import validate_spell

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = Path('docs/reference/spells/r58-source-closure')
REVISION = 'candidate-unblocking-r58'
POLICY = 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md'
CATALOG = 'tools/content-schema/spell-authoring/samples/executable-spell-catalog.json'
NATIVE = 'tools/content-schema/spell-authoring/samples/native-spell-profiles.json'
BASE = Path('imports/spells/r28/player-source-bundles')
LANES = {'world_target','p4_custom','other_cast','paralyze_contract','disabled_examples'}
FULL = {'intense_healing_rune.lua', 'ultimate_healing_rune.lua', 'cancel_magic_shield.lua', 'paralyze_rune.lua'}
EXCLUDE = {'canary-main-current/data/scripts/spells/support/levitate.lua#1',
           'canary-main-current/data/scripts/spells/support/magic_rope.lua#1',
           "canary-main-current/data/scripts/spells/healing/nature's_embrace.lua#1",
           'crystal-summer-current/data/scripts/spells/attack/forked_glacier.lua#1',
           'crystal-summer-current/data/scripts/spells/attack/forked_thorns.lua#1'}


def sha(raw): return hashlib.sha256(raw).hexdigest()
def encoded(value): return (json.dumps(value,sort_keys=True,indent=2,ensure_ascii=False)+'\n').encode()
def canonical(value): return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
def subfolder(reg): return reg.split('/',1)[0]+'/'+sha(reg.encode())[:16]


def differences(raw, target, path=''):
    if isinstance(raw,dict) and isinstance(target,dict):
        result=[]
        for key in sorted(set(raw)|set(target)):
            child=path+'.'+key if path else key
            if key not in raw or key not in target:
                result.append({'field':child,'source_present':key in raw,'target_present':key in target,
                               'source':raw.get(key),'target':target.get(key)})
            else: result.extend(differences(raw[key],target[key],child))
        return result
    return [] if raw==target else [{'field':path,'source_present':True,'target_present':True,'source':raw,'target':target}]


def policy_proofs(root):
    lines=(root/POLICY).read_text().splitlines()
    markers=['F overrides C (S3/S21)', '| both | allowed targets |', 'targeting.allowed_targets`:',
             'Ultimate Healing Rune is refused for the monk', 'Cancel Magic Shield: self cast;',
             'Cancel Magic Shield: no native key.', 'Paralyze Rune: after a successful cast',
             "Paralyze Rune's condition:", 'Both are ordinary Abilities.',
             '**Expose Weakness and Sap Strength (removed, S24; not authored).**']
    selected=[]
    for marker in markers:
        matches=[line for line in lines if marker in line]
        if len(matches)!=1: raise ValueError('accepted policy marker differs: '+marker)
        selected.append(matches[0])
    author=root/'docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md'
    rows=[line for line in author.read_text().splitlines() if '| S5 |' in line or '| S16 |' in line]
    if len(rows)!=2: raise ValueError('accepted formula/unlock policy differs')
    return [{'path':POLICY,'sha256':sha((root/POLICY).read_bytes()),'exact_rows':selected},
            {'path':author.relative_to(root).as_posix(),'sha256':sha(author.read_bytes()),'exact_rows':rows}]


def source_limits(name, donor):
    if name in ('intense_healing_rune.lua','ultimate_healing_rune.lua'):
        return ['Accepted B.5/D.3 replaces donor Monster/self-only refusal with self_or_own_summons; target resolution and master ownership remain provider-owned.',
                ('Accepted B.5/D.3 excludes both monk vocations for UHR; donor exalted-only check and exact refusal messages/POFF chronology are source evidence, not reproduced here.'
                 if name=='ultimate_healing_rune.lua' else
                 'IHR has no accepted Monk exclusion. Its unrestricted source vocation map is retained, including monk/exalted_monk and none; exact refusal presentation stays source evidence.'),
                ('Current Vocation::from_key rejects none. The IHR source-data model is schema valid but its actual operational reader remains capability-blocked; none is not silently removed.'
                 if name=='intense_healing_rune.lua' else
                 'UHR caster vocations follow accepted B.5/D.3 rather than the unrestricted donor map.'),
                'Crystal Leiden direct-health exception and original Variant fallback are not part of the accepted normal rune policy; source observations remain archived.',
                'Canonical S5 level contribution replaces source level/5/helper differences; exact raw bounds remain in source capture.']
    if name=='cancel_magic_shield.lua':
        return ['Accepted C.5 ordinary self remove_condition plus blue presentation is used.',
                'Donor removeCondition happens before Combat:execute; canonical accepted-cast ordering does not preserve rejected-cast pre-removal or exact lexical presentation order.',
                'No claim that side_effect planning executes this bundle; native/runtime/provider execution remains unqualified.']
    if name=='paralyze_rune.lua':
        return ['Accepted D.5 target model: zero-damage health path, six-second paralysis, speed target 40 and caster green after accepted cast.',
                'Source LuaCombatExecute VARIANT_NUMBER may return true despite condition refusal; after_success is canonical policy, not proof of source-equivalent immune-refusal presentation.',
                'R45 preserves exact source speed/C++ conversion and cast chronology; this normalized target model does not replace that evidence.']
    if name in ('heal_friend.lua',"nature's_embrace.lua"):
        return ['Crystal Shared Conservation ordered secondary heal, result independence and party-member selector are not projected by the ordinary primary-heal template.']
    return ['Harmony/equipment/spender callbacks or composed source controller are not completely represented by the basic canonical Ability; kept partial.']


def build(root):
    root=Path(root)
    baseline=json.loads((root/'docs/reference/spells/blocked-173-worklist.json').read_bytes())
    selected=[row for row in baseline['records'] if row['lane'] in LANES and row['registration_key'] not in EXCLUDE]
    if len(selected)!=45: raise ValueError('exact remaining control lane population differs')
    catalog_body=(root/CATALOG).read_bytes(); document=json.loads(catalog_body)
    templates={(row['bundle']['spell']['name'].casefold(),row['bundle']['spell']['carrier']):row for row in document['bundles']}
    native_body=(root/NATIVE).read_bytes(); native={row['name'].casefold():row for row in json.loads(native_body)['profiles']}
    proofs=policy_proofs(root); packet={}; audits=[]; partials=[]; index=[]
    for row in selected:
        reg=row['registration_key']; path=subfolder(reg); old=root/BASE/path
        receipt=json.loads((old/'receipt.json').read_bytes()); header_body=(old/'source-header.json').read_bytes()
        header=json.loads(header_body); name=reg.rsplit('/',1)[1].rsplit('#',1)[0]
        if receipt['status']!='BLOCKED' or receipt['source_header']!=header['spell']: raise ValueError('exact blocked source header required')
        packet[path+'/source-header.json']=header_body
        audit={'registration_key':reg,'source_sha256':receipt['source_sha256'],'source_revision':receipt['source_revision'],
               'source_header_sha256':sha(header_body),'historical_receipt_sha256':sha((old/'receipt.json').read_bytes()),
               'baseline_lane':row['lane'],'name':row['name'],'status':'BLOCKED',
               'runtime_activation':False,'native_execution_qualified':False,'source_numeric_equivalence':False,
               'source_full_mechanics_1_to_1_complete':False}
        if row['lane']=='disabled_examples':
            audit.update(disposition='DISABLED_REFERENCE_EXAMPLE',blockers=['Source filename begins #; registration retained as disabled reference and not admitted.'])
        elif row['name']=='Expose Weakness':
            audit.update(disposition='RETIRED_REFERENCE_ONLY_S24',blockers=['Accepted S24 removes Expose Weakness; preserve source without creating a playable duplicate.'])
        elif row['name']=='Mentor Other':
            audit.update(disposition='BLOCKED_MISSING_ACCEPTED_TARGET_CONTRACT',blockers=['No canonical catalogue entry or accepted complete Mentor Other controller binding.'])
        else:
            identity=(row['name'].casefold(),header['spell']['carrier']); template=templates.get(identity)
            if template is None: raise ValueError('missing concrete target template '+str(identity))
            bundle=copy.deepcopy(template['bundle']); deps=copy.deepcopy(template['dependencies']); external=copy.deepcopy(template['catalog'])
            is_native=row['name'].casefold() in native
            complete=name in FULL and not is_native
            default_proofs=[]
            if name=='intense_healing_rune.lua':
                donor=reg.split('-')[0]
                defaults,default_proofs=base.default_fields(Path('/workspace/spell-sources')/donor,receipt['source_revision'])
                if 'vocations' in header['spell'].get('requirements',{}):
                    raise ValueError('IHR source gained explicit vocation constraints')
                bundle['spell']['requirements']['vocations']=defaults['unrestricted_vocations']
            if complete:
                bundle['spell']['identity']={'key':receipt['candidate_key'],'revision':REVISION}
            errors=validate_spell.validate(bundle,deps,external,template.get('manifest'))
            if errors: raise ValueError('target semantic validation failed: '+str(errors))
            validate_spell.Draft202012Validator(base.receipt_schema(),registry=validate_spell.REGISTRY).validate(receipt)
            controls.validate_reader_shape(root,bundle,deps)
            if is_native:
                profile=native[row['name'].casefold()]
                if bundle!={'spell':profile['spell']} or deps!=profile['dependencies']:
                    raise ValueError('native alias must retain exact closed profile')
            for file,value in [('spell.json',bundle),('dependencies.json',deps),('catalog.json',external)]:
                packet[path+'/'+file]=encoded(value)
            limitations=source_limits(name,reg.split('-')[0]) if not is_native else [
                'PROPOSED existing canonical native profile alias; exact source helper/body/header policy for the current donor is not established.',
                'Whole-profile equality is required by native::spell_from_bundle; no changed native identity, parameters, costs or runtime admission is authorized.']
            audit.update(disposition='CANONICAL_NORMALIZED_TARGET_CANDIDATE' if complete else ('PROPOSED_NATIVE_BINDING' if is_native else 'PARTIAL_TARGET_BUNDLE'),
                         target_identity=bundle['spell']['identity'],target_bundle_path=path,
                         target_header_differences=differences(header['spell'],bundle['spell']),
                         target_model_structural_shape_checked=True,existing_reader_executed_on_this_bundle=False,
                         full_target_projection_complete=complete,blockers=[] if complete else limitations,
                         source_differences_and_execution_limits=limitations)
            if name=='intense_healing_rune.lua':
                audit['reader_capability_status']='SOURCE_SCHEMA_VALID_RUNTIME_CAPABILITY_BLOCKED'
                audit['reader_capability_gaps']=[{'field':'requirements.vocations','source_value':'none',
                    'reader_endpoint':'spell::Vocation::from_key',
                    'reason':'Source permits vocation none; current operational reader recognizes only ten named vocations.',
                    'required_owner':'existing spell runtime coordinator; no identity or protocol mutation in this data batch'}]
            proof={'schema':'OTERYN_REMAINING_CONTROL_TARGET_PROJECTION/v1',**copy.deepcopy(audit),
                   'canonical_selection_changed':False,'native_identity_allocation':False,'input_provider_equivalence':False,
                   'template_catalog_sha256':sha(catalog_body),'native_profile_file_sha256':sha(native_body) if is_native else None,
                   'source_vocation_default_proofs':default_proofs,
                   'policy_proofs':proofs,'target_schema_and_semantic_validation_errors':[]}
            packet[path+'/projection-receipt.json']=encoded(proof)
            if complete:
                audit['status']='CANDIDATE_SCHEMA_VALID'; proof['status']=audit['status']; packet[path+'/projection-receipt.json']=encoded(proof)
                upgraded=copy.deepcopy(receipt);upgraded.update(status=audit['status'],blockers=[],
                    dependencies={key:len(value) for key,value in deps.items()},schema_and_semantic_validation_errors=[],
                    native_execution_qualified=False,item_owner_bindings_required=external['definitions'],
                    remaining_mechanics=[{'source_field':'canonical_projection.source_difference','reason':value} for value in limitations],
                    conversion_notes=['Accepted canonical target normalization; source metadata preserved; no source equivalence or runtime activation.'])
                validate_spell.Draft202012Validator(base.receipt_schema(),registry=validate_spell.REGISTRY).validate(upgraded)
                packet[path+'/receipt.json']=encoded(upgraded)
            else:
                partials.append({'registration_key':reg,'target_identity':bundle['spell']['identity'],'target_bundle_path':path,
                                 'disposition':audit['disposition'],'full_target_projection_complete':False,
                                 'runtime_activation':False,'native_execution_qualified':False})
        audits.append(audit)
        index.append({'registration_key':reg,'candidate_key':receipt['candidate_key'],'status':audit['status'],
                      **({'candidate_revision':REVISION} if audit['status']=='CANDIDATE_SCHEMA_VALID' else {})})
    counts=dict(Counter(row['status'] for row in audits)); dispositions=dict(Counter(row['disposition'] for row in audits))
    summary={'records':len(index),'status_counts':counts,'records_index':index,'candidate_revision':REVISION,
             'runtime_activation':False,'native_execution_qualified':False}
    proof={'schema':'OTERYN_REMAINING_CONTROL_PROJECTION_PROOF/v1','record_count':len(audits),
           'standard_bundle_count':counts.get('CANDIDATE_SCHEMA_VALID',0),'partial_target_bundle_count':len(partials),
           'status_counts':counts,'disposition_counts':dispositions,'policy_proofs':proofs,
           'template_catalog_proof':{'path':CATALOG,'sha256':sha(catalog_body)},
           'native_template_proof':{'path':NATIVE,'sha256':sha(native_body)},
           'receipt_schema_proof':{'path':'imports/spells/r28/schemas/player-bundle-receipt.schema.json',
                                   'sha256':sha((root/'imports/spells/r28/schemas/player-bundle-receipt.schema.json').read_bytes())},
           'producer_sha256':sha(Path(__file__).read_bytes()),'runtime_activation':False,
           'native_execution_qualified':False,'native_identity_allocation':False,'canonical_selection_changed':False,
           'input_provider_equivalence':False,'source_numeric_equivalence':False,'source_full_mechanics_1_to_1_complete':False,
           'existing_reader_executed_on_these_bundles':False}
    proof['reader_shape_proofs'] = [{'path':'apps/game-server/src/spell/'+name,
        'sha256':sha((root/'apps/game-server/src/spell'/name).read_bytes()),
        'scope':'local decoder struct shape; runtime execution remains unqualified'}
        for name in ['executable_catalog.rs','authoring.rs','target.rs','plan.rs','mod.rs']]
    packet.update({'lane-audit.json':encoded({'records':audits}),'partial-data.json':encoded({'records':partials}),
                   'import-summary.json':encoded(summary),'projection-proof.json':encoded(proof),
                   'receipt.schema.json':encoded(base.receipt_schema())})
    packet['README.md'] = ('# R58: dane kontroli czarów\n\n'
        'Pakiet obejmuje wszystkie 45 przydzielonych wariantów. Osiem standardowych kandydatów '
        'danych stosuje przyjęte reguły B.5/D.3, C.5 i D.5: po dwa IHR, UHR, Cancel Magic Shield '
        'i Paralyze. Surowe nagłówki pozostają niezmienione, a różnice źródłowe są jawne.\n\n'
        '30 modeli pozostaje częściowych: 24 proponowane powiązania istniejących profili native '
        'oraz sześć zwykłych modeli z brakującym kontrolerem Harmony/equipment lub Shared Conservation. '
        'Cztery przykłady pozostają wyłączone, dwa Expose Weakness są referencją wycofaną zgodnie z S24, '
        'a Mentor Other wymaga przyjętego kontraktu.\n\n'
        'Kandydat danych nie oznacza zgodności 1:1, wykonania przez serwer ani aktywacji. '
        'Nie uruchomiono readera Rust na tych ośmiu pakietach; sprawdzono schemy, zależności, '
        'walidację semantyczną oraz strukturę istniejącego readera. Powiązania native zachowują '
        'dokładne istniejące identity, nagłówki i zależności, bez promocji źródłowych receipts.\n\n'
        'Źródła: lokalne przypięte dane Canary/Crystal w r28, przyjęte dokumenty authoring '
        'oraz istniejący katalog 246 i 67 profili native. Bez nowych odczytów internetu.\n').encode()
    return packet,proof


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--root',type=Path,default=ROOT);parser.add_argument('--out',type=Path)
    args=parser.parse_args();packet,proof=build(args.root);out=args.out or args.root/OUT
    out.mkdir(parents=True,exist_ok=True)
    for name,body in packet.items():
        path=out/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body)
    manifest={'schema':'OTERYN_REMAINING_CONTROL_PACKAGE/v1','files':{name:sha(body) for name,body in packet.items()},
              'runtime_activation':False,'native_execution_qualified':False,'canonical_selection_changed':False}
    (out/'package-manifest.json').write_bytes(encoded(manifest))
    print(json.dumps({'records':proof['record_count'],'candidates':proof['standard_bundle_count'],
                      'partial_target_bundles':proof['partial_target_bundle_count'],'dispositions':proof['disposition_counts']}))


if __name__=='__main__':main()
