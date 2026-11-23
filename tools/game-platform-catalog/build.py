"""Build the committed Game catalogue artifact.

Use --preproduction to emit populated records for offline Platform compatibility.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

# Cargo opens all workspace member manifests. The catalogue also reads the world
# project and compiles the data embedded by the content crate and its dependencies.
BUILD_INPUTS = (
    ".cargo",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "apps",
    "crates",
    "tests",
    "vendor",
    "content",
    "rulesets",
    "docs/contracts",
    "docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json",
    "docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json",
    "imports/crystalserver/bindings",
    "imports/crystalserver/grimvale-full-moon-maps.json",
    "imports/official/appearance-membership",
    "tools/architecture-check",
    "tools/content-schema/craft-authoring/samples",
    "tools/content-schema/item-authoring/samples",
    "tools/content-schema/native-gameplay",
    "tools/content-schema/reward-claim-authoring",
    "tools/content-schema/spell-authoring/samples",
    "tools/dev-client",
    "tools/game-platform-catalog/producer.py",
    "tools/oteryn-game-ops-top",
    "tools/player-bots",
    "tools/synthetic-asset-compiler",
    "tools/synthetic-client-harness",
    "tools/world-bundle-compiler",
)


def _materialize_tree(root: Path, checkout: Path, revision: str, git_env: dict) -> None:
    tree = subprocess.check_output(
        ["git", "ls-tree", "-rz", "--full-tree", revision, "--", *BUILD_INPUTS],
        cwd=root,
        env=git_env,
    )
    entries = []
    with tempfile.TemporaryFile() as requests:
        for entry in tree.split(b"\0")[:-1]:
            metadata, path = entry.split(b"\t", 1)
            mode, kind, oid = metadata.split()
            relative = Path(os.fsdecode(path))
            if (
                kind != b"blob"
                or mode not in (b"100644", b"100755")
                or relative.is_absolute()
                or ".." in relative.parts
            ):
                raise ValueError(f"unsupported Git tree entry: {relative}")
            entries.append((mode, oid, checkout / relative))
            requests.write(oid + b"\n")
        requests.seek(0)

        # Read raw blobs instead of applying checkout attributes or filters.
        with (
            subprocess.Popen(
                ["git", "cat-file", "--batch"],
                cwd=root,
                env=git_env,
                stdin=requests,
                stdout=subprocess.PIPE,
            ) as objects,
            objects.stdout as stream,
        ):
            for mode, oid, path in entries:
                header = stream.readline().split()
                if len(header) != 3 or header[:2] != [oid, b"blob"]:
                    raise ValueError(f"cannot read committed blob: {path}")
                size = int(header[2])
                data = stream.read(size)
                if len(data) != size or stream.read(1) != b"\n":
                    raise ValueError(f"truncated committed blob: {path}")
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
                if mode == b"100755":
                    path.chmod(path.stat().st_mode | 0o111)
            if objects.wait():
                raise subprocess.CalledProcessError(objects.returncode, objects.args)


def build(root: Path, output: Path, *, preproduction: bool = False) -> str:
    root = root.resolve()
    # Keep the public basename when it already points to a published generation.
    output = output.parent.resolve() / output.name
    # Read the original object and tree identified by the attested commit SHA.
    git_env = {**os.environ, "GIT_NO_REPLACE_OBJECTS": "1"}
    revision = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, env=git_env, text=True
    ).strip()
    generated_at = subprocess.check_output(
        ["git", "show", "-s", "--format=%cI", revision],
        cwd=root,
        env=git_env,
        text=True,
    ).strip()
    generated_at = (
        datetime.fromisoformat(generated_at)
        .astimezone(timezone.utc)
        .strftime("%Y-%m-%dT%H:%M:%SZ")
    )

    # Build on the output volume so compilation does not fill a system TMPDIR.
    with tempfile.TemporaryDirectory(
        prefix=".oteryn-public-catalog-", dir=output.parent
    ) as temporary:
        directory = Path(temporary)
        checkout = directory / "checkout"
        _materialize_tree(root, checkout, revision, git_env)
        # The world, adapter, workspace dependencies and producer all come
        # from the captured commit, even if the caller's checkout moves.
        source = directory / "native-input.json"
        # Give this invocation its own executable path through launch.
        target_dir = directory / "target"
        command = [
            "cargo",
            "run",
            "--locked",
            "--quiet",
            "-p",
            "oteryn-gameplay-content",
            "--bin",
            "oteryn-game-platform-catalog",
            "--target-dir",
            target_dir,
            "--",
            "content/world",
            revision,
            generated_at,
        ]
        if preproduction:
            command.append("--preproduction")
        with source.open("wb") as stream:
            subprocess.run(command, cwd=checkout, stdout=stream, check=True)
        return subprocess.check_output(
            [
                sys.executable,
                "-I",
                "tools/game-platform-catalog/producer.py",
                "produce",
                source,
                output,
            ],
            cwd=checkout,
            text=True,
        ).strip()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--preproduction",
        action="store_true",
        help="emit populated records for offline compatibility",
    )
    args = parser.parse_args()
    try:
        print(build(args.root, args.output, preproduction=args.preproduction))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"catalogue build refused: {str(error)[:512]}", file=sys.stderr)
        sys.exit(1)
