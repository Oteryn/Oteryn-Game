import unittest

import audit_rune_carriers as audit


def profile(key, stackable=True, stack_max=100, stack_class='StackCapable'):
    return {'authoring': {'item': {'key': key}}, 'admission': {'stack_class': stack_class},
            'semantics': {'stack': {'state': 'KNOWN', 'value': {
                'stackable': {'state': 'KNOWN', 'value': stackable},
                'stack_max': {'state': 'KNOWN', 'value': stack_max}}}}}


def carrier(name, item, charges, area=None, creates_fields=False, group='attack', vocations=('sorcerer',),
            aggressive=True, **targeting):
    abilities = [{'area': {'matrix': {'north': area}}}] if area else [{}]
    effects = [{'operation': 'create_item'}] if creates_fields else []
    return {'bundle': {'spell': {
        'name': name, 'carrier': 'rune', 'requirements': {'level': 1, 'vocations': list(vocations)},
        'costs': {'mana': 0, 'soul': 0}, 'pz_locks_caster': False, 'needs_weapon': False,
        'groups': [{'group': group, 'cooldown_ms': 2000}],
        'targeting': {'needs_target': False, 'aggressive': aggressive, 'self_target': False, 'needs_direction': False,
                      'target_or_direction': False, 'block_walls': True, 'allow_on_self': True, 'check_floor': True,
                      'parameter': 'none',
                      **targeting},
        'rune': {'item': {'key': f'candidate:item/{item}'}, 'charges': charges, 'magic_level': 1,
                 'allow_far_use': True, 'blocking': {'solid': False, 'creature': False}},
        'execution': {'ability': {}}}}, 'dependencies': {'abilities': abilities, 'effects': effects}}


def conjure(name, item, count):
    return {'bundle': {'spell': {'name': name, 'carrier': 'instant', 'execution': {
        'conjure': {'count': count, 'result': {'key': f'candidate:item/{item}'}}}}}}


def run(entries, profiles, resolutions=None, drop=None, guards=None, spell_pages=()):
    documents = {'fandom': {'pages': [
        {'template': 'Infobox Object', 'title': name.title(),
         'fields': {'name': name.title(), 'itemid': str(item), 'vocrequired': 'sorcerers'}}
        for name, item in (('sudden death rune', 3155), ('fire wall rune', 3190), ('great fireball rune', 3191))]
        + list(spell_pages)}}
    census = {'canary': [], 'crystal': []}
    for source in census:
        for name, item in (('sudden death rune', 3155), ('fire wall rune', 3190), ('great fireball rune', 3191)):
            census[source].append({'spell_type': 'rune', 'name': name, 'registrar': {
                'runeId': item, 'group': 'attack', 'allowFarUse': True}})
    if drop:
        census[drop] = []
    summary, rows = audit.audit({'bundles': entries}, documents, census, {'spells': resolutions or {}},
                                {'records': profiles}, guards)
    return summary, {row['name']: row for row in rows}


class RuneCarrierAuditTest(unittest.TestCase):
    def test_area_tiles_and_blocking(self):
        self.assertEqual(audit.area_tiles({}), 1)
        self.assertEqual(audit.area_tiles({'area': {'matrix': {'north': ['xxCxx'], 'diagonal': ['..x', '.Cx', 'x..']}}}), 5)
        self.assertEqual(audit.area_tiles({'area': {'matrix': {'north': ['xxx', 'xcx', 'xxx']}}}), 8)
        self.assertIsNone(audit.area_tiles({'area': {'radius_tiles': 5}}))
        self.assertIsNone(audit.area_tiles({'area': {'length_tiles': 8, 'spread_tiles': 3}}))
        self.assertEqual(audit.registrar_blocking({'canary': {'isBlocking': [True, False]}}, 1), {'canary': False})
        self.assertEqual(audit.registrar_blocking({'canary': {'isBlocking': True}}, 0), {'canary': True})
        self.assertEqual(audit.registrar_blocking({'canary': {'isBlocking': True}}, 1), {'canary': False})
        self.assertEqual(audit.registrar_vocations({'canary': {'vocation': ['master sorcerer;true', 'druid;true']}}),
                         {'canary': ['druid', 'sorcerer']})

    def test_checks(self):
        entries = [carrier('sudden death rune', 3155, 3), conjure('sudden death rune', 3155, 3),
                   carrier('fire wall rune', 3190, 5, area=['xxxxxxxxxxxxC'], creates_fields=True),
                   conjure('fire wall rune', 3190, 4),
                   carrier('great fireball rune', 3191, 4, area=['x' * 38 + 'C'])]
        profiles = [profile('candidate:item/3155'), profile('candidate:item/3190', stack_max=1),
                    profile('candidate:item/3191')]
        summary, rows = run(entries, profiles)
        self.assertEqual(rows['sudden death rune']['verdict'], 'checks_pass')
        self.assertEqual(rows['sudden death rune']['checks']['area']['victim_cap'], 2)
        self.assertEqual(rows['fire wall rune']['failed'], ['area', 'charges', 'stack'])
        self.assertEqual(rows['fire wall rune']['checks']['area']['tiles'], 13)
        self.assertEqual(rows['great fireball rune']['failed'], ['area'])
        self.assertEqual(rows['great fireball rune']['checks']['charges']['verdict'], 'no_conjuring_spell')
        self.assertEqual(summary['checks_fail'], ['fire wall rune', 'great fireball rune'])
        self.assertIn('energy wall rune', summary['inventory']['missing'])

    def test_every_conjure_count_is_audited(self):
        entries = [carrier('sudden death rune', 3155, 3), conjure('sudden death rune', 3155, 3),
                   conjure('practise sudden death', 3155, 1)]
        _, rows = run(entries, [profile('candidate:item/3155')])
        charges = rows['sudden death rune']['checks']['charges']
        self.assertEqual(charges['conjure_counts'], {'practise sudden death': 1, 'sudden death rune': 3})
        self.assertFalse(charges['pass'])

    def test_current_resolution_replaces_the_field(self):
        entries = [carrier('sudden death rune', 3155, 3)]
        resolution = {'sudden death rune': {'carrier': 'rune', 'field': 'primary_group', 'value': 'support',
                                            'source': {'url': 'u', 'captured': '2026-10-09'}}}
        _, rows = run(entries, [profile('candidate:item/3155')], resolution)
        self.assertEqual(rows['sudden death rune']['failed'], ['group'])
        self.assertEqual(rows['sudden death rune']['checks']['group']['verdict'], 'current_resolution')

    def test_conflict_follows_the_recorded_winner(self):
        found = {'br': ['druid'], 'fandom': ['druid', 'monk']}
        fandom = {'sources': {'fandom'}}
        self.assertTrue(audit.check(['druid', 'monk'], found, recorded=fandom)['pass'])
        self.assertFalse(audit.check(['druid'], found, recorded=fandom)['pass'])
        self.assertFalse(audit.check(['druid'], found)['pass'])
        source = {'api': 'https://tibia.fandom.com/api.php'}
        entry = {'manifest': {'sources': [source], 'entries': [
            {'destination': '/d', 'status': 'mapped', 'source_index': 0},
            {'destination': '/n', 'status': 'resolved_native_behavior', 'source_index': 0, 'resolution': 'S27'}]}}
        self.assertEqual(audit.recorded(entry, '/d'), {'sources': {'fandom'}, 'native': None})
        # Two mapped registrars recorded agreement; once they diverge Canary decides (S4).
        both = {'sources': {'canary', 'crystal'}}
        self.assertFalse(audit.check('attack', {'canary': 'support', 'crystal': 'attack'}, recorded=both)['pass'])
        self.assertTrue(audit.check('support', {'canary': 'support', 'crystal': 'attack'}, recorded=both)['pass'])
        wikis = {'sources': {'fandom', 'br'}}
        self.assertFalse(audit.check(['druid'], found, recorded=wikis)['pass'])
        self.assertEqual(audit.recorded(entry, '/n')['native'], 'S27')

    def test_group_follows_the_wiki_runegroup(self):
        # A refreshed wiki runegroup that diverges from the registrars wins, as the converter selects it (S3).
        page = {'template': 'Infobox Spell', 'title': 'Sudden Death Rune',
                'fields': {'name': 'Sudden Death Rune', 'runegroup': 'Support'}}
        manifest = {'sources': [{'api': 'https://tibia.fandom.com/api.php'}], 'entries': [
            {'destination': '/spell/spell/groups/0/group', 'status': 'mapped', 'source_index': 0}]}
        for group, failed in (('support', []), ('attack', ['group'])):
            entry = {**carrier('sudden death rune', 3155, 3, group=group), 'manifest': manifest}
            _, rows = run([entry], [profile('candidate:item/3155')], spell_pages=[page])
            row = rows['sudden death rune']
            self.assertEqual(row['checks']['group']['references']['fandom'], 'support')
            self.assertEqual(row['failed'], failed)

    def test_radius_and_beam_areas_fail_as_unaudited(self):
        entry = carrier('great fireball rune', 3191, 4)
        entry['dependencies']['abilities'] = [{'area': {'radius_tiles': 99}}, {}]
        _, rows = run([entry], [profile('candidate:item/3191')])
        area = rows['great fireball rune']['checks']['area']
        self.assertEqual((area['unaudited_abilities'], area['tiles'], area['pass']), ([0], 1, False))
        self.assertEqual(rows['great fireball rune']['failed'], ['area'])

    def test_aim_at_target_reads_the_spell_pages(self):
        profiles = [profile('candidate:item/3155')]
        spell = {'template': 'Infobox Spell', 'title': 'Sudden Death Rune',
                 'fields': {'name': 'Sudden Death Rune', 'aimattarget': 'yes'}}
        thing = {'template': 'Infobox Object', 'title': 'Sudden Death Rune',
                 'fields': {'name': 'Sudden Death Rune', 'itemid': '3155', 'aimattarget': 'yes'}}
        for pages, expected in (([spell], True), ([thing], False)):
            _, rows = run([carrier('sudden death rune', 3155, 3, needs_direction=True, aim_at_target=expected)],
                          profiles, spell_pages=pages)
            aim = rows['sudden death rune']['checks']['aim_at_target']
            self.assertEqual((aim['expected'], aim['pass']), (expected, True))

    def test_targeting_flags_and_native_resolution(self):
        _, rows = run([carrier('sudden death rune', 3155, 3, aggressive=False)], [profile('candidate:item/3155')])
        self.assertEqual(rows['sudden death rune']['failed'], ['aggressive'])
        _, rows = run([carrier('sudden death rune', 3155, 3, block_walls=False, needs_direction=True)],
                      [profile('candidate:item/3155')])
        self.assertEqual(rows['sudden death rune']['failed'], ['block_walls', 'needs_direction'])
        native = audit.check(False, {'canary': True, 'crystal': True}, recorded={'native': 'S27 D.2'})
        self.assertEqual((native['verdict'], native['pass']), ('native_resolution', True))
        reverted = audit.check(True, {'canary': True, 'crystal': True}, recorded={'native': 'S27 D.2'})
        self.assertEqual((reverted['verdict'], reverted['pass']), ('agree', False))

    def test_target_guard(self):
        guards = {'sudden death rune': {'spell_type': 'rune', 'targeting': {'allowed_targets': 'self_or_own_summons'}}}
        profiles = [profile('candidate:item/3155')]
        _, rows = run([carrier('sudden death rune', 3155, 3, allowed_targets='self_or_own_summons')], profiles,
                      guards=guards)
        self.assertEqual(rows['sudden death rune']['verdict'], 'checks_pass')
        _, rows = run([carrier('sudden death rune', 3155, 3, allowed_targets='any')], profiles, guards=guards)
        self.assertEqual(rows['sudden death rune']['failed'], ['target_guard'])
        _, rows = run([carrier('sudden death rune', 3155, 3, allowed_targets='self_only')], profiles)
        self.assertEqual(rows['sudden death rune']['failed'], ['target_guard'])

    def test_parameter_mode(self):
        profiles = [profile('candidate:item/3155')]
        _, rows = run([carrier('sudden death rune', 3155, 3, parameter='player_name')], profiles)
        self.assertEqual(rows['sudden death rune']['failed'], ['parameter'])
        self.assertEqual(rows['sudden death rune']['checks']['parameter']['registrars'], 'none')

    def test_positional_cast_and_unaudited_fields(self):
        profiles = [profile('candidate:item/3155')]
        _, rows = run([carrier('sudden death rune', 3155, 3, cast_at_position=True)], profiles)
        self.assertEqual(rows['sudden death rune']['failed'], ['cast_at_position'])
        _, rows = run([carrier('sudden death rune', 3155, 3, aim_at_target=True)], profiles)
        self.assertEqual(rows['sudden death rune']['failed'], ['aim_at_target'])
        _, rows = run([carrier('sudden death rune', 3155, 3, new_mode='x')], profiles)
        self.assertEqual(rows['sudden death rune']['failed'], ['targeting_fields'])
        self.assertEqual(rows['sudden death rune']['checks']['targeting_fields']['unaudited'], ['new_mode'])

    def test_spell_flags(self):
        entry = carrier('sudden death rune', 3155, 3)
        entry['bundle']['spell']['pz_locks_caster'] = True
        _, rows = run([entry], [profile('candidate:item/3155')])
        self.assertEqual(rows['sudden death rune']['failed'], ['pz_locks_caster'])

    def test_range_is_checked_against_the_registrars(self):
        row = audit.check(99, {'canary': 5, 'crystal': 5})
        self.assertEqual((row['verdict'], row['pass']), ('ours_differs', False))

    def test_source_silent_field_must_stay_absent(self):
        self.assertTrue(audit.check(None, {})['pass'])
        row = audit.check(99, {})
        self.assertEqual((row['verdict'], row['pass']), ('no_source', False))

    def test_both_registrar_records_are_required(self):
        entries = [carrier('sudden death rune', 3155, 3)]
        _, rows = run(entries, [profile('candidate:item/3155')], drop='crystal')
        self.assertEqual(rows['sudden death rune']['failed'], ['registrars'])
        _, rows = run(entries, [profile('candidate:item/3155')])
        self.assertEqual(rows['sudden death rune']['checks']['registrars']['present'], ['canary', 'crystal'])

    def test_missing_item_evidence_fails(self):
        summary, rows = run([carrier('unknown rune', 9999, 1)], [profile('candidate:item/9999')])
        failed = set(rows['unknown rune']['failed'])
        self.assertLessEqual({'group', 'item_id', 'registrars', 'vocations'}, failed)
        # With no reference at all, every flag the catalogue states is an unsourced value and fails too.
        self.assertIn('block_walls', failed)
        self.assertNotIn('range_tiles', failed)
        self.assertEqual(summary['inventory']['unexpected'], ['unknown rune'])
        self.assertIsNone(rows['unknown rune']['section_9_group'])

    def test_duplicate_carriers_are_reported(self):
        entries = [carrier('sudden death rune', 3155, 3), carrier('sudden death rune', 3155, 3)]
        summary, _ = run(entries, [profile('candidate:item/3155')])
        self.assertEqual(summary['inventory']['duplicates'], ['sudden death rune'])

    def test_committed_audit_is_current(self):
        self.assertEqual(audit.main(['--check']), 0)


if __name__ == '__main__':
    unittest.main()
