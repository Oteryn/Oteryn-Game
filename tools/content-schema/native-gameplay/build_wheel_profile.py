#!/usr/bin/env python3
"""Project the pinned Canary 36-slot gates into a bounded, source-qualified candidate."""
import hashlib,json,re,subprocess
from pathlib import Path
SOURCE=Path('/workspace/spell-sources/canary')
REV='99902524e052f37574194466c2949c576e4ab269'
files=['src/creatures/players/components/wheel/player_wheel.cpp','src/creatures/players/components/wheel/player_wheel.hpp','src/creatures/players/components/wheel/wheel_definitions.hpp','src/creatures/combat/spells.cpp','config.lua.dist','src/io/io_wheel.hpp','src/io/io_wheel.cpp']
if subprocess.check_output(['git','rev-parse','HEAD'],cwd=SOURCE,text=True).strip()!=REV:
    raise ValueError('Wheel source HEAD differs from pinned revision')
raw={path:subprocess.check_output(['git','show',f'{REV}:{path}'],cwd=SOURCE) for path in files}
for path,data in raw.items():
    expected_blob=subprocess.check_output(['git','rev-parse',f'{REV}:{path}'],cwd=SOURCE,text=True).strip()
    actual_blob=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
    if actual_blob!=expected_blob:
        raise ValueError(f'Wheel source blob mismatch: {path}')
cpp=raw[files[0]].decode();header=raw[files[2]].decode()
enum=header.split('enum class WheelSlots_t',1)[1].split('};',1)[0]
slots={name:int(number) for name,number in re.findall(r'(SLOT_\w+)\s*=\s*(\d+)',enum)}
assert len(slots)==36 and sorted(slots.values())==list(range(1,37))
body=cpp.split('bool PlayerWheel::canPlayerSelectPointOnSlot(',1)[1].split('uint16_t PlayerWheel::getUnusedPoints()',1)[0]
starts=list(re.finditer(r'(?:if|else if) \(slot == WheelSlots_t::(SLOT_\w+)\)',body))
assert len(starts)==36
records=[]
for index,start in enumerate(starts):
    name=start.group(1);block=body[start.end():starts[index+1].start() if index+1<len(starts) else len(body)]
    minima=re.findall(r'playerPoints < (\d+)u?',block)
    parents=re.findall(r'canSelectSlotFullOrPartial\(WheelSlots_t::(SLOT_\w+)\)',block)
    capacity=int(name.rsplit('_',1)[1]);assert capacity in [50,75,100,150,200]
    if capacity==50: assert not minima and not parents and '|| true' in block
    else: assert len(minima)==1 and parents
    records.append({'slot':slots[name],'capacity':capacity,'minimum_available_points':int(minima[0]) if minima else 0,'colour':name.split('_')[1].lower(),'full_neighbours':sorted({slots[p] for p in parents})})
assert 'wheelPointsPerLevel = 1' in raw[files[4]].decode()
assert 'm_minLevelToStartCountPoints = 50' in raw[files[1]].decode()
stats=[list(map(int,pair)) for pair in re.findall(r'Stats \{ (\d+), (\d+) \}', raw[files[5]].decode())]
assert stats==[[4,4],[9,9],[20,20]]
profile={'revelation_stats':stats,'schema':'OTERYN_WHEEL_SOURCE_PROFILE/v1','source_revision':REV,'source_files':[{'path':p,'sha256':hashlib.sha256(raw[p]).hexdigest(),'git_blob':hashlib.sha1(b'blob '+str(len(raw[p])).encode()+b'\0'+raw[p]).hexdigest()} for p in files],'points_per_level':1,'minimum_level':51,'stage_thresholds':[250,500,1000],'slots':sorted(records,key=lambda x:x['slot'])}
Path(__file__).with_name('wheel-profile.json').write_text(json.dumps(profile,indent=2)+'\n')
print('36 exact source slot gates projected')
