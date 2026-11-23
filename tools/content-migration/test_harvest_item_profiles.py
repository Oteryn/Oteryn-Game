"""Production admission covers harvest inputs, outputs and every regrowth state."""
import hashlib
import json
import unittest

import harvest_item_profiles as tool
import house_decoration_actions


class HarvestItemProfileTests(unittest.TestCase):
    def test_production_provider_contains_the_complete_harvest_closure(self):
        raw = (tool.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes()
        profiles = {r['production_definition']['production_key']: r
                    for r in json.loads(raw)['records']}
        for item in sorted(tool.IDS):
            with self.subTest(item=item):
                self.assertTrue(profiles[f'oteryn:item.tibia.i{item}']['admission']['materializable'])
        packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        self.assertEqual(len(packet['records']), len(tool.IDS))
        house_decoration_actions.apply_top_order(
            packet['records'],
            json.loads((tool.ROOT / house_decoration_actions.PROFILES).read_bytes()))
        # The canonical Item supplies the market policy absent from the harvest capture.
        market_item = 'oteryn:item.tibia.i953'
        item = next(row['definition']
                    for shard in sorted((tool.ROOT / 'content/items/definitions').glob('items-*.json'))
                    for row in json.loads(shard.read_bytes())['records']
                    if row['definition']['identity']['key'] == market_item)
        taxonomy = {r['target']['key']: r['source_taxonomy'] for r in json.loads(
            (tool.ROOT / 'content/items/taxonomy/items.json').read_bytes())['records']}
        for record in packet['records']:
            key = record['production_definition']['production_key']
            # Production registration attaches the existing canonical taxonomy.
            if key in taxonomy:
                record['authoring']['taxonomy'] = taxonomy[key]
            if key == market_item:
                record['semantics']['trade_restrictions'] = item['semantics']['trade_restrictions']
            self.assertEqual(profiles[key], record)
        manifest = json.loads((tool.ROOT / 'content/spells.manifest.json').read_bytes())
        self.assertEqual(manifest['item_profiles']['sha256'], hashlib.sha256(raw).hexdigest())

    def test_duplicate_is_rejected_before_writing(self):
        record = json.loads((tool.ROOT / tool.PATH).read_bytes())['records'][0]
        planned = {'profiles': json.dumps({'records': [record]}).encode(), 'manifest': b'{}'}
        before = dict(planned)
        with self.assertRaisesRegex(ValueError, 'HARVEST_ITEM_PROFILE_DUPLICATE'):
            tool.admit(tool.ROOT, planned, 'profiles', 'manifest', lambda row: 0, lambda value: b'')
        self.assertEqual(planned, before)
