"""Prove Primal registrations are real derived actors with unique source identities."""
import tempfile
import os
import copy
import unittest
from pathlib import Path

import complete_remaining_monster_mechanics as core
import repair_primal_source_references as repair

REPO = Path(__file__).resolve().parents[2]
BASE = Path('/workspace/monster-mitigation-estimate-output/population-v2')
SOURCES = Path('/workspace/monster-final-output/sources-final')
FAMILIARS = Path('/workspace/monster-final-output/familiars-verified')
PREVIOUS = Path('/workspace/monster-final-output/mechanics-qualified')
CANARY = Path('/workspace/monster-reference-sources/canary')


@unittest.skipUnless(BASE.exists() and SOURCES.exists() and FAMILIARS.exists() and PREVIOUS.exists() and CANARY.exists(),
                     'pinned immutable research fixtures unavailable')
class PrimalRegistrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(dir=BASE.parent)
        cls.variants = Path(cls.temp.name) / 'variants'
        cls.mechanics = Path(cls.temp.name) / 'mechanics'
        # Reconstruct the pre-Primal closure from preserved input packets,
        # never from a mutable or future finalized population snapshot.
        cls.current = Path(cls.temp.name) / 'pre_primal'
        index = copy.deepcopy(core.read(BASE / 'population-index.json'))
        rows = {r['monster']: r for r in index['monsters']}
        inputs = {r['monster']: BASE / 'bundles' / r['monster'] for r in index['monsters']}
        for packet in (SOURCES, FAMILIARS):
            for row in core.read(packet / 'completion.json')['index_monsters']:
                rows[row['monster']] = row
                inputs[row['monster']] = packet / 'bundles' / row['monster']
        index['monsters'] = [rows[name] for name in sorted(rows)]
        if len(index['monsters']) != 1752:
            raise AssertionError('immutable source/familiar input closure must contain1752 actors')
        for name, source in inputs.items():
            destination = cls.current / 'bundles' / name
            destination.mkdir(parents=True)
            for filename in core.FILES:
                os.link(source / filename, destination / filename)
        core.write(cls.current / 'population-index.json', index)
        # Existing source bindings come from the immutable1697 stage; the
        # newly derived eleven registrations must never be included here.
        os.link(BASE / 'creature-admission-stage.json', cls.current / 'creature-admission-stage.json')
        cls.receipt = repair.prepare(REPO, BASE, cls.current, PREVIOUS, CANARY, cls.variants, cls.mechanics)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_eleven_distinct_registered_types_with_real_closure(self):
        self.assertEqual(len(self.receipt['actors']), 11)
        self.assertEqual(len({a['identity'] for a in self.receipt['actors']}), 11)
        self.assertTrue(all(a['identity'] != a['base_identity'] for a in self.receipt['actors']))
        closure = core.read(self.mechanics / 'primal-reference-closure.json')
        self.assertEqual(closure['definitions'], 1763)
        self.assertFalse(closure['unresolved_creature_references'])
        self.assertFalse(closure['ordinary_creature_substitution'])

    def test_helper_overrides_and_donor_mitigation_are_honest(self):
        for a in self.receipt['actors']:
            m = core.read(self.variants / 'bundles' / a['monster'] / 'monster.json')
            c = m['creature']
            self.assertEqual(c['stats']['experience'], 0)
            self.assertEqual(c['stats']['max_health'], a['max_health'])
            self.assertEqual(c['stats']['initial_health'], a['max_health'])
            self.assertNotIn('corpse_item', c)
            self.assertNotIn('bestiary', c)
            self.assertEqual(m['loot']['entries'], [])
            self.assertEqual(c['display_name'], 'Primal Pack Beast')
            self.assertFalse(a['global_parity'])
        missing = [a['monster'] for a in self.receipt['actors'] if a['donor_mitigation'] is None]
        self.assertEqual(missing, ['noxious_ripptor_primal'])

    def test_native_bindings_and_dependency_shapes(self):
        import creature_admission_stage as admission
        cache = core.read(Path('/workspace/monster-round7-output/native-item-map-rust.json'))
        stage = admission.Stage(admission.Mapper({r['source_item_id']: r['native_key'] for r in cache['records']}))
        for row in self.receipt['index_monsters']:
            p = self.variants / 'bundles' / row['monster']
            m, deps = core.read(p / 'monster.json'), core.read(p / 'dependencies.json')
            stage.stage_dependencies(deps, m['creature']['identity']['key'])
            stage.stage_monster(m, row['file'], row['binding'])
        tuples = [(b['source_key'], b['source_revision'], b['identity_namespace'], b['external_id']) for b in stage.bindings]
        self.assertEqual(len(set(tuples)), 11)
        self.assertTrue(all('#registered-type=' in b['external_id'] for b in stage.bindings))
        self.assertEqual(self.receipt['source_binding_closure']['collisions'], 0)


if __name__ == '__main__':
    unittest.main()
