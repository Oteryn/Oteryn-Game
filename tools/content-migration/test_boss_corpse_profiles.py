"""Exact sharing of the two pre-existing gathering/corpse Item profiles."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import boss_corpse_profiles as corpses


class SharedSourceCorpseTests(unittest.TestCase):
    def apply(self, rows):
        generated={'profiles':corpses.encode({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':rows}),
                   'manifest':corpses.encode({'item_profiles':{'sha256':'0'*64}})}
        corpses.admit(corpses.ROOT,generated,'profiles','manifest',
            lambda row:row['production_definition']['production_key'],corpses.encode)
        return json.loads(generated['profiles'])

    def shared(self):
        capture=json.loads((corpses.ROOT/'tools/content-schema/native-gameplay/gathering-item-profiles.json').read_bytes())
        return [row for row in capture['records'] if row['production_binding']['external_id'] in ('5973','4026')]

    def test_actual_source_gathering_profiles_keep_their_existing_authoring(self):
        shared=self.shared();self.assertEqual(len(shared),2)
        result=self.apply(shared)
        self.assertEqual(len(result['records']),199)
        for expected in shared:
            self.assertIn(expected,result['records'])

    def test_existing_player_body_and_greedy_eye_records_are_preserved(self):
        native=json.loads((corpses.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        wanted={'4240','4241','4242','4246','4247','4248','5995','4098'}
        shared=[row for row in native['records'] if row['production_binding']['external_id'] in wanted]
        self.assertEqual(len(shared),8)
        result=self.apply(copy.deepcopy(shared))
        for expected in shared:
            self.assertIn(expected,result['records'])
        conflicting=copy.deepcopy(shared)
        conflicting[0]['attributes']['spell_augments']=[{'kind':'UNREVIEWED'}]
        with self.assertRaisesRegex(ValueError,'BOSS_CORPSE_PROFILE_CONFLICT'):
            self.apply(conflicting)
        conflicting=copy.deepcopy(shared)
        conflicting[0]['attributes']['always_on_top_order'] = 3
        conflicting[0]['attributes']['always_on_top'] = True
        with self.assertRaisesRegex(ValueError,'BOSS_CORPSE_PROFILE_CONFLICT'):
            self.apply(conflicting)

    def test_shared_source_profile_cannot_splice_other_effective_metadata(self):
        shared=copy.deepcopy(self.shared())
        shared[0]['attributes']['speed_bonus']=1
        with self.assertRaisesRegex(ValueError,'BOSS_CORPSE_PROFILE_CONFLICT'):
            self.apply(shared)


class NativeTerminalCompositionTests(unittest.TestCase):
    def generated(self, rows=()):
        return {'profiles': corpses.encode({'records': list(rows)}),
                'manifest': corpses.encode({'item_profiles': {}})}

    def admit(self, generated):
        corpses.admit(corpses.ROOT, generated, 'profiles', 'manifest',
                      lambda row: row['production_definition']['production_key'], corpses.encode)

    def refuse(self, replacements, expected):
        generated = self.generated()
        before = copy.deepcopy(generated)
        read_bytes = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda path: replacements[path]
                          if path in replacements else read_bytes(path)):
            with self.assertRaisesRegex(ValueError, expected):
                self.admit(generated)
        self.assertEqual(generated, before)

    def test_native_terminal_changes_only_container_and_preserves_raw_capture(self):
        path = corpses.ROOT / corpses.PATH
        raw = path.read_bytes()
        expected = json.loads(raw)['records']
        terminal = next(row for row in expected if row['production_binding']['external_id'] == '4027')
        self.assertEqual(terminal['semantics']['container'],
                         {'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': 8}}})
        terminal['semantics']['container'] = {'state': 'NOT_APPLICABLE'}
        generated = self.generated()
        self.admit(generated)
        actual = json.loads(generated['profiles'])['records']
        self.assertEqual(len(actual), 199)
        self.assertEqual(actual, sorted(expected, key=lambda row: row['production_definition']['production_key']))
        self.assertEqual(path.read_bytes(), raw)
        # A previously composed native terminal is accepted exactly on replay.
        replay = self.generated([terminal])
        self.admit(replay)
        self.assertEqual(replay, generated)
        rejected = copy.deepcopy(terminal)
        rejected['semantics']['container']['state'] = 'UNKNOWN'
        refused = self.generated([rejected])
        before = copy.deepcopy(refused)
        with self.assertRaisesRegex(ValueError, 'BOSS_CORPSE_PROFILE_CONFLICT'):
            self.admit(refused)
        self.assertEqual(refused, before)

    def test_rehashed_source_capacity_metadata_and_identity_tampering_is_atomic(self):
        source = json.loads((corpses.ROOT / corpses.PATH).read_bytes())
        for mutation in ('capacity', 'speed', 'duration', 'binding'):
            changed = copy.deepcopy(source)
            terminal = next(row for row in changed['records']
                            if row['production_binding']['external_id'] == '4027')
            if mutation == 'capacity':
                terminal['semantics']['container']['value']['capacity']['value'] = 9
            elif mutation == 'speed':
                terminal['attributes']['speed_bonus'] = 1
            elif mutation == 'duration':
                terminal['semantics']['temporal']['value']['duration']['value'] = 60001
            else:
                terminal['production_binding']['target']['revision'] = 'definition-r2'
            raw = corpses.encode(changed)
            replacements = {corpses.ROOT / corpses.PATH: raw,
                            corpses.ROOT / corpses.PIN_PATH: corpses.encode(
                                {'path': corpses.PATH, 'sha256': hashlib.sha256(raw).hexdigest()})}
            with self.subTest(mutation=mutation):
                self.refuse(replacements, 'BOSS_CORPSE_(TERMINAL_SOURCE_BODY|NATIVE_AUTHORITY)')

    def test_canonical_body_identity_and_duplicate_tampering_is_atomic(self):
        path = corpses.ROOT / 'content/items/definitions/items-24500-24999.json'
        original = json.loads(path.read_bytes())
        for mutation in ('capacity', 'movable', 'duration', 'identity', 'client', 'admission',
                         'presentation', 'duplicate'):
            changed = copy.deepcopy(original)
            terminal = next(row['definition'] for row in changed['records']
                            if row['definition']['identity']['key'] == 'oteryn:item.tibia.i4027')
            if mutation == 'capacity':
                terminal['semantics']['container'] = {
                    'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': 8}}}
            elif mutation == 'movable':
                terminal['semantics']['physical']['value']['movable']['value'] = False
            elif mutation == 'duration':
                terminal['semantics']['temporal']['value']['duration']['value'] = 60001
            elif mutation == 'identity':
                terminal['identity']['revision'] = 'definition-r2'
            elif mutation == 'client':
                terminal['client_projection'] = 'Unknown'
            elif mutation == 'admission':
                terminal['materializable'] = False
            elif mutation == 'presentation':
                terminal['semantics']['presentation'] = {'state': 'NOT_APPLICABLE'}
            else:
                other = next(i for i, row in enumerate(changed['records'])
                             if row['definition']['identity']['key'] != terminal['identity']['key'])
                changed['records'][other] = {'definition': copy.deepcopy(terminal)}
            with self.subTest(mutation=mutation):
                self.refuse({path: corpses.encode(changed)}, 'BOSS_CORPSE_TERMINAL_CANONICAL_BODY')

    def test_native_terminal_requires_the_pinned_source_chain(self):
        import encounter_corpse_items as encounter
        self.refuse({corpses.ROOT / encounter.XML: b'<items/>'}, 'source XML differs')
        policies = encounter.source_policies(corpses.ROOT, roots=(5973,), explicit_weights=True)
        for mutation in ('root', 'capacity', 'successor'):
            changed = copy.deepcopy(policies)
            if mutation == 'root':
                changed.pop(5973)
            elif mutation == 'capacity':
                changed[4027] = (8, 60000, None)
            else:
                changed[4027] = (None, 60000, 4026)
            with self.subTest(mutation=mutation), patch.object(encounter, 'source_policies', return_value=changed):
                self.refuse({}, 'BOSS_CORPSE_TERMINAL_SOURCE_POLICY')


class ActualElderCorpseTests(unittest.TestCase):
    def test_actual_elder_native_corpse_contains_all_four_source_bodies(self):
        native=json.loads((corpses.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual={row['production_binding']['external_id']:row for row in native['records']}
        for item in (43669,43668,43667,43666):
            self.assertTrue(str(item) in actual, f'actual native Elder corpse Item{item} is missing')
            self.assertEqual(actual[str(item)]['admission']['legal_destinations'],['Ground'])
        terminal=actual['43666']
        self.assertEqual(terminal['semantics']['temporal'],{'state':'NOT_APPLICABLE'})
        self.assertEqual(terminal['semantics']['container'],{'state':'NOT_APPLICABLE'})
        self.assertEqual(terminal['semantics']['physical']['value']['movable'],{'state':'KNOWN','value':True})


class ActualRottenAmbientCorpseTests(unittest.TestCase):
    def test_actual_native_rotten_ambient_death_has_all_source_decay_descendants(self):
        native=json.loads((corpses.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual={row['production_binding']['external_id']:row for row in native['records']}
        # Pinned Crystal items.xml and the fifteen Rotten hunter definitions.
        # Darklight Construct and Source share the same physical corpse root.
        roots=(43816,43567,43840,43583,43571,43844,43559,43555,43579,43820,43832,43836,43824,43828)
        for root in roots:
            for item in range(root-3,root+1):
                self.assertTrue(str(item) in actual,f'actual Rotten ambient corpse Item{item} is missing')
                self.assertEqual(actual[str(item)]['admission']['legal_destinations'],['Ground'])


class SourceSoulWarPhysicalCapacityTests(unittest.TestCase):
    def test_actual_source_ordinary_soul_war_corpses_keep_capacity48(self):
        from build_full_spell_artifact import native_corpse_capacity
        capture=json.loads((corpses.ROOT/corpses.PATH).read_bytes())
        rows={int(row['production_binding']['external_id']):row for row in capture['records']}
        for item in (33797,33801,33805,33809,33813,33817,33897,33901,33905,33909):
            row=rows[item]
            self.assertEqual(native_corpse_capacity(row['production_binding']['target'],row),48,
                f'actual source ordinary SoulWar corpse{item} declares48 slots')


class ActualCompleteSoulWarCorpseClosureTests(unittest.TestCase):
    def test_actual_native_includes_the_complete_source_corpse_physics_closure(self):
        capture=json.loads((corpses.ROOT/corpses.PATH).read_bytes())
        native=json.loads((corpses.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual={row['production_binding']['external_id']:row for row in native['records']}
        for expected in capture['records']:
            item=expected['production_binding']['external_id']
            self.assertTrue(item in actual,f'actual native source corpse Item{item} is missing')
            self.assertEqual(actual[item]['admission'],expected['admission'],item)
            for part in ('physical','stack','container','temporal'):
                # The source capture retains Crystal's fallback8; the accepted
                # encounter terminal owns native non-container custody.
                value = ({'state':'NOT_APPLICABLE'} if (item,part) == ('4027','container')
                         else expected['semantics'][part])
                self.assertEqual(actual[item]['semantics'][part],value,f'{item}/{part}')


if __name__=='__main__':unittest.main()


class ActualOutburstCorpseTests(unittest.TestCase):
    def test_actual_native_outburst_has_literal_three_stage_physical_corpse(self):
        native=json.loads((corpses.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual={row['production_binding']['external_id']:row for row in native['records']}
        # Crystal/Canary Outburst corpse23564; pinned XML46793-46805.
        for item,capacity,duration in ((23564,30,300000),(23565,15,300000),(23566,None,60000)):
            self.assertTrue(str(item) in actual,f'actual native Outburst corpse Item{item} is missing')
            row=actual[str(item)]
            self.assertEqual(row['admission']['legal_destinations'],['Ground'])
            if capacity is None:
                self.assertEqual(row['semantics']['container'],{'state':'NOT_APPLICABLE'})
            else:
                self.assertEqual(row['semantics']['container']['value']['capacity'],{'state':'KNOWN','value':capacity})
            self.assertEqual(row['semantics']['temporal']['value']['duration'],{'state':'KNOWN','value':duration})
