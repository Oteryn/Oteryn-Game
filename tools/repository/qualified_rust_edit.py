"""Reuse a completed GitHub qualification only for an unchanged hosted Rust target."""
from __future__ import annotations

import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import sys
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
    return f"{run_id}/1" if confirmed and eligible_run() == run_id else None


WORKFLOW = ".github/workflows/merge-gate.yml"


def protected_workflow(fetch, workflow_sha, event_base, live_base):
    if event_base != live_base or not re.fullmatch(r"[0-9a-f]{40}", workflow_sha):
        return None
    # Both callers have checked out the protected base. A synthetic merge SHA
    # can change on an edit, so compare the executing workflow's blob itself.
    content = Path(WORKFLOW).read_bytes()
    protected = hashlib.sha1(f"blob {len(content)}\0".encode() + content).hexdigest()
    executed = fetch(f"contents/{WORKFLOW}?ref={workflow_sha}")
    if executed.get("type") == "file" and executed.get("sha") == protected:
        return protected
    return None


def main(final=False):
    env = os.environ
    event = json.loads(Path(env["GITHUB_EVENT_PATH"]).read_text())
    repo, pr, head, base = (env[k] for k in ("GITHUB_REPOSITORY", "PR_NUMBER", "HEAD_SHA", "BASE_SHA"))
    day = datetime.datetime.now(datetime.UTC).date().isoformat()
    stamp, source = "", None

    def fetch(path):
        req = request.Request(f"https://api.github.com/repos/{repo}/{path}", headers={
            "Authorization": "Bearer " + env["GH_TOKEN"],
            "Accept": "application/vnd.github+json",
        })
        with request.urlopen(req, timeout=10) as response:
            return json.load(response)

    try:
        if (re.fullmatch(r"[1-9][0-9]*", pr) and all(re.fullmatch("[0-9a-f]{40}", x) for x in (head, base))
                and env.get("ImageVersion") and env["MOLEHILL_MODE"] in ("", "off")):
            blob = protected_workflow(fetch, env.get("GITHUB_WORKFLOW_SHA", ""),
                                      event.get("pull_request", {}).get("base", {}).get("sha"), base)
            if blob:
                if final:
                    stamp = env["EXPECTED_STAMP"]
                    if not re.fullmatch(rf"{day}/{blob}/[0-9a-f]{{64}}", stamp):
                        raise ValueError("qualification expired or workflow changed")
                else:
                    inputs = [repo, pr, base, head, env["ImageVersion"], env["RUNNER_OS"], env["RUNNER_ARCH"],
                              env["ROUTING"], env["FILE_RECORDS"]]
                    stamp = f"{day}/{blob}/" + hashlib.sha256(json.dumps(inputs).encode()).hexdigest()
                changes = event.get("changes") or {}
                if ((final or env["GITHUB_RUN_ATTEMPT"] == "1") and event.get("action") == "edited"
                        and isinstance(changes, dict) and changes and changes.keys() <= {"title", "body"}):
                    source = reusable(fetch, repo, pr, head, int(env["GITHUB_RUN_ID"]), stamp)
    except (OSError, ValueError, TypeError, KeyError):
        print("Previous qualification unavailable.")
    if final:
        if not source or source != env["EXPECTED_SOURCE"]:
            raise SystemExit("Hosted Rust proof changed. Rerun all jobs to qualify this target.")
        print(f"Reconfirmed hosted Rust qualification from run/attempt {source}.")
        return
    if source:
        print(f"Reusing successful hosted Rust qualification from run/attempt {source}.")
    with Path(env["GITHUB_OUTPUT"]).open("a") as out:
        print(f"stamp={stamp}", f"reuse={str(bool(source)).lower()}", f"source={source or ''}", sep="\n", file=out)


if __name__ == "__main__":
    main(final=sys.argv[1:] == ["--confirm"])
