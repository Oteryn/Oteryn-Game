"""Selected M34 allocations must preserve distinct claim and fragment identities."""
import hashlib
import importlib.util
import json
import sys
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/reward-claim-authoring'))
import engclaim_native as native

class NativeClaimTest(unittest.TestCase):
    def test_every_allocation_has_exact_callback_and_semantic_digest(self):
        claims,doc=native.records();callbacks={c['world_claim']['key']:c for c in doc['callbacks']}
        self.assertEqual(len(claims),9)
        for row in claims:
            d=row['definition'];c=callbacks[d['identity']['key']]
            terms={k:d[k] for k in ('identity','claim','placements')}
            self.assertEqual(c['world_claim']['authority_sha256'],hashlib.sha256(native.canonical(terms).rstrip(b'\n')).hexdigest())
            p=d['placements'][0];self.assertNotIn('legacy_unique_ids',p['source_binding'])
            actual=[(c['give']['item'],c['give']['count'])]+[(i['item'],i['count']) for i in c['give']['additional']]
            self.assertEqual(actual,[(i['item']['key'],i['count']) for i in p['reward']['items']])

    def test_native_callbacks_pass_the_shared_strict_schema(self):
        from jsonschema import Draft202012Validator
        schema=json.loads((ROOT/'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json').read_bytes())
        Draft202012Validator(schema).validate(native.records()[1])

    def test_final_statue_takes_three_distinct_fragments(self):
        _,doc=native.records();c=next(c for c in doc['callbacks'] if c['key'].endswith('valley-guardian'))
        self.assertEqual([i['item'] for i in c['take']],[f'oteryn:item.tibia.i{i}' for i in (50336,50337,50338)])
        self.assertTrue(c['requires_premium']);self.assertTrue(c['completes'])
        self.assertEqual(c['give']['item'],'oteryn:item.tibia.i50335')

    def test_personal_caches_consolidate_exact_gold_without_a_bag(self):
        _,doc=native.records();c=next(c for c in doc['callbacks'] if c['key'].endswith('potion-fire'))
        self.assertEqual(sum(i['count'] for i in c['give']['additional'] if i['item'].endswith('.i3031')),201)
        self.assertEqual(c['give']['contents'],[])

if __name__=='__main__':unittest.main()
