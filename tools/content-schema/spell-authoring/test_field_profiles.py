"""Golden source fields exercise source ramp/order and honest known-none."""
import json
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET
from build_field_profiles import build,recipe

class FieldProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profiles=build()['profiles']
    def field(self,item,server='canary'):
        return next(row['condition'] for row in self.profiles if row['source']['server']==server and row['source']['server_item_id']==item)
    def test_actual_poison_ramp_has_one_hundred_damage_and_source_order(self):
        entries=self.field(105)['entries']
        self.assertEqual([(x['amount'],x['repetitions']) for x in entries],[(5,4),(4,5),(3,7),(2,9),(1,21)])
        self.assertEqual(sum(x['amount']*x['repetitions'] for x in entries),100)
        self.assertTrue(all(x['interval_ms']==5000 for x in entries))
    def test_actual_fire_and_energy_keep_exact_count_and_interval(self):
        self.assertEqual(self.field(2118)['entries'],[{'amount':20,'interval_ms':10000,'repetitions':7}])
        self.assertEqual(self.field(2122)['entries'],[{'amount':25,'interval_ms':10000,'repetitions':1}])
    def test_empty_fire_is_elemental_and_barriers_have_explicit_known_none(self):
        self.assertEqual(self.field(2120),{'kind':'damage','element':'fire','entries':[]})
        self.assertEqual(self.field(2128),{'kind':'none'})
        self.assertEqual(self.field(2130),{'kind':'none'})
        self.assertTrue(any(x['condition']['kind']=='unsupported' for x in self.profiles if x['source']['server']=='crystal'))
    def test_xml_damage_keeps_attribute_order_and_source_interval_floor(self):
        field=ET.fromstring('<attribute key="field" value="fire"><attribute key="damage" value="3"/><attribute key="ticks" value="5000"/><attribute key="count" value="2"/><attribute key="damage" value="7"/></attribute>')
        self.assertEqual(recipe(field)['entries'],[{'amount':3,'interval_ms':1000,'repetitions':1},{'amount':7,'interval_ms':5000,'repetitions':2}])
    def test_checked_in_catalog_is_reproduced_from_all_exact_pinned_blobs(self):
        expected=json.loads((Path(__file__).parent/'samples/native-field-profiles.json').read_text())
        self.assertEqual(self.profiles,expected['profiles'])
        self.assertEqual(len(self.profiles),90)

if __name__=='__main__':unittest.main()
