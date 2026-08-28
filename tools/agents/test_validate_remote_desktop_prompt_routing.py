#!/usr/bin/env python3
"""Focused regression tests for the Game Remote Desktop prompt-routing gate."""
from __future__ import annotations

from validate_remote_desktop_prompt_routing import (
    CANONICAL_PROMPT_SECTION,
    validate_reusable_prompt_text,
    validate_surface_text,
)

META_SHA = "e002fc7532188e73a0f495da3e20710541ed50e0"
SURFACE_SECTION = f"""## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve this Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@{META_SHA}`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. Game cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.

This provider binding is repository/prompt enforcement only. It MUST NOT be described as connector/router physical enforcement unless the actual Remote Desktop transport has a verified fail-closed hook consuming the same per-action semantics."""


def assert_pass(text: str) -> None:
    errors: list[str] = []
    validate_reusable_prompt_text("prompt.md", text, errors)
    if errors:
        raise AssertionError(f"expected PASS, got: {errors}")


def assert_fail(text: str, needle: str) -> None:
    errors: list[str] = []
    validate_reusable_prompt_text("prompt.md", text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected error containing {needle!r}, got: {errors}")


def assert_surface_pass(text: str) -> None:
    errors: list[str] = []
    validate_surface_text("AGENTS.md", text, errors)
    if errors:
        raise AssertionError(f"expected surface PASS, got: {errors}")


def assert_surface_fail(text: str, needle: str) -> None:
    errors: list[str] = []
    validate_surface_text("AGENTS.md", text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected surface error containing {needle!r}, got: {errors}")


def test_exact_canonical_section_passes() -> None:
    assert_pass("# Prompt\n\nordinary role text\n\n" + CANONICAL_PROMPT_SECTION + "\n")


def test_modified_canonical_section_fails() -> None:
    modified = CANONICAL_PROMPT_SECTION.replace(
        "is exception-only",
        "may be used for routine repository tests",
    )
    assert_fail("# Prompt\n\n" + modified + "\n", "canonical Remote Desktop routing section must match exactly")


def test_remote_desktop_authority_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nUse Remote Desktop to inspect Git when convenient.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_direct_tool_discovery_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nTreat `ping` as ordinary capability discovery.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_physical_enforcement_claim_outside_section_fails() -> None:
    text = (
        "# Prompt\n\nRemote Desktop connector/router physical enforcement is active.\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_fail(text, "Remote Desktop policy text outside canonical section")


def test_duplicate_section_fails() -> None:
    text = CANONICAL_PROMPT_SECTION + "\n\n" + CANONICAL_PROMPT_SECTION + "\n"
    assert_fail(text, "must contain exactly one")


def test_exact_canonical_surface_section_passes() -> None:
    assert_surface_pass("# Surface\n\nordinary repository text\n\n" + SURFACE_SECTION + "\n")


def test_modified_canonical_surface_section_fails() -> None:
    modified = SURFACE_SECTION.replace(
        "repository/prompt enforcement only",
        "connector/router physical enforcement is active",
    )
    assert_surface_fail(
        "# Surface\n\nordinary repository text\n\n" + modified + "\n",
        "canonical Remote Desktop routing section must match exactly",
    )


def test_surface_remote_desktop_authority_outside_section_fails() -> None:
    text = "# Surface\n\nUse Remote Desktop to inspect Git when convenient.\n\n" + SURFACE_SECTION + "\n"
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_direct_tool_discovery_outside_section_fails() -> None:
    text = "# Surface\n\nTreat `ping` as ordinary capability discovery.\n\n" + SURFACE_SECTION + "\n"
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_physical_enforcement_claim_outside_section_fails() -> None:
    text = "# Surface\n\nRemote Desktop connector/router physical enforcement is active.\n\n" + SURFACE_SECTION + "\n"
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def test_surface_additional_restrictive_policy_outside_section_fails() -> None:
    text = "# Surface\n\nRemote Desktop remains exception-only.\n\n" + SURFACE_SECTION + "\n"
    assert_surface_fail(text, "Remote Desktop policy text outside canonical section")


def main() -> int:
    tests = [value for name, value in sorted(globals().items()) if name.startswith("test_") and callable(value)]
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Remote Desktop prompt-routing regression tests PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
