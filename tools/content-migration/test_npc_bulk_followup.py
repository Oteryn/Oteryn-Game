import copy
import json
import unittest

from npc_bulk_followup import EVIDENCE, ROOT, build, values


class FollowupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = json.loads((EVIDENCE / 'source-facts.json').read_text())
        records = {}
        for folder in ['OTV2-20261002-npc-bulk-first45', 'OTV2-20261002-npc-bulk-remaining88']:
            packet = json.loads((ROOT / ('docs/agents/evidence/' + folder + '/native-additions.json')).read_text())
            records.update((r['identity']['key'], r) for r in packet['native_additions']['declarations'])
        previous = json.loads((ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r21/native-enrichment.json').read_text())
        records.update((r['after']['identity']['key'], r['after']) for r in previous['repairs'])
        cls.baseline = {'records': list(records.values())}

    def test_reproduces_packet_without_enabling_services_or_profiles(self):
        packet, rows, counts = build(self.baseline, self.facts)
        self.assertEqual(packet, json.loads((EVIDENCE / 'native-enrichment.json').read_text()))
        self.assertEqual(counts['source_selected_dialogues'], 96)
        self.assertEqual(len(rows), 133)
        self.assertEqual(packet['profile_repairs'], [])
        for repair in packet['repairs']:
            a, b = repair['before'], repair['after']
            self.assertEqual(a['identity'], b['identity'])
            if a['kind'] == 'NPC':
                for key in ['services', 'dialogue', 'presentation', 'behavior']:
                    self.assertEqual(a.get(key), b.get(key))

    def test_wrong_speaker_and_role_identity_are_rejected(self):
        for lane in ['dialogue', 'profession']:
            facts = copy.deepcopy(self.facts)
            row = next(r for r in facts[lane]['records'] if lane == 'dialogue' or r['profession_selection']['label'])
            row['name'] = 'Different speaker'
            with self.assertRaises(ValueError):
                build(self.baseline, facts)

    def test_stateful_or_invalid_source_reference_is_rejected(self):
        for field, value in [('no_stateful_actions', False), ('source_index', 9999)]:
            facts = copy.deepcopy(self.facts)
            row = next(r for r in facts['dialogue']['records'] if r['keyword_candidates'])
            row['keyword_candidates'][0][field] = value
            with self.assertRaises(ValueError):
                build(self.baseline, facts)

    def test_original_role_prose_is_not_a_source_quote(self):
        packet, _, _ = build(self.baseline, self.facts)
        generated = []
        for repair in packet['repairs']:
            after = repair['after']
            if after['kind'] == 'Dialogue' and 'dialogue_followup' in values(after):
                followup = json.loads(values(after)['dialogue_followup'])
                generated.extend(r for r in followup['selected'] if r.get('quality') == 'OTERYN_ORIGINAL_APPROXIMATE_ROLE_REPLY')
            if after['kind'] == 'NPC':
                self.assertIn('wiki_profession', values(after))
                self.assertEqual(values(repair['before'])['wiki_profession'], values(after)['wiki_profession'])
        self.assertEqual(len(generated), 38)
        self.assertTrue(all(r['source_quote'] is False for r in generated))

    def test_portrait_cannot_replace_a_native_outfit(self):
        facts = copy.deepcopy(self.facts)
        facts['portraits'][0]['name'] = 'Different portrait actor'
        with self.assertRaises(ValueError):
            build(self.baseline, facts)


if __name__ == '__main__':
    unittest.main()
