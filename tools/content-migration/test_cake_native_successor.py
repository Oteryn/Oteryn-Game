"""Cake's committed canonical source must reach the actual composed provider."""
import copy
import json
from unittest.mock import patch

import cake_native_successor as successor
import unittest

import compose_monster_current_sources as compose
import import_current_spell_sources as current
import import_spell_families as importer


class CakeNativeSuccessorTests(unittest.TestCase):
    def test_actual_generated_provider_contains_complete_canonical_cake(self):
        baseline = importer.baseline_outputs(importer.ROOT)
        result = compose.compose(importer.ROOT, baseline, current.outputs(importer.ROOT, baseline))
        creatures = json.loads(result[compose.CREATURES])
        cake, = [row for row in creatures['records']
                 if row['profile']['target']['key'] == 'oteryn:creature.cake_golem']
        self.assertEqual(cake['profile']['data']['profile']['details']['corpse_item']['key'],
                         'oteryn:item.tibia.i12141')
        bodies = cake['attack_definitions']['profiles']
        undefined, = [row for row in bodies
                       if row['target']['key'] == 'oteryn:effect.creature.cake_golem.attack-2']
        self.assertEqual(undefined['data']['profile']['damage_type'], 'undefined')

    def test_immutable1870_profiles_are_exact_and_forged_cake_alias_refuses(self):
        baseline = importer.baseline_outputs(importer.ROOT)
        adopted = compose.apply_overlay(importer.ROOT, baseline)
        before = copy.deepcopy(adopted)
        result = successor.admit(importer.ROOT, adopted)
        original = json.loads(adopted[compose.CREATURES])['records']
        combined = json.loads(result[compose.CREATURES])['records']
        self.assertEqual(len(original), 1870)
        self.assertEqual(combined[:1870], original)
        self.assertEqual(adopted, before)
        self.assertEqual([r['profile']['target']['key'] for r in combined[1870:]],
                         ['oteryn:creature.cake_golem'])
        cake = next(r for p in (importer.ROOT / 'content/creatures/definitions').glob('creatures-*.json')
                    for r in json.loads(p.read_bytes())['records']
                    if r['definition']['identity']['key'] == 'oteryn:creature.cake_golem')
        forged = copy.deepcopy(cake)
        forged['authoring']['profile']['details']['display_name'] = 'Rat'
        # Even if a separately validated qualifier returned a colliding new row,
        # union's normal alias refusal cannot silently turn it into admission.
        with patch.object(successor, 'qualify', return_value=forged):
            with self.assertRaisesRegex(ValueError, 'IDENTITY_OR_DISPLAY_NAME_CONFLICT'):
                successor.admit(importer.ROOT, adopted)

    def test_partial_substituted_source_and_non_head_canonical_are_rejected(self):
        root = importer.ROOT
        packet = json.loads((root / successor.PACKET).read_bytes())
        declarations = json.loads((root / 'content/world/definitions/declarations.json').read_bytes())
        reference = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
        bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
        rows = [r for p in (root / 'content/creatures/definitions').glob('creatures-*.json')
                for r in json.loads(p.read_bytes())['records']]
        for operator in ('missing_attack', 'substituted_damage', 'substituted_source'):
            changed = copy.deepcopy(packet)
            if operator == 'missing_attack':
                changed['staged']['authoring_profiles'] = [r for r in changed['staged']['authoring_profiles']
                    if r['target']['key'] != 'oteryn:effect.creature.cake_golem.attack-2']
            elif operator == 'substituted_damage':
                row = next(r for r in changed['staged']['authoring_profiles']
                    if r['target']['key'] == 'oteryn:effect.creature.cake_golem.attack-2')
                row['data']['profile']['damage_type'] = 'earth'
            else:
                changed['sources'][0]['sha256'] = '0' * 64
            with self.subTest(operator=operator), self.assertRaisesRegex(ValueError, 'PARTIAL_OR_SUBSTITUTED'):
                successor.qualify(rows, declarations, reference, changed, bindings)
        # Exercise the exporter's actual HEAD fence, without changing shared input.
        path = root / successor.PACKET
        with patch.object(type(path), 'read_bytes', return_value=b'{}'):
            with self.assertRaisesRegex(ValueError, 'canonical input changed from HEAD'):
                successor.read_canonical(path)


if __name__ == '__main__':
    unittest.main()
