#!/usr/bin/env python3
"""Validate Game prompt binding to the canonical META Remote Desktop gate."""
from __future__ import annotations

import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
LIFECYCLE_PATH = ROOT / "docs/agents/PROMPT_LIFECYCLE.json"
META_SHA = "e002fc7532188e73a0f495da3e20710541ed50e0"
SECTION = "## Remote Desktop execution routing"

REUSABLE_MARKERS = (
    META_SHA,
    "every direct `Remote_Desktop_Commander.*` invocation",
    "positive per-action",
    "`list_devices`",
    "cannot broaden META exception reasons",
    "routine fallback for repository tests, Git inspection, CI/log polling",
    "not automatically a blocker",
)

CANONICAL_SURFACES = (
    "AGENTS.md",
    "docs/agents/GITHUB_ONLY_EXECUTION.md",
    "docs/agents/PROMPTING_STANDARD.md",
    "docs/agents/PROMPT_EVAL_STANDARD.md",
)

FORBIDDEN_PERMISSIVE_PATTERNS = (
    re.compile(r"Remote Desktop is a routine fallback", re.IGNORECASE),
    re.compile(r"may use Remote Desktop for (?:routine )?repository tests", re.IGNORECASE),
    re.compile(r"list_devices may be used for capability discovery", re.IGNORECASE),
    re.compile(r"Remote_Desktop_Commander\.list_devices may be used for capability discovery", re.IGNORECASE),
)


def load_lifecycle(errors: list[str]) -> dict:
    try:
        value = json.loads(LIFECYCLE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        errors.append(f"unable to load prompt lifecycle: {exc}")
        return {}
    if not isinstance(value, dict):
        errors.append("prompt lifecycle root must be an object")
        return {}
    return value


def reusable_prompt_paths(lifecycle: dict, errors: list[str]) -> list[str]:
    entries = lifecycle.get("prompts")
    if not isinstance(entries, list):
        errors.append("prompt lifecycle prompts must be a list")
        return []
    paths: list[str] = []
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"prompt lifecycle entry {index} must be an object")
            continue
        if entry.get("status") != "reusable" or entry.get("reusable") is not True:
            continue
        path = entry.get("path")
        if not isinstance(path, str) or not path.startswith("docs/agents/prompts/") or not path.endswith(".md"):
            errors.append(f"reusable prompt entry {index} has invalid prompt path")
            continue
        paths.append(path)
    if not paths:
        errors.append("prompt lifecycle contains no reusable prompts")
    if len(paths) != len(set(paths)):
        errors.append("reusable prompt paths must be unique")
    return sorted(set(paths))


def validate_text(path: str, text: str, *, require_section: bool, errors: list[str]) -> None:
    if require_section and text.count(SECTION) != 1:
        errors.append(f"{path}: must contain exactly one {SECTION!r} section")
    for marker in REUSABLE_MARKERS:
        if marker not in text:
            errors.append(f"{path}: missing Remote Desktop routing marker: {marker}")
    for pattern in FORBIDDEN_PERMISSIVE_PATTERNS:
        if pattern.search(text):
            errors.append(f"{path}: contains permissive Remote Desktop routing text: {pattern.pattern}")


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
        validate_text(relative, text, require_section=True, errors=errors)

    for relative in CANONICAL_SURFACES:
        path = ROOT / relative
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: unable to read canonical surface: {exc}")
            continue
        validate_text(relative, text, require_section=False, errors=errors)

    root_agents = ROOT / "AGENTS.md"
    if root_agents.is_file():
        root_text = root_agents.read_text(encoding="utf-8")
        stale_match = re.search(
            r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\.json",
            root_text,
        )
        expected = f"Oteryn/Oteryn@{META_SHA}:ecosystem/agent-execution-routing-policy.json"
        if expected not in root_text:
            errors.append("AGENTS.md: canonical META execution-routing coordinate is missing")
        if stale_match is not None and stale_match.group(0) != expected:
            errors.append(f"AGENTS.md: stale META execution-routing coordinate: {stale_match.group(0)}")

    if not errors:
        print(
            f"Validated Remote Desktop per-action routing for {len(prompt_paths)} reusable prompts "
            f"against META {META_SHA}."
        )
    return errors


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(f"Remote Desktop prompt routing validation failed with {len(errors)} error(s).", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
