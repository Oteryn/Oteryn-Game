"""Source contracts for both Wizard addon acquisitions."""
import json
import unittest
import qengine_rewards as q


class WizardSources(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.callbacks=json.loads(q.build()['content/quests/shared-reward-callbacks.json'])

    def test_lugri_wizard_skull_source_steps(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        track='oteryn:quest-progress/quest/u7_8/wizard_outfits'
        for stage,suffix,item in [(0,'start',None),(1,'shield',3436),(2,'mail',3386),(3,'legs',3382),(4,'skull',3006)]:
            row=rows['l2-wizard-'+suffix]
            self.assertEqual(row['npc'],'oteryn:npc.lugri')
            self.assertEqual(row['topic'],stage+2)
            self.assertEqual(row['storage'][0],{'track':track,'minimum':stage,'maximum':stage,'set':stage+1})
            self.assertEqual(row['take'],[] if item is None else [{'item':f'oteryn:item.tibia.i{item}','count':1}])
            self.assertEqual(row['outfit'],{'outfit':'oteryn:outfit.wizard','addons':2} if stage==4 else None)
            self.assertFalse(row.get('requires_premium',False))
            self.assertIsNone(row['achievement'])

    def test_wizard_flowers_follow_skull_and_award_warlock(self):
        rows={c['key'].rsplit('/',1)[-1]:c for c in self.callbacks['callbacks']}
        start=rows['l2-wizard-flowers-start'];reward=rows['l2-wizard-flowers']
        self.assertEqual((start['storage'][0]['minimum'],start['storage'][0]['set'],start['topic']),(5,6,10))
        self.assertEqual((reward['storage'][0]['minimum'],reward['storage'][0]['set'],reward['topic']),(6,7,11))
        self.assertEqual(reward['take'],[{'item':'oteryn:item.tibia.i5922','count':50}])
        self.assertEqual(reward['outfit'],{'outfit':'oteryn:outfit.wizard','addons':1})
        self.assertEqual(reward['achievement'],'oteryn:achievement/warlock')
        self.assertEqual(reward['npc'],'oteryn:npc.the_queen_of_the_banshees')
