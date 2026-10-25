"""Regenerate one capability golden from its authored runtime fragment."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fragment", help="Runtime fragment filename, e.g. 0062-stash-v1.json")
    parser.add_argument("--previous", help="Previous golden filename when renumbering")
    args = parser.parse_args()
    for name in (args.fragment, args.previous):
        if name is not None and (Path(name).name != name or not name.endswith(".json")):
            parser.error("expected a JSON filename without directory components")
    root = Path(__file__).resolve().parents[2] / "crates/protocol-oteryn"
    source = root / "capabilities" / args.fragment
    entry = json.loads(source.read_text(encoding="utf-8"))
    expected_name = f"{entry['id']:04d}-{entry['name'].lower().replace('_', '-')}.json"
    if args.fragment != expected_name:
        parser.error(f"fragment must be named {expected_name}")
    directory = root / "src/tests/capabilities_golden"
    target = directory / args.fragment
    if args.previous is not None:
        previous = directory / args.previous
        old = json.loads(previous.read_text(encoding="utf-8"))
        if old["name"] != entry["name"]:
            parser.error("previous golden belongs to a different capability")
        if previous != target and target.exists():
            parser.error("renumbered golden already exists")
    target.write_text(json.dumps(entry, indent=2) + "\n", encoding="utf-8")
    if args.previous is not None and previous != target:
        previous.unlink()


if __name__ == "__main__":
    main()
