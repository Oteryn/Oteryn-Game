import copy
import json
import unittest
import questpc5_parchment_profile as m


class ParchmentCustody(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.observed = m.shared.capture(ids={2833})['records'][0]
        cls.old = copy.deepcopy(cls.observed)
        cls.old['admission'] = dict(materializable=False, stack_class='NonStackable', legal_destinations=['Ground'])
        cls.old['semantics']['readable_writeable']['value']['writeable'] = {'state': 'UNKNOWN'}

    def test_exact_portable_source_facts_promote_without_inventing_write_target(self):
        new = m.promote(self.old, self.observed)
        self.assertTrue(new['admission']['materializable'])
        self.assertIn('CharacterInventory', new['admission']['legal_destinations'])
        self.assertEqual(new['semantics']['readable_writeable']['value']['write_once_target'], {'state': 'UNKNOWN'})
        self.assertFalse(self.old['admission']['materializable'])
        self.assertEqual(m.promote(new, self.observed), new)

    def test_repository_profile_preserves_existing_native_owner_facts(self):
        rows = json.loads((m.shared.ROOT / m.shared.PROFILE).read_bytes())['records']
        old = next(r for r in rows if r['production_definition']['production_key'] == m.KEY)
        new = m.promote(old, self.observed)
        self.assertEqual(old['production_binding'], new['production_binding'])
        self.assertFalse(m.shared.accepted.known_conflict(old['semantics'], new['semantics']))

    def test_other_identity_or_admission_or_conflicting_fact_is_rejected(self):
        for field in ('identity', 'admission', 'weight'):
            old = copy.deepcopy(self.old)
            if field == 'identity':
                old['production_definition']['production_key'] = 'oteryn:item.tibia.i2834'
            elif field == 'admission':
                old['admission']['materializable'] = True
            else:
                old['semantics']['physical']['value']['weight']['value'] += 1
            with self.assertRaises(ValueError, msg=field):
                m.promote(old, self.observed)


if __name__ == '__main__':
    unittest.main()
