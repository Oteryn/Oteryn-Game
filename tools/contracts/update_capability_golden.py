"""Regenerate one capability golden from its authored runtime fragment."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fragment", help="Runtime fragment filename, e.g. 0062-stash-v1.json")
    parser.add_argument("--previous", help="Previous golden filename when renumbering")
    parser.add_argument(
        "--client-registrations", action="store_true",
        help="Also regenerate this capability's client registration fixture",
    )
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
    if args.client_registrations:
        slug = re.sub(r"_V[0-9]+$", "", entry["name"]).lower().replace("_", "-")
        feature = root.parent / "client-ui/src/features" / f"{entry['id']:04d}-{slug}"
        registrations = [
            json.loads(path.read_text(encoding="utf-8"))
            for path in sorted((feature / "registrations").glob("*.json"))
        ]
        if not registrations:
            parser.error("client feature has no authored registrations")
        registrations.sort(key=lambda registration: registration["id"])
        directory = feature / "fixtures"
        directory.mkdir(exist_ok=True)
        (directory / "registrations.json").write_text(
            json.dumps(registrations, indent=2) + "\n", encoding="utf-8",
        )


if __name__ == "__main__":
    main()
