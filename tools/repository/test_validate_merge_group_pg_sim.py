#!/usr/bin/env python3
"""Exercise the required Merge Queue contract and its credibility controls."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
GATE = ROOT / ".github/workflows/merge-group-gate.yml"
LIFECYCLE = ROOT / "tools/agents/tests/test_governance_lifecycle_discovery.py"
APPROVED = "860a684e5ec71f50ae899f9db36b7c07f9fca623"
LIFECYCLE_COMMAND = "python tools/agents/tests/test_governance_lifecycle_discovery.py"
REGISTERED_POSTGRES_TARGETS = (
    ("durability_postgres", "apps/game-server/tests/durability_postgres.rs"),
    ("character_authority_postgres", "apps/game-server/tests/character_authority_postgres.rs"),
    ("runtime_scope_assignment_postgres", "apps/game-server/tests/runtime_scope_assignment_postgres.rs"),
    ("native_admission_source_postgres", "apps/game-server/tests/native_admission_source_postgres.rs"),
)
NATIVE_POLICY = (
    "$ErrorActionPreference = 'Stop'",
    "$PSNativeCommandUseErrorActionPreference = $true",
)
WINDOWS_REQUIRED_FRAGMENTS = (
    "cargo +1.94.0 build --locked --release -p oteryn-client --target x86_64-pc-windows-msvc",
    "cargo +1.94.0 clippy --locked -p oteryn-client --all-targets --target x86_64-pc-windows-msvc -- -D warnings",
    '$client = ".\\target\\x86_64-pc-windows-msvc\\release\\oteryn-client.exe"',
    "Test-Path -LiteralPath $client -PathType Leaf",
    "& $client --smoke",
    "cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness --target x86_64-pc-windows-msvc",
    "cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc",
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


def _queue_lane_routing_canaries(candidate: str) -> None:
    """Execute the workflow's consumer-routing block, including fallback cases."""
    start = candidate.index("          if result['surface'] == 'full'")
    end = candidate.index("          with open(os.environ['GITHUB_OUTPUT']", start)
    source = textwrap.dedent(candidate[start:end])
    server = dict(
        rust=True, windows=False, surface="server",
        reason="server-only-exact-consumer-closure",
    )
    cases = (
        (server, ("true", "false", "server")),
        (dict(server, surface="durability"), ("true", "false", "durability")),
        (dict(rust=False, windows=False, surface="auxiliary"), ("false", "false", "auxiliary")),
        (dict(rust=False, windows=False, surface="atlas-fullworld"), ("false", "false", "atlas-fullworld")),
        (dict(server, windows=True), ("true", "true", "full")),
        (dict(server, rust=False, windows=True), ("true", "true", "full")),
        (dict(server, reason="unmodelled-input"), ("true", "true", "full")),
        (dict(server, reason="server-only-exact-consumer-closure-plus-atlas-fullworld"), ("true", "true", "full")),
        (dict(server, surface="shared"), ("true", "true", "full")),
        (dict(server, surface="server\nwindows=false"), ("true", "true", "full")),
        (dict(server, rust="true"), ("true", "true", "full")),
        (dict(server, windows="false"), ("true", "true", "full")),
        (dict(server, rust=1), ("true", "true", "full")),
        (dict(server, windows=0), ("true", "true", "full")),
        ({}, ("true", "true", "full")),
        (None, ("true", "true", "full")),
    )
    records = [{"filename": "docs/contracts/server-input.md"}]
    head = "a" * 40
    with tempfile.TemporaryDirectory() as directory:
        metadata_path = Path(directory) / "queue-metadata.json"
        metadata_path.write_text('{"packages": []}', encoding="utf-8")
        for routed, expected in cases:
            classify_calls = []

            def classify(*args, **kwargs):
                classify_calls.append((args, kwargs))
                assert args == (records, 1, {"packages": []})
                assert kwargs == dict(candidate_modes_verified=True, candidate_sha=head)
                return routed

            namespace = dict(
                os=os, json=json, re=re, records=records, head=head,
                module=SimpleNamespace(classify=classify, candidate_modes_safe=lambda sha: sha == head),
                result=dict(rust="true", windows="true", surface="full"),
            )
            with patch.dict(os.environ, RUNNER_TEMP=directory):
                exec(compile(source, "<queue consumer routing>", "exec"), namespace)
            actual = namespace["result"]
            assert tuple(actual[key] for key in ("rust", "windows", "surface")) == expected, (routed, actual)
            assert len(classify_calls) == 1

        # Unreadable/malformed metadata never deselects Windows.
        for metadata in (None, "not json"):
            if metadata is None:
                metadata_path.unlink()
            else:
                metadata_path.write_text(metadata, encoding="utf-8")
            namespace["result"] = dict(rust="true", windows="true", surface="full")
            with patch.dict(os.environ, RUNNER_TEMP=directory):
                exec(compile(source, "<queue consumer routing>", "exec"), namespace)
            assert namespace["result"] == dict(rust="true", windows="true", surface="full")


def _postgres_cache_consumer_regressions() -> None:
    for filename, job in (("rust.yml", "durability-postgres"), ("merge-group-gate.yml", "durability_postgres")):
        text = (ROOT / ".github/workflows" / filename).read_text(encoding="utf-8")
        block = text.split("  " + job + ":\n", 1)[1]
        block = re.split(r"^  [A-Za-z_][A-Za-z_0-9-]*:\n", block, maxsplit=1, flags=re.M)[0]
        assert "actions/cache/restore@55cc8345863c7cc4c66a329aec7e433d2d1c52a9" in block
        assert "actions/cache/save@" not in block
        assert "key: rust-linux-v2-1.94.0-${{ hashFiles('Cargo.lock') }}-${{ github.sha }}" in block
        assert "            rust-linux-v2-1.94.0-${{ hashFiles('Cargo.lock') }}-\n" in block
        assert block.index("Restore trimmed Cargo cache") < block.index("Verify locked metadata")


def _atlas_queue_canaries(original: str, core) -> None:
    """Execute protected routing and prove Atlas cannot be skipped after selection."""
    candidate = core.indented_yaml_mapping_block(original, "candidate", 2)
    start = candidate.index("          atlas_fullworld = 'true'")
    end = candidate.index("          # Use protected-base exact-consumer routing", start)
    source = textwrap.dedent(candidate[start:end])
    spec = importlib.util.spec_from_file_location(
        "atlas_lanes", ROOT / "tools/repository/classify_pr_test_lanes.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    atlas = "tools/game-atlas-fullworld-source/producer.py"
    cases = (
        ([{"filename": atlas, "status": "modified"}], "true"),
        ([{"filename": atlas, "status": "removed"}], "true"),
        ([{"filename": "docs/a.md", "previous_filename": atlas, "status": "renamed"}], "true"),
        ([{"filename": "docs/a.md"}], "false"),
        ([{"filename": "docs/a.md"}, {"filename": atlas}], "true"),
        ([{"filename": "../a"}], "true"),
        ([], "true"),
        (None, "true"),
    )
    for records, expected in cases:
        namespace = {"module": module, "records": records}
        exec(compile(source, "<queue Atlas routing>", "exec"), namespace)
        assert namespace["atlas_fullworld"] == expected, (records, namespace)
    for returned in (None, "false", 0, True):
        namespace = dict(records=[{"filename": "docs/a.md"}], module=SimpleNamespace(
            atlas_fullworld_required=lambda *args: returned,
        ))
        exec(compile(source, "<queue Atlas routing>", "exec"), namespace)
        assert namespace["atlas_fullworld"] == "true", returned
    namespace = dict(records=[{"filename": "docs/a.md"}], module=SimpleNamespace())
    exec(compile(source, "<queue Atlas routing>", "exec"), namespace)
    assert namespace["atlas_fullworld"] == "true"

    job = core.indented_yaml_mapping_block(original, "atlas_fullworld", 2)
    assert "    if: needs.candidate.outputs.atlas_fullworld != 'false'\n" in job
    assert "ref: ${{ github.event.merge_group.head_sha }}" in job
    gate = core.indented_yaml_mapping_block(original, "game_gate", 2)
    assert "codeql, atlas_fullworld, rust_linux" in gate
    assert "ATLAS_FULLWORLD: ${{ needs.atlas_fullworld.result }}" in gate
    script = textwrap.dedent(gate.split("        run: |\n", 1)[1])
    base = dict(os.environ, **dict.fromkeys((
        "CANDIDATE", "DEPENDENCY_REVIEW", "CODEQL", "RUST_LINUX",
        "DURABILITY_POSTGRES", "RUST_WINDOWS", "RUST_SUPPLY_CHAIN",
        "NODE_BOOT", "SERVER_SEAM",
    ), "success"))
    for required in ("true", "", "invalid", "False", "0", "false"):
        for result in ("success", "skipped", "failure", "cancelled", ""):
            env = dict(base, ATLAS_FULLWORLD_REQUIRED=required, ATLAS_FULLWORLD=result)
            actual = subprocess.run(["bash", "-c", script], env=env, capture_output=True).returncode == 0
            expected = result == "success" or (required == "false" and result == "skipped")
            assert actual == expected, (required, result, actual)
    base.pop("ATLAS_FULLWORLD_REQUIRED", None)
    base.pop("ATLAS_FULLWORLD", None)
    assert subprocess.run(["bash", "-c", script], env=base, capture_output=True).returncode != 0
    atlas_only = dict(base, RUST_REQUIRED="false", WINDOWS_REQUIRED="false",
        SERVER_QUALIFICATION_REQUIRED="false", ATLAS_FULLWORLD_REQUIRED="true",
        RUST_LINUX="skipped", DURABILITY_POSTGRES="skipped", RUST_WINDOWS="skipped",
        RUST_SUPPLY_CHAIN="skipped", NODE_BOOT="skipped", SERVER_SEAM="skipped")
    for result in ("success", "skipped"):
        env = dict(atlas_only, ATLAS_FULLWORLD=result)
        actual = subprocess.run(["bash", "-c", script], env=env, capture_output=True).returncode == 0
        assert actual == (result == "success"), ("atlas-only", result)
    print("Atlas MQ PASS: protected routing, exact synthetic checkout and 32 fail-closed fan-in cases")


def main() -> int:
    _postgres_cache_consumer_regressions()
    spec = importlib.util.spec_from_file_location(
        "queue_policy_core", Path(__file__).with_name("validate_repository_policy_core.py")
    )
    assert spec is not None and spec.loader is not None
    core = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(core)
    original = GATE.read_text(encoding="utf-8")
    assert core.git_blob_sha(original.encode()) == APPROVED, "queue protected blob pin drifted"
    assert core.main() == 0, "approved queue workflow must pass full policy"
    _atlas_queue_canaries(original, core)

    candidate = core.indented_yaml_mapping_block(original, "candidate", 2)
    assert candidate is not None
    _queue_lane_routing_canaries(candidate)
    assert LIFECYCLE_COMMAND in candidate, "required MQ candidate does not execute lifecycle discovery"
    for fragment in (
        "      rust: ${{ steps.lanes.outputs.rust }}",
        "      windows: ${{ steps.lanes.outputs.windows }}",
        "      - name: Check out protected-base classifier input",
        "          ref: ${{ github.event.merge_group.base_sha }}",
        "      - name: Classify trusted merge-group lanes",
        "path.startswith('docs/architecture/') and path.endswith('.md')",
        "result = {'rust': 'false', 'windows': 'false', 'surface': 'architecture-docs'}",
        "cargo +1.94.0 metadata --locked --no-deps --all-features --format-version 1 > \"$RUNNER_TEMP/queue-metadata.json\"",
        "if result['surface'] == 'full' and isinstance(records, list) and records:",
        "candidate_modes_verified=module.candidate_modes_safe(head),",
        "if routed.get('rust') is False and routed.get('windows') is False:",
        "                  result = {'rust': 'true', 'windows': 'true', 'surface': 'full'}",
    ):
        assert fragment in candidate, f"Merge Queue trusted docs classifier missing: {fragment}"

    for job, condition in (
        ("rust_linux", "if: needs.candidate.outputs.rust == 'true'"),
        ("durability_postgres", "if: needs.candidate.outputs.rust == 'true'"),
        ("rust_windows", "if: needs.candidate.outputs.windows == 'true'"),
        ("rust_supply_chain", "if: needs.candidate.outputs.rust == 'true'"),
        ("node_boot", "if: needs.candidate.outputs.server_qualification != 'false'"),
        ("server_seam", "if: needs.candidate.outputs.server_qualification != 'false'"),
    ):
        block = core.indented_yaml_mapping_block(original, job, 2)
        assert block is not None and condition in block, (job, condition)

    postgres = core.indented_yaml_mapping_block(original, "durability_postgres", 2)
    assert postgres is not None
    for fragment in (
        "          BASE_SHA: ${{ github.event.merge_group.base_sha }}",
        "          HEAD_SHA: ${{ github.event.merge_group.head_sha }}",
        '            if git cat-file -e "$BASE_SHA:$path" 2>/dev/null; then',
        '            if git cat-file -e "$HEAD_SHA:$path" 2>/dev/null; then',
        '            if [[ "$base_present" == true && "$head_present" == false ]]; then',
        "          verify_registered_target_binding() {",
        '            cargo +1.94.0 metadata --locked --no-deps --format-version 1 > "$metadata"',
        "              owners = [package for package in packages if package.get('name') == 'oteryn-game-server']",
        "              expected_manifest = (pathlib.Path.cwd() / 'apps/game-server/Cargo.toml').resolve(strict=True)",
        "              observed_manifest = pathlib.Path(owners[0]['manifest_path']).resolve(strict=True)",
        "              if observed_manifest != expected_manifest:",
        "              matches = [target for target in targets if target.get('name') == name]",
        "              if len(matches) != 1 or matches[0].get('kind') != ['test']:",
        "              expected = (pathlib.Path.cwd() / registered_path).resolve(strict=True)",
        "              observed = pathlib.Path(matches[0]['src_path']).resolve(strict=True)",
        '              verify_registered_target_binding "$name" "$path"',
        '              cargo +1.94.0 test --locked --workspace --test "$name"',
    ):
        assert fragment in postgres, f"Merge Queue PostgreSQL routing missing: {fragment}"
    for name, target in REGISTERED_POSTGRES_TARGETS:
        marker = f"          run_registered_target {name} {target}"
        assert postgres.count(marker) == 1, marker
    for forbidden in ("glob(", "rglob(", "fnmatch", "TARGETS_JSON", "fromJSON(", "postgres-target-manifest"):
        assert forbidden not in postgres, f"Merge Queue PostgreSQL routing is PR/data controlled: {forbidden}"

    marker = '            python - "$metadata" "$name" "$path" <<\'PY\'\n'
    verifier = textwrap.dedent(postgres.split(marker, 1)[1].split("          PY\n", 1)[0])
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        target = REGISTERED_POSTGRES_TARGETS[0]
        expected = root / target[1]
        expected.parent.mkdir(parents=True)
        expected.write_text("// registered\n", encoding="utf-8")
        remapped = expected.with_name("remapped.rs")
        remapped.write_text("// remapped\n", encoding="utf-8")
        expected_manifest = root / "apps/game-server/Cargo.toml"
        expected_manifest.write_text("[package]\nname = \"fixture\"\n", encoding="utf-8")
        wrong_manifest = root / "other/Cargo.toml"
        wrong_manifest.parent.mkdir(parents=True)
        wrong_manifest.write_text("[package]\nname = \"wrong\"\n", encoding="utf-8")
        metadata = root / "metadata.json"

        def package(manifest_path: object = expected_manifest, src_path: Path = expected):
            return {
                "name": "oteryn-game-server",
                "manifest_path": str(manifest_path) if isinstance(manifest_path, Path) else manifest_path,
                "targets": [{"name": target[0], "kind": ["test"], "src_path": str(src_path)}],
            }

        def verify(packages: list[dict[str, object]]) -> subprocess.CompletedProcess[str]:
            metadata.write_text(json.dumps({"packages": packages}), encoding="utf-8")
            return subprocess.run(
                [sys.executable, "-c", verifier, str(metadata), target[0], target[1]],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )

        assert verify([package()]).returncode == 0
        shadow = dict(package(), name="other-workspace-package")
        assert verify([package(), shadow]).returncode != 0, "workspace target shadow must fail"
        rejected = verify([package(wrong_manifest)])
        assert rejected.returncode != 0 and "package manifest is" in rejected.stderr, rejected
        for packages in (
            [{key: value for key, value in package().items() if key != "manifest_path"}],
            [package(True)],
            [package(), package()],
        ):
            assert verify(packages).returncode != 0, packages
        rejected = verify([package(src_path=remapped)])
        assert rejected.returncode != 0 and "target source is" in rejected.stderr, rejected

    windows = core.indented_yaml_mapping_block(original, "rust_windows", 2)
    assert windows is not None
    for policy in NATIVE_POLICY:
        assert policy in windows, f"Merge Queue Windows block lacks fail-closed policy: {policy}"
    for fragment in WINDOWS_REQUIRED_FRAGMENTS:
        assert windows.count(fragment) == 1, f"required Windows fragment missing or duplicated: {fragment}"
    assert "cargo +1.94.0 run --locked -p oteryn-client --target x86_64-pc-windows-msvc -- --smoke" not in windows

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
        '              cargo +1.94.0 test --locked --workspace --test "$name"',
        "        run: cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc",
        "        run: cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc",
    ):
        assert command in original
        for replacement in (
            command.replace("cargo", "echo cargo", 1),
            (command.replace("oteryn-", "mutated-", 1) if "oteryn-" in command else command.replace("--workspace", "-p oteryn-game-server", 1)),
        ):
            assert replacement != command
            assert validate(original.replace(command, replacement, 1)) != 0
            mutations += 1

    for name, target in REGISTERED_POSTGRES_TARGETS:
        marker = f"          run_registered_target {name} {target}"
        assert original.count(marker) == 1
        assert validate(original.replace(marker, "", 1)) != 0
        mutations += 1

    deletion_guard = '            if [[ "$base_present" == true && "$head_present" == false ]]; then'
    assert original.count(deletion_guard) == 1
    assert validate(original.replace(deletion_guard, '            if [[ "$base_present" == false && "$head_present" == false ]]; then', 1)) != 0
    mutations += 1

    early_exit = original.replace(
        "          run_registered_target() {",
        "          exit 0\n          run_registered_target() {",
        1,
    )
    assert early_exit != original and validate(early_exit) != 0
    mutations += 1

    binding_fragments = (
        '            cargo +1.94.0 metadata --locked --no-deps --format-version 1 > "$metadata"',
        "              expected_manifest = (pathlib.Path.cwd() / 'apps/game-server/Cargo.toml').resolve(strict=True)",
        "              observed_manifest = pathlib.Path(owners[0]['manifest_path']).resolve(strict=True)",
        "              if observed_manifest != expected_manifest:",
        "              if len(matches) != 1 or matches[0].get('kind') != ['test']:",
        "              observed = pathlib.Path(matches[0]['src_path']).resolve(strict=True)",
        '              verify_registered_target_binding "$name" "$path"',
    )
    for fragment in binding_fragments:
        assert original.count(fragment) == 1, fragment
        assert validate(original.replace(fragment, "", 1)) != 0, fragment
        mutations += 1

    for fragment in (LIFECYCLE_COMMAND, *NATIVE_POLICY):
        assert original.count(fragment) == 1
        assert validate(original.replace(fragment, "", 1)) != 0, fragment
        mutations += 1

    block = core.indented_yaml_mapping_block(original, "game_gate", 2)
    assert block is not None
    script = textwrap.dedent(block.split("        run: |\n", 1)[1])
    predicates = (
        "CANDIDATE", "DEPENDENCY_REVIEW", "CODEQL", "ATLAS_FULLWORLD", "RUST_LINUX",
        "DURABILITY_POSTGRES", "RUST_WINDOWS", "RUST_SUPPLY_CHAIN",
        "NODE_BOOT", "SERVER_SEAM",
    )
    env = dict(os.environ, **dict.fromkeys(predicates, "success"))
    env.pop("SERVER_QUALIFICATION_REQUIRED", None)
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
        SERVER_QUALIFICATION_REQUIRED="false",
        NODE_BOOT="skipped",
        SERVER_SEAM="skipped",
        ATLAS_FULLWORLD_REQUIRED="false",
        ATLAS_FULLWORLD="skipped",
    )
    assert subprocess.run(["bash", "-c", script], env=docs_env, check=False).returncode == 0
    server_env = dict(env, RUST_REQUIRED="true", WINDOWS_REQUIRED="false", RUST_WINDOWS="skipped")
    assert subprocess.run(["bash", "-c", script], env=server_env, check=False).returncode == 0
    for predicate in predicates:
        failures = ("failure", "cancelled", "") if predicate == "RUST_WINDOWS" else ("failure", "skipped", "cancelled", "")
        for failure in failures:
            assert subprocess.run(
                ["bash", "-c", script], env=dict(server_env, **{predicate: failure}), check=False
            ).returncode != 0, ("server-only", predicate, failure)
    # Missing or malformed flags cannot turn a required lane into an optional one.
    for flag in ("RUST_REQUIRED", "WINDOWS_REQUIRED"):
        for value in ("", "invalid", "False", "0"):
            assert subprocess.run(
                ["bash", "-c", script], env=dict(server_env, **{flag: value}), check=False
            ).returncode != 0, (flag, value)
    # Physical qualifications fail closed unless explicitly deselected.
    for predicate in ("NODE_BOOT", "SERVER_SEAM"):
        for required in ("true", ""):
            assert subprocess.run(
                ["bash", "-c", script],
                env=dict(docs_env, SERVER_QUALIFICATION_REQUIRED=required),
                check=False,
            ).returncode != 0, (predicate, required)
        for failure in ("failure", "cancelled", ""):
            assert subprocess.run(
                ["bash", "-c", script], env=dict(docs_env, **{predicate: failure}), check=False
            ).returncode != 0, (predicate, failure)
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
        "Queue credibility regressions PASS: approved blob, protected-base exact-consumer routing, "
        f"4 native failure positions, {mutations} workflow mutations, full fan-in failures, "
        "docs-only skipped-lane success and fail-closed docs negatives"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
