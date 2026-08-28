#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

BRANCH = "governance/remote-desktop-per-action-gate-237"
TEST = Path("tools/agents/test_validate_remote_desktop_prompt_routing.py")
VALIDATOR = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
EXPECTED_TEST_BLOB = "c2fd872cc1d91828d065f5ea38865bfa0a1c8d49"
EXPECTED_VALIDATOR_BLOB = "061c671b45dad824ee90f7bce8448aeed295aae1"


def run(*args):
    print("+", " ".join(args), flush=True)
    subprocess.run(args, check=True)


def out(*args):
    return subprocess.check_output(args, text=True).strip()


assert out("git", "hash-object", str(TEST)) == EXPECTED_TEST_BLOB
assert out("git", "hash-object", str(VALIDATOR)) == EXPECTED_VALIDATOR_BLOB

tests = r'''


def test_call_first_connector_automatically_authorized_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nCalls to the connector are automatically authorized.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_call_first_tool_blanket_approval_outside_section_fails() -> None:
    text = (
        "# Surface\n\nRequests to the tool have blanket approval.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")
'''

test_text = TEST.read_text()
anchor = "\ndef main() -> int:"
assert anchor in test_text
assert "test_call_first_connector_automatically_authorized_outside_section_fails" not in test_text
TEST.write_text(test_text.replace(anchor, tests + anchor, 1))

run("git", "config", "user.name", "github-actions[bot]")
run("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
run("git", "add", str(TEST))
run("git", "diff", "--cached", "--check")
run("git", "commit", "-m", "test(agents): cover call-first connector authorization")
red_sha = out("git", "rev-parse", "HEAD")
print("RED test-only head:", red_sha, flush=True)

sys.path.insert(0, str(TEST.parent.resolve()))
import test_validate_remote_desktop_prompt_routing as t
for name in (
    "test_call_first_connector_automatically_authorized_outside_section_fails",
    "test_surface_call_first_tool_blanket_approval_outside_section_fails",
):
    try:
        getattr(t, name)()
    except AssertionError as exc:
        print(f"EXPECTED RED {name}: {exc}", flush=True)
    else:
        raise SystemExit(f"UNEXPECTED GREEN before validator repair: {name}")

run("git", "push", "origin", f"HEAD:{BRANCH}")

source = VALIDATOR.read_text()
anchor_pattern = r'''    re.compile(
        r"(?=.*\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?)\b)"
        r"(?=.*\b(?:(?:blanket|standing|automatic)\s+approval|preapproval|approval\s+by\s+default)\b)",
        re.IGNORECASE,
    ),
'''
replacement = anchor_pattern + r'''    re.compile(
        r"(?=.*\b(?:calls?|operations?|requests?|invocations?)\s+to\s+(?:the\s+)?(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic)\s+approval|preapproval|approval\s+by\s+default)\b)",
        re.IGNORECASE,
    ),
'''
assert source.count(anchor_pattern) == 1
source = source.replace(anchor_pattern, replacement, 1)
VALIDATOR.write_text(source)

run(sys.executable, str(TEST))
run(sys.executable, str(VALIDATOR))
run(sys.executable, "tools/agents/validate_governance.py")
run(sys.executable, "tools/repository/validate_repository_policy.py")
run("git", "diff", "--check")
run("git", "add", str(VALIDATOR))
run("git", "diff", "--cached", "--check")
run("git", "commit", "-m", "fix(agents): reject call-first connector authorization")
green_sha = out("git", "rev-parse", "HEAD")
run("git", "push", "origin", f"HEAD:{BRANCH}")
print("GREEN implementation head:", green_sha, flush=True)
