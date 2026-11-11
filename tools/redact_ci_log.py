#!/usr/bin/env python3
"""Drop potentially secret-bearing lines from a CI log."""

from __future__ import annotations

import argparse
from pathlib import Path
import re


SENSITIVE = re.compile(r"password|postgres://|token=|grant", re.IGNORECASE)


def redact(source: Path, destination: Path) -> None:
    with source.open(encoding="utf-8", errors="replace") as input_file:
        with destination.open("w", encoding="utf-8") as output_file:
            for line in input_file:
                if not SENSITIVE.search(line):
                    output_file.write(line)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    redact(args.source, args.destination)


if __name__ == "__main__":
    main()
