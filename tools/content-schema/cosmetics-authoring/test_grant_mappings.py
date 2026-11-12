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


class GrantAliases(unittest.TestCase):
    def test_aliases_require_identical_source_looks_and_keep_provenance(self):
        import json
        author = json.loads((c.HERE / 'nx5-authoring.json').read_bytes())
        original = copy.deepcopy(author['document']['outfits'][0])
        alias = copy.deepcopy(original)
        alias['key'] += '.alias'
        author['document']['outfits'].append(alias)
        mapping = next(m for m in author['document']['grant_mappings']
                       if m['target'].get('outfit_key') == original['key'])
        mapping['outfit_aliases'] = [alias['key']]
        def promote():
            template = author['promotions'][0]
            author['promotions'] = [{**template, 'path': path,
                'value': copy.deepcopy(c.pointer(author['document'], path))}
                for path in sorted(c.promotion_paths(author['document']))]
        promote()
        c.validate(author)
        for field in ('id', 'source_record_sha256'):
            before = alias['female'][field]
            alias['female'][field] = 999 if field == 'id' else '1' * 64
            promote()
            with self.assertRaisesRegex(ValueError, 'exact source'):
                c.validate(author)
            alias['female'][field] = before
        mapping['outfit_aliases'] = [original['key']]
        promote()
        with self.assertRaisesRegex(ValueError, 'exact source'):
            c.validate(author)
