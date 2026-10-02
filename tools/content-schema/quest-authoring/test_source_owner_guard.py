"""Exact two-track SOURCE exception, including resealed negative controls."""
import copy
import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

import source_owner_guard as owner
from source_fix_guard import normalize_core

ROOT = Path(os.environ.get('QUEST_OWNER_GUARD_TEST_ROOT', Path(__file__).resolve().parents[3]))


class OwnerGuardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        receipt = json.loads((ROOT / 'tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json').read_text())
        cls.descriptor = receipt['owner_associations']
        cls.packet = json.loads((ROOT / cls.descriptor['path']).read_text())
        cls.changes = [r for r in receipt['changes'] if r['key'] in owner.CORES]

    def run_pair(self, change=None, mutate_packet=None, mutate_curation=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); packet = copy.deepcopy(self.packet)
            for field in ('curation', 'evidence'):
                path = root / packet[field]['path']; path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((ROOT / packet[field]['path']).read_bytes())
            if mutate_curation:
                path = root / packet['curation']['path']
                curation = json.loads(path.read_text())
                mutate_curation(curation)
                raw = (json.dumps(curation, indent=2) + '\n').encode()
                path.write_bytes(raw)
                packet['curation']['sha256'] = hashlib.sha256(raw).hexdigest()
            if mutate_packet:
                mutate_packet(packet)
            raw = (json.dumps(packet, indent=2) + '\n').encode()
            path = root / self.descriptor['path']; path.write_bytes(raw)
            descriptor = {'path': self.descriptor['path'], 'sha256': hashlib.sha256(raw).hexdigest()}
            return owner.reviewed_pair(root, change or self.changes[0], descriptor)

    def test_actual_three_cores_only_approved_associations_factored(self):
        self.assertEqual(len(self.changes), 3)
        for change in self.changes:
            before = copy.deepcopy(change)
            old, new = self.run_pair(change)
            self.assertEqual(normalize_core(old), normalize_core(new))
            self.assertEqual(change, before)

    def test_resealed_track_effect_owner_and_provenance_changes_rejected(self):
        mutations = [lambda p: p['tracks'][0]['new']['transitions'][0]['write'].update(to=999),
                     lambda p: p['tracks'][0]['new'].update(auxiliary_of=['canary:quest/other']),
                     lambda p: p['tracks'][0]['new']['transitions'][0]['source_occurrences'][0].update(line=999),
                     lambda p: p['tracks'][1]['new'].update(initial=0)]
        for mutate in mutations:
            with self.assertRaisesRegex(ValueError, 'effects/provenance'):
                self.run_pair(mutate_packet=mutate)

    def test_foreign_core_track_and_native_approval_rejected(self):
        mutations = [lambda p: p['approved_core_digests'][0].update(key='oteryn:quest.foreign'),
                     lambda p: p['tracks'].pop(),
                     lambda p: p.update(native_runtime_admission=True),
                     lambda p: p.update(relocated_graph_key='canary:interaction/foreign')]
        for mutate in mutations:
            with self.assertRaises(ValueError):
                self.run_pair(mutate_packet=mutate)

    def test_wrong_membership_and_changed_relocated_graph_rejected(self):
        change = copy.deepcopy(next(c for c in self.changes if c['key'] == owner.SIDE))
        change['new_core']['source_data']['progress'].clear()
        with self.assertRaisesRegex(ValueError, 'membership'):
            self.run_pair(change)
        with self.assertRaisesRegex(ValueError, 'relocation'):
            self.run_pair(next(c for c in self.changes if c['key'] == owner.SIDE),
                          lambda p: p['relocated_graph']['source'].update(callback='forged'))

    def test_resealed_curation_missing_write_witness_rejected(self):
        for track in (owner.STEAL, owner.GRAVE):
            def mutate(c, track=track):
                c['additions'][track.split(':quest-progress/')[1]]['owner_evidence'].pop()
            with self.subTest(track=track), self.assertRaisesRegex(ValueError, 'owner evidence'):
                self.run_pair(mutate_curation=mutate)

    def test_resealed_curation_same_length_duplicate_witness_rejected(self):
        for track in (owner.STEAL, owner.GRAVE):
            def mutate(c, track=track):
                witnesses = c['additions'][track.split(':quest-progress/')[1]]['owner_evidence']
                witnesses[-1] = copy.deepcopy(witnesses[0])
            with self.subTest(track=track), self.assertRaisesRegex(ValueError, 'owner evidence'):
                self.run_pair(mutate_curation=mutate)

    def test_other_progress_missions_native_and_requirements_remain_protected(self):
        for field in ('native_lowering', 'requirements', 'source_refs', 'readiness'):
            change = copy.deepcopy(self.changes[0]); change['new_core'][field] = 'forged'
            old, new = self.run_pair(change)
            self.assertNotEqual(normalize_core(old), normalize_core(new))
        change = copy.deepcopy(next(c for c in self.changes if c['key'] == owner.DREFIA))
        change['new_core']['source_data']['progress'][0]['missions'] = ['foreign-mission']
        old, new = self.run_pair(change)
        self.assertNotEqual(normalize_core(old), normalize_core(new))


if __name__ == '__main__':
    unittest.main()
