"""Admit the official, fixed grounds used by the pinned Crystal spawn delta."""

import argparse

from crystal_delta_ground import ROOT, outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f"stale Crystal spawn ground: {name}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
