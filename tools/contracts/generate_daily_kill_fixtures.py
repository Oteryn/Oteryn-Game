#!/usr/bin/env python3
"""Regenerate G-8 World/day shared byte fixtures without private dimensions."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'docs/contracts/fixtures/public-projections'
def main():
    OUT.mkdir(parents=True,exist_ok=True)
    for name,days,revision,observed in [
        ('complete',[{'day_end':'1790035200','monsters_killed':'12345','bosses_killed':'12','complete':True}],'2','1790035201'),
        ('provisional',[{'day_end':'1790035200','monsters_killed':'10','bosses_killed':'0','complete':False}],'1','1789949900'),
        ('withdrawn',[],'3','1790035201'),
        ('zero',[{'day_end':'1790035200','monsters_killed':'0','bosses_killed':'0','complete':True}],'1','1790035201')]:
        wire={'contract_version':2,'operation':'PublishWorldDailyKillsV2','source_authority':'oteryn:game:primary',
              'world_id':'01934f10-7c02-7001-805b-3b1122334401','day_start':'1789948800',
              'projection_epoch':'1','projection_revision':revision,'source_observed_at':observed,'days':days}
        (OUT/f'world_daily_kills_{name}_v2.json').write_bytes(json.dumps(wire,separators=(',',':')).encode())
if __name__=='__main__': main()
