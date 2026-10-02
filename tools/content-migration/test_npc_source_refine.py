"""Replay source upgrades and reject attribution, scope and normalization drift."""
import copy
import json
import unittest
from npc_source_refine import ROOT, build, values

EVIDENCE = ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r24'


class SourceRefineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = json.loads((ROOT / 'content/world/definitions/declarations.json').read_text())
        packet = json.loads((EVIDENCE / 'native-enrichment.json').read_text())
        records = {r['identity']['key']: r for r in cls.baseline['records']}
        profiles = {r['target']['key']: r for r in cls.baseline['authoring_profiles']}
        followup = json.loads((ROOT / 'docs/agents/evidence/OTV2-20261002-npc-appearance-r26/native-enrichment.json').read_text())
        invisible = json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r27/native-enrichment.json').read_text())
        for repair in invisible['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in invisible['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        for repair in followup['repairs']:
            records[repair['before']['identity']['key']] = repair['before']
        for repair in followup['profile_repairs']:
            profiles[repair['before']['target']['key']] = repair['before']
        visual = ROOT / 'docs/agents/evidence/OTV2-20261002-npc-appearance-r25/native-enrichment.json'
        if visual.exists():
            successor = json.loads(visual.read_text())
            for repair in successor['repairs']:
                records[repair['before']['identity']['key']] = repair['before']
            for repair in successor['profile_repairs']:
                profiles[repair['before']['target']['key']] = repair['before']
        for repair in packet['repairs']:
            key = repair['before']['identity']['key']
            if records[key] not in (repair['before'], repair['after']):
                raise AssertionError('current declaration differs from the pinned repair')
            records[key] = repair['before']
        for repair in packet['profile_repairs']:
            profiles[repair['before']['target']['key']] = repair['before']
        cls.baseline['records'] = list(records.values())
        cls.baseline['authoring_profiles'] = list(profiles.values())
        cls.upgrades = json.loads((EVIDENCE / 'source-facts.json').read_text())['upgrades']
        cls.packet = packet
        cls.sample = next(r for r in cls.upgrades if r['key'] == 'oteryn:npc.vasko')
        cls.normalized = next(r for r in cls.upgrades if r['key'] == 'oteryn:npc.gnomoney')

    def test_actual_packet_replay_preserves_raw_history_services_and_behavior(self):
        self.assertEqual(build(self.baseline, self.upgrades), self.packet)
        npcs = [r for r in self.packet['repairs'] if r['before']['kind'] == 'NPC']
        self.assertEqual(len(npcs), 133)
        for repair in npcs:
            for field in ['identity', 'presentation', 'behavior', 'dialogue', 'services']:
                self.assertEqual(repair['before'][field], repair['after'][field])
            self.assertEqual(values(repair['before']).get('wiki_profession'),
                             values(repair['after']).get('wiki_profession'))
        for repair in self.packet['repairs']:
            flags = json.loads(values(repair['after'])['quality'])
            self.assertLessEqual(set(flags.values()), {'verified', 'donor', 'defaulted', 'placeholder', 'todo'})

    def test_missing_full_capture_sha_is_rejected(self):
        sample = copy.deepcopy(self.sample)
        sample['dialogue']['greet']['source']['sha256'] = ''
        with self.assertRaises(ValueError):
            build(self.baseline, [sample])

    def test_wrong_source_speaker_is_rejected(self):
        sample = copy.deepcopy(self.sample)
        sample['dialogue']['greet']['quote_evidence'][0]['speaker_identity'] = 'Other Actor'
        with self.assertRaises(ValueError):
            build(self.baseline, [sample])

    def test_rewritten_empty_oversized_or_action_speech_is_rejected(self):
        for change in [{'reply': ['Made up reply.']}, {'reply': ['']},
                       {'reply': ['A', 'B', 'C']}, {'actions': ['trade']}]:
            sample = copy.deepcopy(self.sample)
            sample['dialogue']['greet'].update(change)
            with self.assertRaises(ValueError):
                build(self.baseline, [sample])

    def test_only_explicit_player_placeholder_normalization_is_allowed(self):
        build(self.baseline, [self.normalized])
        for change in [{'classification': 'SOURCE_QUOTED'},
                       {'normalization': {'from': 'Jogador', 'to': 'Traveller'}}]:
            sample = copy.deepcopy(self.normalized)
            sample['dialogue']['greet'].update(change)
            with self.assertRaises(ValueError):
                build(self.baseline, [sample])

    def test_protected_donor_foreign_actor_and_palette_bridge_are_rejected(self):
        sample = copy.deepcopy(next(r for r in self.upgrades if r['key'] == 'oteryn:npc.dwarven_guard'))
        sample['appearance']['selected_outfit_fields']['head'] = 10
        with self.assertRaises(ValueError):
            build(self.baseline, [sample])
        sample = copy.deepcopy(next(r for r in self.upgrades if r['key'] == 'oteryn:npc.candis'))
        sample.update(key='oteryn:npc.blubster', name='Blubster')
        with self.assertRaises(ValueError):
            build(self.baseline, [sample])
        sample = copy.deepcopy(self.sample)
        sample['key'] = 'oteryn:npc.foreign_actor'
        with self.assertRaises(ValueError):
            build(self.baseline, [sample])

    def test_original_role_alignment_is_separate_from_source_quotes(self):
        sample = next(r for r in self.upgrades if r['key'] == 'oteryn:npc.dwarf_captain')
        packet = build(self.baseline, [sample])
        dialogue = next(r for r in packet['repairs'] if r['after']['kind'] == 'Dialogue')
        metadata = json.loads(values(dialogue['after'])['dialogue_source'])
        self.assertFalse(metadata['r24_original_role_alignment']['source_quote'])
        self.assertEqual(json.loads(values(dialogue['after'])['quality'])['dialogue.job'], 'defaulted')
        for field in ['greet', 'farewell']:
            self.assertEqual(dialogue['before'][field], dialogue['after'][field])


if __name__ == '__main__':
    unittest.main()
