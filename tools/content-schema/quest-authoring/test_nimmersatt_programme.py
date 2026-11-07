"""Personal source supplies use the shared atomic world reward participant."""
import hashlib
import json
import unittest
import nimmersatt_programme as author

class NimmersattSupply(unittest.TestCase):
    def test_exact_twenty_once_and_selected_recipe_evidence(self):
        rows=[];author.extend_world(rows);row,=rows
        self.assertEqual(row['key'],'oteryn:quest-callback/authored-nimmersatt-snowbank')
        self.assertEqual(row['quest'],author.QUEST)
        self.assertEqual(row['selector']['position'],dict(x=32232,y=31578,floor=-7))
        self.assertEqual(row['selector']['appearance'],799)
        self.assertEqual(row['effects']['item_reward'],dict(item='oteryn:item.tibia.i2992',count=20,contents=[]))
        self.assertEqual(row['storage'],[
            dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=0,maximum=0,set=1),
            dict(track='oteryn:quest-progress/nimmersatt/wall_hits',minimum=-1,maximum=-1,set=0)])
        self.assertTrue(row['sources'][0].startswith('oteryn-authored:'))
        self.assertTrue(any(s.startswith('oteryn:selected-recipe:') for s in row['sources']))

    def test_prior_legion_records_are_preserved(self):
        path='content/quests/authored-world-callbacks.json'
        current=[r for r in json.loads((author.ROOT/path).read_bytes())['callbacks'] if r['quest']=='oteryn:quest.newhaven_legion_helmet_quest']
        self.assertEqual(hashlib.sha256(json.dumps(current,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'b6c631b9db09e2ed0313e17cca8b3161760252cfe854d5fccbd4fb6ff02ea902')


if __name__=='__main__':unittest.main()
