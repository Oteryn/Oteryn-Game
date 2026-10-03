"""Prepare the exact source actor required by Professor Maxxen's generator core."""
import argparse
import copy
import hashlib
import json
import subprocess
import sys
from pathlib import Path

def prepare(repo, canary, out):
 sys.path[:0]=[str(repo/'tools/content-schema/monster-authoring'),str(repo/'tools/content-schema/encounter-authoring')]
 import canary_batch as cb,validate_monster as vm
 from prepare_encounter_completion import approved_actor_omissions,bundle_digest,write
 p='data-otservbr-global/monster/quests/hero_of_rathleton/glooth-generator.lua';b=subprocess.check_output(['git','-C',str(canary),'show',cb.REVISION+':'+p]);
 if b != (canary/p).read_bytes(): raise ValueError('donor file differs from pinned blob')
 objects=cb.load_appearance_objects(canary/'data/items/appearances.dat');items=cb.load_items_xml(canary/'data/items/items.xml');names,ni=cb.name_index(objects,items);conv=cb.Converter(canary,objects,items,names,ni)
 # The monster declaration evaluator captures data only. No spell callback is evaluated or invoked.
 def no_callback(spell,deps,asset):
  if spell['name']!='glooth-generator summon':raise ValueError('unexpected custom spell')
  return 'UNRESOLVED','registered instant spell "glooth-generator summon" (data-otservbr-global/scripts/spells/monster/glooth-generator_summon.lua) has custom logic: delayed spawn/say/remove needs retained triggering identity and placement/failure semantics. Callback intentionally not executed.'
 conv.registered_spell=no_callback
 conv.pending_definitions=set()
 slug,m,d,c,man,evidence=conv.convert('quests/hero_of_rathleton/glooth-generator');original=copy.deepcopy(man);omitted=approved_actor_omissions(man)
 for family,target in sorted(conv.pending_definitions):
  r=cb.ref(family,target)
  if r not in c['definitions'] and target!='canary:creature/'+slug:c['definitions'].append(r)
 errors=vm.validate(m,d,c,man)
 if errors: raise ValueError(errors)
 for fn,v in [('monster.json',m),('dependencies.json',d),('catalog.json',c),('manifest.json',man)]:write(out/'bundles'/slug/fn,v)
 receipt=json.loads((out/'completion.json').read_text());row={'monster':slug,'file':'quests/hero_of_rathleton/glooth-generator','sha256':bundle_digest(out/'bundles'/slug),'completion_flags':['SOURCE_DONOR_PARTIAL','SOURCE_MECHANICS_OMITTED','MITIGATION_UNKNOWN','GAMEPLAY_UNVERIFIED']};receipt['additional_index_rows']=[row];receipt['new_actor']={'index_row':row,'source':evidence,'source_sha256':hashlib.sha256(b).hexdigest(),'original_manifest':original,'omitted_mechanics':omitted,'source_declaration_capture_only':True,'stats':m['creature']['stats'],'loot_entry_count':len(m.get('loot',{}).get('entries',[])),'validation_errors':errors};script='data-otservbr-global/scripts/spells/monster/glooth-generator_summon.lua';script_data=subprocess.check_output(['git','-C',str(canary),'show',cb.REVISION+':'+script]);receipt['new_actor']['omitted_spell_source']={'file':script,'revision':cb.REVISION,'sha256':hashlib.sha256(script_data).hexdigest(),'git_blob':cb.blob_id(script_data)};receipt['counts']['new_dependency_actors']=1;receipt['counts']['new_dependency_omitted_rows']=len(omitted);write(out/'completion.json',receipt);print(json.dumps(row))

if __name__ == '__main__':
 parser = argparse.ArgumentParser(description=__doc__)
 for name in ('repo', 'canary', 'output'):
  parser.add_argument('--' + name, type=Path, required=True)
 args = parser.parse_args()
 prepare(args.repo, args.canary, args.output)
