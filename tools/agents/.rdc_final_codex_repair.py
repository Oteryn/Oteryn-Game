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
EXPECTED_TEST_BLOB = "8eb87a366c817f70e1f4826c811c7aafc44a2100"
EXPECTED_VALIDATOR_BLOB = "1e418d8fd8fab50a331410e0810083efdc33f17b"


def run(*args: str) -> str:
    print("+", " ".join(args), flush=True)
    return subprocess.check_output(args, text=True).strip()


def call(*args: str) -> None:
    print("+", " ".join(args), flush=True)
    subprocess.check_call(args)


assert run("git", "rev-parse", f"HEAD:{TEST.relative_to(ROOT)}") == EXPECTED_TEST_BLOB
assert run("git", "rev-parse", f"HEAD:{VALIDATOR.relative_to(ROOT)}") == EXPECTED_VALIDATOR_BLOB

test_text = TEST.read_text(encoding="utf-8")
marker = "\ndef main() -> int:\n"
assert test_text.count(marker) == 1
tests = r'''

def test_stale_meta_coordinate_outside_prompt_section_fails() -> None:
    text = (
        "# Prompt\n\n"
        "Use Oteryn/Oteryn@0000000000000000000000000000000000000000:"
        "ecosystem/agent-execution-routing-policy.json for host exceptions.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "stale META execution-routing coordinate")


def test_surface_stale_meta_coordinate_outside_section_fails() -> None:
    text = (
        "# Surface\n\n"
        "Use Oteryn/Oteryn@0000000000000000000000000000000000000000:"
        "ecosystem/agent-execution-routing-policy.json for host exceptions.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "stale META execution-routing coordinate")


def test_connector_authorization_without_direct_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nConnector calls are automatically authorized.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_connector_operation_host_exception_without_direct_fails() -> None:
    text = (
        "# Surface\n\nConnector operations need no host exception.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")
'''
TEST.write_text(test_text.replace(marker, tests + marker), encoding="utf-8")

call("git", "config", "user.name", "github-actions[bot]")
call("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
call("git", "add", str(TEST.relative_to(ROOT)))
call("git", "diff", "--cached", "--check")
call("git", "commit", "-m", "test(agents): cover final Codex routing authority bypasses")
red_sha = run("git", "rev-parse", "HEAD")
print("RED test-only head:", red_sha, flush=True)

sys.path.insert(0, str(TEST.parent))
spec = importlib.util.spec_from_file_location("rdc_tests", TEST)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
new_tests = [
    "test_stale_meta_coordinate_outside_prompt_section_fails",
    "test_surface_stale_meta_coordinate_outside_section_fails",
    "test_connector_authorization_without_direct_outside_section_fails",
    "test_surface_connector_operation_host_exception_without_direct_fails",
]
for name in new_tests:
    try:
        getattr(module, name)()
    except AssertionError as exc:
        print(f"EXPECTED RED {name}: {exc}", flush=True)
    else:
        raise SystemExit(f"{name} unexpectedly passed before validator repair")

call("git", "push", "origin", f"HEAD:{BRANCH}")

validator = VALIDATOR.read_text(encoding="utf-8")
ping_anchor = r'    re.compile(r"\bping\b.{0,100}\b(?:capability|discover|connector|tool|host)\b", re.IGNORECASE),' + "\n"
assert validator.count(ping_anchor) == 1
generic_pattern = r'''    re.compile(
        r"(?=.*\b(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:calls?|operations?|requests?|invocations?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)\b)",
        re.IGNORECASE,
    ),
'''
validator = validator.replace(ping_anchor, generic_pattern + ping_anchor)

surface_anchor = "\nAPPROVED_SURFACE_OUTSIDE_ROUTING_PARAGRAPHS = {\n"
assert validator.count(surface_anchor) == 1
meta_defs = r'''META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\.json"
)
EXPECTED_META_ROUTING_COORDINATE = (
    f"Oteryn/Oteryn@{META_SHA}:ecosystem/agent-execution-routing-policy.json"
)

'''
validator = validator.replace(
    surface_anchor,
    "\n" + meta_defs + "APPROVED_SURFACE_OUTSIDE_ROUTING_PARAGRAPHS = {\n",
)

validate_anchor = "\ndef validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:\n"
assert validator.count(validate_anchor) == 1
meta_helper = '''def _validate_meta_routing_coordinates(path: str, text: str, errors: list[str]) -> None:
    coordinates = META_ROUTING_COORDINATE_RE.findall(text)
    stale = sorted(
        {coordinate for coordinate in coordinates if coordinate != EXPECTED_META_ROUTING_COORDINATE}
    )
    for coordinate in stale:
        errors.append(f"{path}: stale META execution-routing coordinate: {coordinate}")

'''
validator = validator.replace(
    validate_anchor,
    "\n" + meta_helper + "def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:\n",
)

prompt_start = "def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:\n    extracted = _extract_canonical_section(path, text, errors)\n"
assert validator.count(prompt_start) == 1
validator = validator.replace(
    prompt_start,
    "def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:\n"
    "    _validate_meta_routing_coordinates(path, text, errors)\n"
    "    extracted = _extract_canonical_section(path, text, errors)\n",
)

surface_start = "def validate_surface_text(path: str, text: str, errors: list[str]) -> None:\n    extracted = _extract_canonical_section(path, text, errors)\n"
assert validator.count(surface_start) == 1
validator = validator.replace(
    surface_start,
    "def validate_surface_text(path: str, text: str, errors: list[str]) -> None:\n"
    "    _validate_meta_routing_coordinates(path, text, errors)\n"
    "    extracted = _extract_canonical_section(path, text, errors)\n",
)

VALIDATOR.write_text(validator, encoding="utf-8")

call(sys.executable, "tools/agents/test_validate_remote_desktop_prompt_routing.py")
call(sys.executable, "tools/agents/validate_remote_desktop_prompt_routing.py")
call(sys.executable, "tools/agents/validate_governance.py")
call(sys.executable, "tools/repository/validate_repository_policy.py")
call("git", "diff", "--check")
call("git", "add", str(VALIDATOR.relative_to(ROOT)))
call("git", "diff", "--cached", "--check")
call("git", "commit", "-m", "fix(agents): reject stale routing authority bypasses")
green_sha = run("git", "rev-parse", "HEAD")
call("git", "push", "origin", f"HEAD:{BRANCH}")
print("GREEN implementation head:", green_sha, flush=True)
