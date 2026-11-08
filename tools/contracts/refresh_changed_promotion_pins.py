"""Refresh Rust digest constants for regenerated evidence on a bounded CI base.

Only exact old packet digests in constant declarations are replaced. Frozen
receipts and qualification decisions are not rewritten by this tool.
"""

import argparse
import hashlib
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = "4620c6b8e12f6a9dd827bff3ca6dbd273ccd5c1f"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=BASE)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.base):
        raise ValueError("base must be an exact commit SHA")
    paths = subprocess.check_output(
        ["git", "diff", "--name-only", args.base, "--", "docs/agents/evidence"],
        cwd=ROOT,
        text=True,
    ).splitlines()
    replacements = {}
    for name in paths:
        path = ROOT / name
        if path.suffix != ".json" or not path.is_file():
            continue
        old = subprocess.run(
            ["git", "show", f"{args.base}:{name}"],
            cwd=ROOT,
            capture_output=True,
            check=False,
        )
        if old.returncode == 0:
            before = hashlib.sha256(old.stdout).hexdigest()
            after = hashlib.sha256(path.read_bytes()).hexdigest()
            if before != after:
                replacements[before] = after
    pattern = re.compile(
        r'(\bconst\s+([A-Z0-9_]+)\s*:\s*&str\s*=\s*")([0-9a-f]{64})(";)'
    )
    for path in (ROOT / "crates/gameplay/content-promotion/src").rglob("*.rs"):
        text = path.read_text()
        # Bind a pin to its include_bytes path, so repeated regeneration also
        # updates an intermediate digest rather than relying only on the base.
        includes = re.finditer(
            r"\bconst\s+([A-Z0-9_]+)\s*:\s*&\[u8\]\s*=\s*"
            r'include_bytes!\(\s*"([^"]+)"\s*\);',
            text,
        )
        for include in includes:
            packet = (path.parent / include[2]).resolve()
            if not packet.is_relative_to(ROOT / "docs/agents/evidence"):
                continue
            name = include[1] + "_SHA256"
            pin = re.search(rf'\bconst\s+{name}:\s*&str\s*=\s*"([0-9a-f]{{64}})"', text)
            if pin:
                actual = hashlib.sha256(packet.read_bytes()).hexdigest()
                if pin[1] != actual:
                    replacements[pin[1]] = actual

        def replace(match, path=path):
            old = match[3]
            if old not in replacements:
                return match[0]
            print(f"refresh {path.relative_to(ROOT)}: {match[2]}")
            return match[1] + replacements[old] + match[4]

        updated = pattern.sub(replace, text)
        if updated != text:
            path.write_text(updated)
    # The Item-tree equivalence validator pins the same produced packet bytes.
    # Preserve each equality predicate and update only its exact digest literal.
    path = ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py"
    text = path.read_text()
    pattern = re.compile(r'(hexdigest\(\)\s*==\s*")([0-9a-f]{64})(")')

    def replace_validator(match):
        if match[2] not in replacements:
            return match[0]
        print(f"refresh {path.relative_to(ROOT)}: packet digest {match[2]}")
        return match[1] + replacements[match[2]] + match[3]

    updated = pattern.sub(replace_validator, text)
    if updated != text:
        path.write_text(updated)


if __name__ == "__main__":
    main()
