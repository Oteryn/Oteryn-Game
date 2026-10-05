#!/usr/bin/env python3
"""Exact byte replay and fail-closed negatives for the accepted monster snapshot."""
import copy
import json
import unittest
from pathlib import Path
from unittest.mock import patch
import import_spell_families as producer
import monster_seven_spell_overlay as overlay

ROOT=Path(__file__).resolve().parents[2]
class AcceptedOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Exercise real frozen-r25 verification; only the new replay stage is isolated.
        with patch.object(overlay,'apply_overlay',side_effect=lambda root,generated:generated):
            cls.base=producer.outputs(ROOT)
        cls.receipt=json.loads((ROOT/overlay.PATCH_PATH).read_bytes())
    def test_replays_exact_current_four_outputs_without_mutating_base(self):
        before={k:overlay.digest(v) for k,v in self.base.items()}
        actual=overlay.apply_overlay(ROOT,self.base)
        for path in overlay.ALLOWED_PATHS:
            self.assertEqual(actual[path],(ROOT/path).read_bytes(),path)
        self.assertEqual(before,{k:overlay.digest(v) for k,v in self.base.items()})
        self.assertEqual(len(json.loads(actual['content/creatures/definitions/spell-native-profiles.json'])['records']),1870)
        looks=json.loads(actual['content/presentations/bindings/spell-appearances.json'])
        self.assertEqual(len(looks['records']),334)
        self.assertEqual(len(looks['item_appearances']),6)
    def test_registration_replays_all_current_outputs(self):
        import register_spell_families
        actual=register_spell_families.outputs(ROOT)
        self.assertEqual(len(actual),33)
        for path,data in actual.items():
            self.assertEqual(data,(ROOT/path).read_bytes(),path)
    def test_adopted_qualification_tamper_is_refused(self):
        actual_read=Path.read_bytes
        wanted=ROOT/'imports/spells/monster-seven/adopted-qualification.json'
        def changed_read(path):
            raw=actual_read(path)
            return raw+b' ' if path==wanted else raw
        with patch.object(Path,'read_bytes',changed_read):
            with self.assertRaisesRegex(ValueError,'OVERLAY_QUALIFICATION_DIGEST'):
                overlay.apply_overlay(ROOT,self.base)
    def test_wrong_immutable_base_is_refused(self):
        bad=dict(self.base);p=next(iter(overlay.ALLOWED_PATHS));bad[p]+=b' '
        with self.assertRaisesRegex(ValueError,'OVERLAY_BASE_DIGEST'):
            overlay.apply_overlay(ROOT,bad)
    def reject_changed_receipt(self,receipt):
        # Even a syntactically valid, self-consistent successor is not owner-accepted.
        with self.assertRaisesRegex(ValueError,'OVERLAY_RECEIPT_DIGEST'):
            overlay.apply_overlay(ROOT,self.base,receipt)
    def test_tampered_value_with_recomputed_output_hash_is_refused(self):
        bad=copy.deepcopy(self.receipt)
        f=next(f for f in bad['files'] if f['path'].startswith('content/creatures/'))
        op=next(o for o in f['operations'] if o['op']=='replace' and type(o.get('value')) in (int,float))
        op['value']+=1
        altered=json.loads((ROOT/f['path']).read_bytes())
        node,key=overlay.parent(altered,op['pointer'])
        node[int(key) if isinstance(node,list) else key]=op['value']
        f['output_sha256']=overlay.digest(overlay.encode(altered,f['encoding']))
        # Recomputing this receipt digest cannot change the independently code-pinned accepted digest.
        self.assertNotEqual(overlay.digest(overlay.encode(bad,'PRETTY2')),overlay.PATCH_SHA256)
        self.reject_changed_receipt(bad)
    def test_duplicate_pointer_is_refused(self):
        bad=copy.deepcopy(self.receipt)
        f=next(f for f in bad['files'] if f['operations'])
        f['operations'].append(copy.deepcopy(f['operations'][0]))
        self.reject_changed_receipt(bad)
    def test_unknown_output_path_is_refused(self):
        bad=copy.deepcopy(self.receipt);bad['files'][0]['path']='content/unauthorized.json'
        self.reject_changed_receipt(bad)
    def test_unknown_operation_is_refused(self):
        bad=copy.deepcopy(self.receipt)
        f=next(f for f in bad['files'] if f['operations']);f['operations'][0]['op']='merge_unbounded'
        self.reject_changed_receipt(bad)
if __name__=='__main__':unittest.main()
