"""Capture exact owned expertise doors using the pinned startup table and physical map."""
import argparse, hashlib, json, re, sys, tarfile
from pathlib import Path
import world_region_codec as codec
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/quest-authoring'))
import lua_tables
OUTPUT=ROOT/'tools/content-schema/world-authoring/samples/quest-level-door-bindings.json'
PIN='9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
WANTED={(33211,31638,13):100,(32479,32174,15):30}

def document():
    path='data-global/startup/tables/door_level.lua'
    with tarfile.open(ROOT/'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest=json.load(archive.extractfile('corpus-manifest.json'))
        source,=[r for r in manifest['files'] if r['source']=='crystalserver' and r['revision']==PIN and r['path']==path]
        raw=archive.extractfile(source.get('blob_path',source.get('cache_path'))).read()
    if hashlib.sha256(raw).hexdigest()!=source['sha256']:raise ValueError('level source digest')
    table=lua_tables.assignments(raw.decode(),{'LevelDoorAction'})['LevelDoorAction']
    registry=(ROOT/'crates/gameplay/world/src/map/door_registry.rs').read_text().split('const LEVEL_DOORS:',1)[1].split('];',1)[0]
    locked={int(a) for a,b in re.findall(r'\((\d+), (\d+)\)',registry)}
    index=json.loads((ROOT/'content/world/placements/index.json').read_bytes());bindings=[]
    for field in table['fields']:
        if field['key'] not in {1000+n for n in WANTED.values()}:continue
        groups=lua_tables.as_python(field['value'])
        for group in groups if isinstance(groups,list) else [groups]:
            for pos in group['itemPos']:
                xyz=tuple(pos[k] for k in 'xyz')
                if xyz not in WANTED:continue
                if field['key']!=1000+WANTED[xyz]:raise ValueError('level mismatch')
                x,y,z=xyz;region,=[r for r in index['regions'] if r['path'].endswith(codec.region_name(z,x//256,y//256))]
                data=(ROOT/region['path']).read_bytes()
                if hashlib.sha256(data).hexdigest()!=region['sha256']:raise ValueError('level map hash')
                tile,=[t for _,tiles in codec.decode_region(data)[3] for t in tiles if t[:2]==(x,y)]
                if tile[3]!=0:raise ValueError('level House tile')
                top=[index['palette'][i]['source_item_id'] for i,depth,attrs in tile[-1] if depth==0]
                item,=[i for i in top if i in locked]
                if group['itemId'] not in (False,item):raise ValueError('level source appearance')
                bindings.append(dict(position=list(xyz),item=item,ground=top[0],attribute='action',value=field['key'],required_level=WANTED[xyz],source_line=field['line'],map_path=region['path'],map_sha256=region['sha256']))
    if {tuple(b['position']) for b in bindings}!=set(WANTED) or len(bindings)!=len(WANTED):raise ValueError('incomplete level source group')
    return dict(schema=1,classification='OTS_HYPOTHESIS_ONLY',sources=[{k:source[k] for k in ['repository','revision','path','sha256','git_blob_sha1']}],bindings=sorted(bindings,key=lambda b:b['position']))
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();raw=json.dumps(document(),indent=2,ensure_ascii=False)+'\n'
    if args.check:
        if OUTPUT.read_text()!=raw:raise ValueError('level binding drift')
    else:OUTPUT.write_text(raw)
