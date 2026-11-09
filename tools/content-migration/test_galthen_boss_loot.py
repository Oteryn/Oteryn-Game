"""A complete source boss loot table cannot block its corpse and quest credit."""
import json
import copy
from pathlib import Path
import unittest
import galthen_boss_loot as producer

ROOT = Path(__file__).resolve().parents[2]


class GalthenBossLootTests(unittest.TestCase):
    def test_complete_source_records_survive_registration(self):
        packet = json.loads((ROOT / producer.PATH).read_bytes())
        native = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual = {row['production_definition']['production_key']: row for row in native['records']}
        self.assertEqual(len(packet['records']), 19)
        for row in packet['records']:
            self.assertEqual(actual.get(row['production_definition']['production_key']), row)

    def test_known_policy_is_never_replaced(self):
        packet = json.loads((ROOT / producer.PATH).read_bytes())
        row = copy.deepcopy(packet['records'][0])
        row['attributes']['speed_bonus'] = 123
        generated = {'profiles': producer.base.encode({'records': [row]}),
                     'manifest': b'{"item_profiles":{}}'}
        with self.assertRaisesRegex(ValueError, 'M25_LOOT_KNOWN_POLICY_CONFLICT'):
            producer.admit(ROOT, generated, 'profiles', 'manifest',
                           lambda row: row['production_definition']['production_key'], producer.base.encode)

    def test_current_tibia_boot_sockets_do_not_import_crystal_tier_ten(self):
        packet = json.loads((ROOT / producer.PATH).read_bytes())
        for item in ['44648', '44649']:
            row = next(row for row in packet['records'] if row['production_binding']['external_id'] == item)
            value = row['semantics']['imbuement']['value']
            self.assertEqual(value['slot_count']['value'], 1)
            self.assertEqual(value['allowed_family_tiers']['value'], [
                {'family': 'INCREASE_SPEED', 'tier': 'Three'},
                {'family': 'PARALYSIS_REMOVAL', 'tier': 'Three'}])
        self.assertEqual(set(packet['crystal_family_differences']), {'44648', '44649', '50255'})

    def test_crystal_paralysis_name_uses_existing_canonical_family(self):
        import imbuement_item_profiles as imbuement
        xml = b'<items><item id="44648"><attribute key="imbuementslot" value="1"><attribute key="paralysis removal" value="3"/></attribute></item></items>'
        self.assertEqual(imbuement.slots(xml, {44648}), {
            44648: {'slot_count': 1, 'families': {'PARALYSIS_REMOVAL': '3'}}})

    def test_actual_galthen_boss_tables_have_no_inadmissible_yields(self):
        data = json.loads((ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        facts = {row['item']['key']: row for row in data['items']}
        for boss in ['ahau', 'mitmah_vanguard']:
            binding = next(row for row in data['creature_loot']
                           if row['creature']['key'] == 'oteryn:creature.' + boss)
            table = next(row for row in data['tables'] if row['identity'] == binding['loot'])
            for entry in table['entries']:
                with self.subTest(boss=boss, item=entry['item']['key']):
                    self.assertTrue(facts[entry['item']['key']]['materializable'])


if __name__ == '__main__':
    unittest.main()
