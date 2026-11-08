"""Deterministic JSON with one compact record per array line."""
import json


def encode(document):
    def compact(value):
        return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

    def render(value):
        if isinstance(value, list) and value:
            return '[\n' + ',\n'.join(compact(record) for record in value) + '\n]'
        if isinstance(value, dict) and any(isinstance(child, (dict, list)) for child in value.values()):
            return '{\n' + ',\n'.join(compact(key) + ':' + render(child)
                                     for key, child in sorted(value.items())) + '\n}'
        return compact(value)

    return (render(document) + '\n').encode('utf-8')


if __name__ == '__main__':
    import argparse
    from pathlib import Path

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('paths', nargs='+', type=Path)
    args = parser.parse_args()
    for path in args.paths:
        raw = encode(json.loads(path.read_bytes()))
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit('JSON record layout drift: ' + str(path))
        else:
            path.write_bytes(raw)
