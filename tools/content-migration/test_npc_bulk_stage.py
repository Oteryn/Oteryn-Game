import copy
import json
import unittest

import npc_bulk_stage as stage

EVIDENCE = stage.ROOT / 'docs/agents/evidence/OTV2-20261002-npc-bulk-first45'


class BulkStageTests(unittest.TestCase):
    def setUp(self):
        self.profiles = json.loads((EVIDENCE / 'source-profiles.json').read_text())

    def test_closed_batch_and_repeatable_packet(self):
        packet, progress = stage.build(self.profiles)
        self.assertEqual(stage.canonical(packet), (EVIDENCE / 'native-additions.json').read_bytes())
        self.assertEqual(len(progress), 45)
        self.assertEqual(len({r['key'] for r in progress}), 45)
        self.assertEqual(sum(r['field_quality']['presentation'] == 'donor' for r in progress), 7)
        self.assertTrue(all(not r['native_runtime_loaded'] for r in progress))
        self.assertEqual({d['kind'] for d in packet['native_additions']['declarations']}, {'NPC', 'Dialogue'})
        self.assertTrue(all(not d['services'] for d in packet['native_additions']['declarations'] if d['kind'] == 'NPC'))

    def test_unknown_appearance_is_explicit_placeholder(self):
        packet, progress = stage.build(self.profiles)
        goblin = next(r for r in progress if r['key'] == 'oteryn:npc.a_runaway_goblin')
        self.assertEqual(goblin['field_quality']['presentation'], 'placeholder')
        self.assertEqual(goblin['field_quality']['dialogue'], 'placeholder')
        profile = next(p for p in packet['native_additions']['authoring_profiles']
                       if p['target']['key'] == 'oteryn:presentation.npc.a_runaway_goblin')
        self.assertEqual(profile['data']['profile']['asset_binding'], 'canary.appearance:outfit/128')

    def test_missing_radius_default_does_not_become_donor_fact(self):
        _, progress = stage.build(self.profiles)
        actor = next(r for r in progress if r['key'] == 'oteryn:npc.blubster')
        self.assertEqual(actor['field_quality']['movement.walk_radius'], 'defaulted')
        self.assertEqual(actor['field_quality']['presentation'], 'donor')
        self.assertEqual(actor['field_quality']['presentation.mount'], 'defaulted')

    def test_related_actor_alias_is_rejected(self):
        profiles = copy.deepcopy(self.profiles)
        profiles['oteryn:npc.a_runaway_goblin'] = copy.deepcopy(profiles['oteryn:npc.agostina'])
        profiles['oteryn:npc.a_runaway_goblin']['source']['literal_name'] = 'Goblin Exile'
        with self.assertRaisesRegex(ValueError, 'identity mismatch'):
            stage.build(profiles)

    def test_unknown_target_and_bad_source_custody_are_rejected(self):
        profiles = copy.deepcopy(self.profiles)
        profiles['oteryn:npc.unrelated'] = profiles['oteryn:npc.agostina']
        with self.assertRaisesRegex(ValueError, 'unknown'):
            stage.build(profiles)
        profiles = copy.deepcopy(self.profiles)
        profiles['oteryn:npc.agostina']['source']['sha256'] = ''
        with self.assertRaisesRegex(ValueError, 'custody'):
            stage.build(profiles)

    def test_invalid_donor_motion_does_not_silently_default(self):
        profiles = copy.deepcopy(self.profiles)
        profiles['oteryn:npc.agostina']['movement']['walk_radius'] = -1
        with self.assertRaisesRegex(ValueError, 'invalid walk radius'):
            stage.build(profiles)

    def test_remaining88_disjoint_and_missing_tibiopedia_is_flagged(self):
        profiles = json.loads((stage.ROOT / 'docs/agents/evidence/OTV2-20261002-npc-bulk-remaining88/source-profiles.json').read_text())
        packet, progress = stage.build(profiles, remaining=True)
        self.assertEqual(len(progress), 88)
        self.assertEqual(sum(r['field_quality']['presentation'] == 'donor' for r in progress), 13)
        self.assertEqual(sum(r['field_quality']['wiki.tibiopedia'] == 'todo' for r in progress), 1)
        self.assertEqual(len(packet['native_additions']['source_identity_bindings']), 175)
        first_keys = {r['key'] for r in stage.build(self.profiles)[1]}
        self.assertFalse(first_keys & {r['key'] for r in progress})
        path = stage.ROOT / 'docs/agents/evidence/OTV2-20261002-npc-bulk-remaining88/native-additions.json'
        self.assertEqual(stage.canonical(packet), path.read_bytes())


if __name__ == '__main__':
    unittest.main()
