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

CANONICAL_PROMPT_SECTION = f"""## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@{META_SHA}`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. This prompt cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work."""

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

# Reusable prompts have one canonical Remote Desktop authority block. Any additional
# Remote Desktop policy vocabulary outside that block is rejected so another paragraph
# cannot silently broaden, override, or claim physical enforcement of the contract.
OUTSIDE_ROUTING_PATTERNS = (
    re.compile(r"Remote_Desktop_Commander", re.IGNORECASE),
    re.compile(r"\bRemote\s+Desktop\b", re.IGNORECASE),
    re.compile(r"\bDesktop\s+Commander\b", re.IGNORECASE),
    re.compile(r"\bRDC\b", re.IGNORECASE),
    re.compile(r"\blist_devices\b", re.IGNORECASE),
    re.compile(r"\bwho_am_i\b", re.IGNORECASE),
    re.compile(r"\bget_config\b", re.IGNORECASE),
    re.compile(r"\bping\b.{0,100}\b(?:capability|discover|connector|tool|host)\b", re.IGNORECASE),
    re.compile(r"\b(?:capability|discover|connector|tool|host)\b.{0,100}\bping\b", re.IGNORECASE),
    re.compile(r"\b(?:connector|router|transport)\b.{0,100}\bphysical(?:ly)?\b.{0,100}\benforc", re.IGNORECASE),
    re.compile(r"\bphysical(?:ly)?\b.{0,100}\b(?:connector|router|transport)\b.{0,100}\benforc", re.IGNORECASE),
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


def _extract_canonical_section(path: str, text: str, errors: list[str]) -> tuple[str, str] | None:
    if text.count(SECTION) != 1:
        errors.append(f"{path}: must contain exactly one {SECTION!r} section")
        return None

    start = text.index(SECTION)
    remainder = text[start + len(SECTION):]
    next_heading = re.search(r"(?m)^##\s+.+$", remainder)
    end = start + len(SECTION) + next_heading.start() if next_heading is not None else len(text)
    section_text = text[start:end].strip()
    outside_text = (text[:start] + "\n" + text[end:]).strip()
    return section_text, outside_text


def validate_reusable_prompt_text(path: str, text: str, errors: list[str]) -> None:
    extracted = _extract_canonical_section(path, text, errors)
    if extracted is None:
        return
    section_text, outside_text = extracted

    if section_text != CANONICAL_PROMPT_SECTION:
        errors.append(f"{path}: canonical Remote Desktop routing section must match exactly")

    for pattern in OUTSIDE_ROUTING_PATTERNS:
        match = pattern.search(outside_text)
        if match is not None:
            snippet = match.group(0).replace("\n", " ")[:120]
            errors.append(
                f"{path}: Remote Desktop policy text outside canonical section: {snippet!r}"
            )


def validate_surface_text(path: str, text: str, errors: list[str]) -> None:
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
        validate_reusable_prompt_text(relative, text, errors)

    for relative in CANONICAL_SURFACES:
        path = ROOT / relative
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(f"{relative}: unable to read canonical surface: {exc}")
            continue
        validate_surface_text(relative, text, errors)

    root_agents = ROOT / "AGENTS.md"
    if root_agents.is_file():
        root_text = root_agents.read_text(encoding="utf-8")
        coordinates = re.findall(
            r"Oteryn/Oteryn@[0-9a-f]{40}:ecosystem/agent-execution-routing-policy\.json",
            root_text,
        )
        expected = f"Oteryn/Oteryn@{META_SHA}:ecosystem/agent-execution-routing-policy.json"
        if expected not in root_text:
            errors.append("AGENTS.md: canonical META execution-routing coordinate is missing")
        stale = sorted({coordinate for coordinate in coordinates if coordinate != expected})
        for coordinate in stale:
            errors.append(f"AGENTS.md: stale META execution-routing coordinate: {coordinate}")

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
