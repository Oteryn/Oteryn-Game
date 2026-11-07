"""Source identity, preservation and honest proof-level regressions; fully offline."""
import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

import jsonschema
import wiki_all_source_inventory as w

SAMPLES = Path(__file__).parent / 'samples/wiki-source-all373'


class AllSourceInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.selection = w.read(SAMPLES / 'selection373.json')
        cls.previous = w.read(SAMPLES / 'previous105.json')
        cls.schema = w.schema(SAMPLES)
        cls.generated = w.generate(SAMPLES)

    def fixture(self):
        old = self.previous['entries'][0]
        old_titles = {e['wiki_title'] for e in self.previous['entries']}
        new = next(r for r in self.selection['quests'] if r['wiki_title'] not in old_titles and
                   any(s['source_fields']['semantic_enrichment']['unparsed_requirements'] for s in r['fresh_sources']))
        selected = [next(r for r in self.selection['quests'] if r['wiki_title'] == old['wiki_title']), new]
        return ({**copy.deepcopy(self.selection), 'quests': copy.deepcopy(selected),
                 'catalogue_title_inventory': [r['wiki_title'] for r in selected]}, {'entries': [copy.deepcopy(old)]})

    def build(self, selected, previous):
        return w.build(selected, previous, w.read(SAMPLES / 'access-checks.json'), [])

    def test_all_pinned_titles_and_original_entries_preserved(self):
        result = self.generated
        self.assertEqual(result['titles'], len(self.selection['catalogue_title_inventory']))
        self.assertEqual(result['titles'], 373)
        by_title = {e['wiki_title']: e for e in result['entries']}
        for old in self.previous['entries']:
            self.assertEqual(old, by_title[old['wiki_title']])
        self.assertEqual(result['summary']['new_structured_source_revisions'], 729)

    def test_existing_source_bindings_do_not_exclude_titles(self):
        self.assertTrue(any(c['source_binding_records'] for c in self.generated['coverage']))
        self.assertEqual({c['wiki_title'] for c in self.generated['coverage']}, set(self.selection['catalogue_title_inventory']))

    def test_source_scopes_and_unparsed_evidence_retained(self):
        selected, previous = self.fixture(); result = self.build(selected, previous)
        added = result['entries'][1]
        self.assertEqual(added['source_fieldsets'], selected['quests'][1]['fresh_sources'])
        holds = [h for h in added['unresolved'] if h.get('evidence')]
        self.assertTrue(holds)
        self.assertTrue(all(h['evidence']['revid'] and h['evidence']['line_sha256'] for h in holds))
        self.assertEqual({s['target_cut'] for s in added['source_fieldsets']},
                         {s['target_cut'] for s in selected['quests'][1]['fresh_sources']})

    def test_thin_fields_never_become_walkthrough_or_native_readiness(self):
        result = self.generated
        self.assertEqual(result['summary']['new_full_body_captures'], 0)
        self.assertTrue(all(not c['full_walkthrough_complete'] and not c['oteryn_behavior_selected'] for c in result['coverage']))
        self.assertEqual(sum(c['proof_level'] == 'PINNED_STRUCTURED_FIELDS_ONLY' for c in result['coverage']), 268)

    def test_duplicate_or_missing_selected_title_rejected(self):
        for duplicate in (True, False):
            selected, previous = self.fixture()
            selected['quests'] = selected['quests'] * 2 if duplicate else selected['quests'][:-1]
            with self.assertRaisesRegex(ValueError, 'selection'):
                self.build(selected, previous)

    def test_missing_revision_and_stale_semantic_digest_rejected(self):
        for missing in (True, False):
            selected, previous = self.fixture(); source = selected['quests'][1]['fresh_sources'][0]
            if missing: source['revid'] = None
            else: source['source_fields']['semantic_enrichment']['semantic_sha256'] = '0' * 64
            with self.assertRaises(ValueError): self.build(selected, previous)

    def test_changed_prior_source_revision_rejected(self):
        selected, previous = self.fixture(); selected['quests'][0]['fresh_sources'][0]['revid'] += 1
        with self.assertRaisesRegex(ValueError, 'Prior detailed source pin'):
            self.build(selected, previous)

    def test_media_and_runtime_promotions_rejected(self):
        for media in (True, False):
            selected, previous = self.fixture(); semantic = selected['quests'][1]['fresh_sources'][0]['source_fields']['semantic_enrichment']
            if media: semantic['reward_entity_references'].append({'entity': 'Image:fixture.gif'})
            else: semantic['runtime_promotion'] = True
            semantic['semantic_sha256'] = w.digest({k: v for k, v in semantic.items() if k != 'semantic_sha256'})
            with self.assertRaises(ValueError): self.build(selected, previous)

    def test_closed_schema_rejects_extra_field_and_complete_claim(self):
        selected, previous = self.fixture(); good = self.build(selected, previous)
        jsonschema.Draft202012Validator(self.schema).validate(good)
        for field, value in (('invented_runtime_behavior', True), ('definition_complete', True)):
            changed = copy.deepcopy(good); changed['entries'][1][field] = value
            with self.assertRaises(jsonschema.ValidationError):
                jsonschema.Draft202012Validator(self.schema).validate(changed)

    def test_tampered_output_digest_summary_rejected(self):
        changed = copy.deepcopy(self.generated); changed['summary']['new_full_body_captures'] = 729
        with self.assertRaisesRegex(ValueError, 'regeneration'): w.validate(changed, SAMPLES)

    def test_source_access_observation_cannot_be_forged(self):
        selected, previous = self.fixture(); changed = self.build(selected, previous)
        changed['source_access_checks']['remote_desktop_browser']['device_online'] = True
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(self.schema).validate(changed)

    def test_portable_relocation_and_input_tamper(self):
        with tempfile.TemporaryDirectory() as temp:
            relocated = Path(temp)
            for name in (*w.FILES, 'inventory-inputs.json'): shutil.copyfile(SAMPLES / name, relocated / name)
            self.assertEqual(w.generate(relocated), self.generated)
            (relocated / 'selection373.json').write_text('{}')
            with self.assertRaisesRegex(ValueError, 'digest'): w.generate(relocated)


if __name__ == '__main__':
    unittest.main()
