"""W18 source catalogue and native policy closure for ordinary House decoration USE."""
import copy
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from house_decoration_actions import (
    CATALOGUE,
    PROFILES,
    admit,
    apply_top_order,
    apply_bed_semantics,
    apply_source_beds,
    apply_carpet_types,
    initialized_item_ids,
    native_item_specs,
    encode,
    sql_pairs,
)

ROOT = Path(__file__).resolve().parents[2]


class HouseDecorationActions(unittest.TestCase):
    def test_house_admission_preserves_owned_qready1_reward_profiles(self):
        packet = json.loads((ROOT / 'tools/content-schema/native-gameplay/qready1-reward-item-profiles.json').read_bytes())
        original = copy.deepcopy(packet['records'])
        generated = {'profiles': encode({'records': original}),
                     'manifest': encode({'item_profiles': {}})}
        admit(ROOT, generated, 'profiles', 'manifest',
              lambda row: row['production_definition']['production_key'], encode)
        actual = {row['production_definition']['production_key']: row
                  for row in json.loads(generated['profiles'])['records']}
        for record in original:
            self.assertEqual(actual[record['production_definition']['production_key']], record)
        overlay = json.loads((ROOT / PROFILES).read_bytes())
        reused = copy.deepcopy(original)
        apply_top_order(reused, overlay)  # SRX also calls the shared enrichment.
        self.assertEqual(reused, original)
        overlay['always_on_top']['3068'] = 1
        read_bytes = Path.read_bytes
        with patch.object(Path, 'read_bytes',
                          lambda path: encode(overlay) if path == ROOT / PROFILES else read_bytes(path)):
            with self.assertRaisesRegex(ValueError, 'HOUSE_QREADY1_TOP_ORDER_CONFLICT'):
                admit(ROOT, generated, 'profiles', 'manifest',
                      lambda row: row['production_definition']['production_key'], encode)

    def test_bed_parser_zero_reverse_free_slot_is_initialized_in_xml_order(self):
        from xml.etree import ElementTree as ET
        # Crystal item_parse.cpp:741-742 checks zero, not dictionary absence.
        # The occupied Item is parsed first with explicitzero, then male source
        # initializes its reverse free appearance. No canonical IDs are invented.
        occupied = ET.fromstring('<item><attribute key="type" value="bed"/>'
                                 '<attribute key="bedpart" value="pillow"/>'
                                 '<attribute key="partnerdirection" value="south"/>'
                                 '<attribute key="transformto" value="0"/></item>')
        free = ET.fromstring('<item><attribute key="type" value="bed"/>'
                            '<attribute key="bedpart" value="pillow"/>'
                            '<attribute key="partnerdirection" value="south"/>'
                            '<attribute key="maletransformto" value="20"/></item>')
        records = [{'production_definition': {'production_key': f'oteryn:item.tibia.i{item}'},
                    'semantics': {}} for item in (20,10)]
        bindings = {str(item): {'target': {'key': f'oteryn:item.tibia.i{item}',
                                         'revision': 'definition-r1'}} for item in (20,10)}
        self.assertEqual(apply_source_beds(records, {20: occupied, 10: free}, bindings), 2)
        self.assertEqual(records[0]['semantics']['bed']['value']['occupied_female']['key'],
                         'oteryn:item.tibia.i10')
        self.assertEqual(records[1]['semantics']['bed']['value']['occupied_female']['key'],
                         'oteryn:item.tibia.i20')

    def test_exact_source_bed_profiles_and_in_place_admission(self):
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        beds = {row['production_definition']['production_key']: row['semantics']['bed']['value']
                for row in profiles['records'] if row['semantics'].get('bed', {}).get('state') == 'KNOWN'}
        self.assertEqual(len(beds), 400)
        self.assertEqual(set(beds), {f'oteryn:item.tibia.i{item}' for item in catalogue['bed_item_ids']})
        self.assertEqual(beds['oteryn:item.tibia.i696']['occupied_female']['key'], 'oteryn:item.tibia.i2493')
        self.assertEqual(beds['oteryn:item.tibia.i697']['occupied_male']['key'], 'oteryn:item.tibia.i2494')
        # Crystal bed.cpp:288-303 checks target BED, without geometry coercion.
        self.assertEqual(beds['oteryn:item.tibia.i37197']['occupied_male']['key'], 'oteryn:item.tibia.i37200')
        for bed in beds.values():
            for sex in ('occupied_male', 'occupied_female'):
                self.assertIn(bed[sex]['key'], beds)
                self.assertEqual(bed[sex]['revision'], 'definition-r1')
        original = copy.deepcopy(next(row for row in profiles['records']
                                     if row['production_definition']['production_key'] == 'oteryn:item.tibia.i696'))
        row = copy.deepcopy(original)
        row['semantics'].pop('bed')
        before = copy.deepcopy(row)
        apply_bed_semantics([row], profiles)
        self.assertEqual(row['semantics']['bed'], original['semantics']['bed'])
        row['semantics'].pop('bed')
        self.assertEqual(row, before)
        bad = copy.deepcopy(original)
        bad['semantics']['bed']['value']['partner_direction'] = 'WEST'
        with self.assertRaisesRegex(ValueError, 'HOUSE_BED_SEMANTICS_CONFLICT'):
            apply_bed_semantics([bad], profiles)

    def test_captured_source_families_are_complete_without_invented_targets(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        self.assertEqual(len(catalogue['carpets']), 160)
        self.assertEqual(len(catalogue['windows']), 104)
        self.assertEqual(len(catalogue['music']), 33)
        self.assertEqual(len(catalogue['mirror']['item_ids']), 8)
        self.assertEqual(len(catalogue['magic_trees']), 8)
        self.assertEqual(len(catalogue['bed_kits']), 5)
        self.assertEqual(catalogue['mirror']['cooldown_ms'], 72_000_000)
        self.assertEqual(catalogue['fish_tank']['effect_binding'], 'canary.appearance:effect/plunging_fish')
        known = {row['production_definition']['production_key'] for row in profiles['records']}
        for field in ('carpets', 'windows', 'magic_trees', 'native_transforms'):
            for row in catalogue[field]:
                self.assertRegex(row['donor'], r'[^:]+:\d+')
                self.assertIn(f"oteryn:item.tibia.i{row['item_id']}", known)
                self.assertIn(f"oteryn:item.tibia.i{row['target']}", known)
        self.assertEqual({(row['item_id'], row['target']) for row in catalogue['unavailable_native_transforms']},
                         {(34276, 34277), (37782, 37883), (26099, 23397)})
        for row in catalogue['native_transforms']:
            if row.get('noop'):
                self.assertEqual(row['item_id'], row['target'])
                self.assertNotEqual(row['requested_target'], row['target'])
                self.assertEqual(row['engine_donor'], 'src/game/game.cpp:3054-3057')
        pairs = sql_pairs(catalogue)
        self.assertIn(b"('oteryn:item.tibia.i37365','oteryn:item.tibia.i37366')", pairs)
        self.assertIn(b"('oteryn:item.tibia.i2603','oteryn:item.tibia.i2603')", pairs)
        self.assertNotIn(b"'oteryn:item.tibia.i34277'", pairs)
        self.assertNotIn(b"'oteryn:item.tibia.i37883'", pairs)
        self.assertIn(b"('oteryn:item.tibia.i831','oteryn:item.tibia.i2491','oteryn:item.tibia.i734')", pairs)
        self.assertIn(b"('oteryn:item.tibia.i831','oteryn:item.tibia.i2492','oteryn:item.tibia.i735')", pairs)
        self.assertNotIn(b"('oteryn:item.tibia.i832','oteryn:item.tibia.i2491','oteryn:item.tibia.i734')", pairs)
        rotations = {(row['item_id'], row['target']) for row in catalogue['native_transforms'] if row['kind'] == 'ROTATE'}
        self.assertTrue({(35949,35969),(35969,35949),(35951,35971),(35971,35951)} <= rotations,
                        'both aqueduct steady phases retain their source rotations after decay')
        self.assertTrue(encode(catalogue).endswith(b'\n'))

    def test_admission_preserves_existing_specialized_facts_and_adds_closed_sources(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        overlay = json.loads((ROOT / PROFILES).read_bytes())
        existing = copy.deepcopy(next(row for row in overlay['records']
                                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i2603'))
        existing['semantics']['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 9999}
        with TemporaryDirectory() as directory:
            root = Path(directory)
            for path, document in ((CATALOGUE, catalogue), (PROFILES, overlay)):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_bytes(encode(document))
            generated = {'items.json': encode({'records': [existing]}), 'manifest.json': encode({'item_profiles': {}})}
            admit(root, generated, 'items.json', 'manifest.json',
                  lambda row: row['production_definition']['production_key'], encode)
            profiles = {row['production_definition']['production_key']: row
                        for row in json.loads(generated['items.json'])['records']}
            self.assertEqual(profiles['oteryn:item.tibia.i2603']['semantics']['physical'], existing['semantics']['physical'])
            self.assertIn('oteryn:item.tibia.i37365', profiles)
            self.assertIsInstance(profiles['oteryn:item.tibia.i2603']['attributes']['always_on_top'], bool)

    def test_source_carpet_and_mirror_have_real_native_policies(self):
        profiles = {row['production_definition']['production_key']: row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        # Crystal carpets.lua:2-162; wall_mirror.lua:28 registers these real inputs.
        for item in (37365, 37366, 42340, 42341, 2603, 2976, 2975, 26076, 26077):
            profile = profiles[f'oteryn:item.tibia.i{item}']
            self.assertEqual(profile['production_binding']['disposition'], 'EXACT')
            self.assertTrue(profile['admission']['materializable'])
            self.assertIn('Ground', profile['admission']['legal_destinations'])
            self.assertIsInstance(profile['attributes']['always_on_top'], bool)
            self.assertIn(profile['attributes']['always_on_top_order'], (0, 1, 2, 3))

    def test_native_tile_specs_are_complete_read_only_source_facts(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        rows = catalogue['native_item_specs']
        specs = {item: row for row in rows for item in row['item_ids']}
        self.assertEqual(sum(len(row['item_ids']) for row in rows), len(specs))
        self.assertEqual(len(specs), 43392)
        self.assertEqual(sum(row['carpet'] for row in specs.values()), 161)
        self.assertFalse(specs[26151]['carpet'])
        self.assertTrue(specs[51222]['carpet'])
        self.assertTrue(specs[51223]['carpet'])
        self.assertIn(100, specs)
        self.assertNotIn(0, specs)
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        self.assertNotIn('oteryn:item.tibia.i100', {row['production_definition']['production_key'] for row in profiles['records']},
                         'read-only source specs grant no Item materialization')
        self.assertIn(b'FUNCTION game_house_native_item_spec(p_item_id BIGINT)', sql_pairs(catalogue, profiles))

    def test_native_tile_specs_apply_cpp_precedence_and_xml_overrides(self):
        import xml.etree.ElementTree as ET
        xml = {1: ET.fromstring('<item id="1"><attribute key="type" value="carpet"/><attribute key="movable" value="false"/><attribute key="blocking" value="true"/></item>'),
               2: ET.fromstring('<item id="2"><attribute key="type" value="carpet"/></item>')}
        objects = {1: {'flags': {'flags.clip': True, 'flags.top': True, 'flags.has_height': True}},
                   2: {'flags': {}}}
        rows = native_item_specs(xml, objects, {1}, 38, {'carpet', 'container'})
        self.assertEqual(rows, [{'item_ids': [1], 'carpet': True, 'movable': False,
                                'always_on_top': True, 'always_on_top_order': 1,
                                'block_solid': True, 'has_height': True}])

    def test_initialized_wrap_eligibility_preserves_runtime_output_and_source_intent(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        rows = catalogue['wrap_eligibility']
        self.assertEqual(catalogue['wrap_eligible'], sorted({row['item_id'] for row in rows}))
        self.assertEqual(len(rows), 1860)
        self.assertTrue({10210,10211,35949,35950,35951,35952,35969,35970,35971,35972} <= set(catalogue['wrap_eligible']))
        by_id = {row['item_id']: row for row in rows}
        self.assertEqual(by_id[6109]['source_wrapable_to'], 6115)
        self.assertEqual(by_id[6109]['target'], 23398, 'Game::wrapItem:4898 emits the modern kit independently of XML target')
        self.assertEqual(by_id[10210]['source_wrapable_to'], 23398)
        self.assertEqual(by_id[10210]['donor'], 'src/items/items.cpp:238-241')
        known = {row['production_definition']['production_key'] for row in profiles['records']}
        self.assertTrue(all(f"oteryn:item.tibia.i{item}" in known for item in catalogue['wrap_eligible']))
        self.assertIn(b'FUNCTION game_house_native_wrap_eligible(p_source TEXT)', sql_pairs(catalogue, profiles))

    def test_native_xml_wrap_pairs_are_exact_forward_evidence(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        keys = {row['production_definition']['production_key'] for row in profiles['records']}
        pairs = {(row['item_id'], row['target']) for row in catalogue['wrap_pairs']}
        self.assertEqual(len(pairs), 1617)
        self.assertTrue({(6355,6371),(34276,23398),(35949,23398),(35969,23398)} <= pairs)
        self.assertNotIn((23398,34276), pairs, 'reverse unwrap needs prior-wrap or purchased-origin provenance')
        for row in catalogue['wrap_pairs']:
            self.assertRegex(row['donor'], r'data/items/items.xml:\d+')
            self.assertIn(f"oteryn:item.tibia.i{row['item_id']}", keys)
            self.assertIn(f"oteryn:item.tibia.i{row['target']}", keys)
        self.assertIn(b'FUNCTION game_house_native_wrap_pair(p_source TEXT,p_target TEXT)', sql_pairs(catalogue, profiles))

    def test_initialized_appearance_requires_explicit_id_and_flags(self):
        # Crystal items.cpp:156-169,226 excludes missing flags, including an
        # object represented by the parser's default empty dictionary.
        self.assertEqual(initialized_item_ids(bytes.fromhex('0a0208010a0408021a000a021a00')), {2})

    def test_xml_only_carpets_are_qualified_without_fabricated_use_handlers(self):
        overlay = json.loads((ROOT / PROFILES).read_bytes())
        records = {row['production_definition']['production_key']: row for row in overlay['records']}
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        for item in (51222, 51223):
            row = records[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['semantics']['classification']['value']['item_type'], {'state':'KNOWN','value':'CARPET'})
            self.assertNotIn(item, {action['item_id'] for action in catalogue['carpets']})
        self.assertEqual(sum(row['semantics']['classification'].get('value', {}).get('item_type') == {'state':'KNOWN','value':'CARPET'} for row in records.values()), 161)
        self.assertNotEqual(records['oteryn:item.tibia.i26151']['semantics']['classification']['value']['item_type'], {'state':'KNOWN','value':'CARPET'})
        existing = copy.deepcopy(records['oteryn:item.tibia.i37365'])
        existing['semantics']['classification']['value']['item_type'] = {'state':'NA'}
        existing['semantics']['classification']['value']['capabilities'] = {'state':'KNOWN','value':[]}
        apply_carpet_types([existing], overlay)
        self.assertEqual(existing['semantics']['classification']['value']['item_type'], {'state':'KNOWN','value':'CARPET'})
        self.assertEqual(existing['semantics']['classification']['value']['capabilities'], {'state':'KNOWN','value':[]})

    def test_native_decoration_temporal_targets_are_closed(self):
        profiles = {row['production_definition']['production_key']: row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for source, target in ((35949, 35950), (35969, 35970)):
            row = profiles[f'oteryn:item.tibia.i{source}']
            temporal = row['semantics']['temporal']['value']
            self.assertEqual(temporal['decay_target']['value']['key'], f'oteryn:item.tibia.i{target}')
            self.assertTrue(f'oteryn:item.tibia.i{target}' in profiles,
                            f'native decay target {target} must be admitted')

    def test_srx_updates_keep_source_top_order_after_specialized_overrides(self):
        facts = json.loads((ROOT / PROFILES).read_bytes())['always_on_top']
        records = json.loads((ROOT / 'content/items/definitions/srx1-item-updates.json').read_bytes())['records']
        for row in records:
            item = row['production_definition']['production_key'].removeprefix('oteryn:item.tibia.i')
            if item in facts:
                self.assertEqual(row['attributes']['always_on_top_order'], facts[item])
                self.assertEqual(row['attributes']['always_on_top'], facts[item] != 0)

    def test_top_order_enrichment_preserves_specialized_rules_and_unknowns(self):
        rows = [{'production_definition': {'production_key': 'oteryn:item.tibia.i1'},
                 'attributes': {'writing_policy': 'specialized'}, 'semantics': {'specialized': True}},
                {'production_definition': {'production_key': 'oteryn:item.tibia.i2'},
                 'attributes': {'writing_policy': 'specialized'}, 'semantics': {'specialized': True}}]
        before_unknown = copy.deepcopy(rows[1])
        apply_top_order(rows, {'always_on_top': {'1': 3}})
        self.assertEqual(rows[0]['attributes'], {'writing_policy': 'specialized',
                                               'always_on_top': True, 'always_on_top_order': 3})
        self.assertEqual(rows[0]['semantics'], {'specialized': True})
        self.assertEqual(rows[1], before_unknown)


if __name__ == '__main__':
    unittest.main()
