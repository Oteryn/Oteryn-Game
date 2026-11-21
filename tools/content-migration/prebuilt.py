#!/usr/bin/env python3
"""Build the two compatible WorldProject tools once for one exact candidate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
VARIANT = "world-project-dev-test-support-v1"
TARGETS = {
    "canonicalizer": {"name": "oteryn-world-project-canonicalize", "kind": "bin",
                      "features": ["test-support"]},
    "materializer": {"name": "materialize_content_world_project_v2", "kind": "example",
                     "features": []},
}
BUILD = (
    "cargo", "+1.100.0", "build", "--locked", "-p", "oteryn-game-server",
    "-p", "oteryn-gameplay-content", "--example", TARGETS["materializer"]["name"],
    "--bin", TARGETS["canonicalizer"]["name"],
    "--features", "oteryn-gameplay-content/test-support",
    "--message-format=json-render-diagnostics",
)


def identity(repo_root=ROOT):
    root = repo_root.resolve(strict=True)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    workspace = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], cwd=root, text=True,
    ).strip()
    if not re.fullmatch(r"[a-f0-9]{40}", head) or Path(workspace) != root:
        raise ValueError("invalid WorldProject checkout identity")
    # A HEAD receipt must not bless binaries built from unstaged/staged source edits.
    subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=root, check=True)
    return {"head": head, "workspace": workspace, "variant": VARIANT,
            "toolchain": "1.100.0", "profile": "dev"}


def canonical(path):
    if path.absolute() != path.resolve(strict=True):
        raise ValueError("WorldProject executable/manifest path must be canonical")
    return path.absolute()


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def executable(name, manifest_path, repo_root=ROOT):
    if name not in TARGETS:
        raise ValueError("unknown WorldProject executable")
    manifest_path = canonical(manifest_path)
    if not manifest_path.is_file() or not 0 < manifest_path.stat().st_size <= 8192:
        raise ValueError("invalid WorldProject manifest file")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    expected = identity(repo_root)
    if (not isinstance(manifest, dict) or manifest.keys() != {*expected, "executables"}
            or any(manifest[key] != value for key, value in expected.items())):
        raise ValueError("WorldProject executable does not match this source and build variant")
    entries = manifest["executables"]
    if not isinstance(entries, dict) or entries.keys() != TARGETS.keys():
        raise ValueError("wrong WorldProject executable inventory")
    for key, target in TARGETS.items():
        entry = entries[key]
        if (not isinstance(entry, dict) or entry.keys() != {*target, "sha256"}
                or any(entry[field] != value for field, value in target.items())
                or not isinstance(entry["sha256"], str)
                or not re.fullmatch(r"[a-f0-9]{64}", entry["sha256"])):
            raise ValueError("invalid WorldProject executable build binding")
    path = canonical(manifest_path.parent / TARGETS[name]["name"])
    if not path.is_file() or not os.access(path, os.X_OK):
        raise ValueError("WorldProject executable must be an executable regular file")
    if sha256(path) != entries[name]["sha256"]:
        raise ValueError("WorldProject executable checksum mismatch")
    return path


def build(output_dir, repo_root=ROOT):
    binding = identity(repo_root)
    # The server example already requires Content/test-support through dev-dependencies.
    # Select that exact closure explicitly for both tools in ONE Cargo resolution;
    # test-harness and release builds remain separate variants.
    output = subprocess.check_output(BUILD, cwd=repo_root, text=True)
    paths = {}
    by_target = {target["name"]: name for name, target in TARGETS.items()}
    for line in output.splitlines():
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact" or not message.get("executable"):
            continue
        target = message["target"]
        if target["name"] not in by_target:
            continue
        name = by_target[target["name"]]
        expected = TARGETS[name]
        if (name in paths or target["kind"] != [expected["kind"]]
                or message["features"] != expected["features"]
                or message["profile"]["test"] is not False):
            raise ValueError("unexpected or duplicate WorldProject Cargo executable")
        path = canonical(Path(message["executable"]))
        if not path.is_file() or not os.access(path, os.X_OK):
            raise ValueError("invalid WorldProject Cargo executable")
        paths[name] = path
    if paths.keys() != TARGETS.keys() or identity(repo_root) != binding:
        raise ValueError("incomplete WorldProject build or source changed during compilation")
    output_dir.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output_dir.parent) as temporary:
        staging = Path(temporary) / "bundle"
        staging.mkdir()
        manifest = dict(binding, executables={})
        for name, source in paths.items():
            target = TARGETS[name]
            destination = staging / target["name"]
            shutil.copyfile(source, destination)
            destination.chmod(0o755)
            manifest["executables"][name] = dict(target, sha256=sha256(destination))
        (staging / "manifest.json").write_text(json.dumps(manifest) + "\n", encoding="utf-8")
        if output_dir.exists() or output_dir.is_symlink():
            raise ValueError("WorldProject bundle output must be a new directory")
        staging.rename(output_dir)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("build").add_argument("directory", type=Path)
    verify = commands.add_parser("executable")
    verify.add_argument("name", choices=TARGETS)
    verify.add_argument("manifest", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        build(args.directory)
    else:
        print(executable(args.name, args.manifest))


if __name__ == "__main__":
    main()
