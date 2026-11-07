"""Offline reconstruction of schema constants, not a fresh donor verification."""
import argparse
import json
from pathlib import Path
from source_npc_exchange import schema_update


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--schema', type=Path, required=True)
    p.add_argument('--profiles', type=Path, required=True)
    p.add_argument('--check', action='store_true')
    a = p.parse_args()
    encoded = json.dumps(schema_update(json.loads(a.schema.read_text()),
        json.loads(a.profiles.read_text())), ensure_ascii=False, indent=2) + '\n'
    if a.check:
        if a.schema.read_text() != encoded:
            raise SystemExit('stale NPC exchange schema constants')
    else:
        a.schema.write_text(encoded)


if __name__ == '__main__':
    main()
