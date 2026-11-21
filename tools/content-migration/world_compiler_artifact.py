#!/usr/bin/env python3
"""Restore the current run's exact-candidate release World compiler, never a latest build."""
import argparse
import importlib.util
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import zipfile

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ("merge-group-gate.yml", "integration-shards-dispatch.yml")
COMPILER = "oteryn-world-bundle-compiler"
MAX_ARCHIVE_BYTES = 2 << 30
MAX_DISCOVERY_PAGES = 10
_PREBUILT = None


class DiscoveryChanged(RuntimeError):
    """Retry a bounded listing instead of selecting from an incomplete snapshot."""


def dependencies():
    global _PREBUILT
    ci = str(ROOT / "tools/ci")
    if ci not in sys.path:
        sys.path.insert(0, ci)
    import shared_dispatch_archive as shared
    if _PREBUILT is None:
        directory = ROOT / "tools/qualification/node_boot"
        sys.path.insert(0, str(directory))
        spec = importlib.util.spec_from_file_location("world_compiler_node_prebuilt", directory / "prebuilt.py")
        _PREBUILT = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_PREBUILT)
    return shared, _PREBUILT


def context(repo_root, env, workflow):
    if workflow not in WORKFLOWS or env.get("GITHUB_REPOSITORY") != "Oteryn/Oteryn-Game":
        raise ValueError("unsupported World compiler producer workflow/repository")
    candidate, workflow_head = env.get("EXPECTED_SHA", ""), env.get("GITHUB_SHA", "")
    if not all(re.fullmatch(r"[0-9a-f]{40}", value) for value in (candidate, workflow_head)):
        raise ValueError("invalid World compiler candidate/workflow identity")
    if workflow == "merge-group-gate.yml" and candidate != workflow_head:
        raise ValueError("main World compiler must bind the triggering candidate")
    for key in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        if not re.fullmatch(r"[1-9][0-9]*", env.get(key, "")):
            raise ValueError("invalid current World compiler run/attempt")
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo_root, text=True).strip()
    if head != candidate:
        raise ValueError("World compiler consumer is not the expected candidate")
    return {"candidate": candidate, "workflow_head": workflow_head,
            "run": int(env["GITHUB_RUN_ID"]), "attempt": int(env["GITHUB_RUN_ATTEMPT"]),
            "workflow": workflow, "repository": env["GITHUB_REPOSITORY"]}


def validate_run(run, expected):
    repository = run.get("repository", {})
    head_repository = run.get("head_repository", {})
    events = {"workflow_dispatch"} if expected["workflow"] == "integration-shards-dispatch.yml" else {"push", "merge_group", "workflow_dispatch"}
    if (type(run.get("id")) is not int or type(run.get("run_attempt")) is not int
            or run.get("id") != expected["run"] or run.get("run_attempt") != expected["attempt"]
            or run.get("head_sha") != expected["workflow_head"]
            or run.get("path") != ".github/workflows/" + expected["workflow"]
            or run.get("event") not in events
            or repository.get("full_name") != expected["repository"]
            or head_repository.get("full_name") != expected["repository"]
            or type(repository.get("id")) is not int or repository["id"] <= 0
            or head_repository.get("id") != repository["id"]):
        raise ValueError("World compiler current-run identity mismatch")


def read_pages(api, path, key):
    items, seen = [], set()
    total = None
    separator = "&" if "?" in path else "?"
    for page in range(1, MAX_DISCOVERY_PAGES + 1):
        response = api.json(f"{path}{separator}per_page=100&page={page}")
        count, batch = response.get("total_count"), response.get(key)
        if (type(count) is not int or not 0 <= count <= 100 * MAX_DISCOVERY_PAGES
                or not isinstance(batch, list) or len(batch) > 100):
            raise ValueError("World compiler listing is invalid or exceeds its discovery bound")
        if total is not None and count != total:
            raise DiscoveryChanged("World compiler listing changed during pagination")
        total = count
        for item in batch:
            if not isinstance(item, dict) or type(item.get("id")) is not int or item["id"] <= 0:
                raise ValueError("invalid World compiler listing item identity")
            if item["id"] in seen:
                raise DiscoveryChanged("World compiler listing moved during pagination")
            seen.add(item["id"])
            items.append(item)
        if len(items) == total:
            return items
        if not batch or len(items) > total:
            raise DiscoveryChanged("World compiler listing is incomplete")
    raise ValueError("World compiler listing exceeds its discovery bound")


def latest_producer(jobs, expected):
    name = ("Merge Queue / World bundle" if expected["workflow"] == "merge-group-gate.yml"
            else "Integration shards / World compiler")
    producers = [job for job in jobs if job.get("name") == name]
    for job in producers:
        if (job.get("run_id") != expected["run"] or type(job.get("run_attempt")) is not int
                or not 1 <= job["run_attempt"] <= expected["attempt"]
                or job.get("head_sha") != expected["workflow_head"]
                or job.get("status") not in {"queued", "in_progress", "completed"}):
            raise ValueError("World compiler producer job identity mismatch")
    if not producers:
        return None
    attempt = max(job["run_attempt"] for job in producers)
    selected = [job for job in producers if job["run_attempt"] == attempt]
    if len(selected) != 1:
        raise ValueError("ambiguous World compiler producer job in one attempt")
    return selected[0]


def validate_artifact(artifact, run, expected, producer_attempt):
    producer = artifact.get("workflow_run", {})
    digest = artifact.get("digest")
    if (artifact.get("name") != f"world-compiler-{producer_attempt}"
            or type(artifact.get("id")) is not int or artifact["id"] <= 0
            or artifact.get("expired") is not False
            or type(artifact.get("size_in_bytes")) is not int
            or not 0 < artifact["size_in_bytes"] <= MAX_ARCHIVE_BYTES
            or producer.get("id") != expected["run"]
            or producer.get("head_sha") != expected["workflow_head"]
            or producer.get("repository_id") != run["repository"]["id"]
            or producer.get("head_repository_id") != run["head_repository"]["id"]
            or not isinstance(digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest)):
        raise ValueError("World compiler artifact producer identity mismatch")


def extract_archive(download, archive_path):
    with zipfile.ZipFile(download) as bundle:
        members = bundle.infolist()
        if len(members) != 1 or members[0].filename != "bundle.tar.gz":
            raise ValueError("unexpected World compiler ZIP inventory")
        entry = members[0]
        mode = entry.external_attr >> 16
        if (entry.is_dir() or (stat.S_IFMT(mode) and not stat.S_ISREG(mode))
                or not 0 < entry.file_size <= MAX_ARCHIVE_BYTES):
            raise ValueError("invalid World compiler ZIP member")
        with bundle.open(entry) as source, archive_path.open("xb") as target:
            shutil.copyfileobj(source, target, 1 << 20)


def restore(output, *, workflow="merge-group-gate.yml", repo_root=ROOT, env=None, api=None,
            timeout=600, clock=time.monotonic, sleep=time.sleep):
    env = os.environ if env is None else env
    expected = context(repo_root, env, workflow)
    if not 0 <= timeout <= 600 or output.exists() or output.is_symlink():
        raise ValueError("invalid World compiler wait bound or existing output directory")
    shared, prebuilt = dependencies()
    api = api or shared.GitHub(expected["repository"], env.get("GH_TOKEN", ""))
    base = f"/repos/{expected['repository']}/actions"
    deadline = clock() + timeout
    jobs_path = f"{base}/runs/{expected['run']}/jobs?filter=all"
    while True:
        run = api.json(f"{base}/runs/{expected['run']}")
        validate_run(run, expected)
        try:
            producer = latest_producer(read_pages(api, jobs_path, "jobs"), expected)
            producer_attempt = producer["run_attempt"] if producer else None
            artifacts = read_pages(api, f"{base}/runs/{expected['run']}/artifacts", "artifacts") if producer else []
        except DiscoveryChanged:
            if clock() >= deadline:
                raise RuntimeError("World compiler discovery did not stabilize within the wait bound") from None
            sleep(max(0, min(15, deadline - clock())))
            continue
        matches = [artifact for artifact in artifacts if artifact.get("name") == f"world-compiler-{producer_attempt}"]
        if len(matches) > 1:
            raise ValueError("ambiguous current-run World compiler artifact")
        if matches:
            artifact = matches[0]
            validate_artifact(artifact, run, expected, producer_attempt)
            output.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.TemporaryDirectory(prefix="world-compiler-", dir=output.parent) as temporary:
                temporary = Path(temporary)
                download, archive = temporary / "download.zip", temporary / "bundle.tar.gz"
                staged = temporary / "verified"
                api.download(artifact["id"], download)
                if download.stat().st_size > MAX_ARCHIVE_BYTES or "sha256:" + shared.sha256(download) != artifact["digest"]:
                    raise ValueError("World compiler GitHub artifact checksum mismatch")
                extract_archive(download, archive)
                # This existing verifier binds head, workspace, toolchain, release
                # profile, feature variant, inventory and the executable's digest.
                prebuilt.unpack("world-compiler", archive, staged)
                prebuilt.verify("world-compiler", staged)
                if context(repo_root, env, workflow) != expected:
                    raise ValueError("World compiler candidate changed during restore")
                validate_run(api.json(f"{base}/runs/{expected['run']}"), expected)
                latest = latest_producer(read_pages(api, jobs_path, "jobs"), expected)
                if latest is None or (latest["id"], latest["run_attempt"]) != (producer["id"], producer_attempt):
                    raise ValueError("World compiler producer changed during restore")
                staged.rename(output)
            print(f"WORLD_COMPILER_REUSED run={expected['run']} consumer_attempt={expected['attempt']} "
                  f"producer_attempt={producer_attempt} producer_job={producer['id']} artifact={artifact['id']}",
                  file=sys.stderr, flush=True)
            return output.resolve() / "world-compiler" / COMPILER
        if ((producer is not None and producer["status"] == "completed")
                or run.get("status") == "completed" or clock() >= deadline):
            raise RuntimeError("required current-run World compiler artifact was not published within the wait bound")
        print(f"WORLD_COMPILER_WAIT run={expected['run']} attempt={expected['attempt']} "
              f"remaining_seconds={max(0, int(deadline - clock()))}", file=sys.stderr, flush=True)
        sleep(max(0, min(15, deadline - clock())))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--workflow", choices=WORKFLOWS, default=WORKFLOWS[0])
    parser.add_argument("--wait-seconds", type=int, default=600)
    args = parser.parse_args()
    try:
        print(restore(args.directory, workflow=args.workflow, timeout=args.wait_seconds))
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError, zipfile.BadZipFile, tarfile.TarError) as error:
        print(f"World compiler artifact rejected: {type(error).__name__}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
