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
        document['outfit_aliases'] = {alias['key']: canonical['key']}
        self.promote(fixture.authoring)
        return fixture.authoring, alias, canonical

    def promote(self, authoring):
        source = copy.deepcopy(authoring['promotions'][0])
        paths = c.promotion_paths(authoring['document']) | {'/outfit_aliases'}
        authoring['promotions'] = [
            {**source, 'path': path,
             'value': copy.deepcopy(c.pointer(authoring['document'], path))}
            for path in sorted(paths)]

    def test_explicit_exact_alias_is_promoted_and_materialized(self):
        authoring, alias, canonical = self.fixture()
        packet = c.parse(c.build(authoring, c.ROOT / c.MANIFEST))
        self.assertEqual(packet['authoring']['document']['outfit_aliases'],
                         {alias['key']: canonical['key']})

    def test_unknown_targets_cycles_and_different_looks_are_refused(self):
        for mode in ('unknown', 'cycle', 'look', 'premium', 'addon'):
            with self.subTest(mode=mode):
                authoring, alias, canonical = self.fixture()
                if mode == 'unknown':
                    authoring['document']['outfit_aliases'][alias['key']] = 'outfit:absent'
                elif mode == 'cycle':
                    authoring['document']['outfit_aliases'][canonical['key']] = alias['key']
                elif mode == 'look':
                    alias['female']['id'] += 1
                elif mode == 'premium':
                    alias['premium'], canonical['premium'] = True, False
                else:
                    alias['addon_count'] += 1
                self.promote(authoring)
                with self.assertRaises((ValueError, ValidationError)):
                    c.build(authoring, c.ROOT / c.MANIFEST)


if __name__ == '__main__':
    unittest.main()
