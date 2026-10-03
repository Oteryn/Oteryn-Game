"""A repeated effect still binds each distinct source call to its transition."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from collections import Counter

import ots_interactions as interactions
import ots_questlog as quests


class ProgressOccurrenceTests(unittest.TestCase):
    def test_repeated_npc_writes_keep_both_contexts_and_call_bindings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / quests.SOURCES['canary']['datapack'] / 'npc/example.lua'
            script.parent.mkdir(parents=True)
            script.write_text('local function creatureSayCallback(npc, creature, type, message)\n'
                              '  local player = Player(creature)\n'
                              '  if MsgContains(message, "first") then\n'
                              '    player:setStorageValue(Storage.Quest.Example.Track, 1)\n'
                              '  end\n'
                              '  if MsgContains(message, "second") then\n'
                              '    player:setStorageValue(Storage.Quest.Example.Track, 1)\n'
                              '  end\nend\n')
            found = quests.transition_index({'canary': root})[quests.norm('quest/example/track')]
            self.assertEqual(found['count']['canary'], 2)
            self.assertEqual(len(found['transitions']), 1)
            transition = quests.progress_transition(quests.mission_transitions(found)[0])
            occurrences = transition['source_occurrences']
            self.assertEqual([row['line'] for row in occurrences], [4, 7])
            self.assertEqual([row['write']['requested_by']['keywords'] for row in occurrences],
                             [['first'], ['second']])
            self.assertEqual(occurrences[1]['blob_sha256'], hashlib.sha256(script.read_bytes()).hexdigest())
            key = 'canary:quest-progress/quest/example/track'
            (root / 'progress.json').write_text(json.dumps({'progress': [{'key': key, 'transitions': [transition]}]}))
            (root / 'quests.json').write_text(json.dumps({'quests': [{'identity': {'key': 'canary:quest/example'},
                'missions': [{'key': 'mission', 'progress': key}]}]}))
            lookup = interactions.transition_keys(root)
            for line in [4, 7]:
                self.assertEqual(lookup[(quests.norm('quest/example/track'), 'npc/example.lua', line)],
                                 'canary:quest/example#mission:npc_1')

    def test_unexpected_action_writer_blocks_curated_npc_before_all_fallbacks(self):
        target='quest/u10_50/glooth_engineer_outfits/addon1';quest='canary:quest/glooth'
        npc='npc/ezebeth.lua';other='scripts/quests/unknown/shared.lua'
        entries={'npc':{'owner':'npc','callback':None,'from':None,'to':1,'script':npc,
                        'sources':{'canary':{'path':'data-otservbr-global/'+npc,'line':1,
                                             'registrations':[],'dialogue':{'keywords':['yes'],'topics':[1]}}}},
                 'action':{'owner':'action','callback':'onUse','from':None,'to':1,'script':other,
                           'sources':{'canary':{'path':'data-otservbr-global/'+other,'line':1,'registrations':[]}}}}
        found={'count':Counter({'canary':2}),'paths':{('canary',target)},'transitions':entries}
        catalogue=[{'identity':{'key':quest},'display_name':'Glooth','missions':[]},
                   {'identity':{'key':'canary:quest/hero'},'display_name':'Hero',
                    'missions':[{'progress':'canary:quest-progress/quest/u10_50/hero/mission'}]}]
        curator={quests.norm(target):{'quest':quest,'npc_source':npc}}
        gate={'identity':{'key':'canary:door-gate/progress/'+target},'quest':{'key':quest},
              'condition':{'progress':'canary:quest-progress/'+target}}
        for gates in [[],[gate]]:
            with self.subTest(gates=bool(gates)), patch.object(quests,'TRACK_OWNERS',curator), patch.object(quests,'storage_declarations',return_value={}):
                result=quests.auxiliary_tracks({quests.norm(target):found},[],catalogue,gates,{})[0]
            self.assertEqual(result['owner_basis'],'UNKNOWN')
            self.assertEqual(result['auxiliary_of'],[])
            self.assertIn('writer set changed',result['source_checks']['owner'])
            self.assertEqual(len(result['transitions']),2)


if __name__ == '__main__':
    unittest.main()
