#!/usr/bin/env python3
"""Turn a node boot node.log (and the PostgreSQL activity samples) into a Chrome trace.

Usage:

  python3 tools/qualification/node_boot/timeline.py \
      --node-log node.log [--pg-activity postgres-world1-spells0/activity.log] \
      --out trace.json

Open trace.json in https://ui.perfetto.dev or chrome://tracing. Tracks:

- "durability holder": DURABILITY_PASS_SLOW / DURABILITY_HOLDER_STUCK (or their
  `event=` forms) as spans of held_ms ending at the line's time; holder_window
  as the counters held_ms, passes and refused; holder_usage and pass_dropped as
  instants carrying the caller.
- "connections": transport_* / connection_* / serve_end instants, and a span of
  `ms` per transport_close (one track per connection trace).
- "admission": admission_* events.
- "postgres pid N": one span per sampled transaction (xact_start_ms to the last
  sample that saw it), with the wait events and blocking pids seen meanwhile.

node.log `ts=` values are Unix milliseconds. A legacy DURABILITY_* line has no
`ts=`; it takes the previous timestamped line's time and gets `approx_ts=true`.
Only numbers, call sites and fixed labels are copied into event arguments.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys

FIELD = re.compile(r'\b([a-z_]+)=(\S+)')
SAFE_VALUE = re.compile(r'[A-Za-z0-9_.:/,-]{1,220}')
DROP_KEYS = {'account_id', 'character_id', 'world_id', 'channel_id', 'session_id',
             'game_session_id', 'database_url', 'url', 'password', 'error', 'query'}
LEGACY_SPANS = {'DURABILITY_PASS_SLOW': 'durability_pass_slow',
                'DURABILITY_HOLDER_STUCK': 'durability_holder_stuck'}
SPAN_EVENTS = {'durability_pass_slow', 'durability_holder_stuck', 'pass_slow', 'holder_stuck'}
PID = 1


def safe_args(fields: dict[str, str]) -> dict[str, str]:
    return {key: value for key, value in fields.items()
            if key not in DROP_KEYS and '://' not in value and SAFE_VALUE.fullmatch(value)}


def node_events(lines: list[str]) -> list[dict]:
    events: list[dict] = []
    last_ts: int | None = None
    for line in lines:
        marker = line.split(' ', 1)[0]
        fields = dict(FIELD.findall(line))
        if fields.get('ts', '').isdigit():
            last_ts = int(fields['ts'])
            approx = False
        else:
            approx = True
        event = LEGACY_SPANS.get(marker) or fields.get('event')
        if event is None or last_ts is None:
            continue
        args = safe_args({key: value for key, value in fields.items() if key not in ('ts', 'event')})
        if approx:
            args['approx_ts'] = 'true'
        ts_us = last_ts * 1000
        if event in SPAN_EVENTS and fields.get('held_ms', '').isdigit():
            held = int(fields['held_ms']) * 1000
            events.append({'name': f"{event} {fields.get('caller', '')}".strip(), 'ph': 'X',
                           'ts': ts_us - held, 'dur': held, 'pid': PID,
                           'tid': 'durability holder', 'args': args})
        elif event == 'holder_window':
            counters = {key: int(fields[key]) for key in ('held_ms', 'passes', 'refused')
                        if fields.get(key, '').isdigit()}
            events.append({'name': 'holder_window', 'ph': 'C', 'ts': ts_us, 'pid': PID,
                           'args': counters})
        elif event in ('holder_usage', 'pass_dropped'):
            events.append({'name': f"{event} {fields.get('caller', '')}".strip(), 'ph': 'i',
                           's': 't', 'ts': ts_us, 'pid': PID, 'tid': 'durability holder',
                           'args': args})
        elif event == 'transport_close' and fields.get('ms', '').isdigit():
            duration = int(fields['ms']) * 1000
            track = f"connection {fields.get('trace', '?')[:8]}"
            events.append({'name': f"connection {fields.get('outcome', '')}".strip(), 'ph': 'X',
                           'ts': ts_us - duration, 'dur': duration, 'pid': PID, 'tid': track,
                           'args': args})
        elif event.startswith(('transport_', 'connection_', 'listener_')) or event == 'serve_end':
            events.append({'name': event, 'ph': 'i', 's': 't', 'ts': ts_us, 'pid': PID,
                           'tid': 'connections', 'args': args})
        elif event.startswith('admission'):
            events.append({'name': event, 'ph': 'i', 's': 't', 'ts': ts_us, 'pid': PID,
                           'tid': 'admission', 'args': args})
    return events


def pg_events(lines: list[str]) -> list[dict]:
    transactions: dict[tuple[str, str], dict] = {}
    for line in lines:
        if not line.startswith('PG_ACTIVITY '):
            continue
        head, _, query = line.partition(' query=')
        fields = dict(FIELD.findall(head))
        pid, start, ts = fields.get('pid'), fields.get('xact_start_ms'), fields.get('ts')
        if not (pid and ts and ts.isdigit()):
            continue
        begin = int(start) if start and start.isdigit() else int(ts)
        key = (pid, str(begin))
        record = transactions.setdefault(key, {
            'begin': begin, 'end': int(ts), 'waits': [], 'blocked_by': [], 'queries': [],
            'user': fields.get('user', '-')})
        record['end'] = max(record['end'], int(ts))
        for name, value in (('waits', fields.get('wait', '-')),
                            ('blocked_by', fields.get('blocked_by', '-')),
                            ('queries', query[:160])):
            if value not in ('-', '-.-', '') and value not in record[name]:
                record[name].append(value)
    events = []
    for (pid, _), record in sorted(transactions.items(), key=lambda item: item[1]['begin']):
        waits = [wait for wait in record['waits'] if wait.startswith('Lock.')]
        name = 'xact blocked' if record['blocked_by'] else 'xact lock wait' if waits else 'xact'
        events.append({'name': name, 'ph': 'X', 'ts': record['begin'] * 1000,
                       'dur': max(1, record['end'] - record['begin']) * 1000, 'pid': PID,
                       'tid': f'postgres pid {pid}',
                       'args': {'user': record['user'], 'waits': ' '.join(record['waits']),
                                'blocked_by': ' '.join(record['blocked_by']),
                                'queries': ' | '.join(record['queries'][:5])}})
    return events


def convert(node_lines: list[str], pg_lines: list[str]) -> dict:
    events = node_events(node_lines) + pg_events(pg_lines)
    events.sort(key=lambda event: event['ts'])
    return {'traceEvents': events, 'displayTimeUnit': 'ms'}


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split('\n', 1)[0])
    parser.add_argument('--node-log', type=Path, required=True)
    parser.add_argument('--pg-activity', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(argv)

    def read(path: Path | None) -> list[str]:
        return path.read_text(encoding='utf-8', errors='replace').splitlines() if path else []
    trace = convert(read(args.node_log), read(args.pg_activity))
    args.out.write_text(json.dumps(trace, indent=0))
    print(f'NODE_BOOT_TIMELINE events={len(trace["traceEvents"])} out={args.out}')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
