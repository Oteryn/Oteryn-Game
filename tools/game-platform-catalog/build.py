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


def build(root: Path, output: Path, *, preproduction: bool = False) -> str:
    root = root.resolve()
    output = output.resolve()
    revision = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True
    ).strip()
    generated_at = subprocess.check_output(
        ["git", "show", "-s", "--format=%cI", revision], cwd=root, text=True
    ).strip()
    generated_at = datetime.fromisoformat(generated_at).astimezone(timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )

    with tempfile.TemporaryDirectory(prefix="oteryn-public-catalog-") as temporary:
        checkout = Path(temporary) / "checkout"
        subprocess.run(
            ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.autocrlf=false",
             "worktree", "add", "--quiet", "--detach", checkout, revision],
            cwd=root, check=True,
        )
        try:
            # The world, adapter, workspace dependencies and producer all come
            # from the captured commit, even if the caller's checkout moves.
            source = Path(temporary) / "native-input.json"
            cargo_env = os.environ.copy()
            target_dir = Path(cargo_env.get("CARGO_TARGET_DIR", "target"))
            cargo_env["CARGO_TARGET_DIR"] = str((root / target_dir).resolve())
            command = [
                "cargo", "run", "--locked", "--quiet", "-p", "oteryn-gameplay-content",
                "--bin", "oteryn-game-platform-catalog", "--",
                "content/world", revision, generated_at,
            ]
            if preproduction:
                command.append("--preproduction")
            with source.open("wb") as stream:
                subprocess.run(command, cwd=checkout, env=cargo_env, stdout=stream, check=True)
            return subprocess.check_output(
                [sys.executable, "-I", "tools/game-platform-catalog/producer.py",
                 "produce", source, output],
                cwd=checkout, text=True,
            ).strip()
        finally:
            subprocess.run(
                ["git", "worktree", "remove", "--force", checkout], cwd=root, check=True
            )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--preproduction", action="store_true",
                        help="emit populated records for offline compatibility")
    args = parser.parse_args()
    try:
        print(build(args.root, args.output, preproduction=args.preproduction))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"catalogue build refused: {str(error)[:512]}", file=sys.stderr)
        sys.exit(1)
