#!/usr/bin/env python3
"""Regenerate CP02 registration fixtures and remove only its supported baseline rows."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
UNITS = (
    "0098-quick-loot",
    "0132-inspection",
    "0200-team-finder-leader",
    "0232-team-finder-members",
    "0285-store-purchase-history",
)


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    for unit in UNITS:
        feature = ROOT / "crates/client-ui/src/features" / unit
        entries = [
            json.loads(path.read_text())
            for path in sorted((feature / "registrations").glob("*.json"))
        ]
        handlers = [entry for entry in entries if entry["kind"] == "handlers"]
        if not handlers or any(entry["status"] != "supported" for entry in handlers):
            raise ValueError(f"{unit} is not supported")
        write(feature / "fixtures/registrations.json", sorted(entries, key=lambda entry: entry["id"]))
    baseline = ROOT / "tools/architecture-check/client-unsupported-baseline.json"
    write(baseline, [unit for unit in json.loads(baseline.read_text()) if unit not in UNITS])
    for unit in UNITS:
        path = ROOT / "tools/architecture-check/client-unsupported-baseline.d" / f"{unit}.json"
        path.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
