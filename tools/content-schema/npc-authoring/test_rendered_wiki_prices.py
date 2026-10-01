"""Regression cases for rendered reference prices; no native admission implied."""
import json
import unittest
from pathlib import Path

from rendered_wiki_prices import br_rows, capture_facts, compare_pending, fandom_rows


def br_item(name, price):
    return (f'[{name}](https://www.tibiawiki.com.br/wiki/{name.replace(" ", "_")})'
            f'![Image 2: {name}.gif](https://example.com/{name.replace(" ", "_")}.gif)'
            f'{price} [gp](https://www.tibiawiki.com.br/wiki/Gp "Gp") ')


class RenderedPriceTests(unittest.TestCase):
    def test_independent_review_contamination_regressions(self):
        path = Path(__file__).parent / 'samples/rendered-price-parser-fixtures.json'
        for case in json.loads(path.read_text(encoding='utf-8')):
            with self.subTest(case=case['id']):
                page = capture_facts({'url': case['url'], 'raw_content': case['raw_content']}, 'test')
                self.assertEqual(len(page['rows']), case['expected_rows'])

    def test_directions_nested_urls_unicode_and_exact_row_bytes(self):
        text = '# NPC\nÁrvore **Itens negociáveis:**Vende: ' + br_item('Potion (Small)', '1 500')
        text += 'Compra: ' + br_item('Potion (Small)', '0') + '\n## Diálogo\n' + br_item('Fake', '900')
        rows = br_rows(text)
        self.assertEqual([(r['direction'], r['item_name'], r['unit_price']) for r in rows],
                         [('SellToPlayer', 'Potion (Small)', 1500), ('BuyFromPlayer', 'Potion (Small)', 0)])
        for row in rows:
            raw = text.encode()[row['byte_start']:row['byte_end']].decode()
            self.assertTrue(raw.startswith('[Potion (Small)]'))
            self.assertTrue(raw.endswith('"Gp")'))

    def test_ambiguous_prices_and_non_gold_not_scalar(self):
        text = '# NPC\n**Itens negociáveis:**Vende: '
        text += br_item('Arrow', '10 por 100') + br_item('Parcel', '15 (10 após quest)')
        text += '[Token](https://www.tibiawiki.com.br/wiki/Token)5 silver tokens '
        self.assertEqual(br_rows(text), [])

    def test_conditional_shop_context_retains_hash_and_scope_hold(self):
        text = '# NPC\n**Itens negociáveis:**Vende: Somente durante um evento: ' + br_item('Egg', '2')
        row = br_rows(text)[0]
        self.assertEqual(row['scope_holds'], ['CONDITIONAL_OR_HISTORICAL_SHOP_CONTEXT'])
        context = text.encode()[row['context_byte_start']:row['context_byte_end']].decode()
        self.assertIn('Somente durante um evento', context)

    def test_wrong_requested_identity_and_error_page_hold_prices(self):
        raw = '# Other NPC\n**Itens negociáveis:**Vende: ' + br_item('Arrow', '3')
        page = capture_facts({'url': 'https://www.tibiawiki.com.br/wiki/NPC', 'raw_content': raw}, 'request')
        self.assertFalse(page['identity_heading_matches'])
        self.assertEqual(page['rows'], [])

    def test_fandom_shop_zero_price_and_transcript_contamination(self):
        row = ('| image | [Berry](https://tibia.fandom.com/wiki/Berry "Berry") | '
               '0 ![Image 2: Gold](https://example.com/gold.gif) |\n')
        text = '# NPC\n### Buys\n' + row + '### Transcripts\n' + row.replace('0 !', '9 !')
        self.assertEqual([(r['direction'], r['unit_price']) for r in fandom_rows(text)],
                         [('BuyFromPlayer', 0)])
        ambiguous = row.replace('0 !', '0 or 9 !')
        self.assertEqual(fandom_rows('### Buys\n' + ambiguous), [])

    def test_same_item_duplicate_rows_conflict_and_missing_do_not_qualify(self):
        pending = {'records': [{'npc': 'npc', 'name': 'NPC', 'service': 'service',
                               'native_offer_tuple': {'unit_price': 15},
                               'source_candidate_pointer': '/candidate',
                               'source_offer': {'item': {'key': 'item'}, 'direction': 'SellToPlayer',
                                                'unit_price': 15, 'parity_pending': {'fandom': 10}}}]}
        page = capture_facts({'url': 'https://www.tibiawiki.com.br/wiki/NPC',
                              'raw_content': '# NPC\n**Itens negociáveis:**Vende: '
                              + br_item('Parcel', '15') + br_item('Parcel', '10')}, 'request')
        result = compare_pending(pending, {'pages': [page]}, {'item': 'parcel'})
        record = result['records'][0]
        self.assertEqual(record['status'], 'RENDERED_PRICE_CONFLICT')
        self.assertEqual([r['price'] for r in record['observations']], [15, 10])
        self.assertFalse(record['price_parity_qualified'])
        self.assertFalse(result['active_content_modified'])
        self.assertEqual(compare_pending(pending, {'pages': []}, {})['records'][0]['status'],
                         'NO_EXACT_ITEM_PRICE')


if __name__ == '__main__':
    unittest.main()
