"""Explicit source-bound aliases share ownership without duplicating wardrobe entries."""
import copy
import unittest
from jsonschema import ValidationError

import cosmetics_authoring as c
import test_cosmetics_authoring as existing


class OutfitAliases(unittest.TestCase):
    def fixture(self):
        fixture = existing.CosmeticsAuthoring()
        fixture.setUp()
        document = fixture.authoring['document']
        canonical = document['outfits'][0]
        alias = copy.deepcopy(canonical)
        alias['key'] += '/legacy'
        document['outfits'].append(alias)
        document['grant_mappings'] = [{'grant_key':'wardrobe:legacy-base','target':{'kind':'Outfit','outfit_key':alias['key']},'compatible_provenance':['historical-r1'],'outfit_aliases':[canonical['key']]}]
        self.promote(fixture.authoring)
        return fixture.authoring, alias, canonical

    def promote(self, authoring):
        source = copy.deepcopy(authoring['promotions'][0])
        paths = c.promotion_paths(authoring['document'])
        authoring['promotions'] = [
            {**source, 'path': path,
             'value': copy.deepcopy(c.pointer(authoring['document'], path))}
            for path in sorted(paths)]

    def test_explicit_exact_alias_is_promoted_and_materialized(self):
        authoring, alias, canonical = self.fixture()
        packet = c.parse(c.build(authoring, c.ROOT / c.MANIFEST))
        self.assertEqual(packet['authoring']['document']['grant_mappings'][0]['outfit_aliases'],
                         [canonical['key']])

    def test_unknown_targets_cycles_and_different_looks_are_refused(self):
        for mode in ('unknown', 'self', 'look', 'duplicate', 'addon'):
            with self.subTest(mode=mode):
                authoring, alias, canonical = self.fixture()
                if mode == 'unknown':
                    authoring['document']['grant_mappings'][0]['outfit_aliases'] = ['outfit:absent']
                elif mode == 'self':
                    authoring['document']['grant_mappings'][0]['outfit_aliases'] = [alias['key']]
                elif mode == 'look':
                    alias['female']['id'] += 1
                elif mode == 'duplicate':
                    authoring['document']['grant_mappings'][0]['outfit_aliases'] = [canonical['key'],canonical['key']]
                else:
                    canonical['male']['id'] += 1
                self.promote(authoring)
                with self.assertRaises((ValueError, ValidationError)):
                    c.build(authoring, c.ROOT / c.MANIFEST)


if __name__ == '__main__':
    unittest.main()
