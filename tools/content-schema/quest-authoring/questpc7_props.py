"""Source-backed first-party Feaster props; retained B3 files stay immutable."""
import hashlib
import json

def refresh_source_pins(root, reference):
    """Refresh a changed region only when both source cells are byte-equivalent."""
    import subprocess
    import sys
    from pathlib import Path
    sys.path.insert(0,str(root/'tools/content-schema/world-authoring'))
    import world_region_codec as codec
    cache={}
    refreshed=[]
    for row in SITES:
        path=row['path'];current=(root/path).read_bytes()
        digest=hashlib.sha256(current).hexdigest()
        replacement=dict(row)
        if digest!=row['sha256']:
            if path not in cache:
                previous=subprocess.check_output(['git','show',reference+':'+path],cwd=root)
                if hashlib.sha256(previous).hexdigest()!=row['sha256']:
                    raise ValueError('QUESTPC7_PROP_REFERENCE_PIN_MISMATCH')
                def cells(raw):
                    return {(x,y):(flags,house,zones,entries)
                            for _,sector in codec.decode_region(raw)[3]
                            for x,y,flags,house,zones,entries in sector}
                cache[path]=(cells(previous),cells(current))
            previous,current_cells=cache[path]
            for kind in ('position','arrival'):
                cell=tuple(row[kind][:2])
                if previous.get(cell)!=current_cells.get(cell) or cell not in previous:
                    raise ValueError('QUESTPC7_PROP_SOURCE_CELL_CHANGED:'+row['name']+':'+kind)
            replacement['sha256']=digest
        refreshed.append(replacement)
    source=Path(__file__)
    text=source.read_text()
    original=next(line for line in text.splitlines() if line.startswith('SITES='))
    source.write_text(text.replace(original,'SITES='+repr(refreshed),1))
    print('PROP_SOURCE_PINS_REFRESHED: six props and six arrivals retain exact native cells')
QUEST='oteryn:quest.feaster_of_souls_quest'
SITES=[{'name': 'grinder', 'position': [32630, 32075, -7], 'appearance': 21573, 'role': 'Wooden Grinder', 'path': 'content/world/placements/region-z07-x127-y125.b3', 'sha256': '9a29bd789d743e29c2c90e4df547e7125084699bd3d97c325e1f993adf24a0b1', 'arrival': [32630, 32074, -7], 'arrival_appearance': 231}, {'name': 'apparatus', 'position': [32629, 32077, -7], 'appearance': 8995, 'role': 'Alchemical Apparatus', 'path': 'content/world/placements/region-z07-x127-y125.b3', 'sha256': '9a29bd789d743e29c2c90e4df547e7125084699bd3d97c325e1f993adf24a0b1', 'arrival': [32628, 32077, -7], 'arrival_appearance': 231}, {'name': 'ulcer', 'position': [31916, 32354, -8], 'appearance': 32173, 'role': 'Ulcer Probe', 'path': 'content/world/placements/region-z08-x124-y126.b3', 'sha256': 'aed6c165a77e3fa1e7806caac386d83d384cba2488fe2c429b485296335eb4e9', 'arrival': [31915, 32354, -8], 'arrival_appearance': 1020}, {'name': 'soul_source', 'position': [31918, 32354, -8], 'appearance': 23163, 'role': 'Lost Soul (Item)', 'path': 'content/world/placements/region-z08-x124-y126.b3', 'sha256': 'aed6c165a77e3fa1e7806caac386d83d384cba2488fe2c429b485296335eb4e9', 'arrival': [31917, 32354, -8], 'arrival_appearance': 1020}, {'name': 'purification', 'position': [31919, 32353, -8], 'appearance': 8996, 'role': 'Purifier for Lost Souls', 'path': 'content/world/placements/region-z08-x124-y126.b3', 'sha256': 'aed6c165a77e3fa1e7806caac386d83d384cba2488fe2c429b485296335eb4e9', 'arrival': [31918, 32353, -8], 'arrival_appearance': 1020}, {'name': 'crystal', 'position': [33220, 31697, -8], 'appearance': 32537, 'role': 'Luminescent Crystal', 'path': 'content/world/placements/region-z08-x129-y123.b3', 'sha256': '2e4b7303f8f61074c0b7367b184b3ef9239b0eb9d08f58dd76abd3586e5120ad', 'arrival': [33219, 31697, -8], 'arrival_appearance': 429}]

def build(root):
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    definition=next(r['definition'] for r in json.loads(raw)['records'] if r['definition']['identity']['key']==QUEST)
    roles={t for stage in definition['oteryn_recipe']['payload']['recipe']['stages'] for t in stage['targets']}
    if not all(r['role'] in roles for r in SITES):raise ValueError('QUESTPC7_FEASTER_PROP_ROLE_DRIFT')
    sources=['https://www.tibiawiki.com.br/index.php?title=Feaster_of_Souls_Quest&oldid=421868','imports/canary/items-xml/items.xml:sha256:1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2']
    for row in SITES:
        if hashlib.sha256((root/row['path']).read_bytes()).hexdigest()!=row['sha256']:raise ValueError('QUESTPC7_FEASTER_PROP_NATIVE_CELL_DRIFT')
        citation=row['path']+':sha256:'+row['sha256']
        if citation not in sources:sources.append(citation)
    return {'schema':1,'basis':'CHOSEN_OTERYN_APPROXIMATION','quest':{'key':QUEST,'path':path,'sha256':hashlib.sha256(raw).hexdigest()},'sources':sources,'props':[{'key':'oteryn:world-prop/questpc7-feaster-'+r['name'],'item':'oteryn:item.tibia.i'+str(r['appearance']),'position':dict(zip(['x','y','floor'],r['position'])),'role':r['role']} for r in SITES]}

def outputs(root):
    path='content/world/placements/authored-questpc7.json';raw=(json.dumps(build(root),ensure_ascii=False,indent=2)+'\n').encode()
    index_path='content/world/placements/index.json';before=(root/index_path).read_bytes();index=json.loads(before)
    pins=[p for p in index.get('authored_props',[]) if p['path'] not in {path,'content/world/authored-props/questpc7.json'}]+[{'path':path,'sha256':hashlib.sha256(raw).hexdigest()}]
    index['authored_props']=sorted(pins,key=lambda p:p['path'])
    rendered=json.dumps(index,ensure_ascii=False,indent=2) if b'\n  \"' in before else json.dumps(index,ensure_ascii=False,separators=(',',':'))
    return {path:raw,index_path:(rendered+'\n').encode()}

if __name__=='__main__':
    import argparse
    from pathlib import Path
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--refresh-source-pins',metavar='REFERENCE_REVISION',required=True)
    refresh_source_pins(Path(__file__).resolve().parents[3],parser.parse_args().refresh_source_pins)
