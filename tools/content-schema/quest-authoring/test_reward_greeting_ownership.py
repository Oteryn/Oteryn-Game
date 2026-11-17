"""Exact greeting owners coexist without weakening the native duplicate guard."""
import copy
import json
from pathlib import Path
import re
import tempfile
import unittest

import qengine_rewards


ROOT = Path(__file__).resolve().parents[3]
SHARED = "content/dialogues/quests/shared-reward-greetings.json"
CALLBACK_DOCUMENTS = (
    "content/quests/shared-reward-callbacks.json",
    "content/quests/npc-callbacks/m39-stories.json",
    "content/quests/npc-callbacks/l2-ready-stories.json",
    "content/quests/npc-callbacks/l13-outfits.json",
    "content/quests/npc-callbacks/questpc7-programmes.json",
)


def registered_documents():
    owner = ROOT / "crates/gameplay/npc/src/npc_talk.rs"
    source = owner.read_text()
    registry = re.search(r"const REWARD_GREETINGS:.*?= &\[(.*?)\];", source, re.S)
    if registry is None:
        raise AssertionError("native reward greeting registry missing")
    paths = re.findall(r'include_str!\("([^\"]+)"\)', registry.group(1))
    if not paths:
        raise AssertionError("native reward greeting registry empty")
    return [(owner.parent / path).resolve() for path in paths]


class RewardGreetingOwnership(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.callbacks = [
            callback
            for name in CALLBACK_DOCUMENTS
            for callback in json.loads((ROOT / name).read_bytes())["callbacks"]
        ]
        cls.inputs = qengine_rewards.donor_inputs()

    def test_actual_native_registry_has_one_row_per_npc(self):
        seen = {}
        for path in registered_documents():
            document = json.loads(path.read_bytes())
            self.assertEqual(document["schema"], 1)
            for row in document["npcs"]:
                self.assertNotIn(row["npc"], seen, (row["npc"], seen.get(row["npc"]), path))
                seen[row["npc"]] = path
        for npc, path in qengine_rewards.REWARD_GREETING_OWNERS.items():
            self.assertEqual(seen[npc], ROOT / path)

    def test_actual_shared_output_uses_canonical_producer_and_stable_order(self):
        before = {
            path: (ROOT / path).read_bytes()
            for path in qengine_rewards.REWARD_GREETING_OWNERS.values()
        }
        rows = qengine_rewards.reward_greetings(self.callbacks, self.inputs)
        self.assertEqual(json.loads((ROOT / SHARED).read_bytes())["npcs"], rows)
        self.assertEqual(rows, qengine_rewards.reward_greetings(list(reversed(self.callbacks)), self.inputs))
        self.assertTrue(set(qengine_rewards.REWARD_GREETING_OWNERS).isdisjoint(row["npc"] for row in rows))
        self.assertEqual(before, {path: (ROOT / path).read_bytes() for path in before})

    def owner_root(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for name in qengine_rewards.REWARD_GREETING_OWNERS.values():
            destination = root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((ROOT / name).read_bytes())
        return root

    def alter(self, root, npc, change):
        path = root / qengine_rewards.REWARD_GREETING_OWNERS[npc]
        document = json.loads(path.read_bytes())
        change(document["npcs"])
        path.write_text(json.dumps(document))

    def reject(self, root):
        with self.assertRaisesRegex(ValueError, "reward greeting owner"):
            qengine_rewards.reward_greetings(self.callbacks, self.inputs, root=root)

    def test_missing_owner_document_or_row_is_refused(self):
        npc = "oteryn:npc.zalamon"
        root = self.owner_root()
        (root / qengine_rewards.REWARD_GREETING_OWNERS[npc]).unlink()
        self.reject(root)
        root = self.owner_root()
        self.alter(root, npc, lambda rows: rows.__setitem__(slice(None), [r for r in rows if r["npc"] != npc]))
        self.reject(root)

    def test_duplicate_owner_row_is_refused(self):
        for npc in qengine_rewards.REWARD_GREETING_OWNERS:
            with self.subTest(npc=npc):
                root = self.owner_root()
                self.alter(root, npc, lambda rows: rows.append(copy.deepcopy(next(r for r in rows if r["npc"] == npc))))
                self.reject(root)

    def test_wrong_npc_cannot_satisfy_the_declared_owner(self):
        root = self.owner_root()
        npc = "oteryn:npc.zalamon"
        self.alter(root, npc, lambda rows: next(r for r in rows if r["npc"] == npc).__setitem__("npc", "oteryn:npc.unrelated"))
        self.reject(root)

    def test_same_row_in_a_different_owner_file_is_refused(self):
        root = self.owner_root()
        npc = "oteryn:npc.zalamon"
        source = root / qengine_rewards.REWARD_GREETING_OWNERS[npc]
        row = next(r for r in json.loads(source.read_bytes())["npcs"] if r["npc"] == npc)
        self.alter(root, npc, lambda rows: rows.__setitem__(slice(None), [r for r in rows if r["npc"] != npc]))
        self.alter(root, "oteryn:npc.henricus", lambda rows: rows.append(row))
        self.reject(root)


if __name__ == "__main__":
    unittest.main()
