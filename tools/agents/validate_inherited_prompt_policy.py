#!/usr/bin/env python3
"""Validate lean reusable prompts against inherited Game/META execution policy.

Reusable prompts may omit duplicated global Remote Desktop policy. If they contain an
operative legacy canonical block it must remain exact. Any prompt-local Remote Desktop
policy outside that legacy block still fails closed. The authoritative root/supporting
surfaces remain validated by the existing strict canonical checks.
"""
from __future__ import annotations

import re
import sys

from validate_remote_desktop_prompt_routing import (
    CANONICAL_PROMPT_SECTION,
    ROOT,
    SECTION,
    _level2_sections,
    _validate_meta_routing_coordinates,
    _validate_outside_routing_text,
    load_lifecycle,
    reusable_prompt_paths,
    validate_surface_text,
)

INHERITED_POLICY_SURFACES = (
    "AGENTS.md",
    "docs/agents/GITHUB_ONLY_EXECUTION.md",
)

AI_REVIEW_AUTHORITY_BROADENING = (
    re.compile(
        r"\bowner[- ]funded\b.{0,80}\b(?:external\s+)?(?:ai|codex|openai)\b.{0,80}"
        r"\bwithout\s+(?:explicit\s+)?authorization\b",
        re.IGNORECASE | re.DOTALL,
    ),
    re.compile(
        r"\b(?:approval|review)\s+(?:is|becomes|shall\s+be|must\s+be)\s+"
        r"(?:an?\s+)?required\s+(?:merge\s+)?status\b",
        re.IGNORECASE,
    ),
)

MUTATION_MERGE_AUTHORITY_BROADENING = (
    re.compile(
        r"\breviewer\s+may\s+(?:commit|push|merge|implement|modify|fix)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\bapprove\s+(?:its|their|his|her)\s+own\s+(?:fixes|changes|work)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:may|can|should|must|is\s+allowed\s+to|is\s+authorized\s+to)\s+"
        r"bypass\b.{0,80}\b(?:game-gate|merge\s+queue|branch\s+protection|protections?)\b",
        re.IGNORECASE | re.DOTALL,
    ),
)


def _matches_any(patterns: tuple[re.Pattern[str], ...], text: str) -> bool:
    return any(pattern.search(text) is not None for pattern in patterns)


def _validate_inherited_authority_boundaries(path: str, text: str, errors: list[str]) -> None:
    if _matches_any(AI_REVIEW_AUTHORITY_BROADENING, text):
        errors.append(f"{path}: prompt-local AI/review authority broadening is forbidden")
    if _matches_any(MUTATION_MERGE_AUTHORITY_BROADENING, text):
        errors.append(f"{path}: prompt-local mutation/merge authority broadening is forbidden")


def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:
    _validate_meta_routing_coordinates(path, text, errors)
    _validate_inherited_authority_boundaries(path, text, errors)
    matches = [section for section in _level2_sections(text) if section[0] == SECTION]

    if not matches:
        # A fenced/commented copy of the old global block is not a valid inheritance
        # mechanism and should not be mistaken for a clean lean prompt.
        if SECTION in text:
            errors.append(f"{path}: {SECTION!r} appears but is not one operative legacy section")
            return
        _validate_outside_routing_text(path, text, set(), errors)
        return

    if len(matches) != 1:
        errors.append(f"{path}: must contain at most one operative legacy {SECTION!r} section")
        return

    _heading, start, end, section_text = matches[0]
    if section_text != CANONICAL_PROMPT_SECTION:
        errors.append(f"{path}: legacy Remote Desktop routing section must match exactly")
    outside_text = (text[:start] + "\n" + text[end:]).strip()
    _validate_outside_routing_text(path, outside_text, set(), errors)


def validate() -> list[str]:
    errors: list[str] = []

    # Prompts may inherit policy only while the inherited policy surfaces remain
    # present and exact. Deduplication must not become a safety reduction.
    for relative in INHERITED_POLICY_SURFACES:
        path = ROOT / relative
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: unable to read inherited policy surface: {exc}")
            continue
        validate_surface_text(relative, text, errors)

    lifecycle = load_lifecycle(errors)
    prompt_paths = reusable_prompt_paths(lifecycle, errors)
    for relative in prompt_paths:
        path = ROOT / relative
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: unable to read reusable prompt: {exc}")
            continue
        validate_reusable_prompt_text(relative, text, errors)

    if not errors:
        print(
            "Validated inherited Remote Desktop and review/merge authority policy for "
            f"{len(prompt_paths)} reusable prompts and {len(INHERITED_POLICY_SURFACES)} authoritative surfaces; "
            "duplicated per-prompt policy is optional."
        )
    return errors


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(f"Inherited prompt-policy validation failed with {len(errors)} error(s).", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
