"""The NPCFIX producer is independently reproducible before runtime activation."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('npcfix_rewards', HERE / 'npcfix_rewards.py')
PRODUCER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRODUCER)


class NpcfixRewardAuthoring(unittest.TestCase):
    def fixture(self, root):
        schema = root / 'tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json'
        schema.parent.mkdir(parents=True)
        schema.write_bytes((HERE / 'shared-reward-callbacks.schema.json').read_bytes())

    def test_fresh_checkout_materializes_only_the_owned_source_callback(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            outputs = PRODUCER.outputs(root)
            callbacks = json.loads(outputs[root / 'content/quests/shared-reward-callbacks.json'])['callbacks']
            by_npc = {row['npc']: row for row in callbacks}
            self.assertEqual(len(by_npc), 1)
            self.assertEqual(by_npc['oteryn:npc.a_fading_memory']['cooldown']['seconds'], 72_000)
            self.assertEqual(by_npc['oteryn:npc.a_fading_memory']['give']['item'], 'oteryn:item.tibia.i8746')
            self.assertEqual(by_npc['oteryn:npc.a_fading_memory']['storage'][0]['maximum'], 1)

    def test_reproduction_preserves_unrelated_lane_records(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            path = root / 'content/quests/shared-reward-callbacks.json'
            path.parent.mkdir(parents=True)
            other = json.loads(PRODUCER.INPUT.read_text())['reference_callbacks'][0]
            other['key'] = 'oteryn:quest-reward/another-lane'
            other['npc'] = 'oteryn:npc.another_lane'
            path.write_text(json.dumps({'schema': 1, 'callbacks': [other]}))
            first = PRODUCER.outputs(root)
            self.assertEqual(json.loads(first[path])['callbacks'][0], other)
            for target, content in first.items():
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(content)
            self.assertEqual(first, PRODUCER.outputs(root))


if __name__ == '__main__':
    unittest.main()
