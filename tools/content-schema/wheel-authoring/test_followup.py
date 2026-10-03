import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from jsonschema.exceptions import ValidationError
import gem_revisions
import wheel_authoring as wheel


class FollowupTests(unittest.TestCase):
    def setUp(self):
        self.previous_bytes = (wheel.ROOT / 'samples/wheel-candidate.json').read_bytes()
        self.candidate = wheel.decode_json(self.previous_bytes)

    def effects(self, candidate):
        for data in candidate['vocations'].values():
            for slot in data['slots']:
                conviction = slot['conviction']
                if conviction['unique_parameters']:
                    yield from conviction['unique_parameters']['numeric_effects']
                for stage in conviction['augment_stages']:
                    yield from stage['numeric_effects']
            for revelation in data['revelations']:
                for stage in revelation['stages']:
                    yield from stage['numeric_effects']
        for mod in candidate['gems']['supreme_mods']:
            for grade in mod['grades']:
                yield from grade['numeric_effects']

    def test_counts_and_probabilities_refuse_impossible_values(self):
        for unit in ('targets', 'tiles', 'creatures', 'points', 'mana'):
            candidate = copy.deepcopy(self.candidate)
            next(e for e in self.effects(candidate) if e['unit'] == unit)['value'] = 0.5
            with self.subTest(unit=unit), self.assertRaises(ValidationError):
                wheel.validate(candidate)
        for kind in ('momentum_chance', 'critical_hit_chance', 'rune_trigger_chance',
                     'damage_reduction', 'next_attack_damage_reduction', 'shared_mantra_percent',
                     'dodge', 'exclusive_target_health_limit', 'missing_health_step', 'damage_reduction_per_step'):
            candidate = copy.deepcopy(self.candidate)
            next(e for e in self.effects(candidate) if e['kind'] == kind)['value'] = 101
            with self.subTest(kind=kind), self.assertRaises(ValidationError):
                wheel.validate(candidate)

    def test_large_damage_bonus_and_fractional_seconds_remain_valid(self):
        candidate = copy.deepcopy(self.candidate)
        next(e for e in self.effects(candidate) if e['kind'] == 'critical_extra_damage')['value'] = 300
        next(e for e in self.effects(candidate) if e['kind'] == 'cooldown_reduction')['value'] = 0.5
        wheel.validate(candidate)

    def test_original_official_unique_effects_are_complete(self):
        convictions = {s['conviction']['key']: s['conviction']
                       for v in self.candidate['vocations'].values() for s in v['slots']}
        def values(key):
            return {e['kind']: e['value'] for e in convictions[key]['unique_parameters']['numeric_effects']}
        self.assertEqual(values('battle_healing')['shield_healing_multiplier'], 2)
        self.assertEqual(values('guiding_presence'), {'shared_mantra_percent': 100,
            'party_bonus_increase_source_percent': 33})
        self.assertEqual(values('focus_mastery')['focus_spell_group_cooldown_reduction'], 2)
        self.assertNotIn('next_damage_spell_primary_cooldown_reduction', values('focus_mastery'))
        self.assertEqual(convictions['focus_mastery']['unique_parameters']['targets'],
                         ["Hell's Core", 'Rage of the Skies'])
        self.assertEqual(values('healing_link')['self_healing_from_applied_target_heal'], 25)

    def test_manual_path_hash_and_result_are_bound(self):
        evidence = wheel.read(wheel.ROOT / 'samples/verification-evidence.json')
        for field, value, error in [('reference', '../../other.md#517', 'EVIDENCE_MANUAL_PATH'),
                ('notes_sha256', '0' * 64, 'EVIDENCE_MANUAL_DIGEST'),
                ('result', 'UNVERIFIED', 'EVIDENCE_MANUAL_RESULT')]:
            packet = copy.deepcopy(evidence)
            packet['official_manual_notes'][field] = value
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / 'evidence.json'
                path.write_text(json.dumps(packet))
                with self.subTest(field=field), self.assertRaisesRegex(ValueError, error):
                    wheel.validate_evidence(self.candidate,
                        (wheel.ROOT / 'samples/wheel-candidate.json').read_bytes(), path)

    def successor(self):
        candidate = copy.deepcopy(self.candidate)
        candidate.update(revision='r2', release={'kind': 'value_only',
                                               'predecessor': self.candidate['revision'],
            'predecessor_sha256': hashlib.sha256(self.previous_bytes).hexdigest()})
        return candidate

    def test_gem_change_requires_its_own_declaration(self):
        for mutate in (lambda c: c['gems']['grade_costs'][0]['basic'].update(gold=1),
                       lambda c: c['gems']['basic_mods'][0]['effects'][0]['values_by_vocation']['knight'].__setitem__(0, 99)):
            candidate = self.successor()
            mutate(candidate)
            with self.subTest(mutate=mutate), self.assertRaisesRegex(ValueError, 'GEM_REVISION_DECLARATION_REQUIRED'):
                wheel.validate(candidate, self.candidate, self.previous_bytes)

    def write_declaration(self, root, candidate, kind):
        path = root / 'samples/gem-revisions/test.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        record = {'schema': 'OTERYN_WHEEL_GEM_REVISION_REFERENCE/v1', 'kind': kind,
                  'source_revision': self.candidate['revision'], 'destination_revision': candidate['revision'],
                  'source_gem_sha256': gem_revisions.contract_digest(gem_revisions.gem_contract(self.candidate)),
                  'destination_gem_sha256': gem_revisions.contract_digest(gem_revisions.gem_contract(candidate)),
                  'runtime_admitted': False, 'runtime_validation': 'PENDING_GEM_R_ADMISSION'}
        if kind == 'staged_migration':
            record['native_migration_reference'] = 'GEM-R pending DUR-02 staged migration plan'
        path.write_text(json.dumps(record))
        candidate['release']['gem_revision'] = {'kind': kind, 'reference': 'samples/gem-revisions/test.json',
                                               'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        return path

    def test_compatible_fee_change_and_staged_value_change(self):
        for kind in ('declared_compatible', 'staged_migration'):
            candidate = self.successor()
            if kind == 'declared_compatible':
                candidate['gems']['grade_costs'][0]['basic']['gold'] += 1
            else:
                candidate['gems']['basic_mods'][0]['effects'][0]['values_by_vocation']['knight'][0] += 1
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.write_declaration(root, candidate, kind)
                with patch.object(gem_revisions, 'ROOT', root):
                    wheel.validate(candidate, self.candidate, self.previous_bytes)

    def test_duplicate_json_fields_are_rejected_at_every_depth(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'candidate.json'
            for payload in ('{"runtime_admitted":true,"runtime_admitted":false}',
                            '{"release":{"kind":"initial","kind":"wheel_reset"}}',
                            '{"value":1,"v\\u0061lue":2}'):
                path.write_text(payload)
                with self.subTest(payload=payload), self.assertRaisesRegex(ValueError,'DUPLICATE_JSON_KEY'):
                    wheel.read(path)

    def test_wiki_proof_requires_the_actual_host_and_real_capture(self):
        original_read=wheel.read
        original=original_read(wheel.ROOT/'samples/browser-source-audit.json')
        def wrong_url(browser,url):
            for observation in browser['observations']:
                if observation['url'].startswith('https://tibia.fandom.com/'):
                    observation['url']=url+'/wiki/'+observation['url'].split('/wiki/')[1]
        mutations=[(lambda b,u=u:wrong_url(b,u),'EVIDENCE_BROWSER_WIKI_OBSERVATIONS') for u in
            ('https://evil.example/tibia.fandom.com','https://tibia.fandom.com.evil.example',
             'https://tibia.fandom.com@evil.example','http://tibia.fandom.com')]
        index=next(i for i,o in enumerate(original['observations']) if o['url'].startswith('https://tibia.fandom.com/'))
        mutations += [(lambda b:b['observations'][index].update(requested_url='https://evil.example/wiki/Lesser_Gem'),'EVIDENCE_BROWSER_WIKI_REQUEST'),
            (lambda b:b['observations'][index].update(characters=0),'EVIDENCE_BROWSER_WIKI_CAPTURE'),
            (lambda b:b['observations'][index].update(revision='NOT_EXPOSED'),'EVIDENCE_BROWSER_WIKI_CAPTURE'),
            (lambda b:b['observations'][index].update(sha256='unverified'),'EVIDENCE_BROWSER_WIKI_CAPTURE'),
            (lambda b:b['observations'][index].update(status='FAILED'),'EVIDENCE_BROWSER_WIKI_READ'),
            (lambda b:b['observations'].clear(),'EVIDENCE_BROWSER_WIKI_OBSERVATIONS')]
        for mutate,code in mutations:
            browser=copy.deepcopy(original);mutate(browser)
            def supplied(path):return browser if path.name=='browser-source-audit.json' else original_read(path)
            with self.subTest(code=code), patch.object(wheel,'read',side_effect=supplied), self.assertRaisesRegex(ValueError,code):
                wheel.validate_evidence(self.candidate,(wheel.ROOT/'samples/wheel-candidate.json').read_bytes())

    def test_compatible_declaration_cannot_reinterpret_paid_mods(self):
        candidate = self.successor()
        candidate['gems']['basic_mods'][0]['effects'][0]['values_by_vocation']['knight'][0] += 1
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_declaration(root, candidate, 'declared_compatible')
            with patch.object(gem_revisions, 'ROOT', root), self.assertRaisesRegex(ValueError, 'GEM_COMPATIBLE_ROW_CHANGED'):
                gem_revisions.validate_gem_revision(candidate, self.candidate)

    def test_declaration_digest_chain_and_native_plan_are_checked(self):
        for defect, error in [('digest', 'GEM_REVISION_DIGEST'), ('chain', 'GEM_REVISION_CHAIN'),
                              ('plan', 'GEM_NATIVE_MIGRATION_REFERENCE'), ('admission', 'GEM_REVISION_ADMISSION')]:
            candidate = self.successor()
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                path = self.write_declaration(root, candidate, 'staged_migration')
                record = json.loads(path.read_text())
                if defect == 'chain': record['destination_revision'] = 'wrong'
                if defect == 'plan': record.pop('native_migration_reference')
                if defect == 'admission': record['runtime_admitted'] = True
                path.write_text(json.dumps(record))
                if defect != 'digest':
                    candidate['release']['gem_revision']['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
                else:
                    candidate['release']['gem_revision']['sha256'] = '0' * 64
                with patch.object(gem_revisions, 'ROOT', root), self.subTest(defect=defect), self.assertRaisesRegex(ValueError, error):
                    gem_revisions.validate_gem_revision(candidate, self.candidate)
