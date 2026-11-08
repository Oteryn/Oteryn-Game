"""QREADY1 enables only exact canonical selectors from the pinned startup table."""
import hashlib
import json
import tarfile
import unittest
from pathlib import Path

import lua_tables


class Qready1SourceBindingsTest(unittest.TestCase):
    def test_captured_rewards_match_pinned_source_and_canonical_claims(self):
        root = Path(__file__).resolve().parents[3]
        capture = json.loads((root / 'tools/content-schema/world-authoring/samples/quest-chest-bindings/qready1.json').read_bytes())
        source_paths = {'data-global/startup/tables/chest.lua': 'table_sha256',
                        'data-global/startup/others/functions.lua': 'startup_sha256',
                        'data-global/startup/tables/door_level.lua': 'level_source_sha256'}
        sources = {}
        with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
            manifest = json.load(corpus.extractfile('corpus-manifest.json'))
            for path, pin in source_paths.items():
                entry = next(r for r in manifest['files'] if r['source'] == 'crystalserver'
                             and r['revision'] == capture['revision'] and r['path'] == path)
                raw = corpus.extractfile(entry.get('blob_path', entry.get('cache_path'))).read()
                self.assertEqual(capture[pin], hashlib.sha256(raw).hexdigest())
                sources[path] = raw
        table = lua_tables.assignments(sources['data-global/startup/tables/chest.lua'].decode(), {'ChestUnique'})['ChestUnique']
        entries = {f['key']: (f['line'], lua_tables.as_python(f['value'])) for f in table['fields']}
        claims = {}
        for path in json.loads((root / 'content/interactions/reward_claims/index.json').read_bytes())['shards']:
            claims.update((r['definition']['identity']['key'], r['definition'])
                          for r in json.loads((root / path).read_bytes())['records'])
        expected = {
            'adorned_uh_rune_quest', 'barbarian_axe_quest', 'battle_axe_quest',
            'blood_herb_quest', 'captain_iglues_treasure_quest', 'circle_room_quest',
            'combat_knife_quest', 'crusader_helmet_quest', 'crystal_wand_quest',
            'darashia_dragon_quest', 'dark_armor_quest', 'demona_ring_quest',
            'double_hero_quest', 'doublet_quest', 'elephant_tusk_quest',
            'explorer_brooch_quest', 'griffin_shield_quest',
        }
        self.assertEqual(expected, {claims[r['claim']]['quest']['key'].split('/')[1]
                                    for r in capture['bindings']})
        self.assertEqual(24, len(capture['bindings']))
        seen = set()
        for row in capture['bindings']:
            key = row['claim']
            self.assertNotIn('simon_the_beggars', key)
            self.assertNotIn('druid_outfits', key)
            marker = (key, tuple(row['position']))
            self.assertNotIn(marker, seen)
            seen.add(marker)
            claim = claims[key]
            self.assertEqual('ready', claim['readiness'])
            placement = next(p for p in claim['placements']
                             if p['source_binding']['project_position'] == dict(zip('xyz', row['position'])))
            self.assertEqual(placement['appearance_tibia_id'], row['appearance'])
            self.assertIn({'server': 'crystalserver', 'unique_id': row['unique']},
                          placement['source_binding']['legacy_unique_ids'])
            line, setting = entries[row['unique']]
            self.assertEqual(row['source_line'], line)
            self.assertEqual(row['appearance'], setting['itemId'])
            self.assertEqual(dict(zip('xyz', row['position'])), setting['itemPos'])
            reward = [[int(i['item']['key'].rsplit('.i', 1)[1]), i['count']]
                      for i in placement['reward']['items']]
            self.assertEqual(reward, setting['reward'])
