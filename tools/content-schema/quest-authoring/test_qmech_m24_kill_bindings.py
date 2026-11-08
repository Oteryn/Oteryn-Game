import json
import tempfile
import unittest
from pathlib import Path
import qmech_m24_kill_bindings as bindings

class M24BindingsTest(unittest.TestCase):
    def test_committed_bindings_match_generator(self):
        for name,value in bindings.generate(bindings.ROOT).items():
            committed=json.loads((bindings.ROOT/bindings.OUTPUT/name).read_text())
            self.assertEqual(committed,value)

    def test_selected_cake_count_is_twenty_and_pharaoh_steps_are_exact(self):
        outputs=bindings.generate(bindings.ROOT)
        cake=outputs['qmech-m24-a_piece_of_cake.json']['rules']
        self.assertEqual(1,len(cake))
        self.assertEqual('oteryn:creature.cake_golem',cake[0]['creature'])
        increment=[e for e in cake[0]['effects'] if e.get('add')==1]
        self.assertEqual(1,len(increment))
        self.assertEqual({'kind':'lt','value':20},increment[0]['from'])
        self.assertNotIn(increment[0]['track'],cake[0]['epoch_tracks'])
        pharaohs=outputs['qmech-m24-ancient_tombs.json']['rules']
        self.assertEqual({'omruc','thalas','dipthrah','mahrdis','vashresamun','morguthis','rahemos'},
                         {r['creature'].rsplit('.',1)[1] for r in pharaohs})

    def test_rejects_substitution_of_the_shared_world_total(self):
        value=json.loads((bindings.ROOT/bindings.INPUT).read_text())
        quest=next(q for q in value['quests'] if q['quest']=='oteryn:quest.a_piece_of_cake')
        stage=next(t for t in quest['transitions'] if t.get('source',{}).get('chosen_stage',{}).get('key')=='s2')
        stage['source']['chosen_stage']['count']=200
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            target=root/bindings.INPUT;target.parent.mkdir(parents=True);target.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError,'preserve the selected kill stage/count'):
                bindings.generate(root)

if __name__=='__main__':
    unittest.main()
