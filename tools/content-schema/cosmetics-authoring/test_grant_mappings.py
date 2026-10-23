"""APP-3 authored grant mappings through the existing authoring fixture/materializer."""
import copy
import unittest

import cosmetics_authoring as c
import test_cosmetics_authoring as existing
from jsonschema import ValidationError


class GrantMappings(unittest.TestCase):
    def setUp(self):
        self.fixture = existing.CosmeticsAuthoring()
        self.fixture.setUp()
        document = self.fixture.authoring["document"]
        self.mapping = {
            "grant_key": "quest:wardrobe:reward",
            "target": {"kind": "Addon", "outfit_key": document["outfits"][0]["key"], "addon": 1},
            "compatible_provenance": ["previous-content-r7", "previous-content-r9"],
        }
        promotion = copy.deepcopy(self.fixture.authoring["promotions"][0])
        promotion["path"] = "/grant_mappings"
        self.fixture.authoring["promotions"].append(promotion)
        self.fixture.replace("/grant_mappings", [self.mapping])

    def test_authored_historical_provenance_survives_materialization(self):
        packet = c.parse(c.build(self.fixture.authoring, c.ROOT / c.MANIFEST))
        mappings = packet["authoring"]["document"]["grant_mappings"]
        self.assertEqual(mappings, [self.mapping])
        self.assertNotIn(packet["authoring"]["document"]["content_revision"],
                         mappings[0]["compatible_provenance"])

    def test_grants_cannot_bypass_the_promotion_gate(self):
        self.fixture.authoring["promotions"].pop()
        with self.assertRaisesRegex(ValueError, "incomplete"):
            c.validate(self.fixture.authoring)

    def test_duplicate_keys_unknown_targets_and_invalid_addons_are_refused(self):
        for mappings in [
            [self.mapping, self.mapping],
            [{**self.mapping, "target": {"kind": "Outfit", "outfit_key": "oteryn:missing"}}],
            [{**self.mapping, "target": {**self.mapping["target"], "addon": 0}}],
            [{**self.mapping, "compatible_provenance": ["same", "same"]}],
        ]:
            with self.subTest(mappings=mappings):
                self.fixture.replace("/grant_mappings", mappings)
                with self.assertRaises((ValueError, ValidationError)):
                    c.validate(self.fixture.authoring)
