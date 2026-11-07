#!/usr/bin/env python3
"""Identity regression tests for creature admission; no network or bundle regeneration.

Run with `python tools/content-migration/test_creature_admission_stage.py`.
"""
from __future__ import annotations

import json
import unittest

import creature_admission_stage as stage


def item(item_id: int) -> dict:
    return {'identity': {'family': 'Item', 'key': f'oteryn:item.tibia.i{item_id}', 'revision': stage.REVISION}}


def binding(item_id: int) -> dict:
    return {'source_key': 'oteryn:source.crystalserver',
            'source_revision': '00ce02a57ca5a12e48f32a3476e37471167e4c3f',
            'identity_namespace': 'ots/item_server_id', 'external_id': str(item_id),
            'disposition': 'EXACT', 'target': item(item_id)['identity']}


class ItemIdentityTests(unittest.TestCase):
    def test_admitted_bound_and_appearance_only_items(self):
        result = stage.resolve_admitted_item_map({}, [item(1), item(2)], [], [binding(1)], {1, 2})
        self.assertEqual(result, {1: 'oteryn:item.tibia.i1', 2: 'oteryn:item.tibia.i2'})

    def test_numeric_key_without_appearance_is_not_identity_proof(self):
        self.assertEqual(stage.resolve_admitted_item_map({}, [item(1)], [], [binding(1)], set()), {})

    def test_appearance_without_admitted_item_cannot_resolve(self):
        self.assertEqual(stage.resolve_admitted_item_map({}, [], [], [binding(1)], {1}), {})

    def test_conflicting_binding_fails_closed(self):
        other = binding(1)
        other['target'] = item(2)['identity']
        with self.assertRaisesRegex(stage.StageError, 'ambiguous'):
            stage.resolve_admitted_item_map({}, [item(1), item(2)], [], [binding(1), other], {1, 2})

    def test_unsupported_bindings_fail_closed(self):
        for field, value in [('disposition', 'AMBIGUOUS'), ('source_revision', 'unaccepted'),
                             ('target', {**item(1)['identity'], 'revision': 'unaccepted'})]:
            with self.subTest(field=field):
                bad = binding(1)
                bad[field] = value
                with self.assertRaises(stage.StageError):
                    stage.resolve_admitted_item_map({}, [item(1)], [], [bad], {1})

    def test_retired_allocation_cannot_be_revived(self):
        for tombstone in ('old:item', 'oteryn:item.tibia.i1'):
            aliases = [{'key': tombstone, 'state': 'RETIRED_WITHOUT_SUCCESSOR'}]
            self.assertEqual(stage.resolve_admitted_item_map({1: tombstone}, [item(1)], aliases, [], {1}), {})

    def test_baseline_alias_and_protected_rekey_are_preserved(self):
        aliases = [{'key': 'old:allocation', 'state': 'ALIAS', 'target': 'oteryn:item.tibia.i1'},
                   {'key': 'protected:rekey', 'state': 'ALIAS', 'target': 'oteryn:item.tibia.i2'}]
        baseline = {1: 'old:allocation', 2: 'protected:rekey'}
        result = stage.resolve_admitted_item_map(baseline, [item(1), item(2)], aliases,
                                                 [binding(1), binding(2)], {1, 2})
        self.assertEqual(result, baseline)
        self.assertEqual(baseline, {1: 'old:allocation', 2: 'protected:rekey'})

    def test_baseline_without_admitted_alias_fails_closed(self):
        with self.assertRaisesRegex(stage.StageError, 'absent from content/world'):
            stage.resolve_admitted_item_map({1: 'old:allocation'}, [item(1)], [], [], {1})

    def test_real_item_add_1_closes_217_of_218_missing_ids(self):
        records = json.loads(stage.REFERENCE.read_text())['records']
        aliases = json.loads(stage.ITEM_ALIASES.read_text())['entries']
        bindings = json.loads(stage.ITEM_BINDINGS.read_text())['bindings']
        index, manifests = stage.load_admitted()
        current = {entry[0] for entry in manifests[index['newest']]['entries']}
        result = stage.resolve_admitted_item_map({}, records, aliases, bindings, current)
        evidence = stage.ROOT / 'docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json'
        rows = json.loads(evidence.read_text())['deferred']['unregistered_items']
        missing = {item_id for row in rows for item_id in row['items']}
        self.assertEqual(len(rows), 95)
        self.assertEqual(len(missing), 218)
        self.assertEqual(missing - result.keys(), {48296})
        self.assertEqual(sum(set(row['items']) <= result.keys() for row in rows), 94)
        admitted_missing = missing & result.keys()
        self.assertEqual(len(admitted_missing), 217)
        bound_ids = {int(row['external_id']) for row in bindings}
        self.assertEqual(len(admitted_missing & bound_ids), 157)


if __name__ == '__main__':
    unittest.main()
