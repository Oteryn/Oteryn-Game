import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('optional_fields', Path(__file__).with_name('complete_optional_wiki_fields.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class OptionalFieldsTests(unittest.TestCase):
    def setUp(self):
        self.page = {'page_title': 'Own Boss', 'url': 'https://tibiawiki.com.br/wiki/Own_Boss',
                     'revision_id': 5, 'content_sha256': 'a' * 64, 'method': 'actual_capture',
                     'fields': {'bosstype': 'nemesis'}, 'field_lines': {'bosstype': 7}}
        self.doc = {'creature': {'stats': {'mitigation_percent': {'numerator': 2, 'denominator': 1}},
                                'summoning': {'summonable': False, 'convinceable': False, 'is_familiar': False}}}

    def test_complete_bosstiary_from_positive_category(self):
        row = m.patch_actor('own_boss', self.doc, self.page, self.page)[0]
        self.assertFalse(row['expected_present'])
        self.assertEqual(row['value']['mastery_kills'], 5)
        self.assertEqual(row['value']['mastery_points'], 60)
        self.assertEqual(row['source']['fields']['bosstype']['raw'], 'nemesis')
        self.assertEqual(row['supporting_sources'][0]['source_line'], 17)

    def test_stars_encode_project_enum_without_claiming_icon_observation(self):
        self.page['fields']['dificuldade'] = 'trivial'
        self.doc['creature']['bestiary'] = {'difficulty': 'trivial'}
        rows = m.patch_actor('actor', self.doc, self.page, self.page)
        stars = next(r for r in rows if r['pointer'].endswith('/stars'))
        self.assertEqual(stars['value'], 1)
        self.assertEqual(stars['supporting_sources'][0]['source_line'], 240)
        self.assertIn('not an independently observed Global', stars['reason'])

    def test_shared_page_variant_never_inherits(self):
        self.assertEqual(m.patch_actor('own_boss_2', self.doc, self.page, self.page, variant=True), [])

    def test_empty_category_and_existing_record_not_overwritten(self):
        self.page['fields']['bosstype'] = ''
        self.assertEqual(m.patch_actor('boss', self.doc, self.page, self.page), [])
        self.page['fields']['bosstype'] = 'bane'
        self.doc['creature']['bosstiary'] = {'category': 'archfoe'}
        self.assertEqual(m.patch_actor('boss', self.doc, self.page, self.page), [])

    def test_typed_na_not_claimed_global(self):
        rows = {r['field']: r for r in m.assess(self.doc, self.page, False, False)}
        self.assertEqual(rows['summoning.mana_cost']['status'], 'NA_PREPARED_CONTRACT')
        self.assertEqual(rows['bosstiary']['status'], 'UNAVAILABLE')
        self.assertEqual(rows['name_forms.plural']['status'], 'UNAVAILABLE')

    def test_familiar_dynamic_speed_not_fixed_guess(self):
        self.doc['creature']['summoning']['is_familiar'] = True
        rows = {r['field']: r for r in m.assess(self.doc, self.page, False, False)}
        self.assertEqual(rows['summoning.familiar.owner_speed_bonus']['status'], 'UNREPRESENTABLE_DYNAMIC_SOURCE')
        self.assertNotIn('familiar', self.doc['creature']['summoning'])


if __name__ == '__main__':
    unittest.main()
