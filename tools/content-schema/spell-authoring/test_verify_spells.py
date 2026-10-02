"""Offline comparison regressions: identity joins and evidence coverage, not runtime readiness."""
import unittest

import verify_spells as vs


def page(name, **fields):
    return {'template': 'Infobox Object', 'title': name, 'fields': {'name': name, **fields}}


def references(*pages):
    return vs.References({'fandom': {'pages': list(pages)}})


def rune(name='antidote rune', item_key='candidate:item/3153'):
    return {'name': name, 'carrier': 'rune', 'requirements': {'level': 15, 'vocations': ['druid', 'elder_druid']},
            'costs': {'mana': 0, 'soul': 0}, 'cooldown_ms': 1000,
            'groups': [{'group': 'healing', 'cooldown_ms': 1000}], 'targeting': {'range_tiles': 7},
            'rune': {'item': {'key': item_key}, 'magic_level': 0, 'charges': 1}, 'execution': {}}


def verify(spell, refs):
    return vs.verify([spell], refs, {'spells': [{'spell_type': spell['carrier'], 'name': spell['name'],
                                             'status': 'ready'}]}, {'changes': []})


class RuneReferenceTests(unittest.TestCase):
    def test_all_five_source_aliases_without_ids(self):
        for source, wiki in vs.RUNE_ALIASES.items():
            with self.subTest(source=source):
                pages, issues = references(page(wiki.title())).rune_matches(source)
                self.assertEqual(pages['fandom']['title'], wiki.title())
                self.assertFalse(issues)

    def test_actualname_and_item_title_suffix(self):
        refs = references(page('Cure Poison Rune (Item)', actualname='cure poison rune'))
        self.assertIn('fandom', refs.rune_pages('antidote rune'))
        # An actual name also works when the page's title/name is unrelated.
        self.assertIn('fandom', references(page('Archived title', actualname='antidote rune')).rune_pages('cure poison rune'))

    def test_unique_item_id_precedes_names(self):
        refs = references(page('Unrelated translated name', itemid='3153'),
                          page('Antidote Rune', itemid='999'))
        pages, issues = refs.rune_matches('antidote rune', 3153)
        self.assertEqual(pages['fandom']['title'], 'Unrelated translated name')
        self.assertFalse(issues)

    def test_conflicting_id_is_reported_and_never_silently_joined(self):
        summary, rows, unmatched = verify(rune(), references(page('Cure Poison Rune', itemid='999')))
        self.assertEqual(summary['reference_match_issues'][0]['reason'], 'conflicting_item_id')
        self.assertEqual(summary['reference_match_issues'][0]['reference_itemid'], 999)
        self.assertEqual(summary['unmatched'], 1)
        self.assertFalse(rows)
        self.assertEqual(unmatched[0]['name'], 'antidote rune')

    def test_duplicate_id_is_ambiguous_even_with_matching_name(self):
        refs = references(page('Antidote Rune', itemid='3153'), page('Other Rune', itemid='3153'))
        pages, issues = refs.rune_matches('antidote rune', 3153)
        self.assertFalse(pages)
        self.assertEqual(issues[0]['reason'], 'ambiguous_item_id')

    def test_alias_name_collision_is_ambiguous_without_id(self):
        refs = references(page('Antidote Rune'), page('Cure Poison Rune'))
        pages, issues = refs.rune_matches('antidote rune')
        self.assertFalse(pages)
        self.assertEqual(issues[0]['reason'], 'ambiguous_name')

    def test_item_key_namespaces_are_explicit(self):
        self.assertEqual(vs.rune_item_id(rune(item_key='legacyitem:3153')), 3153)
        self.assertEqual(vs.rune_item_id(rune()), 3153)
        for key in ('other:item/3153', 'candidate:item/0', 'candidate:item/3153/extra', 'legacyitem:abc'):
            with self.subTest(key=key):
                self.assertIsNone(vs.rune_item_id(rune(item_key=key)))

    def test_missing_ours_item_id_is_visible_when_reference_has_it(self):
        summary, rows, _ = verify(rune(item_key='custom:cure_poison'),
                                  references(page('Cure Poison Rune', itemid='3153')))
        self.assertEqual(summary['field_counts']['itemid'], {'ours_differs': 1})
        self.assertEqual(next(r for r in rows if r['field'] == 'itemid')['references'], {'fandom': 3153})

    def test_rune_explicit_vocation_cooldown_group_and_range_evidence(self):
        refs = references(page('Cure Poison Rune', itemid='3153', vocrequired='Druid, Elder Druid',
                               cooldown='1', subclass='Healing', cooldowngroup='1', spellrange='7'))
        summary, rows, unmatched = verify(rune(), refs)
        for field in ('itemid', 'vocrequired', 'cooldown', 'subclass', 'cooldowngroup', 'spellrange'):
            self.assertEqual(summary['field_counts'][field], {'agree': 1}, field)
        self.assertFalse(rows)
        self.assertFalse(unmatched)

    def test_broad_item_category_cannot_supply_cooldown_group(self):
        summary, rows, _ = verify(rune(), references(page('Cure Poison Rune', primarytype='Healing Runes')))
        self.assertEqual(summary['field_counts']['subclass'], {'no_source': 1})
        self.assertEqual(summary['field_counts']['cooldowngroup'], {'no_source': 1})
        self.assertFalse(rows)

    def test_new_fields_reveal_mismatches(self):
        refs = references(page('Cure Poison Rune', vocrequired='Sorcerer', cooldown='2',
                               subclass='Support', cooldowngroup='2', spellrange='6'))
        summary, rows, _ = verify(rune(), refs)
        self.assertEqual({r['field'] for r in rows},
                         {'vocrequired', 'cooldown', 'subclass', 'cooldowngroup', 'spellrange'})
        self.assertEqual(summary['ours_differs_ready'], 5)

    def test_absent_range_is_not_zero_or_inferred(self):
        spell = rune()
        spell['targeting'] = {}
        _, rows, _ = verify(spell, references(page('Cure Poison Rune', spellrange='7')))
        self.assertEqual(rows[0]['field'], 'spellrange')
        self.assertIsNone(rows[0]['ours'])
        summary, rows, _ = verify(spell, references(page('Cure Poison Rune')))
        self.assertEqual(summary['field_counts']['spellrange'], {'no_source': 1})
        self.assertFalse(rows)


class PartyManaTests(unittest.TestCase):
    def test_native_fixed_party_cost_is_compared_instead_of_zero_standard_cost(self):
        spell = rune()
        spell.update(carrier='instant', words='utori mas mana')
        spell['execution'] = {'native_behavior': {'key': 'party_buff', 'parameters': {
            'mana': {'mode': 'fixed', 'base': 75}}}}
        refs = vs.References({'fandom': {'pages': [{'template': 'Infobox Spell', 'title': 'Enlighten Party',
                                                  'fields': {'words': spell['words'], 'mana': '75'}}]}})
        summary, rows, _ = verify(spell, refs)
        self.assertEqual(summary['field_counts']['mana'], {'agree': 1})
        self.assertFalse(rows)

    def test_scaled_party_cost_stays_varies(self):
        spell = rune()
        spell['execution'] = {'native_behavior': {'key': 'party_buff', 'parameters': {'mana': {'mode': 'scaled'}}}}
        self.assertEqual(vs.our_spell_values(spell)['mana'], 'varies')


if __name__ == '__main__':
    unittest.main()
