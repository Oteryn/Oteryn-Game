#!/usr/bin/env python3
from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path.cwd()
TEST = ROOT / "tools/agents/test_validate_remote_desktop_prompt_routing.py"
VALIDATOR = ROOT / "tools/agents/validate_remote_desktop_prompt_routing.py"
BRANCH = "governance/remote-desktop-per-action-gate-237"
EXPECTED_TEST_BLOB = "d90fc8bbe749578f754a0c20cff6b7cacb89e566"
EXPECTED_VALIDATOR_BLOB = "1e418d8fd8fab50a331410e0810083efdc33f17b"


def run(*args: str) -> str:
    print("+", " ".join(args), flush=True)
    return subprocess.check_output(args, text=True).strip()


def call(*args: str) -> None:
    print("+", " ".join(args), flush=True)
    subprocess.check_call(args)


assert run("git", "rev-parse", f"HEAD:{TEST.relative_to(ROOT)}") == EXPECTED_TEST_BLOB
assert run("git", "rev-parse", f"HEAD:{VALIDATOR.relative_to(ROOT)}") == EXPECTED_VALIDATOR_BLOB

validator = VALIDATOR.read_text(encoding="utf-8")
ping_anchor = r'    re.compile(r"\bping\b.{0,100}\b(?:capability|discover|connector|tool|host)\b", re.IGNORECASE),' + "\n"
assert validator.count(ping_anchor) == 1
authority_terms = r'''(?:authori[sz]ation|(?:pre)?authori[sz]ed|host[- ]exception|exception|per[- ]action|exempt|without|allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no)'''
generic_patterns = f'''    re.compile(
        r"\\b(?:connectors?|tools?)\\s+(?:calls?|operations?|requests?|invocations?)\\b.{{0,160}}"
        r"\\b{authority_terms}\\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\\b{authority_terms}\\b.{{0,160}}"
        r"\\b(?:connectors?|tools?)\\s+(?:calls?|operations?|requests?|invocations?)\\b",
        re.IGNORECASE,
    ),
'''
validator = validator.replace(ping_anchor, generic_patterns + ping_anchor)

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
call("git", "config", "user.name", "github-actions[bot]")
call("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
call("git", "add", str(VALIDATOR.relative_to(ROOT)))
call("git", "diff", "--cached", "--check")
call("git", "commit", "-m", "fix(agents): reject stale routing authority bypasses")
green_sha = run("git", "rev-parse", "HEAD")
call("git", "push", "origin", f"HEAD:{BRANCH}")
print("GREEN implementation head:", green_sha, flush=True)
