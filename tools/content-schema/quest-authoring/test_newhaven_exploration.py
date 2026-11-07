"""Closed selected-tour bindings preserve all prior access callbacks."""
import hashlib
import json
from pathlib import Path
import unittest
import qengine_access as access

class NewhavenExplorationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document=access.document()

    def test_two_exact_source_pois_and_once_only_tour_predicates(self):
        rows={r['key']:r for r in self.document['callbacks'] if r['key'].startswith('oteryn:quest-callback/newhaven-')}
        self.assertEqual(set(rows),{'oteryn:quest-callback/newhaven-school','oteryn:quest-callback/newhaven-supply'})
        for suffix,x,y in [('school',32570,32507),('supply',32567,32489)]:
            r=rows['oteryn:quest-callback/newhaven-'+suffix]
            self.assertEqual(r['selector'],dict(position=dict(x=x,y=y,floor=-7),unique=None,action=None,appearance=None))
            self.assertEqual(r['trigger'],'step_in')
            self.assertEqual(r['minimum_level'],2)
            self.assertEqual(r['storage'],[
                dict(track='oteryn:quest-progress/newhaven/stage',minimum=0,maximum=0,set=0,preserve=True),
                dict(track='oteryn:quest-progress/newhaven/'+suffix,minimum=-1,maximum=0,set=1)])
            self.assertIsNone(r['teleport'])
            self.assertEqual(r['offerings'],[])
            self.assertTrue(all(value is None for value in r['effects'].values()))
            self.assertIn('data-global/world/world-npc.xml:',r['sources'][0])
            self.assertTrue(r['sources'][1].startswith('oteryn:selected-recipe:'))

    def test_checked_source_producer_reproduces_committed_registry(self):
        self.assertEqual((json.dumps(self.document,ensure_ascii=False,indent=2)+'\n').encode(),access.OUTPUT.read_bytes())

    def test_prior_access_records_are_byte_for_byte_semantically_unchanged(self):
        old=[r for r in self.document['callbacks'] if not r['key'].startswith('oteryn:quest-callback/newhaven-')]
        self.assertEqual(hashlib.sha256(json.dumps(old,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'e07267fc4fa16400571448efeb4e3521543e060273de6c4038ec94863cacd267')

if __name__=='__main__':unittest.main()
