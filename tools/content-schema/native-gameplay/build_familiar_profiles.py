#!/usr/bin/env python3
"""Export five complete formal Familiar profiles with explicit unchanged-blob proof.

The existing formal converter and V2 stage perform the conversion. The source
namespace stays at its real formal revision; current byte identity is evidence,
not a revision rewrite. No Game activation or runtime Lua is performed.
"""
from __future__ import annotations
import argparse, hashlib, importlib.util, io, json, subprocess, sys, tarfile, tempfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
FORMAL = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
CURRENT = '99902524e052f37574194466c2949c576e4ab269'
VOCATIONS = ('druid', 'knight', 'monk', 'paladin', 'sorcerer')
MONSTERS = ROOT/'tools/content-schema/monster-authoring'
sys.path.insert(0, str(MONSTERS))
import canary_batch, validate_monster
spec = importlib.util.spec_from_file_location('creature_admission_stage',ROOT/'tools/content-migration/creature_admission_stage.py')
stage = importlib.util.module_from_spec(spec); spec.loader.exec_module(stage)

def git(root, *args):
    return subprocess.check_output(['git','-C',str(root),*args])

def byte_identity(root, path):
    old=git(root,'show',f'{FORMAL}:{path}'); new=git(root,'show',f'{CURRENT}:{path}')
    if old!=new: raise ValueError(f'Familiar definition changed: {path}')
    return {'path':path,'definition_revision':FORMAL,'current_revision':CURRENT,
            'definition_git_blob':git(root,'rev-parse',f'{FORMAL}:{path}').decode().strip(),
            'current_git_blob':git(root,'rev-parse',f'{CURRENT}:{path}').decode().strip(),
            'sha256':hashlib.sha256(old).hexdigest(),'byte_identical':True}

class SourceIdentityMapper(stage.Mapper):
    """Formal source carrier only; no production identity allocation."""
    def ref(self, ref): return dict(ref)
    def identity(self, family, identity): return {'family':family,**identity}

def build(source:Path, output:Path):
    if git(source,'rev-parse','HEAD').decode().strip()!=CURRENT: raise ValueError('current Canary source HEAD mismatch')
    proofs=[byte_identity(source,f'data-otservbr-global/monster/familiars/{v}_familiar.lua') for v in VOCATIONS]
    output.mkdir(parents=True,exist_ok=True)
    shared=['data/items/items.xml','data/items/appearances.dat','src/utils/utils_definitions.hpp',
            'src/creatures/creatures_definitions.hpp','data/scripts/lib/register_monster_type.lua',
            'src/creatures/monsters/monsters.hpp','src/creatures/monsters/monsters.cpp',
            'data/libs/functions/player.lua','data/libs/systems/familiar.lua']
    shared_proof=[{'path':p,'revision':FORMAL,'sha256':hashlib.sha256(git(source,'show',f'{FORMAL}:{p}')).hexdigest()} for p in shared]
    with tempfile.TemporaryDirectory(prefix='oteryn-familiar-formal-') as tmp:
        pinned=Path(tmp)
        paths=['data/scripts','data-otservbr-global/scripts','data-otservbr-global/monster/familiars',*shared]
        archive=git(source,'archive',FORMAL,*paths)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar: tar.extractall(pinned,filter='data')
        objects=canary_batch.load_appearance_objects(pinned/'data/items/appearances.dat')
        items=canary_batch.load_items_xml(pinned/'data/items/items.xml')
        names,index=canary_batch.name_index(objects,items)
        converter=canary_batch.Converter(pinned,objects,items,names,index)
        mapper=SourceIdentityMapper({}); converted=stage.Stage(mapper)
        creatures=[]; presentations=[]; bundles=[]
        for vocation in VOCATIONS:
            converter.pending_definitions=set()
            slug,monster,deps,catalog,manifest,origin=converter.convert(f'familiars/{vocation}_familiar')
            expected=f'{vocation}_familiar'
            if slug!=expected or monster['creature']['identity']!={'key':f'canary:creature/{expected}','revision':'canary-47dfd51f'}:
                raise ValueError('registered Familiar source identity mismatch')
            # The old formal adapter explicitly reports custom source actions.
            # Retain their complete exact schedule reference in the new carrier;
            # qualification/execution is a separate owning source profile.
            unresolved=[e for e in manifest['entries'] if e['status'].startswith('unresolved')]
            _,source_monster,_=canary_batch.load_monster(pinned/canary_batch.MONSTER_DIR/f'familiars/{vocation}_familiar.lua')
            unresolved_sources=[]
            for entry in unresolved:
                import re
                match=re.fullmatch(r'(attacks|defenses)\[(\d+)\]',entry['source_field'])
                if not match: raise ValueError(f'unrepresentable source field: {entry}')
                group=match[1]; values=source_monster[group]
                if isinstance(values,dict): values=values.get('_list',[])
                raw=values[int(match[2])-1]
                if set(raw)-{'name','interval','chance','target'} or not isinstance(raw.get('name'),str):
                    raise ValueError(f'custom action needs an extended typed schedule: {raw}')
                ref={'family':'Ability','key':'canary:ability/spell/'+canary_batch.slug(raw['name']),'revision':'canary-47dfd51f'}
                schedule={'ability':ref,'interval_ms':raw['interval'],'chance_percent':raw['chance']}
                # Original ordinal matters: preserve the source order.
                monster['behavior'][group].insert(int(match[2])-1,schedule)
                if ref not in catalog['definitions']: catalog['definitions'].append(ref)
                unresolved_sources.append({'manifest':entry,'source_operation':raw,'retained_schedule':schedule})
            issues=validate_monster.validate(monster,deps,catalog,None)
            if issues: raise ValueError(f'{slug}: '+ '; '.join(issues))
            target=output/'formal-bundles'/slug;target.mkdir(parents=True,exist_ok=True)
            for name,document in [('monster.json',monster),('dependencies.json',deps),('catalog.json',catalog),('manifest.json',manifest)]:
                (target/name).write_text(json.dumps(document,ensure_ascii=False,indent=2)+'\n')
            creature=stage.profile(mapper.identity('Creature',monster['creature']['identity']),'Creature',converted.creature_profile(monster))
            behavior=stage.profile(mapper.identity('Behavior',monster['behavior']['identity']),'Behavior',converted.behavior_profile(monster['behavior']))
            presentation=stage.profile(mapper.identity('Presentation',monster['presentation']['identity']),'Presentation',converted.presentation_profile(monster['presentation']))
            creatures.append({'profile':creature,'presentation':dict(monster['creature']['presentation']),'behavior':behavior})
            presentations.append(presentation)
            converted.stage_dependencies(deps,slug)
            bundles.append({'identity':creature['target'],'bundle_sha256':stage.bundle_digest(target),'source':origin,
                            'unresolved_manifest_rows':unresolved_sources})
        documents={'familiar-creature-profiles.json':{'schema':'OTERYN_NATIVE_CREATURE_PROFILES/v1','records':creatures},
                   'familiar-presentation-profiles.json':{'schema':'OTERYN_NATIVE_PRESENTATION_PROFILES/v1','records':presentations},
                   'familiar-dependency-profiles.json':{'schema':'OTERYN_FAMILIAR_SOURCE_DEPENDENCY_PROFILES/v1',
                                                       'records':list(converted.records.values()),'authoring_profiles':list(converted.profiles.values())}}
        hashes={}
        for name,document in documents.items():
            raw=(json.dumps(document,ensure_ascii=False,indent=2)+'\n').encode();(output/name).write_bytes(raw);hashes[name]=hashlib.sha256(raw).hexdigest()
        evidence={'schema':'OTERYN_FAMILIAR_PROFILE_SOURCE_PROOF/v1','classification':'OtsHypothesisOnly; explicit local candidate',
                  'definition_revision':FORMAL,'current_revision':CURRENT,'unchanged_full_definitions':proofs,
                  'formal_shared_sources':shared_proof,'bundles':bundles,'outputs':hashes,
                  'conversion_tools':[{'path':str(p.relative_to(ROOT)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in
                                      [MONSTERS/'canary_batch.py',ROOT/'tools/content-migration/creature_admission_stage.py']],
                  'scope':'Full Creature/Behavior/Presentation and dependency source profiles; runtime source admission and activated closure remain required.'}
        (output/'familiar-profile-source-proof.json').write_text(json.dumps(evidence,indent=2)+'\n')
        return evidence

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--canary',type=Path,default=Path('/workspace/spell-sources/canary'));parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args();proof=build(args.canary,args.out);print(json.dumps({'creatures':len(proof['bundles']),'outputs':proof['outputs']}))
