import copy
import json
from pathlib import Path
import tempfile
import unittest

import apply_monster_field_patches as patcher


class FieldPatchTests(unittest.TestCase):
    def test_append_identity_conflict_and_dedup(self):
        value = {'identity': {'key': 'x', 'revision': 'r'}, 'kind': 'spell'}
        doc = {'abilities': [copy.deepcopy(value)]}
        self.assertEqual(patcher.assign(doc, '/abilities/-', value), '/abilities/0')
        self.assertEqual(len(doc['abilities']), 1)
        with self.assertRaises(ValueError):
            patcher.assign(doc, '/abilities/-', {**value, 'kind': 'melee'})

    def test_superseded_provenance_preserves_original_history(self):
        manifest = {'sources': [{'repository': 'a/b', 'revision': '1' * 40}], 'entries': [
            {'destination': '/monster/creature/stats/max_health', 'status': 'mapped', 'resolution': 'old'},
            {'destination': '/monster/creature/stats/armor', 'status': 'mapped', 'resolution': 'other'}]}
        patch = {'file': 'monster.json', 'pointer': '/creature/stats/max_health', 'source': {'source_line': 9, 'source_file': 'monster.lua'}, 'reason': 'verified'}
        patcher.extend_manifest(manifest, patch, manifest['sources'][0], patch['pointer'], 'stats')
        self.assertEqual(manifest['entries'][0]['status'], 'metadata_only')
        self.assertEqual(manifest['entries'][0]['destination'], '/monster/creature/stats/max_health')
        self.assertIn('Previous: old', manifest['entries'][0]['resolution'])
        self.assertEqual(manifest['entries'][1]['status'], 'mapped')

    def test_wiki_source_requires_exact_capture(self):
        source = {'url': 'https://tibiawiki.com.br/wiki/Cat', 'revision_id': 7, 'content_sha256': 'a' * 64}
        page = {'page_title': 'Cat', 'url': source['url'], 'page_id': 3, 'revision_id': 7, 'content_sha256': 'a' * 64}
        with self.assertRaises(ValueError):
            patcher.manifest_source(source, {}, 'b' * 64)
        result = patcher.manifest_source(source, {(7, 'a' * 64): page}, 'b' * 64)
        self.assertEqual(result['page_id'], 3)
        with self.assertRaises(ValueError):
            patcher.manifest_source({**source, 'url': 'https://tibiawiki.com.br/wiki/Dog'}, {(7, 'a' * 64): page}, 'b' * 64)

    def test_wiki_field_line_is_resolved_from_actual_capture(self):
        source = {'url': 'https://tibiawiki.com.br/wiki/Cat', 'revision_id': 7,
                  'content_sha256': 'a' * 64, 'source_field': 'pushobjects'}
        page = {'page_title': 'Cat', 'url': source['url'], 'page_id': 3,
                'revision_id': 7, 'content_sha256': 'a' * 64, 'field_lines': {'pushobjects': 17}}
        corpus = {(7, 'a' * 64): page}
        resolved = patcher.manifest_source(source, corpus, 'b' * 64)
        details = patcher.verified_source_details(source, resolved, corpus, 'patch.json')
        self.assertEqual(details['source_line'], 17)
        self.assertEqual(details['source_file'], 'Cat')
        with self.assertRaisesRegex(ValueError, 'verified source line'):
            patcher.verified_source_details({**source, 'source_field': 'invented'}, resolved, corpus, 'patch.json')

    def fixture(self, root):
        base = root / 'baseline'
        bundle = base / 'bundles/cat'
        docs = {'monster.json': {'creature': {'stats': {'max_health': 10}}}, 'dependencies.json': {'abilities': []}, 'catalog.json': {'definitions': []}, 'manifest.json': {'sources': [{'repository': 'a/b', 'revision': '1' * 40}], 'entries': []}}
        for name, value in docs.items():
            patcher.write(bundle / name, value)
        patcher.write(base / 'population-index.json', {'monsters': [{'monster': 'cat', 'sha256': patcher.population.admission.bundle_digest(bundle), 'completion_flags': []}], 'bundles': 1})
        patcher.write(base / 'completion-quality.json', {'baseline': True})
        (base / 'encounters').mkdir()
        packet = {'lane': 'stats', 'baseline_index_sha256': patcher.sha(base / 'population-index.json'), 'patches': [{'monster': 'cat', 'file': 'monster.json', 'pointer': '/creature/stats/max_health', 'expected_present': True, 'expected_value': 10, 'value': 20, 'source': {'repository': 'a/b', 'revision': '1' * 40}, 'reason': 'source correction'}], 'actor_flags': {'cat': ['UNVERIFIED_RUNTIME']}}
        path = root / 'packet/patch.json'
        patcher.write(path, packet)
        return base, path, packet

    def test_copy_on_write_preserves_baseline_inode_and_bytes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            base, packet, _ = self.fixture(root)
            before = (base / 'bundles/cat/monster.json').read_bytes()
            output = root / 'output'
            patcher.compose(base, [packet], output, validate=False)
            self.assertEqual((base / 'bundles/cat/monster.json').read_bytes(), before)
            self.assertNotEqual((base / 'bundles/cat/monster.json').stat().st_ino, (output / 'bundles/cat/monster.json').stat().st_ino)
            self.assertEqual((base / 'bundles/cat/dependencies.json').stat().st_ino, (output / 'bundles/cat/dependencies.json').stat().st_ino)
            self.assertEqual(patcher.read(output / 'population-index.json')['monsters'][0]['completion_flags'], ['UNVERIFIED_RUNTIME'])

    def test_wrong_expected_value_and_writer_conflicts_fail_before_output(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            base, path, packet = self.fixture(root)
            packet['patches'][0]['expected_value'] = 12
            path.write_text(json.dumps(packet))
            with self.assertRaisesRegex(ValueError, 'expectation'):
                patcher.compose(base, [path], root / 'output', validate=False)
            self.assertFalse((root / 'output').exists())
            packet['patches'][0]['expected_value'] = 10
            path.write_text(json.dumps(packet))
            second = copy.deepcopy(packet)
            second['patches'][0]['value'] = 30
            other = root / 'packet/other.json'
            patcher.write(other, second)
            with self.assertRaisesRegex(ValueError, 'conflicting'):
                patcher.compose(base, [path, other], root / 'output', validate=False)
            self.assertFalse((root / 'output').exists())

    def test_output_overlap_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            base, path, _ = self.fixture(root)
            with self.assertRaisesRegex(ValueError, 'overlaps'):
                patcher.compose(base, [path], base / 'new', validate=False)

    def test_unbound_core_stays_flagged_without_inventing_an_item(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            base, path, packet = self.fixture(root)
            packet.update(lane='soul-core', patches=[], unresolved=[{'monster': 'cat'}])
            path.write_text(json.dumps(packet))
            output = root / 'output'
            patcher.compose(base, [path], output, validate=False)
            self.assertEqual((base / 'bundles/cat/monster.json').read_bytes(),
                             (output / 'bundles/cat/monster.json').read_bytes())
            flags = patcher.read(output / 'population-index.json')['monsters'][0]['completion_flags']
            self.assertIn('SOUL_CORE_ITEM_BINDING_PENDING', flags)


if __name__ == '__main__':
    unittest.main()
