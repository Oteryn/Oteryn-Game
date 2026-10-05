#!/usr/bin/env python3
"""Produce and check the Character progression ruleset files (ARCH-PROGRESSION-SOURCE-0, A1).

  build_progression.py --check [--samples levels.txt]
  build_progression.py --write --recorded-on YYYY-MM-DD   (first write of evidence.json)
  build_progression.py --write                           (regenerate from committed evidence.json)

The table is generated from the structured formula constants in exact integers. --samples
cross-checks owner-supplied `<level>,<experience>` lines (ascending, any subset); the samples
are never committed. A mismatch is a BLOCKER and nothing is written.
"""
import argparse
import sys
from pathlib import Path

import progression_lib as lib


def derived(evidence):
    return {
        "evidence": evidence,
        "table": lib.build_table(evidence),
        "death": lib.build_death(),
        "reward": lib.build_reward(),
        "differences": lib.build_differences(),
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    parser.add_argument("--samples", type=Path)
    parser.add_argument("--recorded-on")
    parser.add_argument("--section-out", type=Path, help="write the canonical native gameplay progression section")
    args = parser.parse_args(argv)
    try:
        if args.samples:
            print(f"samples ok: {lib.check_samples(args.samples)} levels equal the formula")
        if args.write:
            path = lib.PATHS["evidence"]
            if args.recorded_on:
                evidence = lib.build_evidence(args.recorded_on)
            elif path.exists():
                evidence = lib.strict_loads(path.read_bytes())
            else:
                parser.error("--write needs --recorded-on when evidence.json does not exist")
            docs = derived(evidence)
            path.parent.mkdir(parents=True, exist_ok=True)
            for kind, doc in docs.items():
                lib.PATHS[kind].write_bytes(lib.pretty(doc))
        docs = lib.load_docs()
        lib.verify_documents(docs)
        if args.section_out:
            args.section_out.write_bytes(lib.canonical(lib.build_section(docs)))
        print("progression content ok:", lib.revisions(docs)["policy_revision"])
        return 0
    except lib.ProgressionError as exc:
        print(f"REFUSED: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
