"""Task Board content must reach the registered runtime provider, not only a fixture."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import task_board_item_profiles as tool

PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'


def encoded(value):
    return (json.dumps(value, sort_keys=True) + '\n').encode()


def order(record):
    number = int(record['production_binding']['external_id'])
    return number == 40450, number


class TaskBoardItemProfiles(unittest.TestCase):
    def setUp(self):
        self.overlay = json.loads((tool.ROOT / tool.PATH).read_bytes())
        self.native = json.loads((tool.ROOT / PROFILE).read_bytes())
        self.records = {int(r['production_binding']['external_id']): r
                        for r in self.native['records']}

    def assert_known_semantics_preserved(self, before, after, where):
        if before == tool.source.U:
            return
        if isinstance(before, dict):
            self.assertIsInstance(after, dict, where)
            for key, value in before.items():
                self.assertIn(key, after, where)
                self.assert_known_semantics_preserved(value, after[key], where + '.' + key)
        elif isinstance(before, list):
            self.assertIsInstance(after, list, where)
            self.assertEqual(len(before), len(after), where)
            for index, value in enumerate(before):
                self.assert_known_semantics_preserved(value, after[index], where + f'[{index}]')
        else:
            self.assertEqual(before, after, where)

    def test_every_delivery_and_talisman_has_a_registered_portable_native_profile(self):
        ids = tool.cohort_ids()
        self.assertEqual(len(ids), 437)
        self.assertFalse(ids - self.records.keys(), 'missing production native policies')
        for item in sorted(ids):
            record = self.records[item]
            self.assertTrue(record['admission']['materializable'], item)
            self.assertIn('CharacterInventory', record['admission']['legal_destinations'], item)
            self.assertEqual(record['semantics']['container'], tool.source.NA, item)
            self.assertEqual(record['semantics']['temporal'], tool.source.NA, item)
            self.assertEqual(record['production_definition'], {
                'family': 'Item', 'production_key': f'oteryn:item.tibia.i{item}',
                'revision_ref': 'definition-r1'}, item)
        self.assertNotIn(31464, {int(r['production_binding']['external_id'])
                                  for r in self.overlay['records']})
        self.assertNotIn(31465, {int(r['production_binding']['external_id'])
                                  for r in self.overlay['records']})

    def test_talisman_uses_source_extra_slot_and_current_charge_trade_facts_survive(self):
        talisman = self.records[tool.TALISMAN]
        tool.validate_record(talisman)
        self.assertEqual(talisman['semantics']['physical']['value']['weight'], tool.source.K(75))
        self.assertEqual(talisman['semantics']['charges'], tool.source.NA)
        for item in (21178, 21179, 21180):
            self.assertEqual(self.records[item]['semantics']['charges'],
                             tool.source.K({'count': tool.source.K(2)}))
        for item in (10310, 37109):
            self.assertEqual(self.records[item]['semantics']['trade_restrictions']
                             ['value']['marketable'], tool.source.K(True))

    def test_grappling_hook_uses_only_the_approved_portable_source_id(self):
        delivery = tool.delivery_items()
        self.assertIn((35588, 50), delivery)
        self.assertNotIn(5800, {item for item, _ in delivery})
        self.assertEqual(self.overlay['delivery_corrections'], tool.DELIVERY_CORRECTIONS)
        self.assertNotIn(5800, {int(r['production_binding']['external_id'])
                               for r in self.overlay['records']})
        tool.validate_record(self.records[35588])
        original = [(5800 if item == 35588 else item, amount) for item, amount in delivery]
        self.assertEqual(tool.qualify_delivery_rows(original), delivery)
        with self.assertRaisesRegex(ValueError, 'CORRECTION_SOURCE_DRIFT'):
            tool.qualify_delivery_rows(delivery)

    def test_worm_sponge_keeps_its_exact_accepted_binding_revision(self):
        worm = self.records[43848]
        self.assertEqual(worm['production_binding'], tool.WORM_BINDING)
        self.assertIsNone(worm['production_binding_qualification'])
        self.assertEqual(self.overlay['binding_revision_qualifications'],
                         tool.BINDING_REVISION_QUALIFICATIONS)
        tool.validate_record(worm)
        bindings = tool.binding_index(tool.BINDINGS.read_bytes())
        with self.assertRaisesRegex(ValueError, 'independently qualified source revision'):
            tool.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                'records': [worm]}, bindings, {43848})
        rebound = tool.bind_qualified_cohort([worm], bindings, {43848})
        self.assertEqual(rebound, [worm])
        forged = copy.deepcopy(worm)
        forged['production_binding']['source_revision'] = tool.PIN
        with self.assertRaisesRegex(ValueError, 'TASK_BOARD_WORM_BINDING_CONFLICT'):
            tool.bind_qualified_cohort([forged], bindings, {43848})
        with self.assertRaisesRegex(ValueError, 'TASK_BOARD_WORM_BINDING_QUALIFICATION'):
            tool.bind_qualified_cohort([worm], bindings, set())

    def test_registration_preserves_known_existing_facts_and_is_idempotent(self):
        before = copy.deepcopy(self.native)
        existing = next(r for r in before['records'] if r['production_binding']['external_id'] == '3364')
        # Deliberately different accepted facts prove the source cannot overwrite them.
        existing['attributes']['speed_bonus'] = 7
        existing['semantics']['physical']['value']['weight'] = tool.source.K(123)
        leaf = next(r for r in before['records'] if r['production_binding']['external_id'] == '10310')
        # Reproduce the actual pre-Task-Board native policy even after regeneration.
        leaf['semantics']['container'] = copy.deepcopy(tool.source.U)
        leaf['semantics']['temporal'] = copy.deepcopy(tool.source.U)
        generated = {PROFILE: encoded(before), MANIFEST: b'{"item_profiles":{}}'}
        tool.admit(tool.ROOT, generated, PROFILE, MANIFEST, order, encoded)
        after = json.loads(generated[PROFILE])
        original = {r['production_definition']['production_key']: r for r in before['records']}
        for row in after['records']:
            key = row['production_definition']['production_key']
            if key not in original:
                continue
            if int(row['production_binding']['external_id']) not in tool.EXISTING_NATIVE_IDS:
                self.assertEqual(row, original[key], key)
            else:
                kept = copy.deepcopy(row)
                kept['semantics'] = original[key]['semantics']
                self.assertEqual(kept, original[key], key)
                self.assert_known_semantics_preserved(original[key]['semantics'], row['semantics'], key)
        after_by_id = {int(r['production_binding']['external_id']): r for r in after['records']}
        self.assertEqual(after_by_id[10310]['semantics']['container'], tool.source.NA)
        self.assertEqual(after_by_id[10310]['semantics']['temporal'], tool.source.NA)
        self.assertEqual(after_by_id[3364]['semantics']['physical']['value']['weight'], tool.source.K(123))
        self.assertEqual(after_by_id[3364]['attributes']['speed_bonus'], 7)
        self.assertEqual(json.loads(generated[MANIFEST])['item_profiles']['sha256'],
                         tool.digest(generated[PROFILE]))
        once = copy.deepcopy(generated)
        tool.admit(tool.ROOT, generated, PROFILE, MANIFEST, order, encoded)
        self.assertEqual(generated, once)

    def test_missing_cohort_member_or_stale_canonical_proof_changes_no_outputs(self):
        for failure in ('member', 'canonical'):
            overlay = copy.deepcopy(self.overlay)
            if failure == 'member':
                overlay['records'].pop()
            else:
                overlay['canonical_inputs'][0]['definition_sha256'] = '0' * 64
            with tempfile.TemporaryDirectory() as temporary:
                packet = Path(temporary) / 'overlay.json'
                packet.write_bytes(encoded(overlay))
                generated = {PROFILE: encoded(self.native), MANIFEST: b'{"item_profiles":{}}'}
                before = copy.deepcopy(generated)
                with patch.object(tool, 'PATH', str(packet)):
                    with self.assertRaisesRegex(ValueError, 'TASK_BOARD_(PROFILE_SET|CANONICAL_DRIFT)'):
                        tool.admit(tool.ROOT, generated, PROFILE, MANIFEST, order, encoded)
                self.assertEqual(generated, before)


if __name__ == '__main__':
    unittest.main()
