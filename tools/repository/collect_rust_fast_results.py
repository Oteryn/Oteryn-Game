#!/usr/bin/env python3
"""Join the complete current archive/shard result set without writing a ledger."""

import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile


ROLES = {"all", "bins", "lib", *(f"{kind}-{n}" for kind in ("lib", "dispatch") for n in range(1, 4))}
FIELDS = {"head", "base_sha", "role", "shard", "slug", "run_id", "run_attempt", "image", "platform",
          "build_image", "build_platform", "pending_packages", "exit_code", "completed"}
MAX_RESULT = 64 << 10
MAX_JUNIT = 32 << 20
MAX_BUNDLE = MAX_JUNIT + MAX_RESULT + (1 << 20)


def unique_object(pairs):
    result = {}
    for name, value in pairs:
        if name in result:
            raise ValueError("duplicate JSON field")
        result[name] = value
    return result


def parse_json(source):
    return json.loads(source, object_pairs_hook=unique_object)


def packages(value):
    if (not isinstance(value, list) or any(not isinstance(name, str)
            or not re.fullmatch(r"[A-Za-z0-9_-]+", name) for name in value)
            or len(value) != len(set(value))):
        raise ValueError("invalid package selection")
    return sorted(value)


def identity(value):
    if not isinstance(value, str) or len(value) > 256 or any(ord(char) < 32 for char in value):
        raise ValueError("invalid runner identity")
    return value


def plan(env):
    expected = {field: env[name] for field, name in (
        ("head", "EXPECTED_SHA"), ("base_sha", "BASE_SHA"), ("shard", "SHARD"),
        ("slug", "SLUG"), ("run_id", "GITHUB_RUN_ID"), ("run_attempt", "GITHUB_RUN_ATTEMPT"))}
    if (any(not re.fullmatch(r"[a-f0-9]{40}", expected[name]) for name in ("head", "base_sha"))
            or any(not re.fullmatch(r"[1-9][0-9]*", expected[name]) for name in ("run_id", "run_attempt"))
            or not re.fullmatch(r"[1-5]/5", expected["shard"])
            or expected["slug"] != f"tests-{expected['shard'][0]}-5"):
        raise ValueError("invalid expected result identity")
    roles = parse_json(env["ARCHIVE_ROLES"])
    if (not isinstance(roles, list) or not roles or any(role not in ROLES for role in roles)
            or len(roles) != len(set(roles))):
        raise ValueError("invalid planned archive roles")
    selection = parse_json(env["RUST_SELECTION"])
    selected = packages(selection["packages"])
    pending = packages(parse_json(env["TEST_SHARDS"])[expected["slug"]])
    if selection["mode"] not in ("workspace", "packages") or not selected or not set(pending) <= set(selected):
        raise ValueError("invalid planned package scope")
    planner = identity(env["PLAN_IMAGE"]), identity(env["PLAN_PLATFORM"])
    return expected, roles, selected, pending, planner


def no_symlinks(path):
    if any(parent.is_symlink() for parent in (path, *path.parents)):
        raise ValueError("symlink in result or output path")


def children(path, expected):
    no_symlinks(path)
    if not path.is_dir() or {child.name for child in path.iterdir()} != set(expected):
        raise ValueError("missing, duplicate or unexpected result path")


def read_file(path, limit):
    no_symlinks(path)
    if not path.is_file() or not 0 < path.stat().st_size <= limit:
        raise ValueError("missing or oversized result file")
    return path.read_bytes()


def api_pages(api, path, key):
    """Read a complete small inventory, refusing truncation or changing pages."""
    items, total = [], None
    separator = "&" if "?" in path else "?"
    for page in range(1, 11):
        document = api.json(f"{path}{separator}per_page=100&page={page}")
        count, batch = document.get("total_count"), document.get(key)
        if (type(count) is not int or not 0 <= count <= 1000 or not isinstance(batch, list)
                or len(batch) > 100 or any(not isinstance(item, dict) for item in batch)
                or (total is not None and count != total)):
            raise ValueError("invalid, oversized or changing GitHub result inventory")
        total = count
        items.extend(batch)
        if len(items) == total:
            return items
        if len(items) > total or len(batch) < 100:
            raise ValueError("incomplete GitHub result inventory")
    raise ValueError("GitHub result inventory exceeded the page bound")


def transport():
    # On older target heads the workflow loads both files at its exact immutable
    # workflow revision. Resolve beside this script, not from the target cwd.
    path = Path(__file__).resolve().parents[1] / "ci/shared_dispatch_archive.py"
    no_symlinks(path)
    spec = importlib.util.spec_from_file_location("rust_result_artifact_transport", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def unpack_report(bundle, directory, role, slug):
    prefix = f"{role}/{slug}/"
    allowed = {prefix + "result.json": MAX_RESULT, prefix + "junit.xml": MAX_JUNIT}
    with zipfile.ZipFile(bundle) as archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        if (not 1 <= len(entries) <= 2 or len(names) != len(set(names))
                or prefix + "result.json" not in names or not set(names) <= allowed.keys()):
            raise ValueError("unexpected or incomplete result artifact ZIP inventory")
        output = directory / role / slug
        output.mkdir(parents=True)
        for entry in entries:
            mode = entry.external_attr >> 16
            if (entry.is_dir() or (stat.S_IFMT(mode) and not stat.S_ISREG(mode))
                    or not 0 < entry.file_size <= allowed[entry.filename]):
                raise ValueError("invalid result artifact member type or size")
            with archive.open(entry) as source, (output / Path(entry.filename).name).open("xb") as target:
                shutil.copyfileobj(source, target, 1 << 20)


def download_reports(reports, env, *, api=None):
    expected, roles, _, _, _ = plan(env)
    no_symlinks(reports)
    if reports.exists() and (not reports.is_dir() or any(reports.iterdir())):
        raise ValueError("downloaded result root must be empty")
    shared = transport()
    repository = env.get("GITHUB_REPOSITORY", "")
    if repository != "Oteryn/Oteryn-Game":
        raise ValueError("unsupported result producer repository")
    api = api or shared.GitHub(repository, env.get("GH_TOKEN", ""))
    base = f"/repos/{repository}/actions/runs/{expected['run_id']}"
    run = api.json(base)
    shared.validate_run(run, {"source": {"repository": repository, "head": expected["head"]}})
    if run["id"] != int(expected["run_id"]) or run["run_attempt"] != int(expected["run_attempt"]):
        raise ValueError("result workflow run or current attempt mismatch")
    jobs = api_pages(api, base + "/jobs?filter=all", "jobs")
    selected = {}
    for role in roles:
        suffix = f"Unit-test archive ({role}, {expected['shard']})"
        matches = [job for job in jobs if job.get("name") == suffix
                   or job.get("name", "").endswith(" / " + suffix)]
        job = shared.select_actual_execution(matches, run_id=run["id"],
                                             head_sha=expected["head"], max_attempt=run["run_attempt"])
        if job is None:
            raise ValueError("missing actual archive-shard job")
        if job.get("status") != "completed" or job.get("conclusion") not in ("success", "failure"):
            raise ValueError("latest archive-shard job is not a completed test result")
        selected[role] = job
    artifacts = api_pages(api, base + "/artifacts", "artifacts")
    chosen = {}
    for role, job in selected.items():
        name = f"rust-fast-results-{job['run_attempt']}-{role}-{expected['slug']}"
        matches = [artifact for artifact in artifacts if artifact.get("name") == name]
        if len(matches) != 1:
            raise ValueError("latest archive-shard job has no unique result artifact")
        artifact = matches[0]
        origin = artifact.get("workflow_run", {})
        if (type(artifact.get("id")) is not int or artifact["id"] <= 0 or artifact.get("expired") is not False
                or type(artifact.get("size_in_bytes")) is not int or not 0 < artifact["size_in_bytes"] <= MAX_BUNDLE
                or origin.get("id") != run["id"] or origin.get("head_sha") != expected["head"]
                or origin.get("repository_id") != run["repository"]["id"]
                or origin.get("head_repository_id") != run["head_repository"]["id"]
                or not isinstance(artifact.get("digest"), str)
                or not re.fullmatch(r"sha256:[0-9a-f]{64}", artifact["digest"])):
            raise ValueError("result artifact producer identity mismatch")
        chosen[role] = artifact
    reports.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="download-results-", dir=reports.parent) as temporary:
        temporary = Path(temporary)
        staged = temporary / "verified"
        staged.mkdir()
        for role, job in selected.items():
            artifact = chosen[role]
            bundle = temporary / f"{artifact['id']}.zip"
            api.download(artifact["id"], bundle, max_bytes=MAX_BUNDLE)
            if (not 0 < bundle.stat().st_size <= MAX_BUNDLE
                    or "sha256:" + shared.sha256(bundle) != artifact["digest"]):
                raise ValueError("result artifact GitHub checksum mismatch")
            directory = staged / artifact["name"]
            unpack_report(bundle, directory, role, expected["slug"])
            result = parse_json(read_file(directory / role / expected["slug"] / "result.json", MAX_RESULT))
            binding = {**expected, "role": role, "run_attempt": str(job["run_attempt"])}
            code = 0 if job["conclusion"] == "success" else 100
            if (not isinstance(result, dict) or any(result.get(key) != value for key, value in binding.items())
                    or result.get("completed") is not True or type(result.get("exit_code")) is not int
                    or result["exit_code"] != code):
                raise ValueError("result artifact does not match its actual job attempt and conclusion")
        observed = api.json(base)
        shared.validate_run(observed, {"source": {"repository": repository, "head": expected["head"]}})
        if (any(observed[key] != run[key] for key in ("id", "run_attempt"))
                or any(observed[key]["id"] != run[key]["id"] for key in ("repository", "head_repository"))):
            raise ValueError("result workflow changed during download")
        if reports.exists():
            reports.rmdir()
        staged.rename(reports)
    for role, job in selected.items():
        print(f"RUST_FAST_RESULT role={role} shard={expected['shard']} job={job['id']} "
              f"attempt={job['run_attempt']} artifact={chosen[role]['id']}")
    return {role: str(job["run_attempt"]) for role, job in selected.items()}


def validate_junit(path, pending, code):
    if not path.exists() and not path.is_symlink():
        if code == 100:
            raise ValueError("failed result has no current JUnit witness")
        return False, False
    if not pending:
        raise ValueError("cached empty selection must not carry stale JUnit")
    source = read_file(path, MAX_JUNIT).decode("utf-8")
    if "<!DOCTYPE" in source.upper() or "<!ENTITY" in source.upper():
        raise ValueError("JUnit declarations are not allowed")
    tree = ET.fromstring(source)
    if tree.tag not in ("testsuites", "testsuite"):
        raise ValueError("unexpected JUnit root")
    failed, retried = False, False
    for node in tree.iter():
        if "{" in node.tag or ":" in node.tag:
            raise ValueError("unsupported JUnit namespace")
        if node.tag in ("failure", "error"):
            failed = True
        if node.tag in ("flakyFailure", "rerunFailure", "flakyError", "rerunError"):
            # Nextest retains earlier attempts here, including tests that later
            # passed. Preserve its final exit status but do not cache this run.
            retried = True
        if node.tag not in ("testsuites", "testsuite"):
            continue
        counts = {}
        for name in ("tests", "failures", "errors", "skipped"):
            if name in node.attrib:
                value = node.attrib[name]
                if not re.fullmatch(r"[0-9]+", value):
                    raise ValueError("invalid JUnit count")
                counts[name] = int(value)
        failed |= any(counts.get(name, 0) > 0 for name in ("failures", "errors"))
        cases = len(list(node.iter("testcase")))
        if "tests" in counts and counts["tests"] != cases:
            raise ValueError("incomplete JUnit test count")
        if any(counts.get(name, 0) > cases for name in ("failures", "errors", "skipped")):
            raise ValueError("incomplete JUnit outcome count")
        if node.tag == "testsuite" and node.attrib.get("name", "").split("::", 1)[0] not in pending:
            raise ValueError("JUnit contains an unselected package")
    if failed != (code == 100):
        raise ValueError("JUnit failure status disagrees with the completed result")
    return True, retried


def collect(reports, archive_root, extract_root, env, *, attempts=None):
    expected, roles, selected, pending, planner = plan(env)
    attempts = {role: expected["run_attempt"] for role in roles} if attempts is None else attempts
    if (set(attempts) != set(roles) or any(not isinstance(attempt, str)
            or not re.fullmatch(r"[1-9][0-9]*", attempt)
            or int(attempt) > int(expected["run_attempt"]) for attempt in attempts.values())):
        raise ValueError("invalid verified result attempt map")
    roots = (reports, archive_root, extract_root)
    for root in roots:
        no_symlinks(root)
    canonical = [path.resolve() for path in roots]
    if any(left.is_relative_to(right) or right.is_relative_to(left)
           for index, left in enumerate(canonical) for right in canonical[index + 1:]):
        raise ValueError("result input and output paths must be separate")
    for root in (archive_root, extract_root):
        if root.exists() and (not root.is_dir() or any(root.iterdir())):
            raise ValueError("result staging roots must be empty")
    folder = lambda role: f"rust-fast-results-{attempts[role]}-{role}-{expected['slug']}"
    children(reports, (folder(role) for role in roles))
    verified = []
    code, cacheable = 0, True
    for role in roles:
        artifact = reports / folder(role)
        children(artifact, [role])
        children(artifact / role, [expected["slug"]])
        directory = artifact / role / expected["slug"]
        no_symlinks(directory)
        if not directory.is_dir():
            raise ValueError("missing current archive-shard result directory")
        files = {path.name for path in directory.iterdir()}
        if files not in ({"result.json"}, {"result.json", "junit.xml"}):
            raise ValueError("unexpected archive-shard result files")
        result = parse_json(read_file(directory / "result.json", MAX_RESULT))
        if not isinstance(result, dict) or set(result) != FIELDS:
            raise ValueError("unexpected result schema")
        if any(result[name] != value for name, value in {**expected, "role": role,
                                                      "run_attempt": attempts[role]}.items()):
            raise ValueError("result head, base, run, attempt, role or shard mismatch")
        if result["completed"] is not True or type(result["exit_code"]) is not int or result["exit_code"] not in (0, 100):
            raise ValueError("incomplete or invalid test result")
        runner = identity(result["image"]), identity(result["platform"])
        builder = identity(result["build_image"]), identity(result["build_platform"])
        compatible = all(value not in ("", "unknown", "-") for value in planner) and runner == builder == planner
        wanted = pending if compatible else selected
        if packages(result["pending_packages"]) != wanted or result["pending_packages"] != wanted:
            raise ValueError("result package scope does not match the current identity plan")
        if not wanted and result["exit_code"] != 0:
            raise ValueError("empty cached selection cannot report test failures")
        has_junit, retried = validate_junit(directory / "junit.xml", wanted, result["exit_code"])
        verified.append((role, result, directory / "junit.xml" if has_junit else None))
        code = max(code, result["exit_code"])
        cacheable &= compatible and not retried

    # Validate the entire role union before exposing any report to the existing
    # ledger reader. Missing JUnit remains missing; it certifies no package pass.
    for root in (archive_root, extract_root):
        root.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="archive-results-", dir=archive_root.parent) as archive_tmp, \
            tempfile.TemporaryDirectory(prefix="test-results-", dir=extract_root.parent) as extract_tmp:
        archive_stage, extract_stage = Path(archive_tmp) / "ready", Path(extract_tmp) / "ready"
        archive_stage.mkdir()
        extract_stage.mkdir()
        for role, result, junit in verified:
            artifact, report = archive_stage / role, extract_stage / role
            artifact.mkdir()
            report.mkdir()
            (artifact / "runner-image").write_text(result["build_image"] + "\n", encoding="utf-8")
            (artifact / "runner-platform").write_text(result["build_platform"] + "\n", encoding="utf-8")
            if junit is not None:
                shutil.copyfile(junit, report / "junit.xml")
        for staged, root in ((archive_stage, archive_root), (extract_stage, extract_root)):
            if root.exists():
                root.rmdir()
            staged.rename(root)
    return code, cacheable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, required=True)
    parser.add_argument("--archive-root", type=Path, required=True)
    parser.add_argument("--extract-root", type=Path, required=True)
    parser.add_argument("--download", action="store_true")
    args = parser.parse_args()
    try:
        output_path = Path(os.environ["GITHUB_OUTPUT"])
        no_symlinks(output_path)
        attempts = download_reports(args.reports, os.environ) if args.download else None
        code, cacheable = collect(args.reports, args.archive_root, args.extract_root, os.environ, attempts=attempts)
        with output_path.open("a", encoding="utf-8") as output:
            output.write(f"completed=true\ncode={code}\ncacheable={str(cacheable).lower()}\n")
        return code
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, ET.ParseError, zipfile.BadZipFile) as error:
        print(f"Rust archive results rejected: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
