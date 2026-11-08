"""Personal source supplies use the shared atomic world reward participant."""
import json
import unittest
import nimmersatt_programme as author
import legion_helmet

class NimmersattSupply(unittest.TestCase):
    def test_premium_passage_and_exact_completed_counter_turn_in(self):
        rows=[];author.extend_rewards(rows)
        passage,breath=rows
        self.assertTrue(passage['premium_required'])
        self.assertTrue(breath['premium_required'])
        self.assertEqual(passage['storage'],[dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=-1,maximum=-1,set=0)])
        self.assertEqual(breath['storage'][-1],dict(track='oteryn:quest-progress/nimmersatt/wall_hits',minimum=20,maximum=20,set=20,preserve=True))
        self.assertEqual(breath['give'],dict(item='oteryn:item.tibia.i44179',count=1,contents=[]))
        self.assertTrue(breath['completes'])

    def test_exact_twenty_once_and_selected_recipe_evidence(self):
        rows=[];author.extend_world(rows);row,=[r for r in rows if r['key'].endswith('-snowbank')]
        self.assertEqual(row['key'],'oteryn:quest-callback/authored-nimmersatt-snowbank')
        self.assertEqual(row['quest'],author.QUEST)
        self.assertTrue(row['premium_required'])
        self.assertEqual(row['selector']['position'],dict(x=32232,y=31578,floor=-7))
        self.assertEqual(row['selector']['appearance'],799)
        self.assertEqual(row['effects']['item_reward'],dict(item='oteryn:item.tibia.i2992',count=20,contents=[]))
        self.assertEqual(row['storage'],[
            dict(track='oteryn:quest-progress/nimmersatt/stage',minimum=0,maximum=0,set=1),
            dict(track='oteryn:quest-progress/nimmersatt/wall_hits',minimum=-1,maximum=-1,set=0)])
        self.assertTrue(row['sources'][0].startswith('oteryn-authored:'))
        self.assertTrue(any(s.startswith('oteryn:selected-recipe:') for s in row['sources']))

    def test_wall_counts_exact_twenty_consumed_inputs_then_projects(self):
        rows=[];author.extend_world(rows);row,=[r for r in rows if r['key'].endswith('-wall')]
        self.assertEqual(row['selector']['position'],dict(x=32236,y=31582,floor=-7))
        self.assertIsNone(row['selector']['appearance'])
        self.assertEqual(row['item_on_target'],dict(item='oteryn:item.tibia.i2992',count=1,replacement=None,target_replacement=None))
        self.assertEqual(row['counted_use'],dict(track='oteryn:quest-progress/nimmersatt/wall_hits',limit=20,project_on_complete=True))
        self.assertEqual(row['effects']['transform'],799)

    def test_prior_legion_records_are_preserved(self):
        path='content/quests/authored-world-callbacks.json'
        current=[r for r in json.loads((author.ROOT/path).read_bytes())['callbacks'] if r['quest']=='oteryn:quest.newhaven_legion_helmet_quest']
        baseline=legion_helmet.world_callbacks()['callbacks']
        self.assertEqual(current,baseline)
        author.extend_world(baseline)
        self.assertEqual(current,[row for row in baseline if row['quest']=='oteryn:quest.newhaven_legion_helmet_quest'])


if __name__=='__main__':unittest.main()
