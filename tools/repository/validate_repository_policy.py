#!/usr/bin/env python3
"""Fail-closed wrapper for the Game repository-policy validator."""
from __future__ import annotations

import ast
import importlib.util
import re
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORE_PATH = Path(__file__).with_name("validate_repository_policy_core.py")
PR_GATE_CONTRACT_PATH = Path(__file__).with_name("validate_pr_gate_pg_sim.py")
MERGE_AUTHORITY_AUDIT = ROOT / ".github/workflows/merge-authority-audit.yml"
PR_METADATA_WORKFLOWS = (
    (ROOT / ".github/workflows/agent-governance.yml", "Verify pull request target and metadata"),
    (ROOT / ".github/workflows/merge-gate.yml", "Verify pull request metadata"),
)


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load repository policy module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_core():
    return load_module(CORE_PATH, "validate_repository_policy_core")


def validate_protected_base_audit() -> list[str]:
    if not MERGE_AUTHORITY_AUDIT.is_file():
        return ["missing protected-base merge-authority audit"]

    text = MERGE_AUTHORITY_AUDIT.read_text(encoding="utf-8")
    errors: list[str] = []
    for fragment in (
        "  pull_request_target:\n",
        "      - opened\n",
        "      - reopened\n",
        "      - synchronize\n",
        "      - edited\n",
        "      contents: read\n",
        "      pull-requests: read\n",
        "candidate merge-group gate does not match the protected-base approved blob",
        "candidate modifies the protected-base audit itself",
    ):
        if fragment not in text:
            errors.append(f"protected-base merge-authority audit missing contract: {fragment.strip()}")

    pin = re.search(r'^\s*EXPECTED_MERGE_GROUP_GATE_BLOB:\s*"([0-9a-f]{40})"\s*$', text, re.MULTILINE)
    if pin is None:
        errors.append("protected-base merge-authority audit must own an exact gate blob pin")
    if "actions/checkout@" in text:
        errors.append("protected-base merge-authority audit must not checkout candidate code")
    if re.search(r'^\s*continue-on-error\s*:', text, re.MULTILINE):
        errors.append("protected-base merge-authority audit must not permit continue-on-error")

    for fragment in (
        "merge_group_head_expression = '$' + '{{ github.event.merge_group.head_sha }}'",
        "durability_result_expression = '$' + '{{ needs.durability_postgres.result }}'",
        "f'          EXPECTED_SHA: {merge_group_head_expression}\\n'",
        "f'          DURABILITY_POSTGRES: {durability_result_expression}\\n'",
    ):
        if fragment not in text:
            errors.append(
                "protected-base merge-authority audit must construct future-context "
                f"workflow expressions at Python runtime: {fragment}"
            )

    for unsafe in (
        "          EXPECTED_SHA: ${{ github.event.merge_group.head_sha }}\\n",
        "          DURABILITY_POSTGRES: ${{ needs.durability_postgres.result }}\\n",
    ):
        if unsafe in text:
            errors.append(
                "protected-base merge-authority audit must not embed a future-context "
                f"GitHub expression directly in its pull_request_target run script: {unsafe.strip()}"
            )
    return errors


def _extract_step_python(text: str, step_name: str) -> str:
    step_marker = f"      - name: {step_name}\n"
    step_start = text.find(step_marker)
    if step_start < 0:
        raise ValueError(f"missing workflow step: {step_name}")
    heredoc = "          python - <<'PY'\n"
    source_start = text.find(heredoc, step_start)
    if source_start < 0:
        raise ValueError(f"missing Python heredoc in workflow step: {step_name}")
    source_start += len(heredoc)
    source_end = text.find("\n          PY\n", source_start)
    if source_end < 0:
        raise ValueError(f"unterminated Python heredoc in workflow step: {step_name}")
    return textwrap.dedent(text[source_start:source_end])


def _append_targets(node: ast.AST) -> set[str]:
    targets: set[str] = set()
    for child in ast.walk(node):
        if (
            isinstance(child, ast.Call)
            and isinstance(child.func, ast.Attribute)
            and child.func.attr == "append"
            and isinstance(child.func.value, ast.Name)
        ):
            targets.add(child.func.value.id)
    return targets


def _target_mentions_name(target: ast.AST, name: str) -> bool:
    return any(isinstance(node, ast.Name) and node.id == name for node in ast.walk(target))


def _channel_writes(module: ast.Module, name: str) -> list[ast.AST]:
    writes: list[ast.AST] = []
    for node in ast.walk(module):
        if isinstance(node, ast.AnnAssign) and _target_mentions_name(node.target, name):
            writes.append(node)
        elif isinstance(node, ast.Assign) and any(_target_mentions_name(target, name) for target in node.targets):
            writes.append(node)
        elif isinstance(node, ast.AugAssign) and _target_mentions_name(node.target, name):
            writes.append(node)
        elif isinstance(node, ast.NamedExpr) and _target_mentions_name(node.target, name):
            writes.append(node)
        elif isinstance(node, (ast.For, ast.AsyncFor)) and _target_mentions_name(node.target, name):
            writes.append(node)
        elif isinstance(node, ast.With):
            if any(item.optional_vars is not None and _target_mentions_name(item.optional_vars, name) for item in node.items):
                writes.append(node)
        elif isinstance(node, ast.ExceptHandler) and node.name == name:
            writes.append(node)
        elif isinstance(node, ast.Delete) and any(
            _target_mentions_name(target, name) for target in node.targets
        ):
            writes.append(node)
    return writes


def _is_independent_empty_list_initializer(node: ast.AST, name: str) -> bool:
    if isinstance(node, ast.AnnAssign):
        return (
            isinstance(node.target, ast.Name)
            and node.target.id == name
            and isinstance(node.value, ast.List)
            and not node.value.elts
        )
    if isinstance(node, ast.Assign):
        return (
            len(node.targets) == 1
            and isinstance(node.targets[0], ast.Name)
            and node.targets[0].id == name
            and isinstance(node.value, ast.List)
            and not node.value.elts
        )
    return False


def _channel_mutating_methods(module: ast.Module, name: str) -> set[str]:
    methods: set[str] = set()
    for node in ast.walk(module):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and isinstance(node.func.value, ast.Name)
            and node.func.value.id == name
            and node.func.attr != "append"
        ):
            methods.add(node.func.attr)
    return methods


def validate_pr_metadata_workflow_text(text: str, label: str, step_name: str) -> list[str]:
    errors: list[str] = []
    try:
        source = _extract_step_python(text, step_name)
        module = ast.parse(source)
    except (SyntaxError, ValueError) as exc:
        return [f"{label} metadata validator is not parseable: {exc}"]

    required_identity_tests = (
        "state != 'open'",
        "head_sha != expected_head",
        "head_repository != repository",
        "base_ref != 'main'",
    )
    top_level_ifs = [stmt for stmt in module.body if isinstance(stmt, ast.If)]
    tests = [(ast.unparse(stmt.test), stmt) for stmt in top_level_ifs]
    for required in required_identity_tests:
        matches = [stmt for rendered, stmt in tests if rendered == required]
        if len(matches) != 1 or "errors" not in _append_targets(matches[0]):
            errors.append(f"{label} must hard-fail identity predicate: {required}")

    for channel in ("errors", "warnings"):
        writes = _channel_writes(module, channel)
        if len(writes) != 1 or not _is_independent_empty_list_initializer(writes[0], channel):
            errors.append(
                f"{label} {channel} channel must have exactly one independent empty-list initializer"
            )
        methods = _channel_mutating_methods(module, channel)
        if methods:
            errors.append(
                f"{label} {channel} channel uses forbidden mutating methods: {sorted(methods)!r}"
            )

    warnings_index = next(
        (
            index
            for index, stmt in enumerate(module.body)
            if isinstance(stmt, ast.AnnAssign)
            and isinstance(stmt.target, ast.Name)
            and stmt.target.id == "warnings"
        ),
        None,
    )
    final_error_guard: ast.If | None = None
    if warnings_index is None:
        errors.append(f"{label} missing advisory warnings boundary")
    else:
        for stmt in module.body[warnings_index + 1 :]:
            if isinstance(stmt, ast.If) and ast.unparse(stmt.test) == "errors":
                final_error_guard = stmt
                break
            if "errors" in _append_targets(stmt):
                errors.append(
                    f"{label} presentation section must not append blocking errors after warnings boundary"
                )
                break

    if final_error_guard is None:
        errors.append(f"{label} missing final blocking error guard")
    else:
        terminal_raises = [
            stmt
            for stmt in final_error_guard.body
            if isinstance(stmt, ast.Raise)
            and isinstance(stmt.exc, ast.Call)
            and isinstance(stmt.exc.func, ast.Name)
            and stmt.exc.func.id == "SystemExit"
            and len(stmt.exc.args) == 1
            and isinstance(stmt.exc.args[0], ast.Constant)
            and stmt.exc.args[0].value == 1
        ]
        if len(terminal_raises) != 1:
            errors.append(f"{label} final blocking error guard must raise SystemExit(1)")

    for rendered, stmt in tests:
        names = {node.id for node in ast.walk(stmt.test) if isinstance(node, ast.Name)}
        if names.intersection({"title", "body", "pattern", "headings", "validation_word"}):
            channels = _append_targets(stmt)
            if "errors" in channels:
                errors.append(
                    f"{label} presentation predicate must not append blocking errors: {rendered}"
                )

    required_advisory_markers = (
        "warnings.append('PR title should be at most 72 characters')",
        "warnings.append('PR title should follow type(scope): imperative summary')",
        "warnings.append('PR body should include a Summary section')",
        "warnings.append('PR body should include a Scope section')",
        "warnings.append('PR body should include a validation section')",
        "re.compile(r'(?<![a-z0-9])validation(?![a-z0-9])')",
        "::warning title=PR metadata guidance::",
    )
    for marker in required_advisory_markers:
        if marker not in source:
            errors.append(f"{label} missing advisory metadata behavior: {marker}")
    return errors


def validate_pr_metadata_advisory_contract() -> list[str]:
    errors: list[str] = []
    for path, step_name in PR_METADATA_WORKFLOWS:
        if not path.is_file():
            errors.append(f"missing PR metadata workflow: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        errors.extend(
            validate_pr_metadata_workflow_text(text, str(path.relative_to(ROOT)), step_name)
        )
    return errors

def validate_pr_gate_contract() -> list[str]:
    module = load_module(PR_GATE_CONTRACT_PATH, "validate_pr_gate_pg_sim")
    return module.validate()


def main() -> int:
    errors = validate_protected_base_audit()
    errors.extend(validate_pr_metadata_advisory_contract())
    errors.extend(validate_pr_gate_contract())
    if errors:
        print("Repository policy validation failed:")
        for error in errors:
            print(f"- {error}")
        return 1
    return load_core().main()


if __name__ == "__main__":
    raise SystemExit(main())
