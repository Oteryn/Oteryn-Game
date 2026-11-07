"""Bounded, attributed supplementary rune-range evidence without changing S13 precedence."""
import json
import unittest

import convert_spells as cs
from jsonschema import Draft202012Validator


DESTINATION = '/spell/spell/targeting/range_tiles'


def page(name='Sudden Death Rune', **fields):
    return {'template': 'Infobox Object', 'title': name, 'url': 'https://tibiopedia.pl/spells/Sudden_Death_Rune',
            'content_sha256': 'a' * 64, 'page_id': 1, 'revision_id': 2, 'timestamp': '2026-09-27T00:00:00Z',
            'fields': {'name': name, **fields}}


def doc(*pages):
    return {'pages': list(pages), 'target_cut': '2026-09-28', 'api': 'https://example.org/api.php'}


def wikis(tibio=None, fandom=None, br=None, official=None, tibiacom=None):
    return cs.Wikis(doc(*(fandom or [])), doc(*(br or [])), {'changes': official or []},
                    doc(*(tibio or [])), doc(*(tibiacom or [])) if tibiacom is not None else None)


def records():
    return {source: {'spell_type': 'rune', 'name': 'sudden death rune', 'file': 'runes/sudden_death.lua',
                     'registrar': {'runeId': 3155}} for source in ('canary', 'crystal')}


def bundle(refs, source_records=None):
    source_records = source_records or records()
    return cs.Bundle('sudden death rune', source_records, refs, {},
                     {(s, r['file']): '-- source\n' for s, r in source_records.items()})


def range_field(author, pages=None):
    return author.field(DESTINATION, 'spellrange', 'range', pages or {}, required=False)


class WikiGapCompletionTests(unittest.TestCase):
    def test_real_sudden_death_capture_is_range_seven_and_attributed(self):
        refs = cs.Wikis(json.loads(cs.FANDOM_FACTS.read_text()), json.loads(cs.BR_FACTS.read_text()),
                        json.loads(cs.OFFICIAL.read_text()), json.loads(cs.TIBIOPEDIA_FACTS.read_text()))
        census = json.loads(cs.CENSUS.read_text())
        real = {source: next(r for r in census[source] if r['name'] == 'sudden death rune'
                             and r['spell_type'] == 'rune') for source in ('canary', 'crystal')}
        author = bundle(refs, real)
        self.assertEqual(range_field(author), 7)
        capture = refs.rune_page('tibiopedia', real['canary'])
        self.assertEqual(author.sources, [{'kind': 'community_capture', 'url': capture['url'],
                                          'title': capture['title'], 'captured': '2026-09-28',
                                          'content_sha256': capture['content_sha256']}])
        self.assertEqual(author.rows[0]['destination'], DESTINATION)
        self.assertEqual(author.rows[0]['source_field'], 'spellrange')
        self.assertIn('gap-fill', author.rows[0]['resolution'])
        manifest_schema = json.loads((cs.MONSTER / 'monster-import-readiness.schema.json').read_text())
        self.assertEqual(list(Draft202012Validator(manifest_schema).iter_errors(
            {'sources': author.sources, 'entries': author.rows})), [])

    def test_primary_fandom_range_has_precedence(self):
        primary = page(spellrange='6')
        author = bundle(wikis(tibio=[page(spellrange='7')], fandom=[primary]))
        self.assertEqual(range_field(author, {'fandom': primary}), 6)
        self.assertEqual(author.sources[0]['kind'], 'mediawiki')

    def test_existing_single_engine_range_is_never_overwritten(self):
        for source, existing in [('canary', 6), ('crystal', 0)]:
            with self.subTest(source=source, existing=existing):
                rs = records()
                rs[source]['registrar']['range'] = existing
                author = bundle(wikis(tibio=[page(spellrange='7')]), rs)
                self.assertEqual(range_field(author), existing)
                self.assertNotIn('community_capture', {s.get('kind') for s in author.sources})

    def test_engine_conflict_keeps_canary_precedence(self):
        rs = records()
        rs['canary']['registrar']['range'] = 6
        rs['crystal']['registrar']['range'] = 8
        author = bundle(wikis(tibio=[page(spellrange='7')]), rs)
        self.assertEqual(range_field(author), 6)
        self.assertIn('S21', author.rows[0]['resolution'])

    def test_official_change_has_precedence_with_no_primary_page(self):
        change = {'spell': 'sudden death rune', 'field': 'spellrange', 'value': '9',
                  'date': '2026-09-27', 'fact': 'range changed', 'source': 'https://www.tibia.com/news'}
        author = bundle(wikis(tibio=[page(spellrange='7')], official=[change]))
        self.assertEqual(range_field(author), 9)
        self.assertIn('S24', author.rows[0]['resolution'])

    def test_official_library_has_precedence(self):
        official = page(spellrange='5')
        author = bundle(wikis(tibio=[page(spellrange='7')], tibiacom=[official]))
        self.assertEqual(range_field(author, {'tibiacom': official}), 5)
        self.assertEqual(author.sources[0]['kind'], 'official_capture')

    def test_s13_conflict_resolution_is_unchanged(self):
        fandom, br, tibio = page(spellrange='6'), page(spellrange='8'), page(spellrange='6')
        author = bundle(wikis(tibio=[tibio], fandom=[fandom], br=[br]))
        self.assertEqual(range_field(author, {'fandom': fandom, 'br': br, 'tibiopedia': tibio}), 6)
        self.assertIn('S13', author.rows[0]['resolution'])
        self.assertNotIn('community_capture', {s.get('kind') for s in author.sources})

    def test_only_rune_cast_range_is_supplemented(self):
        rs = records()
        for record in rs.values():
            record['spell_type'] = 'instant'
        self.assertIsNone(range_field(bundle(wikis(tibio=[page(spellrange='7')]), rs)))
        author = bundle(wikis(tibio=[page(spellrange='7', mana='75')]))
        self.assertIsNone(author.field('/spell/spell/costs/mana', 'mana', None, {}, required=False))
        self.assertIsNone(author.field('/other/range_tiles', 'spellrange', 'range', {}, required=False))

    def test_both_engine_records_are_required_and_identity_conflicts_rejected(self):
        rs = records()
        del rs['crystal']
        self.assertIsNone(range_field(bundle(wikis(tibio=[page(spellrange='7')]), rs)))
        rs = records()
        rs['crystal']['registrar']['runeId'] = 999
        self.assertIsNone(range_field(bundle(wikis(tibio=[page(spellrange='7')]), rs)))

    def test_rejects_malformed_range_and_area_size(self):
        for raw in (True, False, -1, 2 ** 32, '-1', '7.0', '7 tiles', '3x3', '?', None, 7.0):
            with self.subTest(raw=raw):
                author = bundle(wikis(tibio=[page(spellrange=raw, area='7')]))
                self.assertIsNone(range_field(author))
                self.assertFalse(author.sources)
        for raw in ('0', 0, str(2 ** 32 - 1)):
            with self.subTest(valid=raw):
                self.assertEqual(range_field(bundle(wikis(tibio=[page(spellrange=raw)]))), int(raw))

    def test_ambiguous_capture_is_not_selected(self):
        author = bundle(wikis(tibio=[page(spellrange='7'), page(spellrange='8')]))
        self.assertIsNone(range_field(author))
        self.assertFalse(author.sources)

    def test_rune_alias_and_actualname_matching(self):
        for source_name, wiki_name in [('antidote rune', 'Cure Poison Rune (Item)'),
                                      ('desintegrate rune', 'Disintegrate Rune'),
                                      ('energybomb rune', 'Energy Bomb Rune'),
                                      ('firebomb rune', 'Fire Bomb Rune'), ('paralyze rune', 'Paralyse Rune')]:
            with self.subTest(source_name=source_name):
                rs = records()
                for record in rs.values():
                    record['name'] = source_name
                captured = page(wiki_name, spellrange='7')
                self.assertEqual(range_field(bundle(wikis(tibio=[captured]), rs)), 7)
        captured = page('Translated Name', actualname='sudden death rune', spellrange='7')
        self.assertEqual(range_field(bundle(wikis(tibio=[captured]))), 7)

    def test_unique_item_id_wins_and_conflicting_or_duplicate_ids_are_rejected(self):
        captured = page('Different Name', itemid='3155', spellrange='7')
        self.assertEqual(range_field(bundle(wikis(tibio=[captured]))), 7)
        conflict = page(itemid='999', spellrange='7')
        self.assertIsNone(range_field(bundle(wikis(tibio=[conflict]))))
        duplicate = page('Another Name', itemid='3155', spellrange='8')
        self.assertIsNone(range_field(bundle(wikis(tibio=[captured, duplicate]))))


if __name__ == '__main__':
    unittest.main()
