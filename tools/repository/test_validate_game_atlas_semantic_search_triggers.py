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


def _quoted_scalar_end(text: str) -> int | None:
    """Return the closing quote index for a leading YAML quoted scalar in linear time."""
    if not text or text[0] not in ("'", '"'):
        return None
    quote = text[0]
    index = 1
    while index < len(text):
        char = text[index]
        if quote == "'":
            if char == "'":
                if index + 1 < len(text) and text[index + 1] == "'":
                    index += 2
                    continue
                return index
            index += 1
            continue
        if char == "\\":
            index += 2
            continue
        if char == '"':
            return index
        index += 1
    return None


def _mapping_key_from_indented_line(line: str) -> str | None:
    """Decode a simple indented YAML mapping key without backtracking regexes."""
    if not line or line[0] not in " \t":
        return None
    text = line.lstrip(" \t")
    if not text or text.startswith("#") or text.startswith("-"):
        return None

    explicit = False
    if text.startswith("?"):
        explicit = True
        text = text[1:].lstrip()
        if not text or text.startswith("#"):
            return None

    if text.startswith(("'", '"')):
        end = _quoted_scalar_end(text)
        if end is None:
            return None
        raw_key = text[: end + 1]
        remainder = text[end + 1 :].lstrip()
        if not explicit and not remainder.startswith(":"):
            return None
        if explicit and remainder and not remainder.startswith("#"):
            return None
        return _decode_yaml_scalar(raw_key)

    if explicit:
        raw_key = text.split("#", 1)[0].rstrip()
        try:
            return _decode_yaml_scalar(raw_key)
        except (AssertionError, json.JSONDecodeError):
            return None

    colon = text.find(":")
    if colon <= 0:
        return None
    raw_key = text[:colon].rstrip()
    try:
        return _decode_yaml_scalar(raw_key)
    except (AssertionError, json.JSONDecodeError):
        return None


def _decoded_indented_mapping_keys(workflow: str) -> tuple[str, ...]:
    keys: list[str] = []
    for line in workflow.splitlines():
        key = _mapping_key_from_indented_line(line)
        if key is not None:
            keys.append(key)
    return tuple(keys)


def _mapping_entry_from_indented_line(line: str) -> tuple[str, str] | None:
    """Decode a simple indented YAML mapping entry, including a step-list prefix."""
    if not line or line[0] not in " \t":
        return None
    text = line.lstrip(" \t")
    if not text or text.startswith("#"):
        return None
    if text.startswith("- "):
        text = text[2:].lstrip()
    elif text.startswith("-"):
        return None
    if not text or text.startswith(("#", "?")):
        return None

    if text.startswith(("'", '"')):
        end = _quoted_scalar_end(text)
        if end is None:
            return None
        raw_key = text[: end + 1]
        remainder = text[end + 1 :].lstrip()
        if not remainder.startswith(":"):
            return None
        raw_value = remainder[1:].strip()
    else:
        colon = text.find(":")
        if colon <= 0:
            return None
        raw_key = text[:colon].rstrip()
        raw_value = text[colon + 1 :].strip()

    try:
        key = _decode_yaml_scalar(raw_key)
    except (AssertionError, json.JSONDecodeError):
        return None
    return key, raw_value


def _strip_yaml_inline_comment(raw: str) -> str:
    """Strip a YAML inline comment without treating hashes inside quotes as comments."""
    quote: str | None = None
    index = 0
    while index < len(raw):
        char = raw[index]
        if quote == "'":
            if char == "'":
                if index + 1 < len(raw) and raw[index + 1] == "'":
                    index += 2
                    continue
                quote = None
            index += 1
            continue
        if quote == '"':
            if char == "\\":
                index += 2
                continue
            if char == '"':
                quote = None
            index += 1
            continue
        if char in ("'", '"'):
            quote = char
            index += 1
            continue
        if char == "#" and (index == 0 or raw[index - 1].isspace()):
            return raw[:index].rstrip()
        index += 1
    return raw.rstrip()


def _strip_github_expressions(raw: str) -> str:
    """Remove GitHub expression delimiters before looking for YAML flow mappings."""
    output: list[str] = []
    index = 0
    while index < len(raw):
        if raw.startswith("${{", index):
            end = raw.find("}}", index + 3)
            assert end >= 0, "unterminated GitHub expression"
            output.append("GITHUB_EXPRESSION")
            index = end + 2
            continue
        output.append(raw[index])
        index += 1
    return "".join(output)


def _has_unquoted_flow_mapping(text: str) -> bool:
    quote: str | None = None
    index = 0
    while index < len(text):
        char = text[index]
        if quote == "'":
            if char == "'":
                if index + 1 < len(text) and text[index + 1] == "'":
                    index += 2
                    continue
                quote = None
            index += 1
            continue
        if quote == '"':
            if char == "\\":
                index += 2
                continue
            if char == '"':
                quote = None
            index += 1
            continue
        if char in ("'", '"'):
            quote = char
            index += 1
            continue
        if char in "{}":
            return True
        index += 1
    return False


def _assert_no_flow_style_mappings(workflow: str) -> None:
    """Fail closed on YAML flow mappings while ignoring literal block-scalar bodies."""
    block_scalar_indent: int | None = None
    for line in workflow.splitlines():
        stripped = line.strip()
        indent = len(line) - len(line.lstrip(" "))
        if block_scalar_indent is not None:
            if not stripped:
                continue
            if indent > block_scalar_indent:
                continue
            block_scalar_indent = None

        structural = _strip_yaml_inline_comment(_strip_github_expressions(line))
        if re.search(r":\s*[|>][+-]?\d?\s*$", structural):
            block_scalar_indent = indent
        assert not _has_unquoted_flow_mapping(structural), (
            f"YAML flow-style mappings are forbidden in protected workflows: {line!r}"
        )


def _decoded_mapping_values(workflow: str, wanted_key: str) -> tuple[str, ...]:
    values: list[str] = []
    for line in workflow.splitlines():
        simple_key = _mapping_key_from_indented_line(line)
        entry = _mapping_entry_from_indented_line(line)
        if simple_key == wanted_key and entry is None:
            raise AssertionError(f"unsupported YAML mapping syntax for {wanted_key!r}")
        if entry is not None and entry[0] == wanted_key:
            values.append(_strip_yaml_inline_comment(entry[1]))
    return tuple(values)


def _decode_mapping_scalar(raw: str) -> str:
    raw = _strip_yaml_inline_comment(raw).strip()
    assert raw, "empty YAML mapping value"
    if raw.startswith(("'", '"')):
        return _decode_yaml_scalar(raw)
    assert not any(char.isspace() for char in raw), f"unsupported YAML mapping value: {raw!r}"
    return raw


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


def _call_name(node: ast.expr) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        prefix = _call_name(node.value)
        return f"{prefix}.{node.attr}" if prefix else node.attr
    return None


def _assert_executable_assertions(code: str, expected: tuple[str, ...]) -> None:
    tree = ast.parse(code)
    allowed_top_level = (ast.Import, ast.ImportFrom, ast.Assign, ast.Assert, ast.Expr)
    assert all(isinstance(statement, allowed_top_level) for statement in tree.body), (
        "oracle heredoc must remain straight-line top-level code"
    )

    terminating_calls = {
        "exit",
        "quit",
        "sys.exit",
        "os._exit",
        "os.abort",
        "builtins.exit",
        "builtins.quit",
    }
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            name = _call_name(node.func)
            assert name not in terminating_calls, f"terminating oracle call is forbidden: {name}"

    expression_statements = [
        (index, statement)
        for index, statement in enumerate(tree.body)
        if isinstance(statement, ast.Expr)
    ]
    assert len(expression_statements) <= 1, "oracle heredoc may only have one trailing expression"
    if expression_statements:
        index, statement = expression_statements[0]
        assert index == len(tree.body) - 1, "oracle expression must be the final statement"
        assert isinstance(statement.value, ast.Call) and _call_name(statement.value.func) == "print", (
            "only a final diagnostic print call is allowed in oracle heredocs"
        )

    actual = tuple(
        ast.dump(statement.test, include_attributes=False)
        for statement in tree.body
        if isinstance(statement, ast.Assert)
    )
    wanted = tuple(
        ast.dump(ast.parse(expression, mode="eval").body, include_attributes=False)
        for expression in expected
    )
    assert actual == wanted, "oracle assertions must remain complete, ordered, and directly executable"


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
    actions = tuple(_decode_mapping_scalar(raw) for raw in _decoded_mapping_values(workflow, "uses"))
    assert actions
    assert all(re.fullmatch(r"[^@\s#]+@[0-9a-f]{40}", action) for action in actions), (
        "every uses action must be pinned to a 40-character commit SHA"
    )
    keys = _decoded_indented_mapping_keys(workflow)
    assert "continue-on-error" not in keys
    assert "continue-on-error" not in workflow
    assert not _decoded_mapping_values(workflow, "if"), (
        "workflow if conditions are forbidden in protected Atlas qualification workflows"
    )


def _assert_contract(semantic: str, static: str) -> None:
    assert _event_keys(semantic, "pull_request") == ("paths",)
    assert _event_keys(semantic, "push") == ("branches", "paths")
    assert _event_keys(static, "pull_request") == ("branches", "paths")
    assert _paths(semantic, "pull_request") == SEMANTIC_PR_PATHS
    assert _paths(semantic, "push") == SEMANTIC_PUSH_PATHS
    assert _paths(static, "pull_request") == STATIC_PR_PATHS
    assert "branches: [main]" in _event_block(semantic, "push")
    assert not re.search(r"(?m)^  push:", static)

    _assert_no_flow_style_mappings(semantic)
    _assert_no_flow_style_mappings(static)
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

    assert _git_blob_sha(semantic) == EXPECTED_SEMANTIC_WORKFLOW_BLOB
    assert _git_blob_sha(static) == EXPECTED_STATIC_WORKFLOW_BLOB


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

    def test_lexical_guards_cover_review_bypasses(self) -> None:
        mutated_paths = self.semantic.replace(
            "      - '.github/workflows/game-atlas-static-creatures.yml'\n",
            "      - '.github/workflows/game-atlas-static-creatures.yml'\n"
            "      # valid YAML comment inside the same paths sequence\n"
            '      - "tools/game-atlas-*/**"\n',
            1,
        )
        self.assertIn("tools/game-atlas-*/**", _paths(mutated_paths, "pull_request"))

        escaped_permissions = (
            "jobs:\n"
            "  build:\n"
            '    "permi\\u0073sions":\n'
            "      contents: write\n"
        )
        self.assertIn("permissions", _decoded_indented_mapping_keys(escaped_permissions))
        self.assertIn("permissions", _decoded_indented_mapping_keys("jobs:\n  build:\n    'permissions':\n      contents: write\n"))

        quoted_uses = (
            "jobs:\n"
            "  verify:\n"
            "    steps:\n"
            "      - name: checkout\n"
            "        'uses': actions/checkout@main\n"
        )
        self.assertEqual(_decoded_mapping_values(quoted_uses, "uses"), ("actions/checkout@main",))
        with self.assertRaises(AssertionError):
            _assert_pinned_actions_and_no_bypass(quoted_uses)

        flow_uses = (
            "jobs:\n"
            "  verify:\n"
            "    steps:\n"
            '      - {name: bad, "uses": actions/setup-python@main}\n'
        )
        with self.assertRaises(AssertionError):
            _assert_no_flow_style_mappings(flow_uses)

        flow_permissions = (
            "jobs:\n"
            "  verify: {runs-on: ubuntu-24.04, permissions: {contents: write}, steps: []}\n"
        )
        with self.assertRaises(AssertionError):
            _assert_no_flow_style_mappings(flow_permissions)

        false_condition = (
            "jobs:\n"
            "  verify:\n"
            "    steps:\n"
            "      - name: disabled\n"
            "        if: ${{ false && true }}\n"
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1\n"
        )
        with self.assertRaises(AssertionError):
            _assert_pinned_actions_and_no_bypass(false_condition)

        with self.assertRaises(AssertionError):
            _assert_executable_assertions("if False:\n    assert value == 1\n", ("value == 1",))
        with self.assertRaises(AssertionError):
            _assert_executable_assertions(
                "import sys\nsys.exit(0)\nassert value == 1\n",
                ("value == 1",),
            )

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
            (
                self.semantic.replace(
                    "          assert sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}, sam[0]\n",
                    "          if False:\n"
                    "              assert sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}, sam[0]\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n",
                    "        'uses': actions/checkout@main\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    "        if: ${{ false }}\n"
                    "        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          import json\n",
                    "          import json, sys\n"
                    "          sys.exit(0)\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact Game revision\n"
                    "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n",
                    '      - {name: bad, "uses": actions/checkout@main}\n',
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "jobs:\n",
                    "jobs:\n"
                    "  injected: {runs-on: ubuntu-24.04, permissions: {contents: write}, steps: []}\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    "        if: ${{ false && true }}\n"
                    "        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    1,
                ),
                self.static,
            ),
        ))

        for semantic, static in mutations:
            with self.assertRaises(AssertionError):
                _assert_contract(semantic, static)


if __name__ == "__main__":
    unittest.main()