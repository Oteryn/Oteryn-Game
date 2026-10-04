import copy
import gzip
import json
from pathlib import Path
import random
import subprocess
import tempfile
import unittest

from jsonschema import Draft202012Validator, ValidationError

import source_monk_formula_library as library

REPO = Path(__file__).resolve().parents[3]
DONOR = Path('/workspace/spell-sources/canary')


class MonkFormulaLibraryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = library.build(REPO, DONOR.parent)
        cls.helper = library.SourceTierEvaluator(library.load_program(REPO))
        cls.temporary = tempfile.TemporaryDirectory(prefix='source-monk-numeric-oracle-')
        cls.oracle = Path(cls.temporary.name) / 'oracle'
        cpp = library.source(DONOR, 'src/creatures/players/player.cpp').decode()
        signature = 'uint16_t Player::calculateFlatDamageHealing() const {'
        start = cpp.index(signature)
        end = cpp.index('\n}', start) + 2
        method = cpp[start:end].replace('Player::', '')
        # Source C++ helper is compiled as-is inside a minimal test holder.
        # Callback arithmetic comes from the actual Lua source lines, with only
        # declarations and identifier substitutions for the independent C++ oracle.
        lua = library.source(DONOR, 'data/scripts/spells/attack/swift_jab.lua').decode()
        equations = []
        for line in lua.splitlines():
            line = line.strip()
            if any(line.startswith('local ' + name + ' =') for name in ('damage', 'min', 'max')):
                equations.append(line.replace('local ', 'const double ', 1).replace('SPELL_BASE_POWER', 'power') + ';')
        if len(equations) != 3:
            raise AssertionError('exact source callback oracle equations absent')
        source = '#include <cstdint>\n#include <limits>\n#include <cmath>\n#include <iostream>\n#include <iomanip>\n'
        source += 'class SourcePlayer { public: uint32_t level;\n' + method + '\n};\n'
        source += 'int main(){ std::cout << std::setprecision(17); char mode; uint32_t level; while(std::cin >> mode >> level){ SourcePlayer player{level}; '
        source += "if(mode=='H'){std::cout << player.calculateFlatDamageHealing() << '\\n';}else{ double power,skill,attack; std::cin >> power >> skill >> attack; const double damageHealing=player.calculateFlatDamageHealing();"
        source += '\n'.join(equations) + "std::cout << min << ' ' << max << '\\n';}}}\n"
        path = Path(cls.temporary.name) / 'oracle.cpp'
        path.write_text(source)
        subprocess.run(['g++', '-std=c++17', '-O2', '-ffp-contract=off', str(path), '-o', str(cls.oracle)], check=True, capture_output=True)

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    def oracle_results(self, requests):
        result = subprocess.run([str(self.oracle)], input='\n'.join(requests) + '\n', text=True, capture_output=True, check=True)
        return result.stdout.splitlines()

    def test_all_uint16_levels_match_exact_source_cpp_helper(self):
        actual = self.oracle_results(['H ' + str(level) for level in range(65536)])
        self.assertEqual(65536, len(actual))
        for level, answer in enumerate(actual):
            self.assertEqual(int(answer), self.helper.evaluate(level), 'level=' + str(level))

    def test_every_tier_boundary_and_uint16_cap_through_million(self):
        levels = {0, 1, 499, 500, 501, 999999, 1000000}
        for boundary in self.helper.bounds:
            levels.update(level for level in (boundary - 1, boundary, boundary + 1) if 0 <= level <= library.MAX_LEVEL)
        levels.update(random.Random(50).sample(range(library.MAX_LEVEL + 1), 512))
        levels = sorted(levels)
        answers = self.oracle_results(['H ' + str(level) for level in levels])
        for level, answer in zip(levels, answers):
            self.assertEqual(int(answer), self.helper.evaluate(level), 'level=' + str(level))
        self.assertEqual(65535, self.helper.evaluate(1000000))
        self.assertEqual(100, self.helper.evaluate(500))
        self.assertEqual(284, self.helper.evaluate(1100))
        self.assertNotEqual(1100 // 5, self.helper.evaluate(1100))

    def test_all_six_callback_pairs_match_source_equations_and_preserve_sign(self):
        examples = [(0, 0, 7), (14, 10, 20), (499, 130, 100), (500, 130, 100),
                    (1100, 130, 100), (1000000, 2147483647, 2147483647),
                    (100, -2147483648, 2147483647), (100, 2147483647, -2147483648)]
        requests, expected = [], []
        for record in self.data['formula_definitions']:
            for level, skill, attack in examples:
                requests.append(f'F {level} {record["base_power"]} {skill} {attack}')
                expected.append(library.evaluate_formula(record, {'level': level, 'attack_skill': skill, 'attack_value': attack}, self.helper))
        for pair, answer in zip(expected, self.oracle_results(requests)):
            self.assertEqual(tuple(float(value) for value in answer.split()), pair)

    def test_library_reference_and_inactive_flags_validate(self):
        library.validate_library(self.data, REPO)
        self.assertEqual(6, self.data['formula_count'])
        self.assertEqual(0, self.data['complete_spell_candidates'])
        self.assertNotIn('tier_program', self.data['helpers'][0])
        self.assertFalse(self.data['input_provider']['live_input_production_qualified'])
        self.assertFalse(self.data['input_provider']['downstream_execution_qualified'])
        self.assertEqual(library.PROGRAM_SHA, self.data['helpers'][0]['program_reference']['program_sha256'])
        for row in self.data['formula_definitions']:
            self.assertTrue(row['formula_data_complete'])
            self.assertFalse(row['complete_spell_candidate'])
            self.assertFalse(row['native_execution_qualified'])

    def test_unknown_helpers_ops_inputs_and_runtime_flags_fail_schema(self):
        for mutation in ('helper', 'operation', 'input', 'runtime', 'extension'):
            modified = copy.deepcopy(self.data)
            if mutation == 'helper':
                modified['formula_definitions'][0]['expressions']['minimum'] = {'helper_ref': 'unowned_helper', 'args': [{'var': 'level'}]}
            elif mutation == 'operation':
                modified['formula_definitions'][0]['expressions']['minimum'] = {'op': 'execute', 'args': [{'const': '1'}, {'const': '2'}]}
            elif mutation == 'input':
                modified['formula_definitions'][0]['expressions']['minimum'] = {'var': 'harmony'}
            elif mutation == 'runtime':
                modified['runtime_activation'] = True
            else:
                modified['helpers'][0]['source_lua'] = 'return 1'
            with self.subTest(mutation=mutation), self.assertRaises(ValidationError):
                library.validate_library(modified, REPO)

    def test_changed_formula_or_duplicate_population_fails_linkage(self):
        for mutation in ('coefficient', 'duplicate', 'program_sha', 'provider_sha'):
            modified = copy.deepcopy(self.data)
            if mutation == 'coefficient':
                modified['formula_definitions'][0]['expressions']['minimum'] = {'const': '99'}
            elif mutation == 'duplicate':
                modified['formula_definitions'][0] = copy.deepcopy(modified['formula_definitions'][1])
            elif mutation == 'program_sha':
                modified['helpers'][0]['program_reference']['program_sha256'] = '0' * 64
            else:
                modified['input_provider']['source_proofs'][0]['sha256'] = '0' * 64
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                library.validate_library(modified, REPO)

    def test_domain_boundaries_and_program_mutation_are_refused(self):
        for level in (-1, 1000001, 3.5, True):
            with self.subTest(level=level), self.assertRaises(ValueError):
                self.helper.evaluate(level)
        program = library.load_program(REPO)
        program['initializers'][3]['value'] = {'const': '501'}
        with self.assertRaises(ValueError):
            library.SourceTierEvaluator(program)
        record = self.data['formula_definitions'][0]
        with self.assertRaises(ValueError):
            library.evaluate_formula(record, {'level': 100, 'attack_skill': 100, 'attack_value': 2147483648}, self.helper)

    def test_gzip_packet_is_deterministic_and_references_imported_bytes(self):
        payload = library.canonical(self.data) + b'\n'
        self.assertEqual(gzip.compress(payload, mtime=0), gzip.compress(library.canonical(library.build(REPO, DONOR.parent)) + b'\n', mtime=0))
        self.assertEqual(library.PROGRAM_FILE_SHA, library.sha((REPO / library.PROGRAM_PATH).read_bytes()))
        Draft202012Validator.check_schema(json.loads(library.SCHEMA_PATH.read_text()))


if __name__ == '__main__':
    unittest.main()
