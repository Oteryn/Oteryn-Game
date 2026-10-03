"""Authority, silence and sound-choice boundaries for field completion."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('audio_completion', Path(__file__).with_name('complete_monster_audio_fields.py'))
audio = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audio)


class AudioAuthorityTests(unittest.TestCase):
    def test_owner_selection_is_reproducible_and_only_selects_allowed_sound(self):
        identity, pin, choices = 'canary:creature/cat', 'abc123', [8, 17, 42]
        chosen, proof = audio.deterministic_selection(identity, pin, choices)
        digest = hashlib.sha256(b'canary:creature/cat\0abc123').hexdigest()
        self.assertEqual(proof['sha256'], digest)
        self.assertEqual(chosen, choices[int(digest, 16) % 3])
        self.assertEqual(proof['candidates'], choices)
        self.assertEqual(proof['qualification'], 'OWNER_ACCEPTED_NON_GLOBAL_SOURCE_SOUND_SELECTION')
        self.assertEqual((chosen, proof), audio.deterministic_selection(identity, pin, choices))
        for invalid in ([], [8, 8]):
            with self.assertRaises(ValueError):
                audio.deterministic_selection(identity, pin, invalid)

    def test_actual_manifest_repository_wins_over_identity_namespace(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            relative = 'data/monster/cat.lua'
            path = root / 'crystal' / relative
            path.parent.mkdir(parents=True)
            path.write_text('monster = {}')
            manifest = {'sources': [{'repository': 'zimbadev/crystalserver'}],
                        'entries': [{'source_index': 0, 'source_file': relative}]}
            repo, actual, rel = audio.choose_donor({'monster': 'canary-cat'}, manifest, root)
            self.assertEqual(repo, 'zimbadev/crystalserver')
            self.assertEqual(actual, path)
            self.assertEqual(rel, relative)

    def test_partial_quest_source_keeps_sound_literals_without_guessing_interval(self):
        text = 'monster.attacks = {\n{name="combat", interval=Quest.unknown * 1000, type=COMBAT_FIREDAMAGE, soundCast=SOUND_EFFECT_TYPE_FIRE, radius=3},\n}\nmonster.defenses = {}'
        rows = audio.static_sound_rows(text, 'attacks')
        self.assertEqual(rows, [{'name': 'combat', 'type': '@COMBAT_FIREDAMAGE', 'soundCast': '@SOUND_EFFECT_TYPE_FIRE', 'radius': 3}])
        self.assertNotIn('interval', rows[0])

    def test_source_file_drift_is_rejected_even_when_git_head_is_pinned(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            path = root / 'source.lua'
            path.write_text('return 17\n')
            subprocess.run(['git', 'add', 'source.lua'], cwd=root, check=True)
            subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'fixture'], cwd=root, check=True)
            pin = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
            self.assertEqual(audio.verify_pinned_file(root, pin, 'source.lua'), path)
            path.write_text('return 42\n')
            with self.assertRaisesRegex(ValueError, 'drifted'):
                audio.verify_pinned_file(root, pin, 'source.lua')

    def test_routing_correction_only_marks_default_impact(self):
        proof = {'loader_sha256': 'abc', 'impact_routing_lines': [958]}
        correction = audio.routing_correction({}, {'impact_cue': 'canary.sound:effect/17'}, proof)
        self.assertEqual(correction['qualification'], 'SOURCE_NON_GLOBAL_AUDIO_ROUTING_CORRECTION')
        self.assertEqual(correction['donor_call'], 'spell:castSound(sounds.impact)')
        self.assertEqual(correction['lines'], [958])
        self.assertIsNone(audio.routing_correction({'explicit_impact': True}, {'impact_cue': 'x'}, proof))
        self.assertIsNone(audio.routing_correction({}, {'cast_cue': 'x'}, proof))

    def test_pure_source_lookup_preserves_rng_choices_and_silence(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            loader, header = root / 'loader.lua', root / 'enum.hpp'
            header.write_text('enum SoundEffect_t : uint16_t { SILENCE = 0, FIRE = 17, MELEE_A = 8, MELEE_B = 9, MELEE_C = 10 };')
            loader.write_text('''local smallAreaRadius = 3
local superDrunkDuration = 4000
local function loadcastSound(row) return SOUND_EFFECT_TYPE_SILENCE end
local function loadSpellSoundType(row, mt)
 if row.name == "silent" then return {cast=SOUND_EFFECT_TYPE_SILENCE, impact=SOUND_EFFECT_TYPE_SILENCE} end
 if row.name == "melee" then
  local allowed = {SOUND_EFFECT_TYPE_MELEE_A,SOUND_EFFECT_TYPE_MELEE_B,SOUND_EFFECT_TYPE_MELEE_C}
  return {cast=SOUND_EFFECT_TYPE_SILENCE, impact=allowed[math.random(1,mt:targetDistance() > 1 and 2 or 3)]}
 end
 return {cast=SOUND_EFFECT_TYPE_FIRE, impact=SOUND_EFFECT_TYPE_SILENCE}
end
function readSpell(row) end
''')
            defaults = audio.SoundDefaults(loader, header)
            self.assertEqual(defaults.choices({'name': 'silent'}, 1), {'cast_cue': [], 'impact_cue': []})
            self.assertEqual(defaults.choices({'name': 'combat'}, 1), {'cast_cue': [17], 'impact_cue': []})
            self.assertEqual(defaults.choices({'name': 'melee'}, 1)['impact_cue'], [8, 9, 10])
            self.assertEqual(defaults.choices({'name': 'melee'}, 4)['impact_cue'], [8, 9])
            with self.assertRaisesRegex(ValueError, 'Unknown donor sound enum'):
                defaults.choices({'name': 'combat', 'soundCast': '@SOUND_EFFECT_TYPE_UNKNOWN'}, 1)


if __name__ == '__main__':
    unittest.main()
