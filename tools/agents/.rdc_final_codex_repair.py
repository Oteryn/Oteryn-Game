#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

ROOT = pathlib.Path.cwd()
TEST = ROOT / "tools/agents/test_validate_remote_desktop_prompt_routing.py"
VALIDATOR = ROOT / "tools/agents/validate_remote_desktop_prompt_routing.py"
BRANCH = "governance/remote-desktop-per-action-gate-237"
EXPECTED_TEST_BLOB = "d90fc8bbe749578f754a0c20cff6b7cacb89e566"
EXPECTED_VALIDATOR_BLOB = "82a6b2309963c4f77338434dcf97ead7a758c0de"


def run(*args: str) -> str:
    print("+", " ".join(args), flush=True)
    return subprocess.check_output(args, text=True).strip()


def call(*args: str) -> None:
    print("+", " ".join(args), flush=True)
    subprocess.check_call(args)


assert run("git", "rev-parse", f"HEAD:{TEST.relative_to(ROOT)}") == EXPECTED_TEST_BLOB
assert run("git", "rev-parse", f"HEAD:{VALIDATOR.relative_to(ROOT)}") == EXPECTED_VALIDATOR_BLOB

test_text = TEST.read_text(encoding="utf-8")
import_anchor = "    validate_reusable_prompt_text,\n"
assert test_text.count(import_anchor) == 1
test_text = test_text.replace(import_anchor, "    reusable_prompt_paths,\n" + import_anchor)
marker = "\ndef main() -> int:\n"
assert test_text.count(marker) == 1
new_tests = r'''

def test_formatted_stale_meta_coordinate_outside_prompt_section_fails() -> None:
    text = (
        "# Prompt\n\n"
        "Use Oteryn/Oteryn@00000000000000000000**00000000000000000000**:"
        "ecosystem/agent-execution-routing-policy.json for host exceptions.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "stale META execution-routing coordinate")


def test_surface_entity_stale_meta_coordinate_outside_section_fails() -> None:
    text = (
        "# Surface\n\n"
        "Use Oteryn/Oteryn@0000000000000000000000000000000000000000:"
        "ecosystem/agent-execution-routing-policy&#46;json for host exceptions.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "stale META execution-routing coordinate")


def test_preapproved_connector_calls_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nConnector calls are preapproved.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_connector_operations_approved_by_default_fails() -> None:
    text = (
        "# Surface\n\nConnector operations are approved by default.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def test_transport_guarantees_per_action_gate_outside_section_fails() -> None:
    text = (
        "# Prompt\n\n"
        "The transport guarantees every per-action decision and makes repository checks advisory.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_connector_implements_per_action_gate_outside_section_fails() -> None:
    text = (
        "# Surface\n\nThe connector implements the per-action gate, so repository checks are advisory.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def test_reusable_status_false_flag_fails_closed_and_remains_in_scope() -> None:
    path = "docs/agents/prompts/EXAMPLE.md"
    lifecycle = {"prompts": [{"status": "reusable", "reusable": False, "path": path}]}
    errors: list[str] = []
    paths = reusable_prompt_paths(lifecycle, errors)
    if not any("inconsistent reusable status/flag" in error for error in errors):
        raise AssertionError(f"expected lifecycle consistency error, got: {errors}")
    if path not in paths:
        raise AssertionError(f"status=reusable prompt must remain validation-scoped, got: {paths}")


def test_nonreusable_status_true_flag_fails_closed() -> None:
    lifecycle = {
        "prompts": [{
            "status": "retired",
            "reusable": True,
            "path": "docs/agents/prompts/RETIRED.md",
        }]
    }
    errors: list[str] = []
    reusable_prompt_paths(lifecycle, errors)
    if not any("inconsistent reusable status/flag" in error for error in errors):
        raise AssertionError(f"expected lifecycle consistency error, got: {errors}")
'''
TEST.write_text(test_text.replace(marker, new_tests + marker), encoding="utf-8")

call("git", "config", "user.name", "github-actions[bot]")
call("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
call("git", "add", str(TEST.relative_to(ROOT)))
call("git", "diff", "--cached", "--check")
call("git", "commit", "-m", "test(agents): cover final Codex fail-closed bypasses")
red_sha = run("git", "rev-parse", "HEAD")
print("RED test-only head:", red_sha, flush=True)

sys.path.insert(0, str(TEST.parent))
spec = importlib.util.spec_from_file_location("rdc_tests", TEST)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
red_tests = [
    "test_formatted_stale_meta_coordinate_outside_prompt_section_fails",
    "test_surface_entity_stale_meta_coordinate_outside_section_fails",
    "test_preapproved_connector_calls_outside_section_fails",
    "test_surface_connector_operations_approved_by_default_fails",
    "test_transport_guarantees_per_action_gate_outside_section_fails",
    "test_surface_connector_implements_per_action_gate_outside_section_fails",
    "test_reusable_status_false_flag_fails_closed_and_remains_in_scope",
    "test_nonreusable_status_true_flag_fails_closed",
]
for name in red_tests:
    try:
        getattr(module, name)()
    except AssertionError as exc:
        print(f"EXPECTED RED {name}: {exc}", flush=True)
    else:
        raise SystemExit(f"{name} unexpectedly passed before validator repair")

call("git", "push", "origin", f"HEAD:{BRANCH}")

validator = VALIDATOR.read_text(encoding="utf-8")

old_lifecycle = '''        if entry.get("status") != "reusable" or entry.get("reusable") is not True:
            continue
        path = entry.get("path")
'''
new_lifecycle = '''        status = entry.get("status")
        reusable = entry.get("reusable")
        if status == "reusable":
            if reusable is not True:
                errors.append(
                    f"prompt lifecycle entry {index} has inconsistent reusable status/flag"
                )
        elif reusable is True:
            errors.append(
                f"prompt lifecycle entry {index} has inconsistent reusable status/flag"
            )
            continue
        else:
            continue
        path = entry.get("path")
'''
assert validator.count(old_lifecycle) == 1
validator = validator.replace(old_lifecycle, new_lifecycle)

old_auth = r"(?:pre)?authori[sz]ed|"
assert validator.count(old_auth) == 5
validator = validator.replace(old_auth, old_auth + r"(?:pre)?approved|")

physical_anchor = '''    re.compile(
        r"(?=.*\\b(?:connector|router|transport)\\b)"
        r"(?=.*\\bphysical(?:ly)?\\b)"
        r"(?=.*\\benforc\\w*\\b)",
        re.IGNORECASE,
    ),
'''
assert validator.count(physical_anchor) == 1
transport_claim = '''    re.compile(
        r"(?=.*\\b(?:connector|router|transport)\\b)"
        r"(?=.*\\b(?:guarantee(?:s|d|ing)?|implement(?:s|ed|ing)?)\\b)"
        r"(?=.*\\b(?:per[- ]action|decision|gate|routing|authori[sz]\\w*)\\b)",
        re.IGNORECASE,
    ),
'''
validator = validator.replace(physical_anchor, transport_claim + physical_anchor)

old_meta = '''def _validate_meta_routing_coordinates(path: str, text: str, errors: list[str]) -> None:
    coordinates = META_ROUTING_COORDINATE_RE.findall(text)
'''
new_meta = '''def _validate_meta_routing_coordinates(path: str, text: str, errors: list[str]) -> None:
    normalized = _normalize_policy_text(_operative_text(text))
    coordinates = META_ROUTING_COORDINATE_RE.findall(normalized)
'''
assert validator.count(old_meta) == 1
validator = validator.replace(old_meta, new_meta)

VALIDATOR.write_text(validator, encoding="utf-8")

call(sys.executable, "tools/agents/test_validate_remote_desktop_prompt_routing.py")
call(sys.executable, "tools/agents/validate_remote_desktop_prompt_routing.py")
call(sys.executable, "tools/agents/validate_governance.py")
call(sys.executable, "tools/repository/validate_repository_policy.py")
call("git", "diff", "--check")
call("git", "add", str(VALIDATOR.relative_to(ROOT)))
call("git", "diff", "--cached", "--check")
call("git", "commit", "-m", "fix(agents): close final Codex fail-closed bypasses")
green_sha = run("git", "rev-parse", "HEAD")
call("git", "push", "origin", f"HEAD:{BRANCH}")
print("GREEN implementation head:", green_sha, flush=True)
