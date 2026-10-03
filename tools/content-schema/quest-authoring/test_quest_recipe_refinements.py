"""Replay approved corrections and reject changes to the immutable recipe scope."""
import copy
import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

import quest_recipe_refinements as tool

ROOT = Path(os.environ.get('QUEST_REFINEMENTS_TEST_ROOT', Path(__file__).resolve().parents[3]))
HERE = Path(__file__).resolve().parent


class RefinementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (ROOT / 'tools/content-schema/quest-authoring/samples/completion242/recipes.json').read_bytes()
        cls.original = json.loads(cls.raw)
        cls.sha = hashlib.sha256(cls.raw).hexdigest()
        cls.packet = json.loads((HERE / 'samples/refinements242/refinements.json').read_bytes())

    def apply(self, packet=None, payload=None, sha=None):
        return tool.apply_packet(payload or self.original, packet or self.packet, sha or self.sha)

    def test_exact_approved21_in13_and_all242_journeys(self):
        before = copy.deepcopy(self.original)
        rows = self.apply()
        self.assertEqual({r['identity']['key']: tool.digest(r) for r in rows}, self.packet['refined_record_digests'])
        self.assertEqual(self.original, before)
        changed = [b for a, b in zip(before['records'], rows) if a != b]
        self.assertEqual(len(changed), 13)
        self.assertEqual(len(self.packet['changes']), 21)
        for old, new in zip(before['records'], rows):
            self.assertEqual(tool.protected(old), tool.protected(new))

    def test_named_approved_corrections_are_present(self):
        recipes = {r['identity']['key'].removeprefix('oteryn:quest.'): r['recipe'] for r in self.apply()}
        self.assertEqual(recipes['illuminator_outfits_quest']['requirements']['prerequisites'], ['Between the Lines Quest'])
        self.assertEqual(recipes['spirithunters_quest']['requirements']['prerequisites'], ['Research and Development Quest completed'])
        self.assertEqual(recipes['newhaven_quest']['stages'][3]['targets'], ['Book of Wisdom'])
        self.assertEqual(recipes['the_dream_courts_quest']['stages'][1]['kind'], 'use')
        self.assertEqual(recipes['the_white_raven_monastery_quest']['stages'][2]['targets'], ["Monk's Diary"])

    def test_changed_old_value_and_wrong_original_bytes_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['changes'][0]['old_value'] = ['wrong original']
        with self.assertRaisesRegex(ValueError, 'old-value'):
            self.apply(packet)
        with self.assertRaisesRegex(ValueError, 'input'):
            self.apply(sha='0' * 64)

    def test_duplicate_and_missing_change_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['changes'].append(copy.deepcopy(packet['changes'][0]))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.apply(packet)
        packet['changes'].pop(); packet['changes'].pop()
        with self.assertRaisesRegex(ValueError, '21 changes'):
            self.apply(packet)

    def test_protected_stage_fields_rejected_even_with_resealed_digests(self):
        for field, value in [('count', 99), ('key', 'foreign'), ('next', []), ('basis', 'SOURCE_REFERENCE')]:
            packet = copy.deepcopy(self.packet)
            change = next(c for c in packet['changes'] if c['path'] == '/stages')
            change['new_value'][0][field] = value
            with self.assertRaisesRegex(ValueError, 'provenance'):
                self.apply(packet)

    def test_new_native_fields_source_refs_and_reward_counts_not_allowed(self):
        for path in ['/source_refs', '/runtime_enabled', '/reward_intents/0/count']:
            packet = copy.deepcopy(self.packet); packet['changes'][0]['path'] = path
            with self.assertRaisesRegex(ValueError, 'protected field'):
                self.apply(packet)
        packet = copy.deepcopy(self.packet); packet['runtime_enabled'] = True
        with self.assertRaisesRegex(ValueError, 'Native'):
            self.apply(packet)

    def test_unknown_selection_refined_digest_and_bad_journey_rejected(self):
        packet = copy.deepcopy(self.packet); packet['changes'][0]['canonical_key'] = 'oteryn:quest.unknown'
        with self.assertRaisesRegex(ValueError, 'unknown'):
            self.apply(packet)
        packet = copy.deepcopy(self.packet); key = packet['changes'][0]['canonical_key']
        packet['refined_record_digests'][key] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'refined record'):
            self.apply(packet)
        packet = copy.deepcopy(self.packet)
        change = next(c for c in packet['changes'] if c['path'] == '/stages')
        change['new_value'][1]['objective'] = change['new_value'][0]['objective']
        with self.assertRaisesRegex(ValueError, 'duplicate stage or objective'):
            self.apply(packet)

    def test_portable_loader_provenance_tamper_guard_and_absent_overlay(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            rows, provenance = tool.apply_refinements(root, self.original)
            self.assertEqual(rows, self.original['records']); self.assertIsNone(provenance)
            path = root / tool.PACKET; path.parent.mkdir(parents=True)
            path.write_bytes((HERE / 'samples/refinements242/refinements.json').read_bytes())
            original = root / 'tools/content-schema/quest-authoring/samples/completion242/recipes.json'
            original.parent.mkdir(parents=True); original.write_bytes(self.raw)
            rows, provenance = tool.apply_refinements(root, self.original)
            self.assertEqual({r['identity']['key']: tool.digest(r) for r in rows}, self.packet['refined_record_digests'])
            self.assertEqual(provenance['original_recipe_packet_sha256'], self.sha)
            path.write_bytes(path.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'unapproved'):
                tool.apply_refinements(root, self.original)


if __name__ == '__main__':
    unittest.main()
