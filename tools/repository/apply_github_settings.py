#!/usr/bin/env python3
"""Apply the retained GitHub repository policy through the REST API."""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
POLICY = json.loads((ROOT / ".github/repository-policy.json").read_text(encoding="utf-8"))
REPOSITORY = os.environ["GITHUB_REPOSITORY"]
TOKEN = os.environ.get("GH_TOKEN", "")
API = f"https://api.github.com/repos/{REPOSITORY}"
API_VERSION = "2026-03-10"
LEGACY_ADMINISTRATION_ENVIRONMENT = "repository-administration"


class ApiError(RuntimeError):
    pass


def request(method: str, path: str, payload: Any | None = None, expected: tuple[int, ...] = (200,)) -> Any:
    body = None if payload is None else json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        f"{API}{path}",
        data=body,
        method=method,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {TOKEN}",
            "X-GitHub-Api-Version": API_VERSION,
            "User-Agent": "Oteryn-v2-repository-policy",
            **({"Content-Type": "application/json"} if body is not None else {}),
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            status = response.status
            data = response.read()
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise ApiError(f"{method} {path} failed with {exc.code}: {detail}") from exc
    if status not in expected:
        raise ApiError(f"{method} {path} returned unexpected status {status}")
    if not data:
        return None
    return json.loads(data)


def remove_legacy_administration_environment() -> None:
    environments = request("GET", "/environments?per_page=100", expected=(200,))
    names = {item.get("name") for item in environments.get("environments", [])}
    if LEGACY_ADMINISTRATION_ENVIRONMENT in names:
        encoded = urllib.parse.quote(LEGACY_ADMINISTRATION_ENVIRONMENT, safe="")
        request("DELETE", f"/environments/{encoded}", expected=(204,))


def configure_repository() -> None:
    repository = dict(POLICY["repository"])
    security = POLICY["security"]
    repository["security_and_analysis"] = {
        "secret_scanning": {"status": security["secret_scanning"]},
        "secret_scanning_push_protection": {
            "status": security["secret_scanning_push_protection"]
        },
    }
    request("PATCH", "", repository, expected=(200,))
    request("PUT", "/topics", {"names": POLICY["topics"]}, expected=(200,))
    request("PUT", "/actions/permissions/workflow", POLICY["actions"], expected=(204,))


def configure_labels() -> None:
    current = request("GET", "/labels?per_page=100", expected=(200,))
    existing = {label["name"] for label in current}
    for label in POLICY["labels"]:
        if label["name"] in existing:
            encoded = urllib.parse.quote(label["name"], safe="")
            request("PATCH", f"/labels/{encoded}", label, expected=(200,))
        else:
            request("POST", "/labels", label, expected=(201,))


def configure_security() -> None:
    security = POLICY["security"]
    if security["vulnerability_alerts"]:
        request("PUT", "/vulnerability-alerts", expected=(204,))
    if security["automated_security_fixes"]:
        request("PUT", "/automated-security-fixes", expected=(204,))
    if security["private_vulnerability_reporting"]:
        request("PUT", "/private-vulnerability-reporting", expected=(204,))


def configure_ruleset() -> None:
    expected_ruleset = POLICY["ruleset"]
    rulesets = request("GET", "/rulesets", expected=(200,))
    existing = next(
        (item for item in rulesets if item.get("name") == expected_ruleset["name"]),
        None,
    )
    if existing is None:
        request("POST", "/rulesets", expected_ruleset, expected=(201,))
    else:
        request("PUT", f"/rulesets/{existing['id']}", expected_ruleset, expected=(200,))


def repository_setting_matches(repo: dict[str, Any], key: str, expected: Any) -> bool:
    actual = repo.get(key)
    if actual == expected:
        return True
    if key == "use_squash_pr_title_as_default" and actual is None:
        return repo.get("squash_merge_commit_title") == "PR_TITLE"
    return False


def required_status_contexts(ruleset: dict[str, Any]) -> list[str]:
    for rule in ruleset.get("rules", []):
        if isinstance(rule, dict) and rule.get("type") == "required_status_checks":
            checks = rule.get("parameters", {}).get("required_status_checks", [])
            if not isinstance(checks, list):
                return []
            return [
                check.get("context")
                for check in checks
                if isinstance(check, dict) and isinstance(check.get("context"), str)
            ]
    return []


def fetch_ruleset_by_name(name: str) -> dict[str, Any]:
    rulesets = request("GET", "/rulesets", expected=(200,))
    match = next((item for item in rulesets if item.get("name") == name), None)
    if match is None:
        raise ApiError(f"ruleset {name!r} was not created")
    return request("GET", f"/rulesets/{match['id']}", expected=(200,))


def verify_ruleset_common(full: dict[str, Any], expected: dict[str, Any]) -> None:
    name = expected["name"]
    if full.get("enforcement") != "active":
        raise ApiError(f"ruleset {name!r} is not active")
    if full.get("bypass_actors") != []:
        raise ApiError(f"ruleset {name!r} must not have bypass actors")
    if full.get("target") != expected.get("target"):
        raise ApiError(
            f"ruleset {name!r} target mismatch: expected {expected.get('target')!r}, got {full.get('target')!r}"
        )


def verify() -> None:
    repo = request("GET", "", expected=(200,))
    for key, expected in POLICY["repository"].items():
        if not repository_setting_matches(repo, key, expected):
            raise ApiError(
                f"repository setting {key} mismatch: expected {expected!r}, got {repo.get(key)!r}"
            )

    topics = request("GET", "/topics", expected=(200,))
    if sorted(topics.get("names", [])) != sorted(POLICY["topics"]):
        raise ApiError("repository topics do not match policy")

    current_labels = request("GET", "/labels?per_page=100", expected=(200,))
    current_names = {label["name"] for label in current_labels}
    missing_labels = [
        label["name"] for label in POLICY["labels"] if label["name"] not in current_names
    ]
    if missing_labels:
        raise ApiError(f"repository labels missing after apply: {missing_labels}")

    permissions = request("GET", "/actions/permissions/workflow", expected=(200,))
    for key, expected in POLICY["actions"].items():
        if permissions.get(key) != expected:
            raise ApiError(
                f"Actions setting {key} mismatch: expected {expected!r}, got {permissions.get(key)!r}"
            )

    branch_ruleset = fetch_ruleset_by_name(POLICY["ruleset"]["name"])
    verify_ruleset_common(branch_ruleset, POLICY["ruleset"])
    expected_contexts = POLICY["required_status_checks"]
    policy_contexts = required_status_contexts(POLICY["ruleset"])
    actual_contexts = required_status_contexts(branch_ruleset)
    if policy_contexts != expected_contexts:
        raise ApiError(
            "repository policy required_status_checks disagrees with branch ruleset: "
            f"policy {expected_contexts!r}, ruleset {policy_contexts!r}"
        )
    if actual_contexts != expected_contexts:
        raise ApiError(
            "Protect main required-status mismatch: "
            f"expected {expected_contexts!r}, got {actual_contexts!r}"
        )

    guard = POLICY["control_plane_guard"]
    if guard.get("required_status_check") not in actual_contexts:
        raise ApiError("control-plane guard is not enforced by Protect main")

    private_reporting = request("GET", "/private-vulnerability-reporting", expected=(200,))
    if private_reporting.get("enabled") is not True:
        raise ApiError("private vulnerability reporting is not enabled")

    environments = request("GET", "/environments?per_page=100", expected=(200,))
    names = {item.get("name") for item in environments.get("environments", [])}
    if LEGACY_ADMINISTRATION_ENVIRONMENT in names:
        raise ApiError("legacy blocking administration environment still exists")

    print(
        "Repository settings, metadata, labels, Actions permissions, security features, "
        "Protect main required statuses, and base-trusted control-plane guard enforcement verified."
    )


def main() -> int:
    if not TOKEN:
        print("REPO_ADMIN_TOKEN is unavailable.", file=sys.stderr)
        return 2
    try:
        remove_legacy_administration_environment()
        configure_repository()
        configure_labels()
        configure_security()
        configure_ruleset()
        verify()
    except ApiError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
