#!/usr/bin/env python3
"""Focused Codex regressions for the Game Remote Desktop prompt-routing gate."""
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


def assert_prompt_suffix_fail(policy: str, needle: str) -> None:
    errors: list[str] = []
    text = "# Prompt\n\n" + CANONICAL_PROMPT_SECTION + "\n\n" + policy + "\n"
    validate_reusable_prompt_text("prompt.md", text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected suffix error containing {needle!r}, got: {errors}")


def assert_surface_suffix_fail(policy: str, needle: str) -> None:
    errors: list[str] = []
    text = "# Surface\n\n" + SURFACE_SECTION + "\n\n" + policy + "\n"
    validate_surface_text("AGENTS.md", text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected suffix surface error containing {needle!r}, got: {errors}")


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


def test_connector_actions_passive_permission_granted_by_default_fails() -> None:
    assert_prompt_fail(
        "Permission is granted by default for connector actions.",
        "Remote Desktop policy text outside canonical section",
    )


def test_connector_actions_passive_permission_automatically_granted_fails() -> None:
    assert_prompt_fail(
        "Permission is automatically granted for connector actions.",
        "Remote Desktop policy text outside canonical section",
    )


def test_named_host_actions_passive_approval_automatically_given_fails() -> None:
    assert_surface_fail(
        "Approval is automatically given for filesystem operations.",
        "Remote Desktop policy text outside canonical section",
    )


def test_named_host_operations_permission_by_default_fails() -> None:
    assert_surface_fail(
        "Filesystem operations have permission by default.",
        "Remote Desktop policy text outside canonical section",
    )


def test_named_host_operations_passive_permission_granted_by_default_fails() -> None:
    assert_surface_fail(
        "Permission is granted by default for filesystem operations.",
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


def test_direct_connectors_default_permission_fails() -> None:
    assert_prompt_fail(
        "Direct connectors have default permission.",
        "Remote Desktop policy text outside canonical section",
    )


def test_by_default_direct_connectors_permission_fails() -> None:
    assert_surface_fail(
        "By default, direct connectors have permission.",
        "Remote Desktop policy text outside canonical section",
    )


def test_provider_enforces_per_action_claim_fails() -> None:
    assert_prompt_fail(
        "The provider enforces every per-action decision.",
        "Remote Desktop policy text outside canonical section",
    )


def test_providers_deny_per_action_claim_fails() -> None:
    assert_surface_fail(
        "Providers deny calls lacking per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_connector_stops_per_action_claim_fails() -> None:
    assert_prompt_fail(
        "The connector stops every call lacking per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_providers_refuse_per_action_claim_fails() -> None:
    assert_surface_fail(
        "Providers refuse calls lacking per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_calls_passively_stopped_by_connector_fails() -> None:
    assert_prompt_fail(
        "Calls lacking per-action authorization are stopped by the connector.",
        "Remote Desktop policy text outside canonical section",
    )


def test_requests_passively_refused_by_provider_fails() -> None:
    assert_surface_fail(
        "Requests lacking per-action authorization are refused by providers.",
        "Remote Desktop policy text outside canonical section",
    )


def test_github_explicit_default_port_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://github.com:443/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_github_explicit_default_port_passes() -> None:
    assert_prompt_pass(
        f"Reference https://github.com:443/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_connector_actions_reordered_by_default_permission_fails() -> None:
    assert_prompt_fail(
        "Connector actions, by default, have permission.",
        "Remote Desktop policy text outside canonical section",
    )


def test_named_host_operations_reordered_by_default_permission_fails() -> None:
    assert_surface_fail(
        "Filesystem operations, by default, have permission.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_calls_automatically_authorized_fails() -> None:
    assert_prompt_fail(
        "Ping calls are automatically authorized.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_requests_blanket_approval_fails() -> None:
    assert_surface_fail(
        "Ping requests have blanket approval.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_automatically_authorized_fails() -> None:
    assert_prompt_fail(
        "Ping is automatically authorized.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_preapproved_fails() -> None:
    assert_surface_fail(
        "Ping is preapproved.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_requires_no_per_action_authorization_fails() -> None:
    assert_prompt_fail(
        "Ping requires no per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_needs_no_host_exception_fails() -> None:
    assert_surface_fail(
        "Ping needs no host exception.",
        "Remote Desktop policy text outside canonical section",
    )


def test_connector_prevents_per_action_claim_fails() -> None:
    assert_prompt_fail(
        "The connector prevents every call lacking per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_providers_prevent_per_action_claim_fails() -> None:
    assert_surface_fail(
        "Providers prevent calls lacking per-action authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_raw_githubusercontent_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_github_raw_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use https://github.com/Oteryn/Oteryn/raw/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_raw_githubusercontent_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_github_raw_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://github.com/Oteryn/Oteryn/raw/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_github_contents_api_mutable_ref_fails() -> None:
    assert_prompt_fail(
        "Use https://api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_github_contents_api_surface_mutable_ref_fails() -> None:
    assert_surface_fail(
        "Use https://api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=feature for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_github_contents_api_percent_encoded_path_mutable_ref_fails() -> None:
    assert_prompt_fail(
        "Use https://api.github.com/repos/Oteryn/Oteryn/contents/%65cosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_github_contents_api_percent_encoded_path_surface_mutable_ref_fails() -> None:
    assert_surface_fail(
        "Use https://api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/%61gent-execution-routing-policy.json?ref=feature for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_github_contents_api_ref_passes() -> None:
    assert_prompt_pass(
        f"Reference https://api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref={META_SHA} for the pinned policy."
    )


def test_pinned_github_contents_api_percent_encoded_path_passes() -> None:
    assert_prompt_pass(
        f"Reference https://api.github.com/repos/Oteryn/Oteryn/contents/%65cosystem/agent-execution-routing-policy.json?ref={META_SHA} for the pinned policy."
    )


def test_invalid_backtick_fence_does_not_hide_prompt_authority() -> None:
    assert_prompt_suffix_fail(
        "## Notes\n\n``` invalid`\nUse Remote Desktop for routine Git inspection.\n```",
        "Remote Desktop policy text outside canonical section",
    )


def test_invalid_backtick_fence_does_not_hide_surface_authority() -> None:
    assert_surface_suffix_fail(
        "## Notes\n\n``` invalid`\nUse Remote Desktop for routine Git inspection.\n```",
        "Remote Desktop policy text outside canonical section",
    )


def test_direct_connectors_passive_permission_automatically_granted_fails() -> None:
    assert_prompt_fail(
        "Permission is automatically granted for direct connectors.",
        "Remote Desktop policy text outside canonical section",
    )


def test_direct_tools_passive_approval_given_by_default_fails() -> None:
    assert_surface_fail(
        "Approval is given by default for direct tools.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_standing_authorization_fails() -> None:
    assert_prompt_fail(
        "Ping has standing authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_blanket_authorization_fails() -> None:
    assert_surface_fail(
        "Ping has blanket authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_calls_passively_stopped_at_connector_fails() -> None:
    assert_prompt_fail(
        "Calls lacking per-action authorization are stopped at the connector.",
        "Remote Desktop policy text outside canonical section",
    )


def test_requests_passively_refused_at_provider_fails() -> None:
    assert_surface_fail(
        "Requests lacking per-action authorization are refused at the provider.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_passive_automatic_authorization_grant_fails() -> None:
    assert_prompt_fail(
        "Authorization is automatically granted for ping.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_passive_automatic_permission_grant_fails() -> None:
    assert_surface_fail(
        "Permission is automatically granted for ping.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_authorized_by_default_fails() -> None:
    assert_prompt_fail(
        "Ping is authorized by default.",
        "Remote Desktop policy text outside canonical section",
    )


def test_calls_active_stop_at_connector_fails() -> None:
    assert_prompt_fail(
        "Calls lacking per-action authorization stop at the connector.",
        "Remote Desktop policy text outside canonical section",
    )


def test_calls_get_stopped_at_connector_fails() -> None:
    assert_surface_fail(
        "Calls lacking per-action authorization get stopped at the connector.",
        "Remote Desktop policy text outside canonical section",
    )


def test_github_trailing_dot_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://github.com./Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_github_trailing_dot_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://github.com./Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_ping_authorization_granted_automatically_fails() -> None:
    assert_prompt_fail(
        "Authorization is granted automatically for ping.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_automatically_granted_authorization_fails() -> None:
    assert_surface_fail(
        "Ping is automatically granted authorization.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_authorization_is_automatic_fails() -> None:
    assert_prompt_fail(
        "Ping authorization is automatic.",
        "Remote Desktop policy text outside canonical section",
    )


def test_ping_permission_is_automatic_fails() -> None:
    assert_surface_fail(
        "Ping permission is automatic.",
        "Remote Desktop policy text outside canonical section",
    )


def test_calls_stop_within_connector_fails() -> None:
    assert_prompt_fail(
        "Calls lacking per-action authorization stop within the connector.",
        "Remote Desktop policy text outside canonical section",
    )


def test_calls_refused_inside_router_fails() -> None:
    assert_surface_fail(
        "Calls lacking per-action authorization are refused inside the router.",
        "Remote Desktop policy text outside canonical section",
    )


def test_requests_get_stopped_within_provider_fails() -> None:
    assert_prompt_fail(
        "Requests lacking per-action authorization get stopped within the provider.",
        "Remote Desktop policy text outside canonical section",
    )


def test_invocations_refuse_inside_transport_fails() -> None:
    assert_surface_fail(
        "Invocations lacking per-action authorization refuse inside the transport.",
        "Remote Desktop policy text outside canonical section",
    )


def test_github_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://reader@github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_raw_github_userinfo_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use https://reader@raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_contents_api_userinfo_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use https://reader@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main for host exceptions.",
        "stale META execution-routing coordinate",
    )


def test_pinned_github_userinfo_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://reader@github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_pinned_raw_userinfo_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference https://reader@raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
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