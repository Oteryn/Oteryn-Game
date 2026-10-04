#!/usr/bin/env python3
"""Generate the SPEED-1 step-speed table (CONDITIONS-0 §4.2).

The table maps every clamped effective speed (10..=65,535) to Canary's step speed
`floor(857.36 × ln(speed + 261.29) − 4795.01 + 0.5)` (Canary `04b83b51`, `creature.hpp:76-78,
1061-1067`), so the game server and the client read a u16 and never evaluate `ln`. It runs once,
offline; `--check` verifies the committed file.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

SCHEMA = "OTERYN_STEP_SPEED_TABLE/v1"
SPEED_MIN = 10
SPEED_MAX = 65_535
SPEED_A = 857.36
SPEED_B = 261.29
SPEED_C = -4795.01
PER_LINE = 16
REPO = Path(__file__).resolve().parents[3]
OUTPUT = REPO / "content" / "movement" / "step_speed_v1.json"


def step_speed(speed: int) -> int:
    """Canary's step speed of one effective speed (positive over the whole clamped range)."""
    value = math.floor(SPEED_A * math.log(speed + SPEED_B) + SPEED_C + 0.5)
    if not 1 <= value <= 0xFFFF:
        raise ValueError(f"step speed {value} of speed {speed} is not a positive u16")
    return value


def table() -> list[int]:
    return [step_speed(speed) for speed in range(SPEED_MIN, SPEED_MAX + 1)]


def digest(values: list[int]) -> str:
    """SHA-256 of the table as consecutive little-endian u16 values."""
    return hashlib.sha256(b"".join(v.to_bytes(2, "little") for v in values)).hexdigest()


def render(values: list[int]) -> str:
    header = {
        "schema": SCHEMA,
        "source": "Canary 04b83b51 creature.hpp:76-78,1061-1067 (OTS_HYPOTHESIS_ONLY)",
        "formula": "floor(857.36 * ln(speed + 261.29) - 4795.01 + 0.5)",
        "speed_min": SPEED_MIN,
        "speed_max": SPEED_MAX,
        "sha256_u16le": digest(values),
    }
    lines = ["{"]
    for key, value in header.items():
        lines.append(f"  {json.dumps(key)}: {json.dumps(value)},")
    rows = [
        ",".join(str(v) for v in values[start : start + PER_LINE])
        for start in range(0, len(values), PER_LINE)
    ]
    lines.append('  "step_speed": [')
    lines.append(",\n".join(f"    {row}" for row in rows))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify the committed table")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    text = render(table())
    if args.check:
        if args.output.read_text(encoding="utf-8") != text:
            print(f"{args.output} is not the generated table")
            return 1
        return 0
    args.output.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
