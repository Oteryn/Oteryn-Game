import json
import tempfile
import unittest
from pathlib import Path

from qengine_shared_documents import compose


class SharedDocuments(unittest.TestCase):
    def test_source_rows_refresh_without_erasing_foreign_callbacks_or_greetings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            callback = 'content/quests/shared-reward-callbacks.json'
            greeting = 'content/dialogues/quests/shared-reward-greetings.json'
            documents = {
                callback: {'schema': 1, 'callbacks': [
                    {'key': 'foreign', 'npc': 'shared', 'reply': 'foreign'},
                    {'key': 'owned', 'npc': 'owned', 'reply': 'old'}], 'tracks': []},
                greeting: {'schema': 1, 'npcs': [{'npc': 'shared', 'greet': 'foreign'}]},
                'content/quests/shared-reward-offers.json': {'schema': 1, 'offers': []},
                'content/quests/shared-outfit-callbacks.json': {'schema': 1, 'callbacks': []},
            }
            for path, document in documents.items():
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(json.dumps(document))
            generated = {path: json.dumps(document).encode() for path, document in documents.items()}
            generated[callback] = json.dumps({'schema': 1, 'tracks': [], 'callbacks': [
                {'key': 'owned', 'npc': 'owned', 'reply': 'fresh'}]}).encode()
            generated[greeting] = json.dumps({'schema': 1, 'npcs': [
                {'npc': 'shared', 'greet': 'default'}]}).encode()
            compose(root, generated)
            rows = json.loads(generated[callback])['callbacks']
            self.assertEqual(rows, [documents[callback]['callbacks'][0],
                                   {'key': 'owned', 'npc': 'owned', 'reply': 'fresh'}])
            self.assertEqual(json.loads(generated[greeting]), documents[greeting])


if __name__ == '__main__':
    unittest.main()
