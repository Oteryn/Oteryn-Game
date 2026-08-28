#!/usr/bin/env python3
"""Focused regression tests for the Game Remote Desktop prompt-routing gate."""
from __future__ import annotations

from validate_remote_desktop_prompt_routing import (
    APPROVED_SURFACE_OUTSIDE_ROUTING_PARAGRAPHS,
    CANONICAL_PROMPT_SECTION,
    CANONICAL_ROUTING_ADJACENT_SECTIONS,
    CANONICAL_SURFACE_SECTIONS,
    validate_reusable_prompt_text,
    validate_surface_text,
)

META_SHA = "e002fc7532188e73a0f495da3e20710541ed50e0"
SURFACE_SECTION = CANONICAL_SURFACE_SECTIONS["AGENTS.md"]
PROMPT_EVAL_PATH = "docs/agents/PROMPT_EVAL_STANDARD.md"
PROMPT_EVAL_GATES = CANONICAL_ROUTING_ADJACENT_SECTIONS[PROMPT_EVAL_PATH]["## Gates"]


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


def assert_surface_path_pass(path: str, text: str) -> None:
    errors: list[str] = []
    validate_surface_text(path, text, errors)
    if errors:
        raise AssertionError(f"expected surface PASS, got: {errors}")


def assert_surface_path_fail(path: str, text: str, needle: str) -> None:
    errors: list[str] = []
    validate_surface_text(path, text, errors)
    if not any(needle in error for error in errors):
        raise AssertionError(f"expected surface error containing {needle!r}, got: {errors}")


def assert_surface_pass(text: str) -> None:
    assert_surface_path_pass("AGENTS.md", text)


def assert_surface_fail(text: str, needle: str) -> None:
    assert_surface_path_fail("AGENTS.md", text, needle)


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


def test_fenced_prompt_section_does_not_count() -> None:
    text = "# Prompt\n\n```markdown\n" + CANONICAL_PROMPT_SECTION + "\n## End sample\n```\n"
    assert_fail(text, "must contain exactly one")


def test_fenced_surface_section_does_not_count() -> None:
    text = "# Surface\n\n~~~markdown\n" + SURFACE_SECTION + "\n## End sample\n~~~\n"
    assert_surface_fail(text, "must contain exactly one")


def test_commented_prompt_section_does_not_count() -> None:
    text = "# Prompt\n\n<!--\n" + CANONICAL_PROMPT_SECTION + "\n## End sample\n-->\n"
    assert_fail(text, "must contain exactly one")


def test_real_prompt_section_after_fenced_example_passes() -> None:
    text = (
        "# Prompt\n\n```markdown\n"
        + CANONICAL_PROMPT_SECTION
        + "\n## End sample\n```\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_pass(text)


def test_real_prompt_section_after_commented_example_passes() -> None:
    text = (
        "# Prompt\n\n<!--\n"
        + CANONICAL_PROMPT_SECTION
        + "\n## End sample\n-->\n\n"
        + CANONICAL_PROMPT_SECTION
        + "\n"
    )
    assert_pass(text)


def test_exact_canonical_surface_section_passes() -> None:
    assert_surface_pass("# Surface\n\nordinary repository text\n\n" + SURFACE_SECTION + "\n")


def test_surface_section_may_name_its_heading_inline() -> None:
    path = "docs/agents/PROMPTING_STANDARD.md"
    section = CANONICAL_SURFACE_SECTIONS[path]
    assert_surface_path_pass(path, "# Surface\n\n" + section + "\n")


def test_surface_approved_routing_bullet_inside_list_passes() -> None:
    section = CANONICAL_SURFACE_SECTIONS[PROMPT_EVAL_PATH]
    text = "# Surface\n\n" + PROMPT_EVAL_GATES + "\n\n" + section + "\n"
    assert_surface_path_pass(PROMPT_EVAL_PATH, text)


def test_surface_adjacent_no_authorization_bullet_fails() -> None:
    section = CANONICAL_SURFACE_SECTIONS[PROMPT_EVAL_PATH]
    gates = PROMPT_EVAL_GATES + "\n- Direct invocations need no authorization."
    text = "# Surface\n\n" + gates + "\n\n" + section + "\n"
    assert_surface_path_fail(PROMPT_EVAL_PATH, text, "routing list")


def test_surface_adjacent_routine_host_git_bullet_fails() -> None:
    section = CANONICAL_SURFACE_SECTIONS[PROMPT_EVAL_PATH]
    gates = PROMPT_EVAL_GATES + "\n- Routine Git inspection through the host is permitted."
    text = "# Surface\n\n" + gates + "\n\n" + section + "\n"
    assert_surface_path_fail(PROMPT_EVAL_PATH, text, "routing list")


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
