"""Regression cases for malformed candidates and name-only wiki evidence."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import promotion_candidates as pc
import tibiawiki_br_crosscheck as br
import validate_npc
import validate_promotion as vp
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent


def read(name):
    return json.loads((ROOT / name).read_text(encoding='utf-8'))


class SchemaRegressionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = Draft202012Validator(read('npc.schema.json'))
        cls.bundle = read('samples/bundles/canary/sam.json')
        cls.report = read('samples/promotion-candidates-v1.json')

    def bundle_errors(self, bundle):
        errors = list(self.schema.iter_errors(bundle))
        return errors or validate_npc.semantic_errors(bundle, False)

    def test_blessing_types_and_symbolic_price(self):
        bundle = copy.deepcopy(self.bundle)
        bundle['dialogue']['keywords'].append({'keywords': ['blessing'], 'kind': 'bless', 'text': None,
            'flags': {}, 'gate': 'NONE', 'effect': 'NONE', 'children': []})
        index = len(bundle['dialogue']['keywords']) - 1
        row = {'keyword': 'blessing', 'blessing': 1, 'price': '|PVPBLESSCOST|', 'gate': 'NONE',
               'dialogue_path': f'dialogue.keywords[{index}]'}
        bundle['services']['blessings'] = [row]
        self.assertEqual(self.bundle_errors(bundle), [])
        for field, value in [('keyword', {}), ('blessing', {}), ('price', -9), ('gate', 'GUESS')]:
            with self.subTest(field=field):
                mutated = copy.deepcopy(bundle)
                mutated['services']['blessings'][0][field] = value
                self.assertTrue(self.bundle_errors(mutated))

    def test_promotion_has_typed_optional_values(self):
        bundle = copy.deepcopy(self.bundle)
        bundle['services']['promotion'] = [{'keyword': 'promot', 'price': -9, 'min_level': {},
            'promotion': None, 'gate': 'NONE', 'dialogue_path': 'dialogue.keywords[0]'}]
        self.assertTrue(self.bundle_errors(bundle))

    def test_all_service_modules_require_existing_matching_node(self):
        for file in (ROOT / 'samples/bundles').rglob('*.json'):
            bundle = json.loads(file.read_text(encoding='utf-8'))
            for service in ('travel', 'spells', 'blessings', 'promotion', 'kick'):
                if not bundle['services'][service]:
                    continue
                for bad_path in ('does.not.exist', 'dialogue.keywords[-1]', 'dialogue.keywords[999999]'):
                    with self.subTest(file=file.name, service=service, bad_path=bad_path):
                        mutated = copy.deepcopy(bundle)
                        mutated['services'][service][0]['dialogue_path'] = bad_path
                        self.assertTrue(self.bundle_errors(mutated))
                mutated = copy.deepcopy(bundle)
                path = mutated['services'][service][0]['dialogue_path']
                nodes = dict(validate_npc.gated_nodes(mutated['dialogue']['keywords']))
                nodes[path]['kind'] = 'say'
                self.assertTrue(any('must name a' in str(e) for e in self.bundle_errors(mutated)))

    def test_inherited_gate_and_converter_paths_remain_valid(self):
        for file in (ROOT / 'samples/bundles').rglob('*.json'):
            with self.subTest(file=file.name):
                self.assertEqual(self.bundle_errors(json.loads(file.read_text(encoding='utf-8'))), [])

    def test_route_bad_fields_rejected_and_postman_allowed(self):
        candidate = copy.deepcopy(next(c for c in self.report['candidates'] if c['travel_service']))
        route = candidate['travel_service']['routes'][0]
        for field, value in [('premium', 'free'), ('min_level', -900), ('min_level', True),
                             ('min_level', 2**32), ('discount', 'Invented'), ('price', 2**64),
                             ('destination_keyword', {}), ('destination_keyword', ' Gray Island')]:
            with self.subTest(field=field, value=value):
                altered = copy.deepcopy(candidate)
                altered['travel_service']['routes'][0][field] = value
                self.assertTrue(vp.candidate_errors(altered, 0))
        route.update(discount='postman', destination_keyword='gray island')
        self.assertEqual(vp.candidate_errors(candidate, 0), [])

    def test_definition_shape_and_unsigned_ranges(self):
        candidate = copy.deepcopy(self.report['candidates'][0])
        for field, value in [('presentation', 'broken'), ('movement', {'walk_interval_ms': -1})]:
            altered = copy.deepcopy(candidate)
            altered[field] = value
            self.assertTrue(vp.candidate_errors(altered, 0))
        for field, value in [('head', -1), ('addons', 4), ('look_type', 2**32)]:
            altered = copy.deepcopy(candidate)
            altered['presentation']['outfit'][field] = value
            self.assertTrue(vp.candidate_errors(altered, 0))

    def test_offer_unsigned_ranges(self):
        candidate = copy.deepcopy(next(c for c in self.report['candidates'] if c['trade_service']))
        for field, value in [('sub_type', -1), ('sub_type', 65536), ('sub_type', True),
                             ('count', 0), ('count', 2**32), ('unit_price', 2**64)]:
            altered = copy.deepcopy(candidate)
            altered['trade_service']['offers'][0][field] = value
            self.assertTrue(vp.candidate_errors(altered, 0))

    def test_D12_does_not_override_nonplain_variants(self):
        for count, subtype in [(20, 7), (20, None), (None, 7)]:
            bundle = copy.deepcopy(self.bundle)
            bundle['services']['trade']['offers'] = [{'item_name': 'test rune', 'client_id': 100,
                'server_item_id': None, 'buy_price': 500, 'sell_price': None, 'count': count,
                'sub_type': subtype, 'stock_gate': None}]
            snapshot = {'npcs': [], 'trade': {'sam': [{'item': 'test rune', 'buy_price': 100, 'sell_price': None}]}}
            items = {'records': [{'source_item_id': 100, 'native_key': 'oteryn:item.test_rune',
                                 'native_revision': 'definition-r1'}]}
            facts = {'pages': [{'title': 'Sam', 'name': 'Sam', 'trades': {'SellToPlayer': {'test rune': [100]}}}]}
            arbitration, left_out = [], []
            builder = pc.Builder(snapshot, items, facts)
            offers = builder.merge_offers({'canary': bundle, 'crystal': copy.deepcopy(bundle)},
                                          'Sam', arbitration, left_out)
            self.assertEqual((offers[0]['unit_price'], arbitration, left_out), (500, [], []))
            snapshot_bytes, facts_bytes = json.dumps(snapshot).encode(), json.dumps(facts).encode()
            report = {'snapshot_sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
                'br_facts_sha256': hashlib.sha256(facts_bytes).hexdigest(),
                'candidates': [{'name': 'Sam', 'arbitration': [], 'trade_service': {'offers': offers}}]}
            self.assertEqual(vp.wiki_price_errors(report, snapshot_bytes, facts_bytes,
                                                {'oteryn:item.test_rune': 'test rune'}), [])

    def test_variant_price_arbitration_is_rejected(self):
        candidate = copy.deepcopy(next(c for c in self.report['candidates']
                                      if any(a['rule'] == 'WIKI_PRICE' for a in c['arbitration'])))
        row = next(a for a in candidate['arbitration'] if a['rule'] == 'WIKI_PRICE')
        item_id = int(row['fact'].split('.')[1])
        direction = row['fact'].rsplit('.', 1)[1]
        offer = next(o for o in candidate['trade_service']['offers']
                     if o['source_item_id'] == item_id and o['direction'] == direction)
        offer['count'] = 20
        row['fact'] = f'trade.{item_id}x20.{direction}'
        self.assertTrue(any('name-only wiki evidence' in e for e in vp.candidate_errors(candidate, 0)))
        for invalid_fact in (None, {}, 0):
            with self.subTest(invalid_fact=invalid_fact):
                row['fact'] = invalid_fact
                self.assertTrue(vp.candidate_errors(candidate, 0))

    def test_name_only_wiki_cannot_select_conflicting_or_one_sided_variant(self):
        for count, subtype in ((20, None), (None, 7), (20, 7)):
            bundle = copy.deepcopy(self.bundle)
            row = {'item_name': 'test rune', 'client_id': 100, 'server_item_id': None,
                   'buy_price': 100, 'sell_price': None, 'count': count, 'sub_type': subtype,
                   'stock_gate': None}
            bundle['services']['trade']['offers'] = [row]
            builder = pc.Builder({'npcs': [], 'trade': {'sam': [
                {'item': 'test rune', 'buy_price': 100, 'sell_price': None}]}}, {'records': [
                {'source_item_id': 100, 'native_key': 'oteryn:item.test_rune', 'native_revision': 'definition-r1'}]})
            other = copy.deepcopy(bundle)
            other['services']['trade']['offers'][0]['buy_price'] = 500
            empty = copy.deepcopy(bundle)
            empty['services']['trade']['offers'] = []
            for sources in ({'canary': other, 'crystal': bundle}, {'canary': empty, 'crystal': bundle}):
                arbitration, left_out = [], []
                self.assertEqual(builder.merge_offers(sources, 'Sam', arbitration, left_out), [])
                self.assertEqual(arbitration, [])
                self.assertEqual(len(left_out), 1)
                self.assertIn(left_out[0]['reason'], ('OFFER_UNCONFIRMED', 'OFFER_CONFLICT_WIKI_UNDECIDED'))

    def test_identical_OT_variants_remain_source_evidence(self):
        bundle = copy.deepcopy(self.bundle)
        bundle['services']['trade']['offers'] = [{'item_name': 'test rune', 'client_id': 100,
            'server_item_id': None, 'buy_price': 500, 'sell_price': None, 'count': 20,
            'sub_type': 7, 'stock_gate': None}]
        builder = pc.Builder({'npcs': [], 'trade': {}}, {'records': [{'source_item_id': 100,
            'native_key': 'oteryn:item.test_rune', 'native_revision': 'definition-r1'}]})
        arbitration, left_out = [], []
        offers = builder.merge_offers({'canary': bundle, 'crystal': copy.deepcopy(bundle)},
                                      'Sam', arbitration, left_out)
        self.assertEqual((offers[0]['unit_price'], arbitration, left_out), (500, [], []))

    def test_variant_source_arbitration_is_rejected(self):
        candidate = copy.deepcopy(self.report['candidates'][0])
        candidate['arbitration'].append({'fact': 'trade.100x20s7', 'rule': 'WIKI_ARBITER',
                                       'chosen': next(iter(candidate['provenance']))})
        self.assertTrue(any('name-only wiki arbitration' in error
                            for error in vp.candidate_errors(candidate, 0)))

    def test_D16_plain_selection_remains_available(self):
        bundle = copy.deepcopy(self.bundle)
        bundle['services']['trade']['offers'] = [{'item_name': 'test rune', 'client_id': 100,
            'server_item_id': None, 'buy_price': 100, 'sell_price': None, 'count': None,
            'sub_type': None, 'stock_gate': None}]
        other = copy.deepcopy(bundle)
        other['services']['trade']['offers'][0]['buy_price'] = 500
        builder = pc.Builder({'npcs': [], 'trade': {'sam': [
            {'item': 'test rune', 'buy_price': 100, 'sell_price': None}]}}, {'records': [
            {'source_item_id': 100, 'native_key': 'oteryn:item.test_rune', 'native_revision': 'definition-r1'}]})
        arbitration, left_out = [], []
        offers = builder.merge_offers({'canary': other, 'crystal': bundle}, 'Sam', arbitration, left_out)
        self.assertEqual((offers[0]['unit_price'], arbitration, left_out),
            (100, [{'fact': 'trade.100', 'rule': 'WIKI_ARBITER', 'chosen': 'crystal'}], []))

    def test_current_D16_direction_prices_fit_native_u64(self):
        row = {'item_name': 'test rune', 'wikis': ['br', 'fandom'], 'directions': [
            {'direction': 'SellToPlayer', 'price': 100, 'status': 'CONFIRMED', 'wikis': ['br', 'fandom']}]}
        for value in (-1, 2 ** 64, True):
            changed = copy.deepcopy(row)
            changed['directions'][0]['price'] = value
            self.assertTrue(vp.arbiter_direction_errors('test', changed, 'crystal'))
        self.assertEqual(vp.arbiter_direction_errors('test', row, 'crystal'), [])

    def test_current_D16_direction_metadata_binds_admitted_price(self):
        candidate = copy.deepcopy(next(c for c in self.report['candidates']
            if any(r.get('rule') == 'WIKI_MAJORITY_ARBITER' for r in c['arbitration'])))
        row = next(r for r in candidate['arbitration'] if r.get('rule') == 'WIKI_MAJORITY_ARBITER')
        row['directions'][0]['price'] += 1
        self.assertTrue(any('admitted direction prices' in error for error in vp.candidate_errors(candidate, 0)))

    def test_current_D16_direction_metadata_cannot_omit_source_direction(self):
        candidate = copy.deepcopy(next(c for c in self.report['candidates']
            if any(r.get('rule') == 'WIKI_MAJORITY_ARBITER' and len(r['directions']) == 2
                   and any(d['status'] == 'UNCONFIRMED' for d in r['directions']) for r in c['arbitration'])))
        row = next(r for r in candidate['arbitration'] if r.get('rule') == 'WIKI_MAJORITY_ARBITER'
                   and len(r['directions']) == 2 and any(d['status'] == 'UNCONFIRMED' for d in r['directions']))
        row['directions'] = [d for d in row['directions'] if d['status'] == 'CONFIRMED']
        self.assertTrue(any('admitted direction prices' in error for error in vp.candidate_errors(candidate, 0)))

    def test_current_D16_arbiter_names_the_registered_item(self):
        report = copy.deepcopy(self.report)
        candidate = next(c for c in report['candidates']
                         if any(r.get('rule') == 'WIKI_MAJORITY_ARBITER' for r in c['arbitration']))
        row = next(r for r in candidate['arbitration'] if r.get('rule') == 'WIKI_MAJORITY_ARBITER')
        names = vp.registry_item_names()
        self.assertEqual(vp.item_name_errors(report, names), [])
        row['item_name'] = 'an unrelated registered item name'
        self.assertTrue(any('registered item' in error for error in vp.item_name_errors(report, names)))

    def test_BR_retired_alias_resolves_but_unknown_is_not_guessed(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            definitions = root / 'definitions'
            definitions.mkdir()
            (definitions / 'items-00000-00499.json').write_text(json.dumps({'records': [{'definition': {
                'identity': {'key': 'oteryn:item.current'}, 'semantics': {'presentation': {
                    'state': 'KNOWN', 'value': {'name': {'state': 'KNOWN', 'value': 'A Rune'}}}}}}]}), encoding='utf-8')
            aliases = root / 'aliases.json'
            aliases.write_text(json.dumps({'entries': [
                {'key': 'oteryn:item.retired', 'state': 'ALIAS', 'target': 'oteryn:item.current'},
                {'key': 'oteryn:item.unknown', 'state': 'ALIAS', 'target': 'oteryn:item.missing'},
                {'key': 'oteryn:item.gone', 'state': 'TOMBSTONE'}]}), encoding='utf-8')
            with patch.object(br, 'ITEMS', definitions), patch.object(br, 'ITEM_ALIASES', aliases):
                names = br.item_names()
            self.assertEqual(names, {'oteryn:item.current': 'a rune', 'oteryn:item.retired': 'a rune'})
            trade = br.check_trade([{'item': {'key': 'oteryn:item.retired'}, 'direction': 'SellToPlayer',
                                     'unit_price': 100}], {'SellToPlayer': {'a rune': [100]}, 'BuyFromPlayer': {}}, names)
            self.assertEqual(trade['status'], 'AGREE')


if __name__ == '__main__':
    unittest.main()
