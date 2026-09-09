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
SEMANTIC_STEP_NAMES = (
    "Check out exact Game revision",
    "Verify exact checked-out revision",
    "Compile exporter",
    "Verify Atlas trigger closure",
    "Run deterministic and negative tests",
    "Verify authority boundaries",
    "Check out exact pinned migration evidence",
    "Build real pinned Game creature and semantic sources",
    "Qualify Sam and Thais on real pinned data",
)
STATIC_STEP_NAMES = (
    "Check out exact Game revision",
    "Set up Python",
    "Verify exact checked-out head",
    "Check out pinned migration evidence",
    "Verify migration evidence revision",
    "Compile and run producer self-test",
    "Verify Atlas trigger closure",
    "Build exact pinned product twice",
    "Verify exact role and creature census",
)

SEMANTIC_ORACLE_CODE = """
import json
data = json.load(open('/tmp/semantic-search-source.json', encoding='utf-8'))
sam = [record for record in data['records'] if record['kind'] == 'npc' and record['label'].casefold() == 'sam']
thais = [record for record in data['records'] if record['kind'] == 'town' and record['label'].casefold() == 'thais']
assert data['semantic_digest'] == 'sha256:035c911b11e588a969e8fb642965772eee598c30ad80d5d04f9ceda32461e530'
assert data['counts'] == {'records': 88684, 'kinds': {'monster': 87565, 'npc': 1068, 'town': 33, 'waypoint': 18}}
assert len(sam) == 1, len(sam)
assert sam[0]['id'] == 'npc:726487438c8308abf291622a52d91b24', sam[0]
assert sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}, sam[0]
assert 'shop' in sam[0]['capabilities'], sam[0]
assert sam[0]['provenance']['service_resolution_state'] == 'RESOLVED', sam[0]
assert len(thais) == 1, len(thais)
assert thais[0]['id'] == 'semantic-record:23716a35099a04179f7b9e3e6c9198ee', thais[0]
assert thais[0]['position'] == {'x': 32369, 'y': 32241, 'floor': -7}, thais[0]
assert thais[0]['bounds'] is None, thais[0]
assert data['input_floor_aliases']['7'] == -7
print(json.dumps({
    'semantic_digest': data['semantic_digest'],
    'records': data['counts']['records'],
    'sam_id': sam[0]['id'],
    'thais_id': thais[0]['id'],
}, sort_keys=True))
"""

STATIC_ORACLE_CODE = """
import collections, json
from pathlib import Path
data=json.loads(Path('/tmp/creatures-a.json').read_text())
assert data['contract_id']=='oteryn-game-atlas-export-v1'
assert data['capability']=='static-creatures-v1'
assert data['semantic_revision']==1
assert data['npc_role_schema_version']==1
assert data['statistics']=={'npcs':1068,'monster_spawns':87565,'unresolved':461,'ambiguous':5}
assert data['semantic_digest']=='sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4'
counts=collections.Counter(role for npc in data['npcs'] for role in npc.get('roles',[]))
assert dict(counts)=={'bank':25,'travel':51,'shop':313,'quest':432,'blessing':26,'trainer':54}
assert sum(bool(npc.get('roles')) for npc in data['npcs'])==705
assert sum(npc.get('role_resolution_state')=='AMBIGUOUS' for npc in data['npcs'])==10
assert all('roles' not in record and 'role_resolution_state' not in record for record in data['monster_spawns'])
print(json.dumps({'semantic_digest':data['semantic_digest'],'roles':dict(counts)},sort_keys=True))
"""


def _git_blob_sha(text: str) -> str:
    data = text.encode("utf-8")
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()


def _decode_yaml_scalar(raw: str) -> str:
    raw = raw.strip()
    assert raw, "empty YAML scalar"
    if raw.startswith("'"):
        assert raw.endswith("'") and len(raw) >= 2
        return raw[1:-1].replace("''", "'")
    if raw.startswith('"'):
        assert raw.endswith('"') and len(raw) >= 2
        value = json.loads(raw)
        assert isinstance(value, str)
        return value
    assert re.fullmatch(r"[A-Za-z0-9_./*?+-]+", raw), f"unsupported YAML scalar: {raw!r}"
    return raw


def _strip_yaml_inline_comment(raw: str) -> str:
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


def _mask_quoted_scalars(raw: str) -> str:
    output = list(raw)
    quote: str | None = None
    index = 0
    while index < len(raw):
        char = raw[index]
        if quote == "'":
            output[index] = " "
            if char == "'":
                if index + 1 < len(raw) and raw[index + 1] == "'":
                    output[index + 1] = " "
                    index += 2
                    continue
                quote = None
            index += 1
            continue
        if quote == '"':
            output[index] = " "
            if char == "\\" and index + 1 < len(raw):
                output[index + 1] = " "
                index += 2
                continue
            if char == '"':
                quote = None
            index += 1
            continue
        if char in ("'", '"'):
            quote = char
            output[index] = " "
        index += 1
    assert quote is None, f"unterminated YAML quote: {raw!r}"
    return "".join(output)


def _assert_safe_yaml_structure(workflow: str) -> None:
    """Reject YAML features that could hide protected keys from the block parser."""
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
        masked = _mask_quoted_scalars(structural)
        assert not any(marker in masked for marker in ("{", "}", "&", "*", "!")), (
            f"advanced/flow YAML syntax is forbidden in protected workflows: {line!r}"
        )
        assert not masked.lstrip().startswith("?"), (
            f"explicit YAML mapping keys are forbidden: {line!r}"
        )
        assert "<<:" not in masked, f"YAML merge keys are forbidden: {line!r}"
        entry = _mapping_entry(line)
        assert not (entry is not None and entry[0] == "env" and indent < 8), (
            f"workflow/job environment mappings are forbidden: {line!r}"
        )
        assert not (indent == 6 and structural.strip() == "-"), (
            f"bare workflow step sequence markers are forbidden: {line!r}"
        )
        assert not (indent == 6 and re.match(r"-\s+\?", masked.lstrip())), (
            f"sequence-prefixed explicit mapping keys are forbidden: {line!r}"
        )
        if indent == 6 and masked.lstrip().startswith("- "):
            step_entry = _mapping_entry(line)
            assert step_entry is None or step_entry[0] == "name", (
                f"block-style workflow step entries must start with name: {line!r}"
            )


def _quoted_scalar_end(text: str) -> int | None:
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


def _mapping_entry(line: str) -> tuple[str, str] | None:
    text = line.lstrip(" \t")
    if not text or text.startswith("#"):
        return None
    if text.startswith("- "):
        text = text[2:].lstrip()
    elif text.startswith("-"):
        return None
    if not text:
        return None
    if text.startswith(("'", '"')):
        end = _quoted_scalar_end(text)
        assert end is not None, f"malformed quoted mapping key: {line!r}"
        raw_key = text[: end + 1]
        remainder = text[end + 1 :].lstrip()
        if not remainder:
            return None
        assert remainder.startswith(":"), f"unsupported quoted mapping syntax: {line!r}"
        raw_value = remainder[1:].strip()
    else:
        colon = text.find(":")
        if colon <= 0:
            return None
        raw_key = text[:colon].rstrip()
        raw_value = text[colon + 1 :].strip()
    return _decode_yaml_scalar(raw_key), _strip_yaml_inline_comment(raw_value)


def _yaml_structural_lines(workflow: str):
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
        yield line
        if re.search(r":\s*[|>][+-]?\d?\s*$", structural):
            block_scalar_indent = indent


def _decoded_mapping_values(workflow: str, wanted_key: str) -> tuple[str, ...]:
    values: list[str] = []
    for line in _yaml_structural_lines(workflow):
        entry = _mapping_entry(line)
        if entry is not None and entry[0] == wanted_key:
            values.append(entry[1])
    return tuple(values)


def _decoded_mapping_keys(workflow: str) -> tuple[str, ...]:
    return tuple(
        entry[0]
        for line in _yaml_structural_lines(workflow)
        if (entry := _mapping_entry(line)) is not None
    )


def _decode_mapping_scalar(raw: str) -> str:
    raw = _strip_yaml_inline_comment(raw).strip()
    assert raw, "empty YAML mapping value"
    if raw.startswith(("'", '"')):
        return _decode_yaml_scalar(raw)
    assert not any(char.isspace() for char in raw), f"unsupported YAML mapping value: {raw!r}"
    return raw


def _decode_inline_sequence(raw: str) -> tuple[str, ...]:
    raw = _strip_yaml_inline_comment(raw).strip()
    assert raw.startswith("[") and raw.endswith("]"), f"expected inline YAML sequence: {raw!r}"
    inner = raw[1:-1].strip()
    assert inner, "empty inline YAML sequence"
    return tuple(_decode_yaml_scalar(part) for part in inner.split(","))


def _top_level_event_keys(workflow: str) -> tuple[str, ...]:
    match = re.search(r"(?ms)^on:\n(?P<body>.*?)(?=^[A-Za-z_][A-Za-z0-9_-]*:|\Z)", workflow)
    assert match is not None, "missing canonical on block"
    keys: list[str] = []
    for line in match.group("body").splitlines():
        if len(line) - len(line.lstrip(" ")) != 2:
            continue
        entry = _mapping_entry(line)
        if entry is not None:
            keys.append(entry[0])
    return tuple(keys)


def _event_block(workflow: str, event: str) -> str:
    match = re.search(
        rf"(?ms)^  {re.escape(event)}:(.*?)(?=^  [a-z_]+:|^permissions:)",
        workflow,
    )
    assert match is not None, f"missing event {event}"
    return match.group(1)


def _event_keys(workflow: str, event: str) -> tuple[str, ...]:
    keys: list[str] = []
    for line in _event_block(workflow, event).splitlines():
        if len(line) - len(line.lstrip(" ")) != 4:
            continue
        entry = _mapping_entry(line)
        if entry is not None:
            keys.append(entry[0])
    return tuple(keys)


def _event_mapping_value(workflow: str, event: str, wanted_key: str) -> str:
    values: list[str] = []
    for line in _event_block(workflow, event).splitlines():
        if len(line) - len(line.lstrip(" ")) != 4:
            continue
        entry = _mapping_entry(line)
        if entry is not None and entry[0] == wanted_key:
            values.append(entry[1])
    assert len(values) == 1, f"expected exactly one {wanted_key!r} mapping for event {event!r}"
    return values[0]


def _branches(workflow: str, event: str) -> tuple[str, ...]:
    return _decode_inline_sequence(_event_mapping_value(workflow, event, "branches"))


def _paths(workflow: str, event: str) -> tuple[str, ...]:
    lines = _event_block(workflow, event).splitlines()
    try:
        start = lines.index("    paths:") + 1
    except ValueError as exc:
        raise AssertionError(f"missing canonical paths list for {event}") from exc
    entries: list[str] = []
    for line in lines[start:]:
        if len(line) - len(line.lstrip(" ")) == 4 and _mapping_entry(line) is not None:
            break
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        item = re.fullmatch(r"      - (.+)", line)
        assert item is not None, f"unexpected content in {event} paths: {line!r}"
        entries.append(_decode_yaml_scalar(item.group(1)))
    assert entries, f"empty paths list for {event}"
    return tuple(entries)


def _step_block(workflow: str, name: str) -> str:
    match = re.search(
        rf"(?ms)^      - name: {re.escape(name)}\n(?P<body>.*?)(?=^      - name:|\Z)",
        workflow,
    )
    assert match is not None, f"missing workflow step {name!r}"
    return match.group("body")


def _step_names(workflow: str) -> tuple[str, ...]:
    names: list[str] = []
    for line in workflow.splitlines():
        if len(line) - len(line.lstrip(" ")) != 6:
            continue
        entry = _mapping_entry(line)
        if entry is not None and entry[0] == "name":
            names.append(_strip_yaml_inline_comment(entry[1]).strip())
    return tuple(names)


def _python_heredoc(workflow: str, step_name: str) -> str:
    """Require the oracle Python heredoc to be the first reachable shell command."""
    lines = _step_block(workflow, step_name).rstrip().splitlines()
    assert lines[:4] == [
        "        shell: bash",
        "        run: |",
        "          set -euo pipefail",
        "          python - <<'PY'",
    ], f"oracle shell prefix changed or became bypassable in {step_name!r}"
    assert lines[-1] == "          PY", f"oracle heredoc must be the final shell command in {step_name!r}"
    code_lines = lines[4:-1]
    assert code_lines and all(line.startswith("          ") for line in code_lines)
    return textwrap.dedent("\n".join(code_lines)) + "\n"


def _assert_exact_oracle_code(code: str, expected: str) -> None:
    actual_tree = ast.parse(code)
    expected_tree = ast.parse(textwrap.dedent(expected).strip() + "\n")
    assert ast.dump(actual_tree, include_attributes=False) == ast.dump(expected_tree, include_attributes=False), (
        "oracle setup, checks, diagnostics, or control flow changed"
    )


def _assert_exact_step_lines(workflow: str, step_name: str, expected: tuple[str, ...]) -> None:
    lines = tuple(_step_block(workflow, step_name).rstrip().splitlines())
    assert lines == expected, f"protected step changed or became bypassable: {step_name!r}"


def _assert_exact_run_step(workflow: str, step_name: str, command: str) -> None:
    _assert_exact_step_lines(workflow, step_name, (f"        run: {command}",))


def _assert_exact_bash_step(workflow: str, step_name: str, commands: tuple[str, ...]) -> None:
    expected = (
        "        shell: bash",
        "        run: |",
        "          set -euo pipefail",
        *(f"          {command}" for command in commands),
    )
    _assert_exact_step_lines(workflow, step_name, expected)


def _assert_read_only_permissions(workflow: str) -> None:
    top = re.search(r"(?ms)^permissions:\n(?P<body>(?:^  [^\n]+\n)+)", workflow)
    assert top is not None, "missing top-level permissions"
    assert top.group("body") == "  contents: read\n", "top-level permissions must remain contents: read"
    keys = _decoded_mapping_keys(workflow)
    assert keys.count("permissions") == 1, "job/step permission overrides are forbidden"
    assert "write-all" not in workflow


def _assert_pinned_actions_and_no_bypass(workflow: str) -> None:
    actions = tuple(_decode_mapping_scalar(raw) for raw in _decoded_mapping_values(workflow, "uses"))
    assert actions
    assert all(re.fullmatch(r"[^@\s#]+@[0-9a-f]{40}", action) for action in actions), (
        "every uses action must be pinned to a 40-character commit SHA"
    )
    keys = _decoded_mapping_keys(workflow)
    assert "continue-on-error" not in keys
    assert "if" not in keys, "workflow if conditions are forbidden in protected Atlas qualification workflows"
    assert "defaults" not in keys, "workflow/job run defaults are forbidden in protected Atlas qualification workflows"
    for dangerous in ("PYTHONOPTIMIZE", "BASH_ENV"):
        assert dangerous not in keys and dangerous not in workflow, f"dangerous execution environment key: {dangerous}"


def _assert_semantic_execution_steps(workflow: str) -> None:
    assert _step_names(workflow) == SEMANTIC_STEP_NAMES, "semantic workflow step sequence changed"
    _assert_exact_step_lines(
        workflow,
        "Check out exact Game revision",
        (
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
            "        with:",
            "          ref: ${{ github.event.pull_request.head.sha || github.sha }}",
            "          persist-credentials: false",
        ),
    )
    _assert_exact_step_lines(
        workflow,
        "Verify exact checked-out revision",
        (
            "        shell: bash",
            "        env:",
            "          EXPECTED_SHA: ${{ github.event.pull_request.head.sha || github.sha }}",
            '        run: test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"',
        ),
    )
    _assert_exact_run_step(
        workflow,
        "Compile exporter",
        "python -m py_compile tools/game-atlas-semantic-search/export.py tools/game-atlas-semantic-search/self_test.py",
    )
    _assert_exact_run_step(workflow, "Verify Atlas trigger closure", REGRESSION_COMMAND)
    _assert_exact_run_step(
        workflow,
        "Run deterministic and negative tests",
        "python tools/game-atlas-semantic-search/self_test.py",
    )
    _assert_exact_bash_step(
        workflow,
        "Verify authority boundaries",
        (
            "grep -q 'semantic-search-source-v1' docs/contracts/OTERYN_GAME_ATLAS_SEMANTIC_SEARCH_PROFILE_V1.md",
            "grep -q 'input_floor_aliases' tools/game-atlas-semantic-search/export.py",
            "! grep -R -E 'action_id|unique_id' tools/game-atlas-semantic-search --include='*.json'",
        ),
    )
    _assert_exact_step_lines(
        workflow,
        "Check out exact pinned migration evidence",
        (
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
            "        with:",
            "          repository: blakinio/Otheryn",
            "          ref: e417c5e7c22986bf4acef0495eb47f7b72c97cce",
            "          path: legacy",
            "          persist-credentials: false",
        ),
    )
    _assert_exact_bash_step(
        workflow,
        "Build real pinned Game creature and semantic sources",
        (
            "python tools/game-atlas-creatures/export.py \\",
            "  legacy/vendor/map-analysis/crystalserver/data-global/world \\",
            "  legacy/vendor/map-analysis/crystalserver/data-global/npc \\",
            "  legacy/vendor/map-analysis/crystalserver/data-global/monster \\",
            "  /tmp/static-creatures.json",
            "python tools/game-atlas-semantic-search/export.py \\",
            "  --creatures /tmp/static-creatures.json \\",
            "  --npc-root legacy/vendor/map-analysis/crystalserver/data-global/npc \\",
            "  --legacy-root legacy \\",
            "  --map-path legacy/vendor/map-analysis/crystalserver/data-global/world/world.otbm \\",
            "  --output /tmp/semantic-search-source.json",
        ),
    )


def _assert_static_execution_steps(workflow: str) -> None:
    assert _step_names(workflow) == STATIC_STEP_NAMES, "static workflow step sequence changed"
    _assert_exact_step_lines(
        workflow,
        "Check out exact Game revision",
        (
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
            "        with:",
            "          ref: ${{ github.event.pull_request.head.sha || github.sha }}",
            "          fetch-depth: 1",
            "          persist-credentials: false",
        ),
    )
    _assert_exact_step_lines(
        workflow,
        "Set up Python",
        (
            "        uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 # v6.0.0",
            "        with:",
            "          python-version: '3.12'",
            "          check-latest: false",
        ),
    )
    _assert_exact_step_lines(
        workflow,
        "Verify exact checked-out head",
        (
            "        shell: bash",
            "        env:",
            "          EXPECTED_HEAD: ${{ github.event.pull_request.head.sha || github.sha }}",
            "        run: |",
            "          set -euo pipefail",
            '          test "$(git rev-parse HEAD)" = "$EXPECTED_HEAD"',
        ),
    )
    _assert_exact_step_lines(
        workflow,
        "Check out pinned migration evidence",
        (
            "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
            "        with:",
            "          repository: blakinio/Otheryn",
            "          ref: e417c5e7c22986bf4acef0495eb47f7b72c97cce",
            "          path: legacy",
            "          fetch-depth: 1",
            "          persist-credentials: false",
        ),
    )
    _assert_exact_bash_step(
        workflow,
        "Verify migration evidence revision",
        (
            "test \"$(git -C legacy rev-parse HEAD)\" = 'e417c5e7c22986bf4acef0495eb47f7b72c97cce'",
            "test -d legacy/vendor/map-analysis/crystalserver/data-global/world",
            "test -d legacy/vendor/map-analysis/crystalserver/data-global/npc",
            "test -d legacy/vendor/map-analysis/crystalserver/data-global/monster",
        ),
    )
    _assert_exact_bash_step(
        workflow,
        "Compile and run producer self-test",
        (
            "python -m py_compile tools/game-atlas-creatures/export.py tools/game-atlas-creatures/self_test.py",
            "python tools/game-atlas-creatures/self_test.py",
        ),
    )
    _assert_exact_run_step(workflow, "Verify Atlas trigger closure", REGRESSION_COMMAND)
    _assert_exact_bash_step(
        workflow,
        "Build exact pinned product twice",
        (
            'ROOT="$GITHUB_WORKSPACE/legacy/vendor/map-analysis/crystalserver/data-global"',
            'python tools/game-atlas-creatures/export.py "$ROOT/world" "$ROOT/npc" "$ROOT/monster" /tmp/creatures-a.json',
            'python tools/game-atlas-creatures/export.py "$ROOT/world" "$ROOT/npc" "$ROOT/monster" /tmp/creatures-b.json',
            "cmp /tmp/creatures-a.json /tmp/creatures-b.json",
        ),
    )


def _assert_contract(semantic: str, static: str, *, verify_blobs: bool = True) -> None:
    _assert_safe_yaml_structure(semantic)
    _assert_safe_yaml_structure(static)

    assert _top_level_event_keys(semantic) == ("pull_request", "push")
    assert _top_level_event_keys(static) == ("pull_request", "workflow_dispatch")
    assert _event_keys(semantic, "pull_request") == ("paths",)
    assert _event_keys(semantic, "push") == ("branches", "paths")
    assert _event_keys(static, "pull_request") == ("branches", "paths")
    assert _branches(semantic, "push") == ("main",)
    assert _branches(static, "pull_request") == ("main",)
    assert _paths(semantic, "pull_request") == SEMANTIC_PR_PATHS
    assert _paths(semantic, "push") == SEMANTIC_PUSH_PATHS
    assert _paths(static, "pull_request") == STATIC_PR_PATHS

    _assert_semantic_execution_steps(semantic)
    _assert_static_execution_steps(static)
    _assert_read_only_permissions(semantic)
    _assert_read_only_permissions(static)
    _assert_pinned_actions_and_no_bypass(semantic)
    _assert_pinned_actions_and_no_bypass(static)

    _assert_exact_oracle_code(
        _python_heredoc(semantic, "Qualify Sam and Thais on real pinned data"),
        SEMANTIC_ORACLE_CODE,
    )
    _assert_exact_oracle_code(
        _python_heredoc(static, "Verify exact role and creature census"),
        STATIC_ORACLE_CODE,
    )

    if verify_blobs:
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
        for dependency in (CREATURE_EXPORT, CREATURE_IDENTITY, FULLWORLD_PRODUCER, THAIS_PRODUCER):
            self.assertIn(dependency, _paths(self.semantic, "pull_request"))
            self.assertIn(dependency, _paths(self.semantic, "push"))
        self.assertIn(CREATURE_IDENTITY, _paths(self.static, "pull_request"))

    def test_shared_regression_and_workflow_changes_select_both(self) -> None:
        semantic_pr = _paths(self.semantic, "pull_request")
        static_pr = _paths(self.static, "pull_request")
        for changed_path in (REGRESSION, SEMANTIC_WORKFLOW_PATH, STATIC_WORKFLOW_PATH):
            self.assertIn(changed_path, semantic_pr)
            self.assertIn(changed_path, static_pr)
        _assert_exact_run_step(self.semantic, "Verify Atlas trigger closure", REGRESSION_COMMAND)
        _assert_exact_run_step(self.static, "Verify Atlas trigger closure", REGRESSION_COMMAND)

    def test_event_models_original_paths_and_exclusions(self) -> None:
        self.assertEqual(_top_level_event_keys(self.semantic), ("pull_request", "push"))
        self.assertEqual(_top_level_event_keys(self.static), ("pull_request", "workflow_dispatch"))
        self.assertEqual(_branches(self.semantic, "push"), ("main",))
        self.assertEqual(_branches(self.static, "pull_request"), ("main",))
        self.assertEqual(_paths(self.semantic, "pull_request"), SEMANTIC_PR_PATHS)
        self.assertEqual(_paths(self.semantic, "push"), SEMANTIC_PUSH_PATHS)
        self.assertEqual(_paths(self.static, "pull_request"), STATIC_PR_PATHS)
        self.assertEqual(_event_keys(self.semantic, "pull_request"), ("paths",))
        self.assertEqual(_event_keys(self.semantic, "push"), ("branches", "paths"))
        self.assertEqual(_event_keys(self.static, "pull_request"), ("branches", "paths"))

    def test_review_bypasses_fail_closed_without_blob_binding(self) -> None:
        attacks = [
            (
                self.semantic.replace(
                    "  semantic-search-source:\n    runs-on: ubuntu-24.04\n",
                    "  semantic-search-source:\n    env:\n      PYTHONPATH: tools/game-atlas-semantic-search\n    runs-on: ubuntu-24.04\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "  verify:\n    name: Game Atlas static creature producer / exact-source\n    runs-on: ubuntu-24.04\n",
                    "  verify:\n    name: Game Atlas static creature producer / exact-source\n    env:\n      PYTHONHOME: /tmp/atlas-python-home\n    runs-on: ubuntu-24.04\n",
                    1,
                ),
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact Game revision\n"
                    "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n"
                    "        with:\n"
                    "          ref: ${{ github.event.pull_request.head.sha || github.sha }}\n"
                    "          persist-credentials: false\n",
                    "      - name: Check out exact Game revision\n"
                    "        run: echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "      - name: Check out exact Game revision\n"
                    "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n"
                    "        with:\n"
                    "          ref: ${{ github.event.pull_request.head.sha || github.sha }}\n"
                    "          fetch-depth: 1\n"
                    "          persist-credentials: false\n",
                    "      - name: Check out exact Game revision\n"
                    "        run: echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n",
                    1,
                ),
            ),
            (
                self.semantic.replace(
                    "          set -euo pipefail\n          python - <<'PY'\n",
                    "          set -euo pipefail\n          exit 0\n          python - <<'PY'\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          import json\n",
                    "          from sys import exit as done\n          ignored = done(0)\n          import json\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "        shell: bash\n        run: |\n          set -euo pipefail\n          python - <<'PY'\n",
                    "        shell: bash\n        env:\n          PYTHONOPTIMIZE: '1'\n        run: |\n          set -euo pipefail\n          python - <<'PY'\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Verify Atlas trigger closure\n",
                    "      - name: &conditional_key if\n"
                    "        ? *conditional_key\n"
                    "        : ${{ false && true }}\n"
                    "      - name: Verify Atlas trigger closure\n",
                    1,
                ),
                self.static,
            ),
            (self.semantic.replace("  push:\n", "  workflow_dispatch:\n  push:\n", 1), self.static),
            (
                self.semantic,
                self.static.replace(
                    "  workflow_dispatch:\n",
                    "  workflow_dispatch:\n  schedule:\n    - cron: '0 0 * * *'\n",
                    1,
                ),
            ),
            (self.semantic, self.static.replace("  workflow_dispatch:\n", "", 1)),
            (
                self.semantic.replace(
                    "    branches: [main]\n",
                    "    # branches: [main]\n    branches:\n      - develop\n",
                    1,
                ),
                self.static,
            ),
            (self.semantic, self.static.replace("    branches: [main]\n", "    branches: [develop]\n", 1)),
            (
                self.semantic.replace(
                    "      - name: Verify Atlas trigger closure\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    "      - name: Verify Atlas trigger closure\n        shell: \"true {0}\"\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "      - name: Verify Atlas trigger closure\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    "      - name: Verify Atlas trigger closure\n        shell: \"true {0}\"\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    1,
                ),
            ),
            (
                self.semantic.replace(
                    "      - name: Run deterministic and negative tests\n        run: python tools/game-atlas-semantic-search/self_test.py\n",
                    "      - name: Run deterministic and negative tests\n        shell: \"true {0}\"\n        run: python tools/game-atlas-semantic-search/self_test.py\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "  semantic-search-source:\n    runs-on: ubuntu-24.04\n",
                    "  semantic-search-source:\n    defaults:\n      run:\n        shell: \"true {0}\"\n    runs-on: ubuntu-24.04\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          ! grep -R -E 'action_id|unique_id' tools/game-atlas-semantic-search --include='*.json'\n",
                    "          ! grep -R -E 'action_id|unique_id' tools/game-atlas-semantic-search --include='*.json'\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - name: Inject environment persistence\n"
                    "        run: echo 'BASH_ENV=/tmp/atlas-bypass' >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - run: |\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - # unnamed executable step\n"
                    "        run: |\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - ? run\n"
                    "        : |\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace("          python tools/game-atlas-creatures/self_test.py\n", "", 1),
            ),
            (
                self.semantic,
                self.static.replace(
                    "          cmp /tmp/creatures-a.json /tmp/creatures-b.json\n",
                    "          exit 0\n          cmp /tmp/creatures-a.json /tmp/creatures-b.json\n",
                    1,
                ),
            ),
        ]
        for semantic, static in attacks:
            with self.assertRaises(AssertionError):
                _assert_contract(semantic, static, verify_blobs=False)

    def test_exact_oracles_and_workflow_safety_remain(self) -> None:
        _assert_contract(self.semantic, self.static)

    def test_every_control_is_mutation_protected_without_blob_binding(self) -> None:
        mutations: list[tuple[str, str]] = []
        for path in (CREATURE_EXPORT, CREATURE_IDENTITY, FULLWORLD_PRODUCER, THAIS_PRODUCER):
            mutations.append((self.semantic.replace(f"      - '{path}'\n", "", 1), self.static))
            push_pos = self.semantic.index("  push:")
            mutations.append((
                self.semantic[:push_pos] + self.semantic[push_pos:].replace(f"      - '{path}'\n", "", 1),
                self.static,
            ))
        for path in (REGRESSION, STATIC_WORKFLOW_PATH):
            mutations.append((self.semantic.replace(f"      - '{path}'\n", "", 1), self.static))
        for path in (CREATURE_IDENTITY, REGRESSION, SEMANTIC_WORKFLOW_PATH):
            mutations.append((self.semantic, self.static.replace(f"      - '{path}'\n", "", 1)))

        mutations.extend((
            (
                self.semantic.replace(
                    "  semantic-search-source:\n    runs-on: ubuntu-24.04\n",
                    "  semantic-search-source:\n    env:\n      PYTHONPATH: tools/game-atlas-semantic-search\n    runs-on: ubuntu-24.04\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "  verify:\n    name: Game Atlas static creature producer / exact-source\n    runs-on: ubuntu-24.04\n",
                    "  verify:\n    name: Game Atlas static creature producer / exact-source\n    env:\n      PYTHONHOME: /tmp/atlas-python-home\n    runs-on: ubuntu-24.04\n",
                    1,
                ),
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact Game revision\n"
                    "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n"
                    "        with:\n"
                    "          ref: ${{ github.event.pull_request.head.sha || github.sha }}\n"
                    "          persist-credentials: false\n",
                    "      - name: Check out exact Game revision\n"
                    "        run: echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "      - name: Check out exact Game revision\n"
                    "        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n"
                    "        with:\n"
                    "          ref: ${{ github.event.pull_request.head.sha || github.sha }}\n"
                    "          fetch-depth: 1\n"
                    "          persist-credentials: false\n",
                    "      - name: Check out exact Game revision\n"
                    "        run: echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n",
                    1,
                ),
            ),
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
                    "    runs-on: ubuntu-24.04\n    'permissions':\n      contents: write\n",
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
                    "          if False:\n              assert sam[0]['position'] == {'x': 32361, 'y': 32198, 'floor': -7}, sam[0]\n",
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
                    "        if: ${{ false && true }}\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
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
                    "jobs:\n  injected: {runs-on: ubuntu-24.04, permissions: {contents: write}, steps: []}\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          set -euo pipefail\n          python - <<'PY'\n",
                    "          set -euo pipefail\n          exit 0\n          python - <<'PY'\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          import json\n",
                    "          from sys import exit as done\n          ignored = done(0)\n          import json\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "        shell: bash\n        run: |\n          set -euo pipefail\n          python - <<'PY'\n",
                    "        shell: bash\n        env:\n          PYTHONOPTIMIZE: '1'\n        run: |\n          set -euo pipefail\n          python - <<'PY'\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Verify Atlas trigger closure\n",
                    "      - name: &conditional_key if\n"
                    "        ? *conditional_key\n"
                    "        : ${{ false && true }}\n"
                    "      - name: Verify Atlas trigger closure\n",
                    1,
                ),
                self.static,
            ),
            (self.semantic.replace("  push:\n", "  workflow_dispatch:\n  push:\n", 1), self.static),
            (self.semantic, self.static.replace("  workflow_dispatch:\n", "", 1)),
            (
                self.semantic.replace(
                    "    branches: [main]\n",
                    "    # branches: [main]\n    branches:\n      - develop\n",
                    1,
                ),
                self.static,
            ),
            (self.semantic, self.static.replace("    branches: [main]\n", "    branches: [develop]\n", 1)),
            (
                self.semantic.replace(
                    "      - name: Verify Atlas trigger closure\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    "      - name: Verify Atlas trigger closure\n        shell: \"true {0}\"\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace(
                    "      - name: Verify Atlas trigger closure\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    "      - name: Verify Atlas trigger closure\n        shell: \"true {0}\"\n        run: python tools/repository/test_validate_game_atlas_semantic_search_triggers.py\n",
                    1,
                ),
            ),
            (
                self.semantic.replace(
                    "      - name: Run deterministic and negative tests\n        run: python tools/game-atlas-semantic-search/self_test.py\n",
                    "      - name: Run deterministic and negative tests\n        shell: \"true {0}\"\n        run: python tools/game-atlas-semantic-search/self_test.py\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "  semantic-search-source:\n    runs-on: ubuntu-24.04\n",
                    "  semantic-search-source:\n    defaults:\n      run:\n        shell: \"true {0}\"\n    runs-on: ubuntu-24.04\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "          ! grep -R -E 'action_id|unique_id' tools/game-atlas-semantic-search --include='*.json'\n",
                    "          ! grep -R -E 'action_id|unique_id' tools/game-atlas-semantic-search --include='*.json'\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - name: Inject environment persistence\n"
                    "        run: echo 'BASH_ENV=/tmp/atlas-bypass' >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - run: |\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - # unnamed executable step\n"
                    "        run: |\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic.replace(
                    "      - name: Check out exact pinned migration evidence\n",
                    "      - ? run\n"
                    "        : |\n"
                    "          printf 'exit 0\\n' > /tmp/atlas-bypass\n"
                    "          echo \"BASH\"\"_ENV=/tmp/atlas-bypass\" >> \"$GITHUB_ENV\"\n\n"
                    "      - name: Check out exact pinned migration evidence\n",
                    1,
                ),
                self.static,
            ),
            (
                self.semantic,
                self.static.replace("          python tools/game-atlas-creatures/self_test.py\n", "", 1),
            ),
            (
                self.semantic,
                self.static.replace(
                    "          cmp /tmp/creatures-a.json /tmp/creatures-b.json\n",
                    "          exit 0\n          cmp /tmp/creatures-a.json /tmp/creatures-b.json\n",
                    1,
                ),
            ),
        ))

        for semantic, static in mutations:
            with self.assertRaises(AssertionError):
                _assert_contract(semantic, static, verify_blobs=False)


if __name__ == "__main__":
    unittest.main()
