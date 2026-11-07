import copy
import json
from pathlib import Path
import unittest

from npc_bulk_enrich import EVIDENCE, ROOT, build


class EnrichmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = json.loads((EVIDENCE / 'source-facts.json').read_text())
        packets = [json.loads((ROOT / ('docs/agents/evidence/' + folder + '/native-additions.json')).read_text())
                   for folder in ['OTV2-20261002-npc-bulk-first45', 'OTV2-20261002-npc-bulk-remaining88']]
        cls.baseline = {key: [r for packet in packets for r in packet['native_additions'][source]]
                        for key, source in [('records', 'declarations'), ('authoring_profiles', 'authoring_profiles')]}

    def test_reproduces_closed_packet_and_preserves_disabled_references(self):
        packet, rows = build(self.baseline, self.facts)
        self.assertEqual(packet, json.loads((EVIDENCE / 'native-enrichment.json').read_text()))
        self.assertEqual(len(rows), 133)
        self.assertFalse(any(r['native_runtime_loaded'] for r in rows))
        for repair in packet['repairs']:
            a, b = repair['before'], repair['after']
            self.assertEqual(a['identity'], b['identity'])
            if a['kind'] == 'NPC':
                self.assertEqual(a['services'], b['services'])
                self.assertEqual(a['dialogue'], b['dialogue'])
        self.assertEqual(len(packet['profile_repairs']), 2)

    def test_unknown_actor_and_source_name_are_rejected(self):
        for foreign in [True, False]:
            facts = copy.deepcopy(self.facts)
            facts['wiki']['records'][0]['key' if foreign else 'name'] = 'foreign'
            with self.assertRaises(ValueError):
                build(self.baseline, facts)

    def test_stateful_keyword_cannot_be_admitted(self):
        facts = copy.deepcopy(self.facts)
        row = next(r for r in facts['dialogue']['records'] if r.get('keyword_candidates'))
        row['keyword_candidates'][0]['no_stateful_actions'] = False
        with self.assertRaises(ValueError):
            build(self.baseline, facts)

    def test_quoted_comment_movement_becomes_stationary_default(self):
        packet, rows = build(self.baseline, self.facts)
        row = next(r for r in rows if r['key'] == 'oteryn:npc.dragon_ancestor_spirit')
        self.assertEqual(row['field_quality']['movement.walk_interval_ms'], 'defaulted')
        profile = next(r['after'] for r in packet['profile_repairs'] if r['after']['target']['family'] == 'Behavior')
        self.assertFalse(profile['data']['profile']['movement']['can_walk'])
        self.assertNotIn('wander', profile['data']['profile']['movement'])

    def test_unknown_professions_and_complex_exchanges_stay_partial(self):
        packet, rows = build(self.baseline, self.facts)
        self.assertEqual(sum(r['field_quality']['profession'] == 'todo' for r in rows), 86)
        for repair in packet['repairs']:
            if repair['after']['kind'] == 'Dialogue':
                self.assertEqual({n['key'] for n in repair['after']['keywords']}, {'name', 'job'})


if __name__ == '__main__':
    unittest.main()
