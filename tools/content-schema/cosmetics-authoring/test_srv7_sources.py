"""SRV7's taming sources must be admitted by the production cosmetics input."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]


class TamingSourcePopulation(unittest.TestCase):
    def test_current_outfits_include_all_source_qualified_sexes(self):
        author = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())
        evidence = json.loads((ROOT / 'docs/reference/content-prep/outfits-content-1/admission-evidence/evidence-00000-00135.json').read_bytes())
        rows = {row['key']: row for row in author['document']['outfits']}
        self.assertEqual(len(rows), 136)
        for source in evidence['records']:
            row = rows[source['key']]
            self.assertEqual(row['male']['id'], source['facts']['male_look_type']['value'])
            self.assertEqual(row['female']['id'], source['facts']['female_look_type']['value'])
        self.assertEqual(rows['oteryn:content.outfit.monk_outfits']['addon_count'], 0)
        for key in ['citizen', 'hunter', 'mage', 'knight']:
            row = rows[f'oteryn:content.outfit.{key}_outfits']
            self.assertTrue(row['starter_female'])
            self.assertTrue(row['starter_male'])
            self.assertFalse(row['premium'])

    def test_real_loot_provider_can_mint_every_named_taming_source(self):
        rules = json.loads((ROOT / 'content/cosmetics/taming.json').read_bytes())
        wanted = {rule['item_key'] for rule in rules}
        loot = json.loads((ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        named = {row['item']['key']: row for row in loot['items'] if row['item']['key'] in wanted}
        self.assertIn('oteryn:item.tibia.i5907', named)
        refused = [key for key, row in named.items() if not row['materializable']]
        self.assertEqual(refused, [], 'the real death-loot consumer must be able to mint the taming source')

    def test_all_taming_pairs_have_source_bound_native_mounts_and_grants(self):
        author = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())
        document = author['document']
        mounts = {row['key']: row for row in document['mounts']}
        mappings = {row['grant_key']: row for row in document['grant_mappings']}
        rules = json.loads((ROOT / 'content/cosmetics/taming.json').read_bytes())
        self.assertEqual(len(rules), 51)
        self.assertEqual(len({r['mount_key'] for r in rules}), 42)
        for rule in rules:
            row = mounts[rule['mount_key']]
            self.assertNotEqual(row['look']['id'], 0)
            self.assertEqual(row['speed_bonus'], 10)
            grant = mappings[rule['grant_key']]
            self.assertEqual(grant['target'], {'kind': 'Mount', 'mount_key': row['key']})
            self.assertIn('mount-taming-v1', grant['compatible_provenance'])

    def test_every_official_item_grant_and_required_base_has_admitted_mapping(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
        document = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())['document']
        mappings = {r['grant_key']: r for r in document['grant_mappings']}
        official = [r for r in source['rules'] if r['mode'] not in {'random_outfit', 'random_mount'}]
        self.assertEqual(len(official), 46)
        for rule in official:
            for choice in rule['choices']:
                for key in choice['grants'] + choice.get('required', []):
                    self.assertIn(key, mappings)

    def test_every_named_official_cosmetic_drop_is_materializable(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
        wanted = {f"oteryn:item.tibia.i{r['item_id']}" for r in source['rules']
                  if r['mode'] not in {'random_outfit', 'random_mount'}}
        loot = json.loads((ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        named = {r['item']['key']: r for r in loot['items'] if r['item']['key'] in wanted}
        self.assertIn('oteryn:item.tibia.i44048', named)
        self.assertEqual([key for key, r in named.items() if not r['materializable']], [])

    def test_official_spiritual_horseshoe_has_native_item_policy(self):
        provider = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = {r['production_definition']['production_key']: r for r in provider['records']}
        record = rows['oteryn:item.tibia.i44048']
        self.assertTrue(record['admission']['materializable'])
        self.assertIn('CharacterInventory', record['admission']['legal_destinations'])

    def test_existing_tibiadrome_production_rewards_have_native_looks(self):
        document = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())['document']
        mounts = {r['key']: r for r in document['mounts']}
        mappings = {r['grant_key']: r for r in document['grant_mappings']}
        self.assertEqual(mounts['oteryn:content.mount.phant']['look']['id'], 1417)
        for name in ['phant', 'lion-of-war-base']:
            self.assertIn('crystal-201189f-tibiadrome-v1',
                          mappings[f'oteryn:appearance-grant/tibiadrome/{name}']['compatible_provenance'])

    def test_ot_only_dolls_remain_without_full_production_grant_authority(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
        document = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())['document']
        admitted = {row['grant_key'] for row in document['grant_mappings']}
        excluded = [row for row in source['rules'] if row['mode'] in {'random_outfit', 'random_mount'}]
        self.assertEqual({row['item_id'] for row in excluded}, {6579, 37746})
        for rule in excluded:
            required = {key for choice in rule['choices']
                        for key in choice['grants'] + choice.get('required', []) + [choice['owned']]}
            self.assertTrue(required - admitted, 'whole-rule production qualifier must refuse the OT-only source')


if __name__ == '__main__':
    unittest.main()
