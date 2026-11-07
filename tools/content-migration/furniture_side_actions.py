"""Pinned Crystal House side-action capture; non-furniture rewards retain W19b ownership."""
from pathlib import Path
import json,re,hashlib
SOURCES=('data-crystal/scripts/actions/other/music.lua','data-crystal/scripts/actions/other/birdcage.lua',
 'data-global/scripts/actions/other/fish_tank.lua','data/scripts/actions/objects/carpets.lua',
 'data/scripts/actions/objects/bath_tub_drain.lua','data/libs/functions/constants.lua','src/utils/utils_definitions.hpp')
def capture(donor):
 raw={p:(donor/p).read_bytes() for p in SOURCES}
 constants={name:int(value) for name,value in re.findall(r'(CONST_ME_[A-Z_]+)\s*=\s*(\d+)',raw[SOURCES[-1]].decode())}
 rules=[];excluded=[]
 for n,line in enumerate(raw[SOURCES[0]].decode().splitlines(),1):
  m=re.search(r'\[(\d+)\]\s*=\s*\{(.*)\}',line)
  if not m:continue
  i=int(m[1]);body=m[2]
  if 'itemId' in body or 'monster' in body:
   excluded.append({'id':i,'reason':'Cornucopia/wooden-whistle reward or summon action; non-furniture W19b scope.'});continue
  effect=re.search(r'\beffect\s*=\s*(CONST_ME_[A-Z_]+)',body)
  failure=re.search(r'failure\s*=\s*(CONST_ME_[A-Z_]+)',body);success=re.search(r'success\s*=\s*(CONST_ME_[A-Z_]+)',body)
  chance=re.search(r'chance\s*=\s*(\d+)',body)
  rules.append({'id':i,'effect':constants[effect[1] if effect else failure[1]],'success_effect':constants[success[1]] if success else None,
   'chance':int(chance[1]) if chance else None,'output':None,'speech':[],'exhaustion_seconds':None,
   'donor':f'crystal-201189f8:{SOURCES[0]}:{n};{SOURCES[-1]}:58-80'})
 rules.extend([{'id':23691,'effect':175,'success_effect':None,'chance':None,'output':None,'speech':[],
  'exhaustion_seconds':None,'donor':'crystal-201189f8:data-global/scripts/actions/other/fish_tank.lua:1-9'},
 {'id':2976,'effect':22,'success_effect':0,'chance':1,'output':2975,'speech':[],
  'exhaustion_seconds':None,'donor':'crystal-201189f8:data-crystal/scripts/actions/other/birdcage.lua:1-13'}])
 carpets=sorted([int(a),int(b)] for a,b in re.findall(r'\[(\d+)\]\s*=\s*(\d+)',raw[SOURCES[3]].decode()))
 if len(carpets)!=160 or 'BATHTUB_EMPTY = 26076' not in raw[SOURCES[5]].decode() or 'BATHTUB_FILLED = 26077' not in raw[SOURCES[5]].decode():
  raise ValueError('HOUSE_SIDE_ACTION_SOURCE_DRIFT')
 return {'side_actions':sorted(rules,key=lambda r:r['id']),'carpets':carpets,'guarded_use':sorted([a for a,b in carpets]+[26077]),
  'extra_use':carpets+[[26077,26076]],'excluded_non_furniture_music':excluded,
  'side_pairs':sorted([[r['id'],r['id']] for r in rules]+[[2976,2975]]),
  'sources':[{'path':p,'sha256':hashlib.sha256(b).hexdigest()} for p,b in raw.items()]}
