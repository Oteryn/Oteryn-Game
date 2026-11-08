"""Recount unchanged captured regions using Main's closed XML-membership evidence."""
import argparse
import json
import re
import subprocess

import convert_appearance_only_ids as appearance
import refresh_palette_capture as capture


def closed_membership(before, after, sample, declared, client_ids):
    if [r['source_item_id'] for r in before] != [r['source_item_id'] for r in after]:
        raise ValueError('captured palette identities changed')
    old = {r['source_item_id'] for r in before if r['provisional']}
    current = {r['source_item_id'] for r in after if r['provisional']}
    represented = {r['id'] for r in sample['rows']} | declared
    if represented - old or (old - represented) & set(client_ids):
        raise ValueError('captured XML membership is incomplete')
    if not current <= old or not declared <= current:
        raise ValueError('captured provisional membership changed outside closed scope')
    return set(declared)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True)
    args = parser.parse_args()
    if not re.fullmatch('[0-9a-f]{40}', args.base):
        raise ValueError('base must be an exact Main commit')
    root = capture.base.ROOT
    subprocess.run(['git', 'merge-base', '--is-ancestor', args.base, 'origin/main'], cwd=root, check=True)
    def old(path):
        return json.loads(subprocess.check_output(['git', 'show', args.base + ':' + path], cwd=root))
    index_path = appearance.PLACEMENTS_INDEX
    before, after = old(index_path), json.loads((root / index_path).read_bytes())
    if before['regions'] != after['regions']:
        raise ValueError('captured map regions changed')
    summary = str(capture.base.SUMMARY.relative_to(root))
    if old(summary)['source'] != json.loads((root / summary).read_bytes())['source']:
        raise ValueError('captured source identity changed')
    declared_ids = appearance.PINNED_DECLARED_PROVISIONAL
    if old(summary)['palette']['provisional']['in_items_xml']['entries'] != len(declared_ids):
        raise ValueError('captured XML membership count changed')
    declared = closed_membership(before['palette'], after['palette'], old(str(appearance.SAMPLE.relative_to(root))), declared_ids, appearance.load_client(root)[0])
    for path, data in {**capture.outputs_from_declared(declared, root), **appearance.build(declared, root)}.items():
        (root / path).write_bytes(data)


if __name__ == '__main__':
    main()
