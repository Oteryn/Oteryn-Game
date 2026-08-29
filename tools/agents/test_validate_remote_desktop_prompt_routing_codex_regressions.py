#!/usr/bin/env python3
"""Focused Codex regressions for the Game Remote Desktop prompt-routing gate."""
# qualification synchronize only
from __future__ import annotations

from validate_remote_desktop_prompt_routing import (
    CANONICAL_PROMPT_SECTION,
    CANONICAL_SURFACE_SECTIONS,
    META_SHA,
    validate_reusable_prompt_text,
    validate_surface_text,
)

SURFACE_SECTION = CANONICAL_SURFACE_SECTIONS["AGENTS.md"]


def assert_prompt_fail(policy: str, needle: str) -> None:
    errors: list[str] = []
    text = "# Prompt\n\n" + policy + "\n\n" + CANONICAL_PROMPT_SECTION + "\n"
    validate_reusable_prompt_text("prompt.md", text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected error containing {needle!r}, got: {errors}")


def assert_surface_fail(policy: str, needle: str) -> None:
    errors: list[str] = []
    text = "# Surface\n\n" + policy + "\n\n" + SURFACE_SECTION + "\n"
    validate_surface_text("AGENTS.md", text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected surface error containing {needle!r}, got: {errors}")


def assert_prompt_pass(policy: str) -> None:
    errors: list[str] = []
    text = "# Prompt\n\n" + policy + "\n\n" + CANONICAL_PROMPT_SECTION + "\n"
    validate_reusable_prompt_text("prompt.md", text, errors)
    if errors:
        raise AssertionError(f"expected PASS, got: {errors}")


def test_http_github_blob_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use [the META routing policy](http://github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json) for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_scheme_relative_github_tree_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use [the META routing policy](//github.com/Oteryn/Oteryn/tree/main/ecosystem/agent-execution-routing-policy.json) for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_https_github_blob_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_connector_actions_permission_by_default_fails() -> None:
    assert_prompt_fail(
        "Connector actions have permission by default.",
        "Remote Desktop policy text outside canonical section",
    )


def test_named_host_operations_permission_by_default_fails() -> None:
    assert_surface_fail(
        "Filesystem operations have permission by default.",
        "Remote Desktop policy text outside canonical section",
    )


def test_plural_router_enforcement_claim_fails() -> None:
    assert_prompt_fail(
        "Routers enforce every per-action authorization decision.",
        "Remote Desktop policy text outside canonical section",
    )


def test_nominal_connector_enforcement_claim_fails() -> None:
    assert_surface_fail(
        "The connector provides routing enforcement for per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_direct_connectors_permission_by_default_fails() -> None:
    assert_prompt_fail(
        "Direct connectors have permission by default.",
        "Remote Desktop policy text outside canonical section",
    )


def test_direct_tools_blanket_permission_fails() -> None:
    assert_surface_fail(
        "Direct tools have blanket permission.",
        "Remote Desktop policy text outside canonical section",
    )


def test_www_github_blob_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://www.github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_connector_fails_closed_enforcement_claim_fails() -> None:
    assert_prompt_fail(
        "The connector fails closed for every missing per-action decision.",
        "Remote Desktop policy text outside canonical section",
    )


def test_router_denies_enforcement_claim_fails() -> None:
    assert_surface_fail(
        "The router denies every call lacking per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ordinary_permission_blocker_prose_passes() -> None:
    assert_prompt_pass(
        "A permission denied blocker must be recorded precisely before continuing through repository-native paths."
    )


def main() -> int:
    tests = [value for name, value in sorted(globals().items()) if name.startswith("test_") and callable(value)]
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Final Codex Remote Desktop regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
