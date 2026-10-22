#!/usr/bin/env python3
"""Exact byte replay and fail-closed negatives for the accepted monster snapshot."""
import copy
import json
import unittest
from pathlib import Path
from unittest.mock import patch
import import_spell_families as producer
import monster_seven_spell_overlay as overlay
import register_spell_families as registration

ROOT=Path(__file__).resolve().parents[2]
class AcceptedOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Exercise real frozen-r25 verification; only the new replay stage is isolated.
        with patch.object(overlay,'apply_overlay',side_effect=lambda root,generated:generated), patch('bind_monster_source_definitions.bind_definitions', side_effect=lambda root,generated:generated):
            cls.base=producer.outputs(ROOT)
        cls.receipt=json.loads((ROOT/overlay.PATCH_PATH).read_bytes())
    def test_replays_exact_current_four_outputs_without_mutating_base(self):
        before={k:overlay.digest(v) for k,v in self.base.items()}
        from bind_monster_source_definitions import bind_definitions
        adopted=overlay.apply_overlay(ROOT,self.base)
        for entry in self.receipt["files"]:
            self.assertEqual(overlay.digest(adopted[entry["path"]]),entry["output_sha256"])
        actual=bind_definitions(ROOT,adopted)
        for path in overlay.ALLOWED_PATHS - {"content/creatures/definitions/spell-native-profiles.json", "content/spells.manifest.json"}:
            self.assertEqual(actual[path],(ROOT/path).read_bytes(),path)
        self.assertEqual(before,{k:overlay.digest(v) for k,v in self.base.items()})
        self.assertEqual(len(json.loads(actual['content/creatures/definitions/spell-native-profiles.json'])['records']),1870)
        looks=json.loads(actual['content/presentations/bindings/spell-appearances.json'])
        self.assertEqual(len(looks['records']),334)
        self.assertEqual(len(looks['item_appearances']),6)
    def test_registration_replays_all_current_outputs(self):
        actual=registration.outputs(ROOT)
        self.assertEqual(len(actual),36)
        for path,data in actual.items():
            self.assertEqual(data,(ROOT/path).read_bytes(),path)
    def test_consumable_overlay_carries_only_the_selected_source_fact(self):
        planned = dict(self.base)
        path = producer.PROVIDER_DESTINATIONS["item_profiles"]
        before = json.loads(planned[path])
        facts = json.loads((ROOT / registration.CONSUMPTION_FACTS).read_bytes())
        fact = next(row for row in facts["rows"]
                    if row["target"]["key"] == registration.CONSUMABLE_KEY)
        self.assertEqual(fact["consumption"], {"kind": "FOOD", "value": {"regeneration_seconds": 180}})
        registration.admit_consumable_profiles(ROOT, planned)
        after = json.loads(planned[path])
        retained = [r for r in after["records"]
                    if r["production_definition"]["production_key"] not in registration.POTION_KEYS]
        for old, new in zip(before["records"], retained, strict=True):
            if old["production_definition"]["production_key"] == registration.CONSUMABLE_KEY:
                self.assertEqual(new["semantics"].pop("consumption"),
                                 {"state": "KNOWN", "value": fact["consumption"]})
            self.assertEqual(new, old)
        added = [r for r in after["records"]
                 if r["production_definition"]["production_key"] in registration.POTION_KEYS]
        self.assertEqual(len(added), 2)
        world = json.loads((ROOT / 'content/world/definitions/reference.json').read_bytes())
        for record in added:
            source = next(r for r in world["records"] if r["identity"] == record["authoring"]["item"])
            self.assertEqual(record["semantics"], source["semantics"])
            self.assertEqual(record["production_binding"]["target"], source["identity"])
            self.assertEqual(record["attributes"], {})
            self.assertEqual(record["admission"]["stack_class"], source["stack_class"])
        manifest = json.loads(planned[registration.NATIVE_MANIFEST])
        self.assertEqual(manifest["item_profiles"]["sha256"], overlay.digest(planned[path]))
        for unchanged in self.base.keys() - {path, registration.NATIVE_MANIFEST}:
            self.assertEqual(planned[unchanged], self.base[unchanged])
    def test_consumable_evidence_tamper_is_refused(self):
        actual_read = Path.read_bytes
        wanted = ROOT / registration.CONSUMPTION_FACTS
        with patch.object(Path, 'read_bytes', lambda path: actual_read(path) + (b' ' if path == wanted else b'')):
            with self.assertRaisesRegex(ValueError, 'CONSUMPTION_FACTS_DIGEST'):
                registration.admit_consumable_profiles(ROOT, dict(self.base))
    def test_consumable_missing_duplicate_revision_and_semantic_conflict_are_refused(self):
        path = producer.PROVIDER_DESTINATIONS["item_profiles"]
        for operator in ("missing", "duplicate", "revision", "semantics"):
            document = json.loads(self.base[path])
            record = next(r for r in document["records"]
                          if r["production_definition"]["production_key"] == registration.CONSUMABLE_KEY)
            if operator == "missing":
                document["records"].remove(record)
            elif operator == "duplicate":
                document["records"].append(copy.deepcopy(record))
            elif operator == "revision":
                record["production_definition"]["revision_ref"] = "unqualified"
            else:
                record["semantics"]["consumption"] = {"state": "UNKNOWN"}
            planned = self.base | {path: producer.canonical_bytes(document)}
            before = dict(planned)
            with self.subTest(operator=operator), self.assertRaisesRegex(ValueError, 'CONSUMPTION_PROFILE_'):
                registration.admit_consumable_profiles(ROOT, planned)
            self.assertEqual(planned, before)
    def test_world_potion_and_flask_identity_admission_and_consumption_conflicts_are_refused(self):
        actual_read = Path.read_bytes
        world_path = ROOT / 'content/world/definitions/reference.json'
        bindings_path = ROOT / 'imports/crystalserver/bindings/items.json'
        sources = [r for r in json.loads(actual_read(world_path))["records"]
                   if r["identity"]["key"] in registration.POTION_KEYS]
        bindings = [b for b in json.loads(actual_read(bindings_path))["bindings"]
                    if b["target"]["key"] in registration.POTION_KEYS]
        for operator in ("missing", "duplicate", "binding", "physical", "consumption"):
            world = {"records": copy.deepcopy(sources)}
            admitted = {"bindings": copy.deepcopy(bindings)}
            potion = next(r for r in world["records"] if r["identity"]["key"] == 'oteryn:item.tibia.i266')
            if operator == "missing":
                world["records"].remove(potion)
            elif operator == "duplicate":
                world["records"].append(copy.deepcopy(potion))
            elif operator == "binding":
                admitted["bindings"][0]["target"]["revision"] = "unqualified"
            elif operator == "physical":
                potion["semantics"]["physical"]["value"]["movable"] = {"state": "UNKNOWN"}
            else:
                potion["semantics"]["consumption"]["value"]["value"]["restores"][0]["min"] = 1
            replacements = {world_path: producer.canonical_bytes(world),
                            bindings_path: producer.canonical_bytes(admitted)}
            planned = dict(self.base)
            with self.subTest(operator=operator), patch.object(
                Path, 'read_bytes', lambda path: replacements[path] if path in replacements else actual_read(path)
            ), self.assertRaisesRegex(ValueError, 'CONSUMPTION_PROFILE_'):
                registration.admit_consumable_profiles(ROOT, planned)
            self.assertEqual(planned, self.base)
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
