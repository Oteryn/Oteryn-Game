"""Focused parsing and comparison checks without a live API request."""
from decimal import Decimal
import unittest
from wiki_current_comparison import infobox_fields, numeric, link_names, compare


class CurrentWikiFacts(unittest.TestCase):
    def test_nested_templates_and_link_pipes_do_not_create_fields(self):
        text='{{Infobox_Criatura|List={{{1|}}}|hp=20|lootcomum={{X|a=b}} [[Gold Coin|coins]]|exp=5}}'
        fields=infobox_fields(text)
        self.assertEqual(fields['hp'],'20')
        self.assertEqual(fields['exp'],'5')
        self.assertEqual(fields['lootcomum'],'{{X|a=b}} [[Gold Coin|coins]]')
        self.assertNotIn('a',fields)

    def test_last_field_does_not_include_closing_template(self):
        self.assertEqual(infobox_fields('{{Infobox_Criatura|hp=20}}')['hp'],'20')

    def test_zero_is_a_known_numeric_fact(self):
        self.assertEqual(numeric('0'),Decimal(0))
        self.assertIsNone(numeric('unknown'))

    def test_damage_modifier_is_not_a_reduction(self):
        monster={'creature':{'stats':{'max_health':20,'experience':5,'armor':1,'defense':5,'speed':134},
            'flags':{'illusionable':True},'spawn_eligibility':{'blocked_by_nearby_players':False},
            'summoning':{'summonable':False,'convinceable':False},
            'resistances':[{'damage_type':'earth','reduction_percent':{'numerator':20,'denominator':1}}]},
            'behavior':{'movement':{'pushable':True,'push_items':False}}}
        rows=compare(monster,{'earthDmgMod':'80%','speed':'67','defense':'1'}, {})
        earth=next(row for row in rows if row['field']=='damage_received_percent/earth')
        speed=next(row for row in rows if row['field']=='speed')
        armor=next(row for row in rows if row['field']=='armor')
        self.assertEqual(armor['classification'],'MATCH')
        self.assertFalse(any(row['field']=='defense' for row in rows))
        self.assertEqual(earth['classification'],'MATCH')
        self.assertEqual(speed['classification'],'NOT_COMPARABLE')

    def test_loot_link_label_is_not_the_item_identity(self):
        self.assertEqual(link_names('0-4 [[Gold Coin]]s, [[Grey Small Book|book]].'),['Gold Coin','Grey Small Book'])


if __name__=='__main__': unittest.main()
