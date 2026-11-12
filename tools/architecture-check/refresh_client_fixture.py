"""Regenerate one native feature's registration golden from its fragments."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("feature", help="Feature directory name, e.g. 0170-arena")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    features = root / "crates/client-ui/src/features"
    feature = features / args.feature
    if feature.parent != features or not (feature / "registrations").is_dir():
        parser.error("expected an existing feature directory name")
    entries = [
        json.loads(path.read_text())
        for path in sorted((feature / "registrations").glob("*.json"))
    ]
    if not entries or any(entry.get("mechanic") != args.feature for entry in entries):
        parser.error("expected registration fragments for this feature")
    (feature / "fixtures/registrations.json").write_text(
        json.dumps(entries, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
