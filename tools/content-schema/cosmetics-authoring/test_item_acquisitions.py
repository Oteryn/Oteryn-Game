"""Source acquisitions require independent owner promotion, not just mapped appearances."""
import copy
import unittest
from jsonschema import ValidationError
import cosmetics_authoring as c
import test_cosmetics_authoring as existing

class ItemAcquisitions(unittest.TestCase):
    def setUp(self):
        self.fixture=existing.CosmeticsAuthoring();self.fixture.setUp()
        row={'item_key':'oteryn:item.tibia.i6579','donor':'source:addon_doll.lua:67',
             'catalogue_sha256':'11'*32,'decision':'owner:source-acquisition',
             'source':copy.deepcopy(self.fixture.authoring['promotions'][0]['source'])}
        self.row=row
        self.fixture.authoring['document']['item_acquisitions']=[row]
        p=copy.deepcopy(self.fixture.authoring['promotions'][0])
        p.update(path='/item_acquisitions',value=[row],source_kind='OWNER_DECISION')
        self.fixture.authoring['promotions'].append(p)
    def test_explicit_admission_materializes_and_cannot_lose_its_owner_promotion(self):
        packet=c.parse(c.build(self.fixture.authoring,c.ROOT/c.MANIFEST))
        self.assertEqual(packet['authoring']['document']['item_acquisitions'],[self.row])
        self.fixture.authoring['promotions'][-1]['source_kind']='STRUCTURED_REFERENCE_DATA'
        with self.assertRaisesRegex(ValueError,'explicit owner'):c.validate(self.fixture.authoring)
        self.fixture.authoring['promotions'].pop()
        with self.assertRaisesRegex(ValueError,'incomplete'):c.validate(self.fixture.authoring)
    def test_duplicate_unbound_and_malformed_source_admissions_are_rejected(self):
        for rows in [[self.row,self.row],[dict(self.row,catalogue_sha256='0'*64)],
                     [dict(self.row,decision='implicit')],[dict(self.row,item_key='oteryn:creature.native')],
                     [self.row]*129]:
            with self.subTest(rows=len(rows)):
                self.fixture.replace('/item_acquisitions',rows)
                with self.assertRaises((ValidationError,ValueError)):c.validate(self.fixture.authoring)
