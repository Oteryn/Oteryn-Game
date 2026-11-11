"""A complete source boss loot table cannot block its corpse and quest credit."""
import copy
import json
import unittest
from pathlib import Path
from unittest.mock import patch

import galthen_boss_loot as producer

ROOT = Path(__file__).resolve().parents[2]


class GalthenBossLootTests(unittest.TestCase):
    def test_captured_boot_speed_and_resistances_reach_native_abilities(self):
        rows = {row['production_binding']['external_id']: row for row in
                json.loads((ROOT / producer.PATH).read_bytes())['records']}
        modifiers = rows['44648']['semantics']['skill_modifiers']['value']['modifiers']['value']
        self.assertIn({'kind': 'SIGNED_POINTS', 'value': 15},
                      [row['parameter']['value'] for row in modifiers if row['kind'] == 'SPEED'])
        resistances = rows['44649']['semantics']['protection']['value']['resistances']['value']
        self.assertEqual(rows['44649']['semantics']['protection']['value']['armor'],
                         {'state': 'KNOWN', 'value': 2})
        self.assertEqual(rows['44648']['semantics']['protection']['value']['armor'],
                         {'state': 'KNOWN', 'value': 2})
        self.assertIn({'kind': 'ICE', 'percent': {'state': 'KNOWN',
                      'value': {'denominator': 1, 'numerator': 6}}}, resistances)
        monk = rows['50255']['semantics']['skill_modifiers']['value']['modifiers']['value']
        self.assertEqual({entry['kind']: entry['parameter']['value']['value'] for entry in monk},
                         {'MANTRA': 16, 'SKILL_FIST': 4})

    def test_unmapped_or_malformed_ability_never_partially_publishes(self):
        for attrs in ({'speed': 'invalid'}, {'speed': '15', 'suppressunknown': '1'},
                      {'speed': '15', 'absorbpercentice': '999'},
                      {'speed': '15', 'mantra': '32768'},
                      {'speed': '15', 'armor': 'invalid'},
                      {'speed': '15', 'armor': '2147483648'}):
            with self.subTest(attrs=attrs):
                row = {'semantics': {}}
                with self.assertRaisesRegex(ValueError, 'M25_LOOT_EQUIPMENT_SOURCE'):
                    producer.complete_source_abilities(row, attrs)
                self.assertEqual(row, {'semantics': {}})

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

    def test_late_boot_completion_preserves_qualified_galthen_records(self):
        import imbuement_item_profiles as imbuement
        packet = json.loads((ROOT / producer.PATH).read_bytes())
        generated = {'profiles': producer.base.encode({'records': packet['records']}),
                     'manifest': b'{"item_profiles":{}}'}
        imbuement.complete_boots(ROOT, generated, 'profiles', 'manifest',
                                lambda row: row['production_definition']['production_key'],
                                producer.base.encode)
        actual = {row['production_definition']['production_key']: row
                  for row in json.loads(generated['profiles'])['records']}
        self.assertEqual(actual, {row['production_definition']['production_key']: row
                                  for row in packet['records']})

    def test_late_boot_completion_requires_exact_crystal_cross_check(self):
        import imbuement_item_profiles as imbuement
        packet = json.loads((ROOT / producer.PATH).read_bytes())
        facts_path = ROOT / imbuement.CRYSTAL_FACTS
        facts = json.loads(facts_path.read_bytes())
        original_read = Path.read_bytes
        for item in ('44648', '44649', '50291'):
            with self.subTest(item=item):
                forged = copy.deepcopy(facts)
                forged['records'][item]['families']['INCREASE_SPEED'] = '2'
                generated = {'profiles': producer.base.encode({'records': packet['records']}),
                             'manifest': b'{"item_profiles":{}}'}
                before = dict(generated)
                with (
                    patch.object(Path, 'read_bytes', lambda path, forged=forged:
                                 producer.base.encode(forged) if path == facts_path
                                 else original_read(path)),
                    self.assertRaisesRegex(ValueError, 'IMBUEMENT_BOOT_CROSS_CHECK ' + item),
                ):
                    imbuement.complete_boots(
                        ROOT, generated, 'profiles', 'manifest',
                        lambda row: row['production_definition']['production_key'],
                        producer.base.encode)
                self.assertEqual(generated, before)

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
