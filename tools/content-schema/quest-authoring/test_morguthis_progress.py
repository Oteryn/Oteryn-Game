import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
TREASURE = 'oteryn:quest-progress/quest/u7_4/the_ancient_tombs/morguthis_treasure'


class MorguthisProgress(unittest.TestCase):
    def test_room_entry_can_preserve_every_value_allowed_by_the_seven_flame_gate(self):
        room = json.loads((ROOT / 'content/quests/world-callbacks/l6-tomb-rooms.json').read_text())
        entry = next(row for row in room['callbacks'] if row['selector']['unique'] == 40085)
        all_callbacks = json.loads((ROOT / 'content/quests/world-callbacks.json').read_text())
        flames = next(row for row in all_callbacks['callbacks']
                      if row['key'].endswith('/ancient-tombs-morguthis-seven-flame-gate'))
        entry_treasure = next(row for row in entry['storage'] if row['track'] == TREASURE)
        flame_treasure = next(row for row in flames['storage'] if row['track'] == TREASURE)
        self.assertEqual(entry_treasure, {
            'track': TREASURE, 'minimum': flame_treasure['minimum'],
            'maximum': flame_treasure['maximum'], 'set': 1, 'at_least': True,
        })
