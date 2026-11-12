#!/usr/bin/env python3
"""Group the failed qualification rows of a node boot run by their root cause.

  failure_summary.py STAGE_LOG... [--summary FILE]

Reads `SEAM_EVIDENCE e2e_all scenario=<row> status=FAIL ...` lines. A row that
carries `root=<row>/<check>` is grouped by that root (`cause=blocked-by` rows
count as blocked). A row without one falls back to the text of its first
`*_GAP` line, then to the kind of its `evidence=` field. At most ten lines go to stdout; with --summary (run.sh passes
$GITHUB_STEP_SUMMARY when set) the same lines are appended as Markdown. Free
text is bounded and stripped of UUIDs, URLs and secret-bearing fragments.
"""
from __future__ import annotations

import argparse
from collections import OrderedDict
from pathlib import Path
import re
import sys

MAX_LINES = 10
FAIL_ROW = re.compile(r'\be2e_all scenario=(?P<scenario>\S+) status=FAIL\b(?P<rest>.*)$')
GAP = re.compile(r'\b[A-Z0-9]+_GAP (?:row=)?(?P<row>[A-Za-z0-9]+)\b[ :]*(?P<text>.*)$')
UUID = re.compile(r'\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b')
SECRET = re.compile(r'password|passwd|secret|token', re.IGNORECASE)
URL = re.compile(r'\S+://\S*')
SAFE_TOKEN = re.compile(r'[A-Za-z0-9_./:-]{1,80}')


def clean(text: str, limit: int = 100) -> str:
    if SECRET.search(text):
        return '<redacted>'
    text = ' '.join(URL.sub('<url>', UUID.sub('<uuid>', text)).split())
    return text if len(text) <= limit else text[:limit - 3] + '...'


def row_name(scenario: str) -> str:
    return scenario.split('.', 1)[0]


def summarize(lines: list[str]) -> list[str]:
    gaps: dict[str, str] = {}
    failures: list[tuple[str, dict[str, str]]] = []
    for line in lines:
        gap = GAP.search(line)
        if gap and gap['row'] not in gaps:
            gaps[gap['row']] = gap['text']
        failed = FAIL_ROW.search(line)
        if failed:
            fields = dict(re.findall(r'\b(cause|root|evidence)=(\S+)', failed['rest']))
            failures.append((row_name(failed['scenario']), fields))
    if not failures:
        return ['NODE_BOOT_ROOT_CAUSES failed_rows=0']
    groups: OrderedDict[str, dict] = OrderedDict()
    for row, fields in failures:
        root = fields.get('root')
        if root and SAFE_TOKEN.fullmatch(root):
            key = f'root={root}'
            cause = fields.get('cause', 'unknown')
        elif row in gaps:
            key = f'gap="{clean(gaps[row])}"'
            cause = 'unclassified'
        elif fields.get('evidence'):
            # `journey-aborted:<file>`: the row's journey ended before its checks ran.
            key = f"evidence={clean(fields['evidence'].split(':', 1)[0], 40)}"
            cause = 'unclassified'
        else:
            key = 'no-cause-recorded'
            cause = 'unclassified'
        group = groups.setdefault(key, {'causes': OrderedDict(), 'rows': [], 'blocked': 0})
        if cause == 'blocked-by':
            group['blocked'] += 1
        else:
            group['causes'][clean(cause, 24)] = None
        group['rows'].append(row)
    ranked = sorted(groups.items(), key=lambda item: len(item[1]['rows']), reverse=True)
    output = [f'NODE_BOOT_ROOT_CAUSES failed_rows={len(failures)} roots={len(groups)}']
    shown = ranked[:MAX_LINES - 2] if len(ranked) > MAX_LINES - 1 else ranked
    for key, group in shown:
        causes = ','.join(group['causes']) or 'blocked-by'
        rows = ','.join(group['rows'][:12]) + (',...' if len(group['rows']) > 12 else '')
        output.append(f'NODE_BOOT_ROOT_CAUSE {key} cause={causes} rows={len(group["rows"])} '
                      f'blocked={group["blocked"]} first={group["rows"][0]} members={rows}')
    if len(shown) < len(ranked):
        rest = ranked[len(shown):]
        output.append(f'NODE_BOOT_ROOT_CAUSE more_roots={len(rest)} '
                      f'rows={sum(len(group["rows"]) for _, group in rest)}')
    return output


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split('\n', 1)[0])
    parser.add_argument('logs', nargs='*', type=Path)
    parser.add_argument('--summary', type=Path)
    args = parser.parse_args(argv)
    lines: list[str] = []
    for path in args.logs:
        try:
            lines.extend(path.read_text(encoding='utf-8', errors='replace').splitlines())
        except FileNotFoundError:
            continue
    output = summarize(lines)
    print('\n'.join(output))
    if args.summary:
        with args.summary.open('a', encoding='utf-8') as stream:
            stream.write('### Node boot failed rows by root cause\n\n```\n'
                         + '\n'.join(output) + '\n```\n')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
