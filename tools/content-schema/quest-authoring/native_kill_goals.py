"""Assemble bounded native-goal cohorts, checking every captured spawn origin."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
COHORTS = ROOT / 'tools/content-schema/quest-authoring/samples/native-kill-goal-cohorts'
OUTPUT = ROOT / 'tools/content-schema/quest-authoring/samples/native-kill-goals.json'


def document():
    goals, keys = [], set()
    for path in sorted(COHORTS.glob('*.json')):
        cohort = json.loads(path.read_bytes())
        if cohort.get('schema') != 1:
            raise ValueError('NATIVE_GOAL_COHORT_SCHEMA')
        for goal in cohort['goals']:
            if goal['key'] in keys:
                raise ValueError('NATIVE_GOAL_DUPLICATE')
            keys.add(goal['key'])
            spawns = []
            for source in goal['sources']:
                relative = Path(source['path'])
                if relative.is_absolute() or '..' in relative.parts or not source['path'].startswith(
                        ('content/world/spawns/', 'tools/content-schema/quest-authoring/samples/')):
                    raise ValueError('NATIVE_GOAL_SOURCE_SCOPE')
                raw = (ROOT / relative).read_bytes()
                if hashlib.sha256(raw).hexdigest() != source['sha256']:
                    raise ValueError('NATIVE_GOAL_SOURCE_DRIFT')
                if source['path'].startswith('content/world/spawns/spawns-'):
                    spawns.append(json.loads(raw))
            for origin in goal['origins']:
                matches = 0
                for source in spawns:
                    value = source
                    try:
                        for field in origin['source_pointer'].strip('/').split('/'):
                            value = value[int(field)] if isinstance(value, list) else value[field]
                    except (KeyError, IndexError, ValueError):
                        continue
                    if (value.get('creature') == origin['creature'] and value.get('cell') == {
                            'x': origin['x'], 'y': origin['y'], 'floor': origin['floor']}):
                        matches += 1
                if matches != 1:
                    raise ValueError('NATIVE_GOAL_EXACT_ORIGIN')
            goals.append(goal)
    if not 1 <= len(goals) <= 16:
        raise ValueError('NATIVE_GOAL_BOUND')
    return dict(schema=1, goals=sorted(goals, key=lambda g: g['key']))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = json.dumps(document(), indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text() != raw:
            raise SystemExit('NATIVE_KILL_GOAL_DRIFT')
    else:
        OUTPUT.write_text(raw)
