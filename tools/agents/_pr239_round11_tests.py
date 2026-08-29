#!/usr/bin/env python3
from pathlib import Path

path = Path("tools/agents/test_validate_remote_desktop_prompt_routing_codex_regressions.py")
text = path.read_text(encoding="utf-8")
marker = "\ndef main() -> int:\n"
if text.count(marker) != 1:
    raise SystemExit("expected exactly one durable-regression main marker")

new_tests = r'''


def test_direct_connectors_automatic_authority_fails() -> None:
    assert_prompt_fail(
        "Direct connectors have automatic authority.",
        "Remote Desktop policy text outside canonical section",
    )


def test_connector_actions_standing_authority_fails() -> None:
    assert_surface_fail(
        "Connector actions have standing authority.",
        "Remote Desktop policy text outside canonical section",
    )


def test_filesystem_operations_default_authority_fails() -> None:
    assert_prompt_fail(
        "Filesystem operations have default authority.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_granted_authority_automatically_fails() -> None:
    assert_prompt_fail(
        "Ping is granted authority automatically.",
        "Remote Desktop policy text outside canonical section",
    )


def test_authority_granted_to_ping_automatically_fails() -> None:
    assert_surface_fail(
        "Authority is granted to ping automatically.",
        "Remote Desktop policy text outside canonical section",
    )


def test_github_percent_encoded_at_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://reader%40x@github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_raw_percent_encoded_at_userinfo_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use https://reader%40x@raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_contents_percent_encoded_at_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://reader%40x@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_github_percent_encoded_at_userinfo_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://reader%40x@github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_raw_percent_encoded_at_userinfo_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://reader%40x@raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_contents_percent_encoded_at_userinfo_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://reader%40x@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref={META_SHA} for the pinned policy."
    )
'''

for name in (
    "test_direct_connectors_automatic_authority_fails",
    "test_connector_actions_standing_authority_fails",
    "test_filesystem_operations_default_authority_fails",
    "test_ping_granted_authority_automatically_fails",
    "test_authority_granted_to_ping_automatically_fails",
    "test_github_percent_encoded_at_userinfo_mutable_selector_fails",
    "test_raw_percent_encoded_at_userinfo_mutable_selector_fails",
    "test_contents_percent_encoded_at_userinfo_mutable_selector_fails",
):
    if name in text:
        raise SystemExit(f"regression already present: {name}")

path.write_text(text.replace(marker, new_tests + marker), encoding="utf-8")
