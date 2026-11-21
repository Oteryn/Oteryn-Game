#!/usr/bin/env python3
"""Verify same-run PostgreSQL archive bundles and select exact rerun tests."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import runpy
import shutil
import tempfile


PLANNER = runpy.run_path(str(Path(__file__).with_name("plan_pg_partitions.py")))
ARTIFACT_PREFIX = "pr-postgres-test-archives-"
ARTIFACT_NAMES = {archive: ARTIFACT_PREFIX + archive.removeprefix("pg-")
                  for archive in PLANNER["ARCHIVES"]}
FILTER = re.compile(r"--filterset=binary_id\(=([A-Za-z0-9_:-]+)\) & test\(=([A-Za-z0-9_:]+)\)")


def artifact_catalog(pages, run_id, head, repository_id):
    names = {name: archive for archive, name in ARTIFACT_NAMES.items()}
    catalog = {}
    ids = set()
    for page in pages:
        for artifact in page["artifacts"]:
            name = artifact["name"]
            if not name.startswith(ARTIFACT_PREFIX):
                continue
            if name not in names:
                raise ValueError(f"unexpected PostgreSQL archive {name!r}")
            archive = names[name]
            artifact_id = artifact["id"]
            if type(artifact_id) is not int or artifact_id < 1 or artifact_id in ids or archive in catalog:
                raise ValueError(f"{name}: invalid or duplicate artifact identity")
            source = artifact["workflow_run"]
            if (not source or source["id"] != run_id or source["head_sha"] != head
                    or source["repository_id"] != repository_id or source["head_repository_id"] != repository_id):
                raise ValueError(f"{name}: artifact source run, head, or repository mismatch")
            if artifact["expired"] is not False:
                raise ValueError(f"{name}: artifact has expired")
            catalog[archive] = artifact_id
            ids.add(artifact_id)
    if not catalog:
        raise ValueError("the source run has no PostgreSQL test archives")
    return catalog


def verify_bundle(directory, archive, head, workspace):
    names = ("head-sha", "workspace", f"{archive}.tar.zst", f"{archive}.json")
    files = set(directory.iterdir())
    manifest = directory / "SHA256SUMS"
    if files != {directory / name for name in (*names, "SHA256SUMS")}:
        raise ValueError(f"{archive}: unexpected or missing bundle files")
    if any(path.is_symlink() or not path.is_file() for path in files):
        raise ValueError(f"{archive}: bundle entries must be regular files")
    lines = manifest.read_bytes().splitlines(keepends=True)
    if len(lines) != len(names):
        raise ValueError(f"{archive}: malformed checksum manifest")
    for name, line in zip(names, lines):
        match = re.fullmatch(rb"([0-9a-f]{64})  " + re.escape(name.encode("ascii")) + rb"\n", line)
        if match is None:
            raise ValueError(f"{archive}: malformed checksum manifest for {name}")
        with (directory / name).open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if digest.encode("ascii") != match[1]:
            raise ValueError(f"{archive}: bad checksum for {name}")
    if (directory / "head-sha").read_bytes() != (head + "\n").encode("ascii"):
        raise ValueError(f"{archive}: archive head mismatch")
    if (directory / "workspace").read_text(encoding="utf-8") != workspace + "\n":
        raise ValueError(f"{archive}: archive workspace mismatch")


def read_filters(path):
    cases = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = FILTER.fullmatch(line)
        if match is None:
            raise ValueError(f"invalid exact PostgreSQL filter in {path.name}")
        cases[match.groups()] = line
    if not cases:
        raise ValueError("no requested PostgreSQL tests")
    return cases


def plan(download, catalog, head, workspace, filters, output):
    bundles = {archive: download / ARTIFACT_NAMES[archive] for archive in catalog}
    entries = set(download.iterdir())
    if len(bundles) == 1 and download / "SHA256SUMS" in entries:
        bundles = {next(iter(bundles)): download}
    elif entries != set(bundles.values()):
        raise ValueError("unexpected or missing PostgreSQL archive downloads")
    with tempfile.TemporaryDirectory(prefix="pg-rerun-") as temporary:
        inventory = Path(temporary)
        for archive, directory in bundles.items():
            if directory.is_symlink() or not directory.is_dir():
                raise ValueError(f"{archive}: missing archive bundle directory")
            verify_bundle(directory, archive, head, workspace)
            shutil.copyfile(directory / f"{archive}.json", inventory / f"{archive}.json")
        # Use the PR planner's ownership rules for the common dispatch tests.
        cases = PLANNER["read_cases"](inventory, optional=True)

    wanted = read_filters(filters)
    selection = {archive: [wanted[case] for case in archive_cases if case in wanted]
                 for archive, archive_cases in cases.items()}
    missing = set(wanted.values()) - {line for lines in selection.values() for line in lines}
    if missing:
        raise ValueError("no test in the archives for " + ", ".join(sorted(missing)))
    output.mkdir()
    for archive, lines in selection.items():
        if lines:
            payload = f"{archive}.tar.zst"
            (bundles[archive] / payload).rename(output / payload)
            (output / f"{archive}.filters").write_text("\n".join(lines) + "\n", encoding="utf-8")


def check_listing(filters, listing):
    wanted = set(read_filters(filters))
    data = json.loads(listing.read_text(encoding="utf-8"))
    matches = [(suite["binary-id"], name) for suite in data["rust-suites"].values()
               for name, case in suite["testcases"].items() if case["filter-match"]["status"] == "matches"]
    if len(matches) != len(wanted) or set(matches) != wanted:
        raise ValueError(f"{listing.name}: nextest selection differs from the requested tests")
    return len(matches)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    catalog = commands.add_parser("catalog")
    catalog.add_argument("artifacts", type=Path)
    catalog.add_argument("run_id", type=int)
    catalog.add_argument("head")
    catalog.add_argument("repository_id", type=int)
    prepare = commands.add_parser("plan")
    prepare.add_argument("download", type=Path)
    prepare.add_argument("catalog", type=Path)
    prepare.add_argument("head")
    prepare.add_argument("workspace")
    prepare.add_argument("filters", type=Path)
    prepare.add_argument("output", type=Path)
    check = commands.add_parser("check-list")
    check.add_argument("filters", type=Path)
    check.add_argument("listing", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "catalog":
            pages = json.loads(args.artifacts.read_text(encoding="utf-8"))
            print(json.dumps(artifact_catalog(pages, args.run_id, args.head, args.repository_id)))
        elif args.command == "plan":
            archives = json.loads(args.catalog.read_text(encoding="utf-8"))
            plan(args.download, archives, args.head, args.workspace, args.filters, args.output)
        else:
            print(check_listing(args.filters, args.listing))
    except (KeyError, OSError, TypeError, ValueError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
