#!/usr/bin/env python3
"""PostgreSQL observability for the disposable node boot database.

run.sh starts the container with lock-wait, slow-statement and
pg_stat_statements settings and then uses two commands:

  pg_observe.py sample --container C --user U --database D --out FILE
      Every 2 s, append one PG_ACTIVITY line per backend of database D that
      waits on a lock, is blocked, is idle in a transaction or has a
      transaction older than 300 ms. Stops on SIGTERM/SIGINT.

  pg_observe.py dump --container C --user U --database D --stage S --dir DIR
                     [--activity FILE] [--verbose]
      At the end of a stage: write the top-20 statements by total time
      (statements-S.txt, then reset the counters), append the new lock-wait,
      deadlock and slow-statement server log lines (server-lock-slow.log) and
      print one NODE_BOOT_PG summary line; --verbose (a failed stage) also
      prints a bounded excerpt.

Query text comes only from pg_stat_statements' normalized text, and every SQL
fragment, including server log statements, passes through redact_sql: string,
dollar-quoted and numeric literals are replaced, UUIDs masked, any fragment
naming a password or URL dropped, and the length bounded. Parameter values are
never logged (the server runs with log_parameter_max_length=0).
"""
from __future__ import annotations

import argparse
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

TOP_STATEMENTS = 20
ACTIVITY_LIMIT = 20000
EXCERPT = 10

UUID = re.compile(r'\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b')
SECRET = re.compile(r'password|passwd|secret|://', re.IGNORECASE)
STRING_LITERAL = re.compile(r"(?:\b[EeBbXxUu]&?)?'(?:[^']|'')*'")
DOLLAR_LITERAL = re.compile(r'\$([A-Za-z_][A-Za-z0-9_]*)?\$.*?\$\1\$', re.DOTALL)
NUMERIC_LITERAL = re.compile(r'(?<![\w$.])\d+(?:\.\d+)?(?:[eE][+-]?\d+)?(?![\w.])')


def redact_sql(text: str, limit: int = 200) -> str:
    """Normalize one SQL fragment so that it carries no literal value."""
    if SECRET.search(text):
        return '<redacted>'
    text = UUID.sub('<uuid>', text)
    text = DOLLAR_LITERAL.sub('$$?$$', text)
    text = STRING_LITERAL.sub("'?'", text)
    text = NUMERIC_LITERAL.sub('?', text)
    text = ' '.join(text.split())
    # An unterminated literal (a truncated fragment) keeps nothing after its quote.
    if text.count("'") % 2:
        text = text[:text.rfind("'")] + "'?"
    return text if len(text) <= limit else text[:limit - 3] + '...'


ACTIVITY_SQL = r"""
SELECT 'PG_ACTIVITY ts=' || (extract(epoch FROM clock_timestamp()) * 1000)::bigint
  || ' pid=' || a.pid
  || ' user=' || coalesce(a.usename::text, '-')
  || ' state=' || replace(coalesce(a.state, '-'), ' ', '_')
  || ' wait=' || coalesce(a.wait_event_type, '-') || '.' || coalesce(a.wait_event, '-')
  || ' xact_start_ms=' || coalesce(((extract(epoch FROM a.xact_start) * 1000)::bigint)::text, '-')
  || ' xact_ms=' || coalesce(((extract(epoch FROM clock_timestamp() - a.xact_start) * 1000)::bigint)::text, '-')
  || ' state_ms=' || coalesce(((extract(epoch FROM clock_timestamp() - a.state_change) * 1000)::bigint)::text, '-')
  || ' blocked_by=' || coalesce(nullif(array_to_string(pg_blocking_pids(a.pid), ','), ''), '-')
  || ' query_id=' || coalesce(a.query_id::text, '-')
  || ' query=' || coalesce(regexp_replace(left(s.query, 400), '\s+', ' ', 'g'), '-')
FROM pg_stat_activity a
LEFT JOIN LATERAL (
  SELECT p.query FROM pg_stat_statements p WHERE p.queryid = a.query_id LIMIT 1
) s ON true
WHERE a.datname = :'db' AND a.pid <> pg_backend_pid() AND a.backend_type = 'client backend'
  AND (a.wait_event_type = 'Lock'
       OR a.state LIKE 'idle in transaction%'
       OR clock_timestamp() - a.xact_start > interval '300 milliseconds'
       OR cardinality(pg_blocking_pids(a.pid)) > 0)
ORDER BY a.pid;
"""

STATEMENTS_SQL = r"""
SELECT 'calls=' || p.calls
  || ' total_ms=' || round(p.total_exec_time)::bigint
  || ' mean_ms=' || round(p.mean_exec_time::numeric, 1)
  || ' max_ms=' || round(p.max_exec_time)::bigint
  || ' rows=' || p.rows
  || ' query=' || regexp_replace(left(p.query, 400), '\s+', ' ', 'g')
FROM pg_stat_statements p JOIN pg_database d ON d.oid = p.dbid
WHERE d.datname = :'db'
ORDER BY p.total_exec_time DESC LIMIT 20;
\o /dev/null
SELECT pg_stat_statements_reset();
"""


def redact_record(line: str) -> str:
    """Redact the trailing query= field of a PG_ACTIVITY or statement record."""
    head, marker, query = line.partition(' query=')
    if not marker:
        return redact_sql(line)
    return f'{head} query={redact_sql(query)}'


def psql(container: str, user: str, database: str, sql: str, timeout: float) -> list[str]:
    # The admin connects through the container's local socket; no URL or password.
    result = subprocess.run(
        ['docker', 'exec', '-i', container, 'psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1',
         '-v', f'db={database}', '-U', user, '-d', 'postgres'],
        input=sql, capture_output=True, text=True, timeout=timeout, check=False)
    if result.returncode != 0:
        raise RuntimeError(f'psql exit={result.returncode}')
    return [line for line in result.stdout.splitlines() if line]


# Server log lines use log_line_prefix '%m [%p] %u ': date, time, zone, [pid], user.
LOG_LINE = re.compile(
    r'^(?P<ts>\d{4}-\d\d-\d\d \d\d:\d\d:\d\d(?:\.\d+)? \S+) \[(?P<pid>\d+)\] (?P<user>\S*) ?'
    r'(?P<level>LOG|ERROR|FATAL|WARNING|DETAIL|STATEMENT|CONTEXT|HINT):\s+(?P<message>.*)$')
INTERESTING = re.compile(
    r'^duration: |still waiting for|acquired \S+ on .* after|deadlock detected'
    r'|canceling statement due to (?:lock|statement) timeout|lock timeout')
SQL_IN_MESSAGE = re.compile(r'^(duration: [0-9.]+ ms\s+(?:statement|execute [^:]*|parse [^:]*|bind [^:]*): )(.*)$')
LOCK_DETAIL = re.compile(r'^(?:Process holding the lock|Processes holding the lock|Wait queue|Process \d+ waits)')


def filter_server_log(lines: list[str]) -> list[str]:
    """Keep redacted lock-wait, deadlock and slow-statement records with their details."""
    kept: list[str] = []
    active_pid = None
    kind = None
    for raw in lines:
        line = raw.rstrip('\n')
        if line.startswith('\t'):
            # A continuation of the previous record, if that record is kept.
            if active_pid is None:
                continue
            body = line.strip()
            if kind == 'sql':
                kept[-1] = kept[-1] + ' ' + redact_sql(body, 120)
            elif kind == 'detail' and LOCK_DETAIL.match(body):
                kept.append(f'  {body[:200]}')
            continue
        match = LOG_LINE.match(line)
        if not match:
            active_pid = None
            continue
        level, message, pid = match['level'], match['message'], match['pid']
        prefix = f"{match['ts']} [{pid}] {match['user'] or '-'} {level}:"
        if level in ('LOG', 'ERROR', 'FATAL', 'WARNING'):
            active_pid, kind = None, None
            if not INTERESTING.search(message):
                continue
            sql = SQL_IN_MESSAGE.match(message)
            if sql:
                message, kind = sql.group(1) + redact_sql(sql.group(2)), 'sql'
            elif SECRET.search(message):
                message = '<redacted>'
            kept.append(f'{prefix} {message[:240]}')
            active_pid = pid
            continue
        if pid != active_pid:
            continue
        if level == 'STATEMENT':
            kind = 'sql'
            kept.append(f'{prefix} {redact_sql(message)}')
        elif level == 'CONTEXT':
            kind = 'sql'
            kept.append(f'{prefix} {redact_sql(message, 160)}')
        elif level == 'DETAIL':
            kind = 'detail'
            if LOCK_DETAIL.match(message):
                kept.append(f'{prefix} {message[:200]}')
        else:
            kind = None
    return kept


def read_offset(path: Path) -> int:
    try:
        return int(path.read_text().strip() or 0)
    except (FileNotFoundError, ValueError):
        return 0


def sample(args: argparse.Namespace) -> int:
    stopping = False

    def stop(_signal, _frame):
        nonlocal stopping
        stopping = True
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    out = Path(args.out)
    written = sum(1 for _ in out.open()) if out.exists() else 0
    failures = 0
    with out.open('a', encoding='utf-8') as stream:
        while not stopping:
            started = time.monotonic()
            try:
                records = psql(args.container, args.user, args.database, ACTIVITY_SQL, 10)
                failures = 0
            except (RuntimeError, subprocess.TimeoutExpired, OSError):
                records = []
                failures += 1
                if failures == 1:
                    stream.write(f'PG_ACTIVITY_UNAVAILABLE ts={int(time.time() * 1000)}\n')
            for record in records:
                if written >= ACTIVITY_LIMIT:
                    stream.write(f'PG_ACTIVITY_LIMIT records={written}\n')
                    stream.flush()
                    return 0
                stream.write(redact_record(record) + '\n')
                written += 1
            stream.flush()
            remaining = args.interval - (time.monotonic() - started)
            while remaining > 0 and not stopping:
                time.sleep(min(0.25, remaining))
                remaining -= 0.25
    return 0


def activity_summary(lines: list[str]) -> tuple[int, int, list[str]]:
    """Count lock waits and blocked samples; return the longest transactions."""
    waits = blocked = 0
    longest: dict[str, tuple[int, str]] = {}
    for line in lines:
        if not line.startswith('PG_ACTIVITY '):
            continue
        fields = dict(re.findall(r'\b([a-z_]+)=(\S+)', line.partition(' query=')[0]))
        if fields.get('wait', '').startswith('Lock.'):
            waits += 1
        if fields.get('blocked_by', '-') != '-':
            blocked += 1
        key = f"{fields.get('pid')}/{fields.get('xact_start_ms')}"
        xact = int(fields['xact_ms']) if fields.get('xact_ms', '').isdigit() else 0
        if xact >= longest.get(key, (-1, ''))[0]:
            longest[key] = (xact, line)
    ranked = sorted(longest.values(), key=lambda item: item[0], reverse=True)
    return waits, blocked, [line for _, line in ranked[:EXCERPT]]


def dump(args: argparse.Namespace) -> int:
    directory = Path(args.dir)
    directory.mkdir(parents=True, exist_ok=True)
    stage = re.sub(r'[^A-Za-z0-9_.-]', '_', args.stage)
    try:
        statements = [redact_record(line) for line in
                      psql(args.container, args.user, args.database, STATEMENTS_SQL, 30)]
    except (RuntimeError, subprocess.TimeoutExpired, OSError):
        statements = ['statements unavailable']
    (directory / f'statements-{stage}.txt').write_text('\n'.join(statements) + '\n')

    offset_file = directory / '.server-log-lines'
    offset = read_offset(offset_file)
    try:
        logs = subprocess.run(['docker', 'logs', args.container], capture_output=True,
                              text=True, errors='replace', timeout=30, check=False)
        server = (logs.stdout + logs.stderr).splitlines()
    except (subprocess.TimeoutExpired, OSError):
        server = []
    if len(server) < offset:
        offset = 0
    offset_file.write_text(str(len(server)))
    server_lines = filter_server_log(server[offset:])
    if server_lines:
        with (directory / 'server-lock-slow.log').open('a', encoding='utf-8') as stream:
            stream.write(f'--- stage {stage}\n' + '\n'.join(server_lines) + '\n')

    activity: list[str] = []
    if args.activity:
        activity_offset_file = directory / '.activity-lines'
        activity_offset = read_offset(activity_offset_file)
        try:
            all_activity = Path(args.activity).read_text(encoding='utf-8', errors='replace').splitlines()
        except FileNotFoundError:
            all_activity = []
        activity_offset_file.write_text(str(len(all_activity)))
        activity = all_activity[activity_offset:]
    waits, blocked, longest = activity_summary(activity)
    slow = sum(' duration: ' in line for line in server_lines)
    lock_waits = sum('still waiting for' in line for line in server_lines)
    deadlocks = sum('deadlock detected' in line for line in server_lines)
    print(f'NODE_BOOT_PG stage={stage} slow_statements={slow} lock_wait_logs={lock_waits} '
          f'deadlocks={deadlocks} sampled={len(activity)} sampled_lock_waits={waits} '
          f'sampled_blocked={blocked}')
    if args.verbose:
        print(f'--- postgres top statements by total time (stage {stage})')
        for line in statements[:EXCERPT]:
            print(f'NODE_BOOT_PG_STATEMENT {line}')
        print('--- postgres lock waits, deadlocks and slow statements')
        for line in server_lines[-2 * EXCERPT:]:
            print(f'NODE_BOOT_PG_LOG {line}')
        print('--- postgres longest sampled transactions')
        for line in longest:
            print(f'NODE_BOOT_{line}')
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split('\n', 1)[0])
    commands = parser.add_subparsers(dest='command', required=True)
    for name in ('sample', 'dump'):
        command = commands.add_parser(name)
        command.add_argument('--container', required=True)
        command.add_argument('--user', required=True)
        command.add_argument('--database', required=True)
    commands.choices['sample'].add_argument('--out', required=True)
    commands.choices['sample'].add_argument('--interval', type=float, default=2.0)
    commands.choices['dump'].add_argument('--stage', required=True)
    commands.choices['dump'].add_argument('--dir', required=True)
    commands.choices['dump'].add_argument('--activity')
    commands.choices['dump'].add_argument('--verbose', action='store_true')
    args = parser.parse_args(argv)
    return sample(args) if args.command == 'sample' else dump(args)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
