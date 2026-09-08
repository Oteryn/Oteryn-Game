#!/usr/bin/env python3
"""Protect the Atlas workflows' trigger closure and exact-source oracles."""

from __future__ import annotations

import ast
import hashlib
import json
import re
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SEMANTIC_WORKFLOW = ROOT / ".github/workflows/game-atlas-semantic-search.yml"
STATIC_WORKFLOW = ROOT / ".github/workflows/game-atlas-static-creatures.yml"

FULLWORLD_PRODUCER = "tools/game-atlas-fullworld-source/producer.py"
THAIS_PRODUCER = "tools/game-atlas-thais-fixture/export.py"
CREATURE_EXPORT = "tools/game-atlas-creatures/export.py"
CREATURE_IDENTITY = "tools/game-atlas-creatures/identity.py"
REGRESSION = "tools/repository/test_validate_game_atlas_semantic_search_triggers.py"
SEMANTIC_WORKFLOW_PATH = ".github/workflows/game-atlas-semantic-search.yml"
STATIC_WORKFLOW_PATH = ".github/workflows/game-atlas-static-creatures.yml"
REGRESSION_COMMAND = f"python {REGRESSION}"
EXPECTED_SEMANTIC_WORKFLOW_BLOB = "8ff643337108876bd709064014a7a146134d69b2"
EXPECTED_STATIC_WORKFLOW_BLOB = "51758c91ebcad0140739f1c1ea9acba32032a238"

SEMANTIC_PR_PATHS = (
    "tools/game-atlas-semantic-search/**",
    CREATURE_EXPORT,
    CREATURE_IDENTITY,
    FULLWORLD_PRODUCER,
    THAIS_PRODUCER,
    REGRESSION,
    "docs/contracts/OTERYN_GAME_ATLAS_SEMANTIC_SEARCH_PROFILE_V1.md",
    SEMANTIC_WORKFLOW_PATH,
    STATIC_WORKFLOW_PATH,
)
SEMANTIC_PUSH_PATHS = (
    "tools/game-atlas-semantic-search/**",
    CREATURE_EXPORT,
    CREATURE_IDENTITY,
    FULLWORLD_PRODUCER,
    THAIS_PRODUCER,
    "docs/contracts/OTERYN_GAME_ATLAS_SEMANTIC_SEARCH_PROFILE_V1.md",
    SEMANTIC_WORKFLOW_PATH,
)
STATIC_PR_PATHS = (
    CREATURE_EXPORT,
    CREATURE_IDENTITY,
    "tools/game-atlas-creatures/self_test.py",
    "tools/game-atlas-creatures/README.md",
    REGRESSION,
    "docs/contracts/OTERYN_GAME_ATLAS_NPC_ROLES_V1.md",
    STATIC_WORKFLOW_PATH,
    SEMANTIC_WORKFLOW_PATH,
)

SEMANTIC_ASSERTIONS = (
    "data['semantic_digest'] == 'sha256:035c911b11e588a969e8fb642965772eee598c30ad80d5d04f9ceda32461e530'",
    "data['counts'] == {'records': 88684, 'kinds': {'monster': 87565, 'npc': 1068, 'town': 33, 'waypoint': 18}}",
    "len(sam) == 1",
    "sam[0]['id'] == 'npc:726487438c8308abf291622a52d91b24'",
    "sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}",
    "'shop' in sam[0]['capabilities']",
    "sam[0]['provenance']['service_resolution_state'] == 'RESOLVED'",
    "len(thais) == 1",
    "thais[0]['id'] == 'semantic-record:23716a35099a04179f7b9e3e6c9198ee'",
    "thais[0]['position'] == {'x': 32369, 'y': 32241, 'floor': -7}",
    "thais[0]['bounds'] is None",
    "data['input_floor_aliases']['7'] == -7",
)
STATIC_ASSERTIONS = (
    "data['contract_id'] == 'oteryn-game-atlas-export-v1'",
    "data['capability'] == 'static-creatures-v1'",
    "data['semantic_revision'] == 1",
    "data['npc_role_schema_version'] == 1",
    "data['statistics'] == {'npcs': 1068, 'monster_spawns': 87565, 'unresolved': 461, 'ambiguous': 5}",
    "data['semantic_digest'] == 'sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4'",
    "dict(counts) == {'bank': 25, 'travel': 51, 'shop': 313, 'quest': 432, 'blessing': 26, 'trainer': 54}",
    "sum(bool(npc.get('roles')) for npc in data['npcs']) == 705",
    "sum(npc.get('role_resolution_state') == 'AMBIGUOUS' for npc in data['npcs']) == 10",
    "all('roles' not in record and 'role_resolution_state' not in record for record in data['monster_spawns'])",
)


def _git_blob_sha(text: str) -> str:
    data = text.encode("utf-8")
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data).hexdigest()


def _event_block(workflow: str, event: str) -> str:
    """Return one top-level event's YAML block without interpreting candidate YAML."""
    match = re.search(
        rf"(?ms)^  {re.escape(event)}:(.*?)(?=^  [a-z_]+:|^permissions:)",
        workflow,
    )
    assert match is not None, f"missing event {event}"
    return match.group(1)


def _event_keys(workflow: str, event: str) -> tuple[str, ...]:
    return tuple(re.findall(r"(?m)^    ([a-z_-]+):", _event_block(workflow, event)))


def _decode_yaml_scalar(raw: str) -> str:
    raw = raw.strip()
    assert raw, "empty YAML scalar"
    if raw.startswith("'"):
        assert raw.endswith("'") and len(raw) >= 2, f"malformed single-quoted scalar: {raw!r}"
        return raw[1:-1].replace("''", "'")
    if raw.startswith('"'):
        assert raw.endswith('"') and len(raw) >= 2, f"malformed double-quoted scalar: {raw!r}"
        value = json.loads(raw)
        assert isinstance(value, str)
        return value
    assert re.fullmatch(r"[A-Za-z0-9_./*?+-]+", raw), f"unsupported YAML scalar: {raw!r}"
    return raw


def _paths(workflow: str, event: str) -> tuple[str, ...]:
    """Parse the complete paths sequence, including entries after comments/blank lines."""
    lines = _event_block(workflow, event).splitlines()
    try:
        start = lines.index("    paths:") + 1
    except ValueError as exc:
        raise AssertionError(f"missing canonical paths list for {event}") from exc

    entries: list[str] = []
    for line in lines[start:]:
        if re.match(r"^    [A-Za-z_][A-Za-z0-9_-]*:\s*", line):
            break
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        item = re.fullmatch(r"      - (.+)", line)
        assert item is not None, f"unexpected content in {event} paths sequence: {line!r}"
        entries.append(_decode_yaml_scalar(item.group(1)))
    assert entries, f"empty paths list for {event}"
    return tuple(entries)


def _decoded_indented_mapping_keys(workflow: str) -> tuple[str, ...]:
    """Decode ordinary/single/double-quoted indented mapping keys fail-closed."""
    keys: list[str] = []
    key_pattern = re.compile(
        r"^[ \t]+(?P<key>'(?:[^']|'')*'|\"(?:\\.|[^\"])*\"|[A-Za-z_][A-Za-z0-9_-]*)\s*:"
    )
    explicit_pattern = re.compile(
        r"^[ \t]+\?\s*(?P<key>'(?:[^']|'')*'|\"(?:\\.|[^\"])*\"|[A-Za-z_][A-Za-z0-9_-]*)\s*$"
    )
    for line in workflow.splitlines():
        match = key_pattern.match(line) or explicit_pattern.match(line)
        if match is not None:
            keys.append(_decode_yaml_scalar(match.group("key")))
    return tuple(keys)


def _step_block(workflow: str, name: str) -> str:
    match = re.search(
        rf"(?ms)^      - name: {re.escape(name)}\n(?P<body>.*?)(?=^      - name:|\Z)",
        workflow,
    )
    assert match is not None, f"missing workflow step {name!r}"
    return match.group("body")


def _python_heredoc(workflow: str, step_name: str) -> str:
    block = _step_block(workflow, step_name)
    match = re.search(
        r"(?ms)^          python - <<'PY'\n(?P<code>.*?)(?=^          PY\s*$)",
        block,
    )
    assert match is not None, f"missing active Python heredoc in {step_name!r}"
    return textwrap.dedent(match.group("code"))


def _assert_executable_assertions(code: str, expected: tuple[str, ...]) -> None:
    tree = ast.parse(code)
    actual = {
        ast.dump(node.test, include_attributes=False)
        for node in ast.walk(tree)
        if isinstance(node, ast.Assert)
    }
    for expression in expected:
        wanted = ast.dump(ast.parse(expression, mode="eval").body, include_attributes=False)
        assert wanted in actual, f"missing executable assertion: {expression}"


def _assert_active_run_command(workflow: str, command: str) -> None:
    assert re.search(rf"(?m)^        run: {re.escape(command)}\s*$", workflow), (
        f"missing active run command: {command}"
    )


def _assert_read_only_permissions(workflow: str) -> None:
    top = re.search(r"(?ms)^permissions:\n(?P<body>(?:^  [^\n]+\n)+)", workflow)
    assert top is not None, "missing top-level permissions"
    assert top.group("body") == "  contents: read\n", "top-level permissions must remain contents: read"
    assert len(re.findall(r"(?m)^permissions:", workflow)) == 1
    assert "permissions" not in _decoded_indented_mapping_keys(workflow), (
        "job/step-level permissions overrides are forbidden"
    )
    assert "write-all" not in workflow


def _assert_pinned_actions_and_no_bypass(workflow: str) -> None:
    actions = re.findall(r"(?m)^\s+uses: ([^\s#]+)", workflow)
    assert actions
    assert all(re.search(r"@[0-9a-f]{40}$", action) for action in actions)
    assert "continue-on-error" not in workflow
    assert "if: false" not in workflow


def _assert_contract(semantic: str, static: str) -> None:
    assert _git_blob_sha(semantic) == EXPECTED_SEMANTIC_WORKFLOW_BLOB
    assert _git_blob_sha(static) == EXPECTED_STATIC_WORKFLOW_BLOB

    assert _event_keys(semantic, "pull_request") == ("paths",)
    assert _event_keys(semantic, "push") == ("branches", "paths")
    assert _event_keys(static, "pull_request") == ("branches", "paths")
    assert _paths(semantic, "pull_request") == SEMANTIC_PR_PATHS
    assert _paths(semantic, "push") == SEMANTIC_PUSH_PATHS
    assert _paths(static, "pull_request") == STATIC_PR_PATHS
    assert "branches: [main]" in _event_block(semantic, "push")
    assert not re.search(r"(?m)^  push:", static)

    _assert_active_run_command(semantic, REGRESSION_COMMAND)
    _assert_active_run_command(static, REGRESSION_COMMAND)
    _assert_active_run_command(semantic, "python tools/game-atlas-semantic-search/self_test.py")

    _assert_read_only_permissions(semantic)
    _assert_read_only_permissions(static)
    _assert_pinned_actions_and_no_bypass(semantic)
    _assert_pinned_actions_and_no_bypass(static)

    _assert_executable_assertions(
        _python_heredoc(semantic, "Qualify Sam and Thais on real pinned data"),
        SEMANTIC_ASSERTIONS,
    )
    _assert_executable_assertions(
        _python_heredoc(static, "Verify exact role and creature census"),
        STATIC_ASSERTIONS,
    )
    static_double_export = _step_block(static, "Build exact pinned product twice")
    assert re.search(
        r"(?m)^          cmp /tmp/creatures-a\.json /tmp/creatures-b\.json\s*$",
        static_double_export,
    ), "static deterministic double-export comparison must remain executable"


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
        for dependency in (CREATURE_EXPORT, CREATURE_IDENTITY, FULLWORLD_PRODUCER, THAIS_PRODUCER):
            self.assertIn(dependency, semantic_pr)
            self.assertIn(dependency, semantic_push)
        self.assertIn(CREATURE_IDENTITY, static_pr)

    def test_shared_regression_and_workflow_changes_select_both(self) -> None:
        semantic_pr = _paths(self.semantic, "pull_request")
        static_pr = _paths(self.static, "pull_request")
        for changed_path in (REGRESSION, SEMANTIC_WORKFLOW_PATH, STATIC_WORKFLOW_PATH):
            self.assertIn(changed_path, semantic_pr)
            self.assertIn(changed_path, static_pr)
        _assert_active_run_command(self.semantic, REGRESSION_COMMAND)
        _assert_active_run_command(self.static, REGRESSION_COMMAND)

    def test_event_models_original_paths_and_exclusions(self) -> None:
        self.assertEqual(_paths(self.semantic, "pull_request"), SEMANTIC_PR_PATHS)
        self.assertEqual(_paths(self.semantic, "push"), SEMANTIC_PUSH_PATHS)
        self.assertEqual(_paths(self.static, "pull_request"), STATIC_PR_PATHS)
        self.assertEqual(_event_keys(self.semantic, "pull_request"), ("paths",))
        self.assertEqual(_event_keys(self.semantic, "push"), ("branches", "paths"))
        self.assertEqual(_event_keys(self.static, "pull_request"), ("branches", "paths"))
        self.assertIn("branches: [main]", _event_block(self.semantic, "push"))
        self.assertNotRegex(self.static, r"(?m)^  push:")

    def test_exact_oracles_and_workflow_safety_remain(self) -> None:
        _assert_contract(self.semantic, self.static)

    def test_every_new_control_is_mutation_protected(self) -> None:
        _assert_contract(self.semantic, self.static)
        mutations: list[tuple[str, str]] = []

        for path in (CREATURE_EXPORT, CREATURE_IDENTITY, FULLWORLD_PRODUCER, THAIS_PRODUCER):
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
            (
                self.semantic.replace(
                    "      - '.github/workflows/game-atlas-static-creatures.yml'\n",
                    "      - '.github/workflows/game-atlas-static-creatures.yml'\n"
                    "      # valid YAML comment inside the same paths sequence\n"
                    '      - "tools/game-atlas-*/**"\n',
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "    runs-on: ubuntu-24.04\n",
                    "    runs-on: ubuntu-24.04\n"
                    '    "permi\\u0073sions":\n'
                    "      contents: write\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          assert sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}, sam[0]\n",
                    "          # assert sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}, sam[0]\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "          assert data['semantic_digest']=='sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4'\n",
                    "          # assert data['semantic_digest']=='sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4'\n",
                    1,
                ),
            ),
        ))

        for semantic, static in mutations:
            with self.assertRaises(AssertionError):
                _assert_contract(semantic, static)


if __name__ == "__main__":
    unittest.main()
