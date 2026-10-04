"""A newer successful reference capture must not make older probes lose profiles."""
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import canary_batch as cb


class ProbeFailureTests(unittest.TestCase):
    def converter(self):
        converter = cb.Converter.__new__(cb.Converter)
        converter.current_slug = 'reference_probe_regression'
        converter.spell_scripts = SimpleNamespace(evaluate=lambda name: {
            'kind': 'instant', 'script': 'data/scripts/spells/monster/regression.lua',
            'tier': 'P4', 'tier_reasons': ['custom cast'], 'spell_calls': {},
            'reference_combats': {0: {'params': {'COMBAT_PARAM_TYPE': 'COMBAT_FIRE'}}},
        })
        converter.behaviour_patterns = lambda: {'regression': 'conditional_summon'}
        return converter

    def test_failed_behavior_probe_keeps_slot_explicitly_unresolved(self):
        converter = self.converter()
        with patch.object(converter, 'probed_spell', side_effect=RuntimeError('missing pinned include')):
            result = converter.registered_spell({'name': 'regression'}, {}, None)
        self.assertEqual(result[0], 'UNRESOLVED')
        self.assertIn('source behavior probe failed: RuntimeError: missing pinned include', result[1])
        self.assertIn('needs a native behaviour', result[1])

    def test_successful_existing_behavior_probe_still_returns_its_result(self):
        converter = self.converter()
        mapped = ('canary:ability/regression', 'existing proven behavior', {})
        with patch.object(converter, 'probed_spell', return_value=(mapped, '')):
            result = converter.registered_spell({'name': 'regression'}, {}, None)
        self.assertEqual(result, mapped)


if __name__ == '__main__':
    unittest.main()
