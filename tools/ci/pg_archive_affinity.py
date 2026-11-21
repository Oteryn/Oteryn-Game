#!/usr/bin/env python3
"""Route one PostgreSQL archive per worker and verify the complete inventory."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import tempfile


# Calibrated from the hosted 36-worker PostgreSQL runs. Single-archive workers
# keep their four nextest threads, so each lane needs enough parallel workers
# even when the duration table underestimates its short cases.
SLOTS = {"lib-1": 15, "lib-2": 7, "lib-3": 7, "e2e": 5, "lib": 2}
TOTAL = sum(SLOTS.values())


def archives(raw):
    selected = json.loads(raw)
    if not isinstance(selected, list) or not selected:
        raise ValueError("expected a nonempty PostgreSQL archive list")
    if any(not isinstance(name, str) or name not in SLOTS for name in selected):
        raise ValueError("unexpected PostgreSQL archive")
    names = tuple(name for name in SLOTS if name in selected)
    if len(names) != len(selected):
        raise ValueError("duplicate PostgreSQL archive")
    return names


def route(names, index, total):
    if total != TOTAL or not 0 <= index < total:
        raise ValueError(f"invalid PostgreSQL worker {index} of {total}")
    weight = sum(SLOTS[name] for name in names)
    shares = {name: divmod(total * SLOTS[name], weight) for name in names}
    counts = {name: share[0] for name, share in shares.items()}
    remainder = total - sum(counts.values())
    for name in sorted(names, key=lambda name: -shares[name][1])[:remainder]:
        counts[name] += 1
    # Spread the four Molehill slots in split mode across the substantial lanes.
    workers = [(name, local_index, count)
               for local_index in range(max(counts.values()))
               for name, count in counts.items() if local_index < count]
    return workers[index]


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_bundle(directory, archive, digest, head, workspace, payload):
    sums = directory / "SHA256SUMS"
    if sha256(sums) != digest:
        raise ValueError(f"{archive}: archive digest mismatch")
    names = ("head-sha", "workspace", f"pg-{archive}.tar.zst", f"pg-{archive}.json")
    lines = sums.read_text(encoding="ascii").splitlines()
    if len(lines) != len(names):
        raise ValueError(f"{archive}: malformed checksum manifest")
    for filename, line in zip(names, lines):
        checksum = re.fullmatch(r"([0-9a-f]{64})  " + re.escape(filename), line)
        if checksum is None:
            raise ValueError(f"{archive}: malformed checksum for {filename}")
        if payload or filename != names[2]:
            if sha256(directory / filename) != checksum[1]:
                raise ValueError(f"{archive}: bad checksum for {filename}")
    if (directory / "head-sha").read_text() != head + "\n":
        raise ValueError(f"{archive}: archive head mismatch")
    if (directory / "workspace").read_text() != workspace + "\n":
        raise ValueError(f"{archive}: archive workspace mismatch")
    return directory / names[3]


def verify(temp, prefix, names, selected, head, workspace, digests, read_cases):
    if selected not in names:
        raise ValueError("selected PostgreSQL archive is outside the build scope")
    downloads = temp / "pg-inventories"
    with tempfile.TemporaryDirectory(dir=temp) as verified:
        inventories = Path(verified)
        for name in names:
            directory = downloads / f"{prefix}-postgres-test-inventory-{name}"
            if len(names) == 1 and not directory.is_dir():
                # download-artifact also supports a directly extracted single result.
                directory = downloads
            listing = verify_bundle(directory, name, digests[name], head, workspace, payload=False)
            shutil.copyfile(listing, inventories / listing.name)
        # The original planner rejects duplicate IDs across archives and filters
        # the shared dispatch unit tests out of secondary archives.
        read_cases(inventories, optional=True)

    directory = temp / "pg-selected"
    listing = verify_bundle(directory, selected, digests[selected], head, workspace, payload=True)
    plan = temp / "pg-plan"
    binaries = temp / "pg-archives"
    plan.mkdir()
    binaries.mkdir()
    shutil.move(listing, plan / listing.name)
    payload = f"pg-{selected}.tar.zst"
    shutil.move(directory / payload, binaries / payload)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    select = commands.add_parser("route")
    select.add_argument("archives", type=archives)
    select.add_argument("index", type=int)
    select.add_argument("total", type=int)
    check = commands.add_parser("verify")
    check.add_argument("temp", type=Path)
    check.add_argument("prefix", choices=("pr", "mq"))
    check.add_argument("archives", type=archives)
    check.add_argument("selected")
    check.add_argument("head")
    check.add_argument("workspace")
    check.add_argument("planner", type=Path)
    args = parser.parse_args()
    if args.command == "route":
        archive, index, total = route(args.archives, args.index, args.total)
        with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
            print(f"archive={archive}\nindex={index}\ntotal={total}", file=output)
    else:
        digests = {name: os.environ[f"{name.replace('-', '_').upper()}_SHA256"]
                   for name in args.archives}
        read_cases = runpy.run_path(str(args.planner))["read_cases"]
        verify(args.temp, args.prefix, args.archives, args.selected, args.head,
               args.workspace, digests, read_cases)


if __name__ == "__main__":
    main()
