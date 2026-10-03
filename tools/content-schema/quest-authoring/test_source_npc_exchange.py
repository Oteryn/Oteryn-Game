import copy
import hashlib
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch
import jsonschema
from referencing import Registry, Resource
import source_npc_exchange as npc


class ExchangeTests(unittest.TestCase):
    def setUp(self):
        local = Path(npc.__file__).parent
        root = Path(os.environ.get('OTERYN_QUEST_TOOL_ROOT', local))
        schemas = [json.loads((p / name).read_text()) for p, name in [
            (root, 'quest_content.schema.json'), (root, 'interaction.schema.json'),
            (local, 'quest_progress.schema.json')]]
        self.schema = schemas[-1]
        registry = Registry().with_resources((s['$id'], Resource.from_contents(s)) for s in schemas)
        self.progress_validator = jsonschema.Draft202012Validator(schemas[-1], registry=registry)
        self.validator = jsonschema.Draft202012Validator(
            {'$ref': self.schema['$id'] + '#/$defs/source_exchange'}, registry=registry)
        self.profiles = {s: self.schema['$defs']['pemaret_' + s]['const'] for s in npc.PINS}

    def test_exact_profiles_and_reconstruction(self):
        for profile in self.profiles.values():
            self.validator.validate(profile)
            branch = profile['rules'][0]['branch'][0]
            self.assertIn({'quest_stage': {'progress': npc.TRACK, 'op': '<', 'value': 1},
                           'negate': False}, branch['when']['all'])
            self.assertEqual(branch['then'][0]['request'], 'consume')
            inner = branch['then'][1]['branch'][0]
            self.assertIn('unresolved', inner['when'])
            self.assertEqual([x['request'] for x in inner['then']], ['hand_out', 'set_progress'])
        self.assertEqual(npc.schema_update(copy.deepcopy(self.schema), self.profiles), self.schema)

    def test_profiles_cannot_promote_or_rewrite(self):
        for field, value in [('callback_complete', True), ('runtime_readiness', 'READY'),
                             ('remaining_holds', []), ('evidence', [])]:
            with self.subTest(field=field):
                mutant = copy.deepcopy(self.profiles['canary']); mutant[field] = value
                self.assertTrue(list(self.validator.iter_errors(mutant)))
        mutant = copy.deepcopy(self.profiles['canary'])
        mutant['rules'][0]['branch'][0]['then'][0]['item']['key'] = 'canary:item/902'
        self.assertTrue(list(self.validator.iter_errors(mutant)))

    def test_changed_blob_and_unknown_script(self):
        for server, (path, _, _) in npc.PINS.items():
            with self.assertRaisesRegex(ValueError, 'blob changed'):
                npc.transcribe(server, path, b'changed actor or reward', {})
            self.assertIsNone(npc.transcribe(server, 'npc/other.lua', b'', {}))

    def test_write_binding_and_boolean_forgeries(self):
        # Synthetic non-prose fixture tests the scanner handshake, not a donor pin.
        raw = b'line\n' * 90
        server = 'canary'; path, _, start = npc.PINS[server]
        write = {'line': start + 19, 'to': 1, 'owner': 'npc', 'target': npc.TARGET,
                 'dialogue': {'keywords': ['yes'], 'topics': [1]}}
        with patch.dict(npc.PINS, {server: (path, hashlib.sha256(raw).hexdigest(), start)}):
            self.assertEqual(npc.transcribe(server, path, raw, write)['remaining_holds'], npc.HOLDS)
            for field, value in [('line', start + 20), ('to', True), ('owner', 'action'),
                                 ('dialogue', {'keywords': ['no'], 'topics': [1]})]:
                with self.subTest(field=field), self.assertRaises(ValueError):
                    npc.transcribe(server, path, raw, {**write, field: value})

    def test_occurrence_owner_namespace_pins_and_counts(self):
        # Reconstruct the pre-existing source occurrence shape without raw prose.
        occurrences = []
        for server, profile in self.profiles.items():
            props = next(r['then']['properties'] for r in self.schema['$defs']['source_occurrence']['allOf']
                         if r.get('if', {}).get('properties', {}).get('source') == {'const': server}
                         and r.get('if', {}).get('required') == ['source_exchange'])
            row = {k: copy.deepcopy(v['const']) for k, v in props.items() if 'const' in v}
            row.update(source=server, registrations=[], dialogue={'keywords': ['yes'], 'topics': [1]},
                       source_exchange=copy.deepcopy(profile))
            occurrences.append(row)
        transition = {'key': 'npc_1', 'script': 'npc/pemaret.lua', 'source_occurrences': occurrences,
            'sources': {o['source']: {k: o[k] for k in ('path', 'line', 'registrations', 'dialogue')} for o in occurrences},
            'write': copy.deepcopy(occurrences[0]['write'])}
        transition['write']['servers'] = sorted(npc.PINS)
        doc = {'progress': [{'key': npc.TRACK, 'missions': [], 'start_of': [], 'read_by_gates': [],
            'auxiliary_of': ['canary:quest/marlin_trophy_quest'], 'owner_basis': 'exclusive curated NPC writers',
            'writes': dict.fromkeys(npc.PINS, 1), 'transitions': [transition]}]}
        self.progress_validator.validate(doc)
        for field, value in [('target', 'Storage.Other'), ('path', 'data-otservbr-global/npc/uzgod.lua'),
                             ('line', 75), ('blob_sha256', '0'*64), ('revision', '0'*40),
                             ('source', 'crystalserver'), ('write', {**occurrences[0]['write'], 'owner': 'action'})]:
            mutant = copy.deepcopy(doc); mutant['progress'][0]['transitions'][0]['source_occurrences'][0][field] = value
            with self.subTest(field=field):
                self.assertTrue(list(self.progress_validator.iter_errors(mutant)))
        for mutate in [lambda d: d['progress'][0].update(key=npc.TRACK.replace('marlin_trophy', 'other')),
                       lambda d: d['progress'][0].update(auxiliary_of=['canary:quest/other']),
                       lambda d: d['progress'][0]['transitions'][0]['source_occurrences'].pop(),
                       lambda d: d['progress'][0]['transitions'][0].update(key='npc_2')]:
            mutant = copy.deepcopy(doc); mutate(mutant)
            self.assertTrue(list(self.progress_validator.iter_errors(mutant)))

    def test_missing_provider_profile(self):
        with self.assertRaises(ValueError):
            npc.schema_update(copy.deepcopy(self.schema), {'canary': self.profiles['canary']})


if __name__ == '__main__':
    unittest.main()
