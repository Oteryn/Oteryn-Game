#!/usr/bin/env python3
"""Durable Codex round-12+ regressions for Remote Desktop prompt routing."""
from __future__ import annotations

from test_validate_remote_desktop_prompt_routing_codex_regressions import (
    META_SHA,
    assert_prompt_fail,
    assert_prompt_pass,
    assert_surface_fail,
)

POLICY_ERROR = "Remote Desktop policy text outside canonical section"
STALE_ERROR = "stale META execution-routing coordinate"


def test_direct_connectors_authority_automatically_fails() -> None:
    assert_prompt_fail("Direct connectors have authority automatically.", POLICY_ERROR)


def test_connector_actions_authority_automatically_fails() -> None:
    assert_surface_fail("Connector actions have authority automatically.", POLICY_ERROR)


def test_filesystem_operations_authority_automatically_fails() -> None:
    assert_prompt_fail("Filesystem operations have authority automatically.", POLICY_ERROR)


def test_github_percent_encoded_space_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://read%20er@github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_raw_percent_encoded_tab_userinfo_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use https://read%09er@raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_contents_percent_encoded_space_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://read%20er@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        STALE_ERROR,
    )


def test_pinned_github_percent_encoded_space_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%20er@github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_raw_percent_encoded_tab_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%09er@raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_contents_percent_encoded_space_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%20er@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref={META_SHA} for the pinned policy."
    )


def test_bare_ping_authority_automatically_fails() -> None:
    assert_prompt_fail("Ping has authority automatically.", POLICY_ERROR)


def test_bare_filesystem_authority_automatically_fails() -> None:
    assert_surface_fail("Filesystem has authority automatically.", POLICY_ERROR)


def test_github_percent_encoded_lf_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://read%0Aer@github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_raw_percent_encoded_cr_userinfo_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use https://read%0Der@raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_contents_percent_encoded_lf_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://read%0Aer@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        STALE_ERROR,
    )


def test_pinned_github_percent_encoded_lf_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%0Aer@github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_github_percent_encoded_question_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://read%3Fer@github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_raw_percent_encoded_hash_userinfo_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use https://read%23er@raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_contents_percent_encoded_gt_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://read%3Eer@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        STALE_ERROR,
    )


def test_pinned_github_percent_encoded_question_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%3Fer@github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_raw_percent_encoded_hash_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%23er@raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_contents_percent_encoded_gt_userinfo_passes() -> None:
    assert_prompt_pass(
        f"Reference https://read%3Eer@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref={META_SHA} for the pinned policy."
    )


def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Codex round-12+ regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
