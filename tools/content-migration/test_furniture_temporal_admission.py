"""Pinned off-wand semantics must survive the owning furniture admission merge."""
import copy
import json
from pathlib import Path
import unittest
import furniture_item_profiles as owner

class FurnitureTemporalAdmission(unittest.TestCase):
    def test_source_off_wand_replaces_only_the_ambiguous_shared_page_duration(self):
        overlay=json.loads((owner.ROOT/owner.PROFILE).read_bytes())
        source=next(r for r in overlay['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i3046')
        self.assertEqual(source['semantics']['temporal'],{'state':'NOT_APPLICABLE'})
        # This is the actual older admitted shared-Wiki-page shape observed in
        # Native19. The pinned XML6886–6890 gives no active duration for3046.
        previous=copy.deepcopy(source)
        previous['semantics']['temporal']={'state':'KNOWN','value':{'consumption_mode':{'state':'UNKNOWN'},'decay_target':{'state':'UNKNOWN'},'duration':{'state':'KNOWN','value':3000000},'stop_duration':{'state':'UNKNOWN'}}}
        profile='profiles';manifest='manifest'
        generated={profile:json.dumps({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[previous]}).encode(),manifest:json.dumps({'item_profiles':{'sha256':'0'*64}}).encode()}
        owner.admit(owner.ROOT,generated,profile,manifest,lambda r:r['production_definition']['production_key'],lambda value:json.dumps(value).encode())
        after=next(r for r in json.loads(generated[profile])['records']if r['production_definition']['production_key']=='oteryn:item.tibia.i3046')
        self.assertEqual(after['semantics']['temporal'],source['semantics']['temporal'])
        before_other=copy.deepcopy(previous);after_other=copy.deepcopy(after)
        del before_other['semantics']['temporal'];del after_other['semantics']['temporal']
        self.assertEqual(after_other,before_other)

    def test_existing_known_absolute_policy_is_preserved(self):
        overlay=json.loads((owner.ROOT/owner.PROFILE).read_bytes())
        source=next(r for r in overlay['records'] if r['production_definition']['production_key']=='oteryn:item.tibia.i3046')
        previous=copy.deepcopy(source)
        previous['semantics']['temporal']={'state':'KNOWN','value':{'consumption_mode':{'state':'KNOWN','value':'DURABLE_ABSOLUTE_DEADLINE'},'decay_target':{'state':'NOT_APPLICABLE'},'duration':{'state':'KNOWN','value':3000000},'stop_duration':{'state':'KNOWN','value':False}}}
        generated={'profiles':json.dumps({'records':[previous]}).encode(),'manifest':json.dumps({'item_profiles':{'sha256':'0'*64}}).encode()}
        owner.admit(owner.ROOT,generated,'profiles','manifest',lambda r:r['production_definition']['production_key'],lambda value:json.dumps(value).encode())
        after=next(r for r in json.loads(generated['profiles'])['records']if r['production_definition']['production_key']=='oteryn:item.tibia.i3046')
        self.assertEqual(after,previous)

if __name__=='__main__':unittest.main()
