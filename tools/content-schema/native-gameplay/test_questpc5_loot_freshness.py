"""Published native corpse/loot admissions must match the owned source encounters."""
import json
import unittest

import build_full_spell_artifact as producer


class Questpc5LootFreshness(unittest.TestCase):
    def test_published_encounter_closures_match_current_native_admission(self):
        _, creatures = producer.read(producer.CREATURE_PROFILES)
        expected = json.loads(producer.loot_tables(creatures))
        actual = json.loads(producer.LOOT_TABLES.read_bytes())
        wanted = {'oteryn:item.tibia.i5960', 'oteryn:item.tibia.i5973',
                  'oteryn:item.tibia.i5983'}
        keys = {'oteryn:loot.creature.' + name for name in
                ('troll', 'dragon', 'minotaur_guard')}
        for table in expected['tables']:
            if table['identity']['key'] in keys:
                wanted.update(entry['item']['key'] for entry in table['entries'])
                self.assertIn(table, actual['tables'], 'source drops remain exact')
        by_key = lambda doc: {row['item']['key']: row for row in doc['items']}
        published, current = by_key(actual), by_key(expected)
        for key in sorted(wanted):
            with self.subTest(item=key):
                self.assertEqual(published[key], current[key])
                self.assertTrue(current[key]['materializable'])


if __name__ == '__main__':
    unittest.main()
