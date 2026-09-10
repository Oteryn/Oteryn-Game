#!/usr/bin/env python3
"""Exercise the required Merge Queue contract and its credibility controls."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
GATE = ROOT / ".github/workflows/merge-group-gate.yml"
LIFECYCLE = ROOT / "tools/agents/tests/test_governance_lifecycle_discovery.py"
APPROVED = "c59b30fde7538e738346eec03a602081dc4ac2d6"
LIFECYCLE_COMMAND = "python tools/agents/tests/test_governance_lifecycle_discovery.py"
NATIVE_POLICY = (
    "$ErrorActionPreference = 'Stop'",
    "$PSNativeCommandUseErrorActionPreference = $true",
)
WINDOWS_NATIVE_COMMANDS = (
    "cargo +1.94.0 build --locked --release -p oteryn-client --target x86_64-pc-windows-msvc",
    "cargo +1.94.0 clippy --locked -p oteryn-client --all-targets --target x86_64-pc-windows-msvc -- -D warnings",
    "cargo +1.94.0 run --locked -p oteryn-client --target x86_64-pc-windows-msvc -- --smoke",
    "cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness",
)


def _powershell_native_canaries() -> None:
    """Prove Stop + native-error policy prevents later command masking."""
    pwsh = shutil.which("pwsh")
    assert pwsh is not None, "pwsh is required for WP1 native failure-propagation qualification"
    executable = sys.executable.replace("'", "''")

    def run_case(failure_position: int | None) -> tuple[int, list[bool]]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            lines = [*NATIVE_POLICY]
            for index in range(4):
                marker = str(root / f"command-{index}.hit").replace("'", "''")
                lines.append(f"Set-Content -LiteralPath '{marker}' -Value hit")
                code = 7 if failure_position == index else 0
                lines.append(f"& '{executable}' -c 'import sys; sys.exit({code})'")
            result = subprocess.run(
                [pwsh, "-NoProfile", "-NonInteractive", "-Command", "; ".join(lines)],
                cwd=ROOT,
                text=True,
                capture_output=True,
                timeout=30,
                check=False,
            )
            markers = [(root / f"command-{index}.hit").exists() for index in range(4)]
            return result.returncode, markers

    result, markers = run_case(None)
    assert result == 0 and markers == [True] * 4, (result, markers)
    for position in range(4):
        result, markers = run_case(position)
        assert result != 0, (position, result)
        assert markers[: position + 1] == [True] * (position + 1), (position, markers)
        assert markers[position + 1 :] == [False] * (3 - position), (position, markers)


def main() -> int:
    spec = importlib.util.spec_from_file_location(
        "queue_policy_core", Path(__file__).with_name("validate_repository_policy_core.py")
    )
    assert spec is not None and spec.loader is not None
    core = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(core)
    original = GATE.read_text(encoding="utf-8")
    assert core.git_blob_sha(original.encode()) == APPROVED, "queue protected blob pin drifted"
    assert core.main() == 0, "approved queue workflow must pass full policy"

    candidate = core.indented_yaml_mapping_block(original, "candidate", 2)
    assert candidate is not None
    assert LIFECYCLE_COMMAND in candidate, "required MQ candidate does not execute lifecycle discovery"
    for fragment in (
        "      rust: ${{ steps.lanes.outputs.rust }}",
        "      windows: ${{ steps.lanes.outputs.windows }}",
        "      - name: Check out protected-base classifier input",
        "          ref: ${{ github.event.merge_group.base_sha }}",
        "      - name: Classify trusted merge-group lanes",
        "path.startswith('docs/architecture/') and path.endswith('.md')",
        "result = {'rust': 'false', 'windows': 'false', 'surface': 'architecture-docs'}",
    ):
        assert fragment in candidate, f"Merge Queue trusted docs classifier missing: {fragment}"

    for job, condition in (
        ("rust_linux", "if: needs.candidate.outputs.rust == 'true'"),
        ("durability_postgres", "if: needs.candidate.outputs.rust == 'true'"),
        ("rust_windows", "if: needs.candidate.outputs.windows == 'true'"),
        ("rust_supply_chain", "if: needs.candidate.outputs.rust == 'true'"),
    ):
        block = core.indented_yaml_mapping_block(original, job, 2)
        assert block is not None and condition in block, (job, condition)

    windows = core.indented_yaml_mapping_block(original, "rust_windows", 2)
    assert windows is not None
    for policy in NATIVE_POLICY:
        assert policy in windows, f"Merge Queue Windows block lacks fail-closed policy: {policy}"
    for command in WINDOWS_NATIVE_COMMANDS:
        assert windows.count(command) == 1, f"required Windows command missing or duplicated: {command}"

    lifecycle = subprocess.run(
        [sys.executable, str(LIFECYCLE)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=60,
        check=False,
    )
    assert lifecycle.returncode == 0, lifecycle.stdout + lifecycle.stderr
    assert "Ran 2 tests" in lifecycle.stderr and "OK" in lifecycle.stderr, lifecycle.stderr
    _powershell_native_canaries()

    read_text = Path.read_text

    def validate(text: str) -> int:
        def read(path: Path, *args, **kwargs):
            return text if path == GATE else read_text(path, *args, **kwargs)
        with patch.object(Path, "read_text", read), contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            return core.main()

    mutations = 0
    for job in ("durability_postgres", "rust_windows"):
        for key in ("if: false", "continue-on-error: true", '"continue-on-error": true'):
            changed = original.replace(f"  {job}:\n", f"  {job}:\n    {key}\n", 1)
            assert changed != original and validate(changed) != 0, (job, key)
            mutations += 1
    for command in (
        "          cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres",
        "        run: cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc",
    ):
        assert command in original
        for replacement in (command.replace("cargo", "echo cargo", 1), "        if: false\n" + command):
            assert validate(original.replace(command, replacement, 1)) != 0
            mutations += 1
    early_exit = original.replace(
        "          test -f apps/game-server/tests/durability_postgres.rs",
        "          exit 0\n          test -f apps/game-server/tests/durability_postgres.rs",
        1,
    )
    assert validate(early_exit) != 0
    mutations += 1

    for fragment in (LIFECYCLE_COMMAND, *NATIVE_POLICY):
        assert original.count(fragment) == 1
        assert validate(original.replace(fragment, "", 1)) != 0, fragment
        mutations += 1

    block = core.indented_yaml_mapping_block(original, "game_gate", 2)
    assert block is not None
    script = textwrap.dedent(block.split("        run: |\n", 1)[1])
    predicates = (
        "CANDIDATE", "DEPENDENCY_REVIEW", "CODEQL", "RUST_LINUX",
        "DURABILITY_POSTGRES", "RUST_WINDOWS", "RUST_SUPPLY_CHAIN",
    )
    env = dict(os.environ, **dict.fromkeys(predicates, "success"))
    assert subprocess.run(["bash", "-c", script], env=env, check=False).returncode == 0
    for predicate in predicates:
        for failure in ("failure", "skipped", "cancelled", ""):
            assert subprocess.run(
                ["bash", "-c", script], env=dict(env, **{predicate: failure}), check=False
            ).returncode != 0, (predicate, failure)

    docs_env = dict(
        env,
        RUST_REQUIRED="false",
        WINDOWS_REQUIRED="false",
        RUST_LINUX="skipped",
        DURABILITY_POSTGRES="skipped",
        RUST_WINDOWS="skipped",
        RUST_SUPPLY_CHAIN="skipped",
    )
    assert subprocess.run(["bash", "-c", script], env=docs_env, check=False).returncode == 0
    for predicate in ("RUST_LINUX", "DURABILITY_POSTGRES", "RUST_WINDOWS", "RUST_SUPPLY_CHAIN"):
        for failure in ("failure", "cancelled", ""):
            assert subprocess.run(
                ["bash", "-c", script], env=dict(docs_env, **{predicate: failure}), check=False
            ).returncode != 0, (predicate, failure)
    assert subprocess.run(
        ["bash", "-c", script],
        env=dict(docs_env, RUST_REQUIRED="false", WINDOWS_REQUIRED="true"),
        check=False,
    ).returncode != 0

    print(
        "Queue credibility regressions PASS: approved blob, protected-base docs classifier, "
        f"4 native failure positions, {mutations} workflow mutations, full fan-in failures, "
        "docs-only skipped-lane success and fail-closed docs negatives"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())