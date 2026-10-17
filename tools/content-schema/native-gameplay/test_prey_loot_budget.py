"""PREY0 §3/RL-16: exercise the actual native loot producer on synthetic content."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import build_full_spell_artifact as full


def ref(family, key):
    return {'family': family, 'key': key, 'revision': 'synthetic-r1'}


class PreyLootCompilation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.paths = tuple(self.root / name for name in ('creatures', 'loot', 'items'))
        for path in self.paths:
            path.mkdir()
        self.item = ref('Item', 'synthetic:item')
        self.table = {'identity': ref('Loot', 'synthetic:loot'),
                      'kind': 'Loot',
                      'algorithm': 'IndependentBernoulliPpm',
                      'entries': [self.entry() for _ in range(7)]}
        self.creatures = {'schema': 'OTERYN_NATIVE_CREATURE_PROFILES/v1', 'records': []}
        self.definitions = []
        self.authoring = {}
        self.tables = [self.table]
        for index in range(27):
            self.add_creature(index)

    def entry(self):
        return {'item': self.item, 'min_count': 1, 'max_count': 1, 'probability_ppm': 1}

    def add_creature(self, index, *, prey=True, loot=None):
        target = ref('Creature', f'synthetic:creature-{index:02}')
        profile = {'experience': 1, 'bestiary': {'difficulty': 'trivial',
                   'occurrence': 'common', 'kill_thresholds': [10, 100, 250], 'charm_points': 5},
                   'details': {'system_eligibility': {'prey': prey}}}
        self.creatures['records'].append({'profile': {'target': target,
                                          'data': {'kind': 'Creature', 'profile': profile}}})
        self.definitions.append({'identity': target, 'loot': loot or self.table['identity']})
        self.authoring[target['key']] = {'kind': 'Creature', 'profile': copy.deepcopy(profile)}
        return target

    def ordinary(self):
        for path, family, records in zip(self.paths, ('Creature', 'Loot', 'Item'),
                (self.definitions, self.tables, [{'identity': self.item, 'materializable': True}])):
            rows = [{'definition': row} for row in records]
            if family == 'Creature':
                for row in rows:
                    row['authoring'] = self.authoring[row['definition']['identity']['key']]
            (path / f'{path.name}-0.json').write_bytes(full.encoded({'family': family, 'records': rows}))
        return full.loot_tables(self.creatures, *self.paths)

    def provider(self, pool=None):
        raw = self.ordinary()
        if pool is None:
            pool = [row['profile']['target'] for row in self.creatures['records']]
        return {'schema': 'OTERYN_PREY_LOOT_PROVIDER/v1',
                'creature_profiles_sha256': hashlib.sha256(full.encoded(self.creatures)).hexdigest(),
                'loot_tables_sha256': hashlib.sha256(raw).hexdigest(),
                'preyable_creatures': pool}

    def compile(self, provider=None):
        if provider is None:
            provider = self.provider()
        return full.loot_tables(self.creatures, *self.paths,
                               prey_provider=full.encoded(provider))

    def test_seven_entries_qualify_without_rewriting_native_output_or_inputs(self):
        provider = self.provider()
        before = copy.deepcopy((self.creatures, self.definitions, provider))
        self.assertEqual(self.compile(provider), self.ordinary())
        self.assertEqual((self.creatures, self.definitions, provider), before)

    def test_eight_entries_fail_closed_and_name_the_creature(self):
        self.table['entries'].append(self.entry())
        with self.assertRaisesRegex(ValueError, 'PREY_LOOT_TABLE_TOO_LARGE.*synthetic:creature-00'):
            self.compile()

    def test_zero_chance_and_repeated_item_entries_are_not_removed_from_budget(self):
        self.table['entries'] = [dict(self.entry(), probability_ppm=0) for _ in range(8)]
        with self.assertRaisesRegex(ValueError, 'PREY_LOOT_TABLE_TOO_LARGE'):
            self.compile()

    def test_oversized_creature_cannot_be_dropped_from_authored_pool(self):
        provider = self.provider()
        provider['preyable_creatures'].pop()
        with self.assertRaisesRegex(ValueError, 'PREY_POOL_MISMATCH.*synthetic:creature-26'):
            self.compile(provider)

    def test_explicit_native_exclusion_preserves_the_oversized_nonprey_table(self):
        excluded = dict(self.table, identity=ref('Loot', 'synthetic:excluded-loot'),
                        entries=[self.entry() for _ in range(8)])
        self.tables.append(excluded)
        self.add_creature(27, prey=False, loot=excluded['identity'])
        pool = [row['identity'] for row in self.definitions[:-1]]
        raw = self.compile(self.provider(pool))
        emitted = json.loads(raw)
        self.assertEqual(len(next(t for t in emitted['tables']
                         if t['identity'] == excluded['identity'])['entries']), 8)
        self.assertEqual(len(emitted['creature_loot']), 28)

    def test_unknown_native_prey_fact_is_not_a_false_exclusion(self):
        eligibility = self.creatures['records'][0]['profile']['data']['profile']['details']['system_eligibility']
        for value in (None, 'true', 1):
            with self.subTest(value=value):
                eligibility['prey'] = value
                with self.assertRaisesRegex(ValueError, 'PREY_ELIGIBILITY_UNKNOWN'):
                    self.compile()
        eligibility.clear()
        with self.assertRaisesRegex(ValueError, 'PREY_ELIGIBILITY_UNKNOWN'):
            self.compile()

    def test_zero_xp_or_absent_bestiary_cannot_enter_pool(self):
        profile = self.creatures['records'][0]['profile']['data']['profile']
        profile['experience'] = 0
        canonical = self.authoring['synthetic:creature-00']['profile']
        canonical['experience'] = 0
        with self.assertRaisesRegex(ValueError, 'PREY_POOL_MISMATCH'):
            self.compile()
        profile['experience'] = 1
        del profile['bestiary']
        canonical['experience'] = 1
        del canonical['bestiary']
        with self.assertRaisesRegex(ValueError, 'PREY_POOL_MISMATCH'):
            self.compile()

    def test_unknown_experience_and_empty_bestiary_refuse(self):
        profile = self.creatures['records'][0]['profile']['data']['profile']
        for value in (None, True, -1):
            profile['experience'] = value
            with self.assertRaisesRegex(ValueError, 'PREY_ELIGIBILITY_UNKNOWN'):
                self.compile()
        profile['experience'] = 1
        profile['bestiary'] = {}
        with self.assertRaisesRegex(ValueError, 'PREY_ELIGIBILITY_UNKNOWN'):
            self.compile()

    def test_both_exact_provider_digest_bindings_are_required(self):
        for field in ('creature_profiles_sha256', 'loot_tables_sha256'):
            provider = self.provider()
            provider[field] = '0' * 64
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'PREY_SOURCE_MISMATCH'):
                self.compile(provider)

    def test_wrong_revision_duplicates_and_unsorted_pool_refuse(self):
        for mutation in ('revision', 'duplicate', 'order'):
            provider = self.provider()
            pool = copy.deepcopy(provider['preyable_creatures'])
            if mutation == 'revision':
                pool[0]['revision'] = 'different-r2'
            elif mutation == 'duplicate':
                pool.append(copy.deepcopy(pool[0]))
            else:
                pool.reverse()
            provider['preyable_creatures'] = pool
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.compile(provider)

    def test_unadmitted_native_alias_does_not_become_no_loot_prey(self):
        self.definitions.pop()
        with self.assertRaisesRegex(ValueError, 'PREY_CREATURE_DEFINITION_MISSING.*synthetic:creature-26'):
            self.compile()

    def test_native_projection_cannot_hide_canonical_prey_eligibility(self):
        profile = self.creatures['records'][0]['profile']['data']['profile']
        original = copy.deepcopy(profile)
        for change in ('prey', 'bestiary', 'experience'):
            profile.clear()
            profile.update(copy.deepcopy(original))
            if change == 'prey':
                profile['details']['system_eligibility']['prey'] = False
            elif change == 'bestiary':
                del profile['bestiary']
            else:
                profile['experience'] = 0
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'PREY_SOURCE_MISMATCH'):
                self.compile()

    def test_missing_canonical_authoring_and_duplicate_canonical_identity_refuse(self):
        provider = self.provider()
        path = self.paths[0] / 'creatures-0.json'
        document = json.loads(path.read_bytes())
        del document['records'][0]['authoring']
        path.write_bytes(full.encoded(document))
        with self.assertRaisesRegex(ValueError, 'PREY_ELIGIBILITY_UNKNOWN'):
            self.compile(provider)
        provider = self.provider()
        document = json.loads(path.read_bytes())
        document['records'].append(copy.deepcopy(document['records'][0]))
        path.write_bytes(full.encoded(document))
        with self.assertRaisesRegex(ValueError, 'duplicate canonical Creature'):
            self.compile(provider)

    def test_nested_or_alternate_entries_never_hide_extra_rolls(self):
        for extra in ('contents_loot', 'alternatives'):
            entry = self.entry()
            entry[extra] = self.table['identity']
            self.table['entries'] = [entry]
            with self.subTest(extra=extra), self.assertRaisesRegex(ValueError, 'PREY_LOOT_UNSUPPORTED'):
                self.compile()

    def test_unknown_canonical_table_fields_cannot_be_projected_away(self):
        ordinary = self.ordinary()
        self.table['alternatives'] = [dict(self.table, entries=[self.entry() for _ in range(8)])]
        provider = self.provider()
        self.assertEqual(self.ordinary(), ordinary)  # The ordinary producer retains its contract.
        with self.assertRaisesRegex(ValueError, 'PREY_LOOT_UNSUPPORTED.*canonical Loot table'):
            self.compile(provider)

    def test_canonical_loot_kind_is_required_and_closed(self):
        self.assertEqual(self.compile(), self.ordinary())
        for kind in (None, 'Item', False):
            with self.subTest(kind=kind):
                if kind is None:
                    self.table.pop('kind', None)
                else:
                    self.table['kind'] = kind
                with self.assertRaisesRegex(ValueError, 'PREY_LOOT_UNSUPPORTED.*canonical Loot table'):
                    self.compile()

    def test_unknown_algorithm_and_boolean_quantity_refuse(self):
        self.table['algorithm'] = 'Unknown'
        with self.assertRaisesRegex(ValueError, 'PREY_LOOT_UNSUPPORTED'):
            self.compile()
        self.table['algorithm'] = 'IndependentBernoulliPpm'
        self.table['entries'][0]['max_count'] = True
        with self.assertRaisesRegex(ValueError, 'PREY_LOOT_UNSUPPORTED'):
            self.compile()

    def test_twenty_six_creatures_cannot_qualify_a_prey_generation(self):
        self.creatures['records'].pop()
        self.definitions.pop()
        with self.assertRaisesRegex(ValueError, 'PREY_POOL_TOO_SMALL'):
            self.compile()

    def test_empty_provider_and_false_enable_field_cannot_skip_gate(self):
        self.ordinary()
        with self.assertRaisesRegex(ValueError, 'PREY_PROVIDER_INVALID'):
            full.loot_tables(self.creatures, *self.paths, prey_provider=b'')
        provider = self.provider()
        provider['enabled'] = False
        with self.assertRaisesRegex(ValueError, 'PREY_PROVIDER_INVALID'):
            self.compile(provider)

    def test_prey_provider_without_native_loot_path_refuses_before_output(self):
        out = self.root / 'output'
        args = SimpleNamespace(out=out, prey_provider=self.root / 'provider.json', progression=None)
        with self.assertRaisesRegex(ValueError, 'PREY provider requires the progression/loot path'):
            full.build(args)
        self.assertFalse(out.exists())


if __name__ == '__main__':
    unittest.main()
