"""Exercise current-source import fences against the actual catalogue."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import import_current_spell_sources as current
import import_spell_families as baseline


class CurrentSourceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.generated = baseline.baseline_outputs()
        cls.config = json.loads((current.ROOT / current.INPUT).read_bytes())

    def test_import_preserves_identities_native_profiles_and_alias_selection(self):
        actual = current.outputs(current.ROOT, self.generated)
        old = json.loads(self.generated[current.CATALOG])
        new = json.loads(actual[current.CATALOG])
        self.assertEqual(old['removed'], new['removed'])
        self.assertEqual(len(new['bundles']), 246)
        replacements = {(binding['target']['key'], binding['target']['revision']): binding['replacement']
                        for binding in self.config['bindings'] if binding['projection'] == 'ordinary_data'}
        changed = set()
        for before, after in zip(old['bundles'], new['bundles']):
            identity = before['bundle']['spell']['identity']
            key = identity['key'], identity['revision']
            self.assertEqual(identity, after['bundle']['spell']['identity'])
            self.assertEqual(before['catalog'], after['catalog'])
            for section in ('abilities', 'effects', 'formulas'):
                self.assertEqual(sorted((row['identity']['key'], row['identity']['revision'])
                                        for row in before['dependencies'][section]),
                                 sorted((row['identity']['key'], row['identity']['revision'])
                                        for row in after['dependencies'][section]))
            self.assertEqual({current.encoded(row['identity']): row['operation']
                              for row in before['dependencies']['effects']},
                             {current.encoded(row['identity']): row['operation']
                              for row in after['dependencies']['effects']})
            if key in replacements:
                self.assertEqual(after['bundle'], replacements[key]['bundle'])
                self.assertEqual(after['dependencies'], replacements[key]['dependencies'])
            else:
                self.assertEqual(current.payload(before), current.payload(after))
            if current.payload(before) != current.payload(after):
                changed.add(key)
        self.assertEqual(len(changed), 20)
        expected_changed = {key for key, replacement in replacements.items()
                            if current.payload(next(row for row in old['bundles']
                                if (row['bundle']['spell']['identity']['key'],
                                    row['bundle']['spell']['identity']['revision']) == key)) !=
                            current.payload({'bundle': replacement['bundle'],
                                             'dependencies': replacement['dependencies'],
                                             'catalog': next(row['catalog'] for row in old['bundles']
                                                if (row['bundle']['spell']['identity']['key'],
                                                    row['bundle']['spell']['identity']['revision']) == key)})}
        self.assertEqual(changed, expected_changed)
        native_profiles = json.loads((current.ROOT /
            'tools/content-schema/spell-authoring/samples/native-spell-profiles.json').read_bytes())['profiles']
        by_name = {(row['bundle']['spell']['name'], row['bundle']['spell']['carrier']): row
                   for row in new['bundles']}
        self.assertEqual(len(native_profiles), 67)
        for profile in native_profiles:
            qualified = by_name[(profile['name'], profile['carrier'])]
            self.assertEqual(qualified['bundle']['spell'], profile['spell'])
            self.assertEqual(qualified['dependencies'], profile['dependencies'])
        a = json.loads(self.generated[current.SELECTION]); b = json.loads(actual[current.SELECTION])
        a.pop('catalog_sha256'); b.pop('catalog_sha256'); self.assertEqual(a, b)
        manifest = json.loads(actual[current.MANIFEST])
        self.assertEqual(manifest['catalog']['sha256'], current.digest(actual[current.CATALOG]))
        self.assertEqual(manifest['source_selection']['sha256'], current.digest(actual[current.SELECTION]))
        self.assertEqual(manifest['creature_profiles']['sha256'], current.digest(actual[current.CREATURE_PROFILES]))
        self.assertEqual(self.generated, baseline.baseline_outputs())

    def test_source_formulas_preserve_magnitudes_and_reach_formula_collection(self):
        actual = current.outputs(current.ROOT, self.generated)
        old = json.loads(self.generated[current.CATALOG])['bundles']
        new = json.loads(actual[current.CATALOG])['bundles']
        formulas = json.loads(actual['content/abilities/formulas/player-spell-formulas.json'])['records']
        evaluate = baseline.spell_validator(current.ROOT).evaluate
        for before, after in zip(old, new):
            if before['bundle']['spell']['name'] not in ('Buzz', 'Scorch'):
                continue
            source = after['dependencies']['formulas'][0]
            self.assertIn(source, formulas)
            self.assertEqual(source['identity'], before['dependencies']['formulas'][0]['identity'])
            for level in (1, 8, 20, 100, 500, 1500, 2500):
                for magic in (0, 1, 30, 100, 130):
                    env = {'level': level, 'magic_level': magic}
                    for bound in ('minimum', 'maximum'):
                        self.assertEqual(evaluate(source[bound], env),
                                         evaluate(before['dependencies']['formulas'][0][bound], env))

    def test_partial_sources_keep_operational_holds(self):
        held = {json.loads(line)['registration']: json.loads(line) for line in
                (current.ROOT / self.config['held_path']).read_bytes().splitlines()}
        updates = [b for b in self.config['bindings'] if b['projection'] == 'ordinary_data']
        self.assertEqual(len(self.config['bindings']), 368)
        self.assertEqual(len(held), 486)
        self.assertEqual(len(updates), 75)
        partial = [b for b in updates if b['replacement']['scope'] == 'canonical_base']
        accepted = [b for b in updates if b['replacement']['scope'] == 'accepted_ordinary_model']
        self.assertEqual(len(partial), 30)
        self.assertEqual(len(accepted), 45)
        self.assertTrue(all(b['registration'] in held for b in partial))
        overlaps = [b for b in accepted if b['registration'] in held]
        self.assertEqual(len(overlaps), 27)
        self.assertTrue(all(held[b['registration']]['reason'] ==
                            'PvP_field_creation_requires_current_ATTACK_owner_field_events_and_PLAYER_PvP_closure'
                            for b in overlaps))

    def test_pvp_field_context_holds_preserve_the_imported_base_model(self):
        held = [json.loads(line) for line in
                (current.ROOT / self.config['held_path']).read_bytes().splitlines()]
        fields = [row for row in held if row.get('scope') == 'runtime_pvp_field_creation']
        self.assertEqual(len(fields), 27)
        self.assertEqual(len({row['name'] for row in fields}), 9)
        bindings = {row['registration']: row for row in self.config['bindings']}
        for flag in fields:
            self.assertEqual(flag['missing_class'], 'RUNTIME_GAP')
            self.assertFalse(flag['runtime_activation'])
            self.assertTrue(flag['base_model_imported'])
            self.assertEqual(flag['supported_context'], 'target tile NoPvP or world FieldWorldType::NoPvp')
            self.assertEqual(flag['blocked_context'], 'PvP world and target tile without NoPvP')
            self.assertEqual(flag['consumer_seam'],
                             'apps/game-server/src/gameplay_transport/ordinary_field_items.rs::append_creations')
            binding = bindings[flag['registration']]
            self.assertEqual(binding['projection'], 'ordinary_data')
            self.assertEqual(binding['replacement']['scope'], 'accepted_ordinary_model')
            self.assertEqual(binding['source']['sha256'], flag['sha256'])
            self.assertEqual(binding['source']['revision'], flag['revision'])
            self.assertFalse(binding['qualification']['raw_source_parity'])
            self.assertFalse(binding['qualification']['runtime_activation'])

    def test_native_partial_bindings_preserve_frozen_payload_and_actual_donor_receipt(self):
        rows = [b for b in self.config['bindings'] if b.get('qualification', {}).get('scope') ==
                'accepted_partial_native_profile']
        self.assertEqual(len(rows), 36)
        before = {current.encoded(e['bundle']['spell']['identity']): e
                  for e in json.loads(self.generated[current.CATALOG])['bundles']}
        held = {json.loads(line)['registration']: json.loads(line) for line in
                (current.ROOT / self.config['held_path']).read_bytes().splitlines()}
        for b in rows:
            e = before[current.encoded(b['target'])]
            self.assertEqual(b['projection'], 'identity_only')
            self.assertEqual(b['payload_sha256'], current.payload(e))
            current.partial_native(b, e, held[b['registration']], json.loads(self.generated[current.SELECTION]))
            self.assertEqual(b['qualification']['accepted_native_profile'],
                             {'bundle': e['bundle'], 'dependencies': e['dependencies']})
            self.assertFalse(b['qualification']['whole_source_controller_equivalence'])

    def test_partial_native_rejects_wrong_profile_receipt_source_flags_or_missing_hold(self):
        for mutation in ('profile', 'normalized', 'whole', 'raw_parity', 'runtime', 'scope',
                         'raw_model', 'receipt_hash', 'receipt_scope', 'source', 'no_hold', 'party_scope'):
            with self.subTest(mutation=mutation):
                def change(config):
                    b = next(b for b in config['bindings'] if b.get('qualification', {}).get('scope') ==
                             'accepted_partial_native_profile')
                    q = b['qualification']
                    if mutation == 'profile': q['accepted_native_profile']['bundle']['spell']['name'] = 'Enchant Party'
                    elif mutation == 'normalized': q['normalized_controller_equality'] = False
                    elif mutation == 'whole': q['whole_source_controller_equivalence'] = True
                    elif mutation == 'raw_parity': q['raw_source_parity'] = True
                    elif mutation == 'runtime': q['runtime_activation'] = True
                    elif mutation == 'scope': q['remaining_source_scope'] = []
                    elif mutation in ('raw_model', 'receipt_scope'):
                        if mutation == 'raw_model': q['source_receipt'].pop('raw_source_model')
                        else: q['source_receipt']['partial_source_scope'] = []
                        q['source_receipt_sha256'] = current.digest(current.encoded(q['source_receipt']))
                    elif mutation == 'receipt_hash': q['source_receipt_sha256'] = '0' * 64
                    elif mutation == 'source': b['source']['sha256'] = '0' * 64
                    elif mutation == 'no_hold': b['registration'] = b['registration'].rsplit('#', 1)[0] + '#999'
                    else: q['scope'] = 'accepted_partial_model'
                self.rejected(change, 'PARTIAL|QUALIFICATION', reviewed=True)

    def test_partial_ordinary_keeps_audio_formula_and_conflict_without_importing_donor_numbers(self):
        rows = [b for b in self.config['bindings'] if b.get('qualification', {}).get('scope') ==
                'accepted_partial_ordinary_base']
        self.assertEqual(len(rows), 18)
        before = {current.encoded(e['bundle']['spell']['identity']): e
                  for e in json.loads(self.generated[current.CATALOG])['bundles']}
        for b in rows:
            e = before[current.encoded(b['target'])]
            self.assertEqual(b['replacement']['bundle'], e['bundle'])
            self.assertEqual(b['replacement']['dependencies'], e['dependencies'])
            self.assertTrue(b['qualification']['remaining_source_scope'])
            self.assertTrue(all(x['missing_class'] == 'SOURCE_CONFLICT_KEPT'
                                for x in b['qualification']['remaining']))
        for mutation in ('audio', 'formula', 'scope', 'conflict'):
            def change(config):
                b = next(b for b in config['bindings'] if b.get('qualification', {}).get('scope') ==
                         'accepted_partial_ordinary_base')
                if mutation == 'audio': b['replacement']['bundle']['spell']['presentation'].pop('cast_cue')
                elif mutation == 'formula': b['replacement']['dependencies']['formulas'][0]['minimum'] = {'const': '1'}
                elif mutation == 'scope': b['qualification']['remaining_source_scope'] = []
                else: b['qualification']['remaining'] = []
            self.rejected(change, 'PARTIAL_ORDINARY_SCOPE', reviewed=True)

    def test_party_partial_bindings_preserve_payload_and_donor_holds(self):
        parties = [b for b in self.config['bindings']
                   if b.get('qualification', {}).get('scope') == 'accepted_partial_model']
        held = {json.loads(line)['registration']: json.loads(line) for line in
                (current.ROOT / self.config['held_path']).read_bytes().splitlines()}
        self.assertEqual(len(parties), 15)
        self.assertEqual(len({b['target']['key'] for b in parties}), 5)
        before = {current.encoded(e['bundle']['spell']['identity']): e
                  for e in json.loads(self.generated[current.CATALOG])['bundles']}
        after = {current.encoded(e['bundle']['spell']['identity']): e
                 for e in json.loads(current.outputs(current.ROOT, self.generated)[current.CATALOG])['bundles']}
        for binding in parties:
            key = current.encoded(binding['target'])
            self.assertEqual(binding['projection'], 'identity_only')
            self.assertEqual(current.payload(before[key]), current.payload(after[key]))
            self.assertEqual(held[binding['registration']]['missing_class'], 'SOURCE_CONFLICT_KEPT')
            self.assertEqual(held[binding['registration']]['source_scope'],
                             binding['qualification']['remaining_source_scope'])

    def test_partial_identity_cannot_bypass_a_different_hold_or_native_guard(self):
        party = next(b for b in self.config['bindings']
                     if b.get('qualification', {}).get('scope') == 'accepted_partial_model')
        def wrong_target(config):
            b = next(b for b in config['bindings'] if b['projection'] == 'identity_only'
                     and not b.get('qualification', {}).get('scope'))
            b['qualification'] = copy.deepcopy(party['qualification'])
        self.rejected(wrong_target, 'PARTIAL_SCOPE', reviewed=True)
        def wrong_hold(config):
            b = copy.deepcopy(party)
            rows = [json.loads(line) for line in
                    (current.ROOT / config['held_path']).read_bytes().splitlines()]
            row = next(r for r in rows if 'familiar' in r['path'])
            # Familiar now has its own qualified binding: replace that row so
            # this negative isolates the party/hold mismatch, not duplication.
            config['bindings'] = [old for old in config['bindings']
                                  if old['registration'] != row['registration']]
            b['registration'] = row['registration']
            b['source'].update(source_id=row['source_id'], path=row['path'],
                               revision=row['revision'], sha256=row['sha256'])
            config['bindings'].append(b)
        self.rejected(wrong_hold, 'PARTIAL_SCOPE', reviewed=True)
        def drop_scope(config):
            b = next(b for b in config['bindings']
                     if b.get('qualification', {}).get('scope') == 'accepted_partial_model')
            b['qualification'].pop('remaining_source_scope')
        self.rejected(drop_scope, 'PARTIAL_SCOPE', reviewed=True)

    def test_dot_import_preserves_schedule_and_roles_and_enables_zero_health_path(self):
        actual = current.outputs(current.ROOT, self.generated)
        before = {row['bundle']['spell']['name']: row
                  for row in json.loads(self.generated[current.CATALOG])['bundles']}
        after = {row['bundle']['spell']['name']: row
                 for row in json.loads(actual[current.CATALOG])['bundles']}
        names = {'Curse', 'Electrify', 'Envenom', 'Holy Flash', 'Ignite', 'Inflict Wound', 'soulfire rune'}
        for name in names:
            old, new = before[name], after[name]
            self.assertEqual(old['bundle'], new['bundle'])
            self.assertEqual(old['catalog'], new['catalog'])
            self.assertEqual(old['dependencies']['effects'], new['dependencies']['effects'])
            expected = copy.deepcopy(old['dependencies'])
            for ability in expected['abilities']:
                ability['zero_damage_health_path'] = True
            self.assertEqual(new['dependencies'], expected)
            effects = {current.encoded(effect['identity']): effect for effect in new['dependencies']['effects']}
            for ability in new['dependencies']['abilities']:
                roles = [effects[current.encoded({key: ref[key] for key in ('key', 'revision')})]['operation']
                         for ref in ability['effects']]
                self.assertEqual(roles, ['presentation_only', 'condition'])

    def rejected(self, change, reason, reviewed=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); config = copy.deepcopy(self.config); change(config)
            target = root / current.INPUT; target.parent.mkdir(parents=True)
            input_bytes = current.encoded(config)
            target.write_bytes(input_bytes)
            held = root / config['held_path']
            held.write_bytes((current.ROOT / config['held_path']).read_bytes())
            monster_held = root / config['monster_melee']['held_path']
            monster_held.write_bytes((current.ROOT / config['monster_melee']['held_path']).read_bytes())
            with patch.object(current, 'adapter', return_value=current.adapter(current.ROOT)):
                if reviewed:
                    # Exercise downstream guards independently of the receipt fence.
                    with patch.object(current, 'INPUT_SHA256', current.digest(input_bytes)):
                        with self.assertRaisesRegex(ValueError, reason):
                            current.outputs(root, self.generated)
                else:
                    with self.assertRaisesRegex(ValueError, reason):
                        current.outputs(root, self.generated)

    def test_forged_provenance_and_retargeting_require_review(self):
        self.rejected(lambda c: c['bindings'][0]['source'].update(sha256='0'*64), 'UNREVIEWED_INPUT')
        self.rejected(lambda c: c['bindings'][0]['source'].update(path='data/scripts/spells/phantom.lua'), 'UNREVIEWED_INPUT')
        self.rejected(lambda c: c['bindings'][0]['target'].update(key='invented'), 'UNREVIEWED_INPUT')

    def test_changed_target_payload_and_unknown_identity_are_rejected(self):
        self.rejected(lambda c: c['bindings'][0].update(payload_sha256='0'*64), 'PAYLOAD_CHANGED', reviewed=True)
        self.rejected(lambda c: c['bindings'][0]['target'].update(key='invented'), 'PAYLOAD_CHANGED', reviewed=True)

    def test_activation_and_duplicate_registration_are_rejected(self):
        self.rejected(lambda c: c.update(runtime_activation=True), 'INPUT_SHAPE', reviewed=True)
        self.rejected(lambda c: c['bindings'].append(c['bindings'][0]), 'REGISTRATION_CONFLICT', reviewed=True)

    def test_monster_preimages_use_qualified_compact_records_without_newline(self):
        records = json.loads(self.generated[current.CREATURE_PROFILES])['records']
        by_id = {current.encoded(row['profile']['target']): row for row in records}
        data = self.config['monster_melee']
        self.assertEqual(data['base_profiles_sha256'], current.digest(self.generated[current.CREATURE_PROFILES]))
        self.assertEqual(len(data['refresh_bindings']), 1253)
        self.assertEqual(len(data['new_melee_bindings']), 98)
        for binding in data['refresh_bindings'] + data['new_melee_bindings']:
            record = by_id[current.encoded(binding['creature'])]
            qualified_bytes = json.dumps(record, sort_keys=True,
                                         separators=(',', ':')).encode()
            self.assertFalse(qualified_bytes.endswith(b'\n'))
            self.assertEqual(hashlib.sha256(qualified_bytes).hexdigest(), binding['base_record_sha256'])
            self.assertNotEqual(hashlib.sha256(qualified_bytes + b'\n').hexdigest(), binding['base_record_sha256'])

    def test_unicode_monster_preimage_uses_escaped_record_inside_utf8_collection(self):
        collection = self.generated[current.CREATURE_PROFILES]
        record = next(row for row in json.loads(collection)['records']
                      if row['profile']['target']['key'] == 'oteryn:creature.frost_giant')
        data = self.config['monster_melee']
        binding = next(row for row in data['refresh_bindings'] + data['new_melee_bindings']
                       if row['creature'] == record['profile']['target'])
        qualified_bytes = json.dumps(record, sort_keys=True, separators=(',', ':')).encode()
        utf8_bytes = json.dumps(record, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
        self.assertIn('Hörre Sjan Flan!'.encode(), collection)
        self.assertIn(b'H\\u00f6rre Sjan Flan!', qualified_bytes)
        self.assertEqual(hashlib.sha256(qualified_bytes).hexdigest(), binding['base_record_sha256'])
        self.assertNotEqual(hashlib.sha256(utf8_bytes).hexdigest(), binding['base_record_sha256'])

    def test_monster_import_refreshes_only_revision_and_adds_only_bound_melee(self):
        actual = current.outputs(current.ROOT, self.generated)
        old = json.loads(self.generated[current.CREATURE_PROFILES])
        new = json.loads(actual[current.CREATURE_PROFILES])
        before_records, after_records = old.pop('records'), new.pop('records')
        self.assertEqual(old, new)
        self.assertEqual(len(before_records), len(after_records))
        data = self.config['monster_melee']
        refresh = {current.encoded(row['creature']): row for row in data['refresh_bindings']}
        additions = {current.encoded(row['creature']): row for row in data['new_melee_bindings']}
        self.assertEqual(set(refresh) & set(additions), set())
        donors = current.adapter(current.ROOT).DONORS
        self.assertEqual(sum(bool(row.get('monster_melee')) for row in before_records), 1253)
        self.assertEqual(sum(bool(row.get('monster_melee')) for row in after_records), 1351)
        for before, after in zip(before_records, after_records):
            key = current.encoded(before['profile']['target'])
            expected = copy.deepcopy(before)
            if key in refresh:
                expected['monster_melee']['source_revision'] = donors[refresh[key]['source_id']][2]
            elif key in additions:
                self.assertNotIn('monster_melee', before)
                expected['monster_melee'] = copy.deepcopy(additions[key]['monster_melee'])
                melee = expected['monster_melee']
                self.assertTrue(any(attack['ability'] == melee['ability'] and
                                    attack['interval_ms'] == melee['interval_ms'] and
                                    attack['chance_ppm'] == melee['chance_ppm'] and
                                    attack.get('range_tiles', 1) == 1
                                    for attack in before['behavior']['data']['profile']['attacks']))
            self.assertEqual(expected, after)
            for field in ('profile', 'presentation', 'behavior'):
                self.assertEqual(before[field], after[field])

    def monster_rejected(self, change, reason):
        # Exercise the melee guards independently of the production receipt pin.
        data = copy.deepcopy(self.config['monster_melee'])
        change(data)
        with self.assertRaisesRegex(ValueError, reason):
            current.monster_melee(data, self.generated, current.adapter(current.ROOT).DONORS)

    def test_monster_bad_record_preimage_is_rejected(self):
        self.monster_rejected(lambda data: data['refresh_bindings'][0].update(base_record_sha256='0' * 64),
                              'CURRENT_MONSTER_RECORD_CHANGED')

    def test_monster_duplicate_creature_binding_is_rejected(self):
        self.monster_rejected(lambda data: data['refresh_bindings'].append(data['refresh_bindings'][0]),
                              'CURRENT_MONSTER_RECORD_CHANGED')

    def test_monster_unknown_donor_is_rejected(self):
        self.monster_rejected(lambda data: data['refresh_bindings'][0].update(source_id='unknown-donor'),
                              'CURRENT_MONSTER_DONOR_MISMATCH')

    def test_monster_unbound_ability_or_interval_is_rejected(self):
        self.monster_rejected(lambda data: data['new_melee_bindings'][0]['monster_melee']['ability'].update(key='invented'),
                              'CURRENT_MONSTER_ABILITY_UNBOUND')
        self.monster_rejected(lambda data: data['new_melee_bindings'][0]['monster_melee'].update(interval_ms=999999),
                              'CURRENT_MONSTER_ABILITY_UNBOUND')


if __name__ == '__main__':
    unittest.main()
