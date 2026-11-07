#!/usr/bin/env python3
"""Refresh reviewed Rust and explicit non-trigger SQL key-33 inventories.

Use --reader path::function only after reviewing that lock-only reader. The
refresh refuses a reader containing a direct key-33 table mutation. It never
changes lane proofs, delegated callers, the three SQL trigger pins or assertions.
Use --sql-writer path::function only after its independent SQL authority review.
"""
import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
PIN = ROOT / 'apps/game-server/tests/integration_rest/spell_lane_key33_pin.rs'
TABLE = r'(?:game_item_ground_locations|game_item_corpse_container_entries|game_native_map_scope_adoptions)\b'
WRITE = re.compile(r'(?:INSERT INTO|DELETE FROM|UPDATE) ' + TABLE)
LOCK = re.compile(r"'hex'\), ?33\)")
ROW = re.compile(r'\("([^"]+)", "([^"]+)"\)')


def block(text, name):
    return re.search(r'const ' + name + r':[^=]+?= &\[(.*?)\n\];', text, re.S)


def sql_function(text, name):
    start = text.index('CREATE FUNCTION ' + name + '(')
    end = text.index('END $$;', start) + len('END $$;')
    return text[start:end]


def validate_sql_writer(path, name):
    # This is the one independently reviewed current-clock owner. A new owner
    # must add its own guard review, rather than inheriting a generic SECDEF pass.
    if name != 'game_furniture_ground_decay_expire' or not path.startswith('apps/game-server/migrations/') or not path.endswith('.sql'):
        raise ValueError('unreviewed direct SQL key33 owner')
    sql = ' '.join((ROOT / path).read_text().split())
    body = sql_function(sql, name)
    fence = 'PERFORM game_furniture_ground_clock_fence(p_node,p_node_revision,p_world,p_channel,p_scope);'
    lock = LOCK.search(body)
    if 'RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$' not in body or lock is None or body.index(fence) >= lock.start():
        raise ValueError('direct SQL owner lacks its reviewed pre-lock authority')
    for proof in [
        'ground.scope_kind=1 AND ground.channel_id=p_channel AND ground.runtime_scope_ownership_generation=p_scope',
        'timer.deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint',
        'FOR UPDATE OF i,timer', 'FOR UPDATE OF i,g',
        'INSERT INTO game_furniture_carried_decay_receipts', 'INSERT INTO game_item_timed_state_writes',
        'UPDATE game_item_instances', 'UPDATE game_item_timed_states',
        'DELETE FROM game_item_ground_locations WHERE item_instance_id=c.item_instance_id',
    ]:
        if proof not in body:
            raise ValueError('direct SQL clock/custody proof changed: ' + proof)
    for helper, proofs in [
        ('game_furniture_ground_clock_fence', [
            'game_runtime_scope_assignment_history_valid() IS DISTINCT FROM TRUE',
            'game_node_lock_current_registration(p_node,p_node_revision) IS DISTINCT FROM TRUE',
            'runtime.scope_kind=1 AND runtime.world_id=p_world AND runtime.channel_id=p_channel',
            'runtime.state=1 AND runtime.ownership_generation=p_scope',
            'runtime.holder_node_id=p_node AND runtime.holder_registration_revision=p_node_revision',
            'g.ready AND g.ownership_generation=p_scope',
        ]),
        ('game_furniture_carried_decay_complete', [
            'r.created_xact_id<>pg_current_xact_id()', 'w.created_xact_id<>pg_current_xact_id()',
            'w.cause<>4', 'PERFORM game_furniture_ground_clock_fence(',
            "w.expected_revision<>(r.before_timer->>'state_revision')::numeric",
            'to_jsonb(i)=r.after_item', 'to_jsonb(t)=r.after_timer',
            "to_jsonb(g)=r.location->'custody' AND g.scope_kind=1",
            "g.world_id=(r.context->>'world')::uuid AND g.channel_id=(r.context->>'channel')::uuid",
            "g.runtime_scope_ownership_generation=(r.context->>'scope')::numeric",
        ]),
    ]:
        for proof in proofs:
            if proof not in sql_function(sql, helper):
                raise ValueError('direct SQL reciprocal/fence proof changed: ' + proof)
    if ('CREATE CONSTRAINT TRIGGER game_furniture_carried_decay_proven AFTER INSERT ON game_furniture_carried_decay_receipts '
        'DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_furniture_carried_decay_proven()') not in sql:
        raise ValueError('direct SQL owner lost its deferred receipt guard')
    if 'game_furniture_carried_decay_complete(NEW.transaction_id) IS DISTINCT FROM TRUE' not in sql_function(sql, 'game_furniture_carried_decay_proven'):
        raise ValueError('direct SQL owner lost receipt reciprocity')


def refresh(readers, sql_writers=()):
    original = text = PIN.read_text()
    direct_block = block(text, 'SQL_KEY33_DIRECT_WRITERS')
    direct = set(ROW.findall(direct_block[1]))
    for writer in sql_writers:
        path, name = writer.rsplit('::', 1)
        direct.add((path, name))
    for path, name in direct:
        validate_sql_writer(path, name)
    trigger_block = re.search(r'const SQL_KEY33_FUNCTIONS:[^=]+?= \[(.*?)\n\];', text, re.S)
    triggers = {name for name, _ in re.findall(r'\(\s*"([^"]+)",\s*"([^"]+)",?\s*\)', trigger_block[1])}
    found = set()
    for path in sorted((ROOT / 'apps/game-server/migrations').glob('*.sql')):
        sql = ' '.join(path.read_text().split())
        for match in LOCK.finditer(sql):
            before = sql[:match.start()]
            found.add(re.search(r'FUNCTION (\w+)\(', before[before.rfind('FUNCTION '):])[1])
    if found != triggers | {name for _, name in direct} or triggers & {name for _, name in direct}:
        raise ValueError('SQL key33 exact inventory changed; review trigger/direct owners separately')
    registered = block(text, 'NON_LANE_KEY33_READS')
    non_lane = set(ROW.findall(registered[1]))
    for reader in readers:
        path, name = reader.rsplit('::', 1)
        non_lane.add((path, name))
    sites, writes = set(), set()
    roots = [ROOT / 'apps/game-server/src', *(ROOT / 'crates/gameplay').glob('*/src')]
    for root in roots:
        for path in sorted(root.rglob('*.rs')):
            relative = path.relative_to(ROOT)
            if any(p == 'tests' or p.endswith('_tests') for p in relative.parts) or path.name.endswith('_tests.rs'):
                continue
            source = re.split(r'#\[cfg\(test\)\]\s*(?:#\[allow\([^]]*\)\]\s*)?(?:pub\(crate\) )?mod ', path.read_text())[0]
            source = ' '.join(source.split())
            for pattern, destination in [(LOCK, sites), (WRITE, writes)]:
                for match in pattern.finditer(source):
                    functions = list(re.finditer(r'\bfn (\w+)', source[:match.start()]))
                    if not functions:
                        raise ValueError(f'{relative}: site outside function')
                    destination.add((relative.as_posix(), functions[-1][1]))
            # SQL delegation does not remove the Rust lane-proof requirement.
            for _, name in direct:
                for match in re.finditer(r'\bSELECT ' + re.escape(name) + r'\(', source):
                    functions = list(re.finditer(r'\bfn (\w+)', source[:match.start()]))
                    if not functions:
                        raise ValueError(f'{relative}: SQL owner call outside function')
                    sites.add((relative.as_posix(), functions[-1][1]))
    sites |= writes
    if not non_lane <= sites or non_lane & writes:
        raise ValueError('non-lane readers must be present and contain no key-33 table writes')
    for name, values in [('RUST_KEY33_SITES', sites), ('NON_LANE_KEY33_READS', non_lane), ('SQL_KEY33_DIRECT_WRITERS', direct)]:
        match = block(text, name)
        rows = ''.join(f'    ("{path}", "{function}"),\n' for path, function in sorted(values))
        text = text[:match.start(1)] + '\n' + rows.rstrip('\n') + text[match.end(1):]
    if text != original:
        PIN.write_text(text)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reader', action='append', default=[], metavar='PATH::FUNCTION')
    parser.add_argument('--sql-writer', action='append', default=[], metavar='PATH::FUNCTION')
    args = parser.parse_args()
    refresh(args.reader, args.sql_writer)
