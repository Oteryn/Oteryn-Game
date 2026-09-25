#!/usr/bin/env python3
"""Regression tests for canonical PR PostgreSQL/SIM evidence-step validation."""
from __future__ import annotations

import copy
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import urllib.error
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = Path(__file__).with_name("validate_pr_gate_pg_sim.py")
MERGE_GATE = ROOT / ".github/workflows/merge-gate.yml"
TARGET = "apps/game-server/tests/durability_postgres.rs"
BLOB_SHA = "1" * 40
REGISTERED_POSTGRES_TARGETS = {
    "durability_postgres": "apps/game-server/tests/durability_postgres.rs",
    "character_authority_postgres": "apps/game-server/tests/character_authority_postgres.rs",
    "runtime_scope_assignment_postgres": "apps/game-server/tests/runtime_scope_assignment_postgres.rs",
    "native_admission_source_postgres": "apps/game-server/tests/native_admission_source_postgres.rs",
}


def classification_output(durability_blob: str | None) -> str:
    lines = []
    for name in REGISTERED_POSTGRES_TARGETS:
        blob = durability_blob if name == "durability_postgres" else None
        lines.append(f"{name}_present={'true' if blob is not None else 'false'}")
        lines.append(f"{name}_blob={blob or ''}")
    return "\n".join(lines) + "\n"

# Self-contained snapshot of the protected classifier's >300-file rejection path.
PROTECTED_PG_CLASSIFIER_FIXTURE = """\
  rust_linux:
    steps:
      - name: Classify Durability PostgreSQL target
        run: |
          python - <<'PY'
          import json
          import os
          import urllib.request

          repository = os.environ['REPOSITORY']
          number_text = os.environ['PULL_NUMBER'].strip()
          headers = {'Authorization': f"Bearer {os.environ['GH_TOKEN']}"}

          def api(path: str):
              request = urllib.request.Request(
                  f'https://api.github.com/repos/{repository}{path}', headers=headers
              )
              with urllib.request.urlopen(request, timeout=30) as response:
                  return json.load(response)

          pull = api(f'/pulls/{number_text}')
          changed_files = pull.get('changed_files')
          if (
              isinstance(changed_files, bool)
              or not isinstance(changed_files, int)
              or changed_files < 0
              or changed_files > 300
          ):
              raise SystemExit('invalid or over-cap changed-files count')
          present = False
          for page in range(1, (changed_files + 99) // 100 + 1):
              for item in api(f'/pulls/{number_text}/files?per_page=100&page={page}'):
                  present |= item.get('filename') == 'apps/game-server/tests/durability_postgres.rs'
          with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
              output.write(f"present={'true' if present else 'false'}\\n")
          PY
"""


def load_validator():
    spec = importlib.util.spec_from_file_location("validate_pr_gate_pg_sim", VALIDATOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load validator: {VALIDATOR_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_mutated_gate(text: str) -> list[str]:
    module = load_validator()
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "merge-gate.yml"
        path.write_text(text, encoding="utf-8")
        module.MERGE_GATE = path
        return module.validate()


def inject_step_condition(text: str, step_name: str) -> str:
    marker = f"      - name: {step_name}\n"
    if text.count(marker) != 1:
        raise AssertionError(f"expected exactly one step named {step_name!r}")
    return text.replace(marker, marker + "        if: false\n", 1)


def missing_target(message="Not Found", code=404):
    return urllib.error.HTTPError(
        "https://api.github.test/exact",
        code,
        "error",
        {},
        io.BytesIO(json.dumps({"message": message}).encode("utf-8")),
    )


def test_registered_postgres_targets_are_materially_routed() -> None:
    workflows = (
        MERGE_GATE,
        ROOT / ".github/workflows/merge-group-gate.yml",
        ROOT / ".github/workflows/rust.yml",
    )
    for workflow in workflows:
        text = workflow.read_text(encoding="utf-8")
        for target, target_path in REGISTERED_POSTGRES_TARGETS.items():
            assert target in text, f"{workflow.name} does not route registered target {target}"
            assert target_path in text, f"{workflow.name} does not bind registered path {target_path}"


def test_pr_gate_rejects_explicit_cargo_test_path_remap() -> None:
    workflow = MERGE_GATE.read_text(encoding="utf-8")
    step = load_validator().step_block(
        load_validator().job_block(workflow, "rust_linux"),
        "Run registered PostgreSQL E2E targets when allocated",
    )
    assert step is not None
    marker = '            python - "$metadata" "$name" "$path" <<\'PY\'\n'
    verifier = textwrap.dedent(step.split(marker, 1)[1].split("          PY\n", 1)[0])
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        expected = root / TARGET
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
                "targets": [{
                    "name": "durability_postgres", "kind": ["test"], "src_path": str(src_path)
                }],
            }

        def verify(packages: list[dict[str, object]]) -> subprocess.CompletedProcess[str]:
            metadata.write_text(json.dumps({"packages": packages}), encoding="utf-8")
            return subprocess.run(
                [sys.executable, "-c", verifier, str(metadata), "durability_postgres", TARGET],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )

        assert verify([package()]).returncode == 0
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


def test_postgres_evidence_step_cannot_be_skipped() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    mutated = inject_step_condition(baseline, "Run registered PostgreSQL E2E targets when allocated")
    errors = validate_mutated_gate(mutated)
    assert errors, "validator accepted a skipped PostgreSQL evidence step"
    assert any("rust_linux" in error and "if" in error for error in errors), errors


def test_simulation_evidence_step_cannot_be_skipped() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    mutated = inject_step_condition(baseline, "Verify deterministic simulation golden fixtures")
    errors = validate_mutated_gate(mutated)
    assert errors, "validator accepted a skipped simulation evidence step"
    assert any("rust_windows" in error and "if" in error for error in errors), errors


def test_input_platform_evidence_contract_is_mandatory() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    marker = "      - name: Test Windows input platform\n"
    command = "        run: cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc\n"
    assert baseline.count(marker) == baseline.count(command) == 1
    mutations = (
        baseline.replace(marker, "      - name: Optional Windows input platform\n", 1),
        baseline.replace(marker, marker + "        if: false\n", 1),
        baseline.replace(command, command.replace("oteryn-input-platform", "oteryn-client"), 1),
        baseline.replace(marker + "        shell: pwsh\n" + command, "", 1),
    )
    for mutated in mutations:
        errors = validate_mutated_gate(mutated)
        assert any("rust_windows" in error and "input platform" in error for error in errors), errors


def run_classifier(
    files,
    initial_change=None,
    final_change=None,
    expected_base="b" * 40,
    scope=False,
    immutable_files=None,
    tree_payloads=None,
    checkout_present=True,
    checkout_files=None,
    checkout_blob=BLOB_SHA,
    commit_payloads=None,
    workflow_text=None,
):
    """Execute the real workflow script with only GitHub HTTP responses replaced."""
    validator = load_validator()
    source_text = workflow_text or MERGE_GATE.read_text(encoding="utf-8")
    classifier_step = (
        "Classify registered PostgreSQL targets"
        if "Classify registered PostgreSQL targets" in source_text
        else "Classify Durability PostgreSQL target"
    )
    step = validator.step_block(
        validator.job_block(
            source_text,
            "scope" if scope else "rust_linux",
        ),
        "Resolve and validate exact pull request head" if scope else classifier_step,
    )
    assert step is not None
    script = textwrap.dedent(step.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    initial = {
        "state": "open",
        "head": {"sha": "a" * 40, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
        "base": {"sha": "b" * 40, "ref": "main"},
        "changed_files": len(files),
    }
    final = copy.deepcopy(initial)
    if initial_change:
        initial_change(initial)
    if final_change:
        final_change(final)
    pulls = iter((initial, final))
    base_tree_sha = "2" * 40
    head_tree_sha = "3" * 40
    commit_payloads = commit_payloads or {
        "b" * 40: {"sha": "b" * 40, "tree": {"sha": base_tree_sha}},
        "a" * 40: {"sha": "a" * 40, "tree": {"sha": head_tree_sha}},
    }
    present_entry = {"path": TARGET, "mode": "100644", "type": "blob", "sha": BLOB_SHA}
    tree_payloads = tree_payloads or {
        base_tree_sha: {"sha": base_tree_sha, "truncated": False, "tree": [present_entry]},
        head_tree_sha: {"sha": head_tree_sha, "truncated": False, "tree": [present_entry]},
    }

    def response(payload):
        if isinstance(payload, BaseException):
            raise payload
        return io.StringIO(json.dumps(payload))

    def urlopen(request, timeout):
        assert timeout == 30
        prefix = "https://api.github.com/repos/Oteryn/Oteryn-Game/pulls/287"
        if request.full_url == prefix:
            return response(next(pulls))
        commits = "https://api.github.com/repos/Oteryn/Oteryn-Game/git/commits/"
        if request.full_url.startswith(commits):
            return response(commit_payloads[request.full_url.removeprefix(commits)])
        trees = "https://api.github.com/repos/Oteryn/Oteryn-Game/git/trees/"
        if request.full_url.startswith(trees):
            tree_sha = request.full_url.removeprefix(trees).removesuffix("?recursive=1")
            return response(tree_payloads[tree_sha])
        comparison = f"https://api.github.com/repos/Oteryn/Oteryn-Game/compare/{'b' * 40}...{'a' * 40}?per_page=1"
        if request.full_url == comparison:
            return io.StringIO(json.dumps({"files": files if immutable_files is None else immutable_files}))
        page_prefix = prefix + "/files?per_page=100&page="
        assert request.full_url.startswith(page_prefix), request.full_url
        page = int(request.full_url.removeprefix(page_prefix))
        return io.StringIO(json.dumps(files[(page - 1) * 100:page * 100]))

    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "output"
        env = {
            "REPOSITORY": "Oteryn/Oteryn-Game", "PULL_NUMBER": "287",
            "EXPECTED_HEAD": "a" * 40, "EXPECTED_BASE": expected_base,
            "EVENT_PR_NUMBER": "287", "EVENT_PR_HEAD_SHA": "a" * 40,
            "GH_TOKEN": "test-only", "GITHUB_OUTPUT": str(output),
        }
        failure = None
        completed = subprocess.CompletedProcess([], 0, stdout=f"{checkout_blob}\n")
        def isfile(path):
            if checkout_files is not None:
                return path in checkout_files
            return path == TARGET and checkout_present

        with patch.dict(os.environ, env), patch("urllib.request.urlopen", urlopen), patch(
            "os.path.isfile", side_effect=isfile
        ), patch(
            "subprocess.run", return_value=completed
        ), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            try:
                exec(compile(script, "merge-gate.yml:pg_target", "exec"), {})
            except SystemExit as error:
                failure = str(error)
        return failure, output.read_text(encoding="utf-8") if output.exists() else ""


def test_classifiers_bind_aba_to_immutable_authority() -> None:
    mutable = [{"filename": "docs/new.md", "status": "modified"}]
    immutable = [{"filename": "apps/game-server/tests/durability_postgres.rs", "status": "removed"}]
    failure, output = run_classifier(mutable, immutable_files=immutable, scope=True)
    assert failure is None
    assert json.loads(dict(line.split("=", 1) for line in output.splitlines())["file_records"]) == immutable
    failure, output = run_classifier(mutable)
    assert failure is None and output == classification_output(BLOB_SHA), (failure, output)


def test_scope_rejects_comparison_truncation_while_pg_accepts_large_diff() -> None:
    files = [{"filename": f"docs/{i}.md"} for i in range(301)]
    failure, output = run_classifier(files, immutable_files=files[:300], scope=True)
    assert failure is None and output.endswith("complete=false\n")
    failure, output = run_classifier([{"filename": f"docs/{i}.md"} for i in range(803)])
    assert failure is None and output == classification_output(BLOB_SHA), (failure, output)


def test_protected_classifier_red_for_valid_803_file_target() -> None:
    files = [{"filename": f"docs/{i}.md"} for i in range(802)] + [
        {"filename": "apps/game-server/tests/durability_postgres.rs", "status": "modified"}
    ]
    failure, output = run_classifier(files, workflow_text=PROTECTED_PG_CLASSIFIER_FIXTURE)
    assert failure == "invalid or over-cap changed-files count" and not output, (failure, output)


def test_scope_rejects_identity_races() -> None:
    # A Rust event head can otherwise consume a replacement docs-only listing of the same size.
    files = [{"filename": f"docs/file-{index}.md"} for index in range(101)]
    failure, output = run_classifier(files, scope=True)
    assert failure is None and output.endswith("complete=true\n"), (failure, output)
    mutations = {
        "closed": lambda pull: pull.update(state="closed"),
        "head": lambda pull: pull["head"].update(sha="c" * 40),
        "repository": lambda pull: pull["head"]["repo"].update(full_name="other/repository"),
        "base": lambda pull: pull["base"].update(sha="c" * 40),
        "base_ref": lambda pull: pull["base"].update(ref="other"),
        "count": lambda pull: pull.update(changed_files=102),
    }
    accepted = []
    for name, change in mutations.items():
        failure, output = run_classifier(files, final_change=change, scope=True)
        if failure is None or output:
            accepted.append(name)
    assert not accepted, f"scope emitted authority after PR identity changed: {accepted}"


def test_scope_preserves_stable_classification() -> None:
    for files, rust in (
        ([], False),
        ([{"filename": "README.md"}], False),
        ([{"filename": "apps/game-server/src/lib.rs"}], True),
        ([{"filename": "docs/old.md", "previous_filename": "crates/old.rs"}], True),
    ):
        failure, output = run_classifier(files, scope=True)
        expected = f"pr_number=287\ntarget_sha={'a' * 40}\nbase_sha={'b' * 40}\nfile_count={len(files)}\nfile_records={json.dumps(files, separators=(',', ':'))}\ncomplete=true\n"
        assert failure is None and output == expected, (failure, output)


def test_scope_bounds_environment_transport() -> None:
    files = [{"filename": "apps/game-server/src/lib.rs", "status": "modified", "patch": "x" * 140000}]
    failure, output = run_classifier(files, scope=True)
    fields = dict(line.split("=", 1) for line in output.splitlines())
    assert failure is None and fields["complete"] == "true"
    assert json.loads(fields["file_records"]) == [{"filename": files[0]["filename"], "status": "modified"}], "scope must transport classification fields only"
    # Even projected paths must not exceed an environment-variable launch limit.
    files = [{"filename": f"docs/{i}/" + "x" * 200 + ".md", "status": "modified"} for i in range(300)]
    failure, output = run_classifier(files, scope=True)
    fields = dict(line.split("=", 1) for line in output.splitlines())
    assert failure is None and fields["complete"] == "false" and fields["file_records"] == "[]", "oversize scope must select FULL before process launch"


def test_classifier_rejects_identity_races() -> None:
    # Two pages and an unchanged file count reproduce the review's race, not a count mismatch.
    files = [{"filename": f"docs/file-{index}.md", "status": "modified"} for index in range(101)]
    failure, output = run_classifier(files)
    assert failure is None and output == classification_output(BLOB_SHA), (failure, output)
    mutations = {
        "closed": lambda pull: pull.update(state="closed"),
        "head": lambda pull: pull["head"].update(sha="c" * 40),
        "repository": lambda pull: pull["head"]["repo"].update(full_name="other/repository"),
        "base": lambda pull: pull["base"].update(sha="c" * 40),
        "count": lambda pull: pull.update(changed_files=102),
    }
    accepted = []
    for name, change in mutations.items():
        failure, output = run_classifier(files, final_change=change)
        if failure is None or output:
            accepted.append(name)
    assert not accepted, f"classifier emitted a result after PR identity changed: {accepted}"


def test_classifier_rejects_unbound_base() -> None:
    cases = (
        {"expected_base": ""},
        {"expected_base": "not-a-sha"},
        {"initial_change": lambda pull: pull["base"].update(sha="c" * 40)},
    )
    for case in cases:
        failure, output = run_classifier([], **case)
        assert failure is not None and not output, "classifier accepted an invalid or changed base"


def test_classifier_exact_target_state_matrix() -> None:
    present = {"path": TARGET, "mode": "100644", "type": "blob", "sha": BLOB_SHA}
    executable = {"path": TARGET, "mode": "100755", "type": "blob", "sha": BLOB_SHA}
    absent = {"path": "docs/readme.md", "mode": "100644", "type": "blob", "sha": "4" * 40}
    cases = (
        ("large-present", present, present, True, None, classification_output(BLOB_SHA)),
        ("executable-present", executable, executable, True, None, classification_output(BLOB_SHA)),
        ("introduced", absent, present, True, None, classification_output(BLOB_SHA)),
        ("both-absent", absent, absent, False, None, classification_output(None)),
        ("removed", present, absent, False, "removed or renamed", ""),
        ("renamed-away", present, absent, False, "removed or renamed", ""),
    )
    for name, base, head, checkout, expected_failure, expected_output in cases:
        failure, output = run_classifier(
            [{"filename": TARGET if name != "renamed-away" else "tests/renamed.rs"}] * 803,
            tree_payloads={
                "2" * 40: {"sha": "2" * 40, "truncated": False, "tree": [base]},
                "3" * 40: {"sha": "3" * 40, "truncated": False, "tree": [head]},
            },
            checkout_present=checkout,
        )
        assert (expected_failure is None and failure is None) or expected_failure in failure, (name, failure)
        assert output == expected_output, (name, output)


def test_classifier_fails_closed_for_each_protected_registered_target_removal() -> None:
    elsewhere = {"path": "docs/readme.md", "mode": "100644", "type": "blob", "sha": "4" * 40}
    for name, target in REGISTERED_POSTGRES_TARGETS.items():
        present = {"path": target, "mode": "100644", "type": "blob", "sha": BLOB_SHA}
        failure, output = run_classifier(
            [],
            tree_payloads={
                "2" * 40: {"sha": "2" * 40, "truncated": False, "tree": [present]},
                "3" * 40: {"sha": "3" * 40, "truncated": False, "tree": [elsewhere]},
            },
            checkout_files=set(),
        )
        assert f"{name} PostgreSQL target was removed or renamed" in failure and not output, (name, failure, output)


def test_classifier_rejects_bad_target_evidence_and_checkout_mismatch() -> None:
    present = {"path": TARGET, "mode": "100644", "type": "blob", "sha": BLOB_SHA}
    bad = (
        [],
        {"sha": "3" * 40, "truncated": True, "tree": []},
        {"sha": "4" * 40, "truncated": False, "tree": []},
        {"sha": "3" * 40, "truncated": False, "tree": "invalid"},
        {"sha": "3" * 40, "truncated": False, "tree": [{"path": TARGET, "type": "tree", "mode": "040000", "sha": BLOB_SHA}]},
        {"sha": "3" * 40, "truncated": False, "tree": [{"path": TARGET, "type": "blob", "mode": "100644"}]},
        {"sha": "3" * 40, "truncated": False, "tree": [{"path": TARGET, "type": "blob", "mode": "100644", "sha": "bad"}]},
        missing_target("forbidden"),
        missing_target(),
        missing_target(code=401),
        missing_target(code=403),
        missing_target(code=429),
        missing_target(code=500),
        urllib.error.URLError("transport"),
        ValueError("malformed JSON"),
    )
    for payload in bad:
        failure, output = run_classifier(
            [], tree_payloads={
                "2" * 40: {"sha": "2" * 40, "truncated": False, "tree": [present]},
                "3" * 40: payload,
            }
        )
        assert failure is not None and not output, payload
    for api_present, checkout_present in ((True, False), (False, True)):
        payload = [present] if api_present else []
        failure, output = run_classifier(
            [],
            tree_payloads={
                "2" * 40: {"sha": "2" * 40, "truncated": False, "tree": []},
                "3" * 40: {"sha": "3" * 40, "truncated": False, "tree": payload},
            },
            checkout_present=checkout_present,
        )
        assert "target state disagree" in failure and not output
    failure, output = run_classifier([], checkout_blob="2" * 40)
    assert "target blob disagree" in failure and not output


def test_classifier_rejects_unavailable_exact_commits() -> None:
    failures = (missing_target(), missing_target(code=403), {"sha": "c" * 40}, [])
    for commit in ("b" * 40, "a" * 40):
        for payload in failures:
            commits = {
                "b" * 40: {"sha": "b" * 40, "tree": {"sha": "2" * 40}},
                "a" * 40: {"sha": "a" * 40, "tree": {"sha": "3" * 40}},
            }
            commits[commit] = payload
            failure, output = run_classifier([], commit_payloads=commits)
            assert "exact commit inspection" in failure and not output, (commit, payload)


def test_classifier_requires_successful_tree_evidence_for_absence() -> None:
    for unavailable in (missing_target(), missing_target(code=403), urllib.error.URLError("transport")):
        failure, output = run_classifier(
            [],
            tree_payloads={
                "2" * 40: unavailable,
                "3" * 40: {"sha": "3" * 40, "truncated": False, "tree": []},
            },
            checkout_present=False,
        )
        assert "exact tree inspection failed" in failure and not output


def test_classifier_validates_every_tree_entry_before_proving_absence() -> None:
    valid_elsewhere = (
        {"path": "docs", "mode": "040000", "type": "tree", "sha": "4" * 40},
        {"path": "link", "mode": "120000", "type": "blob", "sha": "5" * 40},
        {"path": "vendor/submodule", "mode": "160000", "type": "commit", "sha": "6" * 40},
    )
    valid_trees = {
        "2" * 40: {"sha": "2" * 40, "truncated": False, "tree": list(valid_elsewhere)},
        "3" * 40: {"sha": "3" * 40, "truncated": False, "tree": list(valid_elsewhere)},
    }
    failure, output = run_classifier([], tree_payloads=valid_trees, checkout_present=False)
    assert failure is None and output == classification_output(None), (failure, output)

    malformed_paths = (None, "", "/absolute", "bad\x00path", "a//b", "./a", "a/../b")
    invalid_entries = [
        {"path": path, "mode": "100644", "type": "blob", "sha": "4" * 40}
        for path in malformed_paths
    ]
    invalid_entries.append(
        {"path": "docs/readme.md", "mode": "bogus", "type": "blob", "sha": "4" * 40}
    )
    legal_modes = {
        "blob": ("100644", "100755", "120000"),
        "tree": ("040000",),
        "commit": ("160000",),
    }
    for entry_type in legal_modes:
        for other_type, modes in legal_modes.items():
            if entry_type != other_type:
                invalid_entries.extend(
                    {"path": "elsewhere", "mode": mode, "type": entry_type, "sha": "4" * 40}
                    for mode in modes
                )
    for entry in invalid_entries:
        trees = copy.deepcopy(valid_trees)
        trees["3" * 40]["tree"] = [entry]
        failure, output = run_classifier([], tree_payloads=trees, checkout_present=False)
        assert "malformed entry" in failure and not output, entry

    for target_entry in (
        {"path": TARGET, "mode": "120000", "type": "blob", "sha": "4" * 40},
        {"path": TARGET, "mode": "040000", "type": "tree", "sha": "4" * 40},
        {"path": TARGET, "mode": "160000", "type": "commit", "sha": "4" * 40},
    ):
        trees = copy.deepcopy(valid_trees)
        trees["3" * 40]["tree"] = [target_entry]
        failure, output = run_classifier([], tree_payloads=trees)
        assert "unexpected" in failure and not output, target_entry


def test_classifier_rejects_invalid_changed_file_counts() -> None:
    for value in (-1, True, "803", None):
        failure, output = run_classifier([], initial_change=lambda pull, value=value: pull.update(changed_files=value))
        assert failure is not None and not output
    for value in (-1, True, 1.0, "0", None):
        failure, output = run_classifier(
            [], final_change=lambda pull, value=value: pull.update(changed_files=value)
        )
        assert "invalid post-inspection changed-files count" in failure and not output


def test_evidence_step_condition_family() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    baseline_errors = validate_mutated_gate(baseline)
    assert not baseline_errors, baseline_errors
    for job, name in (
        ("rust_linux", "Run registered PostgreSQL E2E targets when allocated"),
        ("rust_windows", "Verify deterministic simulation golden fixtures"),
        ("rust_windows", "Test Windows input platform"),
    ):
        marker = f"      - name: {name}\n"
        for condition in ('"if": false', "'if': false", "continue-on-error: true", '"continue-on-error": true', "'continue-on-error': true"):
            errors = validate_mutated_gate(baseline.replace(marker, marker + f"        {condition}\n", 1))
            assert any(job in error and "evidence step" in error for error in errors), (name, condition, errors)


def test_evidence_job_failure_cannot_be_tolerated() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    baseline_errors = validate_mutated_gate(baseline)
    assert not baseline_errors, baseline_errors
    accepted = []
    for job in ("rust_linux", "rust_windows"):
        marker = f"  {job}:\n"
        assert baseline.count(marker) == 1
        for key in ("continue-on-error", '"continue-on-error"', "'continue-on-error'"):
            errors = validate_mutated_gate(baseline.replace(marker, marker + f"    {key}: true\n", 1))
            if not any(job in error and "continue-on-error" in error for error in errors):
                accepted.append((job, key))
    assert not accepted, f"validator accepted tolerated evidence-job failure: {accepted}"


def test_postgres_early_exit_cannot_preserve_contract_strings() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    marker = "          set -euo pipefail\n"
    linux = load_validator().job_block(baseline, "rust_linux")
    assert linux is not None and linux.count(marker) == 1
    mutated = baseline.replace(linux, linux.replace(marker, marker + "          exit 0\n", 1), 1)
    errors = validate_mutated_gate(mutated)
    assert errors, "validator accepted early exit before PG evidence with contract strings intact"


def test_post_classification_target_drift_cannot_preserve_contract_strings() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    evidence = load_validator().step_block(
        load_validator().job_block(baseline, "rust_linux"),
        "Run registered PostgreSQL E2E targets when allocated",
    )
    assert evidence is not None
    mutations = (
        evidence.replace('                checkout_blob="$(git hash-object -- "$path")"\n', "", 1),
        evidence.replace(
            '                if [[ ! "$classified_blob" =~ ^[0-9a-f]{40}$ || "$checkout_blob" != "$classified_blob" ]]; then\n',
            '                if [[ ! "$classified_blob" =~ ^[0-9a-f]{40}$ ]]; then\n',
            1,
        ),
        evidence.replace(
            '                if [[ ! -f "$path" || -L "$path" ]]; then\n',
            '                if [[ ! -f "$path" ]]; then\n',
            1,
        ),
        evidence.replace(
            '                if [[ -e "$path" || -L "$path" || -n "$classified_blob" ]]; then\n',
            '                if [[ -e "$path" ]]; then\n',
            1,
        ),
    )
    for mutated_evidence in mutations:
        assert mutated_evidence != evidence
        mutated = baseline.replace(evidence, mutated_evidence, 1)
        # The fixed Cargo commands and target mappings remain, but deleting or
        # mutating a target after classification must no longer validate.
        for name, target in REGISTERED_POSTGRES_TARGETS.items():
            assert f"run_registered_target {name} {target}" in mutated
        errors = validate_mutated_gate(mutated)
        assert errors, "validator accepted post-classification target drift with old commands intact"


def test_cargo_target_source_remap_cannot_preserve_contract_strings() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    evidence = load_validator().step_block(
        load_validator().job_block(baseline, "rust_linux"),
        "Run registered PostgreSQL E2E targets when allocated",
    )
    assert evidence is not None
    mutations = (
        evidence.replace(
            "              matches = [target for target in targets if target.get('name') == name]\n",
            "              matches = [target for target in targets if target.get('src_path')]\n",
            1,
        ),
        evidence.replace(
            "              if len(matches) != 1 or matches[0].get('kind') != ['test']:\n",
            "              if not matches:\n",
            1,
        ),
        evidence.replace(
            "              if observed != expected:\n",
            "              if not observed:\n",
            1,
        ),
        evidence.replace('                verify_registered_target_binding "$name" "$path"\n', "", 1),
    )
    for mutated_evidence in mutations:
        assert mutated_evidence != evidence
        mutated = baseline.replace(evidence, mutated_evidence, 1)
        for name, target in REGISTERED_POSTGRES_TARGETS.items():
            assert f"run_registered_target {name} {target}" in mutated
        assert validate_mutated_gate(mutated), (
            "validator accepted an explicit [[test]] name/path remap bypass while fixed commands remained"
        )


def test_fixed_postgres_target_mapping_cannot_be_suppressed() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    baseline_errors = validate_mutated_gate(baseline)
    assert not baseline_errors, baseline_errors
    for name, target in REGISTERED_POSTGRES_TARGETS.items():
        variable = name.upper()
        marker = (
            f'          run_registered_target {name} {target} '
            f'"${variable}_PRESENT" "${variable}_BLOB"\n'
        )
        assert baseline.count(marker) == 1, (name, target)
        errors = validate_mutated_gate(baseline.replace(marker, "", 1))
        assert any("rust_linux" in error for error in errors), (name, errors)


def test_canonical_workflow_directory_predicate_is_not_content_consumption() -> None:
    classifier_path = Path(__file__).with_name("classify_pr_test_lanes.py")
    spec = importlib.util.spec_from_file_location("routing_directory_predicate_regression", classifier_path)
    assert spec is not None and spec.loader is not None
    classifier = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(classifier)

    incident_paths = (
        "docs/agents/PROMPT_LIFECYCLE.json",
        "docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md",
        "docs/agents/prompts/OTV2_FULL_CONTENT_CENSUS_PROGRAMME.md",
        "docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md",
        "docs/architecture/README.md",
        "tools/agents/tests/test_meta_agent_policy_adoption.py",
    )
    exact_helper = "tools/content/helper.py"
    directory_helper = "tools/content/fixtures/input.json"
    mixed_directory_helper = "tools/mixed/fixtures/input.json"
    dynamic_predicate_helper = "tools/dynamic/fixtures/input.json"

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        roots = {
            "oteryn-game-server": "apps/game-server",
            "oteryn-client": "apps/client",
            "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
            "oteryn-simulation-determinism": "crates/simulation-determinism",
        }
        for package_root in roots.values():
            manifest = root / package_root / "Cargo.toml"
            manifest.parent.mkdir(parents=True, exist_ok=True)
            manifest.write_text("[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n", encoding="utf-8")

        for path in (
            *incident_paths,
            exact_helper,
            directory_helper,
            mixed_directory_helper,
            dynamic_predicate_helper,
        ):
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("fixture\n", encoding="utf-8")

        workflows = root / ".github/workflows"
        workflows.mkdir(parents=True, exist_ok=True)
        (workflows / "merge-gate.yml").write_text("name: merge-gate\n", encoding="utf-8")
        (workflows / "rust.yml").write_text("name: rust\n", encoding="utf-8")
        (workflows / "merge-group-gate.yml").write_text(
            "routing = all(path.startswith('docs/architecture/') and path.endswith('.md') for path in paths)\n"
            f"run = 'python {exact_helper}'\n"
            "discover = 'python -m unittest discover -s tools/content/fixtures/'\n"
            "mixed_routing = all(path.startswith('tools/mixed/fixtures/') for path in paths)\n"
            "mixed_discover: python -m unittest discover -s tools/mixed/fixtures/\n"
            "dynamic_routing = path.startswith('tools/dynamic/fixtures/')\n"
            "dynamic_consumer = open(path).read()\n",
            encoding="utf-8",
        )

        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(
            [
                "git", "-c", "user.name=Oteryn CI", "-c", "user.email=ci@example.invalid",
                "commit", "-q", "-m", "fixture",
            ],
            cwd=root,
            check=True,
        )
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
        metadata = {
            "workspace_root": str(root),
            "workspace_members": list(roots),
            "packages": [
                {
                    "id": name,
                    "name": name,
                    "manifest_path": str(root / package_root / "Cargo.toml"),
                    "dependencies": [],
                }
                for name, package_root in roots.items()
            ],
        }

        previous = Path.cwd()
        try:
            os.chdir(root)
            consumers = classifier.candidate_reference_consumers(
                metadata,
                sha,
                [
                    *incident_paths,
                    exact_helper,
                    directory_helper,
                    mixed_directory_helper,
                    dynamic_predicate_helper,
                ],
            )
        finally:
            os.chdir(previous)

    for path in incident_paths:
        assert consumers[path] == set(), (path, consumers[path])
    assert consumers[exact_helper] == {classifier.CONTROL_CONSUMER}, consumers[exact_helper]
    assert consumers[directory_helper] == {classifier.CONTROL_CONSUMER}, consumers[directory_helper]
    assert consumers[mixed_directory_helper] == {classifier.CONTROL_CONSUMER}, consumers[mixed_directory_helper]
    assert consumers[dynamic_predicate_helper] == {classifier.CONTROL_CONSUMER}, consumers[dynamic_predicate_helper]


def test_audited_routing_predicate_rejects_additional_glob_consumer() -> None:
    classifier_path = Path(__file__).with_name("classify_pr_test_lanes.py")
    spec = importlib.util.spec_from_file_location("routing_glob_consumer_regression", classifier_path)
    assert spec is not None and spec.loader is not None
    classifier = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(classifier)

    consumer_path = ".github/workflows/merge-group-gate.yml"
    pattern = "docs/architecture"
    predicate_only = (
        b"routing = all(path.startswith('docs/architecture/') "
        b"and path.endswith('.md') for path in paths)\n"
    )
    assert classifier.workflow_directory_reference_is_routing_only(
        consumer_path,
        predicate_only,
        pattern,
    )

    predicate_and_glob = (
        predicate_only
        + b"hash = hashFiles('docs/architecture/**/*.md')\n"
    )
    occurrences = classifier.directory_reference_occurrences(
        predicate_and_glob,
        pattern,
        include_descendants=True,
    )
    assert len(occurrences) == 2, occurrences
    assert not classifier.workflow_directory_reference_is_routing_only(
        consumer_path,
        predicate_and_glob,
        pattern,
    )

    predicate_and_adjacent_glob = (
        predicate_only
        + b"hash = hashFiles('docs/architecture*/**/*.md')\n"
    )
    adjacent_occurrences = classifier.directory_reference_occurrences(
        predicate_and_adjacent_glob,
        pattern,
        include_descendants=True,
    )
    assert len(adjacent_occurrences) == 2, adjacent_occurrences
    assert not classifier.workflow_directory_reference_is_routing_only(
        consumer_path,
        predicate_and_adjacent_glob,
        pattern,
    )

    predicate_and_shell_consumer = (
        predicate_only
        + b"run: tar -cf out docs/architecture; echo done\n"
    )
    shell_occurrences = classifier.directory_reference_occurrences(
        predicate_and_shell_consumer,
        pattern,
        include_descendants=True,
    )
    assert len(shell_occurrences) == 2, shell_occurrences
    assert not classifier.workflow_directory_reference_is_routing_only(
        consumer_path,
        predicate_and_shell_consumer,
        pattern,
    )


def test_postgres_digest_and_invocation_are_mandatory() -> None:
    baseline = MERGE_GATE.read_text(encoding="utf-8")
    stale = baseline.replace("      - name: Build workspace\n", "      - name: Build workspace # stale\n", 1)
    assert any("rust_linux" in error and "exactly match" in error for error in validate_mutated_gate(stale))
    invocation = '              cargo +1.94.0 test --locked -p oteryn-game-server --test "$name"\n'
    assert baseline.count(invocation) == 1
    errors = validate_mutated_gate(baseline.replace(invocation, "", 1))
    assert any("rust_linux" in error for error in errors), errors


def main() -> int:
    tests = (
        test_registered_postgres_targets_are_materially_routed,
        test_pr_gate_rejects_explicit_cargo_test_path_remap,
        test_postgres_evidence_step_cannot_be_skipped,
        test_simulation_evidence_step_cannot_be_skipped,
        test_input_platform_evidence_contract_is_mandatory,
        test_classifier_rejects_identity_races,
        test_classifier_rejects_unbound_base,
        test_classifier_exact_target_state_matrix,
        test_classifier_fails_closed_for_each_protected_registered_target_removal,
        test_classifier_rejects_bad_target_evidence_and_checkout_mismatch,
        test_classifier_rejects_unavailable_exact_commits,
        test_classifier_requires_successful_tree_evidence_for_absence,
        test_classifier_validates_every_tree_entry_before_proving_absence,
        test_classifier_rejects_invalid_changed_file_counts,
        test_evidence_step_condition_family,
        test_scope_rejects_identity_races,
        test_scope_preserves_stable_classification,
        test_scope_bounds_environment_transport,
        test_classifiers_bind_aba_to_immutable_authority,
        test_scope_rejects_comparison_truncation_while_pg_accepts_large_diff,
        test_protected_classifier_red_for_valid_803_file_target,
        test_evidence_job_failure_cannot_be_tolerated,
        test_postgres_early_exit_cannot_preserve_contract_strings,
        test_post_classification_target_drift_cannot_preserve_contract_strings,
        test_cargo_target_source_remap_cannot_preserve_contract_strings,
        test_fixed_postgres_target_mapping_cannot_be_suppressed,
        test_canonical_workflow_directory_predicate_is_not_content_consumption,
        test_audited_routing_predicate_rejects_additional_glob_consumer,
        test_postgres_digest_and_invocation_are_mandatory,
    )
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    spec = importlib.util.spec_from_file_location("queue_regressions", Path(__file__).with_name("test_validate_merge_group_pg_sim.py"))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.main() == 0
    spec = importlib.util.spec_from_file_location("risk_regressions", Path(__file__).with_name("test_classify_pr_test_lanes.py"))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.main() == 0
    spec = importlib.util.spec_from_file_location(
        "content_routing_regressions",
        Path(__file__).with_name("test_classify_content_routing.py"),
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.main() == 0
    print(f"Canonical PR PG/SIM validator regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
