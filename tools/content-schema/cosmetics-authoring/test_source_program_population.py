"""Actual maintained-source/materializer regression; no donor downloads or mutation."""
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

ROOT = Path(os.environ['OTERYN_REPO_ROOT']) if 'OTERYN_REPO_ROOT' in os.environ else Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/cosmetics-authoring'))
import cosmetics_authoring as subject


def verify_programme_metadata(root, document):
    raw = (root / 'content/cosmetics/npc-acquisitions.json').read_bytes()
    programme = json.loads(raw)
    expected = json.loads((root / 'tools/content-schema/cosmetics-authoring/srv7-source-program-grants.json').read_bytes())['records']
    qualified = {grant['key'] for npc in programme['npcs'] for transition in npc['transitions'] for grant in transition['grants']}
    if qualified != {mapping['grant_key'] for mapping in expected}:
        raise ValueError('programme source mappings differ')
    actual = {mapping['grant_key']: mapping for mapping in document['grant_mappings']}
    for mapping in expected:
        if actual.get(mapping['grant_key']) != mapping:
            raise ValueError('missing or changed source mapping: ' + mapping['grant_key'])
    digest = hashlib.sha256(raw).hexdigest()
    affected = {grant['outfit_key'] for npc in programme['npcs'] for transition in npc['transitions'] for grant in transition['grants']}
    outfits = {outfit['key']: outfit for outfit in document['outfits']}
    for key in affected:
        citations = [source for source in outfits[key]['unlock_sources'] if source['reference'] == 'content/cosmetics/npc-acquisitions.json']
        if len(citations) != 1 or citations[0]['sha256'] != digest:
            raise ValueError('source programme citation is missing or stale: ' + key)
    for outfit in outfits.values():
        if any(source['sha256'] != digest for source in outfit['unlock_sources'] if source['reference'] == 'content/cosmetics/npc-acquisitions.json'):
            raise ValueError('conflicting programme citation digest')
    mounts = {mount['key']: mount for mount in document['mounts']}
    for filename in ['horse-rental-cosmetics.json', 'npc-permanent-mount-cosmetics.json']:
        metadata = json.loads((root / 'tools/content-schema/cosmetics-authoring' / filename).read_bytes())
        for mount in metadata['mounts']:
            if mounts.get(mount['key']) != mount:
                raise ValueError('source mount metadata missing or changed: ' + mount['key'])
        for mapping in metadata['grant_mappings']:
            if actual.get(mapping['grant_key']) != mapping:
                raise ValueError('source mount mapping missing or changed: ' + mapping['grant_key'])


class SourceProgrammePopulation(unittest.TestCase):
    def test_actual_materialized_native_section_has_every_source_mapping_and_digest(self):
        authoring = json.loads((ROOT / subject.AUTHORING).read_bytes())
        section = json.loads(subject.build(authoring, ROOT / subject.MANIFEST))
        verify_programme_metadata(ROOT, section['authoring']['document'])

    def test_normal_tree_first_materialization_preserves_canonical_authoring_and_source_features(self):
        before = (ROOT / subject.AUTHORING).read_bytes()
        first = subject.materialized_outputs(ROOT)
        second = subject.materialized_outputs(ROOT)
        self.assertEqual(first, second)
        self.assertEqual((ROOT / subject.AUTHORING).read_bytes(), before)
        pin = json.loads(first[subject.OUTPUT])
        self.assertEqual(pin['authoring']['sha256'], hashlib.sha256(before).hexdigest())
        self.assertEqual((ROOT / subject.OUTPUT).read_bytes(), first[subject.OUTPUT], 'active native provisioning must be regenerated after maintained population')
        verify_programme_metadata(ROOT, json.loads(before)['document'])

    def test_unpopulated_mapping_or_stale_programme_citation_is_rejected(self):
        document = json.loads((ROOT / subject.AUTHORING).read_bytes())['document']
        expected = json.loads((ROOT / 'tools/content-schema/cosmetics-authoring/srv7-source-program-grants.json').read_bytes())['records']
        missing = json.loads(json.dumps(document))
        missing['grant_mappings'] = [mapping for mapping in missing['grant_mappings'] if mapping['grant_key'] != expected[0]['grant_key']]
        with self.assertRaisesRegex(ValueError, 'missing or changed source mapping'):
            verify_programme_metadata(ROOT, missing)
        stale = json.loads(json.dumps(document))
        for outfit in stale['outfits']:
            for source in outfit['unlock_sources']:
                if source['reference'] == 'content/cosmetics/npc-acquisitions.json':
                    source['sha256'] = '00' * 32
        with self.assertRaisesRegex(ValueError, 'citation'):
            verify_programme_metadata(ROOT, stale)


if __name__ == '__main__':
    unittest.main()
