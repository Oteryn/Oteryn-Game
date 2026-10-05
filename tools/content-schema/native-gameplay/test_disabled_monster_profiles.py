#!/usr/bin/env python3
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from audit_disabled_monster_profiles import attest_source, closure_facts


class DisabledProfileEvidence(unittest.TestCase):
    def test_frozen_source_bytes_are_required_for_disabled_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = root / 'bundle'
            bundle.mkdir()
            path = bundle / 'monster.json'
            source = root / 'captured' / 'crystal' / 'data-global' / 'monster' / 'stag.lua'
            source.parent.mkdir(parents=True)
            raw = b'local monster = {health = 50}\n'
            source.write_bytes(raw)
            manifest = {'sources': [{'repository': 'zimbadev/crystalserver', 'revision': 'pinned'}],
                        'entries': [{'source_file': 'data-global/monster/stag.lua'}]}
            path.with_name('manifest.json').write_text(json.dumps(manifest))
            attested = {'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw),
                        'git_blob': hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest()}
            index = {('zimbadev/crystalserver', 'pinned', 'data-global/monster/stag.lua'): attested}
            self.assertEqual(attest_source(path, root / 'captured', index), attested)
            source.write_bytes(raw.replace(b'50', b'99'))
            with self.assertRaises(ValueError):
                attest_source(path, root / 'captured', index)
            source.write_bytes(raw)
            index[next(iter(index))] = {**attested, 'git_blob': 'incorrect'}
            with self.assertRaises(ValueError):
                attest_source(path, root / 'captured', index)

    def test_zero_damage_and_secondary_conditions_are_preserved_in_evidence(self):
        ability, effect, formula = ({'key': key, 'revision': 'source'} for key in ['ability', 'effect', 'formula'])
        condition = {'key': 'poison', 'revision': 'source'}
        monster = {'behavior': {'attacks': [{'ability': ability}]}}
        dependencies = {'abilities': [{'identity': ability, 'kind': 'melee', 'range_tiles': 1,
                                       'effects': [effect, condition]}],
                        'effects': [{'identity': effect, 'formula': formula},
                                    {'identity': condition, 'operation': 'condition', 'condition': {'type': 'poison'}}],
                        'formulas': [{'identity': formula, 'kind': 'range', 'magnitude': {'minimum': 0, 'maximum': 0}}]}
        facts = closure_facts(monster, dependencies)
        self.assertEqual(facts[0]['formula']['magnitude']['maximum'], 0)
        self.assertEqual(facts[0]['secondary_effects'][0]['condition']['type'], 'poison')
        self.assertEqual(facts[0]['source_slot'], 1)
        dependencies['abilities'][0]['kind'] = 'spell'
        self.assertEqual(closure_facts(monster, dependencies), [])


if __name__ == '__main__':
    unittest.main()
