"""Offline negative cases for data completeness without native canonical admission."""
import copy
import json
import unittest
from pathlib import Path
import jsonschema
import reward_claim_variant_migration as tool


class VariantMigration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        if not (cls.root / tool.SOURCE).exists():
            cls.root = Path('/workspace/Oteryn-Game')
        evidence = json.loads(Path(__file__).parent.joinpath('samples/migration/reward-claim-charge-evidence.json').read_text())
        cls.packet = tool.build(cls.root, evidence)

    def test_all_variants_exactly_cover_existing_source_identities(self):
        self.assertEqual(self.packet['source_claim_count'], 336)
        self.assertEqual(self.packet['record_count'], 105)
        self.assertEqual(self.packet['canonical_eligible'], 231)
        self.assertEqual(self.packet['canonical_eligible_missing'], [])
        self.assertTrue(all(not tool.plain_once(r['source_claim']) for r in self.packet['records']))
        self.assertTrue(all(r['native_lowering']['status'] == 'WAITING_IMPLEMENTATION' for r in self.packet['records']))

    def test_container_random_key_text_and_cooldown_preserve_original_shapes(self):
        originals = {tool.identity(c['identity']): c for c in tool.read(self.root, tool.SOURCE)['claims']}
        for record in self.packet['records']:
            self.assertEqual(record['source_claim'], originals[tool.identity(record['source_claim']['identity'])])
        self.assertEqual(sum(r['source_claim']['claim']['repeat']['kind'] == 'cooldown' for r in self.packet['records']), 6)
        self.assertEqual(sum(any('key_binding' in p['reward'] for p in r['source_claim']['placements']) for r in self.packet['records']), 26)

    def test_might_ring_and_ice_rapier_also_preserve_charge_subtypes(self):
        bindings = [b for r in self.packet['records'] for b in r['item_bindings']]
        for item_id, default in ((3048,20),(3284,1)):
            matching = [b for b in bindings if b['source_item']['key'].endswith(f':item/{item_id}') and b['argument']]
            self.assertTrue(matching)
            for binding in matching:
                self.assertEqual(binding['argument']['kind'], 'CHARGES_SUBTYPE')
                self.assertEqual(binding['argument']['source_quantity'], 1)
                self.assertEqual(binding['argument']['source_charges'], binding['argument']['raw_count_argument'])

    def test_draconia_3081_raw_five_is_one_charged_source_object(self):
        record = next(r for r in self.packet['records'] if r['source_claim']['identity']['key'].endswith('/draconia2'))
        charged = next(b for b in record['item_bindings'] if b['source_item']['key'].endswith('/3081'))
        self.assertEqual(charged['argument'], {'raw_count_argument': 5, 'kind': 'CHARGES_SUBTYPE', 'source_quantity': 1, 'source_charges': 5})
        self.assertEqual(record['source_claim']['placements'][0]['reward']['items'][0]['count'], 5)
        self.assertIn('NATIVE_INSTANCE_LOWERING_NOT_IMPLEMENTED', record['native_lowering']['pending_implementation'])
        broken = copy.deepcopy(self.packet)
        target = next(r for r in broken['records'] if r['source_claim']['identity']['key'].endswith('/draconia2'))
        next(b for b in target['item_bindings'] if b['source_item']['key'].endswith('/3081'))['argument']['source_quantity'] = 5
        with self.assertRaises(jsonschema.ValidationError):
            tool.validate(broken, self.root)

    def test_count_revision_and_item_binding_substitution_are_rejected(self):
        broken = copy.deepcopy(self.packet); broken['record_count'] += 1
        with self.assertRaisesRegex(ValueError, 'record_count'):
            tool.validate(broken, self.root)
        broken = copy.deepcopy(self.packet); broken['records'][0]['item_bindings'][0]['canonical_item']['revision'] = 'invented-r2'
        with self.assertRaisesRegex(ValueError, 'revision'):
            tool.validate(broken, self.root)
        broken = copy.deepcopy(self.packet); broken['records'][0]['item_bindings'][0]['canonical_item']['key'] = 'oteryn:item.tibia.i3031'
        with self.assertRaisesRegex(ValueError, 'binding'):
            tool.validate(broken, self.root)

    def test_omitted_item_argument_cannot_hide_behind_valid_source_claim(self):
        broken = copy.deepcopy(self.packet); broken['records'][0]['item_bindings'].pop()
        with self.assertRaisesRegex(ValueError, 'references'):
            tool.validate(broken, self.root)

    def test_unresolved_text_carrier_is_never_claimed_complete(self):
        medusa = next(r for r in self.packet['records'] if r['source_claim']['identity']['key'].endswith('/the_medusa_quest'))
        self.assertEqual(medusa['source_data']['status'], 'WAITING_SOURCE')
        self.assertEqual(medusa['source_data']['missing'][0]['code'], 'SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED')
        broken = copy.deepcopy(self.packet); medusa = next(r for r in broken['records'] if r['source_claim']['identity']['key'].endswith('/the_medusa_quest'))
        medusa['source_data'] = {'status':'COMPLETE','missing':[]}
        with self.assertRaisesRegex(ValueError, 'text carrier'):
            tool.validate(broken, self.root)

    def test_complete_record_cannot_drop_a_canonical_item_binding(self):
        broken = copy.deepcopy(self.packet)
        record = next(r for r in broken['records'] if r['source_data']['status'] == 'COMPLETE')
        record['item_bindings'][0]['canonical_item'] = None
        with self.assertRaisesRegex(ValueError, 'binding'):
            tool.validate(broken, self.root)

    def test_charged_argument_cannot_downgrade_kind_or_drop_native_hold(self):
        for change in ('argument', 'blocker'):
            broken = copy.deepcopy(self.packet)
            record = next(r for r in broken['records'] if r['source_claim']['identity']['key'].endswith('/draconia2'))
            if change == 'argument':
                argument = next(b['argument'] for b in record['item_bindings'] if b['source_item']['key'].endswith('/3081'))
                argument.update(kind='COUNT_OR_SUBTYPE_RETAINED', source_quantity=None, source_charges=None)
            else:
                record['native_lowering']['pending_implementation'].remove('NATIVE_INSTANCE_LOWERING_NOT_IMPLEMENTED')
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'argument kind|blocker inventory'):
                tool.validate(broken, self.root)

    def test_manifest_uid_position_and_source_witness_are_immutable(self):
        for change in ('uid', 'line', 'position'):
            broken = copy.deepcopy(self.packet)
            entry = broken['records'][0]['placement_bindings'][0]['manifest_entries'][0]
            if change == 'uid': entry['sources'][0]['uid'] += 1
            elif change == 'line': entry['sources'][0]['source_lines'][0] += 1
            else: entry['position'][0] += 1
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'UID witnesses'):
                tool.validate(broken, self.root)

    def test_authoring_source_inventory_paths_and_digests_are_immutable(self):
        for change in ('digest', 'path', 'omission'):
            broken = copy.deepcopy(self.packet)
            if change == 'digest': broken['authoring_sources'][0]['sha256'] = '0'*64
            elif change == 'path': broken['authoring_sources'][0]['path'] = 'invented/source.json'
            else: broken['authoring_sources'].pop()
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'authoring_sources'):
                tool.validate(broken, self.root)

    def test_source_refs_and_charge_evidence_cannot_define_their_own_truth(self):
        for change in ('source_refs', 'charge_evidence'):
            broken = copy.deepcopy(self.packet)
            if change == 'source_refs': broken['source_refs'][0]['revision'] = '0'*40
            else: broken['charge_evidence']['charged_items'].pop()
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, change):
                tool.validate(broken, self.root)

    def test_readiness_and_missing_data_inventory_are_source_derived(self):
        broken = copy.deepcopy(self.packet)
        record = next(r for r in broken['records'] if r['source_data']['status'] == 'COMPLETE')
        record['source_data'] = {'status':'WAITING_SOURCE','missing':[{'code':'SOURCE_PLACEMENT_BINDING_MISSING','placement_index':0}]}
        with self.assertRaisesRegex(ValueError, 'source completeness'):
            tool.validate(broken, self.root)
        broken = copy.deepcopy(self.packet)
        broken['canonical_eligible'] += 1
        broken['source_claim_count'] += 1
        with self.assertRaisesRegex(ValueError, 'source coverage'):
            tool.validate(broken, self.root)

    def test_d277_dispositions_keep_data_coverage_separate_from_native_admission(self):
        self.assertEqual(self.packet['architect_rulings'][0]['comment_id'],5933264015)
        self.assertEqual(self.packet['architect_rulings'][0]['classification'],'ACCEPTED_ARCHITECT_RULING')
        self.assertTrue(all(d['data_format']=='COVERED' for r in self.packet['records'] for d in r['authoring_dispositions']))
        self.assertTrue(all('MISSING_CONTRACT' not in x for r in self.packet['records'] for x in r['native_lowering']['pending_implementation']))
        for record in self.packet['records']:
            for disposition in record['authoring_dispositions']:
                if disposition['feature']=='container':self.assertEqual(disposition['data_status'],'WAITING_DATA')
                if disposition['feature'] in ('key_binding','random_one_of','cooldown'):self.assertEqual(disposition['data_status'],'AUTHORED')
        broken=copy.deepcopy(self.packet);broken['records'][0]['native_lowering']['status']='READY'
        with self.assertRaises(jsonschema.ValidationError):tool.validate(broken,self.root)
        broken=copy.deepcopy(self.packet);broken['architect_rulings'][0]['body_sha256']='0'*64
        with self.assertRaisesRegex(ValueError,'architect_rulings'):tool.validate(broken,self.root)

    def test_achievement_typed_grants_join_unique_numeric_source_identity(self):
        record=next(r for r in self.packet['records'] if r['achievement_grants'])
        self.assertEqual(len(record['achievement_grants']),4)
        for grant in record['achievement_grants']:
            self.assertEqual(grant['grant_request'],{'state':'KNOWN','value':{'owner':'Achievement','request':'grant','achievement':{'family':'Achievement','key':'oteryn:achievement/annihilator','revision':'1'}}})
            self.assertEqual(grant['native_status'],'WAITING_ARCHITECTURE')
        unknown=tool.achievement_grant(record['achievement_grants'][0]['source_achievement'],0,[],self.packet['architect_rulings'][0])
        self.assertEqual(unknown['grant_request'],{'state':'UNKNOWN'})
        broken=copy.deepcopy(self.packet);target=next(r for r in broken['records']if r['achievement_grants']);target['achievement_grants'][0]['grant_request']={'state':'UNKNOWN'}
        with self.assertRaisesRegex(ValueError,'achievement_grants'):tool.validate(broken,self.root)

    def test_charge_disposition_does_not_replace_source_mismatch_with_definition_default(self):
        checks=[c for r in self.packet['records']for c in r['charge_dispositions']]
        self.assertEqual(len(checks),9)
        for check in checks:
            self.assertEqual(check['native_status'],'WAITING_IMPLEMENTATION')
            if check['source_raw_argument']==check['definition_charges'].get('value'):
                self.assertEqual(check['normalized_quantity'],{'state':'KNOWN','value':1})
                self.assertEqual(check['data_status'],'AUTHORED')
            else:
                self.assertEqual(check['reason'],'SOURCE_CHARGE_MISMATCH')
                self.assertEqual(check['normalized_quantity'],{'state':'UNKNOWN'})
        self.assertEqual(sum(c['data_status']=='CONFLICT'for c in checks),3)
        broken=copy.deepcopy(self.packet);target=next(r for r in broken['records']if any(c['data_status']=='CONFLICT'for c in r['charge_dispositions']));check=next(c for c in target['charge_dispositions']if c['data_status']=='CONFLICT');check.update(data_status='AUTHORED',reason='DEFINITION_CHARGES_EQUAL_SOURCE_EVIDENCE',normalized_quantity={'state':'KNOWN','value':1})
        with self.assertRaisesRegex(ValueError,'charge_dispositions'):tool.validate(broken,self.root)

    def test_source_schema_missing_native_blocker_and_false_ready_are_rejected(self):
        broken = copy.deepcopy(self.packet); del broken['records'][0]['source_claim']['claim']
        with self.assertRaises(jsonschema.ValidationError):
            tool.validate(broken, self.root)
        broken = copy.deepcopy(self.packet); broken['records'][0]['native_lowering']['pending_implementation'] = []
        with self.assertRaises(jsonschema.ValidationError):
            tool.validate(broken, self.root)
        broken = copy.deepcopy(self.packet); broken['records'][0]['native_lowering']['status'] = 'READY'
        with self.assertRaises(jsonschema.ValidationError):
            tool.validate(broken, self.root)


if __name__ == '__main__':
    unittest.main()
