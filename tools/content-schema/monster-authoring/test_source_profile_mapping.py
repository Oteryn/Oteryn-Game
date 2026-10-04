"""Focused mapper fidelity checks using already-staged pinned source inputs."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch
from jsonschema import Draft202012Validator
import canary_batch as cb

SOURCE = Path('/workspace/spells-r22-monster-import-current/source-inputs/canary')


class SourceProfileMappingTests(unittest.TestCase):
    def converter(self, directory='data-otservbr-global/monster'):
        if not (SOURCE / cb.EFFECT_CONSTANTS).exists():
            self.skipTest('existing local source stage unavailable')
        result = cb.Converter(SOURCE, {}, {}, {}, {})
        result.monster_dir = directory
        result.pending_definitions = set()
        return result

    def test_actual_demon_source_sound_schedule_preserved_without_runtime_cues(self):
        converter = self.converter('data-canary/monster')
        source_path = SOURCE / converter.monster_dir / 'demons/demon.lua'
        original = source_path.read_bytes()
        result = converter.convert('demons/demon')
        audio = result[1]['presentation']['audio']
        self.assertEqual(audio['event_bindings'], [])
        expected = {'cue_binding_status': 'source_only_unbound', 'interval_ms': 5000, 'chance_pct': 10,
                    'death_constant': 'SOUND_EFFECT_TYPE_DEMON_DEATH',
                    'idle_constants': ['SOUND_EFFECT_TYPE_DEMON_BARK', 'SOUND_EFFECT_TYPE_UNKNOWN_CREATURE_DEATH_1']}
        self.assertEqual(audio['source_sound_schedule'], expected)
        rows = [r for r in result[4]['entries'] if r['source_field'] == 'sounds']
        self.assertEqual(rows[0]['status'], 'mapped')
        self.assertEqual(rows[0]['destination'], '/monster/presentation/audio/source_sound_schedule')
        self.assertEqual(source_path.read_bytes(), original)
        schema = json.loads((Path(__file__).parent / 'monster.schema.json').read_text())
        Draft202012Validator(schema['$defs']['presentation']['properties']['audio']['properties']['source_sound_schedule']).validate(expected)

    def test_heal_and_reflection_rows_point_to_actual_preserved_values(self):
        for relative, field, target, percentage in [('elementals/lava_lurker', 'heals', 'healing_from_damage', 100),
                                                    ('humanoids/crazed_summer_rearguard', 'reflects', 'damage_reflection', 70)]:
            with self.subTest(field=field):
                result = self.converter().convert(relative)
                row = next(r for r in result[4]['entries'] if r['source_field'] == field + '[1]')
                self.assertEqual(row['status'], 'mapped')
                self.assertEqual(row['destination'], '/monster/creature/' + target + '/0')
                value = result[1]['creature'][target][0]
                self.assertEqual(value, {'damage_type': 'fire', 'percent': {'numerator': percentage, 'denominator': 1}})

    def test_invalid_sound_input_retained_as_unresolved_without_fabricated_binding(self):
        converter = self.converter('data-canary/monster')
        path = SOURCE / converter.monster_dir / 'demons/demon.lua'
        name, monster, callbacks = cb.load_monster(path, [])
        monster = copy.deepcopy(monster)
        monster['sounds']['death'] = '@CONST_ME_FIREAREA'
        with patch.object(cb, 'load_monster', return_value=(name, monster, callbacks)):
            result = converter.convert('demons/demon')
        self.assertNotIn('source_sound_schedule', result[1]['presentation']['audio'])
        row = next(r for r in result[4]['entries'] if r['source_field'] == 'sounds')
        self.assertEqual(row['status'], 'unresolved_semantics')

    def test_sound_schedule_refuses_numeric_coercions_and_unknown_fields(self):
        for source in ({'death': 200}, {'ticks': 5000}, {'ticks': True, 'chance': 10, 'ids': ['@SOUND_EFFECT_TYPE_DEMON_BARK']},
                       {'death': '@SOUND_EFFECT_TYPE_DEMON_DEATH', 'made_up': 1}):
            with self.subTest(source=source), self.assertRaises(ValueError):
                cb.source_sound_schedule(source)


if __name__ == '__main__':
    unittest.main()
