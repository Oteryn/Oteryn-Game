"""CI-ready, offline export from the server's Rust CONTENT loader.

No normalized input or output snapshot is retained in the repository.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

from producer import build_snapshot, load_json_file, write_snapshot_files


def build(root: Path, output: Path) -> str:
    root = root.resolve()
    revision = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True
    ).strip()
    # A commit SHA may never attest dirty or untracked content.
    status = subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=all", "--", "content"],
        cwd=root, text=True,
    )
    if status:
        raise ValueError("content must match the declared source revision")
    generated_at = subprocess.check_output(
        ["git", "show", "-s", "--format=%cI", revision], cwd=root, text=True
    ).strip()
    generated_at = datetime.fromisoformat(generated_at).astimezone(timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )
    with tempfile.TemporaryDirectory(prefix="oteryn-public-catalog-") as temporary:
        source = Path(temporary) / "native-input.json"
        with source.open("wb") as stream:
            subprocess.run(
                ["cargo", "run", "--locked", "--quiet", "-p", "oteryn-gameplay-content",
                 "--bin", "oteryn-game-platform-catalog", "--",
                 "content/world", revision, generated_at],
                cwd=root, stdout=stream, check=True,
            )
        snapshot = build_snapshot(load_json_file(source))
        if subprocess.check_output(
            ["git", "status", "--porcelain", "--untracked-files=all", "--", "content"],
            cwd=root, text=True,
        ):
            raise ValueError("content changed during the catalogue build")
        write_snapshot_files(snapshot, output)
    return snapshot["payload_digest"]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    try:
        print(build(args.root, args.output))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"catalogue build refused: {str(error)[:512]}", file=sys.stderr)
        sys.exit(1)
