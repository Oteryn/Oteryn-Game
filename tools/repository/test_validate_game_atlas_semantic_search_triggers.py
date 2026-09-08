#!/usr/bin/env python3
"""Protect the Atlas workflows' trigger closure and exact-source oracles."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SEMANTIC_WORKFLOW = ROOT / ".github/workflows/game-atlas-semantic-search.yml"
STATIC_WORKFLOW = ROOT / ".github/workflows/game-atlas-static-creatures.yml"

FULLWORLD_PRODUCER = "tools/game-atlas-fullworld-source/producer.py"
THAIS_PRODUCER = "tools/game-atlas-thais-fixture/export.py"
CREATURE_IDENTITY = "tools/game-atlas-creatures/identity.py"
REGRESSION = "tools/repository/test_validate_game_atlas_semantic_search_triggers.py"
SEMANTIC_WORKFLOW_PATH = ".github/workflows/game-atlas-semantic-search.yml"
STATIC_WORKFLOW_PATH = ".github/workflows/game-atlas-static-creatures.yml"
REGRESSION_COMMAND = f"python {REGRESSION}"


def _event_block(workflow: str, event: str) -> str:
    """Return one top-level event's YAML block without interpreting YAML."""
    match = re.search(
        rf"(?ms)^  {re.escape(event)}:(.*?)(?=^  [a-z_]+:|^permissions:)",
        workflow,
    )
    return match.group(1) if match else ""


def _paths(workflow: str, event: str) -> set[str]:
    return set(re.findall(r"(?m)^      - '([^']+)'\s*$", _event_block(workflow, event)))


def _assert_contract(semantic: str, static: str) -> None:
    semantic_pr = _paths(semantic, "pull_request")
    semantic_push = _paths(semantic, "push")
    static_pr = _paths(static, "pull_request")

    assert {FULLWORLD_PRODUCER, THAIS_PRODUCER} <= semantic_pr
    assert {FULLWORLD_PRODUCER, THAIS_PRODUCER} <= semantic_push
    assert {REGRESSION, SEMANTIC_WORKFLOW_PATH, STATIC_WORKFLOW_PATH} <= semantic_pr
    assert {CREATURE_IDENTITY, REGRESSION, STATIC_WORKFLOW_PATH, SEMANTIC_WORKFLOW_PATH} <= static_pr
    assert REGRESSION_COMMAND in semantic
    assert REGRESSION_COMMAND in static


class AtlasTriggerClosureTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.semantic = SEMANTIC_WORKFLOW.read_text(encoding="utf-8")
        cls.static = STATIC_WORKFLOW.read_text(encoding="utf-8")

    def test_dependency_chain_and_trigger_selection(self) -> None:
        fullworld = (ROOT / FULLWORLD_PRODUCER).read_text(encoding="utf-8")
        semantic_export = (ROOT / "tools/game-atlas-semantic-search/export.py").read_text(encoding="utf-8")
        creature_export = (ROOT / "tools/game-atlas-creatures/export.py").read_text(encoding="utf-8")

        self.assertIn('"game-atlas-fullworld-source" / "producer.py"', semantic_export)
        self.assertIn("tools/game-atlas-thais-fixture/export.py", fullworld)
        self.assertRegex(creature_export, r"from identity import .*stable_creature_entity_id")

        semantic_pr = _paths(self.semantic, "pull_request")
        semantic_push = _paths(self.semantic, "push")
        static_pr = _paths(self.static, "pull_request")
        for dependency in (FULLWORLD_PRODUCER, THAIS_PRODUCER):
            self.assertIn(dependency, semantic_pr)
            self.assertIn(dependency, semantic_push)
        self.assertIn(CREATURE_IDENTITY, static_pr)

    def test_shared_regression_and_workflow_changes_select_both(self) -> None:
        semantic_pr = _paths(self.semantic, "pull_request")
        static_pr = _paths(self.static, "pull_request")
        for changed_path in (REGRESSION, SEMANTIC_WORKFLOW_PATH, STATIC_WORKFLOW_PATH):
            self.assertIn(changed_path, semantic_pr)
            self.assertIn(changed_path, static_pr)
        self.assertIn(REGRESSION_COMMAND, self.semantic)
        self.assertIn(REGRESSION_COMMAND, self.static)

    def test_event_models_original_paths_and_exclusions(self) -> None:
        semantic_pr = _paths(self.semantic, "pull_request")
        semantic_push = _paths(self.semantic, "push")
        static_pr = _paths(self.static, "pull_request")
        self.assertTrue({
            "tools/game-atlas-semantic-search/**",
            "docs/contracts/OTERYN_GAME_ATLAS_SEMANTIC_SEARCH_PROFILE_V1.md",
            SEMANTIC_WORKFLOW_PATH,
        } <= semantic_pr)
        self.assertTrue({
            "tools/game-atlas-semantic-search/**",
            "docs/contracts/OTERYN_GAME_ATLAS_SEMANTIC_SEARCH_PROFILE_V1.md",
            SEMANTIC_WORKFLOW_PATH,
        } <= semantic_push)
        self.assertTrue({
            "tools/game-atlas-creatures/export.py",
            "tools/game-atlas-creatures/self_test.py",
            "tools/game-atlas-creatures/README.md",
            "docs/contracts/OTERYN_GAME_ATLAS_NPC_ROLES_V1.md",
            STATIC_WORKFLOW_PATH,
        } <= static_pr)
        self.assertIn("branches: [main]", _event_block(self.semantic, "push"))
        self.assertNotRegex(self.static, r"(?m)^  push:")
        for unrelated in ("README.md", "src/main.rs", "tools/game-atlas-creatures/monster.py"):
            self.assertNotIn(unrelated, semantic_pr | semantic_push | static_pr)

    def test_exact_oracles_and_workflow_safety_remain(self) -> None:
        for workflow in (self.semantic, self.static):
            self.assertIn("permissions:\n  contents: read", workflow)
            self.assertNotIn("continue-on-error", workflow)
            self.assertNotIn("if: false", workflow)
            actions = re.findall(r"(?m)^\s+uses: ([^\s]+)", workflow)
            self.assertTrue(actions)
            self.assertTrue(all(re.search(r"@[0-9a-f]{40}$", action) for action in actions))

        for required in (
            "python tools/game-atlas-semantic-search/self_test.py",
            "sha256:0cc0546aff0e9a8f85716dcbe5babc6043e148e946a448b1d87f083410cb1005",
            "'records': 88684",
            "npc:726487438c8308abf291622a52d91b24",
            "semantic-record:23716a35099a04179f7b9e3e6c9198ee",
            "'capabilities'",
        ):
            self.assertIn(required, self.semantic)
        for required in (
            "python tools/game-atlas-creatures/self_test.py",
            "/tmp/creatures-a.json",
            "/tmp/creatures-b.json",
            "cmp /tmp/creatures-a.json /tmp/creatures-b.json",
            "'npcs':1068",
            "'monster_spawns':87565",
            "'bank':25",
            "sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4",
        ):
            self.assertIn(required, self.static)

    def test_every_new_control_is_mutation_protected(self) -> None:
        _assert_contract(self.semantic, self.static)
        mutations = []
        for path in (FULLWORLD_PRODUCER, THAIS_PRODUCER):
            mutations.append((self.semantic.replace(f"      - '{path}'\n", "", 1), self.static))
            push_pos = self.semantic.index("  push:")
            mutations.append((
                self.semantic[:push_pos]
                + self.semantic[push_pos:].replace(f"      - '{path}'\n", "", 1),
                self.static,
            ))
        for path in (REGRESSION, STATIC_WORKFLOW_PATH):
            mutations.append((self.semantic.replace(f"      - '{path}'\n", "", 1), self.static))
        for path in (CREATURE_IDENTITY, REGRESSION, SEMANTIC_WORKFLOW_PATH):
            mutations.append((self.semantic, self.static.replace(f"      - '{path}'\n", "", 1)))
        mutations.extend((
            (self.semantic.replace(REGRESSION_COMMAND, "python missing-regression.py", 1), self.static),
            (self.semantic, self.static.replace(REGRESSION_COMMAND, "python missing-regression.py", 1)),
        ))
        for semantic, static in mutations:
            with self.assertRaises(AssertionError):
                _assert_contract(semantic, static)


if __name__ == "__main__":
    unittest.main()
