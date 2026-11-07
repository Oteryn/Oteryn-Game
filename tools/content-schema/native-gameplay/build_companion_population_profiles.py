#!/usr/bin/env python3
"""Preserve exact canonical source-normalized summon/convince policies.

Imports accepted canonical identities and full profiles unchanged. Full Lua blob
equality is proved independently. Source-only dependencies never activate AI.
"""
from __future__ import annotations
import argparse, hashlib, json, re, subprocess
from pathlib import Path
import build_familiar_profiles as formal

ROOT = formal.ROOT
INDEX = ROOT/'tools/content-schema/monster-authoring/samples/population-bundles-canary-47dfd51f.json'
ORDINARY = ROOT/'tools/content-schema/native-gameplay/ordinary-group.json'

def encoded(value): return (json.dumps(value,indent=2,ensure_ascii=False)+'\n').encode()
def digest(raw): return hashlib.sha256(raw).hexdigest()
def ref(value): return tuple(value[k] for k in ('family','key','revision'))
def refs(value):
    if isinstance(value,dict):
        if all(k in value for k in ('family','key','revision')): yield ref(value)
        for child in value.values(): yield from refs(child)
    elif isinstance(value,list):
        for child in value: yield from refs(child)

def read_canonical(path):
    raw=path.read_bytes()
    accepted=formal.git(ROOT,'show',f'HEAD:{path.relative_to(ROOT)}')
    if raw!=accepted: raise ValueError(f'Canonical source changed without its accepted binding: {path}')
    return raw,json.loads(raw)

def build(source:Path,output:Path,include_rift=False,crystal_source=None):
    if formal.git(source,'rev-parse','HEAD').decode().strip()!=formal.CURRENT:
        raise ValueError('Current Canary HEAD differs from pinned source')
    files=list(sorted((ROOT/'content/creatures/definitions').glob('creatures-*.json')))
    rows=[];source_files=[]
    for path in files:
        raw,doc=read_canonical(path);rows.extend(doc['records'])
        source_files.append({'path':str(path.relative_to(ROOT)),'sha256':digest(raw)})
    declarations_raw,declarations=read_canonical(ROOT/'content/world/definitions/declarations.json')
    reference_raw,reference=read_canonical(ROOT/'content/world/definitions/reference.json')
    for path,raw in [(ROOT/'content/world/definitions/declarations.json',declarations_raw),(ROOT/'content/world/definitions/reference.json',reference_raw)]:
        source_files.append({'path':str(path.relative_to(ROOT)),'sha256':digest(raw)})
    profiles={ref(p['target']):p for p in declarations['authoring_profiles']}
    records={ref(r['identity']):r for r in reference['records']}
    index_raw=INDEX.read_bytes();index=json.loads(index_raw)
    indexed={row['file']:row for row in index['monsters'] if not row.get('binding')}
    ordinary_raw=ORDINARY.read_bytes();ordinary=json.loads(ordinary_raw)
    if ordinary['group_id']!=1 or {'cansummonall','canconvinceall'} & set(ordinary['enabled_flags']):
        raise ValueError('Ordinary group bypass flags changed; selection requires owning policy')
    selected=[r for r in rows if any(r['authoring']['profile']['details']['summoning'][k]
                                     for k in ('summonable','convinceable'))]
    creatures=[];presentations=[];source_only=[];bindings=[];source_paths={};proofs=[];unsupported=[];foreign_proofs=[]
    preserved={'canary:creature/rat':'mammals/rat','canary:creature/skeleton':'undeads/skeleton'}
    preserved_files=set(preserved.values())
    for row in selected:
        target=row['definition']['identity'];exact_bindings=row['source_bindings']
        if len(exact_bindings)!=1 or exact_bindings[0]['disposition']!='EXACT' or exact_bindings[0]['target']!=target:
            raise ValueError('Canonical Creature source binding is not exact and unique')
        binding=exact_bindings[0]
        presentation=profiles[ref(row['definition']['presentation'])]
        behavior=profiles[ref(row['definition']['behavior'])]
        creature={'profile':{'target':target,'data':row['authoring']},
                  'presentation':row['definition']['presentation'],'behavior':behavior}
        if binding['source_key']!='oteryn:source.canary':
            from build_spell_appearances import CRYSTAL_PIN, CRYSTAL_FORMAL, STAG_PATH, pinned
            if target != {'family':'Creature','key':'oteryn:creature.stag','revision':'definition-r1'} or binding['source_key']!='oteryn:source.crystalserver' or binding['source_revision']!='crystalserver-creature-1530:'+CRYSTAL_FORMAL or binding['external_id']!=STAG_PATH:
                raise ValueError('Unqualified foreign Creature source identity')
            crystal_source = crystal_source or Path('/workspace/spell-sources/crystal')
            if formal.git(crystal_source,'rev-parse','HEAD').decode().strip()!=CRYSTAL_PIN:
                raise ValueError('Current Crystal HEAD differs from pinned source')
            before,before_blob,sha=pinned(crystal_source,STAG_PATH,CRYSTAL_FORMAL)
            current,current_blob,_=pinned(crystal_source,STAG_PATH,CRYSTAL_PIN)
            if before!=current or before_blob!=current_blob:
                raise ValueError('Crystal Stag complete source Lua changed')
            values={}
            for name in ('health','maxHealth'):
                matches=re.findall(rf'^\s*monster\.{name}\s*=\s*(\d+)\s*(?:--[^\n]*)?$',before.decode(),re.MULTILINE)
                if len(matches)!=1:
                    raise ValueError(f'Crystal Stag source {name} is not one static literal')
                values[name]=int(matches[0])
            if values['health']!=values['maxHealth'] or values['maxHealth']!=row['authoring']['profile']['health']:
                raise ValueError('Crystal Stag source initial/maximum health differs')
            foreign_proofs.append({'path':STAG_PATH,'definition_revision':CRYSTAL_FORMAL,'current_revision':CRYSTAL_PIN,'definition_git_blob':before_blob,'current_git_blob':current_blob,'sha256':sha,'byte_identical':True,'source_initial_health':values['health'],'source_maximum_health':values['maxHealth'],'canonical_target':target})
            bindings.append(binding);creatures.append(creature);presentations.append(presentation)
            continue
        if binding['source_revision']!=formal.FORMAL or binding['identity_namespace']!='canary/monster-file':
            raise ValueError('Canonical Canary binding source revision/namespace changed')
        file=binding['external_id']
        if file not in indexed: raise ValueError(f'Canonical Creature absent from fully-converted source census: {file}')
        path=f'data-otservbr-global/monster/{file}.lua'
        source_proof=formal.byte_identity(source,path)
        raw=formal.git(source,'show',f'{formal.FORMAL}:{path}').decode()
        values={}
        for name in ('health','maxHealth'):
            matches=re.findall(rf'^\s*monster\.{name}\s*=\s*(\d+)\s*(?:--[^\n]*)?$',raw,re.MULTILINE)
            if len(matches)!=1:raise ValueError(f'Non-static/nonunique source {name}: {path}')
            values[name]=int(matches[0])
        source_proof['source_initial_health']=values['health']
        source_proof['source_maximum_health']=values['maxHealth']
        if values['health']!=values['maxHealth']:
            raise ValueError(f'Source initial health differs from maximum; real spawn owner required: {path}')
        if values['maxHealth']!=row['authoring']['profile']['health']:
            raise ValueError(f'Canonical source maximum health differs: {path}')
        source_proof['canonical_maximum_health']=row['authoring']['profile']['health']
        source_proof['canonical_target']=target;source_proof['formal_bundle_sha256']=indexed[file]['sha256']
        proofs.append(source_proof);bindings.append(binding)
        if file in preserved_files: continue
        asset=presentation['data']['profile'].get('asset_binding','')
        supported_object = include_rift and target == {'family':'Creature','key':'oteryn:creature.rift_fragment','revision':'definition-r1'} and asset == 'canary.appearance:object/2122'
        if not asset.startswith('canary.appearance:outfit/') and not supported_object:
            unsupported.append({'identity':target,'source_binding':binding,
                'reason':'Creature runtime currently has Outfit/Invisible policy only; source Object appearance cannot become Outfit/zero',
                'presentation':presentation})
            source_only.append(creature);continue
        if 'mana_cost' not in row['authoring']['profile']['details']['summoning']:
            raise ValueError('Eligible canonical source lacks its exact mana cost')
        creatures.append(creature);presentations.append(presentation)
        source_paths[target['key']]=path
    # Preserve genuine source declarations transitively as a sidecar, without
    # claiming they passed a runtime monster Ability/Effect execution boundary.
    pending=list(set(refs([creatures,presentations])));seen=set();retained_records={};retained_profiles={};missing=[]
    while pending:
        current=pending.pop()
        if current in seen:continue
        seen.add(current)
        record=records.get(current);profile=profiles.get(current)
        if record is None:missing.append(current);continue
        retained_records[current]=record
        if profile is not None:retained_profiles[current]=profile
        pending.extend(set(refs([record,profile]))-seen)
    documents={
      'companion-population-creature-profiles.json':{'schema':'OTERYN_NATIVE_CREATURE_PROFILES/v1','records':creatures},
      'companion-population-presentation-profiles.json':{'schema':'OTERYN_NATIVE_PRESENTATION_PROFILES/v1','records':presentations},
      'companion-population-dependency-profiles.json':{'schema':'OTERYN_COMPANION_POPULATION_SOURCE_DEPENDENCY_PROFILES/v1',
          'records':[retained_records[k] for k in sorted(retained_records)],
          'authoring_profiles':[retained_profiles[k] for k in sorted(retained_profiles)],
          'missing_exact_records':[dict(zip(('family','key','revision'),k)) for k in sorted(missing)],
          'record_only_dependencies':[dict(zip(('family','key','revision'),k)) for k in sorted(set(retained_records)-set(retained_profiles))]},
      'companion-population-source-only-profiles.json':{'schema':'OTERYN_COMPANION_POPULATION_SOURCE_ONLY_PROFILES/v1','records':source_only},
    }
    output.mkdir(parents=True,exist_ok=True);hashes={}
    for name,doc in documents.items():
        raw=encoded(doc);(output/name).write_bytes(raw);hashes[name]=digest(raw)
    proof={'schema':'OTERYN_COMPANION_POPULATION_SOURCE_PROOF/v1','canonical_base_commit':formal.git(ROOT,'rev-parse','HEAD').decode().strip(),
      'classification':'Source-bound canonical policies; no activation or wild monster AI qualification',
      'source_revision':formal.FORMAL,'current_revision':formal.CURRENT,'ordinary_group_sha256':digest(ordinary_raw),
      'ordinary_group_can_summon_all':False,'ordinary_group_can_convince_all':False,
      'canonical_files':source_files,'formal_population_index_sha256':digest(index_raw),
      'canonical_population_count':len(rows),'canonical_eligible_count':len(selected),
      'canary_eligible_count':len(proofs),'added_creature_count':len(creatures),'preserved_base_source_files':sorted(preserved_files),
      'unchanged_full_definitions':proofs,'unchanged_foreign_definitions':foreign_proofs,
      'rift_object_support_requested':include_rift,'exact_source_bindings':bindings,'appearance_source_paths':source_paths,
      'unsupported_source_profiles':unsupported,'outputs':hashes,
      'bounded_output_bytes':{n:(output/n).stat().st_size for n in documents},
      'producer_sha256':digest(Path(__file__).read_bytes())}
    (output/'companion-population-source-proof.json').write_bytes(encoded(proof))
    return proof

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary',type=Path,default=Path('/workspace/spell-sources/canary'))
    parser.add_argument('--include-rift-object',action='store_true',help='Use only after actual ObjectAppearance adapter qualification')
    parser.add_argument('--crystal',type=Path,default=Path('/workspace/spell-sources/crystal'))
    parser.add_argument('--out',type=Path,required=True);args=parser.parse_args()
    proof=build(args.canary,args.out,args.include_rift_object,args.crystal)
    print(json.dumps({k:proof[k] for k in ('canonical_eligible_count','canary_eligible_count','added_creature_count','bounded_output_bytes')}))
