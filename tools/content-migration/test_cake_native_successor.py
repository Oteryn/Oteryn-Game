"""Cake's committed canonical source must reach the actual composed provider."""
import copy
import json
import hashlib
import tempfile
from pathlib import Path
from unittest.mock import patch

import cake_native_successor as successor
import unittest

import compose_monster_current_sources as compose
import import_current_spell_sources as current
import import_spell_families as importer


class CakeNativeSuccessorTests(unittest.TestCase):
    def test_m57_tampered_binding_table_is_rejected_before_cake_admission(self):
        path = importer.ROOT / 'imports/crystalserver/bindings/items.json'
        read_bytes = Path.read_bytes
        baseline = importer.baseline_outputs(importer.ROOT)
        with patch.object(Path, 'read_bytes', lambda selected:
                          read_bytes(selected) + b' ' if selected == path else read_bytes(selected)):
            with self.assertRaisesRegex(ValueError, 'Item binding table digest mismatch'):
                successor.admit(importer.ROOT, baseline)

    def test_generated_project_pin_accepts_new_revision_but_rejects_tampering(self):
        for mutation in [None, 'bytes', 'length', 'hash', 'membership', 'root', 'revision', 'schema', 'bound']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                world = root / 'content/world'
                path = world / 'definitions/declarations.json'
                path.parent.mkdir(parents=True)
                document = {'schema': 'OTERYN_WORLD_PROJECT_DECLARATIONS/v2', 'authoring_profiles': []}
                raw = json.dumps(document).encode()
                member = {'locator': 'definitions/declarations.json', 'schema': document['schema'],
                          'byte_length': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
                manifest = {'schema': 'OTERYN_WORLD_PROJECT_MANIFEST/v2',
                            'package_key': 'oteryn:content.world-project',
                            'package_revision': 'merged-revision', 'documents': [member]}
                if mutation == 'length': member['byte_length'] += 1
                if mutation == 'hash': member['sha256'] = '0' * 64
                if mutation == 'membership': manifest['documents'].append(dict(member))
                if mutation == 'schema': member['schema'] = 'unqualified'
                if mutation == 'bound': member['byte_length'] = 96_000_001
                manifest_raw = json.dumps(manifest).encode()
                project = {'schema': 'OTERYN_WORLD_PROJECT_ROOT/v2', 'project_revision': 'merged-revision',
                           'manifest_locator': 'manifest.json',
                           'manifest_sha256': hashlib.sha256(manifest_raw).hexdigest()}
                if mutation == 'root': project['manifest_sha256'] = '0' * 64
                if mutation == 'revision': project['project_revision'] = 'another-revision'
                path.write_bytes(raw + (b' ' if mutation == 'bytes' else b''))
                (world / 'manifest.json').write_text(manifest_raw.decode())
                (world / 'project.json').write_text(json.dumps(project))
                if mutation is None:
                    self.assertEqual(successor.read_project_document(root, member['locator']), document)
                else:
                    with self.assertRaisesRegex(ValueError, 'CAKE_NATIVE_PROJECT_'):
                        successor.read_project_document(root, member['locator'])

    def test_actual_generated_provider_contains_complete_canonical_cake(self):
        baseline = importer.baseline_outputs(importer.ROOT)
        result = compose.compose(importer.ROOT, baseline, current.outputs(importer.ROOT, baseline))
        creatures = json.loads(result[compose.CREATURES])
        cake, = [row for row in creatures['records']
                 if row['profile']['target']['key'] == 'oteryn:creature.cake_golem']
        self.assertEqual(cake['profile']['data']['profile']['details']['corpse_item']['key'],
                         'oteryn:item.tibia.i12141')
        loot = json.loads(result['tools/content-schema/native-gameplay/loot-tables.json'])
        binding, = [r for r in loot['creature_loot'] if r['creature'] == cake['profile']['target']]
        self.assertEqual(binding['loot']['key'], 'oteryn:loot.creature.cake_golem')
        table, = [r for r in loot['tables'] if r['identity'] == binding['loot']]
        self.assertEqual(table['algorithm'], 'IndependentBernoulliPpm')
        self.assertEqual(table['entries'][0]['item']['key'], 'oteryn:item.tibia.i12143')
        self.assertEqual(table['entries'][0]['probability_ppm'], 500000)
        prior_loot = json.loads((importer.ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        for field, key in [('creature_loot', 'creature'), ('tables', 'identity')]:
            retained = {json.dumps(r[key],sort_keys=True):r for r in loot[field]}
            for row in prior_loot[field]:
                self.assertEqual(retained[json.dumps(row[key],sort_keys=True)], row)
        item_facts = {r['item']['key']:r for r in loot['items']}
        corpse = item_facts['oteryn:item.tibia.i12141']
        self.assertTrue(corpse['materializable'])
        self.assertTrue(1 <= corpse['container_capacity'] <= 30)
        self.assertTrue(item_facts['oteryn:item.tibia.i12143']['materializable'])
        self.assertTrue(item_facts['oteryn:item.tibia.i12143']['stackable'])
        self.assertEqual(len(loot['creature_loot']), len(creatures['records']))
        self.assertEqual({json.dumps(r['creature'],sort_keys=True) for r in loot['creature_loot']},
                         {json.dumps(r['profile']['target'],sort_keys=True) for r in creatures['records']})
        bodies = cake['attack_definitions']['profiles']
        undefined, = [row for row in bodies
                       if row['target']['key'] == 'oteryn:effect.creature.cake_golem.attack-2']
        self.assertEqual(undefined['data']['profile']['damage_type'], 'undefined')

    def test_substituted_empty_cake_loot_is_rejected(self):
        original_read = successor.read_canonical
        def substituted(path):
            document, pin = original_read(path)
            if path.parent == importer.ROOT / 'content/loot':
                for row in document['records']:
                    if row['definition']['identity']['key'] == 'oteryn:loot.creature.cake_golem':
                        row['definition']['entries'] = []
            return document, pin
        with patch.object(successor, 'read_canonical', side_effect=substituted):
            with self.assertRaisesRegex(ValueError, 'CANONICAL_LOOT_MISMATCH'):
                successor.loot_successor(importer.ROOT, {'records': []})

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
        changed = copy.deepcopy(declarations)
        attack = next(row for row in changed['authoring_profiles']
                      if row['target']['key'] == 'oteryn:effect.creature.cake_golem.attack-2')
        attack['data']['profile']['damage_type'] = 'earth'
        with self.assertRaisesRegex(ValueError, 'CANONICAL_SOURCE_PROFILE_MISMATCH'):
            successor.qualify(rows, changed, reference, packet, bindings)
        duplicated = copy.deepcopy(reference)
        duplicated['records'].append(copy.deepcopy(duplicated['records'][0]))
        with self.assertRaisesRegex(ValueError, 'CANONICAL_IDENTITY_DUPLICATE'):
            successor.qualify(rows, declarations, duplicated, packet, bindings)
        # Exercise the exporter's actual HEAD fence, without changing shared input.
        path = root / successor.PACKET
        with patch.object(type(path), 'read_bytes', return_value=b'{}'):
            with self.assertRaisesRegex(ValueError, 'canonical input changed from HEAD'):
                successor.read_canonical(path)


if __name__ == '__main__':
    unittest.main()
