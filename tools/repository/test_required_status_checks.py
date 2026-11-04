#!/usr/bin/env python3
"""Both required aggregates must survive policy validation and API readback."""
from contextlib import redirect_stdout, redirect_stderr
from copy import deepcopy
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import validate_repository_policy_core as core

ROOT = Path(__file__).resolve().parents[2]


class RequiredStatusChecksTests(unittest.TestCase):
    def setUp(self):
        self.policy = json.loads(core.POLICY_PATH.read_text())

    def test_policy_requires_both_aggregates_in_both_fields(self):
        self.assertEqual(self.policy["required_status_check"], ["game-gate", "heavy-gate"])
        self.assertEqual(core.required_status_contexts(self.policy["ruleset"]), ["game-gate", "heavy-gate"])
        for field in ("required_status_check", "ruleset"):
            for contexts in (["game-gate", "heavy-gate"], ["game-gate"], ["heavy-gate"],
                             ["game-gate", "heavy-gate", "extra"], "game-gate"):
                with self.subTest(field=field, contexts=contexts):
                    policy = deepcopy(self.policy)
                    if field == "required_status_check":
                        policy[field] = contexts
                    else:
                        rule = next(r for r in policy[field]["rules"] if r["type"] == "required_status_checks")
                        rule["parameters"]["required_status_checks"] = [{"context": c} for c in contexts]
                    with tempfile.TemporaryDirectory(dir=ROOT) as directory:
                        path = Path(directory) / "policy.json"
                        path.write_text(json.dumps(policy))
                        with patch.object(core, "POLICY_PATH", path), patch.object(core.subprocess, "run") as run:
                            run.return_value.returncode = 0
                            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                                result = core.main()
                    self.assertEqual(result, 0 if contexts == ["game-gate", "heavy-gate"] else 1)

    def test_settings_readback_accepts_both_and_rejects_missing_checks(self):
        spec = importlib.util.spec_from_file_location("settings_under_test", ROOT / "tools/repository/apply_github_settings.py")
        settings = importlib.util.module_from_spec(spec)
        with patch.dict(os.environ, {"GITHUB_REPOSITORY": "Oteryn/Oteryn-Game"}):
            spec.loader.exec_module(settings)
        responses = {
            "": self.policy["repository"],
            "/topics": {"names": self.policy["topics"]},
            "/labels?per_page=100": self.policy["labels"],
            "/actions/permissions/workflow": self.policy["actions"],
            "/private-vulnerability-reporting": {"enabled": True},
            "/environments?per_page=100": {"environments": []},
        }

        def read_only_request(method, path, **kwargs):
            self.assertEqual(method, "GET")
            return responses[path]

        for contexts in (["game-gate", "heavy-gate"], ["game-gate"], ["heavy-gate"]):
            with self.subTest(contexts=contexts):
                live_ruleset = deepcopy(self.policy["ruleset"])
                rule = next(r for r in live_ruleset["rules"] if r["type"] == "required_status_checks")
                rule["parameters"]["required_status_checks"] = [{"context": c} for c in contexts]
                with patch.object(settings, "request", side_effect=read_only_request), \
                     patch.object(settings, "fetch_ruleset_by_name", return_value=live_ruleset), \
                     patch.object(settings, "repository_supports_push_ruleset", return_value=False), \
                     patch.object(settings, "list_rulesets", return_value=[]), redirect_stdout(io.StringIO()):
                    if contexts == ["game-gate", "heavy-gate"]:
                        settings.verify()
                    else:
                        with self.assertRaisesRegex(settings.ApiError, "required-status mismatch"):
                            settings.verify()
        with patch.object(settings, "POLICY", {**self.policy, "required_status_check": ["game-gate"]}), \
             patch.object(settings, "request", side_effect=read_only_request), \
             patch.object(settings, "fetch_ruleset_by_name", return_value=self.policy["ruleset"]):
            with self.assertRaisesRegex(settings.ApiError, "disagrees with branch ruleset"):
                settings.verify()


if __name__ == "__main__":
    unittest.main()
