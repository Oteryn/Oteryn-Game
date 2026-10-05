import copy
import json
import unittest

from npc_bulk_finish import EVIDENCE, ROOT, build, values


class FinishTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = json.loads((EVIDENCE / 'source-facts.json').read_text())
        records, profiles = {}, {}
        for folder in ['OTV2-20261002-npc-bulk-first45', 'OTV2-20261002-npc-bulk-remaining88']:
            packet = json.loads((ROOT / ('docs/agents/evidence/' + folder + '/native-additions.json')).read_text())
            records.update((r['identity']['key'], r) for r in packet['native_additions']['declarations'])
            profiles.update((r['target']['key'], r) for r in packet['native_additions']['authoring_profiles'])
        for folder in ['OTV2-20261002-npc-enrichment-r21', 'OTV2-20261002-npc-enrichment-r22']:
            packet = json.loads((ROOT / ('docs/agents/evidence/' + folder + '/native-enrichment.json')).read_text())
            records.update((r['after']['identity']['key'], r['after']) for r in packet['repairs'])
            profiles.update((r['after']['target']['key'], r['after']) for r in packet['profile_repairs'])
        cls.baseline = {'records': list(records.values()), 'authoring_profiles': list(profiles.values())}

    def test_closed_basics_preserve_source_unknowns_and_deferred_runtime(self):
        packet, rows, counts = build(self.baseline, self.facts)
        self.assertEqual(packet, json.loads((EVIDENCE / 'native-enrichment.json').read_text()))
        self.assertEqual(len(rows), 133)
        self.assertEqual(counts['basic_components'], 532)
        self.assertEqual(counts['unselected_roles'], 0)
        allowed_quality = {'verified', 'donor', 'defaulted', 'placeholder', 'todo'}
        self.assertTrue(all(q in allowed_quality for r in rows for q in r['field_quality'].values()))
        for repair in packet['repairs']:
            a, b = repair['before'], repair['after']
            self.assertEqual(a['identity'], b['identity'])
            if b['kind'] == 'NPC':
                self.assertEqual(values(a)['wiki_profession'], values(b)['wiki_profession'])
                self.assertEqual(a['services'], b['services'])
                completion = json.loads(values(b)['completion'])
                self.assertFalse(completion['canonical_tibia_fidelity_claim'])
                self.assertFalse(completion['native_runtime_loaded'])
                self.assertIn('quest effects', completion['deferred_gameplay'])

    def test_missing_component_and_wrong_actor_are_rejected(self):
        for missing in [True, False]:
            facts = copy.deepcopy(self.facts)
            row = facts['dialogue']['records'][0]
            if missing:
                del row['selected_parts']['greet']
            else:
                row['name'] = 'Wrong speaker'
            with self.assertRaises(ValueError):
                build(self.baseline, facts)

    def test_rewritten_quote_or_false_original_attribution_is_rejected(self):
        for quote in [True, False]:
            facts = copy.deepcopy(self.facts)
            proposal = next(p for r in facts['dialogue']['records'] for p in r['selected_parts'].values() if p['source_quote'] == quote)
            if quote:
                proposal['reply'] = ['A rewritten source quotation']
            else:
                proposal['source_quote'] = True
            with self.assertRaises(ValueError):
                build(self.baseline, facts)

    def test_template_rewrite_and_actor_equivalence_claim_are_rejected(self):
        for exact in [True, False, None]:
            facts = copy.deepcopy(self.facts)
            row = next(r for r in facts['appearances'] if bool(r['source_template']) == (exact is not None))
            if exact:
                row['actor_exact_match'] = True
            else:
                row['outfit']['look_type'] += 1
            with self.assertRaises(ValueError):
                build(self.baseline, facts)

    def test_donor_profile_and_movement_are_preserved(self):
        packet, _, _ = build(self.baseline, self.facts)
        placeholder_keys = {r['key'].replace('oteryn:npc.', 'oteryn:presentation.npc.') for r in self.facts['appearances']}
        self.assertEqual({r['before']['target']['key'] for r in packet['profile_repairs']}, placeholder_keys)
        self.assertTrue(all(r['before']['target']['family'] == 'Presentation' for r in packet['profile_repairs']))
        for repair in packet['repairs']:
            if repair['before']['kind'] == 'NPC':
                for field in ['behavior', 'presentation', 'dialogue']:
                    self.assertEqual(repair['before'][field], repair['after'][field])

    def test_known_role_cannot_be_overridden(self):
        facts = copy.deepcopy(self.facts)
        known = next(r for r in self.baseline['records'] if r['kind'] == 'NPC' and json.loads(values(r)['quality'])['profession'] == 'verified')
        facts['profession']['records'][0]['key'] = known['identity']['key']
        with self.assertRaises(ValueError):
            build(self.baseline, facts)


if __name__ == '__main__':
    unittest.main()
