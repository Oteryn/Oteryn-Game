"""M35 source groups preserve the exact independently authored once claims."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/reward-claim-authoring'))
import reward_claim_authoring as claims

GROUPS = {
    'circle_room_quest': ['circleroom1', 'circleroom2'],
    'crystal_wand_quest': ['crystalwand1', 'crystalwand2'],
    'demona_ring_quest': ['demonaring1', 'demonaring2'],
    'double_hero_quest': ['doublehero1', 'doublehero2'],
    'griffin_shield_quest': ['griffinshield1', 'griffinshield2', 'griffinshield3'],
}

class AggregateClaimAuthorityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rules = json.loads((ROOT / 'content/quests/claim-sets.json').read_text())
        index = json.loads((ROOT / 'content/quests/definitions/index.json').read_text())
        cls.quests = {row['definition']['identity']['key']: row['definition']
                      for shard in index['shards']
                      for row in json.loads((ROOT / shard).read_text())['records']}

    def test_selected_groups_require_every_exact_source_claim(self):
        for slug, markers in GROUPS.items():
            quest = 'oteryn:quest.' + slug
            rule = self.rules[quest]
            self.assertEqual(rule, {'groups': [['kv/' + m] for m in markers], 'mint_only': True})
            expected = {'oteryn:reward-claim.kv.' + m for m in markers}
            self.assertEqual({c['key'] for c in self.quests[quest]['claims']}, expected)
            self.assertTrue(all(c['revision'] == 'reward-claim-r1' for c in self.quests[quest]['claims']))

    def test_native_obligations_do_not_rewrite_pinned_legacy_claims(self):
        legacy = claims.chest_transitions()
        for markers in GROUPS.values():
            for marker in markers:
                self.assertNotIn('oteryn:quest-transition/kv/' + marker + '/chest', legacy)
        self.assertIn('oteryn:quest-transition/kv/bear_chest1/chest', legacy)
