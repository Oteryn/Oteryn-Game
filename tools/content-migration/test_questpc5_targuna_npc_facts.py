"""Targuna NPC facts are exact, idempotent and preserve existing source rows."""
import copy
import json
import unittest
from pathlib import Path
import questpc5_targuna_npc_facts as author

ROOT = Path(__file__).resolve().parents[2]


class TargunaNpcFacts(unittest.TestCase):
    def test_exact_names_and_all_source_positions_without_mutating_legacy(self):
        original = json.loads((ROOT / 'content/world/definitions/declarations.json').read_bytes())
        before = copy.deepcopy(original)
        result = author.extend(original, ROOT)
        self.assertEqual(original, before)
        self.assertEqual(author.extend(result, ROOT), result)
        selected = {'oteryn:npc.' + slug for slug in author.SPECS}
        self.assertEqual([r for r in result['records'] if r.get('identity', {}).get('key') not in selected],
                         [r for r in original['records'] if r.get('identity', {}).get('key') not in selected])
        for slug, (_, _, _, name, positions) in author.SPECS.items():
            row, = [r for r in result['records'] if r.get('kind') == 'NPC' and r['identity']['key'] == 'oteryn:npc.' + slug]
            fields = {f['field_path']: f['value']['value'] for f in row['fields']}
            self.assertEqual(fields['oteryn:source.npc.bulk.name'], name)
            self.assertEqual(json.loads(fields['oteryn:source.npc.bulk.wiki_positions']), positions)

    def test_conflicting_established_name_is_refused(self):
        declarations = json.loads((ROOT / 'content/world/definitions/declarations.json').read_bytes())
        row, = [r for r in declarations['records'] if r.get('kind') == 'NPC' and r['identity']['key'] == 'oteryn:npc.sterling']
        row['fields'].append(dict(field_path='oteryn:source.npc.bulk.name', value=dict(type='Text', value='Wrong pirate')))
        with self.assertRaisesRegex(ValueError, 'TARGUNA_NPC_EXISTING_FACT_CONFLICT'):
            author.extend(declarations, ROOT)


if __name__ == '__main__':
    unittest.main()
