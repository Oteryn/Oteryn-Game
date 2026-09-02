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


def test_two_github_urls_mutable_then_pinned_userinfo_fails() -> None:
    assert_prompt_fail(
        f"Use https://github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json first, then https://reader@github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json.",
        STALE_ERROR,
    )


def test_two_raw_urls_mutable_then_pinned_userinfo_fails() -> None:
    assert_surface_fail(
        f"Use https://raw.githubusercontent.com/Oteryn/Oteryn/main/ecosystem/agent-execution-routing-policy.json first, then https://reader@raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ecosystem/agent-execution-routing-policy.json.",
        STALE_ERROR,
    )


def test_two_contents_urls_mutable_then_pinned_userinfo_fails() -> None:
    assert_prompt_fail(
        f"Use https://api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref=main first, then https://reader@api.github.com/repos/Oteryn/Oteryn/contents/ecosystem/agent-execution-routing-policy.json?ref={META_SHA}.",
        STALE_ERROR,
    )


def test_bare_filesystem_prefixed_authority_fails() -> None:
    assert_prompt_fail("Filesystem has automatic authority.", POLICY_ERROR)


def test_bare_process_prefixed_authority_fails() -> None:
    assert_surface_fail("Process has standing authority.", POLICY_ERROR)


def test_bare_session_prefixed_authority_fails() -> None:
    assert_prompt_fail("Session has default authority.", POLICY_ERROR)


def test_ping_authorized_automatically_fails() -> None:
    assert_prompt_fail("Ping is authorized automatically.", POLICY_ERROR)


def test_ping_approved_automatically_fails() -> None:
    assert_surface_fail("Ping is approved automatically.", POLICY_ERROR)


def test_connector_drops_calls_fails() -> None:
    assert_prompt_fail("The connector drops calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_discarded_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are discarded by the router.", POLICY_ERROR)


def test_transport_filters_out_calls_fails() -> None:
    assert_prompt_fail("The transport filters out calls without per-action authorization.", POLICY_ERROR)


def test_bare_filesystem_passive_authority_grant_fails() -> None:
    assert_prompt_fail("Filesystem is granted automatic authority.", POLICY_ERROR)


def test_bare_filesystem_subject_last_passive_authority_grant_fails() -> None:
    assert_surface_fail("Automatic authority is granted to filesystem.", POLICY_ERROR)


def test_connector_suppresses_calls_fails() -> None:
    assert_prompt_fail("The connector suppresses calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_suppressed_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are suppressed by the router.", POLICY_ERROR)


def test_ping_needs_no_permission_fails() -> None:
    assert_prompt_fail("Ping needs no permission.", POLICY_ERROR)


def test_ping_requires_no_approval_fails() -> None:
    assert_surface_fail("Ping requires no approval.", POLICY_ERROR)


def test_connector_ignores_calls_fails() -> None:
    assert_prompt_fail("The connector ignores calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_ignored_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are ignored by the router.", POLICY_ERROR)


def test_ping_does_not_need_permission_fails() -> None:
    assert_prompt_fail("Ping does not need permission.", POLICY_ERROR)


def test_ping_does_not_need_approval_fails() -> None:
    assert_surface_fail("Ping does not need approval.", POLICY_ERROR)


def test_connector_skips_calls_fails() -> None:
    assert_prompt_fail("The connector skips calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_skipped_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are skipped by the router.", POLICY_ERROR)


def test_ping_requires_no_per_action_decision_fails() -> None:
    assert_prompt_fail("Ping requires no per-action decision.", POLICY_ERROR)


def test_filesystem_automatically_authorized_fails() -> None:
    assert_surface_fail("Filesystem is automatically authorized.", POLICY_ERROR)


def test_filesystem_authorized_automatically_fails() -> None:
    assert_prompt_fail("Filesystem is authorized automatically.", POLICY_ERROR)


def test_search_approved_automatically_fails() -> None:
    assert_surface_fail("Search is approved automatically.", POLICY_ERROR)


def test_filesystem_requires_no_per_action_decision_fails() -> None:
    assert_prompt_fail("Filesystem requires no per-action decision.", POLICY_ERROR)


def test_search_needs_no_approval_fails() -> None:
    assert_surface_fail("Search needs no approval.", POLICY_ERROR)


def test_process_does_not_need_permission_fails() -> None:
    assert_prompt_fail("Process does not need permission.", POLICY_ERROR)


def test_connector_cancels_calls_fails() -> None:
    assert_prompt_fail("The connector cancels calls lacking per-action authorization.", POLICY_ERROR)


def test_router_intercepts_calls_fails() -> None:
    assert_surface_fail("The router intercepts calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_cancelled_by_connector_fails() -> None:
    assert_prompt_fail("Calls lacking per-action authorization are cancelled by the connector.", POLICY_ERROR)


def test_calls_intercepted_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are intercepted by the router.", POLICY_ERROR)


def test_schemeless_www_github_mutable_selector_fails() -> None:
    assert_prompt_fail(
        "Use www.github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_schemeless_github_mutable_selector_fails() -> None:
    assert_surface_fail(
        "Use github.com/Oteryn/Oteryn/blob/main/ecosystem/agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_schemeless_pinned_github_selector_passes() -> None:
    assert_prompt_pass(
        f"Reference www.github.com/Oteryn/Oteryn/blob/{META_SHA}/ecosystem/agent-execution-routing-policy.json for the pinned policy."
    )


def test_filesystem_always_authorized_fails() -> None:
    assert_prompt_fail("Filesystem is always authorized.", POLICY_ERROR)


def test_ping_always_authorized_fails() -> None:
    assert_surface_fail("Ping is always authorized.", POLICY_ERROR)


def test_connector_quarantines_calls_fails() -> None:
    assert_prompt_fail("The connector quarantines calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_quarantined_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are quarantined by the router.", POLICY_ERROR)


def test_filesystem_without_permission_fails() -> None:
    assert_prompt_fail("Filesystem may be used without permission.", POLICY_ERROR)


def test_search_without_per_action_decision_fails() -> None:
    assert_surface_fail("Search can run without a per-action decision.", POLICY_ERROR)


def test_connector_declines_calls_fails() -> None:
    assert_prompt_fail("The connector declines calls lacking per-action authorization.", POLICY_ERROR)


def test_calls_declined_by_router_fails() -> None:
    assert_surface_fail("Calls lacking per-action authorization are declined by the router.", POLICY_ERROR)


def test_github_backslash_mutable_selector_fails() -> None:
    assert_prompt_fail(
        r"Use https://github.com\Oteryn\Oteryn\blob\main\ecosystem\agent-execution-routing-policy.json for host exceptions.",
        STALE_ERROR,
    )


def test_github_backslash_pinned_selector_passes() -> None:
    assert_prompt_pass(
        rf"Reference https://github.com\Oteryn\Oteryn\blob\{META_SHA}\ecosystem\agent-execution-routing-policy.json for the pinned policy."
    )


def test_semantic_blanket_authority_matrix_fails() -> None:
    actions = ("Filesystem", "Search", "Process", "Session", "Terminal", "History", "Ping")
    forms = (
        "{action} retains reusable authority.",
        "Reusable authority is retained by {action}.",
        "By default, {action} remains permitted.",
        "Standing permission attaches to {action}.",
    )
    for action in actions:
        for form in forms:
            assert_prompt_fail(form.format(action=action), POLICY_ERROR)


def test_semantic_fresh_decision_exemption_matrix_fails() -> None:
    actions = ("Filesystem", "Search", "Process", "Session", "Terminal", "History", "Ping")
    forms = (
        "{action} proceeds without a fresh per-action decision.",
        "Without a fresh exact approval, {action} may proceed.",
        "No fresh permission check is required before {action}.",
        "{action} is exempt from checking per-action authorization.",
    )
    for action in actions:
        for form in forms:
            assert_prompt_fail(form.format(action=action), POLICY_ERROR)


def test_semantic_provider_enforcement_matrix_fails() -> None:
    providers = ("connector", "router", "provider", "transport")
    forms = (
        "The {provider} performs rejection of calls lacking per-action authorization.",
        "Call denial for missing per-action approval occurs at the {provider}.",
        "The {provider} applies suppression to requests missing per-action authorization.",
        "Per-action authorization failures lead to interception of calls at the {provider}.",
    )
    for provider in providers:
        for form in forms:
            assert_surface_fail(form.format(provider=provider), POLICY_ERROR)


def test_structured_github_coordinate_normalization_matrix() -> None:
    mutable = (
        "https://github.com/Oteryn/Oteryn/blob/main/ignored/../ecosystem/agent-execution-routing-policy.json",
        "github.com/Oteryn/Oteryn/blob/main/a/%2e%2e/ecosystem/agent-execution-routing-policy.json",
        "https://raw.githubusercontent.com/Oteryn/Oteryn/main/ignored/../ecosystem/agent-execution-routing-policy.json",
        "https://api.github.com/repos/Oteryn/Oteryn/contents/ignored/../ecosystem/agent-execution-routing-policy.json?ref=main",
    )
    for url in mutable:
        assert_prompt_fail(f"Use {url} for host exceptions.", STALE_ERROR)

    pinned = (
        f"https://github.com/Oteryn/Oteryn/blob/{META_SHA}/ignored/../ecosystem/agent-execution-routing-policy.json",
        f"github.com/Oteryn/Oteryn/blob/{META_SHA}/a/%2e%2e/ecosystem/agent-execution-routing-policy.json",
        f"https://raw.githubusercontent.com/Oteryn/Oteryn/{META_SHA}/ignored/../ecosystem/agent-execution-routing-policy.json",
        f"https://api.github.com/repos/Oteryn/Oteryn/contents/ignored/../ecosystem/agent-execution-routing-policy.json?ref={META_SHA}",
    )
    for url in pinned:
        assert_prompt_pass(f"Reference {url} for the pinned policy.")


def test_semantic_positive_controls_pass() -> None:
    controls = (
        "Filesystem paths are documented for repository layout only.",
        "The connector does not enforce per-action authorization; repository governance does.",
        "Calls are not blocked by the transport; repository governance performs the decision.",
        "No standing authority is granted to filesystem operations.",
        "Filesystem operations cannot proceed without a fresh per-action decision.",
    )
    for text in controls:
        assert_prompt_pass(text)


def test_mixed_subject_denial_does_not_hide_blanket_authority_fails() -> None:
    assert_prompt_fail(
        "Ping has no authority, but filesystem is always authorized.",
        POLICY_ERROR,
    )


def test_unrelated_restriction_does_not_hide_fresh_decision_exemption_fails() -> None:
    assert_surface_fail(
        "Ping must not run, but filesystem may run without permission.",
        POLICY_ERROR,
    )


def test_contrastive_negation_does_not_hide_provider_enforcement_fails() -> None:
    assert_prompt_fail(
        "The connector does not merely log but blocks calls lacking per-action authorization.",
        POLICY_ERROR,
    )


def test_provider_implements_request_logging_passes() -> None:
    assert_prompt_pass("The connector implements per-action request logging.")


def test_provider_guarantees_request_auditability_passes() -> None:
    assert_prompt_pass("The connector guarantees per-action request auditability.")


def test_provider_implements_authorization_gate_still_fails() -> None:
    assert_surface_fail(
        "The connector implements the per-action authorization gate.",
        POLICY_ERROR,
    )


def test_provider_guarantees_authorization_decision_still_fails() -> None:
    assert_prompt_fail(
        "The connector guarantees every per-action authorization decision.",
        POLICY_ERROR,
    )


def test_relation_scoped_legitimate_negatives_pass() -> None:
    controls = (
        "Ping has no authority, and filesystem has no standing authority.",
        "Filesystem must not run without permission.",
        "The connector does not block calls; it only logs per-action request outcomes.",
    )
    for text in controls:
        assert_prompt_pass(text)


def test_coordinated_subject_denial_does_not_hide_blanket_authority_fails() -> None:
    assert_prompt_fail(
        "Ping has no authority, and filesystem is always authorized.",
        POLICY_ERROR,
    )


def test_coordinated_restriction_does_not_hide_fresh_decision_exemption_fails() -> None:
    assert_surface_fail(
        "Ping must not run, and filesystem may run without permission.",
        POLICY_ERROR,
    )


def test_provider_anaphora_after_semicolon_preserves_enforcement_fails() -> None:
    assert_prompt_fail(
        "The connector does not merely log; instead, it blocks calls lacking per-action authorization.",
        POLICY_ERROR,
    )


def test_physical_provider_request_logging_passes() -> None:
    assert_prompt_pass("The connector physically implements per-action request logging.")


def test_coordinated_legitimate_restrictions_pass() -> None:
    assert_prompt_pass(
        "Ping must not run, and filesystem must not run without permission."
    )


def test_provider_anaphora_observability_only_passes() -> None:
    assert_prompt_pass(
        "The connector logs requests; instead, it records per-action audit metadata."
    )


def test_concessive_clause_subject_binding_matrix_fails() -> None:
    leaders = ("Although", "Though", "Even though", "Whereas", "While")
    for leader in leaders:
        assert_prompt_fail(
            f"{leader} ping has no authority, filesystem is always authorized.",
            POLICY_ERROR,
        )
        assert_surface_fail(
            f"{leader} ping must not run, filesystem may run without permission.",
            POLICY_ERROR,
        )
        assert_prompt_fail(
            f"{leader} search has no standing authority, process is approved automatically.",
            POLICY_ERROR,
        )
        assert_surface_fail(
            f"{leader} session is forbidden, search may run without a fresh per-action decision.",
            POLICY_ERROR,
        )


def test_concessive_clause_legitimate_restrictions_pass() -> None:
    leaders = ("Although", "Though", "Even though", "Whereas", "While")
    for leader in leaders:
        assert_prompt_pass(
            f"{leader} ping has no authority, filesystem has no standing authority."
        )
        assert_prompt_pass(
            f"{leader} ping must not run, filesystem must not run without permission."
        )


def test_provider_contrastive_anaphora_matrix_fails() -> None:
    connectors = (
        ("but", ""),
        ("however", ","),
        ("yet", ""),
        ("whereas", ""),
        ("instead", ","),
        ("rather", ","),
    )
    for connector, punctuation in connectors:
        assert_prompt_fail(
            f"The connector merely logs requests; {connector}{punctuation} it blocks calls lacking per-action authorization.",
            POLICY_ERROR,
        )


def test_provider_contrastive_anaphora_observability_controls_pass() -> None:
    connectors = (
        ("but", ""),
        ("however", ","),
        ("yet", ""),
        ("whereas", ""),
        ("instead", ","),
        ("rather", ","),
    )
    for connector, punctuation in connectors:
        assert_prompt_pass(
            f"The connector logs requests; {connector}{punctuation} it records per-action audit metadata."
        )


def test_physical_effect_requires_enforcement_target_matrix() -> None:
    legitimate = (
        "The connector physically filters request logs for per-action auditability.",
        "The router physically suppresses audit records for per-action reporting.",
        "The transport physically drops telemetry records after per-action diagnostics.",
    )
    for text in legitimate:
        assert_prompt_pass(text)

    enforcement = (
        "The connector physically filters calls lacking per-action authorization.",
        "The router physically suppresses requests missing per-action approval.",
        "The transport physically drops invocations without per-action permission.",
        "The connector physically enforces the per-action authorization gate.",
    )
    for text in enforcement:
        assert_surface_fail(text, POLICY_ERROR)


def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    failures: list[tuple[str, str]] = []
    for test in tests:
        try:
            test()
        except AssertionError as exc:
            failures.append((test.__name__, str(exc)))
            print(f"FAIL {test.__name__}: {exc}")
        else:
            print(f"PASS {test.__name__}")
    if failures:
        print(f"Codex round-12+ regressions FAIL: {len(failures)} of {len(tests)}")
        return 1
    print(f"Codex round-12+ regressions PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
