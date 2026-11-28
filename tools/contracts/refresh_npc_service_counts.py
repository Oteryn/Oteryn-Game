#!/usr/bin/env python3
"""Refresh strict NPC model count assertions from the retained production catalogue."""
import argparse
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TARGET = ROOT / 'apps/game-server/tests/content_npc_service_model.rs'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
env = {**os.environ, 'OTERYN_CAPTURE_NPC_SERVICE_COUNTS': '1'}
result = subprocess.run(['cargo', 'test', '--locked', '-p', 'oteryn-game-server',
    '--test', 'content_npc_service_model', '--', '--nocapture', '--test-threads=1'],
    cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
# Libtest can print the current test name before the capture on the same line.
records = re.findall(r'\bNPC_SERVICE_COUNTS (\{.*\})$', result.stdout, re.MULTILINE)
if len(records) != 1:
    raise SystemExit(result.stdout)
counts = json.loads(records[0])
print(json.dumps(counts, sort_keys=True))
text = TARGET.read_text()
for expression, field in [('model.npcs().len()', 'npcs'), ('model.held_npcs().len()', 'held_npcs'),
    ('model.services().len()', 'services'), ('model.admitted_offer_count()', 'offers'),
    ('model.admitted_route_count()', 'routes'), ('model.generated_reply_npc_count()', 'replies')]:
    count = format(counts[field], ',').replace(',', '_')
    text, n = re.subn(r'(assert_eq!\(' + re.escape(expression) + r', )\d[\d_]*(\);)',
        lambda m, count=count: m[1] + count + m[2], text)
    if n != 1:
        raise SystemExit(f'Expected one strict assertion for {expression}')
for reason, count in counts['held'].items():
    text, n = re.subn(r'(\("' + re.escape(reason) + r'", )\d[\d_]*(\),)',
        lambda m, count=count: m[1] + str(count) + m[2], text)
    if n != 1:
        raise SystemExit(f'Expected one strict held-offer assertion for {reason}')
if args.check:
    if text != TARGET.read_text():
        raise SystemExit('STALE NPC service model count pins; run refresh_npc_service_counts.py')
    if result.returncode:
        raise SystemExit(result.stdout)
else:
    TARGET.write_text(text)
print('PASS strict NPC service model count pins' if args.check else 'Regenerated NPC service model count pins')
