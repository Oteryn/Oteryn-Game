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
        r"\b(?:external\s+)?(?:ai|codex|openai)(?:\s+(?:service|reviewer))?\s+"
        r"(?:is|are)\s+(?:pre[- ]?)?(?:authorized|allowed|permitted)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?m)^\s*(?:[-*]\s*)?(?:please\s+)?(?:invoke|use|run|call)\s+"
        r"(?:owner[- ]funded\s+)?(?:external\s+)?(?:ai|codex|openai)\b.{0,80}"
        r"\bwithout\s+(?:explicit\s+)?authorization\b",
        re.IGNORECASE | re.DOTALL,
    ),
    re.compile(
        r"\b(?:may|can|(?:is|are)\s+(?:allowed|authorized|permitted)\s+to|"
        r"(?:has|have)\s+(?:authority|permission)\s+to)\s+"
        r"(?:invoke|use|run|call)\s+(?:owner[- ]funded\s+)?(?:external\s+)?"
        r"(?:ai|codex|openai)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:approval|review)\s+(?:is|becomes|shall\s+be|must\s+be)\s+"
        r"(?:an?\s+)?required\s+(?:merge\s+)?(?:status|check|gate)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:codex|ai(?:\s+reviewer)?|reviewer)\s+(?:approval|review)\s+"
        r"(?:is|becomes|shall\s+be|must\s+be)\s+(?:an?\s+)?required\s+"
        r"(?:merge\s+)?(?:status|check|gate)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\bmerge\s+(?:requires|needs|must\s+have)\s+"
        r"(?:(?:codex|ai(?:\s+reviewer)?|reviewer)\s+)?(?:approval|review)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"\b(?:codex|ai(?:\s+reviewer)?|reviewer)\s+(?:approval|review)\s+"
        r"(?:must|shall|has\s+to)\s+pass\b.{0,60}\bbefore\s+merg(?:e|ing)\b",
        re.IGNORECASE | re.DOTALL,
    ),
)

REVIEWER_MUTATION_GRANT = re.compile(
    r"\breviewer(?:s)?\b"
    r"(?:(?!\b(?:not|no|never|cannot|can't)\b).){0,40}?"
    r"\b(?:may|can|is\s+(?:allowed|authorized|permitted)\s+to|"
    r"has\s+(?:authority|permission)\s+to)\s+"
    r"(?:commit|push|merge|implement|modify|edit|update|change|write|fix|approve)\b",
    re.IGNORECASE | re.DOTALL,
)

BYPASS_AUTHORITY_GRANT = re.compile(
    r"\b(?:may|can|should|must|is\s+(?:allowed|authorized|permitted)\s+to)\s+"
    r"bypass\b.{0,80}\b(?:game-gate|merge\s+queue|branch\s+protection|protections?)\b",
    re.IGNORECASE | re.DOTALL,
)

LOCAL_SUBJECT_DENIAL = re.compile(
    r"\b(?:no|neither|not\s+one)(?:\s+[A-Za-z0-9_-]+){0,8}\s+$",
    re.IGNORECASE,
)


def _matches_any(patterns: tuple[re.Pattern[str], ...], text: str) -> bool:
    return any(pattern.search(text) is not None for pattern in patterns)


def _reviewer_grant_is_negated(text: str, start: int) -> bool:
    prefix = text[max(0, start - 96):start]
    clause_prefix = re.split(r"[.;:\n]", prefix)[-1]
    return LOCAL_SUBJECT_DENIAL.search(clause_prefix) is not None


def _has_unnegated_reviewer_grant(text: str) -> bool:
    for match in REVIEWER_MUTATION_GRANT.finditer(text):
        if not _reviewer_grant_is_negated(text, match.start()):
            return True
    return False


def _has_unnegated_bypass_grant(text: str) -> bool:
    for match in BYPASS_AUTHORITY_GRANT.finditer(text):
        prefix = text[max(0, match.start() - 96):match.start()]
        clause_prefix = re.split(r"[.;:\n]", prefix)[-1]
        if LOCAL_SUBJECT_DENIAL.search(clause_prefix):
            continue
        return True
    return False


def _validate_inherited_authority_boundaries(path: str, text: str, errors: list[str]) -> None:
    if _matches_any(AI_REVIEW_AUTHORITY_BROADENING, text):
        errors.append(f"{path}: prompt-local AI/review authority broadening is forbidden")
    if _has_unnegated_reviewer_grant(text) or _has_unnegated_bypass_grant(text):
        errors.append(f"{path}: prompt-local mutation/merge authority broadening is forbidden")


def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:
    _validate_meta_routing_coordinates(path, text, errors)
    _validate_inherited_authority_boundaries(path, text, errors)
    matches = [section for section in _level2_sections(text) if section[0] == SECTION]

    if not matches:
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
