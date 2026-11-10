import copy
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

    def test_herald_preserves_three_grouped_branches_for_top_damage_only(self):
        document = bindings.herald_binding(bindings.ROOT)
        rules = document['rules']
        self.assertEqual(3, len(rules))
        self.assertEqual({'mission-1', 'mission-2', 'progressed'},
                         {r['transition'].removeprefix(bindings.HERALD_PREFIX) for r in rules})
        committed = json.loads((bindings.ROOT / bindings.lowering.OUTPUT).read_text())
        transitions = {t['key']: t for q in committed['quests'] for t in q['transitions']}
        for rule in rules:
            self.assertEqual('oteryn:creature.herald_of_fire', rule['creature'])
            self.assertTrue(rule['top_damage_only'])
            self.assertFalse(rule['define_transition'])
            self.assertEqual(2, len(rule['effects']))
            self.assertEqual(2, len(rule['sources']))
            expected = transitions[rule['transition']]['effects']
            actual = [{'track': e['track'],
                       'from': {'op': e['from']['kind'].upper(), 'value': e['from']['value']},
                       'from_exact': True,
                       'effect': {'kind': 'SET' if 'set' in e else 'ADD',
                                  'value': e.get('set', e.get('add'))}}
                      for e in rule['effects']]
            self.assertEqual(expected, actual)

    def test_herald_rejects_source_witness_drift_before_binding(self):
        original = json.loads((bindings.ROOT / bindings.HERALD_INPUT).read_text())
        for field in ('blob_sha256', 'line_sha256', 'revision', 'target', 'write'):
            document = copy.deepcopy(original)
            definition = next(row['definition'] for row in document['records']
                              if row['definition']['identity']['key'] == bindings.HERALD_QUEST)
            progress = next(p for p in definition['source_data']['progress']
                            if p['key'].endswith('burning_heart/mission'))
            transition = next(t for t in progress['transitions'] if 'herald_of_fire' in t['script'])
            transition['source_occurrences'][0][field] = 'wrong-source'
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / bindings.HERALD_INPUT
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps(document))
                with self.assertRaisesRegex(ValueError, 'Herald source witness changed'):
                    bindings.herald_binding(root)

    def test_herald_rejects_grouped_runtime_payload_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / bindings.HERALD_INPUT
            source.parent.mkdir(parents=True)
            source.write_bytes((bindings.ROOT / bindings.HERALD_INPUT).read_bytes())
            payload = json.loads((bindings.ROOT / bindings.lowering.OUTPUT).read_text())
            transition = next(t for q in payload['quests'] for t in q['transitions']
                              if t['key'] == bindings.HERALD_PREFIX + 'mission-1')
            transition['effects'][0]['effect']['value'] = 2
            target = root / bindings.lowering.OUTPUT
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(payload))
            with self.assertRaisesRegex(ValueError, 'Herald grouped runtime payload drift'):
                bindings.herald_binding(root)

if __name__=='__main__':
    unittest.main()
