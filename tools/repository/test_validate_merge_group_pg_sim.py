#!/usr/bin/env python3
"""Exercise the activated queue contract and each fan-in failure independently."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import textwrap
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
GATE = ROOT / ".github/workflows/merge-group-gate.yml"
APPROVED = "e3291fe8fca8fcf70166d5652b43d5a26fa0d762"


def main() -> int:
    spec = importlib.util.spec_from_file_location(
        "queue_policy_core", Path(__file__).with_name("validate_repository_policy_core.py")
    )
    assert spec is not None and spec.loader is not None
    core = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(core)
    original = GATE.read_text(encoding="utf-8")
    assert core.git_blob_sha(original.encode()) == APPROVED, "queue PG/SIM activation absent or altered"
    assert core.main() == 0, "approved queue workflow must pass full policy"
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
    early_exit = original.replace("          test -f apps/game-server/tests/durability_postgres.rs", "          exit 0\n          test -f apps/game-server/tests/durability_postgres.rs", 1)
    assert validate(early_exit) != 0
    mutations += 1

    block = core.indented_yaml_mapping_block(original, "game_gate", 2)
    assert block is not None
    script = textwrap.dedent(block.split("        run: |\n", 1)[1])
    predicates = ("CANDIDATE", "DEPENDENCY_REVIEW", "CODEQL", "RUST_LINUX", "DURABILITY_POSTGRES", "RUST_WINDOWS", "RUST_SUPPLY_CHAIN")
    env = dict(os.environ, **dict.fromkeys(predicates, "success"))
    assert subprocess.run(["bash", "-c", script], env=env, check=False).returncode == 0
    for predicate in predicates:
        for failure in ("failure", "skipped", "cancelled", ""):
            assert subprocess.run(["bash", "-c", script], env=dict(env, **{predicate: failure}), check=False).returncode != 0, (predicate, failure)
    print(f"Queue PG/SIM regressions PASS: approved blob, {mutations} mutations, 28 fan-in failures and success control")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
