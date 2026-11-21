#!/usr/bin/env python3
"""Bind a reusable native-entry test executable to its exact candidate and inventory."""
import argparse
import hashlib
import json
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

ROOT = Path(__file__).resolve().parents[3]
TARGET = "content_native_entry_room"
TESTS = {
    "committed_room_source_carries_no_world_id",
    "committed_room_binds_any_world_id_deterministically",
    "platform_issued_world_binds_and_qualifies_the_committed_room",
}
IGNORED = {"platform_issued_world_binds_and_qualifies_the_committed_room"}
MAX_BUNDLE_BYTES = 2 << 30


def package_selection(value=None):
    if value is None:
        return {"mode": "packages", "packages": ["oteryn-game-server"]}
    if not isinstance(value, dict):
        raise ValueError("invalid native-entry package selection")
    if value.get("mode") == "workspace":
        return {"mode": "workspace"}
    packages = value.get("packages")
    if (value.get("mode") != "packages" or not isinstance(packages, list)
            or not all(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_-]+", name) for name in packages)
            or len(set(packages)) != len(packages) or "oteryn-game-server" not in packages):
        raise ValueError("native-entry producer selection must contain the server exactly once")
    return {"mode": "packages", "packages": sorted(packages)}


def identity(repo_root=ROOT):
    root = repo_root.resolve(strict=True)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    workspace = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], cwd=root, text=True,
    ).strip()
    if not re.fullmatch(r"[a-f0-9]{40}", head) or Path(workspace) != root:
        raise ValueError("invalid native-entry checkout identity")
    subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=root, check=True)
    return {"head": head, "workspace": workspace, "target": TARGET,
            "profile": "test", "features": [], "toolchain": "1.100.0"}


def canonical(path):
    if path.absolute() != path.resolve(strict=True):
        raise ValueError("native-entry artifact path must be canonical")
    return path.absolute()


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def list_tests(binary, repo_root=ROOT, *, ignored=False):
    args = [str(binary), "--list", "--format", "terse"]
    if ignored:
        args.append("--ignored")
    environment = {key: value for key, value in os.environ.items() if key not in {"GH_TOKEN", "GITHUB_TOKEN"}}
    output = subprocess.check_output(args, cwd=repo_root / "apps/game-server"
                                     if (repo_root / "apps/game-server").is_dir() else repo_root,
                                     text=True, env=environment)
    names = [line[:-6] for line in output.splitlines() if line.endswith(": test")]
    if len(names) != len(set(names)) or any(line.endswith(": benchmark") for line in output.splitlines()):
        raise ValueError("invalid native-entry test inventory")
    return set(names)


def executable(directory, repo_root=ROOT):
    directory = canonical(directory)
    manifest_path = canonical(directory / "manifest.json")
    if not manifest_path.is_file() or not 0 < manifest_path.stat().st_size <= 4096:
        raise ValueError("invalid native-entry manifest")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    binding = identity(repo_root)
    if (not isinstance(manifest, dict)
            or manifest.keys() != {*binding, "sha256", "tests", "ignored", "selection", "libraries"}
            or any(manifest[key] != value for key, value in binding.items())
            or manifest["tests"] != sorted(TESTS) or manifest["ignored"] != sorted(IGNORED)
            or not isinstance(manifest["sha256"], str)
            or not re.fullmatch(r"[a-f0-9]{64}", manifest["sha256"])):
        raise ValueError("native-entry artifact does not match source, build variant and inventory")
    if (manifest["selection"] != package_selection(manifest["selection"])
            or type(manifest["libraries"]) is not bool
            or (not manifest["libraries"] and manifest["selection"] != package_selection())):
        raise ValueError("native-entry artifact has an invalid package/target selection")
    path = canonical(directory / TARGET)
    if not path.is_file() or not os.access(path, os.X_OK):
        raise ValueError("native-entry test executable is not executable")
    if sha256(path) != manifest["sha256"]:
        raise ValueError("native-entry executable checksum mismatch")
    if list_tests(path, repo_root) != TESTS or list_tests(path, repo_root, ignored=True) != IGNORED:
        raise ValueError("native-entry executable has the wrong test inventory")
    return path


def build(output_dir, repo_root=ROOT, *, rust_selection=None):
    binding = identity(repo_root)
    selection = package_selection(rust_selection)
    package_args = (["--workspace"] if selection["mode"] == "workspace"
                    else [argument for name in selection["packages"] for argument in ("-p", name)])
    # A common-lib producer retains its entire package/dev-feature union. Its
    # library harnesses are already compiled; only the native integration target
    # is added. The focused fallback keeps the original server-only invocation.
    library_args = ["--lib"] if rust_selection is not None else []
    output = subprocess.check_output([
        "cargo", "+1.100.0", "test", "--locked", *package_args, *library_args,
        "--test", TARGET, "--no-run", "--message-format=json-render-diagnostics",
    ], cwd=repo_root, text=True)
    binary = None
    for line in output.splitlines():
        message = json.loads(line)
        if (message.get("reason") != "compiler-artifact" or not message.get("executable")
                or message["target"]["name"] != TARGET):
            continue
        if (binary is not None or message["target"]["kind"] != ["test"]
                or message["features"] != [] or message["profile"]["test"] is not True):
            raise ValueError("unexpected or duplicate native-entry Cargo executable")
        binary = canonical(Path(message["executable"]))
    if binary is None or not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("missing native-entry Cargo executable")
    if list_tests(binary, repo_root) != TESTS or list_tests(binary, repo_root, ignored=True) != IGNORED:
        raise ValueError("native-entry build has the wrong test inventory")
    if identity(repo_root) != binding:
        raise ValueError("native-entry source changed during compilation")
    output_dir.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output_dir.parent) as temporary:
        staging = Path(temporary) / "bundle"
        staging.mkdir()
        target = staging / TARGET
        shutil.copyfile(binary, target)
        target.chmod(0o755)
        manifest = dict(binding, sha256=sha256(target), tests=sorted(TESTS), ignored=sorted(IGNORED),
                        selection=selection, libraries=bool(library_args))
        (staging / "manifest.json").write_text(json.dumps(manifest) + "\n", encoding="utf-8")
        if output_dir.exists() or output_dir.is_symlink():
            raise ValueError("native-entry bundle output must be a new directory")
        staging.rename(output_dir)


def unpack(bundle, destination):
    """Accept exactly the producer's one ZIP member and two regular tar members."""
    with zipfile.ZipFile(bundle) as archive:
        members = archive.infolist()
        if len(members) != 1 or members[0].filename != "bundle.tar.gz":
            raise ValueError("unexpected native-entry artifact ZIP inventory")
        entry = members[0]
        mode = entry.external_attr >> 16
        if (entry.is_dir() or (stat.S_IFMT(mode) and not stat.S_ISREG(mode))
                or not 0 < entry.file_size <= MAX_BUNDLE_BYTES):
            raise ValueError("invalid native-entry artifact ZIP member")
        with archive.open(entry) as compressed, tarfile.open(fileobj=compressed, mode="r|gz") as contents:
            seen = set()
            for member in contents:
                limit = 4096 if member.name == "manifest.json" else MAX_BUNDLE_BYTES
                if (member.name not in {"manifest.json", TARGET} or member.name in seen
                        or member.type != tarfile.REGTYPE or not 0 < member.size <= limit):
                    raise ValueError("invalid native-entry artifact tar inventory")
                seen.add(member.name)
                target = destination / member.name
                with contents.extractfile(member) as source, target.open("xb") as output:
                    shutil.copyfileobj(source, output, 1 << 20)
                target.chmod(0o700 if member.name == TARGET else 0o600)
            if seen != {"manifest.json", TARGET}:
                raise ValueError("incomplete native-entry artifact inventory")


def transport():
    # Share the bounded, credential-safe GitHub redirect implementation with the
    # Dispatch consumer; native-entry owns its own source and inventory checks.
    sys.path.insert(0, str(ROOT / "tools/ci"))
    import shared_dispatch_archive
    return shared_dispatch_archive


def latest_producer(jobs, run_id, head_sha, max_attempt):
    selected = [job for job in jobs
                if re.search(r"Build unit-test archive \((lib|all|lib-1)\)$", job.get("name", ""))]
    return transport().select_actual_execution(selected, run_id=run_id, head_sha=head_sha,
                                               max_attempt=max_attempt)


def restore(output, *, repo_root=ROOT, env=None, api=None, clock=time.monotonic,
            sleep=time.sleep, timeout=600):
    env = os.environ if env is None else env
    shared = transport()
    binding = identity(repo_root)
    repository = env.get("GITHUB_REPOSITORY")
    if repository != "Oteryn/Oteryn-Game" or env.get("EXPECTED_SHA") != binding["head"]:
        raise ValueError("invalid native-entry expected source identity")
    if not 0 <= timeout <= 600:
        raise ValueError("invalid native-entry producer wait bound")
    if output.exists() or output.is_symlink():
        raise ValueError("native-entry restore output must be a new directory")
    api = api or shared.GitHub(repository, env.get("GH_TOKEN", ""))
    source = {"source": {"repository": repository, "head": binding["head"]}}
    base = f"/repos/{repository}/actions"
    runs = api.json(f"{base}/workflows/merge-gate.yml/runs?event=pull_request&head_sha={binding['head']}&per_page=10")
    candidates = [run for run in runs.get("workflow_runs", [])
                  if run.get("head_repository", {}).get("full_name") == repository]
    if not candidates:
        raise shared.Unavailable("no exact-head native-entry producer run")
    for run in candidates:
        shared.validate_run(run, source)
    run = max(candidates, key=lambda item: item["id"])
    run_id, attempt = run["id"], run["run_attempt"]
    jobs_path = f"{base}/runs/{run_id}/jobs?filter=all"
    started = clock()
    deadline = started + timeout
    while True:
        shared.validate_run(run, source)
        if (run["id"], run["run_attempt"]) != (run_id, attempt):
            raise ValueError("native-entry producer run or attempt changed")
        job = latest_producer(shared.github_items(api, jobs_path, "jobs"), run_id, binding["head"], attempt)
        producer_attempt = job["run_attempt"] if job else None
        ready = job is not None and (job["status"] == "in_progress" or (
            job["status"] == "completed" and job.get("conclusion") == "success"))
        if job is not None and job["status"] == "completed" and not ready:
            raise shared.Unavailable("native-entry producer is not running or successful")
        artifacts = shared.github_items(api, f"{base}/runs/{run_id}/artifacts", "artifacts") if ready else []
        matches = [item for item in artifacts if item.get("name") == f"native-entry-room-{producer_attempt}"]
        if len(matches) > 1:
            raise ValueError("ambiguous native-entry producer artifact")
        if matches:
            artifact = matches[0]
            producer = artifact.get("workflow_run", {})
            if (artifact.get("expired") is not False or type(artifact.get("id")) is not int
                    or artifact["id"] <= 0 or type(artifact.get("size_in_bytes")) is not int
                    or not 0 < artifact["size_in_bytes"] <= MAX_BUNDLE_BYTES
                    or producer.get("id") != run_id or producer.get("head_sha") != binding["head"]
                    or producer.get("repository_id") != run["repository"]["id"]
                    or producer.get("head_repository_id") != run["head_repository"]["id"]
                    or not re.fullmatch(r"sha256:[0-9a-f]{64}", artifact.get("digest", ""))):
                raise ValueError("native-entry artifact producer identity mismatch")
            output.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.TemporaryDirectory(prefix="native-entry-", dir=output.parent) as temporary:
                temporary = Path(temporary)
                bundle = temporary / "artifact.zip"
                staged = temporary / "verified"
                staged.mkdir()
                api.download(artifact["id"], bundle)
                if bundle.stat().st_size > MAX_BUNDLE_BYTES or "sha256:" + sha256(bundle) != artifact["digest"]:
                    raise ValueError("native-entry GitHub artifact checksum mismatch")
                unpack(bundle, staged)
                if json.loads((staged / "manifest.json").read_text()).get("libraries") is not True:
                    raise ValueError("native-entry producer did not preserve the common-lib recipe")
                executable(staged, repo_root)
                current = api.json(f"{base}/runs/{run_id}")
                shared.validate_run(current, source)
                if (current["id"], current["run_attempt"]) != (run_id, attempt):
                    raise ValueError("native-entry producer run or attempt changed during restore")
                latest = latest_producer(shared.github_items(api, jobs_path, "jobs"),
                                         run_id, binding["head"], attempt)
                if latest is None or (latest["id"], latest["run_attempt"]) != (job["id"], producer_attempt):
                    raise ValueError("native-entry producer changed during restore")
                if latest["status"] != "in_progress" and not (
                        latest["status"] == "completed" and latest.get("conclusion") == "success"):
                    raise ValueError("native-entry producer is no longer running or successful")
                staged.rename(output)
            print(f"NATIVE_ENTRY_PREBUILT_REUSED run={run_id} attempt={producer_attempt} "
                  f"producer_job={job['id']} artifact={artifact['id']}")
            return
        running = job is not None and job["status"] in {"queued", "in_progress"}
        # Allow a short initial matrix-discovery window; never wait for another
        # source revision, an unrelated role, or beyond the explicit time budget.
        discovering = job is None and clock() - started < min(60, timeout)
        if run.get("status") not in {"queued", "in_progress"} or not (running or discovering) or clock() >= deadline:
            raise shared.Unavailable("no compatible native-entry producer within the wait bound")
        print(f"NATIVE_ENTRY_PREBUILT_WAIT run={run_id} attempt={attempt} "
              f"remaining_seconds={max(0, int(deadline - clock()))}", file=sys.stderr, flush=True)
        sleep(max(0, min(15, deadline - clock())))
        run = api.json(f"{base}/runs/{run_id}")


def rejection_reason(error):
    # Emit only fixed diagnostic codes. API payloads and arbitrary exception
    # messages must never become log output or change the restore decision.
    if not isinstance(error, ValueError):
        return "unknown"
    return {
        "invalid, oversized or changing GitHub inventory": "github_inventory_invalid_or_changing",
        "invalid or duplicate GitHub inventory identity": "github_inventory_identity_invalid",
        "incomplete GitHub inventory": "github_inventory_incomplete",
        "GitHub inventory exceeded the page bound": "github_inventory_page_limit",
        "invalid native-entry expected source identity": "native_source_binding",
        "selected producer run identity mismatch": "producer_run_identity",
        "native-entry producer run or attempt changed": "producer_run_changed",
        "native-entry producer run or attempt changed during restore": "producer_run_changed_during_restore",
        "native-entry producer changed during restore": "producer_job_changed_during_restore",
        "native-entry producer is no longer running or successful": "producer_failed_during_restore",
    }.get(str(error), "unknown")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("build", "executable", "restore"))
    parser.add_argument("directory", type=Path)
    parser.add_argument("--rust-selection", type=json.loads,
                        help="retain the common-lib producer's full Cargo package selection")
    parser.add_argument("--wait-seconds", type=int, default=600)
    args = parser.parse_args()
    if args.command == "restore":
        shared = transport()
        try:
            restore(args.directory, timeout=args.wait_seconds)
        except shared.Unavailable as error:
            print(f"Native-entry producer unavailable: {error}", file=sys.stderr)
            return 3
        except (OSError, ValueError, subprocess.CalledProcessError, zipfile.BadZipFile, tarfile.TarError) as error:
            print(f"Native-entry producer rejected: reason={rejection_reason(error)}", file=sys.stderr)
            return 1
    elif args.command == "build":
        build(args.directory, rust_selection=args.rust_selection)
    else:
        if args.rust_selection is not None:
            parser.error("--rust-selection applies only to build")
        print(executable(args.directory))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
