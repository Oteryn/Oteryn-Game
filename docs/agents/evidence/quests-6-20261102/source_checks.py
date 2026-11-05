"""Pinned donor excerpt integrity only; no Lua or Oteryn NPC runtime is executed.

QUESTION: https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589761172
ANSWER section 4: https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589908362

Run: python3 docs/agents/evidence/quests-6-20261102/source_checks.py -v
The checksum covers the captured excerpt, not a newly fetched donor file.
This test neither evaluates getInquisitionGold nor claims a native adapter.
"""

import hashlib
import json
from pathlib import Path
import unittest


CAPTURE = Path(__file__).with_name("sandomo-record-capture.json")
SOURCE = (
    "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:"
    "data-global/npc/sandomo.lua:114-133"
)
EXCERPT_SHA256 = "59a504cb5a60feca1ebfd630a6b3e31401c13a90785dd056ccaf502a2a31896b"


class SandomoPinnedSourceIntegrity(unittest.TestCase):
    def setUp(self):
        self.capture = json.loads(CAPTURE.read_text(encoding="utf-8"))
        self.lines = self.capture["excerpt"].splitlines()

    def test_exact_donor_excerpt_is_preserved(self):
        self.assertEqual(self.capture["source"], SOURCE)
        self.assertEqual(self.capture["evidence_class"], "SOURCE_INTEGRITY_ONLY")
        self.assertEqual(self.capture["excerpt_sha256"], EXCERPT_SHA256)
        self.assertEqual(
            hashlib.sha256(self.capture["excerpt"].encode("utf-8")).hexdigest(),
            EXCERPT_SHA256,
        )
        self.assertEqual(len(self.lines), 20)

    def test_record_preserves_literal_set_and_three_resets(self):
        self.assertEqual(
            self.lines[123 - 114].strip(),
            "elseif npcHandler:getTopic(playerId) == 5 then",
        )
        self.assertEqual(self.lines[124 - 114].strip(), 'if MsgContains(message, "yes") then')
        writes = [
            (number, line.strip())
            for number, line in enumerate(self.lines, 114)
            if line.lstrip().startswith("player:setStorageValue(")
        ]
        self.assertEqual(writes, [
            (129, "player:setStorageValue(Storage.Quest.U10_30.RoshamuulQuest.Roshamuul_Gold_Record, player:getInquisitionGold())"),
            (130, "player:setStorageValue(Storage.Quest.U10_30.RoshamuulQuest.Roshamuul_Mortar_Thrown, 0)"),
            (131, "player:setStorageValue(Storage.Quest.U10_30.RoshamuulQuest.Roshamuul_Killed_Frazzlemaws, 0)"),
            (132, "player:setStorageValue(Storage.Quest.U10_30.RoshamuulQuest.Roshamuul_Killed_Silencers, 0)"),
        ])
        # Preserve the conflicting wording as a possible donor bug, per ANSWER.
        self.assertIn("Do you want me to add this amount to my books?", self.lines[120 - 114])
        self.assertIn("inquisition gold added.", self.lines[126 - 114])


if __name__ == "__main__":
    unittest.main()
