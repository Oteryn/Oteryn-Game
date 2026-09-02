#!/usr/bin/env python3
"""Regression tests for lean reusable prompt inheritance."""
from __future__ import annotations

from validate_remote_desktop_prompt_routing import validate_reusable_prompt_text


def validate(text: str) -> list[str]:
    errors: list[str] = []
    validate_reusable_prompt_text("prompt.md", text, errors)
    return errors


def test_lean_prompt_inherits_global_routing_without_copy() -> None:
    errors = validate("# Prompt\n\nImplement the allocated lane and satisfy its acceptance criteria.\n")
    if errors:
        raise AssertionError(f"lean prompt should inherit root routing policy, got: {errors}")


def test_prompt_local_remote_desktop_policy_still_fails_closed() -> None:
    errors = validate("# Prompt\n\nUse Remote Desktop for routine Git inspection when convenient.\n")
    if not any("Remote Desktop policy text outside canonical section" in error for error in errors):
        raise AssertionError(f"prompt-local routing policy must fail closed, got: {errors}")


def main() -> int:
    tests = [value for name, value in sorted(globals().items()) if name.startswith("test_") and callable(value)]
    for test in tests:
        test()
        print(f"PASS {test.__name__}")
    print(f"Lean prompt deduplication regression tests PASS: {len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
