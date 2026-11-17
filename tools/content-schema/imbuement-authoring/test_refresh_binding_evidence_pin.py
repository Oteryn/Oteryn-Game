import copy
import unittest

from refresh_binding_evidence_pin import verify_reference_only


class BindingPinRefresh(unittest.TestCase):
    def test_only_canonical_definition_digest_may_change(self):
        old = {'material_bindings': {'Demon Horn': {
            'item_ref': {'key': 'oteryn:item.tibia.i5954'},
            'evidence': {'canonical': {'name': 'demon horn', 'definition_sha256': 'a' * 64},
                         'client': {'id': 5954}}}}}
        changed = copy.deepcopy(old)
        changed['material_bindings']['Demon Horn']['evidence']['canonical']['definition_sha256'] = 'b' * 64
        verify_reference_only(old, changed)
        for field in ('identity', 'name', 'client', 'digest'):
            bad = copy.deepcopy(changed)
            row = bad['material_bindings']['Demon Horn']
            if field == 'identity':
                row['item_ref']['key'] = 'oteryn:item.tibia.i5955'
            elif field == 'name':
                row['evidence']['canonical']['name'] = 'different'
            elif field == 'client':
                row['evidence']['client']['id'] = 5955
            else:
                row['evidence']['canonical']['definition_sha256'] = 'invalid'
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify_reference_only(old, bad)

    def test_eligibility_allows_only_the_identity_definition_digest(self):
        old = {'items': [{'key': 'oteryn:item.tibia.i3284', 'allowed_types': ['sword'],
                          'identity_evidence': {'canonical_definition_sha256': 'a' * 64}}]}
        changed = copy.deepcopy(old)
        changed['items'][0]['identity_evidence']['canonical_definition_sha256'] = 'b' * 64
        verify_reference_only(old, changed)
        changed['items'][0]['allowed_types'] = ['axe']
        with self.assertRaises(ValueError):
            verify_reference_only(old, changed)
