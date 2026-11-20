"""Reuse a completed GitHub qualification only for an unchanged hosted Rust target."""
from __future__ import annotations

import datetime
import hashlib
import json
import os
from pathlib import Path
import re
from urllib import request


def reusable(fetch, repo, pr, head, current, stamp):
    # The Actions run pull_requests array can be empty. The fixed workflow run
    # title identifies the PR, and the successful step binds its validated base.
    title = f"Merge gate • pull_request • {pr}"
    def eligible_run():
        runs = fetch(f"actions/workflows/merge-gate.yml/runs?event=pull_request&head_sha={head}&per_page=100")
        if len(runs["workflow_runs"]) != runs["total_count"]:
            return None
        candidates = [r for r in runs["workflow_runs"] if r["display_title"] == title and r["id"] != current]
        # A rerun keeps its original id. An older failed or active attempt must
        # never be hidden by a green edit with a higher id.
        if not candidates or any(
            r["id"] >= current or r["run_attempt"] != 1 or r["status"] != "completed"
            or r["conclusion"] != "success" or r["event"] != "pull_request" or r["head_sha"] != head
            or r["path"] != ".github/workflows/merge-gate.yml" or r["head_repository"]["full_name"] != repo
            for r in candidates
        ):
            return None
        return max(r["id"] for r in candidates)

    run_id = eligible_run()
    if run_id is None:
        return False
    jobs = fetch(f"actions/runs/{run_id}/attempts/1/jobs?per_page=100")
    if len(jobs["jobs"]) != jobs["total_count"]:
        return False
    passed = {j["name"]: j for j in jobs["jobs"] if j["conclusion"] == "success"}
    confirmed = "game-gate" in passed and any(
        s["name"] == f"Qualified hosted Rust inputs {stamp}" and s["conclusion"] == "success"
        for s in passed.get("Merge gate / validate", {}).get("steps", [])
    )
    # Recheck the complete history after reading the jobs, including older ids.
    return confirmed and eligible_run() == run_id


def main():
    env = os.environ
    event = json.loads(Path(env["GITHUB_EVENT_PATH"]).read_text())
    repo, pr, head, base = (env[k] for k in ("GITHUB_REPOSITORY", "PR_NUMBER", "HEAD_SHA", "BASE_SHA"))
    image = env.get("ImageVersion")
    stamp, use = "", False
    if (re.fullmatch(r"[1-9][0-9]*", pr) and all(re.fullmatch("[0-9a-f]{40}", x) for x in (head, base))
            and image and env["MOLEHILL_MODE"] in ("", "off")):
        inputs = [repo, pr, base, head, image, env["RUNNER_OS"], env["RUNNER_ARCH"], env["ROUTING"], env["FILE_RECORDS"],
                  datetime.datetime.now(datetime.UTC).date().isoformat()]
        stamp = hashlib.sha256(json.dumps(inputs).encode()).hexdigest()
        changes = event.get("changes") or {}
        if (env["GITHUB_RUN_ATTEMPT"] == "1" and event.get("action") == "edited"
                and isinstance(changes, dict) and changes and changes.keys() <= {"title", "body"}):
            def fetch(path):
                req = request.Request(f"https://api.github.com/repos/{repo}/{path}", headers={
                    "Authorization": "Bearer " + env["GH_TOKEN"],
                    "Accept": "application/vnd.github+json",
                })
                with request.urlopen(req, timeout=10) as response:
                    return json.load(response)
            try:
                use = reusable(fetch, repo, pr, head, int(env["GITHUB_RUN_ID"]), stamp)
            except (OSError, ValueError, TypeError, KeyError):
                print("Previous qualification unavailable. Running Rust qualification.")
    if use:
        print("Reusing successful hosted Rust qualification for identical inputs.")
    with Path(env["GITHUB_OUTPUT"]).open("a") as out:
        print(f"stamp={stamp}", f"reuse={str(use).lower()}", sep="\n", file=out)


if __name__ == "__main__":
    main()
