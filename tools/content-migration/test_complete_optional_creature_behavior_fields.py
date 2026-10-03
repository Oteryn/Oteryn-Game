import copy
import gzip
import json
import sys
import tempfile
import unittest
from pathlib import Path

import complete_optional_creature_behavior_fields as completion

ROOT = Path(__file__).resolve().parents[2]
POP = Path('/workspace/monster-field-fill-20261002/population')
DONOR = Path('/workspace/monster-reference-sources/canary')


class OptionalCreatureBehaviorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.packet = completion.prepare(POP, DONOR, cls.tmp.name)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def applied(self):
        changed = {}
        for p in self.packet['patches']:
            files = changed.setdefault(p['monster'], {f: completion.read(POP/'bundles'/p['monster']/f) for f in ['monster.json','dependencies.json','catalog.json']})
            target = files[p['file']]
            for part in p['pointer'].strip('/').split('/')[:-1]:
                target = target[part]
            self.assertFalse(p['expected_present'])
            self.assertIsNone(p['expected_value'])
            self.assertNotIn(p['value'], target)
            target.append(copy.deepcopy(p['value']))
        return changed

    def test_schema_native_profiles_and_registered_target_closure(self):
        sys.path.insert(0, str(ROOT/'tools/content-schema/monster-authoring'))
        import validate_monster
        import creature_admission_stage as native
        targets = {completion.read(POP/'bundles'/name/'monster.json')['creature']['identity']['key'] for name in ['poor_soul','dragon_egg']}
        for name, docs in self.applied().items():
            self.assertEqual(validate_monster.validate(docs['monster.json'], docs['dependencies.json'], docs['catalog.json']), [], name)
            stage = native.Stage(native.Mapper({}))
            stage.stage_dependencies(docs['dependencies.json'], name)
            named = [p['data']['profile']['affects'] for (family, key), p in stage.profiles.items() if family=='Effect' and 'affects' in p['data']['profile']]
            self.assertEqual(len(named), 1)
            self.assertEqual(named[0]['kind'], 'NamedCreatures')
            for e in docs['dependencies.json']['effects']:
                for ref in e.get('affects', {}).get('creatures', []):
                    self.assertIn(ref['key'], targets)

    def test_exact_schedule_source_area_and_narrow_egg_filter(self):
        changed = self.applied()
        for name, section, chance, matrix in [('a_greedy_eye','attacks',100,['x','x','C']), ('icicle','defenses',60,['..xxx..','.xxxxx.','xxxxxxx','xxxCxxx','xxxxxxx','.xxxxx.','..xxx..'])]:
            self.assertEqual(changed[name]['monster.json']['behavior'][section][-1]['interval_ms'],2000)
            self.assertEqual(changed[name]['monster.json']['behavior'][section][-1]['chance_percent'],chance)
            self.assertEqual(changed[name]['dependencies.json']['abilities'][-1]['area']['matrix']['north'],matrix)
        effect = changed['icicle']['dependencies.json']['effects'][-1]
        self.assertEqual(effect['affects']['kind'],'named_creatures')
        self.assertEqual(effect['affects']['creatures'],[completion.reference('Creature','dragon_egg')])
        self.assertFalse(effect['affects']['includes_caster'])
        self.assertEqual(changed['icicle']['dependencies.json']['formulas'][-1]['magnitude'], {'minimum':100,'maximum':100})

    def test_source_proof_and_proxy_never_claim_global_parity(self):
        for p in self.packet['patches']:
            src = p['source']
            self.assertFalse(src['global_parity'])
            self.assertEqual(src['qualification'],'OWNER_ACCEPTED_NON_GLOBAL_HP_DELTA_PROXY')
            self.assertEqual(src['revision'], completion.PIN)
            self.assertGreater(src['source_line'],0)
            self.assertEqual(len(src['sha256']),64)
            proof = completion.source_evidence(DONOR,src['source_file'])
            self.assertEqual(proof['sha256'],src['sha256'])
        for flags in self.packet['actor_flags'].values():
            self.assertIn('SOURCE_BEHAVIOR_PARTIAL',flags)
            self.assertIn('SOURCE_NON_GLOBAL_PROXY',flags)

    def test_complete_positive_field_census_without_duplicate_passive_reflection(self):
        audit = json.load(gzip.open(Path(self.tmp.name)/'source-positive-audit.json.gz','rt'))
        self.assertEqual(audit['mapped_destination_gaps'],[])
        for field, count in [('reflects[',81), ('heals[',20)]:
            selected=[r for r in audit['observations'] if r['source_field'].startswith(field)]
            self.assertEqual(len(selected),count)
            self.assertTrue(all(r['state']=='SOURCE_POSITIVE_PRESENT' for r in selected))
        self.assertEqual(audit['counts']['damage_reflection_present'],1763)
        self.assertFalse(any('damage_reflection' in p['pointer'] for p in self.packet['patches']))

    def test_all_718_non_narrative_census_cells_have_concrete_dispositions(self):
        ledger=json.load(gzip.open(Path(self.tmp.name)/'census-dispositions.json.gz','rt'))
        self.assertEqual(len(ledger['rows']),718)
        self.assertEqual(ledger['counts'],{'TYPED_CORE_RESTORED_NON_GLOBAL_PROXY':5,'ITEM_ADMISSION_LANE_REVIEW':3,'STATISTICS_LANE_REVIEW':10,'NO_NUMERIC_FORMULA_APPLICABLE':2,'ARCHITECT_DOMAIN_162':697,'SOURCE_EMPTY_TEXT_NOOP':1})
        noops=[r for r in ledger['rows'] if r['disposition'] in {'NO_NUMERIC_FORMULA_APPLICABLE','SOURCE_EMPTY_TEXT_NOOP'}]
        self.assertEqual({r['monster'] for r in noops},{'deaththrower','incredibly_old_witch','the_rootkraken'})
        self.assertTrue(all(r['proof'] for r in ledger['rows']))

    def test_deterministic_receipt_and_baseline_drift_guard(self):
        with tempfile.TemporaryDirectory() as other:
            completion.prepare(POP,DONOR,other)
            self.assertEqual(completion.sha(Path(other)/'fieldpatch.json'),completion.sha(Path(self.tmp.name)/'fieldpatch.json'))
        self.assertEqual(completion.sha(POP/'population-index.json'),completion.BASELINE)
        with tempfile.TemporaryDirectory() as invalid:
            path=Path(invalid);(path/'population-index.json').write_text('{}')
            with self.assertRaisesRegex(ValueError,'Unexpected baseline'):
                completion.prepare(path,DONOR,path/'out')
            self.assertFalse((path/'out').exists())


if __name__ == '__main__':
    unittest.main()
