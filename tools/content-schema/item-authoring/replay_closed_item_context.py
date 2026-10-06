"""Replay closed Item packets; retain current Item, binding and owner guards.

The retained World inputs predate MAP-KIND-CLASS-1. The current World tree is
checked independently before its historical byte view is used for reproduction.
The retained Rust file is only an old compiler input witness, never executed or
used to qualify the current runtime. Current runtime tests remain separate.
"""

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import zipfile
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CONTEXT = (
    "docs/agents/evidence/monster-final-seven-20261004/item-closed-context-20261005"
)
CONTEXT_SHA = "3139ccfbfcd9968b2d4138b271244e525e4be2dddaeefc20acc7c23c1a4db6ec"
RUST = "apps/game-server/src/content/reference_playable.rs"
TOOL = "tools/content-schema/item-authoring"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def checked_inputs(root=ROOT):
    raw = (root / CONTEXT / "context.json").read_bytes()
    if sha(raw) != CONTEXT_SHA:
        raise ValueError("closed Item context digest drift")
    c = json.loads(raw)
    if c["purpose"] != "HISTORICAL_INPUT_REPLAY_NOT_RUNTIME_AUTHORITY":
        raise ValueError("closed Item context purpose drift")
    archive = root / CONTEXT / "retained-inputs.zip"
    if sha(archive.read_bytes()) != c["archive_sha256"]:
        raise ValueError("closed Item archive digest drift")
    current_maps = {
        str(p.relative_to(root)): sha(p.read_bytes())
        for family in ("objects", "terrain")
        for p in (root / f"content/world/{family}").glob("*.json")
    }
    if current_maps != c["current_world_inputs"]:
        raise ValueError("current World qualification inputs drift")
    retained = {}
    with zipfile.ZipFile(archive) as z:
        if set(z.namelist()) != set(c["retained_inputs"]):
            raise ValueError("closed Item retained input scope drift")
        for path, pin in c["retained_inputs"].items():
            data = z.read(path)
            if (
                sha(data) != pin["historical_sha256"]
                or hashlib.sha1(
                    b"blob " + str(len(data)).encode() + b"\0" + data
                ).hexdigest()
                != pin["git_blob_oid"]
                or sha((root / path).read_bytes()) != pin["current_sha256"]
            ):
                raise ValueError(f"closed/current input witness drift: {path}")
            retained[path] = data

    # The historical view must not change which current Item keys are routed to
    # World. Every existing producer still evaluates current native definitions,
    # source bindings and the sealed 411/700 authoring-owner closure.
    def routed(historical):
        rows = []
        for path in current_maps:
            data = retained.get(path) if historical else None
            document = json.loads(data or (root / path).read_bytes())
            rows.extend(
                (path, r.get("identity"), r["provenance"]["item_pointer"])
                for r in document.get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
        return rows

    if routed(True) != routed(False):
        raise ValueError("historical/current World Item routing differs")
    historical_maps = current_maps | {
        p: sha(data) for p, data in retained.items() if p != RUST
    }
    if historical_maps != c["historical_world_inputs"]:
        raise ValueError("closed historical World input closure drift")
    return retained


def mirror(source, target, retained, prefix=""):
    for p in source.iterdir():
        if p.name in {".git", "__pycache__"}:
            continue
        rel = f"{prefix}/{p.name}" if prefix else p.name
        dest = target / p.name
        if rel in retained:
            dest.write_bytes(retained[rel])
        elif rel == TOOL:
            # Existing tests/replay helpers rebuild writable schemas/templates.
            # Detached copies prevent their writes reaching the product tree.
            shutil.copytree(p, dest, ignore=shutil.ignore_patterns("__pycache__"))
        elif p.is_dir() and (
            rel == "tools"
            or rel.startswith("tools/")
            or any(k.startswith(rel + "/") for k in (*retained, TOOL))
        ):
            dest.mkdir()
            mirror(p, dest, retained, rel)
        else:
            dest.symlink_to(p.resolve(), target_is_directory=p.is_dir())


@contextmanager
def qualification_context(root=ROOT):
    retained = checked_inputs(root)
    with tempfile.TemporaryDirectory(
        prefix="oteryn-closed-item-", dir=root.parent
    ) as directory:
        path = Path(directory)
        mirror(root, path, retained)
        yield path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        parser.error("provide a historical Item qualification script")
    with qualification_context() as root:
        return subprocess.run(
            [sys.executable, *command], cwd=root / TOOL, check=False
        ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
