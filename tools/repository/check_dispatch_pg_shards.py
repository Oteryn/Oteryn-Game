#!/usr/bin/env python3
"""Verify the generated dispatch PostgreSQL mounts without compiling the test crate."""

import argparse
from collections import Counter
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
DISPATCH = ROOT / "crates/gameplay/dispatch"
OWNER = DISPATCH / "src/gameplay_transport/monk_save_postgres_tests.rs"
SHARD_ENV = "OTERYN_DISPATCH_PG_SHARD"
MOUNT = re.compile(r'^#\[path\s*=\s*"([^"]+)"\s*\]([^\n]*)$', re.M)
MOD = re.compile(r"^(?:pub(?:\(.+?\))? )?mod (\w+);$")
INCLUDE = re.compile(r'^include!\("([^"]+)"\);$')
PRIMARY = "#[cfg(oteryn_pg_shard_primary)]"
TEST = re.compile(r"#\[(?:\w+::)?test(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*"
                  r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")
SINGLE_CASE = re.compile(r"^\s*(?:quest_case|native_case)!\(\s*(\w+)\s*,", re.M)
DAMAGE_CASES = re.compile(r"^\s*damage_cases!\((.*?)^\s*\);", re.M | re.S)
DAMAGE_PAIR = re.compile(r'\(\s*(\w+)\s*,\s*(\w+)\s*,\s*"(?:[^"\\]|\\.)*"\s*,?\s*\)')



def scan(path, primary, prefix=()):
    """Yield named tests and their specific module/include paths."""
    source = path.read_text(encoding="utf-8")
    names = [match[1] for match in TEST.finditer(source)
             if primary or not source[:match.start()].rstrip().endswith(PRIMARY)]
    # These macro calls name the tests they generate. A damage tuple generates two cases.
    names.extend(SINGLE_CASE.findall(source))
    for invocation in DAMAGE_CASES.finditer(source):
        for ordinary, native in DAMAGE_PAIR.findall(invocation[1]):
            names.extend((ordinary, native))
    yield path, tuple((*prefix, name) for name in names)
    override = None
    primary_only = False
    for line in map(str.strip, source.splitlines()):
        if attribute := MOUNT.fullmatch(line):
            override = path.parent / attribute[1]
            line = attribute[2].strip()
        if line in (PRIMARY, "#[cfg(not(oteryn_pg_shard_secondary))]"):
            primary_only = True
        elif included := INCLUDE.fullmatch(line):
            if primary or not primary_only:
                yield from scan((path.parent / included[1]).resolve(), primary, prefix)
            primary_only = False
        elif module := MOD.fullmatch(line):
            if primary or not primary_only:
                name = module[1]
                child = override or default_child(path, name)
                yield from scan(child.resolve(), primary, (*prefix, name))
            override = None
            primary_only = False
        elif line and not line.startswith(("#[", "//")):
            primary_only = False
            override = None


def default_child(path, name):
    parent = path.parent
    candidates = (directory / filename
                  for directory in (parent / path.stem, parent)
                  for filename in (f"{name}.rs", f"{name}/mod.rs"))
    return next(candidate for candidate in candidates if candidate.is_file())


def inventory(mounts, primary):
    files, cases = set(), Counter()
    for path in mounts:
        for child, names in scan(path, primary, (path.stem,)):
            files.add(child)
            cases.update(names)
    return files, cases


def verify_partition(expected, shards, label):
    observed = sum(shards, Counter())
    duplicates = sorted(name for name, count in (observed | expected).items() if count > 1)
    missing = sorted(expected.keys() - observed.keys())
    extra = sorted(observed.keys() - expected.keys())
    if duplicates or missing or extra:
        raise ValueError(f"{label}: {duplicates=} {missing=} {extra=}")


def mounts(dir, filename):
    return tuple(Path(path) for path, _ in MOUNT.findall((dir / filename).read_text()))


def check(count):
    with tempfile.TemporaryDirectory() as tmp:
        output = Path(tmp)
        build = output / "build-script"
        subprocess.run([os.environ.get("RUSTC", "rustc"), "--edition=2024",
                        "build.rs", "-o", str(build)], cwd=DISPATCH, check=True)
        env = dict(os.environ, CARGO_MANIFEST_DIR=str(DISPATCH), OUT_DIR=str(output))
        env.pop(SHARD_ENV, None)

        def generate(shard=None):
            if shard is not None:
                env[SHARD_ENV] = f"{shard}/{count}"
            result = subprocess.check_output([build], cwd=DISPATCH, env=env, text=True)
            primary = "cargo:rustc-cfg=oteryn_pg_shard_primary" in result.splitlines()
            if primary != (shard in (None, 1)):
                raise ValueError(f"incorrect primary cfg for shard {shard}")
            return primary, mounts(output, "postgres_tests_mounts.rs"), mounts(
                output, "postgres_test_support_mounts.rs"
            )

        primary, all_mounts, support = generate()
        support_files, support_tests = inventory(support, primary)
        if support_tests:
            raise ValueError(f"support contains tests: {sorted(support_tests)}")
        _, all_tests = inventory((*all_mounts, OWNER), primary)
        mount_sets, case_sets = [], []
        for shard in range(1, count + 1):
            primary, test_mounts, shared_mounts = generate(shard)
            if shared_mounts != support:
                raise ValueError(f"support mounts changed for shard {shard}")
            files, tests = inventory((*test_mounts, OWNER), primary)
            mount_sets.append(Counter(test_mounts))
            case_sets.append(tests)
            test_bytes = sum(path.stat().st_size for path in files - support_files)
            support_bytes = sum(path.stat().st_size for path in support_files)
            print(f"{shard}/{count}: {test_bytes:,} test bytes ({test_bytes / 1e6:.3f} MB), "
                  f"{support_bytes:,} shared bytes, {sum(tests.values()):,} named tests")
            if count == 3 and test_bytes > 5_500_000:
                raise ValueError(f"shard {shard} exceeds the 5.5 MB test source budget")
        verify_partition(Counter(all_mounts), mount_sets, "mounts")
        verify_partition(all_tests, case_sets, "tests")
        print(f"Union matches all {sum(all_tests.values()):,} named tests, zero duplicates")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("count", type=int)
    args = parser.parse_args()
    if args.count < 1:
        parser.error("count must be positive")
    check(args.count)


if __name__ == "__main__":
    main()
