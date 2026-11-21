#!/usr/bin/env python3
"""Build or reuse one immutable, identically configured Dispatch test archive.

Exit 3 means no compatible producer is available: the caller may build locally.
Invalid selected-producer evidence is an error, never a cache miss.
"""

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
import tempfile
import time
import tomllib
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import HTTPRedirectHandler, Request, build_opener
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = "oteryn-shared-dispatch/v1"
WORKFLOW = ".github/workflows/merge-gate.yml"
BINARY_ID = "oteryn-gameplay-dispatch::oteryn-gameplay-dispatch"
MANIFEST = "shared-dispatch.json"
RECEIPT = ".build-receipt.json"
FILES = ("unit-tests.tar.zst", "inventory.json", "cargo-nextest", "runner-image", "runner-platform")
LIMITS = {"unit-tests.tar.zst": 4 << 30, "inventory.json": 16 << 20,
          "cargo-nextest": 128 << 20, "runner-image": 256, "runner-platform": 256, MANIFEST: 64 << 10}
MAX_ZIP_BYTES = (4 << 30) + (160 << 20)
CARGO_ARGS = ("nextest", "archive", "--locked", "--workspace", "--features",
              "oteryn-gameplay-dispatch/test-support", "--config",
              "profile.dev.package.oteryn-gameplay-dispatch.debug=0", "--test",
              "oteryn-gameplay-dispatch", "--zstd-level", "1")


class Unavailable(RuntimeError):
    """A local canonical build is required; no unverified artifact was exposed."""


def recipe(role):
    if not re.fullmatch(r"dispatch-[123]", role):
        raise ValueError("invalid Dispatch archive role")
    return {"version": 2, "toolchain": "1.100.0", "nextest": "0.9.146", "mold": "2.42.1",
            "cargo_args": list(CARGO_ARGS), "shard": f"{role[-1]}/3", "mode": "dedicated",
            "jobs": 2, "incremental": False, "linker_proof": "archived-elf-comment-mold-2.42.1"}


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def entrypoints(root):
    dispatch = root / "crates/gameplay/dispatch"
    target = dispatch / "src/ci_unit_archive.rs"
    if not target.is_file():
        raise Unavailable("the source predates the dedicated Dispatch target")
    manifest = tomllib.loads((dispatch / "Cargo.toml").read_text())
    if not any(item.get("name") == "oteryn-gameplay-dispatch"
               and item.get("path") == "src/ci_unit_archive.rs" and item.get("test") is False
               for item in manifest.get("test", [])):
        raise Unavailable("the dedicated Dispatch target is not registered")
    sources = [(dispatch / "src/lib.rs").read_text(), target.read_text()]
    guards = ('#![cfg(not(all(test, oteryn_dispatch_test_archive = "common")))]',
              '#![cfg(all(test, oteryn_dispatch_test_archive = "dedicated"))]')
    bodies = []
    for source, guard in zip(sources, guards):
        lines = [line.strip() for line in source.splitlines()
                 if line.strip() and not line.lstrip().startswith("//")]
        if lines.count(guard) != 1:
            raise Unavailable("the Dispatch entrypoint guard is unsupported")
        bodies.append([line for line in lines if line != guard])
    if bodies[0] != bodies[1] or bodies[0] != ["pub mod gameplay_transport;", "pub mod observability;",
                                             'include!("dispatch.rs");']:
        raise Unavailable("the Dispatch entrypoint parity is unsupported")


def context(root, role, env):
    entrypoints(root)
    expected = env.get("EXPECTED_SHA", "")
    repository = env.get("GITHUB_REPOSITORY", "")
    if not re.fullmatch(r"[0-9a-f]{40}", expected) or repository != "Oteryn/Oteryn-Game":
        raise ValueError("invalid expected source identity")
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    dirty = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"],
                                    cwd=root, text=True).strip()
    if head != expected or dirty:
        raise ValueError("Dispatch source must be the clean exact expected head")
    image = env.get("ImageVersion", "")
    platform = f"{env.get('RUNNER_OS', '')}-{env.get('RUNNER_ARCH', '')}"
    if not image or image == "unknown" or platform != "Linux-X64":
        raise Unavailable("no supported hosted runner identity")
    return {"schema": SCHEMA, "source": {"repository": repository, "head": head,
            "cargo_lock_sha256": sha256(root / "Cargo.lock"), "workspace": str(root.resolve())},
            "recipe": recipe(role), "role": role, "runtime": {"image": image, "platform": platform}}


def build_environment(env, role, root):
    denied = {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
              "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP", "RUSTDOC", "RUSTDOCFLAGS",
              "CARGO_ENCODED_RUSTDOCFLAGS", "LD_LIBRARY_PATH", "CC", "CXX", "CFLAGS",
              "CXXFLAGS", "CPPFLAGS", "LDFLAGS", "AR", "ARFLAGS", "RANLIB",
              "PKG_CONFIG_PATH", "PKG_CONFIG_LIBDIR", "CMAKE_TOOLCHAIN_FILE"}
    empty_overrides = []
    for name, value in env.items():
        if (name in denied or name.startswith(("CARGO_PROFILE_", "CARGO_ALIAS_", "CC_", "CXX_",
                                               "CFLAGS_", "CXXFLAGS_", "AR_"))
                or (name.startswith("CARGO_TARGET_") and name != "CARGO_TARGET_DIR")
                or (name.startswith("CARGO_BUILD_") and name != "CARGO_BUILD_JOBS")):
            if value:
                raise ValueError(f"unsupported build configuration: {name}")
            empty_overrides.append(name)
    fixed = {"OTERYN_DISPATCH_TEST_ARCHIVE": "dedicated", "OTERYN_DISPATCH_PG_SHARD": recipe(role)["shard"],
             "CARGO_BUILD_JOBS": "2", "CARGO_INCREMENTAL": "0"}
    if any(env.get(name, value) != value for name, value in fixed.items()):
        raise ValueError("conflicting canonical build configuration")
    if env.get("RUSTUP_TOOLCHAIN", "1.100.0") not in ("1.100.0", "1.100.0-x86_64-unknown-linux-gnu"):
        raise ValueError("conflicting toolchain configuration")
    target = Path(env.get("CARGO_TARGET_DIR", "target"))
    if not target.is_absolute():
        target = root / target
    if target.resolve() != (root / "target").resolve():
        raise ValueError("conflicting target directory configuration")
    cargo_home = Path(env.get("CARGO_HOME", Path.home() / ".cargo"))
    # Do not read user Cargo configuration: it may contain credentials. Its
    # existence makes this fixed recipe unsupported until the caller removes it.
    configs = [cargo_home / name for name in ("config", "config.toml")]
    configs += [parent / ".cargo" / name for parent in root.resolve().parents
                for name in ("config", "config.toml")]
    if any(path.exists() for path in configs):
        raise ValueError("external Cargo configuration cannot qualify the canonical recipe")
    preload = env.get("MOLD_PRELOAD", "")
    if not preload or env.get("LD_PRELOAD") != preload or not Path(preload).is_file():
        raise ValueError("canonical build requires the pinned mold preload")
    result = dict(env, **fixed)
    for name in (*empty_overrides, "GH_TOKEN", "GITHUB_TOKEN"):
        result.pop(name, None)
    return result


def inventory(path):
    listing = json.loads(path.read_text())
    suites = listing.get("rust-suites", {})
    if len(suites) != 1:
        raise ValueError("expected exactly one Dispatch test binary")
    suite = next(iter(suites.values()))
    cases = suite.get("testcases", {})
    # Default nextest list retains #[ignore] cases as ignored-only mismatches.
    # Keep them in the complete inventory and its identity; any other filter
    # exclusion still means this is not the unfiltered canonical archive.
    if (suite.get("binary-id") != BINARY_ID or not cases or listing.get("test-count") != len(cases)
            or any(not re.fullmatch(r"\w+(?:::\w+)*", name)
                   or case.get("filter-match") != (
                       {"status": "mismatch", "reason": "ignored"} if case.get("ignored") is True
                       else {"status": "matches"})
                   or not isinstance(case.get("ignored"), bool) for name, case in cases.items())):
        raise ValueError("invalid complete Dispatch test inventory")
    names = sorted((name, case["ignored"]) for name, case in cases.items())
    return {"binary_id": BINARY_ID, "tests": len(names),
            "identity": hashlib.sha256(json.dumps(names, separators=(",", ":")).encode()).hexdigest()}


def verify_archived_linker(path, extracted, env):
    # Inspect bytes extracted from this archive, not an older executable left
    # under target/. The one-suite inventory binds which executable to inspect.
    inventory(path)
    suite = next(iter(json.loads(path.read_text())["rust-suites"].values()))
    value = suite.get("binary-path")
    if not isinstance(value, str) or not value:
        raise ValueError("missing archived executable path for mold proof")
    binary = Path(value)
    if (not binary.is_absolute() or binary.resolve(strict=True) != binary
            or not binary.is_relative_to(extracted.resolve())
            or not stat.S_ISREG(binary.lstat().st_mode) or not binary.stat().st_mode & 0o111):
        raise ValueError("mold proof requires the exact regular archived executable")
    comment = subprocess.check_output(["readelf", "-p", ".comment", str(binary)],
                                      cwd=extracted, env=env, stdin=subprocess.DEVNULL, text=True)
    if not re.search(r"\bmold 2\.42\.1(?:\s|\(|$)", comment):
        raise ValueError("archived Dispatch executable was not linked by pinned mold")


def manifest(directory, identity, run_id, attempt):
    for name in FILES:
        path = directory / name
        if not path.is_file() or path.is_symlink() or not 0 < path.stat().st_size <= LIMITS[name]:
            raise ValueError(f"missing or invalid expected file: {name}")
    return {**identity, "producer": {"run_id": run_id, "run_attempt": attempt},
            "sha256": {name: sha256(directory / name) for name in FILES},
            "inventory": inventory(directory / "inventory.json")}


def producer(env):
    values = [env.get(name, "") for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")]
    if any(not re.fullmatch(r"[1-9][0-9]*", value) for value in values):
        raise ValueError("invalid producer run identity")
    return tuple(map(int, values))


def verify_manifest(directory, document, identity, run_id, attempt):
    actual = manifest(directory, identity, run_id, attempt)
    if document.get("sha256") != actual["sha256"]:
        raise ValueError("shared Dispatch file digest mismatch")
    if document != actual:
        raise ValueError("shared Dispatch source, recipe, producer or inventory mismatch")
    for name, field in (("runner-image", "image"), ("runner-platform", "platform")):
        if (directory / name).read_text().strip() != identity["runtime"][field]:
            raise ValueError("shared Dispatch runner identity mismatch")


def record(directory, role, *, root=ROOT, env=None):
    env = os.environ if env is None else env
    receipt = directory / RECEIPT
    if not receipt.is_file() or receipt.is_symlink() or receipt.stat().st_size > LIMITS[MANIFEST]:
        raise ValueError("a canonical build receipt is required")
    document = json.loads(receipt.read_text())
    verify_manifest(directory, document, context(root, role, env), *producer(env))
    (directory / MANIFEST).write_text(json.dumps(document, sort_keys=True) + "\n")
    receipt.unlink()


def build(directory, role, *, root=ROOT, env=None):
    env = os.environ if env is None else env
    identity = context(root, role, env)
    child_env = build_environment(env, role, root)
    if directory.exists() and (directory.is_symlink() or any(directory.iterdir())):
        raise ValueError("canonical build output must be empty")
    nextest = shutil.which("cargo-nextest", path=env.get("PATH"))
    if not nextest or not subprocess.check_output([nextest, "--version"], env=child_env, text=True).startswith("cargo-nextest 0.9.146 "):
        raise ValueError("pinned cargo-nextest is required")
    mold = env.get("MOLD_PATH", "")
    if not mold or not subprocess.check_output([mold, "--version"], env=child_env, text=True).startswith("mold 2.42.1 "):
        raise ValueError("pinned mold is required")
    directory.mkdir(parents=True, exist_ok=True)
    archive = directory / "unit-tests.tar.zst"
    subprocess.run(["cargo", "+1.100.0", *CARGO_ARGS, "--archive-file", str(archive)],
                   cwd=root, env=child_env, stdin=subprocess.DEVNULL, check=True)
    with tempfile.TemporaryDirectory(prefix="shared-dispatch-linker-") as temporary:
        extracted = Path(temporary)
        listing = subprocess.check_output(
            ["cargo", "+1.100.0", "nextest", "list", "--archive-file", str(archive),
             "--workspace-remap", str(root), "--extract-to", str(extracted), "--message-format", "json"],
            cwd=root, env=child_env, stdin=subprocess.DEVNULL,
        )
        (directory / "inventory.json").write_bytes(listing)
        verify_archived_linker(directory / "inventory.json", extracted, child_env)
    shutil.copyfile(nextest, directory / "cargo-nextest")
    (directory / "runner-image").write_text(identity["runtime"]["image"] + "\n")
    (directory / "runner-platform").write_text(identity["runtime"]["platform"] + "\n")
    (directory / RECEIPT).write_text(json.dumps(manifest(directory, identity, *producer(env))))
    record(directory, role, root=root, env=env)


def verify(directory, role, *, root=ROOT, env=None, producer_image=None, producer_attempt=None):
    env = os.environ if env is None else env
    identity = context(root, role, env)
    if producer_image is not None:
        identity["runtime"]["image"] = producer_image
    run_id, current_attempt = producer(env)
    if producer_attempt is None:
        producer_attempt = current_attempt
    if type(producer_attempt) is not int or not 1 <= producer_attempt <= current_attempt:
        raise ValueError("invalid actual producer attempt")
    path = directory / MANIFEST
    if not path.is_file() or path.is_symlink() or not 0 < path.stat().st_size <= LIMITS[MANIFEST]:
        raise ValueError("missing or invalid shared manifest")
    document = json.loads(path.read_text())
    verify_manifest(directory, document, identity, run_id, producer_attempt)


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, url):
        return None


class GitHub:
    def __init__(self, repository, token):
        if repository != "Oteryn/Oteryn-Game" or not token:
            raise Unavailable("no authorized repository artifact reader")
        self.repository, self.token = repository, token
        self.opener = build_opener(NoRedirect())

    def open(self, path):
        if not path.startswith(f"/repos/{self.repository}/actions/"):
            raise ValueError("artifact API path is outside the repository")
        request = Request("https://api.github.com" + path, headers={
            "Authorization": "Bearer " + self.token, "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        })
        return self.opener.open(request, timeout=30)

    def json(self, path):
        try:
            with self.open(path) as response:
                data = response.read((8 << 20) + 1)
        except (HTTPError, URLError) as error:
            raise Unavailable("GitHub artifact metadata is unavailable") from error
        if len(data) > 8 << 20:
            raise ValueError("artifact metadata exceeded its limit")
        return json.loads(data)

    def download(self, artifact_id, destination, *, max_bytes=None):
        limit = MAX_ZIP_BYTES if max_bytes is None else max_bytes
        if type(limit) is not int or not 0 < limit <= MAX_ZIP_BYTES:
            raise ValueError("invalid artifact download limit")
        try:
            try:
                response = self.open(f"/repos/{self.repository}/actions/artifacts/{artifact_id}/zip")
            except HTTPError as error:
                if error.code != 302:
                    raise
                url = error.headers.get("Location", "")
                parsed = urlparse(url)
                if (parsed.scheme != "https" or parsed.username or parsed.password or not parsed.hostname
                        or not parsed.hostname.endswith((".blob.core.windows.net", ".actions.githubusercontent.com",
                                                        ".githubusercontent.com", ".amazonaws.com"))):
                    raise ValueError("unexpected artifact storage redirect") from None
                # Never forward the GitHub bearer token to artifact storage.
                response = self.opener.open(Request(url), timeout=30)
            with response, destination.open("wb") as target:
                copied = 0
                while block := response.read(1 << 20):
                    copied += len(block)
                    if copied > limit:
                        raise ValueError("artifact exceeded its download limit")
                    target.write(block)
        except (HTTPError, URLError) as error:
            raise Unavailable("GitHub artifact download is unavailable") from error


def artifact_name(role, attempt):
    recipe(role)
    return f"rust-fast-unit-tests-{attempt}-{role}"


def validate_run(run, identity):
    repository = identity["source"]["repository"]
    if (run.get("head_sha") != identity["source"]["head"] or run.get("event") != "pull_request"
            or run.get("path") != WORKFLOW or run.get("repository", {}).get("full_name") != repository
            or run.get("head_repository", {}).get("full_name") != repository
            or type(run.get("id")) is not int or run["id"] <= 0
            or type(run.get("run_attempt")) is not int or run["run_attempt"] <= 0
            or type(run.get("repository", {}).get("id")) is not int or run["repository"]["id"] <= 0
            or run["head_repository"].get("id") != run["repository"]["id"]):
        raise ValueError("selected producer run identity mismatch")


def validate_artifact(artifact, run, role):
    source = artifact.get("workflow_run", {})
    if (artifact.get("name") != artifact_name(role, run["run_attempt"])
            or artifact.get("expired") is not False
            or type(artifact.get("id")) is not int or artifact["id"] <= 0
            or type(artifact.get("size_in_bytes")) is not int
            or not 0 < artifact["size_in_bytes"] <= MAX_ZIP_BYTES
            or source.get("id") != run["id"] or source.get("head_sha") != run["head_sha"]
            or source.get("repository_id") != run["repository"].get("id")
            or source.get("head_repository_id") != run["head_repository"].get("id")
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", artifact.get("digest", ""))):
        raise ValueError("selected artifact identity mismatch")


def extract(bundle, directory, role):
    expected = {f"{role}/{name}": name for name in (*FILES, MANIFEST)}
    with zipfile.ZipFile(bundle) as archive:
        entries = archive.infolist()
        if len(entries) != len(expected) or {entry.filename for entry in entries} != expected.keys():
            raise ValueError("unexpected, duplicate or missing expected artifact files")
        for entry in entries:
            mode = entry.external_attr >> 16
            name = expected[entry.filename]
            if (entry.is_dir() or (stat.S_IFMT(mode) and not stat.S_ISREG(mode))
                    or not 0 < entry.file_size <= LIMITS[name]):
                raise ValueError("invalid artifact file type or size")
            with archive.open(entry) as source, (directory / name).open("xb") as target:
                shutil.copyfileobj(source, target, 1 << 20)


def restore(role, output, *, root=ROOT, env=None, api=None, clock=time.monotonic, sleep=time.sleep,
            timeout=1200, discovery_grace=60):
    env = os.environ if env is None else env
    identity = context(root, role, env)
    if not 0 <= timeout <= 1200 or not 0 <= discovery_grace <= 60:
        raise ValueError("invalid producer wait bound")
    if output.exists() and (output.is_symlink() or any(output.iterdir())):
        raise ValueError("restore output must be empty")
    repository = identity["source"]["repository"]
    api = api or GitHub(repository, env.get("GH_TOKEN", ""))
    base = f"/repos/{repository}/actions"
    runs = api.json(f"{base}/workflows/merge-gate.yml/runs?event=pull_request&head_sha={identity['source']['head']}&per_page=10")
    candidates = [run for run in runs.get("workflow_runs", [])
                  if run.get("head_repository", {}).get("full_name") == repository]
    if not candidates:
        raise Unavailable("no exact-head producer run")
    for candidate in candidates:
        validate_run(candidate, identity)
    run = max(candidates, key=lambda item: item["id"])
    run_id, attempt = run["id"], run["run_attempt"]
    started = clock()
    deadline, discovery_deadline = started + timeout, started + min(discovery_grace, timeout)
    while True:
        validate_run(run, identity)
        if (run["id"], run["run_attempt"]) != (run_id, attempt):
            raise ValueError("producer run or attempt changed during discovery")
        jobs = api.json(f"{base}/runs/{run_id}/attempts/{attempt}/jobs?per_page=100").get("jobs", [])
        label = f"Build unit-test archive ({role})"
        selected = [job for job in jobs if job.get("name", "") == label
                    or job.get("name", "").endswith(" / " + label)]
        if selected and (len(selected) != 1 or selected[0].get("run_id") != run_id
                         or selected[0].get("run_attempt") != attempt):
            raise ValueError("canonical producer job identity mismatch")
        if not selected or selected[0].get("status") == "queued":
            if run.get("status") in ("queued", "in_progress") and clock() < discovery_deadline:
                sleep(max(0, min(5, discovery_deadline - clock())))
                run = api.json(f"{base}/runs/{run_id}")
                continue
            raise Unavailable("canonical build not discovered within its short grace period")
        job = selected[0]
        if job.get("status") != "in_progress" and not (
                job.get("status") == "completed" and job.get("conclusion") == "success"):
            raise Unavailable("canonical archive build is not running or successful")
        artifacts = api.json(f"{base}/runs/{run_id}/artifacts?per_page=100").get("artifacts", [])
        matches = [item for item in artifacts if item.get("name") == artifact_name(role, attempt)]
        if len(matches) > 1:
            raise ValueError("ambiguous producer artifact")
        if matches:
            artifact = matches[0]
            validate_artifact(artifact, run, role)
            output.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.TemporaryDirectory(prefix="shared-dispatch-", dir=output.parent) as temporary:
                temporary = Path(temporary)
                bundle, staged = temporary / "artifact.zip", temporary / "verified"
                staged.mkdir()
                api.download(artifact["id"], bundle)
                if "sha256:" + sha256(bundle) != artifact["digest"]:
                    raise ValueError("GitHub artifact digest mismatch")
                extract(bundle, staged, role)
                document = json.loads((staged / MANIFEST).read_text())
                producer_identity = {**identity, "runtime": {**identity["runtime"],
                    "image": document.get("runtime", {}).get("image")}}
                verify_manifest(staged, document, producer_identity, run_id, attempt)
                if producer_identity["runtime"]["image"] != identity["runtime"]["image"]:
                    raise Unavailable("verified producer uses a different runner image")
                if output.exists():
                    output.rmdir()
                staged.rename(output)
                (output / "cargo-nextest").chmod(0o755)
            return
        if run.get("status") != "in_progress" or job.get("status") != "in_progress" or clock() >= deadline:
            raise Unavailable("no running compatible producer within the wait bound")
        sleep(max(0, min(30, deadline - clock())))
        run = api.json(f"{base}/runs/{run_id}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("build", "record", "verify"):
        command = commands.add_parser(name)
        command.add_argument("directory", type=Path)
        command.add_argument("role")
        if name == "verify":
            command.add_argument("--producer-image")
            command.add_argument("--producer-attempt", type=int)
    command = commands.add_parser("restore")
    command.add_argument("role")
    command.add_argument("directory", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "restore":
            restore(args.role, args.directory)
        elif args.command == "verify":
            verify(args.directory, args.role, producer_image=args.producer_image,
                   producer_attempt=args.producer_attempt)
        else:
            {"build": build, "record": record}[args.command](args.directory, args.role)
    except Unavailable as error:
        print(f"Shared Dispatch archive unavailable: {error}", file=sys.stderr)
        return 3
    except (OSError, ValueError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        print(f"Shared Dispatch archive rejected: {type(error).__name__}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
