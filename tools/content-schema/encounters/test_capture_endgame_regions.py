"""Source map custody and exact lever-variable bindings, using synthetic Lua."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import capture_endgame_regions as capture


class EndgameRegionCustody(unittest.TestCase):
    def test_substituted_map_is_rejected_before_reading_layouts(self):
        for raw in (b'',b'OTBM',b'\0\0\0\0\xfe'):
            with self.subTest(raw=raw), self.assertRaisesRegex(ValueError,'pinned Crystal map blob mismatch'):
                capture.capture(raw,Path('/unused'),{})

    def test_constructor_variables_and_all_six_soul_positions_remain_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            soul='SoulWarQuest = { levers = {\n'
            for index,name in enumerate(capture.source.SOUL_CONFIGURATIONS):
                soul += name+'Position = {x='+str(index+1)+',y=10,z=15}, '+name+' = {},\n'
            soul += '} }\n'
            for name in capture.source.SOUL_CONFIGURATIONS:
                soul += 'local '+name+'Lever = BossLever(SoulWarQuest.levers.'+name+')\n'
                soul += name+'Lever:position(SoulWarQuest.levers.'+name+'Position)\n'
            rows=[]
            for index,path in enumerate(capture.source.ENDGAME_PATHS):
                text=soul if index==0 else ('local lever'+str(index)+' = BossLever(config)\n'+
                    'lever'+str(index)+':position(Position('+str(100+index)+',20,15))')
                target=root/path
                target.parent.mkdir(parents=True,exist_ok=True)
                target.write_text(text)
                rows.append({'path':path,'sha256':hashlib.sha256(text.encode()).hexdigest()})
            known={row['path']:row['sha256'] for row in rows if row['path']!=capture.source.SOUL_PATH}
            with patch.object(capture.source,'SOUL_SHA',hashlib.sha256(soul.encode()).hexdigest()), patch.object(capture.source,'ROTTEN_SOURCE_SHA256',known):
                positions=capture.source_levers(root,rows)
                self.assertEqual(len(positions),12)
                self.assertIn((6,10,15),positions)
                self.assertIn((106,20,15),positions)
                target.write_text(text+' -- substituted')
                with self.assertRaisesRegex(ValueError,'M26_SOURCE_SUBSTITUTION'):
                    capture.source_levers(root,rows)
                target.write_text('local lever6 = BossLever(config)\nlever6:position(config.position)')
                rows[-1]['sha256']=hashlib.sha256(target.read_bytes()).hexdigest()
                known[rows[-1]['path']]=rows[-1]['sha256']
                with self.assertRaisesRegex(ValueError,'nonliteral Rotten Blood lever position'):
                    capture.source_levers(root,rows)


if __name__=='__main__':
    unittest.main()
