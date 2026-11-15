#!/usr/bin/env python3
"""Optional check against external reference inputs; no private paths or fixtures bundled."""
import argparse
import json
import pathlib
import sys

import extract


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', required=True, type=pathlib.Path)
    parser.add_argument('--inventory', required=True, type=pathlib.Path)
    parser.add_argument('--style', required=True)
    args = parser.parse_args()
    try:
        data = extract.extract(args.sources, args.inventory, args.style)
        facts = list(data['style_tokens'].values()) + data['rule_evidence'] + data['asset_references']
        for font in data['font_definitions'].values():
            facts.append(font)
            facts.extend(font['attributes'].values())
        for fact in facts:
            source = fact['source']
            lines = (args.sources / source['file']).read_text(encoding='utf-8').splitlines()
            if not 1 <= source['line'] <= len(lines):
                raise extract.InputError(f'{source}: evidence line outside source')
        print(json.dumps({'verified_sources': len(data['sources']), 'checked_evidence_locations': len(facts),
                          'style_tokens': len(data['style_tokens']), 'font_definitions': len(data['font_definitions'])}))
    except (extract.InputError, OSError, json.JSONDecodeError) as exc:
        print(f'reference check failed: {exc}', file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
