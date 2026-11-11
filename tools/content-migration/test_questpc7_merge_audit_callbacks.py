"""The historical merge audit must keep the current two-owner callback contract."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import questpc7_merge_audit_inventories as audit


class CallbackOwnerInventoryTests(unittest.TestCase):
    MAIN = 'content/quests/questpc7-world-callbacks.json'
    PHASE = 'content/quests/world-callbacks/feaster-phase-routes.json'
    FIXTURE = 'crates/gameplay/quests/src/quest/world_callbacks/outfit_tests.rs'

    @classmethod
    def setUpClass(cls):
        cls.main = json.loads((audit.ROOT / cls.MAIN).read_bytes())
        cls.phase = json.loads((audit.ROOT / cls.PHASE).read_bytes())
        cls.fixture = (audit.ROOT / cls.FIXTURE).read_text()
        # The committed predecessor may still contain the two phase routes.
        # Apply only questpc7_integration.outputs' existing key partition; keep
        # every remaining main row and the complete published phase body.
        phase_keys = {row['key'] for row in cls.phase['callbacks']}
        cls.main['callbacks'] = [row for row in cls.main['callbacks']
                                 if row['key'] not in phase_keys]

    def check(self, main=None, phase=None, fixture=None, *, refused=False):
        main = self.main if main is None else main
        phase = self.phase if phase is None else phase
        fixture = self.fixture if fixture is None else fixture
        original = {
            self.MAIN: (json.dumps(main, ensure_ascii=False, indent=2) + '\n').encode(),
            self.PHASE: (json.dumps(phase, ensure_ascii=False, indent=2) + '\n').encode(),
            self.FIXTURE: fixture.encode(),
        }
        with tempfile.TemporaryDirectory(prefix='questpc7-callback-audit-') as temp:
            root = Path(temp)
            for path, raw in original.items():
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(raw)
            if refused:
                with self.assertRaisesRegex(ValueError, 'QUESTPC7_CALLBACK_'):
                    audit.check_callback_inventory(root)
            else:
                self.assertEqual(audit.check_callback_inventory(root), (110, 2, 112, 103))
                self.assertEqual(audit.check_callback_inventory(root), (110, 2, 112, 103))
            self.assertEqual({path: (root / path).read_bytes() for path in original}, original,
                             'validation and refusal must preserve both owners and the Rust guards')

    def test_current_two_owners_and_replay_preserve_the_original_fixture(self):
        self.check()

    def test_missing_and_duplicate_rows_are_refused_in_each_owner(self):
        for owner in ('main', 'phase'):
            for mutation in ('missing', 'duplicate_append', 'duplicate_replace'):
                with self.subTest(owner=owner, mutation=mutation):
                    document = copy.deepcopy(getattr(self, owner))
                    rows = document['callbacks']
                    if mutation == 'missing':
                        rows.pop()
                    elif mutation == 'duplicate_append':
                        rows.append(copy.deepcopy(rows[0]))
                    else:
                        rows[-1] = copy.deepcopy(rows[0])
                    self.check(**{owner: document}, refused=True)

    def test_foreign_keys_cannot_keep_the_same_counts(self):
        for owner in ('main', 'phase'):
            with self.subTest(owner=owner):
                document = copy.deepcopy(getattr(self, owner))
                document['callbacks'][-1]['key'] = 'oteryn:quest-callback/unreviewed-substitution'
                self.check(**{owner: document}, refused=True)

    def test_overlap_and_count_preserving_owner_swaps_are_refused(self):
        overlapping = copy.deepcopy(self.main)
        overlapping['callbacks'].extend(copy.deepcopy(self.phase['callbacks']))
        self.check(main=overlapping, refused=True)
        main, phase = copy.deepcopy(self.main), copy.deepcopy(self.phase)
        main['callbacks'][-1], phase['callbacks'][0] = phase['callbacks'][0], main['callbacks'][-1]
        self.check(main=main, phase=phase, refused=True)

    def test_original_named_cohorts_cannot_be_removed_or_substituted(self):
        for key in (
            'oteryn:quest-callback/questpc7-goblin-sign-1',
            'oteryn:quest-callback/questpc7-goblin-pond',
            'oteryn:quest-callback/questpc7-goblin-trunk-1',
            'oteryn:quest-callback/authored-smith-cache',
            'oteryn:quest-callback/authored-smith-cache-reward',
        ):
            with self.subTest(key=key):
                main = copy.deepcopy(self.main)
                row = next(row for row in main['callbacks'] if row['key'] == key)
                row['key'] = 'oteryn:quest-callback/unreviewed-substitution'
                self.check(main=main, refused=True)
        empty = copy.deepcopy(self.main)
        empty['callbacks'] = []
        self.check(main=empty, refused=True)

    def test_missing_commented_or_monolithic_runtime_guards_are_refused(self):
        guards = (
            'assert_eq!((callbacks.len(), phase_callbacks.len()), (110, 2));',
            'assert!(keys.is_disjoint(&phase_keys));',
            'assert_eq!(keys.len() + phase_keys.len(), 112);',
        )
        for guard in guards:
            self.assertEqual(self.fixture.count(guard), 1)
            for replacement in ('', '// ' + guard, 'assert_eq!(parse(source)?.len(), 112);'):
                with self.subTest(guard=guard, replacement=replacement):
                    self.check(fixture=self.fixture.replace(guard, replacement), refused=True)


if __name__ == '__main__':
    unittest.main()
