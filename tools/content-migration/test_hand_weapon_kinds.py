import copy
import json
from pathlib import Path
import tempfile
import unittest

import hand_weapon_kinds as hand


class HandKinds(unittest.TestCase):
    def setUp(self):
        self.profiles={'records':[{'production_definition':{'production_key':'shield'},
            'admission':{'materializable':True},'semantics':{'weapon':{'state':'KNOWN','value':{
                'weapon_type':{'state':'UNKNOWN'},'defense':{'state':'KNOWN','value':34}}}}},
            {'production_definition':{'production_key':'other'},'semantics':{'weapon':{'state':'UNKNOWN'}}}]}
        self.facts={'records':[{'key':'shield','weapon_type':'SHIELD'}]}

    def test_kind_completion_preserves_numeric_owner_facts_and_other_items(self):
        before=copy.deepcopy(self.profiles)
        result=hand.complete(self.profiles,self.facts)
        self.assertEqual(self.profiles,before)
        self.assertEqual(result['records'][0]['semantics']['weapon']['value']['defense'],{'state':'KNOWN','value':34})
        self.assertEqual(result['records'][0]['admission'],before['records'][0]['admission'])
        self.assertEqual(result['records'][1],before['records'][1])
        self.assertEqual(result['records'][0]['semantics']['weapon']['value']['weapon_type'],{'state':'KNOWN','value':'SHIELD'})
        self.assertEqual(hand.complete(result,self.facts),result)

    def test_known_conflict_and_not_applicable_refuse(self):
        self.profiles['records'][0]['semantics']['weapon']['value']['weapon_type']={'state':'KNOWN','value':'AXE'}
        with self.assertRaisesRegex(ValueError,'KNOWN_CONFLICT'):hand.complete(self.profiles,self.facts)
        self.profiles['records'][0]['semantics']['weapon']={'state':'NOT_APPLICABLE'}
        with self.assertRaisesRegex(ValueError,'CONTRADICTORY'):hand.complete(self.profiles,self.facts)

    def test_capture_agrees_with_existing_identity_and_pinned_xml(self):
        profiles=json.loads((hand.ROOT/hand.PROFILE).read_bytes())
        facts=hand.capture(hand.ROOT,profiles)
        self.assertEqual(facts,json.loads((hand.ROOT/hand.PROOF).read_bytes()))
        row=next(r for r in facts['records'] if r['source_id']==3425)
        self.assertEqual((row['weapon_type'],row['source_line']),('SHIELD',10447))
        profiles['records'][0]['authoring']['taxonomy']={'primary':'Shields'}
        with self.assertRaises(ValueError):hand.capture(hand.ROOT,profiles)

    def test_source_pin_mismatch_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for file in (hand.XML,hand.BINDINGS):
                path=root/file;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError,'SOURCE_PIN'):hand.capture(root,{'records':[]})


if __name__=='__main__':unittest.main()
