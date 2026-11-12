"""Regenerate explicit client-feature registration goldens from authored fragments."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("features", nargs="+")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2] / "crates/client-ui/src/features"
    for name in args.features:
        if Path(name).name != name or name in (".", ".."):
            parser.error("expected feature directory names without path components")
        feature = root / name
        sources = sorted((feature / "registrations").glob("*.json"))
        if not sources:
            parser.error(f"no registration fragments for {name}")
        entries = [json.loads(path.read_text(encoding="utf-8")) for path in sources]
        entries.sort(key=lambda entry: entry["id"])
        if any(entry["mechanic"] != name for entry in entries):
            parser.error(f"mechanic mismatch for {name}")
        (feature / "fixtures/registrations.json").write_text(
            json.dumps(entries, indent=2) + "\n", encoding="utf-8"
        )


if __name__ == "__main__":
    main()
