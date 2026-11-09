import copy
import unittest
import questpc3_native_entries as source


class NativeEntries(unittest.TestCase):
    def test_dead_archer_entry_is_exact_and_cannot_complete_or_grant(self):
        document = source.build()
        quest = document['quests'][0]
        self.assertEqual(quest['quest'], source.QUEST)
        self.assertEqual(len(quest['stages']), 4)
        key = (32493 << 32) | (32259 << 16) | (7 << 8)
        self.assertEqual({s['trigger']['usage']['result'] for s in quest['stages']}, {594,607,609,868})
        for stage in quest['stages']:
            self.assertFalse(stage['completes'])
            self.assertEqual(stage['trigger']['usage']['placement_key'], key)
            self.assertEqual(stage['trigger']['usage']['storage'][0]['set'], 1)

    def test_fanfare_collection_requires_the_exact_numbered_copper_key(self):
        quest = source.build_fanfare()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.fanfare_quest')
        self.assertEqual(len(quest['stages']), 1)
        stage = quest['stages'][0]
        self.assertFalse(stage['completes'])
        collection = stage['trigger']['collection']
        self.assertEqual(collection['items'], [{'item':'oteryn:item.tibia.i2973','count':1,'key_number':3520}])
        self.assertEqual(collection['storage'][0]['set'], 1)
        self.assertEqual(quest['tracks'][0]['maximum'], 4)

    def test_black_knight_collection_binds_its_real_key_and_level(self):
        quest = source.build_black_knight()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.black_knight_quest')
        self.assertEqual(quest['admission'], {'minimum_level':50})
        stage = quest['stages'][0]
        self.assertFalse(stage['completes'])
        self.assertEqual(stage['trigger']['collection']['items'], [{'item':'oteryn:item.tibia.i2969','count':1,'key_number':5010}])

    def test_grimvale_requires_five_harvested_blossoms_and_premium(self):
        quest = source.build_grimvale()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.grimvale_quest')
        self.assertEqual(quest['admission'], {'minimum_level':0,'premium':True})
        stage = quest['stages'][0]
        self.assertFalse(stage['completes'])
        self.assertEqual(stage['trigger']['collection']['items'], [{'item':'oteryn:item.tibia.i22082','count':5}])

    def test_actor_admission_changes_fail_closed(self):
        path, recipe = source.chosen()
        changed = copy.deepcopy(recipe)
        changed['requirements']['min_level'] = 50
        # Exercise the canonical reader with a modified definition in an isolated tree.
        import json
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            target = root / path
            target.parent.mkdir(parents=True)
            definition = {'identity':{'key':source.QUEST},'oteryn_recipe':{
                'profile':'chosen_source_completion_v1','chosen_data_complete':True,
                'payload':{'recipe':changed}}}
            target.write_text(json.dumps({'records':[{'definition':definition}]}))
            with self.assertRaisesRegex(ValueError,'ACTOR_ADMISSION'):
                source.chosen(root)


if __name__ == '__main__':
    unittest.main()
