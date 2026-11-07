import copy
import json
import unittest
from pathlib import Path

import wevents_creature_profiles as producer

ROOT = Path(__file__).resolve().parents[2]


class EventActorPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.creatures = json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())
        cls.presentations = json.loads((ROOT / 'content/presentations/definitions/spell-native-profiles.json').read_bytes())

    def test_source_health_and_named_appearances_survive_lowering(self):
        before = copy.deepcopy(next(r for r in self.creatures['records'] if r['profile']['target']['key']=='oteryn:creature.chikhaton'))
        records, cues = producer.candidates(ROOT, self.creatures, self.presentations)
        by_name = {r['profile']['target']['key'].removeprefix(producer.PREFIX): r for r in records}
        for name, hp in [('anmothra', 7000), ('irahsae', 35000), ('teneshpar', 15000), ('phrodomo', 90000), ('devovorga', 500000)]:
            self.assertEqual(by_name[name]['profile']['data']['profile']['health'], hp)
            self.assertIn('sha256', by_name[name]['source_qualification'])
        cue_key = by_name['devovorga']['presentation']['key']
        cue = next(c for c in self.presentations['records'] + cues if c['target']['key'] == cue_key)
        self.assertEqual(cue['data']['profile']['asset_binding'], 'canary.appearance:outfit/354')
        self.assertEqual(next(r for r in self.creatures['records'] if r['profile']['target']['key']=='oteryn:creature.chikhaton'), before)

    def test_attacks_are_bound_and_passive_mutations_cannot_attack(self):
        records, _ = producer.candidates(ROOT, self.creatures, self.presentations)
        for r in records:
            p = r['profile']['data']['profile']
            b = r['behavior']['data']['profile']
            self.assertIsNone(p['details']['corpse_item'])
            self.assertEqual(p['experience'], 0)
            self.assertFalse(p['details']['system_eligibility']['prey'])
            if r['profile']['target']['key'].endswith(('bloom_of_doom', 'vulnerable_cocoon')):
                self.assertNotIn('monster_melee', r)
                self.assertFalse(b['movement']['can_walk'])
                self.assertEqual(b['attacks'], [])
            else:
                self.assertEqual(p['abilities'], [r['monster_melee']['ability']])
                self.assertEqual(b['attacks'][0]['ability'], r['monster_melee']['ability'])
        by_name = {r['profile']['target']['key'].split('.')[-1]: r for r in records}
        self.assertEqual(by_name['teneshpar']['monster_melee']['maximum'], 550)
        self.assertEqual(by_name['phrodomo']['monster_melee']['maximum'], 6030)
        self.assertEqual(by_name['devovorga']['monster_melee']['maximum'], 10050)


if __name__ == '__main__':
    unittest.main()
