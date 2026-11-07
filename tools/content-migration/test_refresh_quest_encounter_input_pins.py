"""Derived pin refresh accepts only exact admitted facts, never unrelated shard changes."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import quest_encounter_item_semantics as encounter
import refresh_quest_encounter_input_pins as refresh

class DerivedPinTests(unittest.TestCase):
    def fixture(self, root):
        source = encounter.ROOT / encounter.PATH
        target = root / encounter.PATH
        target.parent.mkdir(parents=True)
        target.write_bytes(source.read_bytes())
        legacy = root / 'content/world/definitions/reference.json'
        legacy.parent.mkdir(parents=True)
        legacy.write_text(json.dumps({'records':list(encounter.legacy_definitions().values())}))
        path = Path('content/items/definitions/items-29500-29999.json')
        document = json.loads((encounter.ROOT / path).read_bytes())
        original = copy.deepcopy(document)
        for row in original['records']:
            definition = row['definition']
            if definition['identity']['key'] in encounter.profiles():
                row['definition'] = {name: copy.deepcopy(definition[name])
                                     for name in ('identity','kind','client_projection')}
                row['definition'].update(materializable=False, stack_class='Unknown')
        before = json.dumps(original).encode()
        all_definitions = [r['definition'] for p in (encounter.ROOT / 'content/items/definitions').glob('items-*.json')
                           for r in json.loads(p.read_bytes())['records']]
        indexed = {d['identity']['key']:d for d in all_definitions}
        for row in document['records']:
            if row['definition']['identity']['key'] in encounter.profiles():
                row['definition'] = copy.deepcopy(indexed[row['definition']['identity']['key']])
        (root / path).parent.mkdir(parents=True)
        (root / path).write_text(json.dumps(document))
        capture = root / 'tools/content-schema/native-gameplay/fixture.json'
        encode = lambda value: json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()
        original_demon = next(r['definition'] for r in original['records'] if r['definition']['identity']['key']=='oteryn:item.tibia.i5995')
        capture.write_text(json.dumps({'canonical_inputs':[{'path':path.as_posix(),'sha256':hashlib.sha256(before).hexdigest()},
                                       {'path':path.as_posix(),'key':'oteryn:item.tibia.i5995','definition_sha256':hashlib.sha256(encode(original_demon)).hexdigest()}],
                                       'sources':[{'path':'donor.xml','sha256':'preserved'}], 'records':[{'native':'preserved'}]}))
        return path, before, document, capture

    def test_only_derived_hash_changes_and_no_files_are_written_by_preparation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path, before, _, capture = self.fixture(root)
            original = capture.read_bytes()
            with patch.object(refresh.subprocess,'check_output',return_value=before):
                outputs = refresh.outputs(root=root)
            result = json.loads(outputs[capture])
            self.assertEqual(capture.read_bytes(),original)
            self.assertEqual(result['sources'],json.loads(original)['sources'])
            self.assertEqual(result['records'],json.loads(original)['records'])
            self.assertEqual(result['canonical_inputs'][0]['sha256'],hashlib.sha256((root/path).read_bytes()).hexdigest())
            demon = next(r['definition'] for r in json.loads((root/path).read_bytes())['records'] if r['definition']['identity']['key']=='oteryn:item.tibia.i5995')
            encoded = json.dumps(demon,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()
            self.assertEqual(result['canonical_inputs'][1]['definition_sha256'],hashlib.sha256(encoded).hexdigest())

    def test_newhaven_refresh_is_closed_to_the_exact_three_item_projection(self):
        import quest_finish_newhaven_items as newhaven
        root_source = encounter.ROOT
        reference = json.loads((root_source / 'content/world/definitions/reference.json').read_bytes())
        ring = next(d for d in reference['records'] if d['identity']['key'] == 'oteryn:item.tibia.i52732')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'content/items/definitions/items-00000-00499.json'
            path.parent.mkdir(parents=True)
            original = {'records': [{'definition': copy.deepcopy(ring)}]}
            before = json.dumps(original).encode()
            projected = copy.deepcopy(original)
            newhaven.apply_admissions([projected['records'][0]['definition']], root)
            path.write_text(json.dumps(projected))
            with patch.object(refresh.subprocess, 'check_output', return_value=before), \
                 patch.object(encounter, 'profiles', return_value={}), \
                 patch.object(encounter, 'legacy_definitions', return_value={}):
                self.assertEqual(refresh.outputs(root=root, newhaven=True), {})
                projected['records'][0]['definition']['identity']['revision'] = 'unrelated'
                path.write_text(json.dumps(projected))
                with self.assertRaisesRegex(ValueError, 'UNRELATED_CHANGE'):
                    refresh.outputs(root=root, newhaven=True)

    def test_unrelated_metadata_or_item_change_refuses_the_complete_refresh(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path, before, document, capture = self.fixture(root)
            document['unrelated'] = 'must not be hidden by pin refresh'
            (root/path).write_text(json.dumps(document))
            original = capture.read_bytes()
            with patch.object(refresh.subprocess,'check_output',return_value=before):
                with self.assertRaisesRegex(ValueError,'UNRELATED_CHANGE'):
                    refresh.outputs(root=root)
            self.assertEqual(capture.read_bytes(),original)

if __name__ == '__main__':
    unittest.main()
