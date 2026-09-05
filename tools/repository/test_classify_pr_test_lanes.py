#!/usr/bin/env python3
"""Behavioral fixtures for trusted-base risk classification and gate fan-in."""
from __future__ import annotations

import copy
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import textwrap
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
MODULE = Path(__file__).with_name("classify_pr_test_lanes.py")


def test_aggregate():
    gate = (ROOT / ".github/workflows/merge-gate.yml").read_text()
    block = gate.split("  validate:\n", 1)[1].split("  game_gate:\n", 1)[0]
    script = textwrap.dedent(block.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    mandatory = ("SCOPE", "LANES", "GOVERNANCE", "DEPENDENCY_REVIEW", "CODEQL")
    rust = ("RUST_POLICY", "RUST_LINUX", "RUST_SUPPLY_CHAIN")
    env = dict.fromkeys(mandatory + rust + ("RUST_WINDOWS",), "success")
    env.update(RUST_REQUIRED="true", WINDOWS_REQUIRED="true")

    def accepts(changes):
        with patch.dict(os.environ, dict(env, **changes)), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            try:
                exec(compile(script, "merge-gate:aggregate", "exec"), {})
                return True
            except SystemExit:
                return False

    assert accepts({})
    assert accepts({"WINDOWS_REQUIRED": "false", "RUST_WINDOWS": "skipped"}), "proven server-only lane cannot omit Windows"
    assert accepts(dict.fromkeys(rust + ("RUST_WINDOWS",), "skipped") | {"RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "false"})
    for name in mandatory + rust + ("RUST_WINDOWS",):
        for value in ("failure", "cancelled", "skipped", ""):
            assert not accepts({name: value}), (name, value)
    for name in ("RUST_REQUIRED", "WINDOWS_REQUIRED"):
        for value in ("", "TRUE", "unknown", "0"):
            assert not accepts({name: value}), (name, value)
    assert not accepts({"RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "true"})
    print("Risk aggregate PASS: full/server/docs controls, every selected failure and invalid output")


def fixture():
    roots = {
        "oteryn-game-server": "apps/game-server",
        "oteryn-client": "apps/client",
        "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
        "oteryn-simulation-determinism": "crates/simulation-determinism",
        "oteryn-foundation": "crates/foundation",
    }
    edges = {
        "oteryn-game-server": ["oteryn-foundation", "oteryn-simulation-determinism"],
        "oteryn-client": ["oteryn-foundation"],
        "oteryn-synthetic-client-harness": ["oteryn-foundation"],
    }
    return {
        "workspace_root": "/repo",
        "workspace_members": list(roots),
        "packages": [
            {"id": name, "name": name, "manifest_path": f"/repo/{path}/Cargo.toml",
             "dependencies": [{"name": dep, "path": f"/repo/{roots[dep]}", "kind": None, "target": None, "optional": False} for dep in edges.get(name, [])]}
            for name, path in roots.items()
        ],
    }


def main() -> int:
    assert MODULE.is_file(), "dependency-aware trusted-base classifier is not implemented"
    spec = importlib.util.spec_from_file_location("risk_classifier", MODULE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    def classify(paths, metadata=None, digest=None, **kwargs):
        files = [dict(filename=p, status="modified") if isinstance(p, str) else p for p in paths]
        return module.classify(files, kwargs.pop("count", len(files)), fixture() if metadata is None else metadata,
                               module.AUDITED_INPUT_SHA256 if digest is None else digest, **kwargs)

    server = "apps/game-server/src/lib.rs"
    result = classify([server])
    assert result["rust"] is True and result["windows"] is False, result
    assert result["surface"] == "server", result
    for path in ("apps/game-server/src/durability/mod.rs", "apps/game-server/migrations/0001.sql",
                 "apps/game-server/tests/support/postgres.rs", "apps/game-server/src/foundation/reconnect.rs"):
        result = classify([path])
        assert result["rust"] is True and result["windows"] is False, (path, result)

    full_paths = (
        "apps/client/src/lib.rs", "crates/foundation/src/lib.rs",
        "crates/simulation-determinism/src/lib.rs", "crates/simulation-determinism/fixtures/golden.json",
        "Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "apps/game-server/build.rs",
        "apps/game-server/Cargo.toml", ".github/workflows/rust.yml", ".github/actions/custom/action.yml",
        "tools/repository/classify_pr_test_lanes.py", "AGENTS.md", "docs/agents/AGENTS.md",
        "docs/migration/input.json", "unknown/input.dat", "apps/game-server/unknown.md",
    )
    for path in full_paths:
        result = classify([path])
        assert result["rust"] is True and result["windows"] is True, (path, result)
    for paths in ([server, "apps/client/src/lib.rs"], [server, "unknown/input.dat"],
                  [{"filename": server, "status": "renamed", "previous_filename": "apps/client/src/old.rs"}],
                  [{"filename": "docs/new.md", "status": "renamed", "previous_filename": server}]):
        result = classify(paths)
        assert result["rust"] and result["windows"], result
    for paths in (["README.md"], ["docs/architecture/example.md"], ["docs/agents/tasks/active/task.md"]):
        result = classify(paths)
        assert result["rust"] is False and result["windows"] is False, result
    result = classify([server, "docs/agents/tasks/active/task.md"])
    assert result["rust"] and not result["windows"], result

    invalid = (
        ([], {}), ([server], {"count": 2}), ([server], {"complete": False}),
        ([server], {"complete": "true"}), ([server, server], {}),
        ([{"filename": server, "status": "renamed"}], {}),
        ([{"filename": "../apps/game-server/lib.rs", "status": "modified"}], {}),
        ([{"filename": server, "status": "unknown"}], {}),
    )
    for paths, kwargs in invalid:
        result = classify(paths, **kwargs)
        assert result["rust"] and result["windows"], (paths, kwargs, result)
    for metadata in ({}, {"packages": []}, {"workspace_root": "/repo"}):
        result = classify([server], metadata=metadata)
        assert result["rust"] and result["windows"], result
    # A later accepted cross-package include, symlink, build input or dependency edit
    # changes the protected-base input snapshot even without a Cargo edge change.
    for digest in ("", "0" * 64, "cross-package-include", "symlink"):
        result = classify([server], digest=digest)
        assert result["rust"] and result["windows"], result
    for kind, target, optional in ((None, None, False), ("dev", None, False),
                                   ("build", None, False), (None, "cfg(windows)", True)):
        metadata = fixture()
        metadata["packages"][1]["dependencies"].append({"name": "oteryn-game-server", "path": "/repo/apps/game-server",
                                                       "kind": kind, "target": target, "optional": optional})
        result = classify([server], metadata=metadata)
        assert result["rust"] and result["windows"], (kind, target, result)
    for change in (
        lambda m: m["packages"].pop(),
        lambda m: m["packages"].append(copy.deepcopy(m["packages"][0])),
        lambda m: m["packages"][0]["dependencies"].append({"path": "/outside", "name": "unknown"}),
    ):
        metadata = fixture()
        change(metadata)
        result = classify([server], metadata=metadata)
        assert result["rust"] and result["windows"], result
    print("Risk classifier fixtures PASS: surfaces, transitive dependency kinds, protected inputs and fail-closed enumeration")
    test_aggregate()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
