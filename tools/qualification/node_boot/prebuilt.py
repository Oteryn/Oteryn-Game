#!/usr/bin/env python3
"""Build and bind the executables shared by one Node boot workflow run."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

from matrix import ROOT, validate_shard


CARGO = ("cargo", "+1.100.0")
SERVER_BINS = ("oteryn-game-server", "oteryn-game-ops", "oteryn-game-migrate")
LIBTEST = "node-boot-libtest"
COMPILER = "oteryn-world-bundle-compiler"
MANIFEST = "manifest.json"
BUNDLES = {
    "default": (*SERVER_BINS, LIBTEST),
    "qualification-clock": (*SERVER_BINS, LIBTEST),
    "world-compiler": (COMPILER,),
}


def select(shard):
    if not isinstance(shard, dict):
        raise ValueError("invalid Node boot shard")
    validate_shard(shard)
    if shard["entry"] == "game_platform":
        # The portal wrapper forces World mode with the ordinary shipping clock.
        return "default", False, True
    env = shard["env"]
    world = env.get("NODE_BOOT_WORLD", "1") == "1" and env.get("NODE_BOOT_SPELLS") != "1"
    if not world:
        return "default", False, False
    return "qualification-clock", env.get("NODE_BOOT_SKIP_BASIC") != "1", True


def plan(shards):
    if not isinstance(shards, list) or not shards:
        raise ValueError("Node boot plan must contain at least one shard")
    needed = set()
    for shard in shards:
        variant, needs_default, needs_world = select(shard)
        needed.add(variant)
        if needs_default:
            needed.add("default")
        if needs_world:
            needed.add("world-compiler")
    return [variant for variant in BUNDLES if variant in needed]


def cargo_executables(package, args, kind, names, *, test=False):
    command, *options = args
    output = subprocess.check_output(
        [*CARGO, command, "--locked", "-p", package, *options,
         "--message-format=json-render-diagnostics"],
        cwd=ROOT, text=True,
    )
    paths = {}
    for line in output.splitlines():
        message = json.loads(line)
        if message["reason"] != "compiler-artifact" or not message.get("executable"):
            continue
        target = message["target"]
        name = target["name"]
        if target["kind"] != [kind] or message["profile"]["test"] != test or name not in names:
            continue
        executable = Path(message["executable"])
        if name in paths or not executable.is_file() or executable.is_symlink() or not os.access(executable, os.X_OK):
            raise ValueError(f"invalid Cargo executable for {name}")
        paths[name] = executable
    if paths.keys() != set(names):
        raise ValueError(f"missing Cargo executables: {sorted(set(names) - paths.keys())}")
    return paths


def identity(variant):
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    workspace = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], cwd=ROOT, text=True,
    ).strip()
    if not re.fullmatch(r"[a-f0-9]{40}", head) or Path(workspace) != ROOT:
        raise ValueError("invalid Node boot checkout identity")
    return {"head": head, "workspace": workspace, "variant": variant}


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def build(variant, output_dir):
    manifest = identity(variant)
    if variant == "world-compiler":
        paths = cargo_executables(COMPILER, ("build", "--release", "--bin", COMPILER), "bin", (COMPILER,))
    else:
        features = ("--features", "qualification-clock") if variant == "qualification-clock" else ()
        paths = cargo_executables("oteryn-game-server", ("build", "--bins", *features), "bin", SERVER_BINS)
        tests = cargo_executables(
            "oteryn-game-server", ("test", "--lib", "--no-run", *features), "lib", ("oteryn_game_server",), test=True,
        )
        paths[LIBTEST] = tests["oteryn_game_server"]
        listed = subprocess.check_output([paths[LIBTEST], "--list", "--ignored"], cwd=ROOT / "apps/game-server", text=True)
        expected = {
            "qualification::node_boot_seam_against_running_node: test",
            "qualification::node_boot_spells_against_running_node: test",
        }
        if not expected.issubset(listed.splitlines()):
            raise ValueError("Game library test executable lacks the Node boot tests")
    if identity(variant) != manifest:
        raise ValueError("checkout changed during the Node boot build")
    output_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output_dir) as work:
        work = Path(work)
        manifest["sha256"] = {name: sha256(path) for name, path in paths.items()}
        (work / MANIFEST).write_text(json.dumps(manifest) + "\n", encoding="utf-8")
        bundle = work / "bundle.tar.gz"
        with tarfile.open(bundle, "w:gz", compresslevel=1) as archive:
            archive.add(work / MANIFEST, arcname=MANIFEST)
            for name, path in paths.items():
                member = tarfile.TarInfo(name)
                member.size = path.stat().st_size
                member.mode = 0o755
                with path.open("rb") as stream:
                    archive.addfile(member, stream)
        bundle.replace(output_dir / bundle.name)


def verify_manifest(manifest, variant):
    expected = identity(variant)
    if not isinstance(manifest, dict) or manifest.keys() != {*expected, "sha256"}:
        raise ValueError("invalid Node boot manifest")
    if any(manifest[key] != value for key, value in expected.items()):
        raise ValueError("Node boot artifact does not match this checkout and build variant")
    digests = manifest["sha256"]
    if not isinstance(digests, dict) or digests.keys() != set(BUNDLES[variant]):
        raise ValueError("wrong Node boot executable inventory")
    if not all(isinstance(value, str) and re.fullmatch(r"[a-f0-9]{64}", value) for value in digests.values()):
        raise ValueError("invalid Node boot executable digests")
    return digests


def canonical(path):
    if path.absolute() != path.resolve():
        raise ValueError(f"Node boot artifact path is not canonical: {path}")
    return path.absolute()


def verify(variant, root):
    directory = canonical(root / variant)
    manifest = canonical(directory / MANIFEST)
    if not manifest.is_file() or not 0 < manifest.stat().st_size <= 4096:
        raise ValueError("invalid Node boot manifest file")
    digests = verify_manifest(json.loads(manifest.read_text(encoding="utf-8")), variant)
    for name, expected in digests.items():
        executable = canonical(directory / name)
        if not executable.is_file() or not os.access(executable, os.X_OK) or sha256(executable) != expected:
            raise ValueError(f"Node boot executable does not match the manifest: {name}")


def unpack(variant, archive_path, output_root):
    archive_path = canonical(archive_path)
    output_root = canonical(output_root)
    destination = output_root / variant
    if not archive_path.is_file() or destination.exists() or destination.is_symlink():
        raise ValueError("invalid Node boot archive or extraction destination")
    output_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    with tempfile.TemporaryDirectory(dir=output_root) as work:
        staging = Path(work)
        with tarfile.open(archive_path, "r|gz") as archive:
            header = archive.next()
            if (header is None or header.name != MANIFEST or header.type != tarfile.REGTYPE
                    or not 0 < header.size <= 4096):
                raise ValueError("invalid Node boot archive manifest member")
            manifest = json.load(archive.extractfile(header))
            digests = verify_manifest(manifest, variant)
            (staging / MANIFEST).write_text(json.dumps(manifest) + "\n", encoding="utf-8")
            seen = set()
            while (member := archive.next()) is not None:
                name = member.name
                if (name not in digests or name in seen or member.type != tarfile.REGTYPE
                        or not member.size or not member.mode & 0o111):
                    raise ValueError(f"invalid Node boot archive member: {name}")
                seen.add(name)
                executable = staging / name
                with archive.extractfile(member) as source, executable.open("xb") as target:
                    shutil.copyfileobj(source, target)
                if sha256(executable) != digests[name]:
                    raise ValueError(f"Node boot executable checksum mismatch: {name}")
                executable.chmod(0o700)
            if seen != digests.keys():
                raise ValueError("missing Node boot executable in archive")
        staging.rename(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    for command in ("build", "unpack", "verify"):
        subparser = subcommands.add_parser(command)
        subparser.add_argument("variant", choices=BUNDLES)
        if command == "unpack":
            subparser.add_argument("archive", type=Path)
        subparser.add_argument("directory", type=Path)
    subcommands.add_parser("plan").add_argument("shards_json", type=Path)
    subcommands.add_parser("select").add_argument("shard_json", type=json.loads)
    args = parser.parse_args()
    try:
        if args.command == "plan":
            shards = json.loads(args.shards_json.read_text(encoding="utf-8"))
            print(json.dumps(plan(shards), separators=(",", ":")))
        elif args.command == "select":
            variant, needs_default, needs_world = select(args.shard_json)
            print(f"main_variant={variant}")
            print(f"needs_default={str(needs_default).lower()}")
            print(f"needs_world={str(needs_world).lower()}")
        elif args.command == "build":
            build(args.variant, args.directory)
        elif args.command == "unpack":
            unpack(args.variant, args.archive, args.directory)
        else:
            verify(args.variant, args.directory)
    except (OSError, ValueError, subprocess.SubprocessError, tarfile.TarError) as error:
        parser.exit(1, f"Node boot prebuilt {args.command}: {error}\n")


if __name__ == "__main__":
    main()
