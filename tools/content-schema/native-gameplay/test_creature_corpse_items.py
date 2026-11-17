import json
import re
import unittest

import creature_corpse_items as wave
import native_goal_corpse_items as corpses
import native_goal_loot_items as loot


def canonical(keys):
    return {row['definition']['identity']['key']: row['definition']
            for path in (wave.ROOT / 'content/items/definitions').glob('items-*.json')
            for row in json.loads(path.read_bytes())['records']
            if row['definition']['identity']['key'] in keys}


class CreatureCorpseWaveS1(unittest.TestCase):
    def test_every_corpse_root_fits_the_runtime_cap_and_is_admitted_canonically(self):
        profiles = json.loads((wave.ROOT / corpses.CREATURES).read_bytes())['records']
        roots = {r['profile']['data']['profile']['details']['corpse_item']['key'] for r in profiles
                 if r['profile']['target']['key'] in wave.CREATURES}
        policies, _ = corpses.policies(wave.ROOT, wave.CREATURES)
        for key in roots:
            capacity = policies[int(key.rsplit('.i', 1)[1])][0]
            self.assertTrue(capacity is not None and 1 <= capacity <= 30, key)
        items = canonical(roots | {f'oteryn:item.tibia.i{i}' for i in loot.selected(wave.ROOT, wave.CREATURES)})
        self.assertTrue(all(d['materializable'] for d in items.values()))
        for key in roots:
            container = items[key]['semantics']['container']
            capacity = policies[int(key.rsplit('.i', 1)[1])][0]
            self.assertEqual(container['value']['capacity'], {'state': 'KNOWN', 'value': capacity}, key)

    def test_admission_is_idempotent_on_the_canonical_tree(self):
        keys = {f'oteryn:item.tibia.i{i}' for i in corpses.policies(wave.ROOT, wave.CREATURES)[0]}
        keys |= {f'oteryn:item.tibia.i{i}' for i in loot.selected(wave.ROOT, wave.CREATURES)}
        items = canonical(keys)
        replay = json.loads(json.dumps(list(items.values())))
        wave.apply_admissions(replay, wave.ROOT)
        self.assertEqual({d['identity']['key']: d for d in replay}, items)

    def test_every_corpse_fact_has_its_pinned_source_line_in_the_parity_table(self):
        table = (wave.ROOT / 'docs/reference/parity/corpses.md').read_text()
        lines = (wave.ROOT / 'imports/canary/items-xml/items.xml').read_text().split('\n')
        policies, weights = corpses.policies(wave.ROOT, wave.CREATURES)
        for item, (capacity, duration, target) in policies.items():
            row = re.search(rf'^\| i{item} \|[^|]*\| (\S+) \| (\d+) \| (\S+) \| (\d+) \| items\.xml:(\d+)-\d+ \|',
                            table, re.M)
            self.assertIsNotNone(row, item)
            self.assertEqual(row[1], '—' if capacity is None else str(capacity))
            self.assertEqual((int(row[2]), row[3], int(row[4])),
                             (duration, f'i{target}' if target else '—', weights[item]))
            self.assertIn(f'<item id="{item}"', lines[int(row[5]) - 1])


if __name__ == '__main__':
    unittest.main()
