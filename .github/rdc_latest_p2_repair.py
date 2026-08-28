#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys

BRANCH = "governance/remote-desktop-per-action-gate-237"
TEST = Path("tools/agents/test_validate_remote_desktop_prompt_routing.py")
VALIDATOR = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
EXPECTED_TEST_BLOB = "af498560aee13ec07ac96d95dba2c9c26f8d0130"
EXPECTED_VALIDATOR_BLOB = "5ed176f1ec3b74edc0f1ee48b810c2c97ba6c705"


def run(*args):
    print("+", " ".join(args), flush=True)
    subprocess.run(args, check=True)


def out(*args):
    return subprocess.check_output(args, text=True).strip()


assert out("git", "hash-object", str(TEST)) == EXPECTED_TEST_BLOB
assert out("git", "hash-object", str(VALIDATOR)) == EXPECTED_VALIDATOR_BLOB

tests = r'''


def test_angle_bracket_stale_meta_coordinate_fails() -> None:
    stale = "&lt;Oteryn/Oteryn@0000000000000000000000000000000000000000:ecosystem/agent-execution-routing-policy.json&gt;"
    text = "# Prompt\n\n" + stale + "\n\n" + CANONICAL_PROMPT_SECTION + "\n"
    assert_fail(text, "stale META execution-routing coordinate")


def test_surface_angle_bracket_stale_meta_coordinate_fails() -> None:
    stale = "&lt;Oteryn/Oteryn@0000000000000000000000000000000000000000:ecosystem/agent-execution-routing-policy.json&gt;"
    text = "# Surface\n\n" + stale + "\n\n" + SURFACE_SECTION + "\n"
    assert_surface_fail(text, "stale META execution-routing coordinate")


def test_blanket_approval_connector_calls_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nConnector calls have blanket approval.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_standing_approval_connector_operations_outside_section_fails() -> None:
    text = (
        "# Surface\n\nConnector operations operate under standing approval.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def test_transport_implicit_enforcement_claim_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nThe transport enforces every per-action decision.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_router_implicit_enforcement_claim_outside_section_fails() -> None:
    text = (
        "# Surface\n\nThe router enforces the per-action authorization gate.\n\n"
        + SURFACE_SECTION
        + "\n"
    )
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")
'''

test_text = TEST.read_text()
anchor = "\ndef main() -> int:"
assert anchor in test_text
assert "test_angle_bracket_stale_meta_coordinate_fails" not in test_text
TEST.write_text(test_text.replace(anchor, tests + anchor, 1))

run("git", "config", "user.name", "github-actions[bot]")
run("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
run("git", "add", str(TEST))
run("git", "diff", "--cached", "--check")
run("git", "commit", "-m", "test(agents): cover latest Codex RDC bypasses")
red_sha = out("git", "rev-parse", "HEAD")
print("RED test-only head:", red_sha, flush=True)

sys.path.insert(0, str(TEST.parent.resolve()))
import test_validate_remote_desktop_prompt_routing as t

new_tests = [
    "test_angle_bracket_stale_meta_coordinate_fails",
    "test_surface_angle_bracket_stale_meta_coordinate_fails",
    "test_blanket_approval_connector_calls_outside_section_fails",
    "test_surface_standing_approval_connector_operations_outside_section_fails",
    "test_transport_implicit_enforcement_claim_outside_section_fails",
    "test_surface_router_implicit_enforcement_claim_outside_section_fails",
]
for name in new_tests:
    try:
        getattr(t, name)()
    except AssertionError as exc:
        print(f"EXPECTED RED {name}: {exc}", flush=True)
    else:
        raise SystemExit(f"UNEXPECTED GREEN before validator repair: {name}")

run("git", "push", "origin", f"HEAD:{BRANCH}")

source = VALIDATOR.read_text()

meta_anchor = '''META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\\.json"
)
EXPECTED_META_ROUTING_COORDINATE = (
'''
meta_replacement = '''META_ROUTING_COORDINATE_RE = re.compile(
    r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\\.json"
)
ANGLE_BRACKET_META_ROUTING_COORDINATE_RE = re.compile(
    r"<(Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\\.json)>",
    re.IGNORECASE,
)
EXPECTED_META_ROUTING_COORDINATE = (
'''
assert source.count(meta_anchor) == 1
source = source.replace(meta_anchor, meta_replacement, 1)

normalize_anchor = '''    value = HTML_COMMENT_INLINE_RE.sub(" ", value)
    value = HTML_TAG_RE.sub(" ", value)
'''
normalize_replacement = '''    value = HTML_COMMENT_INLINE_RE.sub(" ", value)
    value = ANGLE_BRACKET_META_ROUTING_COORDINATE_RE.sub(r"\\1", value)
    value = HTML_TAG_RE.sub(" ", value)
'''
assert source.count(normalize_anchor) == 1
source = source.replace(normalize_anchor, normalize_replacement, 1)

pattern_anchor = '''    re.compile(r"\\bping\\b.{0,100}\\b(?:capability|discover|connector|tool|host)\\b", re.IGNORECASE),
'''
pattern_insertion = r'''    re.compile(
        r"(?=.*\b(?:connectors?|tools?)\s+(?:calls?|operations?|requests?|invocations?)\b)"
        r"(?=.*\b(?:(?:blanket|standing|automatic)\s+approval|preapproval|approval\s+by\s+default)\b)",
        re.IGNORECASE,
    ),
    re.compile(r"\bping\b.{0,100}\b(?:capability|discover|connector|tool|host)\b", re.IGNORECASE),
'''
assert source.count(pattern_anchor) == 1
source = source.replace(pattern_anchor, pattern_insertion, 1)

enforce_anchor = '''    re.compile(
        r"(?=.*\\b(?:connector|router|transport)\\b)"
        r"(?=.*\\bphysical(?:ly)?\\b)"
        r"(?=.*\\benforc\\w*\\b)",
        re.IGNORECASE,
    ),
'''
enforce_insertion = '''    re.compile(
        r"(?=.*\\b(?:connector|router|transport)\\b)"
        r"(?=.*\\benforc\\w*\\b)"
        r"(?=.*\\b(?:per[- ]action|decision|gate|routing|authori[sz]\\w*)\\b)",
        re.IGNORECASE,
    ),
''' + enforce_anchor
assert source.count(enforce_anchor) == 1
source = source.replace(enforce_anchor, enforce_insertion, 1)

VALIDATOR.write_text(source)

run(sys.executable, str(TEST))
run(sys.executable, str(VALIDATOR))
run(sys.executable, "tools/agents/validate_governance.py")
run(sys.executable, "tools/repository/validate_repository_policy.py")
run("git", "diff", "--check")
run("git", "add", str(VALIDATOR))
run("git", "diff", "--cached", "--check")
run("git", "commit", "-m", "fix(agents): close latest Codex RDC bypasses")
green_sha = out("git", "rev-parse", "HEAD")
run("git", "push", "origin", f"HEAD:{BRANCH}")
print("GREEN implementation head:", green_sha, flush=True)
