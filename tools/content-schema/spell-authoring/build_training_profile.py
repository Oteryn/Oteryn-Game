"""Extract exact training tables from the pinned native reference evidence."""
import argparse, hashlib, json, re, subprocess, xml.etree.ElementTree as ET
from pathlib import Path

def build(root):
    paths=['data/XML/vocations.xml','src/creatures/players/vocations/vocation.cpp','src/creatures/players/vocations/vocation.hpp']
    payload=[(root/p).read_bytes() for p in paths]
    cpp=payload[1].decode(); hpp=payload[2].decode()
    if '1600 * std::pow<double>(manaMultiplier, static_cast<int32_t>(magLevel) - 1)' not in cpp:
        raise ValueError('unknown magic training source formula')
    if 'float manaMultiplier' not in hpp or 'float skillMultipliers' not in hpp:
        raise ValueError('unknown multiplier source storage')
    base=re.search(r'skillBase\[SKILL_LAST \+ 1\] = \{ ([0-9, ]+) \}',cpp)
    bases=[int(x) for x in base[1].split(',')]
    if len(bases)!=7: raise ValueError('incomplete skill base')
    vocations=[]
    for v in ET.fromstring(payload[0]):
        key=v.attrib['name'].lower().replace(' ','_')
        skills=sorted((int(s.attrib['id']),s.attrib['multiplier']) for s in v.findall('skill'))
        if [s[0] for s in skills]!=list(range(7)):raise ValueError('incomplete skills')
        vocations.append({'key':key,'mana_multiplier':v.attrib['manamultiplier'],'skill_multipliers':[s[1] for s in skills]})
    expected={'none','druid','elder_druid','sorcerer','master_sorcerer','knight','elite_knight','paladin','royal_paladin','monk','exalted_monk'}
    if {v['key'] for v in vocations}!=expected:raise ValueError('incomplete vocations')
    revision=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()
    return {'schema':'OTERYN_BUILD_TRAINING_PROFILE/v1','source':{'repository':'opentibiabr/canary','revision':revision,'files':[{'path':p,'sha256':hashlib.sha256(b).hexdigest()} for p,b in zip(paths,payload)]},'magic_base':1600,'skill_bases':bases,'multiplier_storage':'ieee754_binary32','vocations':sorted(vocations,key=lambda v:v['key'])}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--canary',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    a.out.write_text(json.dumps(build(a.canary),sort_keys=True,indent=2)+'\n')
