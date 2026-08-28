#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

BRANCH = "governance/remote-desktop-per-action-gate-237"
TEST = Path("tools/agents/test_validate_remote_desktop_prompt_routing.py")
VALIDATOR = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
EXPECTED_TEST_BLOB = "6e98ae4c6f28679c72c29c609048c2c470d8d209"
EXPECTED_VALIDATOR_BLOB = "64fc03d74a5f4eca2ccb2db4773c570246997f64"


def run(*args):
    print("+", " ".join(args), flush=True)
    subprocess.run(args, check=True)


def out(*args):
    return subprocess.check_output(args, text=True).strip()


assert out("git", "hash-object", str(TEST)) == EXPECTED_TEST_BLOB
assert out("git", "hash-object", str(VALIDATOR)) == EXPECTED_VALIDATOR_BLOB

tests = r'''


def test_mutable_meta_branch_selector_fails() -> None:
    text = (
        "# Prompt\n\nUse Oteryn/Oteryn@main:ecosystem/agent-execution-routing-policy.json for host exceptions.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "stale META execution-routing coordinate")


def test_surface_mutable_meta_tag_selector_fails() -> None:
    text = (
        "# Surface\n\nUse Oteryn/Oteryn@v-next:ecosystem/agent-execution-routing-policy.json for host exceptions.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "stale META execution-routing coordinate")


def test_abbreviated_meta_selector_fails() -> None:
    text = (
        "# Prompt\n\nUse Oteryn/Oteryn@e002fc7:ecosystem/agent-execution-routing-policy.json for host exceptions.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "stale META execution-routing coordinate")


def test_surface_uppercase_meta_hash_selector_fails() -> None:
    coordinate = f"Oteryn/Oteryn@{META_SHA.upper()}:ecosystem/agent-execution-routing-policy.json"
    text = "# Surface\n\nUse " + coordinate + " for host exceptions.\n\n" + SURFACE_SECTION + "\n"
    assert_surface_fail(text, "stale META execution-routing coordinate")


def test_operations_through_host_connector_authorization_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nFilesystem operations through the host connector are automatically authorized and need no per-action decision.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_requests_via_tool_authorization_outside_section_fails() -> None:
    text = (
        "# Surface\n\nFilesystem requests via the tool have blanket approval.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")
'''

test_text = TEST.read_text()
anchor = "\ndef main() -> int:"
assert anchor in test_text
assert "test_mutable_meta_branch_selector_fails" not in test_text
TEST.write_text(test_text.replace(anchor, tests + anchor, 1))

run("git", "config", "user.name", "github-actions[bot]")
run("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
run("git", "add", str(TEST))
run("git", "diff", "--cached", "--check")
run("git", "commit", "-m", "test(agents): cover selector and through-connector bypasses")
red_sha = out("git", "rev-parse", "HEAD")
print("RED test-only head:", red_sha, flush=True)

sys.path.insert(0, str(TEST.parent.resolve()))
import test_validate_remote_desktop_prompt_routing as t
for name in (
    "test_mutable_meta_branch_selector_fails",
    "test_surface_mutable_meta_tag_selector_fails",
    "test_abbreviated_meta_selector_fails",
    "test_surface_uppercase_meta_hash_selector_fails",
    "test_operations_through_host_connector_authorization_outside_section_fails",
    "test_surface_requests_via_tool_authorization_outside_section_fails",
):
    try:
        getattr(t, name)()
    except AssertionError as exc:
        print(f"EXPECTED RED {name}: {exc}", flush=True)
    else:
        raise SystemExit(f"UNEXPECTED GREEN before validator repair: {name}")

run("git", "push", "origin", f"HEAD:{BRANCH}")

source = VALIDATOR.read_text()
old_meta = '''META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\\.json"
)
ANGLE_BRACKET_META_ROUTING_COORDINATE_RE = re.compile(
    r"<(Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\\.json)>",
    re.IGNORECASE,
)
'''
new_meta = '''META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[^\\s`<>:]+:ecosystem/agent-execution-routing-policy\\.json",
    re.IGNORECASE,
)
ANGLE_BRACKET_META_ROUTING_COORDINATE_RE = re.compile(
    r"<(Oteryn/Oteryn@[^\\s`<>:]+:ecosystem/agent-execution-routing-policy\\.json)>",
    re.IGNORECASE,
)
'''
assert source.count(old_meta) == 1
source = source.replace(old_meta, new_meta, 1)

call_first_anchor = r'''    re.compile(
        r"(?=.*\b(?:calls?|operations?|requests?|invocations?)\s+to\s+(?:the\s+)?(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic)\s+approval|preapproval|approval\s+by\s+default)\b)",
        re.IGNORECASE,
    ),
'''
through_pattern = call_first_anchor + r'''    re.compile(
        r"(?=.*\b(?:calls?|operations?|requests?|invocations?)\b.{0,100}\b(?:through|via)\b.{0,100}\b(?:host\s+)?(?:connectors?|tools?)\b)"
        r"(?=.*\b(?:authori[sz]ation|(?:pre)?authori[sz]ed|(?:pre)?approved|host[- ]exception|exception|per[- ]action|exempt|without|"
        r"allow(?:ed|ance)?|permit(?:ted|s)?|require(?:d|s)?|need(?:s)?\s+no|(?:blanket|standing|automatic)\s+approval|preapproval|approval\s+by\s+default)\b)",
        re.IGNORECASE,
    ),
'''
assert source.count(call_first_anchor) == 1
source = source.replace(call_first_anchor, through_pattern, 1)

root_old = '''        coordinates = re.findall(
            r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\\.json",
            root_text,
        )
'''
root_new = '''        coordinates = META_ROUTING_COORDINATE_RE.findall(_normalize_policy_text(_operative_text(root_text)))
'''
assert source.count(root_old) == 1
source = source.replace(root_old, root_new, 1)

VALIDATOR.write_text(source)

run(sys.executable, str(TEST))
run(sys.executable, str(VALIDATOR))
run(sys.executable, "tools/agents/validate_governance.py")
run(sys.executable, "tools/repository/validate_repository_policy.py")
run("git", "diff", "--check")
run("git", "add", str(VALIDATOR))
run("git", "diff", "--cached", "--check")
run("git", "commit", "-m", "fix(agents): reject mutable selectors and through-connector authority")
green_sha = out("git", "rev-parse", "HEAD")
run("git", "push", "origin", f"HEAD:{BRANCH}")
print("GREEN implementation head:", green_sha, flush=True)
