"""Source ownership, closed-schema and original-core regression controls."""
import copy
import hashlib
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import jsonschema

import quest_donor_attachment as tool


class DonorAttachmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        here = Path(__file__).parent
        cls.base_schema = tool.read(here / 'quest_completion.schema.json')
        cls.schema = tool.read(here / 'quest_donor_attachment.schema.json')
        baseline = {}
        for path in tool.read(cls.root / 'content/quests/definitions/index.json')['shards']:
            for row in tool.read(cls.root / path)['records']:
                d = row['definition']
                baseline[d['identity']['key']] = d
        cls.definition = copy.deepcopy(baseline['oteryn:quest.the_secret_library_quest'])
        cls.definition['oteryn_recipe'].pop('donor_source_data', None)

    def test_schema_extension_preserves_every_original_constraint(self):
        extended = tool.derived_completion_schema(self.base_schema)
        extended['$id'] = self.base_schema['$id']
        del extended['$defs']['supplement']['properties']['donor_source_data']
        chosen = extended['$defs']['supplement']['properties'].pop('chosen_journal_corrections')
        self.assertEqual(chosen, tool.read(Path(__file__).with_name(
            'quest_chosen_journal_attachment.schema.json')))
        self.assertIs(chosen['additionalProperties'], False)
        self.assertIs(chosen['properties']['runtime_enabled']['const'], False)
        self.assertIs(chosen['properties']['source_holds_preserved']['const'], True)
        self.assertNotIn('chosen_journal_corrections',
                         extended['$defs']['supplement']['required'])
        self.assertEqual(extended, self.base_schema)

    def test_attachment_preserves_source_core_recipe_payload_and_holds(self):
        before = copy.deepcopy(self.definition)
        after = tool.attach(self.root, [{'definition': before}])[0]['definition']
        self.assertIn('donor_source_data', after['oteryn_recipe'])
        del after['oteryn_recipe']['donor_source_data']
        self.assertEqual(after, before)

    def test_references_resolve_exact_packet_and_record(self):
        after = tool.attach(self.root, [{'definition': self.definition}])[0]['definition']
        supplement = after['oteryn_recipe']['donor_source_data']
        jsonschema.Draft202012Validator(self.schema).validate(supplement)
        for ref in supplement['refs']:
            path = self.root / ref['path']
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), ref['packet_sha256'])
            value = tool.pointer(tool.read(path), ref['json_pointer'])
            self.assertEqual(tool.digest(value), ref['record_sha256'])

    def test_native_and_complete_promotions_and_unknown_fields_rejected(self):
        supplement = tool.attach(self.root, [{'definition': self.definition}])[0]['definition']['oteryn_recipe']['donor_source_data']
        validator = jsonschema.Draft202012Validator(self.schema)
        for field in ('native_admission', 'whole_quest_complete'):
            changed = copy.deepcopy(supplement)
            changed[field] = True
            with self.assertRaises(jsonschema.ValidationError):
                validator.validate(changed)
        changed = dict(supplement, fabricated=True)
        with self.assertRaises(jsonschema.ValidationError):
            validator.validate(changed)

    def test_equal_interaction_key_with_other_graph_variant_is_rejected(self):
        data = copy.deepcopy(self.definition['source_data'])
        keys = {r['interaction'] for r in tool.read(self.root / (tool.DATA + 'semantic-conditions.json'))['conditions']}
        selected = next(g for g in data['interactions'] if g['identity']['key'] in keys)
        selected['rules'] = []
        with self.assertRaisesRegex(ValueError, 'another Source graph variant'):
            tool.semantic_refs(self.root, data)

    def test_quest_with_no_owned_graph_gets_no_foreign_condition_or_reward(self):
        self.assertEqual(tool.semantic_refs(self.root, {}), [])

    def test_changed_condition_packet_cannot_reuse_qualification(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in tool.INPUTS:
                dest = root / (tool.DATA + name)
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(self.root / (tool.DATA + name), dest)
            path = root / (tool.DATA + 'semantic-conditions.json')
            path.write_bytes(path.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'qualification digest differs'):
                tool.semantic_refs(root, self.definition['source_data'])

    def test_changed_reward_packet_cannot_reuse_qualification(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in [tool.DATA + name for name in tool.INPUTS] + [tool.DIRECTORY + 'samples/interactions/interactions.json']:
                dest = root / path
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(self.root / path, dest)
            path = root / (tool.DATA + 'semantic-rewards.json')
            path.write_bytes(path.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'reward qualification digest differs'):
                tool.semantic_refs(root, self.definition['source_data'])

    def test_modified_output_rejected_against_existing_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = tool.DATA + 'refinements/joins.json'
            summary = tool.DATA + 'refinements/summary.json'
            dest = root / path
            dest.parent.mkdir(parents=True)
            dest.write_text('{"records": []}')
            (root / summary).write_text(json.dumps({'outputs': [{'path': 'joins.json',
                'sha256': hashlib.sha256(dest.read_bytes()).hexdigest()}]}))
            tool.verified_output(root, path, summary, tool.DATA + 'refinements/', {})
            dest.write_text('{"records": [{"unqualified": true}]}')
            with self.assertRaisesRegex(ValueError, 'qualification digest differs'):
                tool.verified_output(root, path, summary, tool.DATA + 'refinements/', {})

    def test_operational_specs_preserve_exact_partial_baselines(self):
        conditions = tool.read(self.root / (tool.DATA + 'refinements/conditions.json'))
        partials = {tool.digest(r): r for r in conditions['records']
                    if r['status'] == 'PARTIAL_SOURCE_GUARD'}
        specs = tool.read(self.root / (tool.DATA + 'refinements/guard-specs.json'))
        self.assertEqual(len(specs['records']), 125)
        self.assertEqual({r['baseline_record_sha256'] for r in specs['records']}, set(partials))
        for record in specs['records']:
            base = partials[record['baseline_record_sha256']]
            for field in ('interaction', 'quest_keys', 'baseline_gap', 'provenance', 'source_ref'):
                self.assertEqual(record[field], base[field])
            self.assertEqual(record['status'], 'SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN')
            self.assertFalse(record['native_admission'])
            self.assertFalse(record['runtime_activation'])

    def test_operational_spec_with_substituted_source_witness_is_rejected(self):
        original = tool.verified_output

        def substitute(root, path, summary, prefix, cache):
            value = original(root, path, summary, prefix, cache)
            if path.endswith('/guard-specs.json'):
                value = copy.deepcopy(value)
                value['records'][0]['source_ref']['expression'] = 'fabricated'
            return value

        with patch.object(tool, 'verified_output', side_effect=substitute):
            with self.assertRaisesRegex(ValueError, 'Source witness differs'):
                tool.refinement_refs(self.root, self.definition, {})


if __name__ == '__main__':
    unittest.main()
