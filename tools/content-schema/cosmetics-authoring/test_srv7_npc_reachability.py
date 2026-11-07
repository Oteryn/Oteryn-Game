"""Placed source identities require an explicit policy/owner classification.

Programme presence verifies source authoring, not enabled adapters or earned prerequisites.
"""

import json
import unittest

import capture_npc_reachability as source


class NpcReachabilityTest(unittest.TestCase):
    def test_every_placed_grant_source_has_explicit_policy_or_existing_owner(self):
        census = json.loads(source.OUTPUT.read_bytes())
        grants = json.loads(
            source.OUTPUT.with_name("srv7-npc-grants.json").read_bytes()
        )
        all_sources = census["reachable_grant_npcs"] + census["unplaced_grant_npcs"]
        actual = source.placed(source.ROOT)
        reachable = {row["npc"] for row in all_sources if row["npc"] in actual}
        self.assertEqual(
            reachable, {row["npc"] for row in census["reachable_grant_npcs"]}
        )
        programme = json.loads((source.ROOT / "content/cosmetics/npc-acquisitions.json").read_bytes())
        programme_sources = {row["npc"] for row in programme["npcs"]
            if any(effect["kind"] in {"grant_look_addon", "grant_mount"}
                for transition in row["transitions"] for effect in transition["effects"])}
        legacy_sources = {row["npc"] for row in grants["sources"]}
        # These are deliberately absent from permanent generic source planning.
        # Rental mounts are character-temporal; Revenant already has a mature
        # Soul War final owner; Walter has no executable active donor grant.
        delegated = {
            "oteryn:npc.appaloosa", "oteryn:npc.palomino",
            "oteryn:npc.flickering_soul", "oteryn:npc.walter_jaeger",
        }
        self.assertFalse(programme_sources & delegated)
        self.assertEqual(reachable, (legacy_sources | programme_sources | delegated) & actual)
        qualified_keys = {
            grant["key"] for npc in programme["npcs"]
            for transition in npc["transitions"] for grant in transition["grants"]
        }
        mappings = json.loads((source.ROOT / "tools/content-schema/cosmetics-authoring/srv7-source-program-grants.json").read_bytes())["records"]
        self.assertEqual(qualified_keys, {row["grant_key"] for row in mappings})
        self.assertNotIn("oteryn:npc.testserver_assistant", actual)
        self.assertEqual(len(all_sources), census["grant_files"])
        self.assertGreater(census["npc_files_checked"], len(all_sources))
        self.assertEqual(
            {row["grant_key"] for row in grants["records"]},
            {row["grant_key"] for row in grants["sources"]},
        )


if __name__ == "__main__":
    unittest.main()
