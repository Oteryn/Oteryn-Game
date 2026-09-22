#!/usr/bin/env python3
"""Validate Game's immutable META v3 binding, overlay and reusable task prompts."""
from __future__ import annotations

import base64
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
import types
import urllib.error
import urllib.parse
import urllib.request

try:
    from tools.agents import validate_governance as game_governance
except ModuleNotFoundError:  # Direct execution from outside the repository root.
    import validate_governance as game_governance

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



def _valid_repository_path(value: object) -> bool:
    return (
        isinstance(value, str)
        and bool(value)
        and not any(char in value for char in "\x00\n\r\\")
        and not value.startswith("/")
        and ".." not in value.split("/")
        and str(PurePosixPath(value)) == value
    )


def _normalize_positive_integer(raw: object, label: str) -> int:
    value = str(raw or "").strip()
    prefix = f"{label}:"
    if value.lower().startswith(prefix.lower()):
        value = value[len(prefix):].strip()
    if re.fullmatch(r"[1-9][0-9]*", value) is None:
        raise ValueError(f"{label} must be a positive integer")
    return int(value)


def _pull_request_active_task_paths(
    number: int,
    expected_head: str,
) -> set[str]:
    """Return active task packets changed by one exact-head live PR snapshot.

    Protected main may advance independently of an immutable PR head. Bind the
    candidate to the exact head and a stable live base snapshot observed during
    this read instead of requiring the triggering event's historical base SHA.
    """
    if re.fullmatch(r"[0-9a-f]{40}", expected_head) is None:
        raise ValueError("expected pull request head SHA is invalid")

    pr_url = f"https://api.github.com/repos/{PROVIDER}/pulls/{number}"
    pull = _request(pr_url)
    if not isinstance(pull, dict):
        raise ValueError("invalid GitHub pull response")

    head = pull.get("head", {}).get("sha", "")
    base = pull.get("base", {}).get("sha", "")
    changed_files = pull.get("changed_files")
    if pull.get("state") != "open":
        raise ValueError("live-state candidate validation requires an open pull request")
    if pull.get("head", {}).get("repo", {}).get("full_name") != PROVIDER:
        raise ValueError("live-state candidate validation requires a same-repository head")
    if pull.get("base", {}).get("ref") != "main":
        raise ValueError("live-state candidate validation requires base=main")
    if head != expected_head:
        raise ValueError("pull request head moved during live-state candidate validation")
    if re.fullmatch(r"[0-9a-f]{40}", base or "") is None:
        raise ValueError("live pull request base SHA is invalid")
    if type(changed_files) is not int or changed_files < 0 or changed_files > 3000:
        raise ValueError("invalid pull request changed-files count")

    items: list[dict] = []
    page = 1
    while len(items) < changed_files:
        batch = _request(
            f"https://api.github.com/repos/{PROVIDER}/pulls/{number}/files"
            f"?per_page=100&page={page}"
        )
        if not isinstance(batch, list) or not all(isinstance(item, dict) for item in batch):
            raise ValueError("invalid GitHub pull-files response")
        items.extend(batch)
        if len(batch) < 100:
            break
        page += 1
        if page > 30:
            raise ValueError("pull request file enumeration exceeded bounded pagination")

    if len(items) != changed_files:
        raise ValueError("pull request file enumeration is incomplete")

    pull_after = _request(pr_url)
    if not isinstance(pull_after, dict):
        raise ValueError("invalid GitHub pull readback")
    if (
        pull_after.get("state") != "open"
        or pull_after.get("head", {}).get("sha") != head
        or pull_after.get("base", {}).get("sha") != base
        or pull_after.get("base", {}).get("ref") != "main"
        or pull_after.get("changed_files") != changed_files
    ):
        raise ValueError("pull request moved during live-state candidate validation")

    prefix = "docs/agents/tasks/active/"
    selected: set[str] = set()
    seen_filenames: set[str] = set()
    for item in items:
        filename = item.get("filename")
        if not _valid_repository_path(filename):
            raise ValueError("invalid pull request filename")
        if filename in seen_filenames:
            raise ValueError("duplicate pull request filename")
        seen_filenames.add(filename)
        if (
            filename.startswith(prefix)
            and filename.endswith(".md")
            and filename != prefix + "README.md"
        ):
            selected.add(filename)

        previous = item.get("previous_filename")
        if previous is not None:
            if not _valid_repository_path(previous):
                raise ValueError("invalid pull request previous_filename")
            if (
                previous.startswith(prefix)
                and previous.endswith(".md")
                and previous != prefix + "README.md"
            ):
                selected.add(previous)
    return selected


def _active_task_live_scope() -> set[str] | None:
    """Use candidate-scoped live checks on PRs and full checks for main health."""
    event_name = os.environ.get("GITHUB_EVENT_NAME", "").strip()
    if event_name not in {"pull_request", "workflow_dispatch"}:
        return None

    event_path = os.environ.get("GITHUB_EVENT_PATH", "").strip()
    if not event_path:
        raise ValueError("GITHUB_EVENT_PATH is required for candidate live-state validation")
    event = json.loads(Path(event_path).read_text(encoding="utf-8"))
    if not isinstance(event, dict):
        raise ValueError("GitHub event payload must be an object")

    if event_name == "pull_request":
        pull = event.get("pull_request")
        if not isinstance(pull, dict):
            raise ValueError("pull_request event is missing pull_request payload")
        number = _normalize_positive_integer(event.get("number") or pull.get("number"), "pull_request_number")
        head = pull.get("head", {}).get("sha", "")
        return _pull_request_active_task_paths(number, head)

    inputs = event.get("inputs")
    if not isinstance(inputs, dict):
        raise ValueError("workflow_dispatch event is missing inputs")
    number = _normalize_positive_integer(inputs.get("pull_request_number"), "pull_request_number")
    head = (os.environ.get("TARGET_SHA") or os.environ.get("GITHUB_SHA") or "").strip().lower()
    return _pull_request_active_task_paths(number, head)


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
        errors.extend(
            game_governance.validate_active_task_live_state(
                _request,
                _active_task_live_scope(),
            )
        )
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
