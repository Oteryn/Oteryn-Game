#!/usr/bin/env python3
"""Regenerate owner-accepted world_daily V2 exact byte fixtures (synthetic keys)."""
from datetime import datetime, timedelta
import json
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'docs/contracts/fixtures/public-projections'
BERLIN = ZoneInfo('Europe/Berlin')
WORLD = '01934f10-7c02-7001-805b-3b1122334401'


def stamp(day):
    return int(datetime.fromisoformat(day).replace(tzinfo=BERLIN).timestamp())


def fixture(name, start, previous, withdrawn=False, empty=False):
    end = int((datetime.fromtimestamp(start, BERLIN).replace(tzinfo=None) + timedelta(days=1)).replace(tzinfo=BERLIN).timestamp())
    observed = start + 8 * 3600
    daily = [] if withdrawn else [{
        'reset_epoch': '100', 'effective_from': str(start), 'effective_until': str(end),
        'creature_key': 'oteryn:monster.dragon', 'boss_key': 'oteryn:boss.morgaroth',
        'history_complete_from': str(start if empty else previous),
        'history': [] if empty else [{
            'reset_epoch': '99', 'effective_from': str(previous), 'effective_until': str(start),
            'creature_key': 'oteryn:monster.demon', 'boss_key': 'oteryn:boss.orshabaal'}],
        'raids': [{'raid_key': f'raid:public-{state}', 'scheduled_at': str(start + (n + 1) * 3600), 'state': state}
                  for n, state in enumerate(['running', 'done', 'cancelled'])]}]
    value = {'contract_version': 2, 'operation': 'PublishWorldDailyV2',
             'source_authority': 'oteryn:game:primary', 'world_id': WORLD,
             'projection_epoch': '1', 'projection_revision': '3',
             'source_observed_at': str(observed), 'daily': daily}
    (OUT / f'world_daily_{name}_v2.json').write_bytes(
        json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode())


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    fixture('dst_spring', stamp('2026-03-29T06:00:00'), stamp('2026-03-28T06:00:00'))
    fixture('dst_autumn', stamp('2026-10-25T06:00:00'), stamp('2026-10-24T06:00:00'))
    fixture('empty_history', stamp('2026-10-25T06:00:00'), 0, empty=True)
    fixture('withdrawn', stamp('2026-10-25T06:00:00'), 0, withdrawn=True)


if __name__ == '__main__':
    main()
