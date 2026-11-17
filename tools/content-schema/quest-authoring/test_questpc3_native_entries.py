import copy
import unittest
import questpc3_native_entries as source


class NativeEntries(unittest.TestCase):
    def test_ancient_aucar_requires_ahau_premium_and_completed_parent(self):
        quest = source.build_ancient_aucar()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.ancient_aucar_outfits_quest')
        self.assertEqual(quest['admission'], {'minimum_level':0, 'premium':True,
            'prerequisites':[{'requirement':'Galthen and the City of Gold',
                              'quest':'oteryn:quest.adventures_of_galthen_quest'}]})
        self.assertFalse(quest['stages'][0]['completes'])
        counter = quest['stages'][0]['trigger']['counter']
        self.assertEqual(counter['creature'], 'oteryn:creature.ahau')
        self.assertEqual((counter['gate_value'], counter['minimum'], counter['maximum'],
                          counter['advance_gate_to']), (0, 0, 1, 1))

    def test_draconia_requires_its_two_real_numbered_doors_and_level_25(self):
        quest = source.build_draconia()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.draconia_quest')
        self.assertEqual(quest['admission'], {'minimum_level':25})
        self.assertEqual(len(quest['stages']), 2)
        expected = {(x << 32) | (y << 16) | (246 << 8) | 1
                    for x, y in [(32675, 31649), (32676, 31671)]}
        self.assertEqual({s['trigger']['usage']['native_placement_key']
                          for s in quest['stages']}, expected)
        for stage in quest['stages']:
            self.assertFalse(stage['completes'])
            self.assertEqual(stage['trigger']['kind'], 'door_use')
            usage = stage['trigger']['usage']
            self.assertEqual(usage['key_number'], 3012)
            self.assertEqual(usage['storage'], [{'track':'oteryn:quest-progress/m01c-draconia-route',
                                               'minimum':0, 'maximum':0, 'set':1}])

    def test_task_board_entries_preserve_requirements_and_never_complete(self):
        for slug, quest, level in [('falconer', 'oteryn:quest.falconer_outfits_quest', 100), ('feral-trappers', 'oteryn:quest.feral_trappers_quest', 0)]:
            bound = source.build_task_board(slug)['quests'][0]
            self.assertEqual(bound['quest'], quest)
            self.assertEqual(bound['admission'], {'minimum_level':level})
            self.assertEqual(len(bound['stages']), 1)
            stage = bound['stages'][0]
            self.assertFalse(stage['completes'])
            self.assertEqual(stage['trigger']['kind'], 'task_board_use')
            self.assertEqual(stage['trigger']['usage']['programme'], 'weekly')
            self.assertEqual(stage['trigger']['usage']['storage'][0]['set'], 1)

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

    def test_grimvale_species_share_one_bounded_active_mission_counter(self):
        quest = source.build_grimvale()['quests'][0]
        counters = [stage['trigger']['counter'] for stage in quest['stages'] if stage['trigger']['kind'] == 'kill']
        self.assertEqual({c['creature'] for c in counters}, {'oteryn:creature.werewolf','oteryn:creature.werebear','oteryn:creature.wereboar'})
        self.assertEqual(len({c['track'] for c in counters}), 1)
        for counter in counters:
            self.assertEqual((counter['gate_value'],counter['minimum'],counter['maximum'],counter['advance_gate_to']), (1,0,10,2))

    def test_decaying_defender_starts_only_with_its_exact_native_boss_and_parent(self):
        quest = source.build_decaying_defender()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.decaying_defender_outfits_quest')
        self.assertEqual(quest['admission'], {
            'minimum_level':250, 'premium':True,
            'prerequisites':[{'requirement':'Rotten Blood journey','quest':'oteryn:quest/rotten-blood'}],
        })
        stage = quest['stages'][0]
        self.assertFalse(stage['completes'])
        counter = stage['trigger']['counter']
        self.assertEqual(counter['creature'], 'oteryn:creature.bakragore')
        self.assertEqual((counter['gate_value'],counter['minimum'],counter['maximum'],counter['advance_gate_to']), (0,0,1,1))

    def test_illuminator_shell_entry_preserves_level_premium_and_parent(self):
        quest = source.build_illuminator()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.illuminator_outfits_quest')
        self.assertEqual(quest['admission'], {'minimum_level':200,'premium':True,
            'prerequisites':[{'requirement':'Between the Lines Quest','quest':'oteryn:quest.between_the_lines_quest'}]})
        self.assertEqual(len(quest['stages']), 1)
        self.assertFalse(quest['stages'][0]['completes'])
        collection = quest['stages'][0]['trigger']['collection']
        self.assertEqual(collection['items'], [{'item':'oteryn:item.tibia.i51656','count':1}])
        self.assertEqual(collection['storage'][0]['set'], 1)

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
