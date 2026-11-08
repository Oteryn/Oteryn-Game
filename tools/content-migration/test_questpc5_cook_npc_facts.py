"""The Cook overlay preserves unrelated declarations and rejects source drift."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import questpc5_cook_npc_facts as cook

ROOT = Path(__file__).resolve().parents[2]


class CookNpcFacts(unittest.TestCase):
    def test_retained_position_is_idempotent_and_preserves_other_declarations(self):
        original = json.loads((ROOT / 'content/world/definitions/declarations.json').read_bytes())
        result = cook.extend(original, ROOT)
        self.assertEqual(cook.extend(result, ROOT), result)
        for before, after in zip(original['records'], result['records'], strict=True):
            if before.get('identity', {}).get('key') == cook.KEY:
                self.assertEqual(before['fields'], [])
                self.assertEqual(len(after['fields']), 3)
                for key in before.keys() - {'fields'}:
                    self.assertEqual(before[key], after[key])
            else:
                self.assertEqual(before, after)

    def test_changed_position_is_not_silently_admitted(self):
        facts = json.loads((ROOT / cook.FACTS).read_bytes())
        page, = [p for p in facts['pages'] if p.get('pageid') == 61322]
        page['positions'][0][0] += 1
        original = Path.read_bytes
        def read(path):
            return json.dumps(facts).encode() if path == ROOT / cook.FACTS else original(path)
        with patch.object(Path, 'read_bytes', read):
            with self.assertRaisesRegex(ValueError, 'RETAINED_FACT_DRIFT'):
                cook.extend({'records': []}, ROOT)

    def test_existing_conflicting_fact_is_preserved_by_refusal(self):
        declaration = dict(kind='NPC', identity=dict(key=cook.KEY), fields=[dict(
            field_path='oteryn:source.npc.bulk.name', value=dict(type='Text', value='Other'))])
        original = dict(records=[declaration])
        expected = copy.deepcopy(original)
        with self.assertRaisesRegex(ValueError, 'EXISTING_FACT_CONFLICT'):
            cook.extend(original, ROOT)
        self.assertEqual(original, expected)


if __name__ == '__main__':
    unittest.main()
