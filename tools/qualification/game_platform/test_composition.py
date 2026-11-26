"""Deployment documents must preserve purpose separation and discovered scope."""
from pathlib import Path
import json
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

from composition import bootstrap, node_documents


class CompositionTests(unittest.TestCase):
    def test_bootstrap_binds_default_and_explicit_sex(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            (work / "portal" / "secrets").mkdir(parents=True)
            (work / "portal" / "secrets" / "env.json").write_text("{}")
            (work / "identity.json").write_text('{"portal_binary": "portal"}')
            binding = {"operationId": "operation", "accountId": "account", "targetWorldId": "world"}
            response = subprocess.CompletedProcess([], 0, stdout=json.dumps(binding))
            revisions = ["profile", "ruleset", "content", "starter"]
            with patch("composition.subprocess.run", return_value=response) as run:
                bootstrap(work, "account", "world", "operation", revisions)
                request = json.loads(run.call_args.args[0][3])
                self.assertEqual(request["requestedSex"], "Female")
                bootstrap(work, "account", "world", "operation", revisions, "Witness", "Knight", "Male")
                request = json.loads(run.call_args.args[0][3])
                self.assertEqual(request, {
                    **binding, "profileRevision": "profile", "rulesetRevision": "ruleset",
                    "contentRevision": "content", "starterTemplateRevision": "starter",
                    "requestedName": "Witness", "requestedVocation": "Knight", "requestedSex": "Male",
                })

    def test_game_configs_use_discovered_san_scope_and_route(self):
        world = "01934f10-7c02-7001-805b-3b1122334401"
        channel = "01934f10-7c03-7001-805b-3b1122334401"
        san = "runtime-status.game.e2e.oteryn.test"
        report = {"clients": {"runtime-status": {"san": san}},
                  "baseUrls": {"internal": "https://127.0.0.1:18543"},
                  "seed": {"route": {"worldId": world, "channelId": channel,
                                     "routeRevision": "rt.1.0123456789abcdef0123456789abcdef"}}}
        source = '[scope]\nassignment_wait_ms = 120000\n[readiness]\nroute_revision = "route-s3b-1"\n[platform]\npeer_name = "source.test"\nsource_authority = "platform"\n'
        node_text, ops_text = node_documents(report, source, Path("/srv/test-game-platform"))
        node, ops = tomllib.loads(node_text), tomllib.loads(ops_text)
        self.assertEqual(node["readiness"]["route_revision"], report["seed"]["route"]["routeRevision"])
        self.assertEqual(node["platform"]["peer_name"], "localhost")
        self.assertEqual(node["platform"]["source_authority"], "oteryn-platform-e2e")
        self.assertEqual(node["scope"]["assignment_wait_ms"], 600000)
        self.assertEqual(ops["scope"], [{"world_id": world, "channel_id": channel, "node_identities": [san]}])
        self.assertEqual(set(ops["node_certificate_files"]), {san})
        self.assertEqual(ops["assignment_epoch"], node["platform"]["runtime_status"]["assignment_epoch"])
        paths = {node["platform"][purpose]["client_certificate_file"]
                 for purpose in ["runtime_status", "bootstrap_intents", "account_characters"]}
        self.assertEqual(len(paths), 3)
        self.assertNotIn(ops["client_certificate_file"], paths)
        self.assertEqual(node["platform"]["account_characters"]["source_authority"], "oteryn-game-e2e")
        self.assertEqual(len(ops["other_producer_certificate_files"]), 2)


if __name__ == "__main__":
    unittest.main()
