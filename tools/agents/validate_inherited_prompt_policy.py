#!/usr/bin/env python3
"""Validate lean reusable prompts against inherited Game/META execution policy.

Reusable prompts may omit duplicated global Remote Desktop policy. If they contain an
operative legacy canonical block it must remain exact. Any prompt-local Remote Desktop
policy outside that legacy block still fails closed.
"""
from __future__ import annotations

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
)


def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:
    _validate_meta_routing_coordinates(path, text, errors)
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
            "Validated inherited Remote Desktop policy for "
            f"{len(prompt_paths)} reusable prompts; duplicated per-prompt policy is optional."
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
