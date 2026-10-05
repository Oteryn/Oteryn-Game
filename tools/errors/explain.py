#!/usr/bin/env python3
"""Explain an Oteryn Game error code (ARCH-ERROR-CODES-0 §1.7).

    explain.py E3004 | 3004 | ADMISSION_GRANT_EXPIRED
    explain.py --scan FILE
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import registry  # noqa: E402

CODE_RE = re.compile(r"\bcode=E(\d{4,})\b")


def load_entries(root: Path = registry.ROOT) -> dict[int, dict]:
    """Every registered code from both registries, normalised."""
    entries: dict[int, dict] = {}
    protocol = registry.load_json(root / registry.PROTOCOL_PATH)
    for raw in protocol["error_codes"]:
        derived = registry.derived_progression(raw)
        progression, retry, is_derived = derived if derived else (None, None, False)
        entry = dict(raw)
        entry.update(
            registry_name="protocol",
            progression=progression,
            progression_derived=is_derived,
            retry_requires=retry,
        )
        entries[raw["code"]] = entry
    game = registry.load_json(root / registry.GAME_PATH)
    for raw in game["codes"]:
        entry = dict(raw)
        entry.update(registry_name="game", progression_derived=False)
        entries[raw["code"]] = entry
    return entries


def lookup(entries: dict[int, dict], token: str):
    text = token.strip().upper()
    if re.fullmatch(r"E?\d+", text):
        return entries.get(int(text.lstrip("E")))
    for entry in entries.values():
        if entry["name"] == text:
            return entry
    return None


def format_entry(entry: dict) -> str:
    progression = entry["progression"] or "UNKNOWN"
    if entry["progression_derived"]:
        progression += " (derived from " + entry["default_disposition"] + ")"
    lines = [
        f"E{entry['code']:04d} {entry['name']}",
        f"  category:    {entry['category']}",
        f"  progression: {progression}",
    ]
    if entry.get("retry_requires"):
        lines.append(f"  retry needs: {entry['retry_requires']}")
    if "default_disposition" in entry:
        lines.append(f"  disposition: {entry['default_disposition']}")
    lines.append(f"  registry:    {entry['registry_name']}")
    for key in ("status", "owner", "contract", "public_class", "hint"):
        if entry.get(key) is not None:
            lines.append(f"  {key + ':':<12} {entry[key]}")
    return "\n".join(lines)


def scan(entries: dict[int, dict], text: str) -> list[str]:
    counts = Counter(int(n) for n in CODE_RE.findall(text))
    out = []
    for number, count in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])):
        entry = entries.get(number)
        if entry is None:
            out.append(f"{count:>6}  E{number:04d} (unregistered)")
        else:
            hint = entry.get("hint") or ""
            out.append(f"{count:>6}  E{number:04d} {entry['name']}  {hint}".rstrip())
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("code", nargs="?", help="E3004, 3004 or a code name")
    parser.add_argument("--scan", metavar="FILE", help="count the codes found in a log")
    args = parser.parse_args(argv)
    entries = load_entries()
    if args.scan:
        lines = scan(entries, Path(args.scan).read_text(encoding="utf-8", errors="replace"))
        print("\n".join(lines) if lines else "no codes found")
        return 0
    if not args.code:
        parser.error("give a code or --scan FILE")
    entry = lookup(entries, args.code)
    if entry is None:
        print(f"unregistered code: {args.code}", file=sys.stderr)
        return 1
    print(format_entry(entry))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
