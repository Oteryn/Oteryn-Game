#!/usr/bin/env python3
"""Validate Game's immutable META v3 binding, overlay and reusable task prompts."""
from __future__ import annotations

import base64
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import sys
import types
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
BINDING_PATH = ROOT / "docs/agents/META_AGENT_POLICY_BINDING.json"
LIFECYCLE_PATH = ROOT / "docs/agents/PROMPT_LIFECYCLE.json"
PROVIDER = "Oteryn/Oteryn-Game"
CENTRAL_VALIDATOR_PATH = "tools/governance/central_agent_policy.py"
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
RETIRED_REVIEW_CONTROLLER_RE = re.compile(
    r"\bCODEX_REVIEW_POLICY(?:\.json)?\b|"
    r"\bCODEX_(?:REQUIRED|OPTIONAL|NOT_REQUIRED(?:_BY_THIS_POLICY)?)\b",
    re.IGNORECASE,
)
CURRENT_REVIEW_POLICY_CONSUMERS = (
    "docs/agents/OWNER_FUNDED_AI_POLICY.md",
)
CURRENT_ACTIVE_REVIEW_CONSUMERS = (
    "docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md",
)


def _request(url: str, *, timeout: float = 30.0) -> object:
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.load(response)


def _content(repository: str, commit: str, relative: str) -> str:
    path = urllib.parse.quote(relative, safe="/")
    payload = _request(f"https://api.github.com/repos/{repository}/contents/{path}?ref={commit}")
    if not isinstance(payload, dict) or payload.get("encoding") != "base64":
        raise ValueError(f"invalid GitHub contents response for {relative}")
    encoded = payload.get("content")
    if not isinstance(encoded, str) or not encoded:
        raise ValueError(f"empty GitHub contents response for {relative}")
    return base64.b64decode(encoded).decode("utf-8")


def _authenticate_binding(binding: object) -> tuple[str, str, str]:
    if not isinstance(binding, dict):
        raise ValueError("provider binding must be an object")
    repository = binding.get("authority_repository")
    commit = binding.get("authority_commit")
    if repository != "Oteryn/Oteryn" or not isinstance(commit, str) or SHA_RE.fullmatch(commit) is None:
        raise ValueError("binding must name Oteryn/Oteryn at a lowercase full commit SHA")

    commit_payload = _request(f"https://api.github.com/repos/{repository}/commits/{commit}")
    branch = _request(f"https://api.github.com/repos/{repository}/branches/main")
    if not isinstance(commit_payload, dict) or commit_payload.get("sha") != commit:
        raise ValueError("binding commit could not be resolved exactly")
    if not isinstance(branch, dict) or branch.get("protected") is not True:
        raise ValueError("META main is not verified as protected")
    main = branch.get("commit")
    main_sha = main.get("sha") if isinstance(main, dict) else None
    if not isinstance(main_sha, str) or SHA_RE.fullmatch(main_sha) is None:
        raise ValueError("META main SHA is invalid")
    compare = _request(f"https://api.github.com/repos/{repository}/compare/{commit}...{main_sha}")
    if not isinstance(compare, dict) or compare.get("status") not in {"ahead", "identical"}:
        raise ValueError("binding commit is not an ancestor of protected META main")
    for key in ("base_commit", "merge_base_commit"):
        coordinate = compare.get(key)
        if not isinstance(coordinate, dict) or coordinate.get("sha") != commit:
            raise ValueError("binding ancestry response does not match the exact commit")
    return repository, commit, main_sha


def _load_central(source: str) -> types.ModuleType:
    module = types.ModuleType("bound_central_agent_policy")
    module.__file__ = f"{PROVIDER}:{CENTRAL_VALIDATOR_PATH}"
    sys.modules[module.__name__] = module
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def _reusable_prompt_paths(lifecycle: object) -> list[str]:
    if not isinstance(lifecycle, dict) or not isinstance(lifecycle.get("prompts"), list):
        raise ValueError("prompt lifecycle registry must contain a prompts list")
    paths: list[str] = []
    for entry in lifecycle["prompts"]:
        if isinstance(entry, dict) and entry.get("status") == "reusable" and entry.get("reusable") is True:
            path = entry.get("path")
            if not isinstance(path, str) or not path.startswith("docs/agents/prompts/"):
                raise ValueError("reusable prompt has an invalid path")
            paths.append(path)
    if not paths or len(paths) != len(set(paths)):
        raise ValueError("reusable prompt paths must be non-empty and unique")
    return paths


def _legacy_review_controller_errors(text: str, central: types.ModuleType) -> list[str]:
    """Reject operative use of Game's retired review controller.

    The bound central policy owns Markdown statement extraction and the explicit
    audit/negative exemption. Keeping that semantic view here avoids treating
    comments, fenced examples, quoted evidence or retirement instructions as
    live authority.
    """
    statements = getattr(central, "_statements", None)
    is_audit_or_negative = getattr(central, "_is_audit_or_negative", None)
    if not callable(statements) or not callable(is_audit_or_negative):
        raise ValueError("bound central validator lacks operative-statement helpers")
    return [
        f"operative statement uses retired Game review controller: {match.group(0)}"
        for statement in statements(text)
        if not is_audit_or_negative(statement)
        for match in [RETIRED_REVIEW_CONTROLLER_RE.search(statement)]
        if match is not None
    ]


def validate() -> list[str]:
    try:
        binding = json.loads(BINDING_PATH.read_text(encoding="utf-8"))
        lifecycle = json.loads(LIFECYCLE_PATH.read_text(encoding="utf-8"))
        repository, commit, main_sha = _authenticate_binding(binding)
        requested = [
            CENTRAL_VALIDATOR_PATH,
            "ecosystem/organization-agent-policy.json",
            binding["organization_policy_path"],
            binding["prompting_standard_path"],
            binding["prompt_eval_standard_path"],
        ]
        with ThreadPoolExecutor(max_workers=len(requested)) as executor:
            fetched = dict(zip(requested, executor.map(lambda path: _content(repository, commit, path), requested)))
        central = _load_central(fetched[CENTRAL_VALIDATOR_PATH])
        policy = json.loads(fetched["ecosystem/organization-agent-policy.json"])
        surfaces = {path: fetched[path] for path in requested[2:]}
        resolved = {
            "repository": repository,
            "commit": commit,
            "merged_to_protected_main": True,
            "protected_main_sha": main_sha,
            "branch_protected": True,
            "policy": policy,
            "human_surfaces": surfaces,
        }
        errors = list(central.validate_provider_binding(
            binding,
            policy=policy,
            authority_resolver=lambda expected_repository, expected_commit: (
                resolved if (expected_repository, expected_commit) == (repository, commit) else None
            ),
        ))
        errors.extend(central.validate_provider_overlay(
            PROVIDER,
            (ROOT / "AGENTS.md").read_text(encoding="utf-8"),
            policy=policy,
        ))
        prompt_paths = _reusable_prompt_paths(lifecycle)
        present_active_review_consumers = (
            relative for relative in CURRENT_ACTIVE_REVIEW_CONSUMERS
            if (ROOT / relative).is_file()
        )
        review_consumers = dict.fromkeys(
            (*prompt_paths, *CURRENT_REVIEW_POLICY_CONSUMERS, *present_active_review_consumers)
        )
        for relative in prompt_paths:
            prompt_errors = central.validate_task_prompt_text(
                (ROOT / relative).read_text(encoding="utf-8"), policy=policy,
            )
            errors.extend(f"{relative}: {error}" for error in prompt_errors)
        for relative in review_consumers:
            errors.extend(
                f"{relative}: {error}"
                for error in _legacy_review_controller_errors(
                    (ROOT / relative).read_text(encoding="utf-8"), central,
                )
            )
        for relative, expected in (
            ("docs/agents/PROMPTING_STANDARD.md", binding["prompting_standard_path"]),
            ("docs/agents/PROMPT_EVAL_STANDARD.md", binding["prompt_eval_standard_path"]),
        ):
            if expected not in (ROOT / relative).read_text(encoding="utf-8"):
                errors.append(f"{relative}: missing bound META source path {expected}")
        if not errors:
            print(
                f"Validated META policy {binding['policy_version']} at {commit}: "
                f"authenticated protected-main ancestry, Game overlay, delivery extensions and "
                f"{len(prompt_paths)} reusable task prompts."
            )
        return errors
    except (OSError, KeyError, ValueError, TypeError, json.JSONDecodeError, UnicodeError,
            urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as exc:
        return [f"META provider-policy validation could not complete: {exc}"]


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(f"META provider-policy validation failed with {len(errors)} error(s).", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
