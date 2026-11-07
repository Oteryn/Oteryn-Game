"""Loot presence parsing keeps uncertain amounts separate from names and rates."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import canary_batch as cb
import wiki_compare as wc


class LootItemQuantities(unittest.TestCase):
    def test_exact_captured_imperial_uncertain_quantity(self):
        # Imperial page109642/rev1195192, cut wikitext SHA256
        # ac83a4be2fcc81b837eb51ea33cb4d99ec16a864f10909be8f5e2c08edf59bfd.
        self.assertEqual({'meat'}, wc.wiki_loot('{{Loot Item|1-?|Meat|common}}'))

    def test_normal_and_reversed_order_with_certain_and_uncertain_counts(self):
        for quantity in ('1', '0-5', '1-?', '0-1?', '0-5+', '0-39+', '?', '~', '~1',
                         '1,000-2,000', '0 – 5', '0-?+', '1 +'):
            for arguments in (quantity+'|Gold Coin', 'Gold Coin|'+quantity):
                with self.subTest(arguments=arguments):
                    result = wc.wiki_loot_items('{{Loot Item|'+arguments+'|common}}')
                    self.assertEqual({'gold coin'}, result['names'])
                    self.assertEqual([], result['unresolved'])
                    self.assertEqual(result['names'], wc.wiki_loot('{{Loot Item|'+arguments+'}}'))

    def test_number_containing_names_do_not_become_quantity(self):
        for name in ('Key 4600', 'Book (Riddle 1)', '7-Layer Cake'):
            for arguments in (name+'|0-1', '0-1|'+name, name):
                with self.subTest(arguments=arguments):
                    self.assertEqual({name.lower()}, wc.wiki_loot('{{Loot Item|'+arguments+'}}'))

    def test_named_metadata_numeric_slots_links_and_whitespace(self):
        self.assertEqual({'key 4600'}, wc.wiki_loot(
            '{{ Loot_Item |image=Key.gif|2=Key 4600|1=0-1?|note=source observation}}'))
        self.assertEqual({'gold coin'}, wc.wiki_loot(
            '{{Loot Item|0-39+|[[Gold Coin|coins]]|rare|link=ignored metadata}}'))
        self.assertEqual({'gold coin'}, wc.wiki_loot('{{Loot Item||Gold Coin}}'))
        self.assertEqual({'gold coin'}, wc.wiki_loot('{{Loot Item|Gold Coin|always}}'))

    def test_missing_ambiguous_duplicate_and_uncertain_names_fail_closed(self):
        for arguments in ('', '0-1?', '?', '0-1|0-5', 'Gold Coin|Silver Token',
                          '1=Gold Coin|Gold Coin', 'name=Gold Coin|image=Coin.gif',
                          '0-1|Gold Coin?', '1-xyz|Meat', '0-1|{{Unknown Item}}'):
            with self.subTest(arguments=arguments):
                result = wc.wiki_loot_items('{{Loot Item|'+arguments+'}}')
                self.assertEqual(set(), result['names'])
                self.assertEqual(1, len(result['unresolved']))
        self.assertTrue(wc.wiki_loot_items('{{Loot Item|0-1|[[Gold Coin}')['unresolved'])

    def test_unknown_entry_keeps_other_definite_names_without_quantities_or_rates(self):
        result = wc.wiki_loot_items('{{Loot Table|{{Loot Item|0-5+|Gold Coin}}|'
                                    '{{Loot Item|Meat|Fish}}|{{Loot Item|1-?|Meat|common}}}}')
        self.assertEqual({'gold coin', 'meat'}, result['names'])
        self.assertEqual(1, len(result['unresolved']))
        self.assertEqual({'names', 'unresolved'}, set(result))

    def test_nested_item_never_escapes_unknown_argument_but_siblings_are_kept(self):
        for outer in ('{{Loot Item|1|{{Loot Item|Gold Coin}}}}',
                      '{{Loot Item|Gold Coin|{{Loot Item|Silver Token}}}}'):
            with self.subTest(outer=outer):
                result = wc.wiki_loot_items(outer)
                self.assertEqual(set(), result['names'])
                self.assertEqual(1, len(result['unresolved']))
                siblings = wc.wiki_loot_items('{{Loot Table|'+outer+
                    '|{{Loot Item|0-1|Meat}}|{{Loot Item|Fish}}}}')
                self.assertEqual({'meat', 'fish'}, siblings['names'])
                self.assertEqual(1, len(siblings['unresolved']))
        self.assertEqual({'meat'}, wc.wiki_loot('{{Loot Item|Meat|1|'
            'note={{Loot Item|Gold Coin}}}}'))

    def test_compact_keeps_unknown_presence_diagnostics_without_other_unknown_fields(self):
        loot = {'field':'loot.items', 'status':'WIKI_UNPARSED', 'only_canary':[],
                'only_wiki':['meat'], 'parse_diagnostics':[{'reason':'ambiguous name'}]}
        out = wc.compact({'monster':'synthetic','status':'COMPARED','rows':[
            loot, {'field':'armor','status':'WIKI_UNPARSED'}, {'field':'health','status':'MATCH'}]})
        self.assertEqual([loot], out['rows'])
        self.assertEqual([], out['loot_chances'])


class PresenceComparison(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        source=Path('/workspace/monster-reference-sources/canary')
        objects=cb.load_appearance_objects(source/'data/items/appearances.dat')
        items=cb.load_items_xml(source/'data/items/items.xml'); names,index=cb.name_index(objects,items)
        cls.converter=cb.Converter(source,objects,items,names,index)

    def test_unknown_names_not_sent_to_probability_lookup_or_reported_as_absent(self):
        cut={'title':'Dragon', 'page_id':1, 'revision_id':2,
             'revision_timestamp':'2026-09-26T00:00:00Z',
             'content':'{{Infobox Creature|loot={{Loot Table|{{Loot Item|Meat|Fish}}|'
                       '{{Loot Item|1|{{Loot Item|Demon Horn}}}}|'
                       '{{Loot Item|0-39+|Gold Coin}}}}}}'}
        record={'cut':cut,'current':cut,'retrieved_at':'2026-10-01T00:00:00Z'}
        called=[]
        def statistics(title,wanted,cache):
            called.extend(wanted); return {'status':'MISSING'}
        with tempfile.TemporaryDirectory() as cache, patch.object(cb,'CONVERTER',self.converter,create=True), \
             patch.object(wc,'creature_page',return_value=(record,{'status':'VERIFIED'})), \
             patch.object(wc,'loot_statistics',side_effect=statistics):
            result=wc.compare('dragons/dragon', self.converter.canary, None, Path(cache))
        row=next(r for r in result['rows'] if r['field']=='loot.items')
        self.assertEqual('WIKI_UNPARSED',row['status'])
        self.assertEqual([],row['only_canary']); self.assertTrue(row['unconfirmed_canary'])
        self.assertNotIn('meat',called); self.assertNotIn('fish',called)
        self.assertNotIn('demon horn',called)
        self.assertEqual([],row['only_wiki'])  # Gold Coin is already in Dragon's source loot.
        self.assertNotIn('loot_chances',result)
        self.assertEqual(row,next(r for r in wc.compact(result)['rows'] if r['field']=='loot.items'))


if __name__ == '__main__':
    unittest.main()
