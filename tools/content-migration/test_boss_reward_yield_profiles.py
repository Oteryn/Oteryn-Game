"""Source custody completion cannot overwrite existing equipment or known facts."""
import copy
import hashlib
import json
import unittest
from unittest.mock import patch
import boss_reward_yield_profiles as yields


class ExistingRottenYieldCustodyTests(unittest.TestCase):
    def native(self):
        raw = (yields.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes()
        document = json.loads(raw)
        updates = json.loads((yields.ROOT/'content/items/definitions/srx1-item-updates.json').read_bytes())
        self.assertEqual(updates['base_sha256'], hashlib.sha256(raw).hexdigest())
        by_key = {row['production_definition']['production_key']: row
                  for row in document['records']}
        for row in updates['records']:
            key = row['production_definition']['production_key']
            if key in by_key:
                self.assertEqual(by_key[key]['production_definition'], row['production_definition'])
                document['records'][document['records'].index(by_key[key])] = row
            else:
                document['records'].append(row)
            by_key[key] = row
        return document

    def test_known_ham_food_or_market_fact_cannot_be_replaced(self):
        for part in ['consumption', 'trade_restrictions']:
            document = self.native()
            row = next(r for r in document['records'] if r['production_binding']['external_id'] == '3583')
            if part == 'consumption':
                row['semantics'][part] = {'state': 'KNOWN', 'value': {'kind': 'FOOD', 'value': {'regeneration_seconds': 360}}}
            else:
                captured = json.loads((yields.ROOT/yields.PATH).read_bytes())
                proven = next(r for r in captured['records'] if r['production_binding']['external_id'] == '3583')
                row['semantics'][part] = copy.deepcopy(proven['semantics'][part])
                row['semantics'][part]['value']['marketable']['value'] = False
            with self.assertRaisesRegex(ValueError, 'BOSS_REWARD_HAM_KNOWN_CUSTODY_CONFLICT'):
                self.apply(document)

    def apply(self, document):
        generated={'profiles':yields.corpses.encode(document),
                   'manifest':yields.corpses.encode({'item_profiles':{'sha256':'0'*64}})}
        yields.admit(yields.ROOT,generated,'profiles','manifest',
                     lambda row:row['production_definition']['production_key'],yields.corpses.encode)
        return json.loads(generated['profiles'])

    def test_actual_grand_mother_loot_has_complete_carried_native_profiles(self):
        native={row['production_definition']['production_key']:row for row in self.native()['records']}
        selected=[]
        for path in sorted((yields.ROOT/'content/loot').glob('loot-*.json')):
            for row in json.loads(path.read_bytes())['records']:
                definition=row['definition']
                if definition['identity']['key']=='oteryn:loot.creature.grand_mother_foulscale':
                    selected=definition['entries']
        self.assertEqual(len(selected),19)
        for entry in selected:
            key=entry['item']['key']
            self.assertTrue(key in native,'actual rewardBoss loot needs a native carried profile: '+key)
            policy=native[key]['admission']
            self.assertTrue(policy['materializable'],key)
            self.assertIn('CharacterInventory',policy['legal_destinations'],key)

    def test_actual_ham_custody_keeps_existing_food_and_market_rule(self):
        row=next((row for row in self.native()['records']
                  if row['production_definition']['production_key']=='oteryn:item.tibia.i3583'),None)
        self.assertIsNotNone(row,'actual reward dragon ham needs its carried native policy')
        self.assertEqual(row['semantics']['consumption'],
            {'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':720}}})
        self.assertEqual(row['semantics']['trade_restrictions']['value']['marketable'],
            {'state':'KNOWN','value':True})

    def test_actual_existing_yields_keep_equipment_rune_and_fluid_semantics(self):
        before=self.native()
        after=self.apply(copy.deepcopy(before))
        actual={row['production_definition']['production_key']:row for row in after['records']}
        mutable={'classification','stack','container','charges','temporal'}
        for row in before['records']:
            if int(row['production_binding']['external_id']) not in yields.CLASSIFICATION_ENRICHMENT_IDS:
                continue
            result=actual[row['production_definition']['production_key']]
            for key,value in row['semantics'].items():
                if key not in mutable:
                    self.assertEqual(result['semantics'][key],value,key)
            self.assertEqual(result['attributes'],row['attributes'])

    def test_exact_timed_owner_keeps_known_amulet_and_ring_semantics(self):
        import timed_item_profiles
        generated={'profiles':b'{"records":[]}', 'manifest':b'{"item_profiles":{}}'}
        timed_item_profiles.admit(yields.ROOT,generated,'profiles','manifest',
            lambda row:row['production_definition']['production_key'],yields.corpses.encode)
        before=json.loads(generated['profiles'])
        after=self.apply(copy.deepcopy(before))
        actual={row['production_definition']['production_key']:row for row in after['records']}
        for row in before['records']:
            if row['production_binding']['external_id'] in ('3081','3097'):
                updated=actual[row['production_definition']['production_key']]
                self.assertEqual(updated['authoring'],row['authoring'])
                self.assertEqual(updated['semantics']['equipment'],row['semantics']['equipment'])
                self.assertEqual(updated['semantics']['skill_modifiers'],row['semantics']['skill_modifiers'])
        conflicting=copy.deepcopy(before)
        ring=next(row for row in conflicting['records'] if row['production_binding']['external_id']=='3081')
        ring['semantics']['physical']['value']['weight']['value']+=1
        with self.assertRaisesRegex(ValueError,'BOSS_REWARD_YIELD_PROFILE_CONFLICT'):
            self.apply(conflicting)

    def test_actual_existing_known_charge_cannot_be_replaced(self):
        document=self.native()
        target=next(row for row in document['records'] if row['production_binding']['external_id']=='3081')
        target['semantics']['charges']={'state':'KNOWN','value':{'count':{'state':'KNOWN','value':777}}}
        with self.assertRaisesRegex(ValueError,'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT'):
            self.apply(document)


class SharedCrimsonSwordCustodyTests(unittest.TestCase):
    KEY = 'oteryn:item.tibia.i7385'
    RETAINED = 'oteryn:item.tibia.i112'

    @classmethod
    def setUpClass(cls):
        # Retain actual native identity/attributes without constructing the full
        # bank in memory or invoking the complete registration pipeline.
        raw = (yields.ROOT/'content/items/definitions/spell-native-profiles.json').read_text()
        position = raw.index('[', raw.index('"records"')) + 1
        decoder = json.JSONDecoder()
        selected = []
        while True:
            while raw[position].isspace() or raw[position] == ',':
                position += 1
            if raw[position] == ']':
                break
            row, position = decoder.raw_decode(raw, position)
            if row['production_definition']['production_key'] in (cls.KEY, cls.RETAINED):
                selected.append(row)
        if len(selected) != 2 or {r['production_definition']['production_key'] for r in selected} != {cls.KEY, cls.RETAINED}:
            raise ValueError('exact existing sword and unrelated native row required')
        definitions = [row['definition']
                       for path in sorted((yields.ROOT/'content/items/definitions').glob('items-*.json'))
                       for row in json.loads(path.read_bytes())['records']
                       if row['definition']['identity']['key'] == cls.KEY]
        if len(definitions) != 1:
            raise ValueError('exact canonical Crimson Sword required')
        cls.definition = definitions[0]
        # The earlier callback owner constructs semantics from this accepted
        # definition. Rebuild that boundary even after the final provider has
        # been regenerated with the later boss custody completion.
        import quest_callback_profiles as callbacks
        prior = next(row for row in selected if row['production_definition']['production_key'] == cls.KEY)
        prior['semantics'] = callbacks.grant_semantics(yields.ROOT, cls.definition)
        cls.before = {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': selected}
        cls.capture = json.loads((yields.ROOT/yields.PATH).read_bytes())
        cls.manifest = json.loads((yields.ROOT/'content/spells.manifest.json').read_bytes())

    def generated(self, document):
        raw = yields.corpses.encode(document)
        manifest = copy.deepcopy(self.manifest)
        manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
        return {'profiles': raw, 'manifest': yields.corpses.encode(manifest)}

    def admit(self, generated):
        yields.admit(yields.ROOT, generated, 'profiles', 'manifest',
                     lambda row: row['production_definition']['production_key'], yields.corpses.encode)

    def refuse(self, document, reason):
        generated = self.generated(document)
        before = copy.deepcopy(generated)
        with self.assertRaisesRegex(ValueError, reason):
            self.admit(generated)
        self.assertEqual(generated, before, 'refusal must leave every output byte unchanged')

    def test_actual_callback_owner_then_yield_retains_all_other_facts(self):
        import quest_callback_profiles as callbacks
        authored = json.loads((yields.ROOT/'content/quests/chosen-reward-callbacks.json').read_bytes())
        selected = {row['key']: row for row in authored['callbacks']}
        self.assertEqual(selected['oteryn:quest-reward/chosen/trader/buy-sword']['give'],
                         {'item': self.KEY, 'count': 1, 'contents': []})
        self.assertEqual(selected['oteryn:quest-reward/chosen/trader/sword']['take'],
                         [{'item': self.KEY, 'count': 1}])
        document = copy.deepcopy(self.before)
        prior = next(row for row in document['records'] if row['production_definition']['production_key'] == self.KEY)
        before = copy.deepcopy(prior)
        self.assertEqual(prior['authoring']['item'], self.definition['identity'])
        self.assertEqual(prior['semantics'], callbacks.grant_semantics(yields.ROOT, self.definition))
        self.assertEqual(prior['semantics']['stack']['value']['stack_max'], {'state': 'UNKNOWN'})
        for group in ('container', 'charges', 'temporal'):
            self.assertEqual(prior['semantics'][group], {'state': 'UNKNOWN'})
        callbacks.complete_native_stack(yields.ROOT, prior, self.definition, prior['production_binding'])
        self.assertEqual(prior, before, 'the actual earlier callback owner keeps its accepted facts')
        generated = self.generated(document)
        self.admit(generated)
        after = json.loads(generated['profiles'])
        actual = {row['production_definition']['production_key']: row for row in after['records']}
        self.assertEqual(len(after['records']), len(yields.IDS) + 1)
        expected = copy.deepcopy(before)
        proven = next(row for row in self.capture['records'] if row['production_definition']['production_key'] == self.KEY)
        for group in ('classification', 'stack', 'container', 'charges', 'temporal'):
            expected['semantics'][group] = copy.deepcopy(proven['semantics'][group])
        self.assertEqual(actual[self.KEY], expected)
        self.assertEqual(actual[self.KEY]['semantics']['stack']['value']['stack_max'], {'state': 'NOT_APPLICABLE'})
        for group in ('container', 'charges', 'temporal'):
            self.assertEqual(actual[self.KEY]['semantics'][group], {'state': 'NOT_APPLICABLE'})
        self.assertEqual(actual[self.RETAINED], next(row for row in document['records']
                         if row['production_definition']['production_key'] == self.RETAINED))
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(generated['profiles']).hexdigest())

    def test_known_custody_and_physical_conflicts_refuse_without_writes(self):
        known = lambda value: {'state': 'KNOWN', 'value': value}
        mutations = {
            'stack': known({'stackable': known(False), 'stack_max': known(1)}),
            'container': known({'capacity': known(8)}),
            'charges': known({'count': known(1)}),
            'temporal': known({'duration': known(60000), 'decay_target': {'state': 'NOT_APPLICABLE'},
                               'stop_duration': known(False), 'consumption_mode': known('DURABLE_ABSOLUTE_DEADLINE')}),
        }
        for group, fact in mutations.items():
            with self.subTest(group=group):
                document = copy.deepcopy(self.before)
                row = next(row for row in document['records'] if row['production_definition']['production_key'] == self.KEY)
                row['semantics'][group] = fact
                self.refuse(document, 'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT:' + self.KEY)
        document = copy.deepcopy(self.before)
        row = next(row for row in document['records'] if row['production_definition']['production_key'] == self.KEY)
        row['semantics']['physical']['value']['weight']['value'] += 1
        self.refuse(document, 'BOSS_REWARD_YIELD_PROFILE_CONFLICT:' + self.KEY)

    def test_identity_binding_and_admission_mismatches_remain_refused(self):
        for part in ('definition', 'binding', 'admission'):
            with self.subTest(part=part):
                document = copy.deepcopy(self.before)
                row = next(row for row in document['records'] if row['production_definition']['production_key'] == self.KEY)
                if part == 'definition':
                    row['production_definition']['revision_ref'] = 'definition-r2'
                elif part == 'binding':
                    row['production_binding']['source_revision'] = '0' * 40
                else:
                    row['admission']['materializable'] = False
                self.refuse(document, 'BOSS_REWARD_YIELD_PROFILE_CONFLICT:' + self.KEY)

    def test_other_yield_does_not_gain_unknown_custody_completion(self):
        row = copy.deepcopy(next(row for row in self.capture['records']
                                 if row['production_binding']['external_id'] == '7416'))
        row['semantics']['charges'] = {'state': 'UNKNOWN'}
        self.refuse({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]},
                    'BOSS_REWARD_YIELD_PROFILE_CONFLICT:oteryn:item.tibia.i7416')

    def test_changed_source_capture_still_refuses_before_any_output(self):
        from pathlib import Path
        from unittest.mock import patch
        capture = copy.deepcopy(self.capture)
        row = next(row for row in capture['records'] if row['production_definition']['production_key'] == self.KEY)
        row['semantics']['physical']['value']['weight']['value'] += 1
        changed = yields.corpses.encode(capture)
        original = Path.read_bytes

        def read_bytes(path):
            return changed if path == yields.ROOT/yields.PATH else original(path)

        with patch.object(Path, 'read_bytes', read_bytes):
            self.refuse(self.before, 'BOSS_REWARD_YIELD_CAPTURE_DRIFT')


class ActualOutburstRewardYieldTests(unittest.TestCase):
    def test_actual_outburst_personal_reward_has_complete_carried_yields(self):
        native=json.loads((yields.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual={row['production_definition']['production_key']:row for row in native['records']}
        for item in (23474,7428,23477):
            key='oteryn:item.tibia.i'+str(item)
            self.assertTrue(key in actual,'actual rewardBoss Outburst Item policy missing: '+key)
            self.assertTrue(actual[key]['admission']['materializable'])
            self.assertIn('CharacterInventory',actual[key]['admission']['legal_destinations'])


class ActualTraderRewardYieldTests(unittest.TestCase):
    KEY = 'oteryn:item.tibia.i7385'

    @classmethod
    def setUpClass(cls):
        import authored_static_item_constructors as authored
        import lion_trial_item_profiles as lion
        import quest_callback_profiles as owner
        from qualification_inputs import appearance_input_scope

        keys, authored_keys, callbacks = owner._requested_inputs(yields.ROOT)
        selected = {cls.KEY} | {f'oteryn:item.tibia.i{i}' for i in lion.SOURCE_RECORDS}
        if not selected <= keys:
            raise AssertionError('actual trader and Lion callback inputs are required')
        cls.callbacks = [callback for callback in callbacks
                         if cls.KEY in json.dumps(callback)]
        generated = {
            'profiles': b'{"records":[]}',
            'manifest': b'{"item_profiles":{}}',
            authored.ADMISSIONS: (yields.ROOT / authored.ADMISSIONS).read_bytes(),
        }
        # Limit the real requested cohort, retaining all selected source facts
        # and the four Lion records required by this existing owner.
        with appearance_input_scope(), patch.object(
                owner, '_requested_inputs',
                return_value=(keys & selected, authored_keys & selected, callbacks)):
            owner.admit(yields.ROOT, generated, 'profiles', 'manifest',
                        lambda row: row['production_definition']['production_key'],
                        yields.corpses.encode)
        cls.before = json.loads(generated['profiles'])
        cls.source = json.loads((yields.ROOT / yields.PATH).read_bytes())

    def generated(self, document):
        return {'profiles': yields.corpses.encode(document),
                'manifest': b'{"item_profiles":{}}'}

    def admit(self, generated):
        yields.admit(yields.ROOT, generated, 'profiles', 'manifest',
                     lambda row: row['production_definition']['production_key'],
                     yields.corpses.encode)

    def test_actual_trader_callback_yield_retains_known_facts_and_replays(self):
        self.assertEqual({row['key'] for row in self.callbacks}, {
            'oteryn:quest-reward/chosen/trader/buy-sword',
            'oteryn:quest-reward/chosen/trader/sword',
        })
        previous = {row['production_definition']['production_key']: row
                    for row in self.before['records']}
        before = previous[self.KEY]
        self.assertEqual(before['semantics']['stack']['value']['stack_max'],
                         {'state': 'UNKNOWN'})
        for group in ('container', 'charges', 'temporal'):
            self.assertEqual(before['semantics'][group], {'state': 'UNKNOWN'})
        generated = self.generated(self.before)
        self.admit(generated)
        actual = {row['production_definition']['production_key']: row
                  for row in json.loads(generated['profiles'])['records']}
        source = {row['production_definition']['production_key']: row
                  for row in self.source['records']}
        self.assertEqual(set(actual), set(previous) | set(source))
        expected = copy.deepcopy(before)
        for group in ('classification', 'stack', 'container', 'charges', 'temporal'):
            expected['semantics'][group] = copy.deepcopy(source[self.KEY]['semantics'][group])
        self.assertEqual(actual[self.KEY], expected)
        for key in previous.keys() - {self.KEY}:
            self.assertEqual(actual[key], previous[key])
        self.assertEqual(actual[self.KEY]['semantics']['imbuement']['value']['slot_count'],
                         {'state': 'KNOWN', 'value': 2})
        self.assertEqual(actual[self.KEY]['semantics']['trade_restrictions']['value']['marketable'],
                         {'state': 'KNOWN', 'value': True})
        from build_full_spell_artifact import native_loot_yield_facts
        self.assertIsNotNone(native_loot_yield_facts(
            source[self.KEY]['production_binding']['target'], actual[self.KEY]))
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(generated['profiles']).hexdigest())
        replay = dict(generated)
        self.admit(generated)
        self.assertEqual(generated, replay)

    def test_known_trader_custody_or_authority_conflicts_leave_outputs_unchanged(self):
        known = lambda value: {'state': 'KNOWN', 'value': value}
        cases = [
            (('semantics', 'stack', 'value', 'stack_max'), known(1),
             'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT'),
            (('semantics', 'stack', 'value', 'stackable'), known(True),
             'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT'),
            (('semantics', 'container'), known({'capacity': known(1)}),
             'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT'),
            (('semantics', 'charges'), known({'count': known(1)}),
             'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT'),
            (('semantics', 'temporal'), known({'duration': known(1000)}),
             'BOSS_REWARD_KNOWN_CUSTODY_CONFLICT'),
            (('semantics', 'physical', 'value', 'weight'), known(3601),
             'BOSS_REWARD_YIELD_PROFILE_CONFLICT'),
            (('production_definition', 'revision_ref'), 'definition-r2',
             'BOSS_REWARD_YIELD_PROFILE_CONFLICT'),
            (('production_binding', 'target', 'revision'), 'definition-r2',
             'BOSS_REWARD_YIELD_PROFILE_CONFLICT'),
            (('admission', 'materializable'), False,
             'BOSS_REWARD_YIELD_PROFILE_CONFLICT'),
            (('admission', 'legal_destinations'), ['Ground'],
             'BOSS_REWARD_YIELD_PROFILE_CONFLICT'),
        ]
        for path, value, error in cases:
            with self.subTest(path=path):
                document = copy.deepcopy(self.before)
                row = next(row for row in document['records']
                           if row['production_definition']['production_key'] == self.KEY)
                target = row
                for part in path[:-1]:
                    target = target[part]
                target[path[-1]] = value
                generated = self.generated(document)
                before = dict(generated)
                with self.assertRaisesRegex(ValueError, error + ':' + self.KEY):
                    self.admit(generated)
                self.assertEqual(generated, before)

    def test_other_yield_unknown_custody_does_not_acquire_trader_enrichment(self):
        key = 'oteryn:item.tibia.i7416'
        row = copy.deepcopy(next(row for row in self.source['records']
                                 if row['production_definition']['production_key'] == key))
        row['semantics']['charges'] = {'state': 'UNKNOWN'}
        generated = self.generated({'records': [row]})
        before = dict(generated)
        with self.assertRaisesRegex(ValueError, 'BOSS_REWARD_YIELD_PROFILE_CONFLICT:' + key):
            self.admit(generated)
        self.assertEqual(generated, before)


if __name__=='__main__':unittest.main()
