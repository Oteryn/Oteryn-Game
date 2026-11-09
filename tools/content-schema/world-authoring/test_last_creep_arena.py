"""M23 authored geometry: exact codec, no peer overwrite and no raw teleport."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
CODEC_ROOT=Path(os.environ.get('M23_TEST_REPO_ROOT',str(ROOT)))
sys.path.insert(0,str(CODEC_ROOT/'tools/content-schema/world-authoring'))
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module);return module
arena=load('m23_arena_test',HERE/'last_creep_arena.py')
source=load('m23_source_test',HERE.parent/'quest-authoring/last_creep_native.py')

class LastCreepArena(unittest.TestCase):
    def setUp(self):
        self.raw=(ROOT/'content/quests/last-creep-native.json').read_bytes()
        self.document=json.loads(self.raw)
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        self.base=Path(temporary.name)
        palette=[{'key':f'oteryn:item.tibia.i{item}','provisional':False,'source_item_id':item}for item in [231,409,1949]]
        positions=[arena.native(self.document[name])for name in ['start','outside']]
        sx,sy=positions[0][0]//32,positions[0][1]//32
        self.assertTrue(all((x//32,y//32)==(sx,sy)for x,y,z in positions))
        tiles=[(x,y,0,0,(),[(0,0,None)])for x,y,z in sorted(positions,key=lambda at:(at[1],at[0]))]
        region='content/world/placements/'+arena.codec.region_name(7,sx//8,sy//8)
        raw=arena.codec.encode_region(7,sx//8,sy//8,{(sy%8)*8+sx%8:arena.codec.encode_sector_tiles(sx,sy,tiles)})
        totals={'regions':1,'sectors':1,'tiles':2,'items':2}
        row={'path':region,'sha256':arena.digest(raw),'tiles':2,'items':2}
        index={'source':{},'regions':[row],'shards':[region],'palette':palette,'totals':totals}
        summary={'source':{},'totals':totals.copy(),'bytes_on_disk':len(raw),'tiles_by_floor':{'7':2},
                 'palette':{'entries':3,'families':{'item':{'entries':3,'occurrences':2}}}}
        bounds={'records':[{'declaration':{'bounds':{'min_x':0,'min_y':0,'max_x_exclusive':65536,'max_y_exclusive':65536}}}]}
        for path,value in [(arena.INDEX,index),(arena.SUMMARY,summary),('content/world/worlds/worlds-00000-00000.json',bounds)]:
            target=self.base/path;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(arena.canonical(value))
        target=self.base/region;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
        self.region=region
    def test_exact_source_ids_and_direct_canonical_stage_guards(self):
        actual=source.build(root=ROOT)
        self.assertEqual(actual,self.document)
        self.assertEqual([f['appearance']for f in actual['forms']],[12052,12047,12053,12048,12056,12049,12050,12051])
        for index,form in enumerate(actual['forms'],1):
            self.assertEqual(form['previous'],f'oteryn:quest-progress/authored/last_creep_standing/s{index}')
            self.assertEqual(form['stage'],f'oteryn:quest-progress/authored/last_creep_standing/s{index+1}')
        self.assertFalse(actual['source_equivalence']);self.assertNotIn('phase_track',actual)
    def test_static_callback_portals_exact_layout_idempotence_and_peer_preservation(self):
        outputs={};arena.overlay_outputs(outputs,self.base,self.raw)
        first=dict(outputs);arena.overlay_outputs(outputs,self.base,self.raw);self.assertEqual(first,outputs)
        self.assertEqual(set(outputs),{arena.INDEX,arena.SUMMARY,arena.ARENA,self.region})
        index=json.loads(outputs[arena.INDEX]);proof=index['source'][arena.SOURCE]
        self.assertEqual((proof['tiles_added'],proof['items_added'],proof['raw_teleports_added']),(484,503,0))
        for _,tiles in arena.codec.decode_region(outputs[arena.ARENA])[3]:
            for _,_,flags,house,zones,items in tiles:
                self.assertEqual((flags,house,zones),(1,0,()))
                self.assertTrue(all(depth==0 and attrs is None for _,depth,attrs in items))
    def test_preexisting_arena_region_refuses_instead_of_replacing_peer(self):
        index=json.loads((self.base/arena.INDEX).read_bytes())
        index['regions'].append({'path':arena.ARENA,'sha256':'0'*64,'tiles':2,'items':2})
        outputs={arena.INDEX:arena.canonical(index)}
        with self.assertRaisesRegex(ValueError,'COLLIDES_WITH_EXISTING_WORLD'):
            arena.overlay_outputs(outputs,self.base,self.raw)

if __name__=='__main__':unittest.main()
