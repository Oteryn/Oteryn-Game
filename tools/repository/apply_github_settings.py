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


def configure_environment() -> None:
    env = POLICY["administration_environment"]
    payload = {
        "wait_timer": 0,
        "prevent_self_review": False,
        "reviewers": [{"type": "User", "id": env["reviewer_user_id"]}],
        "deployment_branch_policy": None,
    }
    request("PUT", f"/environments/{env['name']}", payload, expected=(200,))


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


def verify() -> None:
    repo = request("GET", "", expected=(200,))
    for key, expected in POLICY["repository"].items():
        if repo.get(key) != expected:
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

    rulesets = request("GET", "/rulesets", expected=(200,))
    match = next(
        (item for item in rulesets if item.get("name") == POLICY["ruleset"]["name"]),
        None,
    )
    if match is None:
        raise ApiError("Protect main ruleset was not created")
    full = request("GET", f"/rulesets/{match['id']}", expected=(200,))
    if full.get("enforcement") != "active":
        raise ApiError("Protect main ruleset is not active")

    private_reporting = request("GET", "/private-vulnerability-reporting", expected=(200,))
    if private_reporting.get("enabled") is not True:
        raise ApiError("private vulnerability reporting is not enabled")

    print(
        "Repository settings, metadata, labels, Actions permissions, security features, "
        "administration environment, and main ruleset applied and verified."
    )


def main() -> int:
    if not TOKEN:
        print("REPO_ADMIN_TOKEN is unavailable.", file=sys.stderr)
        return 2
    try:
        configure_environment()
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
