"""Resolve source spell cue aliases from pinned enum declarations, not visual membership."""
import argparse, hashlib, json, re, subprocess
from pathlib import Path
PIN = '99902524e052f37574194466c2949c576e4ab269'
FILES = ['src/utils/utils_definitions.hpp', 'src/creatures/creatures_definitions.hpp']
def build(source, catalog):
    revision = subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    if revision != PIN: raise ValueError('unqualified source revision')
    blobs = [subprocess.check_output(['git','-C',str(source),'show',PIN+':'+p]) for p in FILES]
    values = dict((k,int(v)) for k,v in re.findall(r'\b([A-Z][A-Z_0-9]*)\s*=\s*(\d+)\b', b'\n'.join(blobs).decode()))
    aliases = set()
    def walk(v):
        if isinstance(v,dict):
            for z in v.values(): walk(z)
        elif isinstance(v,list):
            for z in v: walk(z)
        elif isinstance(v,str) and re.match(r'(canary\.)?(sound:|appearance:(effect|missile)/)',v): aliases.add(v)
    walk(json.loads(catalog.read_bytes()))
    records = []
    for alias in sorted(aliases):
        short = alias.removeprefix('canary.')
        if short == 'appearance:missile/weapontype':
            # Runtime weapon policy must resolve this dynamic symbol; no guessed projectile.
            continue
        if short.startswith('appearance:effect/'):
            kind, enum = 'effect', 'CONST_ME_' + short.split('/')[1].upper()
        elif short.startswith('appearance:missile/'):
            kind, enum = 'projectile', 'CONST_ANI_' + short.split('/')[1].upper()
        else:
            kind, enum = 'sound', short.split(':')[1].upper()
        if enum not in values: raise ValueError('unknown source cue '+alias)
        records.append({'alias':alias,'kind':kind,'source_enum':enum,'value':values[enum]})
    return {'schema':'OTERYN_SOURCE_SPELL_CUES/v1','source_revision':PIN,'source_files':[{'path':p,'sha256':hashlib.sha256(b).hexdigest()} for p,b in zip(FILES,blobs)],'records':records,'dynamic_aliases':['canary.appearance:missile/weapontype']}
if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--canary',type=Path,required=True);p.add_argument('--catalog',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    a.out.write_text(json.dumps(build(a.canary,a.catalog),sort_keys=True,indent=2)+'\n')
