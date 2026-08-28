#!/usr/bin/env python3
"""Focused regression tests for the Game Remote Desktop prompt-routing gate."""
from __future__ import annotations

from validate_remote_desktop_prompt_routing import (
    CANONICAL_PROMPT_SECTION,
    validate_reusable_prompt_text,
)


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


def main() -> int:
    tests = [value for name, value in sorted(globals().items()) if name.startswith("test_") and callable(value)]
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Remote Desktop prompt-routing regression tests PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
